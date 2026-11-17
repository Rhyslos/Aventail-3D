//! How the editor draws code (plan, milestone 3): colours from the
//! `script` crate's own small reader, mistakes underlined, line numbers
//! in a gutter. The colouring is worked out once per change of the text
//! or its mistakes, never per frame, and egui lays out only the lines
//! that changed, so typing stays smooth in long scripts.

use crate::theme::code;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Galley, Stroke};
use script::{Language, Problem, TokenKind};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

/// The colour a piece of code is drawn in.
pub(crate) fn colour(kind: Option<TokenKind>) -> Color32 {
    match kind {
        Some(TokenKind::Keyword) => code::KEYWORD,
        Some(TokenKind::TypeName) => code::TYPE_NAME,
        Some(TokenKind::Number) => code::NUMBER,
        Some(TokenKind::Text) => code::TEXT,
        Some(TokenKind::Comment) => code::COMMENT,
        Some(TokenKind::Symbol) => code::SYMBOL,
        Some(TokenKind::Name) | None => code::NAME,
    }
}

/// The byte ranges in `text` to underline for `problems` in the file
/// `file`: the piece of code a mistake points at, or its whole line
/// (without the space before it) when it names no column. Mistakes in
/// other files, or past the text's end, underline nothing.
pub(crate) fn underlines(text: &str, file: &str, problems: &[Problem]) -> Vec<Range<usize>> {
    let starts = line_starts(text);
    let mut ranges: Vec<Range<usize>> = problems
        .iter()
        .filter(|p| p.file == file && p.line > 0)
        .filter_map(|p| {
            let start = *starts.get(p.line as usize - 1)?;
            let end = text[start..].find('\n').map_or(text.len(), |n| start + n);
            let line = &text[start..end];
            let range = if p.column == 0 {
                let lead = line.len() - line.trim_start().len();
                start + lead..end
            } else {
                let at = line
                    .char_indices()
                    .nth(p.column as usize - 1)
                    .map_or(line.len(), |(i, _)| i);
                let rest = &line[at..];
                let word = rest
                    .char_indices()
                    .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
                    .map_or(rest.len(), |(i, _)| i);
                let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'');
                // Text in quotes: all of it; a symbol or the line's end: at
                // least one character.
                let len = if word > 0 {
                    word
                } else if let Some(q) = quote {
                    rest[1..].find(q).map_or(rest.len(), |i| i + 2)
                } else {
                    rest.chars().next().map_or(0, char::len_utf8)
                };
                start + at..start + at + len
            };
            // A mistake at the end of a line still shows: under the
            // line's last character.
            if range.is_empty() && !line.is_empty() {
                let last = line.char_indices().next_back().map_or(0, |(i, _)| i);
                return Some(start + last..end);
            }
            (!range.is_empty()).then_some(range)
        })
        .collect();
    ranges.sort_by_key(|r| r.start);
    ranges
}

/// Where each line of `text` starts, in bytes.
pub(crate) fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

/// A stretch of code drawn one way: where it is, what it is (nothing for
/// space), and whether it's underlined as a mistake.
pub(crate) type Piece = (Range<usize>, Option<TokenKind>, bool);

