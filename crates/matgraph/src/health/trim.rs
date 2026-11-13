//! The health check's trim sheet warnings: strips and the project's trim
//! rows disagreeing, and cut strips cutting through a row of their
//! pattern ([decision 0289](../../../../docs/decisions/0289-trim-sheet-box.md)).

use super::{Action, Concern, Looking, Remedy, Warning, length_words, measure};
use crate::baking::join;
use crate::boxes::trim_sheet::{
    Laid, Layout, ON_ROWS, OWN_HEIGHTS, SHRINK, STRIP_KEYS, STRIPS, StripPlace, TRIM_SHEET,
    layout_of,
};
use crate::eval::real_size_cm;
use crate::graph::{BoxId, PlugRef};
use crate::settings::SettingValue;
use crate::value::Value;
use words::{tr, trf};

/// How far (a share of the height) a strip's edge may lie from the
/// project's row before they disagree.
const SAME_PLACE: f32 = 0.002;

/// The Trim sheet boxes feeding Material out (not bypassed), each with
/// where its strips lie.
fn sheets(looking: &Looking<'_>) -> Vec<(BoxId, Layout)> {
    looking
        .upstream()
        .into_iter()
        .filter_map(|id| {
            let b = looking.graph.get(id)?;
            if b.kind != TRIM_SHEET {
                return None;
            }
            let mut plugged = [false; STRIPS];
            for (i, key) in STRIP_KEYS.iter().enumerate() {
                plugged[i] = looking.graph.wire_into(&PlugRef::new(id, *key)).is_some();
            }
            Some((
                id,
                layout_of(&b.settings, &plugged, looking.registry.trim_rows()),
            ))
        })
        .collect()
}

fn strip_names(strips: &[usize]) -> String {
    let names: Vec<String> = strips.iter().map(|s| trf!("Strip {}", s + 1)).collect();
    join(&names)
}

fn lay(id: BoxId, how: u32, words: String) -> Remedy {
    Remedy {
        words,
        action: Action::SetSetting {
            node: id,
            key: "lay".into(),
            value: SettingValue::Choice(how),
        },
    }
}

fn make_rows(id: BoxId) -> Remedy {
    Remedy {
        words: tr("Make these the project's trim rows").to_string(),
        action: Action::MakeTrimRows(id),
    }
}

/// Whether the strips lie where the project's rows are.
fn on_the_rows(layout: &Layout, rows: &crate::trim::TrimRows) -> bool {
    layout.places.len() == rows.len()
        && layout.places.iter().zip(rows.iter()).all(|(p, r)| {
            (p.from - r.from).abs() <= SAME_PLACE && (p.to - r.to).abs() <= SAME_PLACE
        })
}

/// Strips and the project's trim rows disagreeing.
pub(super) fn rows(looking: &Looking<'_>, out: &mut Vec<Warning>) {
    for (id, layout) in sheets(looking) {
        if layout.places.is_empty() && layout.past_rows.is_empty() {
            continue;
        }
        let name = looking.name(id);
        match layout.laid {
            Laid::NoRows => out.push(Warning {
                concern: Concern::NoTrimRows,
                words: trf!(
                    "{} lays its strips on the project's trim rows, but the project has none \
                     yet: the strips share the texture by their own heights for now, and the \
                     UV Editor's Put on trim row has no rows to put strips of a model on.",
                    name
                ),
                remedies: vec![make_rows(id)],
                boxes: vec![id],
            }),
            Laid::OnRows if !layout.past_rows.is_empty() => out.push(Warning {
                concern: Concern::PastTrimRows,
                words: trf!(
                    "{} {} a material but no trim row to lie on: the project has {} rows, so \
                     {} left off the sheet. Add rows in the UV Editor's Trim menu, or lay the \
                     strips by their own heights.",
                    strip_names(&layout.past_rows),
                    if layout.past_rows.len() == 1 {
                        tr("has")
                    } else {
                        tr("have")
                    },
                    layout.rows,
                    if layout.past_rows.len() == 1 {
                        tr("it is")
                    } else {
                        tr("they are")
                    }
                ),
                remedies: vec![lay(
                    id,
                    OWN_HEIGHTS,
                    tr("Lay the strips by their own heights").to_string(),
                )],
                boxes: vec![id],
            }),
            Laid::OwnHeights
                if !looking.registry.trim_rows().is_empty()
                    && !on_the_rows(&layout, looking.registry.trim_rows()) =>
            {
                out.push(Warning {
                    concern: Concern::RowsDiffer,
                    words: trf!(
                        "{} lays its strips by their own heights, and they don't match the \
                         project's {} trim rows: strips of a model put on a row in the UV \
                         Editor won't line up with a strip.",
                        name,
                        looking.registry.trim_rows().len()
                    ),
                    remedies: vec![
                        make_rows(id),
                        lay(
                            id,
                            ON_ROWS,
                            tr("Lay the strips on the project's rows").to_string(),
                        ),
                    ],
                    boxes: vec![id],
                });
            }
            _ => {}
        }
    }
}

