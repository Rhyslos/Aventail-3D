//! Finding boxes and comment frames in a graph by name (milestone 17,
//! [decision 0279](../../../../docs/decisions/0279-find-and-minimap.md)).
//!
//! A search looks at what a person reads on the canvas: each box's name
//! (a standard box's plain name, or your own box's name), the choice a
//! box's choice settings are set to ("Star", "Multiply"), the names of
//! the pieces drawn on a Shape box's canvas, and each comment frame's
//! title and note. Every word typed must appear (in any order, any case)
//! in one of those. Matches come in reading order, left to right as the
//! graph flows, then top to bottom, so "next" moves along the graph.

use super::{BoxId, FrameId, Graph};
use crate::registry::Registry;
use crate::settings::{SettingKind, SettingValue};
use glam::Vec2;

/// The most matches a search lists: a graph can hold far more boxes than
/// anyone reads down a list.
pub const MAX_FOUND: usize = 200;

/// What a match is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FoundThing {
    /// A box.
    Box(BoxId),
    /// A comment frame.
    Frame(FrameId),
}

/// Why it matched: which of its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundBy {
    /// Its kind's plain name ("Bricks").
    Name,
    /// Its name as your own box.
    OwnBox,
    /// A choice setting set to the words: the setting's key.
    Choice,
    /// The name of a piece on its shape canvas.
    Piece,
    /// A comment frame's title.
    Title,
    /// A comment frame's note.
    Note,
}

/// One match.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The box or frame.
    pub thing: FoundThing,
    /// Which of its words matched.
    pub by: FoundBy,
    /// Its name as shown (a box's name, a frame's title).
    pub name: String,
    /// The words that matched, when they aren't its name (the choice, the
    /// piece's name, a line of the note); empty otherwise.
    pub detail: String,
    /// Its top left corner on the canvas.
    pub at: Vec2,
}