/// `text` cut into stretches drawn one way each, in order, covering it
/// all. No stretch crosses a line's end, so lines can be laid out one by
/// one.
pub(crate) fn pieces(text: &str, language: Language, underlined: &[Range<usize>]) -> Vec<Piece> {
    let tokens = script::tokens(text, language);
    // Every place the look may change, each list already in order, put
    // together in one pass (long scripts have hundreds of thousands).
    let at_tokens: Vec<usize> = tokens
        .iter()
        .flat_map(|t| [t.range.start, t.range.end])
        .collect();
    let at_lines: Vec<usize> = text
        .match_indices('\n')
        .flat_map(|(at, _)| [at, at + 1])
        .collect();
    let mut at_marks: Vec<usize> = underlined
        .iter()
        .flat_map(|r| [r.start.min(text.len()), r.end.min(text.len())])
        .collect();
    at_marks.sort_unstable();
    let mut cuts = merged(&merged(&at_tokens, &at_lines), &at_marks);
    if cuts.first() != Some(&0) {
        cuts.insert(0, 0);
    }
    if cuts.last() != Some(&text.len()) {
        cuts.push(text.len());
    }
    cuts.dedup();
    let mut out = Vec::with_capacity(cuts.len());
    let mut token = 0;
    let mut line = 0;
    for pair in cuts.windows(2) {
        let range = pair[0]..pair[1];
        while token < tokens.len() && tokens[token].range.end <= range.start {
            token += 1;
        }
        let kind = tokens
            .get(token)
            .filter(|t| t.range.start <= range.start)
            .map(|t| t.kind);
        while line < underlined.len() && underlined[line].end <= range.start {
            line += 1;
        }
        let marked = underlined[line..]
            .iter()
            .take_while(|r| r.start <= range.start)
            .any(|r| r.end > range.start);
        out.push((range, kind, marked));
    }
    out
}

/// Two lists in order, as one in order.
fn merged(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

/// How a stretch of code is drawn.
fn format(kind: Option<TokenKind>, marked: bool, font: &FontId) -> TextFormat {
    TextFormat {
        font_id: font.clone(),
        color: colour(kind),
        italics: kind == Some(TokenKind::Comment),
        underline: if marked {
            Stroke::new(1.5, code::MISTAKE)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

/// A job for `text` from its `pieces` (their ranges counted from
/// `offset`), never wrapped (code keeps its lines; the editor scrolls
/// sideways instead).
fn job_from(text: &str, pieces: &[Piece], offset: usize, font: &FontId) -> LayoutJob {
    let mut job = LayoutJob {
        text: text.to_string(),
        ..Default::default()
    };
    job.wrap.max_width = f32::INFINITY;
    for (range, kind, marked) in pieces {
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: (range.start - offset).into()..(range.end - offset).into(),
            format: format(*kind, *marked, font),
        });
    }
    if job.sections.is_empty() {
        // An empty line still needs a font, for its height.
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: 0.into()..0.into(),
            format: format(None, false, font),
        });
    }
    job
}

/// `text` coloured as `language`, with `underlined` ranges underlined,
/// in `font`, as one job (the editor lays out each line on its own; this
/// shows what they make together).
#[cfg(test)]
pub(crate) fn layout_job(
    text: &str,
    language: Language,
    underlined: &[Range<usize>],
    font: FontId,
) -> LayoutJob {
    job_from(text, &pieces(text, language, underlined), 0, &font)
}

/// The coloured, laid-out text, kept until the text, its mistakes, the
/// font or the screen's scale change: an unchanged script costs nothing
/// to draw again, however long. When it changes, each line is coloured
/// and laid out again only if it changed itself (as egui lays out long
/// text, with the lines kept here rather than in egui's memory, which
/// forgets them after a frame unused).
#[derive(Default)]
pub(crate) struct Coloured {
    laid_out: Option<LaidOut>,
    /// Each line laid out, by what it holds and how it looks.
    lines: HashMap<u64, Arc<Galley>>,
}

struct LaidOut {
    key: u64,
    pixels_per_point: f32,
    /// How full egui's glyph atlas was: when it's emptied (it's made
    /// again once nearly full), the old layout's glyphs are gone.
    atlas_fill: f32,
    galley: Arc<Galley>,
}

