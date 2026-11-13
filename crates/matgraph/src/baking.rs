//! What an export should know about baking a graph into textures (Phase
//! 12, milestone 6, [decision 0262](../../../docs/decisions/0262-baked-export.md)).
//!
//! Every export bakes a graph: it is worked out once, at the export's
//! size, and written as ordinary pictures. Most graphs come out exactly as
//! the preview shows them. A few things can't travel that way, and the
//! export check says so in plain words, with what to do about it:
//!
//! - **Animated boxes** (Time, Scroll, Pulse and Flow, milestone 15,
//!   [decision 0277](../../../docs/decisions/0277-animated-boxes.md)) are
//!   baked at the graph's moment ([`Graph::moment`], chosen with the
//!   preview's player), so they stand still in the game until the engine
//!   translators come. [`BoxInfo::animated`] marks them.
//! - **Settings marked "change in the game"** are kept on the graph for
//!   the translators, and baked at their value until then.
//! - **From the model** boxes need a model to follow. A texture set
//!   exported on its own has none, so they give their stand-in of rounded
//!   bumps; given to a shape, the model's own maps are used.
//! - **Boxes from a newer app** can't run here at all.
//!
//! [`BoxInfo::animated`]: crate::BoxInfo::animated

use crate::{BoxId, Graph, Registry};
use words::{tr, trf};

/// One thing an export should say about baking a graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakeNote {
    /// Boxes that change over time: baked at the graph's moment.
    Animated(Vec<BoxId>),
    /// Boxes with settings marked "change in the game": baked at their
    /// value until the engine translators come.
    InGame(Vec<BoxId>),
    /// From the model boxes, exported without a model: their stand-in is
    /// baked.
    FollowsModel(Vec<BoxId>),
    /// Boxes of kinds this app doesn't know (from a newer app): the graph
    /// can't be baked until they are removed.
    Unknown(Vec<BoxId>),
}

impl BakeNote {
    /// The boxes it is about.
    pub fn boxes(&self) -> &[BoxId] {
        match self {
            Self::Animated(b) | Self::InGame(b) | Self::FollowsModel(b) | Self::Unknown(b) => b,
        }
    }

    /// Whether it stops the bake (rather than only changing what comes
    /// out).
    pub fn stops(&self) -> bool {
        matches!(self, Self::Unknown(_))
    }

    /// What it means, in plain words, with what to do about it; the boxes
    /// named by their plain names in `graph`.
    pub fn describe(&self, registry: &Registry, graph: &Graph) -> String {
        let names = names(registry, graph, self.boxes());
        match self {
            Self::Animated(_) => trf!(
                "{} moves over time. Exported textures can't move, so it is baked as it is \
                 at {} s, the moment chosen under the preview (Bake this moment): in the game \
                 it stands still until the engine translators come.",
                names,
                seconds(graph.moment())
            ),
            Self::InGame(_) => {
                let settings: Vec<String> = graph
                    .in_game(registry)
                    .iter()
                    .map(|s| trf!("{}'s {}", s.info.name.show(), s.spec.name.show()))
                    .collect();
                trf!(
                    "{} {} marked to change in the game. Until the engine translators come, \
                     the textures are baked with them as they are set now; the marks are kept \
                     for then.",
                    join(&settings),
                    if settings.len() == 1 {
                        tr("is")
                    } else {
                        tr("are")
                    }
                )
            }
            Self::FollowsModel(_) => trf!(
                "{} follows the model it is on, and a texture set on its own has no model: \
                 its stand-in of rounded bumps is baked instead. To bake it on a model, give \
                 the graph to the shape and export the model.",
                names
            ),
            Self::Unknown(_) => trf!(
                "{} was made by a newer version of the app, so it can't be worked out here. \
                 Remove it, or export from the newer app.",
                names
            ),
        }
    }
}

