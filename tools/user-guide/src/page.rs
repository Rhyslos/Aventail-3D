//! One manual page (or the welcome words) from Markdown to the guide's
//! HTML: headings numbered into sections with anchors, links between pages
//! turned into links inside the guide, pictures into figures, keys into
//! keyboard keys, the one-line summary under the title into the lead.

use crate::keys::{self, Context};
use crate::links::Site;
use crate::slug::PageSlugs;
use pulldown_cmark::{BlockQuoteKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// The Markdown features the manual uses (GitHub's tables and the rest).
pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}

/// Each heading's level and words, in order, for working out anchors
/// before any page is drawn.
pub(crate) fn headings(markdown: &str) -> Vec<(usize, String)> {
    let events: Vec<Event<'_>> = Parser::new_ext(markdown, options()).collect();
    let mut out = Vec::new();
    for (i, event) in events.iter().enumerate() {
        if let Event::Start(Tag::Heading { level, .. }) = event {
            out.push((level_number(*level), heading_text(&events, i)));
        }
    }
    out
}

/// The anchors a page sets in its own HTML (`<a id="mipmaps"></a>`), so
/// links can name a term in the middle of a page.
pub(crate) fn raw_ids(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    for event in Parser::new_ext(markdown, options()) {
        if let Event::Html(html) | Event::InlineHtml(html) = event {
            out.extend(ids_in(&html).into_iter().map(|(_, _, id)| id.to_string()));
        }
    }
    out
}

/// Each `id="…"` in some HTML: where its value starts and ends, and the
/// value.
fn ids_in(html: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = html[from..].find("id=\"") {
        let start = from + at + 4;
        let before = html[..from + at].chars().next_back();
        let Some(len) = html[start..].find('"') else {
            break;
        };
        if before.is_some_and(char::is_whitespace) {
            out.push((start, start + len, &html[start..start + len]));
        }
        from = start + len;
    }
    out
}

/// What is being drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A manual page: its `#` title starts a section, its `##` headings
    /// are parts of it.
    Page,
    /// The words before the first chapter, under the guide's title.
    Welcome,
}

/// A page drawn as HTML.
pub(crate) struct Rendered {
    /// The words of its `#` title, if it has one.
    pub(crate) title: Option<String>,
    /// The HTML.
    pub(crate) html: String,
}

/// Draws one page. Links and pictures that lead nowhere are reported to
/// `site` and drawn as written.
pub(crate) fn render(stem: &str, markdown: &str, kind: Kind, site: &mut Site<'_>) -> Rendered {
    let events: Vec<Event<'_>> = Parser::new_ext(markdown, options()).collect();
    let mut r = Renderer {
        site,
        stem,
        kind,
        out: Vec::with_capacity(events.len()),
        slugs: PageSlugs::default(),
        depth: 0,
        part_open: false,
        title: None,
        title_open: false,
        lead_pending: kind == Kind::Welcome,
        closers: Vec::new(),
        paragraph_closer: None,
        heading_closer: None,
        recent: String::new(),
        last_kbd: None,
        table_head: false,
        headers: Vec::new(),
        column: 0,
        next_column: 0,
        cell_text: String::new(),
    };
    r.run(events);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, r.out.into_iter());
    Rendered {
        title: r.title,
        html,
    }
}

struct Renderer<'s, 'a, 'b> {
    site: &'b mut Site<'a>,
    stem: &'b str,
    kind: Kind,
    out: Vec<Event<'s>>,
    slugs: PageSlugs,
    /// How deep inside lists, quotes and tables: headings at 0 start parts.
    depth: usize,
    part_open: bool,
    title: Option<String>,
    /// The title's heading is being drawn.
    title_open: bool,
    /// The next block may be the lead (the page's summary line).
    lead_pending: bool,
    closers: Vec<&'static str>,
    paragraph_closer: Option<&'static str>,
    heading_closer: Option<String>,
    /// The words of the current paragraph, item or cell so far, for
    /// telling keys from other bold words.
    recent: String,
    /// Where in `recent` the last key ended.
    last_kbd: Option<usize>,
    table_head: bool,
    headers: Vec<String>,
    column: usize,
    next_column: usize,
    cell_text: String,
}

