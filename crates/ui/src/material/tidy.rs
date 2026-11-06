//! The app keeping a graph's layout tidy, left to right (the plan's
//! answer to Substance Designer's "spider web" of wires; decision 0258).
//!
//! After every move, wire and new box, the tidy layout:
//!
//! 1. snaps each box into the nearest column;
//! 2. pushes a box right of everything feeding it, so a wire never runs
//!    backwards (what a box feeds moves along with it);
//! 3. snaps each box's height to a small step, and pushes a box down when
//!    it would sit on the one above it in its column;
//! 4. grows a comment frame when one of its boxes was pushed out of it,
//!    so a frame's boxes stay together.
//!
//! You can still put a box wherever you like: the app tidies around it.
//! The result is a list of edits, made in the same undo step as the
//! change that called for them.

use super::look::{COLUMN, FRAME_MARGIN, FRAME_TITLE, GAP, STEP, size_of};
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, Registry};
use std::collections::BTreeMap;

/// The farthest from the middle the tidy layout puts a box: inside what a
/// graph allows, with room to spare.
const FARTHEST: f32 = matgraph::graph::MAX_PLACE * 0.99;

/// The edits that tidy `graph` (none when it is tidy already).
pub(crate) fn tidy(graph: &Graph, registry: &Registry) -> Vec<GraphEdit> {
    let placed = places(graph, registry);
    let mut edits = Vec::new();
    for (id, at) in &placed {
        let Some(b) = graph.get(*id) else { continue };
        let by = *at - b.at;
        if by.length_squared() > 1e-6 {
            edits.push(GraphEdit::MoveBoxes { ids: vec![*id], by });
        }
    }
    edits.extend(frames_kept_together(graph, registry, &placed));
    edits
}

/// Where each box goes.
fn places(graph: &Graph, registry: &Registry) -> BTreeMap<BoxId, Vec2> {
    let snap = |v: f32, step: f32| (v / step).round() * step;
    let mut column: BTreeMap<BoxId, i64> = graph
        .boxes()
        .map(|(id, b)| {
            (
                id,
                (b.at.x.clamp(-FARTHEST, FARTHEST) / COLUMN).round() as i64,
            )
        })
        .collect();
    // Each box after what feeds it, so a push carries on down the line.
    for id in graph.order(&[]) {
        let fed_from = graph
            .sources_of(id)
            .filter_map(|s| column.get(&s).copied())
            .max();
        if let (Some(from), Some(here)) = (fed_from, column.get_mut(&id)) {
            *here = (*here).max(from + 1);
        }
    }
    let mut placed: BTreeMap<BoxId, Vec2> = graph
        .boxes()
        .map(|(id, b)| {
            let x = (column[&id] as f32 * COLUMN).clamp(-FARTHEST, FARTHEST);
            (
                id,
                Vec2::new(x, snap(b.at.y.clamp(-FARTHEST, FARTHEST), STEP)),
            )
        })
        .collect();
    // In each column, top to bottom: a box sitting on the one above is
    // pushed down below it.
    let mut by_column: BTreeMap<i64, Vec<BoxId>> = BTreeMap::new();
    for (id, c) in &column {
        by_column.entry(*c).or_default().push(*id);
    }
    for ids in by_column.values_mut() {
        ids.sort_by(|a, b| placed[a].y.total_cmp(&placed[b].y).then(a.cmp(b)));
        let mut lowest: Option<f32> = None;
        for id in ids.iter() {
            let Some(b) = graph.get(*id) else { continue };
            let height = size_of(registry, &b.kind).y;
            let Some(at) = placed.get_mut(id) else {
                continue;
            };
            if let Some(free) = lowest
                && at.y < free
            {
                at.y = (free / STEP).ceil() * STEP;
            }
            at.y = at.y.min(FARTHEST);
            lowest = Some(at.y + height + GAP);
        }
    }
    placed
}