/// Where a cut strip leaves a sliver of a row of its material's pattern
/// (a thin part of a plank or brick row at its edge), in rows of the
/// picture. A strip ending inside its only row just shows a narrower
/// piece, which the sheet's own edge finishes: that is fine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Cut {
    /// At its bottom edge: how far down to the next line (taller) and how
    /// many rows of sliver show (shorter by that much).
    pub bottom: Option<(usize, usize)>,
    /// At its top edge: how far down the first line is.
    pub top: Option<usize>,
}

/// Where strip rows `0..rows` of a material `n` tall with `lines` across
/// leave slivers; `None` when they don't.
pub(super) fn cut_of(lines: &[(usize, usize)], rows: usize, n: usize) -> Option<Cut> {
    if lines.is_empty() || n == 0 || rows == 0 {
        return None;
    }
    let near = (n / 128).max(1);
    // A sliver is under half the usual distance between lines.
    let sliver = n / lines.len() / 2;
    let below = rows % n;
    let above = (below + n - 1) % n;
    let down = measure::rows_to_line(lines, below, n, true);
    let on_line = measure::rows_to_line(lines, above, n, false) <= near;
    // The line above the bottom edge must lie inside the strip, below its
    // top edge: else the strip shows part of one piece.
    let inside: Vec<(usize, usize)> = lines
        .iter()
        .copied()
        .filter(|(first, _)| rows >= n || (*first > near && *first < rows))
        .collect();
    let up = measure::rows_to_line(&inside, above, n, false);
    let bottom =
        (down > near && !on_line && !inside.is_empty() && up < sliver).then_some((down, up));
    let first = measure::rows_to_line(lines, 0, n, true);
    let last = measure::rows_to_line(lines, n - 1, n, false);
    let top = (first.min(last) > near && first < sliver && first + near < rows).then_some(first);
    (bottom.is_some() || top.is_some()).then_some(Cut { bottom, top })
}

/// Cut strips cutting through a row of their pattern.
pub(super) fn cut_through(looking: &Looking<'_>, out: &mut Vec<Warning>) {
    for (id, layout) in sheets(looking) {
        for place in layout.places.iter().filter(|p| !p.shrink) {
            if let Some(warning) = cut_strip(looking, id, place) {
                out.push(warning);
            }
        }
    }
}

fn cut_strip(looking: &Looking<'_>, id: BoxId, place: &StripPlace) -> Option<Warning> {
    let into = PlugRef::new(id, STRIP_KEYS[place.strip]);
    let from = looking.graph.wire_into(&into)?.from.clone();
    let m = looking.output(&from).and_then(Value::material)?;
    let n = m.size() as usize;
    let look = measure::look(m);
    let lines = measure::lines_across(&m.height, &look)?;
    let rows = ((place.to - place.from) * n as f32).round().max(1.0) as usize;
    let cut = cut_of(&lines, rows, n)?;
    let cm_a_row = real_size_cm(looking.registry, looking.graph) / n as f32;
    let cm = |r: usize| length_words(r as f32 * cm_a_row);
    let strip = trf!("Strip {}", place.strip + 1);
    let material = material_name(looking, from.node);
    let words = match (cut.bottom, cut.top) {
        (Some((taller, shorter)), _) => trf!(
            "{} cuts its material ({}) through a row of its pattern at its bottom edge, so a \
             sliver of the next row, {} of it, shows there. Make the strip {} shorter to end on \
             the line between rows, or {} taller to show the whole row; or let it shrink its \
             material to fit.",
            strip,
            material,
            cm(shorter),
            cm(shorter),
            cm(taller)
        ),
        (None, Some(down)) => trf!(
            "{} starts with a sliver of a row of its material's pattern ({}): the first line \
             between rows is {} down. A Transform box before the strip, moving the material up \
             that much, starts it on a line; or let the strip shrink its material to fit.",
            strip,
            material,
            cm(down)
        ),
        (None, None) => return None,
    };
    let fit_key = format!("fit{}", place.strip + 1);
    Some(Warning {
        concern: Concern::CutThrough,
        words,
        remedies: vec![
            Remedy {
                words: trf!("Shrink {} to fit", strip),
                action: Action::SetSetting {
                    node: id,
                    key: fit_key,
                    value: SettingValue::Choice(SHRINK),
                },
            },
            Remedy {
                words: trf!("Look at {}", material),
                action: Action::LookAt(from.node),
            },
        ],
        boxes: vec![id, from.node],
    })
}

/// A strip's material by the comment frame round the box making it (the
/// starters name each strip's frame: "Planks"), else by the box's name.
fn material_name(looking: &Looking<'_>, id: BoxId) -> String {
    looking
        .graph
        .frames()
        .filter(|(f, _)| looking.graph.boxes_in(*f).contains(&id))
        .min_by(|(_, a), (_, b)| {
            let area = |f: &crate::graph::Frame| (f.max - f.min).x * (f.max - f.min).y;
            area(a).total_cmp(&area(b))
        })
        .map(|(_, f)| f.title.clone())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| looking.name(id))
}