impl Coloured {
    /// The laid-out text for the editor, laid out again only where
    /// something changed.
    pub(crate) fn galley(
        &mut self,
        ctx: &egui::Context,
        text: &str,
        language: Language,
        file: &str,
        problems: &[Problem],
        font: &FontId,
    ) -> Arc<Galley> {
        let key = key(text, problems, font);
        let pixels_per_point = ctx.pixels_per_point();
        let atlas_fill = ctx.fonts(|f| f.font_atlas_fill_ratio());
        if let Some(kept) = &mut self.laid_out {
            let fresh = kept.pixels_per_point == pixels_per_point && atlas_fill >= kept.atlas_fill;
            if !fresh {
                self.lines.clear();
            } else if kept.key == key {
                kept.atlas_fill = atlas_fill;
                return kept.galley.clone();
            }
        }
        let underlined = underlines(text, file, problems);
        let all = pieces(text, language, &underlined);
        let mut kept = HashMap::with_capacity(self.lines.len());
        let mut lines = Vec::new();
        let mut at = 0;
        for range in paragraph_ranges(text) {
            let first = at;
            while at < all.len() && all[at].0.end <= range.end {
                at += 1;
            }
            // The line's own stretches, without the line's end before it.
            let skip = all[first..at]
                .iter()
                .take_while(|p| p.0.start < range.start || p.0.is_empty())
                .count();
            let own = &all[first + skip..at];
            let line_text = &text[range.clone()];
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            line_text.hash(&mut hasher);
            for (r, kind, marked) in own {
                (r.start - range.start, kind, marked).hash(&mut hasher);
            }
            font.hash(&mut hasher);
            let hash = hasher.finish();
            let galley = match self.lines.remove(&hash).or_else(|| kept.get(&hash).cloned()) {
                Some(galley) => galley,
                None => {
                    let job = job_from(line_text, own, range.start, font);
                    ctx.fonts_mut(|f| f.layout_job(job))
                }
            };
            kept.insert(hash, galley.clone());
            lines.push(galley);
        }
        self.lines = kept;
        let galley = if lines.len() == 1 {
            lines[0].clone()
        } else {
            let whole = LayoutJob {
                sections: vec![egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: 0.into()..text.len().into(),
                    format: format(None, false, font),
                }],
                ..LayoutJob::default()
            };
            let mut whole = whole;
            whole.text = text.to_string();
            whole.wrap.max_width = f32::INFINITY;
            Arc::new(Galley::concat(Arc::new(whole), &lines, pixels_per_point))
        };
        self.laid_out = Some(LaidOut {
            key,
            pixels_per_point,
            atlas_fill: ctx.fonts(|f| f.font_atlas_fill_ratio()),
            galley: galley.clone(),
        });
        galley
    }
}

/// Where each of `text`'s lines is, as egui splits long text: each line
/// without its `\n`, the last keeping a final one.
pub(crate) fn paragraph_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    loop {
        let mut end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        if end + 1 == text.len() && text.ends_with('\n') {
            end += 1;
        }
        out.push(start..end);
        start = end + 1;
        if start > text.len() || end >= text.len() {
            return out;
        }
    }
}

/// What the colouring depends on, in one number.
fn key(text: &str, problems: &[Problem], font: &FontId) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    problems.hash(&mut hasher);
    font.hash(&mut hasher);
    hasher.finish()
}

/// Draws the line numbers for `galley` (laid out at `galley_pos`) in the
/// gutter ending at `right`, only for the rows that can be seen.
pub(crate) fn paint_line_numbers(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    right: f32,
    font: &FontId,
) {
    let clip = ui.clip_rect();
    let painter = ui.painter();
    let mut line = 1;
    let mut starts_line = true;
    for row in &galley.rows {
        let top = galley_pos.y + row.pos.y;
        let bottom = top + row.rect().height();
        if starts_line && bottom >= clip.top() && top <= clip.bottom() {
            painter.text(
                egui::pos2(right, top),
                egui::Align2::RIGHT_TOP,
                line.to_string(),
                font.clone(),
                code::LINE_NUMBER,
            );
        }
        starts_line = row.ends_with_newline;
        if row.ends_with_newline {
            line += 1;
        }
        if top > clip.bottom() {
            break;
        }
    }
}

/// The gutter's width for `lines` lines in `font`.
pub(crate) fn gutter_width(ui: &egui::Ui, lines: usize, font: &FontId) -> f32 {
    let digits = lines.max(1).ilog10() as usize + 1;
    let digit = ui.ctx().fonts_mut(|f| f.glyph_width(font, '0'));
    digit * digits.max(2) as f32 + 10.0
}
