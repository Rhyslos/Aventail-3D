//! Reroute dots (milestone 16, decision 0278): points a wire bends
//! through, to steer long wires round boxes.
//!
//! The plan's controls: **click a wire** to add a dot where you clicked;
//! **drag a dot** to move it; **Delete** removes the selected dot (click a
//! dot to select it). A wire bends through its dots, leaving and reaching
//! each level, so it reads left to right. The tidy layout moves dots with
//! the boxes at their ends and keeps them between the two, so a wire
//! never runs backwards (`tidy`). Each change is one undo step.

use super::MaterialEditor;
use super::drawing::wire_shape;
use egui::epaint::CubicBezierShape;
use egui::{Color32, Pos2};
use glam::Vec2;
use matgraph::{Graph, GraphEdit, PlugRef, ValueKind};
use scene::Document;

/// How close (on screen) the pointer must be to a wire to click it.
pub(crate) const WIRE_REACH: f32 = 6.0;
/// How close (on screen) the pointer must be to a dot to take it.
pub(crate) const DOT_REACH: f32 = 8.0;
/// A dot's radius on screen at full zoom.
pub(crate) const DOT_RADIUS: f32 = 5.0;
/// How many straight pieces a wire's curve is checked in, per stretch.
const PIECES: usize = 24;

/// A wire as drawn this frame: the input it goes into, what it carries,
/// and its points on screen (its output, its dots, its input).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WirePlace {
    pub(crate) to: PlugRef,
    pub(crate) kind: ValueKind,
    pub(crate) points: Vec<Pos2>,
}

impl WirePlace {
    /// Its dots on screen.
    pub(crate) fn dots(&self) -> &[Pos2] {
        let n = self.points.len();
        if n < 2 { &[] } else { &self.points[1..n - 1] }
    }

    /// Its curves, one per stretch between points.
    pub(crate) fn curves(&self, zoom: f32, colour: Color32) -> Vec<CubicBezierShape> {
        self.points
            .windows(2)
            .map(|w| wire_shape(w[0], w[1], zoom, colour))
            .collect()
    }

    /// The stretch of the wire under the screen point `at` (the index of
    /// the dot a click there would add) and how far away it is, if it is
    /// within reach.
    pub(crate) fn near(&self, at: Pos2, zoom: f32) -> Option<(usize, f32)> {
        self.curves(zoom, Color32::WHITE)
            .iter()
            .enumerate()
            .filter_map(|(i, curve)| {
                let points: Vec<Pos2> = (0..=PIECES)
                    .map(|k| curve.sample(k as f32 / PIECES as f32))
                    .collect();
                let d = points
                    .windows(2)
                    .map(|w| distance_to_line(at, w[0], w[1]))
                    .fold(f32::INFINITY, f32::min);
                (d <= WIRE_REACH).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// The distance from `p` to the straight line from `a` to `b`.
fn distance_to_line(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let length = ab.length_sq();
    let t = if length > 0.0 {
        ((p - a).dot(ab) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// The dots of the wire into `to` in `graph`, with `at` put in as dot
/// number `index`.
pub(crate) fn with_dot(graph: &Graph, to: &PlugRef, index: usize, at: Vec2) -> Vec<Vec2> {
    let mut points = graph.bends(to).to_vec();
    points.insert(index.min(points.len()), at);
    points
}

impl MaterialEditor {
    /// A click on the wire into `to`: a dot where it was clicked (`at`,
    /// on the canvas), as dot number `index`; it is selected.
    pub(crate) fn add_dot(
        &mut self,
        document: &mut Document,
        to: &PlugRef,
        index: usize,
        at: Vec2,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let points = with_dot(&graph, to, index, at);
        let edit = GraphEdit::SetBends {
            to: to.clone(),
            points,
        };
        match self.run(
            document,
            vec![edit],
            crate::text::tr("Add a reroute dot"),
            None,
            false,
        ) {
            Ok(()) => {
                self.canvas.selected.clear();
                self.canvas.frame = None;
                self.canvas.dot = Some((to.clone(), index));
                *note = Some(
                    crate::text::tr(
                        "Added a reroute dot: drag it to steer the wire; Delete removes it.",
                    )
                    .to_string(),
                );
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// Moves dot `index` of the wire into `to` to `at` (on the canvas),
    /// merged into the undo step `key`.
    pub(crate) fn move_dot(
        &mut self,
        document: &mut Document,
        to: &PlugRef,
        index: usize,
        at: Vec2,
        key: u64,
    ) -> bool {
        let Some(graph) = self.current(document).cloned() else {
            return false;
        };
        let mut points = graph.bends(to).to_vec();
        let Some(dot) = points.get_mut(index) else {
            return false;
        };
        *dot = at;
        let edit = GraphEdit::SetBends {
            to: to.clone(),
            points,
        };
        self.run(
            document,
            vec![edit],
            crate::text::tr("Move a reroute dot"),
            Some(key),
            false,
        )
        .is_ok()
    }

    /// Removes the selected dot (Delete or Backspace with a dot selected).
    pub(crate) fn remove_dot(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some((to, index)) = self.canvas.dot.take() else {
            return;
        };
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let mut points = graph.bends(&to).to_vec();
        if index >= points.len() {
            return;
        }
        points.remove(index);
        let edit = GraphEdit::SetBends { to, points };
        let label = crate::text::tr("Remove a reroute dot");
        if let Err(why) = self.run(document, vec![edit], label, None, false) {
            *note = Some(why.reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bent() -> WirePlace {
        WirePlace {
            to: PlugRef::new(matgraph::BoxId(2), "in"),
            kind: ValueKind::Height,
            points: vec![
                Pos2::new(0.0, 0.0),
                Pos2::new(100.0, 100.0),
                Pos2::new(200.0, 0.0),
            ],
        }
    }

    #[test]
    fn a_wire_is_found_near_its_curves_and_says_which_stretch() {
        let w = bent();
        assert_eq!(w.dots(), &[Pos2::new(100.0, 100.0)]);
        assert_eq!(w.curves(1.0, Color32::WHITE).len(), 2);
        // Its ends and its dot are on it.
        assert_eq!(w.near(Pos2::new(1.0, 0.0), 1.0).map(|n| n.0), Some(0));
        assert_eq!(w.near(Pos2::new(199.0, 0.0), 1.0).map(|n| n.0), Some(1));
        // Halfway along each stretch.
        let first = w.curves(1.0, Color32::WHITE)[0].sample(0.5);
        assert_eq!(w.near(first, 1.0).map(|n| n.0), Some(0));
        // Far away: not on it.
        assert_eq!(w.near(Pos2::new(100.0, -80.0), 1.0), None);
        // A straight wire has no dots and one stretch.
        let straight = WirePlace {
            points: vec![Pos2::ZERO, Pos2::new(50.0, 0.0)],
            ..bent()
        };
        assert!(straight.dots().is_empty());
        assert_eq!(
            straight.near(Pos2::new(25.0, 2.0), 1.0).map(|n| n.0),
            Some(0)
        );
    }

    #[test]
    fn distance_to_a_line_is_to_its_nearest_point() {
        let (a, b) = (Pos2::ZERO, Pos2::new(10.0, 0.0));
        assert_eq!(distance_to_line(Pos2::new(5.0, 3.0), a, b), 3.0);
        assert_eq!(distance_to_line(Pos2::new(-4.0, 3.0), a, b), 5.0);
        assert_eq!(distance_to_line(Pos2::new(1.0, 1.0), a, a), 2f32.sqrt());
    }
}