/// The words of `query`, lower case; none for a blank search.
fn words(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// Whether every one of `words` is in `text` (any case).
fn holds(text: &str, words: &[String]) -> bool {
    let text = text.to_lowercase();
    words.iter().all(|w| text.contains(w.as_str()))
}

/// The first line of `note` holding every word, cut short to be listed.
fn note_line(note: &str, words: &[String]) -> Option<String> {
    let line = note.lines().find(|l| holds(l, words))?;
    let line = line.trim();
    const MOST: usize = 60;
    Some(if line.chars().count() > MOST {
        let cut: String = line.chars().take(MOST).collect();
        format!("{}…", cut.trim_end())
    } else {
        line.to_string()
    })
}

impl Graph {
    /// The boxes and frames matching `query`, in reading order (left to
    /// right, then top to bottom), at most [`MAX_FOUND`]. A blank query
    /// finds nothing. Box names come from `registry` (the project's, so
    /// your own boxes are found by their names); a box of a kind it
    /// doesn't know is found by its kind's id.
    pub fn find(&self, registry: &Registry, query: &str) -> Vec<Found> {
        let words = words(query);
        if words.is_empty() {
            return Vec::new();
        }
        let mut found = Vec::new();
        for (id, b) in self.boxes() {
            let info = registry.get(&b.kind).map(|t| t.info());
            let name = info.map_or_else(|| b.kind.clone(), |i| i.name.show());
            let own = matches!(info.map(|i| i.group), Some(crate::registry::Group::Own));
            let mut hit = holds(&name, &words).then(|| {
                let by = if own { FoundBy::OwnBox } else { FoundBy::Name };
                (by, String::new())
            });
            if hit.is_none()
                && let Some(info) = info
            {
                hit = info.settings.iter().find_map(|spec| {
                    // A palette chosen by name (milestone 19) is found as
                    // a choice is.
                    if let (SettingKind::Palette { .. }, Some(SettingValue::Palette(name))) =
                        (&spec.kind, b.settings.get(&spec.key))
                    {
                        return (!name.is_empty() && holds(&name, &words))
                            .then(|| (FoundBy::Choice, format!("{}: {}", spec.name.show(), name)));
                    }
                    let SettingKind::Choice { options, standard } = &spec.kind else {
                        return None;
                    };
                    let index = match b.settings.get(&spec.key) {
                        Some(SettingValue::Choice(i)) => i,
                        _ => *standard,
                    };
                    let chosen = options.get(index as usize)?.show();
                    holds(&chosen, &words).then(|| {
                        let detail = format!("{}: {}", spec.name.show(), chosen);
                        (FoundBy::Choice, detail)
                    })
                });
            }
            if hit.is_none()
                && let Some(drawing) = &b.drawing
            {
                hit = drawing
                    .pieces()
                    .iter()
                    .find(|p| holds(&p.name, &words))
                    .map(|p| (FoundBy::Piece, p.name.clone()));
            }
            if let Some((by, detail)) = hit {
                found.push(Found {
                    thing: FoundThing::Box(id),
                    by,
                    name,
                    detail,
                    at: b.at,
                });
            }
        }
        for (id, f) in self.frames() {
            let hit = if holds(&f.title, &words) {
                Some((FoundBy::Title, String::new()))
            } else {
                note_line(&f.note, &words).map(|line| (FoundBy::Note, line))
            };
            if let Some((by, detail)) = hit {
                found.push(Found {
                    thing: FoundThing::Frame(id),
                    by,
                    name: f.title.clone(),
                    detail,
                    at: f.min,
                });
            }
        }
        found.sort_by(|a, b| {
            a.at.x
                .total_cmp(&b.at.x)
                .then(a.at.y.total_cmp(&b.at.y))
                .then(a.thing.cmp(&b.thing))
        });
        found.truncate(MAX_FOUND);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Frame, GraphEdit};
    use crate::own::{Proposal, fresh_id, registry_with};
    use std::sync::Arc;

    fn add(g: &mut Graph, registry: &Registry, kind: &str, x: f32, y: f32) -> BoxId {
        let id = BoxId(g.next_id());
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(x, y),
            },
        )
        .unwrap();
        id
    }

    fn things(found: &[Found]) -> Vec<FoundThing> {
        found.iter().map(|f| f.thing).collect()
    }

    #[test]
    fn boxes_are_found_by_name_in_any_case_and_word_order() {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        let bricks = add(&mut g, r, "bricks", 0.0, 0.0);
        let mat = add(&mut g, r, "material_from_height", 400.0, 0.0);
        add(&mut g, r, "noise", 200.0, 0.0);
        let found = g.find(r, "BRICKS");
        assert_eq!(things(&found), [FoundThing::Box(bricks)]);
        assert_eq!(found[0].by, FoundBy::Name);
        assert_eq!(found[0].name, "Bricks");
        assert!(found[0].detail.is_empty());
        // Every word, in any order, part words too.
        assert_eq!(things(&g.find(r, "height mat")), [FoundThing::Box(mat)]);
        assert!(g.find(r, "height bricks").is_empty());
    }

    #[test]
    fn a_blank_search_or_one_matching_nothing_finds_nothing() {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        add(&mut g, r, "noise", 0.0, 0.0);
        assert!(g.find(r, "").is_empty());
        assert!(g.find(r, "   \t ").is_empty());
        assert!(g.find(r, "zebra").is_empty());
        assert!(Graph::new("Empty").find(r, "noise").is_empty());
    }

    #[test]
    fn matches_come_left_to_right_then_top_to_bottom() {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        let right = add(&mut g, r, "noise", 600.0, 0.0);
        let low = add(&mut g, r, "noise", 0.0, 300.0);
        let high = add(&mut g, r, "noise", 0.0, -300.0);
        assert_eq!(
            things(&g.find(r, "noise")),
            [
                FoundThing::Box(high),
                FoundThing::Box(low),
                FoundThing::Box(right)
            ]
        );
    }

    #[test]
    fn frames_are_found_by_title_and_by_their_notes() {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        let mut moss = Frame::new("Moss", Vec2::new(500.0, 0.0), Vec2::new(900.0, 400.0));
        moss.note = "Green in the cracks.\nClouds decide where the moss grows thickest \
                     in the deep gaps between the stones."
            .into();
        g.apply(
            r,
            &GraphEdit::AddFrame(Frame::new("Stone shapes", Vec2::ZERO, Vec2::splat(400.0))),
        )
        .unwrap();
        g.apply(r, &GraphEdit::AddFrame(moss)).unwrap();
        let found = g.find(r, "stone");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].by, FoundBy::Title);
        assert_eq!(found[0].name, "Stone shapes");
        // The note's line, cut short to be listed.
        assert_eq!(found[1].by, FoundBy::Note);
        assert_eq!(found[1].name, "Moss");
        assert!(found[1].detail.starts_with("Clouds decide"));
        assert!(found[1].detail.ends_with('…'));
        assert!(found[1].detail.chars().count() <= 61);
        let cracks = g.find(r, "cracks");
        assert_eq!(cracks[0].detail, "Green in the cracks.");
    }

    #[test]
    fn a_box_is_found_by_the_choice_it_is_set_to() {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        let blend = add(&mut g, r, "blend", 0.0, 0.0);
        // The standard choice counts too.
        let found = g.find(r, "mix");
        assert!(found.iter().any(|f| f.thing == FoundThing::Box(blend)));
        g.apply(
            r,
            &GraphEdit::SetSetting {
                node: blend,
                key: "mode".into(),
                value: SettingValue::Choice(3),
            },
        )
        .unwrap();
        let found = g.find(r, "multiply");
        assert_eq!(things(&found), [FoundThing::Box(blend)]);
        assert_eq!(found[0].by, FoundBy::Choice);
        assert_eq!(found[0].detail, "Mode: Multiply");
        assert!(g.find(r, "darker").is_empty());
        // A Palette map by the palette it names.
        let map = BoxId(g.next_id());
        for edit in [
            GraphEdit::AddBox {
                kind: "palette_map".into(),
                at: glam::Vec2::new(900.0, 900.0),
            },
            GraphEdit::SetSetting {
                node: map,
                key: "palette".into(),
                value: SettingValue::Palette("Mossy greens".into()),
            },
        ] {
            g.apply(r, &edit).unwrap();
        }
        let found = g.find(r, "mossy");
        assert_eq!(things(&found), [FoundThing::Box(map)]);
        assert_eq!(found[0].detail, "Palette: Mossy greens");
    }

    #[test]
    fn a_shape_box_is_found_by_its_pieces_names() {
        let r = Registry::standard();
        let mut g = Graph::new("Leaves");
        let shape = add(&mut g, r, "drawn_shape", 0.0, 0.0);
        let square = shapes::Path::polygon([
            Vec2::new(0.25, 0.25),
            Vec2::new(0.75, 0.25),
            Vec2::new(0.75, 0.75),
            Vec2::new(0.25, 0.75),
        ]);
        let drawing = shapes::Drawing::new()
            .with_piece("Oak leaf", square, shapes::PieceKind::Add)
            .unwrap();
        g.apply(
            r,
            &GraphEdit::SetDrawing {
                node: shape,
                drawing: Arc::new(drawing),
            },
        )
        .unwrap();
        let found = g.find(r, "oak");
        assert_eq!(things(&found), [FoundThing::Box(shape)]);
        assert_eq!(
            (found[0].by, found[0].detail.as_str()),
            (FoundBy::Piece, "Oak leaf")
        );
    }

    #[test]
    fn your_own_boxes_are_found_by_their_names() {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        let noise = add(&mut g, r, "noise", 0.0, 0.0);
        let levels = add(&mut g, r, "levels", 200.0, 0.0);
        g.apply(
            r,
            &GraphEdit::Connect {
                from: super::super::PlugRef::new(noise, "height"),
                to: super::super::PlugRef::new(levels, "in"),
            },
        )
        .unwrap();
        let mut proposal = Proposal::of(&g, r, &[noise, levels].into()).unwrap();
        proposal.name = "Rocky ground".into();
        let id = fresh_id("Rocky ground", |_| false);
        let (own, edits) = proposal.make(&g, r, id, None).unwrap();
        let registry = registry_with(&[Arc::new(own)]);
        for edit in &edits {
            g.apply(&registry, edit).unwrap();
        }
        let found = g.find(&registry, "rocky");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].by, FoundBy::OwnBox);
        assert_eq!(found[0].name, "Rocky ground");
        // The boxes inside aren't in this graph any more.
        assert!(g.find(&registry, "levels").is_empty());
        // Without the own box known, it is found by its kind's id.
        let unknown = g.find(r, "own:");
        assert_eq!(unknown.len(), 1);
    }

    #[test]
    fn at_most_max_found_are_listed() {
        let r = Registry::standard();
        let mut g = Graph::new("Many");
        for i in 0..(MAX_FOUND + 20) {
            add(&mut g, r, "noise", i as f32 * 10.0, 0.0);
        }
        assert_eq!(g.find(r, "noise").len(), MAX_FOUND);
    }
}