/// What an export of `graph` should say about baking it, in the order
/// worth reading: what stops it first. `on_model` is whether it is baked
/// on a model (given to a shape) rather than on its own.
pub fn bake_notes(registry: &Registry, graph: &Graph, on_model: bool) -> Vec<BakeNote> {
    let mut animated = Vec::new();
    let mut model = Vec::new();
    let mut unknown = Vec::new();
    let mut in_game = Vec::new();
    for (id, b) in graph.boxes() {
        if !b.in_game.is_empty() && registry.get(&b.kind).is_some() {
            in_game.push(id);
        }
        match registry.get(&b.kind) {
            None => unknown.push(id),
            Some(t) if t.info().animated => animated.push(id),
            Some(_) if !on_model && crate::model::is_from_model(registry, &b.kind) => {
                model.push(id);
            }
            Some(_) => {}
        }
    }
    [
        (!unknown.is_empty()).then_some(BakeNote::Unknown(unknown)),
        (!animated.is_empty()).then_some(BakeNote::Animated(animated)),
        (!in_game.is_empty()).then_some(BakeNote::InGame(in_game)),
        (!model.is_empty()).then_some(BakeNote::FollowsModel(model)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// `boxes` by their plain names, joined: "Curvature", "Curvature and
/// Occlusion", "Curvature, Occlusion and Position". Each name once.
fn names(registry: &Registry, graph: &Graph, boxes: &[BoxId]) -> String {
    let mut out: Vec<String> = Vec::new();
    for id in boxes {
        let name = graph
            .get(*id)
            .map(|b| {
                registry
                    .get(&b.kind)
                    .map_or_else(|| b.kind.clone(), |t| t.info().name.show())
            })
            .unwrap_or_default();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    }
    if out.is_empty() {
        return tr("A box").to_string();
    }
    join(&out)
}

/// `items` joined: "a", "a and b", "a, b and c".
fn join(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => trf!("{} and {}", rest.join(", "), last),
    }
}

/// A moment in seconds as shown: to a tenth, without a trailing ".0".
pub fn seconds(moment: f32) -> String {
    let tenths = (moment * 10.0).round() / 10.0;
    if tenths.fract() == 0.0 {
        format!("{tenths:.0}")
    } else {
        format!("{tenths:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GraphEdit;
    use glam::Vec2;

    fn graph(kinds: &[&str]) -> Graph {
        let registry = Registry::standard();
        let mut g = Graph::new("Test");
        for kind in kinds {
            g.apply(
                registry,
                &GraphEdit::AddBox {
                    kind: (*kind).into(),
                    at: Vec2::ZERO,
                },
            )
            .unwrap();
        }
        g
    }

    #[test]
    fn a_plain_graph_bakes_without_a_word() {
        let g = graph(&["noise", "material_from_height", "material_out"]);
        assert!(bake_notes(Registry::standard(), &g, false).is_empty());
        assert!(bake_notes(Registry::standard(), &g, true).is_empty());
    }

    #[test]
    fn boxes_from_the_model_are_named_only_off_the_model() {
        let registry = Registry::standard();
        let g = graph(&["model_curvature", "model_occlusion", "material_out"]);
        let notes = bake_notes(registry, &g, false);
        assert_eq!(
            notes,
            vec![BakeNote::FollowsModel(vec![BoxId(1), BoxId(2)])]
        );
        assert!(!notes[0].stops());
        let words = notes[0].describe(registry, &g);
        assert!(
            words.starts_with("Curvature and Occlusion follows"),
            "{words}"
        );
        assert!(words.contains("give the graph to the shape"), "{words}");
        // On a model they follow it: nothing to say.
        assert!(bake_notes(registry, &g, true).is_empty());
    }

    #[test]
    fn boxes_from_a_newer_app_stop_the_bake_and_come_first() {
        let registry = Registry::standard();
        let mut parts = graph(&["model_curvature", "noise"]).to_parts();
        parts.boxes[1].1.kind = "sparkle_from_the_future".into();
        let g = Graph::from_parts(parts, registry).unwrap();
        let notes = bake_notes(registry, &g, false);
        assert!(matches!(notes[0], BakeNote::Unknown(_)), "{notes:?}");
        assert!(notes[0].stops());
        assert!(
            notes[0]
                .describe(registry, &g)
                .starts_with("sparkle_from_the_future was made by a newer")
        );
    }

    #[test]
    fn animated_boxes_are_baked_at_the_graphs_moment() {
        let registry = Registry::standard();
        let mut g = graph(&["noise", "scroll", "pulse", "material_out"]);
        let notes = bake_notes(registry, &g, true);
        assert_eq!(notes, vec![BakeNote::Animated(vec![BoxId(2), BoxId(3)])]);
        assert!(!notes[0].stops());
        let words = notes[0].describe(registry, &g);
        assert!(words.starts_with("Scroll and Pulse moves"), "{words}");
        assert!(words.contains("at 0 s"), "{words}");
        g.apply(registry, &GraphEdit::SetMoment(2.5)).unwrap();
        assert!(notes[0].describe(registry, &g).contains("at 2.5 s"));
        // Each animated box says so; the others don't.
        let moving: Vec<&str> = registry
            .iter()
            .filter(|t| t.info().animated)
            .map(|t| t.info().id.as_ref())
            .collect();
        assert_eq!(moving, ["time", "scroll", "pulse", "flow"]);
    }

    #[test]
    fn settings_marked_for_the_game_are_listed() {
        let registry = Registry::standard();
        let mut g = graph(&["scroll", "noise"]);
        for (node, key) in [(BoxId(1), "speed"), (BoxId(1), "angle"), (BoxId(2), "scale")] {
            g.apply(
                registry,
                &GraphEdit::ChangeInGame {
                    node,
                    key: key.into(),
                    on: true,
                },
            )
            .unwrap();
        }
        let notes = bake_notes(registry, &g, false);
        assert_eq!(
            notes,
            vec![
                BakeNote::Animated(vec![BoxId(1)]),
                BakeNote::InGame(vec![BoxId(1), BoxId(2)])
            ]
        );
        let words = notes[1].describe(registry, &g);
        assert!(
            words.starts_with("Scroll's Direction, Scroll's Speed and Noise's Scale are marked"),
            "{words}"
        );
        assert!(!notes[1].stops());
    }

    #[test]
    fn moments_are_shown_to_a_tenth() {
        assert_eq!(seconds(0.0), "0");
        assert_eq!(seconds(2.5), "2.5");
        assert_eq!(seconds(2.04), "2");
        assert_eq!(seconds(12.36), "12.4");
    }

    #[test]
    fn names_are_joined_in_plain_words() {
        let registry = Registry::standard();
        let g = graph(&["model_curvature", "model_occlusion", "model_position"]);
        let all = [BoxId(1), BoxId(2), BoxId(3)];
        assert_eq!(
            names(registry, &g, &all),
            "Curvature, Occlusion and Position"
        );
        assert_eq!(names(registry, &g, &[]), "A box");
        assert_eq!(names(registry, &g, &[BoxId(1), BoxId(1)]), "Curvature");
    }
}
