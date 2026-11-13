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
//!    so a frame's boxes stay together;
//! 5. moves each wire's reroute dots along with the boxes at its ends
//!    (decision 0278), and keeps them between the two ends, each no
//!    further left than the one before, so the wire never runs
//!    backwards.
//!
//! You can still put a box wherever you like: the app tidies around it.
//! The result is a list of edits, made in the same undo step as the
//! change that called for them.

use super::look::{BOX_WIDTH, COLUMN, FRAME_MARGIN, FRAME_TITLE, GAP, STEP, size_of};
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
    edits.extend(dots_kept_between(graph, &placed));
    edits
}

/// How far in from a wire's ends its dots are kept.
const DOT_MARGIN: f32 = 10.0;

/// Each wire's dots moved with its boxes and kept between its ends, left
/// to right.
fn dots_kept_between(graph: &Graph, placed: &BTreeMap<BoxId, Vec2>) -> Vec<GraphEdit> {
    let mut edits = Vec::new();
    for (to, points) in graph.all_bends() {
        let Some(wire) = graph.wire_into(to) else {
            continue;
        };
        let ends = |id: BoxId| Some((graph.get(id)?.at, *placed.get(&id)?));
        let (Some((from_was, from_now)), Some((to_was, to_now))) =
            (ends(wire.from.node), ends(wire.to.node))
        else {
            continue;
        };
        let tidied = dots_between(points, (from_was, from_now), (to_was, to_now));
        let moved = tidied
            .iter()
            .zip(points)
            .any(|(a, b)| a.distance_squared(*b) > 1e-6);
        if moved {
            edits.push(GraphEdit::SetBends {
                to: to.clone(),
                points: tidied,
            });
        }
    }
    edits
}

/// `points`, the dots of a wire from a box that was at `from.0` and goes
/// to `from.1`, into one that was at `to.0` and goes to `to.1` (corners):
/// each dot moved as much as the end it is nearer, by its share of the
/// way across, then kept between the ends' plugs, left to right.
pub(crate) fn dots_between(points: &[Vec2], from: (Vec2, Vec2), to: (Vec2, Vec2)) -> Vec<Vec2> {
    let out_x = |corner: Vec2| corner.x + BOX_WIDTH;
    let (start, end) = (out_x(from.0), to.0.x);
    let (moved_from, moved_to) = (from.1 - from.0, to.1 - to.0);
    let lo = out_x(from.1) + DOT_MARGIN;
    let hi = (to.1.x - DOT_MARGIN).max(lo);
    let mut least = lo;
    points
        .iter()
        .map(|p| {
            let share = if end - start > 1e-3 {
                ((p.x - start) / (end - start)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let mut q = *p + moved_from.lerp(moved_to, share);
            q.x = q.x.clamp(least, hi);
            q.y = q.y.clamp(-FARTHEST, FARTHEST);
            least = q.x;
            q
        })
        .collect()
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

    /// Noise → Levels with dots at `points` on the wire, Levels two
    /// columns on.
    fn bent(points: Vec<Vec2>) -> (Graph, PlugRef) {
        let mut g = Graph::new("Bent");
        let noise = add(&mut g, "noise", 0.0, 0.0);
        let levels = add(&mut g, "levels", 2.0 * COLUMN, 0.0);
        let to = PlugRef::new(levels, "in");
        with(
            &mut g,
            &[
                GraphEdit::Connect {
                    from: PlugRef::new(noise, "height"),
                    to: to.clone(),
                },
                GraphEdit::SetBends {
                    to: to.clone(),
                    points,
                },
            ],
        );
        (g, to)
    }

    #[test]
    fn dots_between_the_ends_are_left_as_they_are() {
        let points = vec![Vec2::new(200.0, 300.0), Vec2::new(300.0, 300.0)];
        let (g, _) = bent(points);
        assert!(tidy(&g, Registry::standard()).is_empty());
    }

    #[test]
    fn dots_never_make_a_wire_run_backwards() {
        // One behind its output, one past its input, and two out of order.
        let points = vec![
            Vec2::new(-300.0, 50.0),
            Vec2::new(380.0, 60.0),
            Vec2::new(250.0, 70.0),
            Vec2::new(900.0, 80.0),
        ];
        let (g, to) = bent(points);
        let t = tidied(&g);
        let xs: Vec<f32> = t.bends(&to).iter().map(|p| p.x).collect();
        let start = t.get(BoxId(1)).unwrap().at.x + BOX_WIDTH;
        let end = t.get(BoxId(2)).unwrap().at.x;
        assert!(xs.windows(2).all(|w| w[0] <= w[1]), "{xs:?}");
        assert!(xs.iter().all(|x| *x > start && *x < end), "{xs:?}");
        // Their heights are kept.
        let ys: Vec<f32> = t.bends(&to).iter().map(|p| p.y).collect();
        assert_eq!(ys, [50.0, 60.0, 70.0, 80.0]);
        // And tidying again changes nothing.
        assert!(tidy(&t, Registry::standard()).is_empty());
    }

    #[test]
    fn dots_move_with_the_boxes_tidied() {
        // Blend sits in Clouds' column, which feeds it: the tidy layout
        // pushes it a column right. The wire Noise → Blend has a dot near
        // each end: the one by Blend goes most of the way with it, the
        // one by Noise hardly moves.
        let mut g = Graph::new("Pushed");
        let noise = add(&mut g, "noise", 0.0, 0.0);
        let clouds = add(&mut g, "clouds", COLUMN, 400.0);
        let blend = add(&mut g, "blend", COLUMN, 0.0);
        let to = PlugRef::new(blend, "bottom");
        let near_noise = Vec2::new(BOX_WIDTH + 10.0, 40.0);
        let near_blend = Vec2::new(COLUMN - 10.0, 40.0);
        with(
            &mut g,
            &[
                GraphEdit::Connect {
                    from: PlugRef::new(noise, "height"),
                    to: to.clone(),
                },
                GraphEdit::Connect {
                    from: PlugRef::new(clouds, "height"),
                    to: PlugRef::new(blend, "top"),
                },
                GraphEdit::SetBends {
                    to: to.clone(),
                    points: vec![near_noise, near_blend],
                },
            ],
        );
        let t = tidied(&g);
        assert_eq!(t.get(blend).unwrap().at.x, 2.0 * COLUMN);
        let dots = t.bends(&to);
        let moved: Vec<f32> = dots
            .iter()
            .zip([near_noise, near_blend])
            .map(|(a, b)| a.x - b.x)
            .collect();
        assert!(moved[0] < 0.25 * COLUMN, "{moved:?}");
        assert!(moved[1] > 0.75 * COLUMN, "{moved:?}");
        assert!(dots.iter().all(|p| p.y == 40.0));
        assert!(tidy(&t, Registry::standard()).is_empty());
    }

    #[test]
    fn dots_between_stays_in_order_whatever_it_is_given() {
        let corner = |x: f32| (Vec2::new(x, 0.0), Vec2::new(x, 0.0));
        let mut state = 7u32;
        for _ in 0..200 {
            let mut next = || {
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (state >> 8) as f32 / (1 << 24) as f32
            };
            let points: Vec<Vec2> = (0..5)
                .map(|_| Vec2::new(next() * 2000.0 - 1000.0, next() * 100.0))
                .collect();
            let out = dots_between(&points, corner(0.0), corner(next() * 800.0));
            assert_eq!(out.len(), points.len());
            assert!(out.windows(2).all(|w| w[0].x <= w[1].x));
            assert!(out.iter().all(|p| p.is_finite()));
        }
    }
}