/// Frames grown to hold again the boxes the tidy layout pushed out.
fn frames_kept_together(
    graph: &Graph,
    registry: &Registry,
    placed: &BTreeMap<BoxId, Vec2>,
) -> Vec<GraphEdit> {
    let mut edits = Vec::new();
    for (id, frame) in graph.frames() {
        let mut grown = frame.clone();
        for inside in graph.boxes_in(id) {
            let (Some(at), Some(b)) = (placed.get(&inside), graph.get(inside)) else {
                continue;
            };
            let far = *at + size_of(registry, &b.kind);
            grown.min = grown
                .min
                .min(*at - Vec2::new(FRAME_MARGIN, FRAME_MARGIN + FRAME_TITLE));
            grown.max = grown.max.max(far + Vec2::splat(FRAME_MARGIN));
        }
        let fits =
            grown.min.abs().max_element() <= FARTHEST && grown.max.abs().max_element() <= FARTHEST;
        if grown != *frame && fits {
            edits.push(GraphEdit::ChangeFrame { id, frame: grown });
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::{Frame, PlugRef};

    fn with(graph: &mut Graph, edits: &[GraphEdit]) {
        for edit in edits {
            graph.apply(Registry::standard(), edit).unwrap();
        }
    }

    fn add(graph: &mut Graph, kind: &str, x: f32, y: f32) -> BoxId {
        let id = BoxId(graph.next_id());
        with(
            graph,
            &[GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(x, y),
            }],
        );
        id
    }

    fn tidied(graph: &Graph) -> Graph {
        let mut out = graph.clone();
        with(&mut out, &tidy(graph, Registry::standard()));
        out
    }

    #[test]
    fn boxes_snap_into_columns() {
        let mut g = Graph::new("Stone");
        let a = add(&mut g, "noise", 230.0, 13.0);
        let t = tidied(&g);
        assert_eq!(t.get(a).unwrap().at, Vec2::new(COLUMN, 10.0));
    }

    #[test]
    fn wires_never_run_backwards() {
        let mut g = Graph::new("Stone");
        let noise = add(&mut g, "noise", 2.0 * COLUMN, 0.0);
        let levels = add(&mut g, "levels", 0.0, 0.0);
        let out = add(&mut g, "material_from_height", COLUMN, 300.0);
        with(
            &mut g,
            &[
                GraphEdit::Connect {
                    from: PlugRef::new(noise, "height"),
                    to: PlugRef::new(levels, "in"),
                },
                GraphEdit::Connect {
                    from: PlugRef::new(levels, "out"),
                    to: PlugRef::new(out, "height"),
                },
            ],
        );
        let t = tidied(&g);
        let x = |id| t.get(id).unwrap().at.x;
        assert!(x(levels) > x(noise), "pushed right of its source");
        assert!(x(out) > x(levels), "and what it feeds goes along");
        for wire in t.wires() {
            assert!(x(wire.to.node) > x(wire.from.node));
        }
    }

    #[test]
    fn boxes_sitting_on_each_other_are_pushed_apart() {
        let mut g = Graph::new("Stone");
        let a = add(&mut g, "noise", 0.0, 0.0);
        let b = add(&mut g, "tiles", 10.0, 40.0);
        let t = tidied(&g);
        let height = size_of(Registry::standard(), "noise").y;
        assert_eq!(t.get(a).unwrap().at.y, 0.0, "the top one stays");
        assert!(t.get(b).unwrap().at.y >= height + GAP);
    }

    #[test]
    fn a_tidy_graph_stays_as_it_is() {
        let mut g = Graph::new("Stone");
        let a = add(&mut g, "noise", 50.0, 7.0);
        let b = add(&mut g, "levels", -400.0, 3.0);
        add(&mut g, "tiles", 20.0, 20.0);
        with(
            &mut g,
            &[GraphEdit::Connect {
                from: PlugRef::new(a, "height"),
                to: PlugRef::new(b, "in"),
            }],
        );
        let once = tidied(&g);
        assert!(tidy(&once, Registry::standard()).is_empty());
        assert!(tidy(&Graph::new("Empty"), Registry::standard()).is_empty());
    }

    #[test]
    fn a_frame_grows_to_keep_its_boxes() {
        let mut g = Graph::new("Stone");
        let noise = add(&mut g, "noise", COLUMN, 0.0);
        let levels = add(&mut g, "levels", 0.0, 0.0);
        with(
            &mut g,
            &[
                GraphEdit::AddFrame(Frame::new(
                    "Shapes",
                    Vec2::new(-30.0, -50.0),
                    Vec2::new(200.0, 300.0),
                )),
                GraphEdit::Connect {
                    from: PlugRef::new(noise, "height"),
                    to: PlugRef::new(levels, "in"),
                },
            ],
        );
        let (frame_id, _) = g.frames().next().unwrap();
        assert_eq!(g.boxes_in(frame_id), vec![levels]);
        let t = tidied(&g);
        assert_eq!(t.get(levels).unwrap().at.x, 2.0 * COLUMN);
        assert!(t.boxes_in(frame_id).contains(&levels), "still inside");
    }

    #[test]
    fn far_away_boxes_stay_inside_what_a_graph_allows() {
        let mut g = Graph::new("Far");
        let a = add(&mut g, "noise", FARTHEST, FARTHEST);
        let b = add(&mut g, "levels", FARTHEST, FARTHEST);
        with(
            &mut g,
            &[GraphEdit::Connect {
                from: PlugRef::new(a, "height"),
                to: PlugRef::new(b, "in"),
            }],
        );
        // Every edit can be made (none is refused as out of place).
        let _ = tidied(&g);
        // An unknown kind (from a newer app) is tidied like any other.
        let mut parts = g.to_parts();
        parts.boxes[0].1.kind = "from_the_future".into();
        let g = Graph::from_parts(parts, Registry::standard()).unwrap();
        let _ = tidied(&g);
    }
}