impl<'s> Renderer<'s, '_, '_> {
    fn html(&mut self, text: impl Into<String>) {
        self.out.push(Event::Html(CowStr::from(text.into())));
    }

    fn inline(&mut self, text: impl Into<String>) {
        self.out.push(Event::InlineHtml(CowStr::from(text.into())));
    }

    fn new_block(&mut self) {
        self.recent.clear();
        self.last_kbd = None;
    }

    fn run(&mut self, mut events: Vec<Event<'s>>) {
        let mut i = 0;
        while i < events.len() {
            i = self.step(&mut events, i);
        }
        if self.part_open {
            self.html("</section>\n");
        }
    }

    /// Handles the event at `i` and returns the index of the next one.
    fn step(&mut self, events: &mut [Event<'s>], i: usize) -> usize {
        let event = events[i].clone();
        match event {
            Event::Start(Tag::Heading { level, .. }) => return self.heading(events, i, level),
            Event::End(TagEnd::Heading(_)) => {
                let closer = self.heading_closer.take().unwrap_or_default();
                self.html(closer);
                // The quote straight after the title is the page's summary.
                self.lead_pending = std::mem::take(&mut self.title_open);
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                let class = match kind {
                    None if self.lead_pending && self.depth == 0 => "lead",
                    Some(BlockQuoteKind::Tip) => "tip",
                    Some(BlockQuoteKind::Warning | BlockQuoteKind::Caution) => "warn",
                    _ => "note",
                };
                self.lead_pending = false;
                self.html(format!("<div class=\"{class}\">\n"));
                self.closers.push("</div>\n");
                self.depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                let closer = self.closers.pop().unwrap_or_default();
                self.html(closer);
                self.depth = self.depth.saturating_sub(1);
            }
            Event::Start(Tag::Paragraph) => return self.paragraph(events, i),
            Event::End(TagEnd::Paragraph) => match self.paragraph_closer.take() {
                Some(closer) => self.html(closer),
                None => self.out.push(event),
            },
            Event::Start(Tag::List(Some(first))) => {
                self.lead_pending = false;
                self.depth += 1;
                if first == 1 {
                    self.html("<ol class=\"steps\">\n");
                } else {
                    let before = first.saturating_sub(1);
                    self.html(format!(
                        "<ol class=\"steps\" start=\"{first}\" style=\"counter-reset: step {before}\">\n"
                    ));
                }
            }
            Event::End(TagEnd::List(true)) => {
                self.depth = self.depth.saturating_sub(1);
                self.html("</ol>\n");
            }
            Event::Start(Tag::Item) => {
                self.new_block();
                self.out.push(event);
            }
            Event::Start(
                Tag::List(None)
                | Tag::Table(_)
                | Tag::FootnoteDefinition(_)
                | Tag::DefinitionList
                | Tag::CodeBlock(_)
                | Tag::HtmlBlock,
            ) => {
                self.lead_pending = false;
                self.depth += 1;
                self.out.push(event);
            }
            Event::End(
                TagEnd::List(false)
                | TagEnd::Table
                | TagEnd::FootnoteDefinition
                | TagEnd::DefinitionList
                | TagEnd::CodeBlock
                | TagEnd::HtmlBlock,
            ) => {
                self.depth = self.depth.saturating_sub(1);
                self.out.push(event);
            }
            Event::Start(Tag::TableHead) => {
                self.table_head = true;
                self.headers.clear();
                self.out.push(event);
            }
            Event::End(TagEnd::TableHead) => {
                self.table_head = false;
                self.out.push(event);
            }
            Event::Start(Tag::TableRow) => {
                self.next_column = 0;
                self.out.push(event);
            }
            Event::Start(Tag::TableCell) => {
                self.new_block();
                self.cell_text.clear();
                self.column = self.next_column;
                self.next_column += 1;
                self.out.push(event);
            }
            Event::End(TagEnd::TableCell) => {
                if self.table_head {
                    self.headers.push(self.cell_text.to_lowercase());
                }
                self.out.push(event);
            }
            Event::Start(Tag::Strong) => return self.strong(events, i),
            Event::Code(code) => match keys::html(&code, Context::Keys) {
                Some(kbd) => {
                    self.inline(kbd);
                    self.recent.push_str(&code);
                    self.last_kbd = Some(self.recent.len());
                }
                None => {
                    self.recent.push_str(&code);
                    if self.table_head {
                        self.cell_text.push_str(&code);
                    }
                    self.out.push(Event::Code(code));
                }
            },
            Event::Text(text) => {
                self.recent.push_str(&text);
                if self.table_head {
                    self.cell_text.push_str(&text);
                }
                self.out.push(Event::Text(text));
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let end = end_of(events, i, &TagEnd::Image);
                let alt = plain_text(&events[i + 1..end]);
                let img = self.image(&dest_url, &alt, false);
                self.inline(img);
                return end + 1;
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let href = self.site.link(self.stem, &dest_url);
                self.out.push(Event::Start(Tag::Link {
                    link_type,
                    dest_url: CowStr::from(href),
                    title,
                    id,
                }));
            }
            Event::Html(html) => {
                let html = self.page_ids(&html);
                self.html(html);
            }
            Event::InlineHtml(html) => {
                let html = self.page_ids(&html);
                self.inline(html);
            }
            other => self.out.push(other),
        }
        i + 1
    }

    /// Raw HTML with its anchors made the page's own (`mipmaps` becomes
    /// `concepts-mipmaps`), as links to them are.
    fn page_ids(&self, html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut last = 0;
        for (start, end, id) in ids_in(html) {
            out.push_str(&html[last..start]);
            out.push_str(&crate::escape(&self.site.anchor(self.stem, id)));
            last = end;
        }
        out.push_str(&html[last..]);
        out
    }

    fn heading(&mut self, events: &[Event<'s>], i: usize, level: HeadingLevel) -> usize {
        self.new_block();
        self.lead_pending = false;
        let text = heading_text(events, i);
        let slug = self.slugs.next(&text);
        let id = crate::escape(&self.site.anchor(self.stem, &slug));
        let level = level_number(level);
        match self.kind {
            Kind::Page if level == 1 && self.title.is_none() => {
                self.title = Some(text);
                self.title_open = true;
                self.html("<h3>");
                self.heading_closer = Some("</h3>\n".into());
            }
            Kind::Page if level <= 2 && self.depth == 0 => {
                if self.part_open {
                    self.html("</section>\n");
                }
                self.part_open = true;
                self.html(format!("<section class=\"part\" id=\"{id}\">\n<h4>"));
                self.heading_closer = Some("</h4>\n".into());
            }
            Kind::Page => {
                let n = (level + 2).clamp(4, 6);
                self.html(format!("<h{n} id=\"{id}\">"));
                self.heading_closer = Some(format!("</h{n}>\n"));
            }
            Kind::Welcome => {
                let n = level.clamp(3, 6);
                self.html(format!("<h{n} id=\"{id}\">"));
                self.heading_closer = Some(format!("</h{n}>\n"));
            }
        }
        i + 1
    }

    fn paragraph(&mut self, events: &mut [Event<'s>], i: usize) -> usize {
        self.new_block();
        if let Some(next) = self.figure(events, i) {
            self.lead_pending = false;
            return next;
        }
        if self.lead_pending && self.kind == Kind::Welcome && self.depth == 0 {
            self.lead_pending = false;
            self.html("<p class=\"lead\">");
            self.paragraph_closer = Some("</p>\n");
            return i + 1;
        }
        self.lead_pending = false;
        if self.depth == 0
            && let (
                Some(Event::Start(Tag::Strong)),
                Some(Event::Text(label)),
                Some(Event::End(TagEnd::Strong)),
            ) = (events.get(i + 1), events.get(i + 2), events.get(i + 3))
        {
            let class = match label.as_ref() {
                "Tip:" => Some("tip"),
                "Note:" | "Good to know:" => Some("note"),
                "Careful:" | "Warning:" => Some("warn"),
                _ => None,
            };
            if let Some(class) = class {
                self.html(format!("<div class=\"{class}\"><p>"));
                self.paragraph_closer = Some("</p></div>\n");
                if let Some(Event::Text(t)) = events.get(i + 4) {
                    let trimmed = t.trim_start().to_string();
                    events[i + 4] = Event::Text(CowStr::from(trimmed));
                }
                return i + 4;
            }
        }
        self.out.push(Event::Start(Tag::Paragraph));
        i + 1
    }

    /// A paragraph that is only a picture becomes a figure with its
    /// words as the caption. Returns the index after the paragraph.
    fn figure(&mut self, events: &[Event<'s>], i: usize) -> Option<usize> {
        let start = skip_blank(events, i + 1);
        let Some(Event::Start(Tag::Image {
            dest_url, title, ..
        })) = events.get(start)
        else {
            return None;
        };
        let end = end_of(events, start, &TagEnd::Image);
        let after = skip_blank(events, end + 1);
        if !matches!(events.get(after), Some(Event::End(TagEnd::Paragraph))) {
            return None;
        }
        let alt = plain_text(&events[start + 1..end]);
        let caption = if title.trim().is_empty() {
            alt.clone()
        } else {
            title.to_string()
        };
        let img = self.image(dest_url, &alt, true);
        self.html(format!(
            "<figure>{img}<figcaption>{}</figcaption></figure>\n",
            crate::escape(&caption)
        ));
        Some(after + 1)
    }

    fn image(&mut self, dest: &str, alt: &str, figure: bool) -> String {
        let (src, size) = self.site.picture(self.stem, dest);
        let size = size
            .map(|(w, h)| format!(" width=\"{w}\" height=\"{h}\""))
            .unwrap_or_default();
        let lazy = if figure { " loading=\"lazy\"" } else { "" };
        format!(
            "<img{lazy} src=\"{}\"{size} alt=\"{}\">",
            crate::escape(&src),
            crate::escape(alt)
        )
    }

    /// Bold words that are a key become keyboard keys.
    fn strong(&mut self, events: &[Event<'s>], i: usize) -> usize {
        if let (Some(Event::Text(text)), Some(Event::End(TagEnd::Strong))) =
            (events.get(i + 1), events.get(i + 2))
        {
            let in_key_column = !self.table_head
                && self
                    .headers
                    .get(self.column)
                    .is_some_and(|h| h.contains("shortcut") || h.contains("key"));
            let chained = self
                .last_kbd
                .and_then(|at| self.recent.get(at..))
                .is_some_and(keys::joins);
            let context = if in_key_column || chained || keys::cued(&self.recent) {
                Context::Keys
            } else {
                Context::Plain
            };
            if let Some(kbd) = keys::html(text, context) {
                self.inline(kbd);
                self.recent.push_str(text);
                self.last_kbd = Some(self.recent.len());
                return i + 3;
            }
        }
        self.out.push(Event::Start(Tag::Strong));
        i + 1
    }
}

fn level_number(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// The words of the heading starting at `i`.
fn heading_text(events: &[Event<'_>], i: usize) -> String {
    let end = events[i..]
        .iter()
        .position(|e| matches!(e, Event::End(TagEnd::Heading(_))))
        .map_or(events.len(), |p| i + p);
    plain_text(&events[(i + 1).min(end)..end])
}

/// The words in some events, without their formatting.
fn plain_text(events: &[Event<'_>]) -> String {
    let mut text = String::new();
    for event in events {
        match event {
            Event::Text(t) | Event::Code(t) => text.push_str(t),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }
    text
}

/// The index of the end matching the start at `i` (the last event if the
/// end is missing, which the parser never does).
fn end_of(events: &[Event<'_>], i: usize, end: &TagEnd) -> usize {
    events[i + 1..]
        .iter()
        .position(|e| matches!(e, Event::End(t) if t == end))
        .map_or(events.len().saturating_sub(1).max(i), |p| i + 1 + p)
}

/// The first index from `i` that isn't blank text or a line break.
fn skip_blank(events: &[Event<'_>], mut i: usize) -> usize {
    while let Some(e) = events.get(i) {
        match e {
            Event::Text(t) if t.trim().is_empty() => i += 1,
            Event::SoftBreak | Event::HardBreak => i += 1,
            _ => break,
        }
    }
    i
}
