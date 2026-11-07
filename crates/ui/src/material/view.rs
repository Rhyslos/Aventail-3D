//! Where the node canvas is looking: how far it is panned and how far
//! zoomed. Middle drag pans and the wheel zooms round the pointer, as in
//! the UV Editor; Z frames the selected boxes (decision 0258).

use egui::{Pos2, Rect};
use glam::Vec2;

/// The closest the canvas zooms in.
pub(crate) const MOST_ZOOM: f32 = 2.5;
/// The furthest it zooms out: a big graph then reads as its frames.
pub(crate) const LEAST_ZOOM: f32 = 0.15;
/// How much one notch of the wheel (50 points of scrolling) zooms.
const WHEEL_STEP: f32 = 1.12;
/// The room framing leaves round what it frames, on screen.
const FRAME_ROOM: f32 = 40.0;

/// The canvas's pan and zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct View {
    /// Where the canvas's origin is on screen, from the panel's top left.
    pub(crate) pan: Vec2,
    /// Screen points per canvas unit.
    pub(crate) zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            pan: Vec2::new(40.0, 60.0),
            zoom: 1.0,
        }
    }
}

impl View {
    /// Canvas point `p` on screen, in the panel `rect`.
    pub(crate) fn on_screen(&self, rect: Rect, p: Vec2) -> Pos2 {
        let s = Vec2::new(rect.min.x, rect.min.y) + self.pan + p * self.zoom;
        Pos2::new(s.x, s.y)
    }

    /// Screen point `s` on the canvas.
    pub(crate) fn on_canvas(&self, rect: Rect, s: Pos2) -> Vec2 {
        (Vec2::new(s.x - rect.min.x, s.y - rect.min.y) - self.pan) / self.zoom
    }

    /// A rectangle on the canvas (two corners) on screen.
    pub(crate) fn rect_on_screen(&self, rect: Rect, min: Vec2, max: Vec2) -> Rect {
        Rect::from_min_max(self.on_screen(rect, min), self.on_screen(rect, max))
    }

    /// Pans by `by` screen points.
    pub(crate) fn pan_by(&mut self, by: egui::Vec2) {
        self.pan += Vec2::new(by.x, by.y);
    }

    /// Zooms by `scroll` points of wheel, keeping the canvas point under
    /// `around` where it is.
    pub(crate) fn wheel(&mut self, rect: Rect, around: Pos2, scroll: f32) {
        let factor = WHEEL_STEP.powf((scroll / 50.0).clamp(-40.0, 40.0));
        self.zoom_by(rect, around, factor);
    }

    /// Zooms by `factor` round the screen point `around`.
    pub(crate) fn zoom_by(&mut self, rect: Rect, around: Pos2, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let fixed = self.on_canvas(rect, around);
        self.zoom = (self.zoom * factor).clamp(LEAST_ZOOM, MOST_ZOOM);
        let now = self.on_screen(rect, fixed);
        self.pan += Vec2::new(around.x - now.x, around.y - now.y);
    }

    /// Shows the canvas rectangle from `min` to `max` whole in `rect`,
    /// no closer than full size.
    pub(crate) fn frame(&mut self, rect: Rect, min: Vec2, max: Vec2) {
        let size = (max - min).max(Vec2::splat(1.0));
        let room = Vec2::new(
            (rect.width() - 2.0 * FRAME_ROOM).max(40.0),
            (rect.height() - 2.0 * FRAME_ROOM).max(40.0),
        );
        let zoom = (room / size).min_element().clamp(LEAST_ZOOM, 1.0);
        self.zoom = zoom;
        let middle = (min + max) / 2.0;
        let centre = Vec2::new(rect.width(), rect.height()) / 2.0;
        self.pan = centre - middle * zoom;
    }

    /// Shows the canvas rectangle from `min` to `max` whole if that is
    /// close enough to read the frames' notes; otherwise starts at its
    /// top left at [`READING_ZOOM`], to be read along (a starter graph,
    /// decision 0264).
    pub(crate) fn frame_readable(&mut self, rect: Rect, min: Vec2, max: Vec2) {
        self.frame(rect, min, max);
        if self.zoom < READING_ZOOM {
            self.zoom = READING_ZOOM;
            self.pan = Vec2::splat(FRAME_ROOM) - min * READING_ZOOM;
        }
    }
}

/// The zoom a graph opened to be read along is shown at, at least.
pub(crate) const READING_ZOOM: f32 = 0.8;

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(800.0, 600.0))
    }

    #[test]
    fn screen_and_canvas_points_round_trip() {
        let view = View {
            pan: Vec2::new(13.0, -7.0),
            zoom: 0.6,
        };
        let p = Vec2::new(250.0, -40.0);
        let back = view.on_canvas(panel(), view.on_screen(panel(), p));
        assert!((back - p).length() < 1e-3);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        let mut view = View::default();
        let around = Pos2::new(400.0, 300.0);
        let before = view.on_canvas(panel(), around);
        view.wheel(panel(), around, 200.0);
        assert!(view.zoom > 1.0);
        let after = view.on_canvas(panel(), around);
        assert!((after - before).length() < 1e-3);
        // And never past the limits, nor for a silly amount.
        view.wheel(panel(), around, 1.0e6);
        assert_eq!(view.zoom, MOST_ZOOM);
        view.wheel(panel(), around, -1.0e6);
        assert_eq!(view.zoom, LEAST_ZOOM);
        view.zoom_by(panel(), around, f32::NAN);
        assert_eq!(view.zoom, LEAST_ZOOM);
    }

    #[test]
    fn a_wide_graph_to_read_starts_at_its_left_close_enough_to_read() {
        let mut view = View::default();
        let (min, max) = (Vec2::new(-20.0, -40.0), Vec2::new(3000.0, 900.0));
        view.frame_readable(panel(), min, max);
        assert_eq!(view.zoom, READING_ZOOM);
        let corner = view.on_screen(panel(), min);
        assert_eq!(corner, panel().min + egui::vec2(FRAME_ROOM, FRAME_ROOM));
        // A small one is shown whole, as framing does.
        let mut small = View::default();
        small.frame_readable(panel(), Vec2::ZERO, Vec2::splat(100.0));
        let mut framed = View::default();
        framed.frame(panel(), Vec2::ZERO, Vec2::splat(100.0));
        assert_eq!(small, framed);
    }

    #[test]
    fn framing_shows_the_whole_rectangle() {
        let mut view = View::default();
        let (min, max) = (Vec2::new(-500.0, 0.0), Vec2::new(2500.0, 900.0));
        view.frame(panel(), min, max);
        assert!(panel().contains(view.on_screen(panel(), min)));
        assert!(panel().contains(view.on_screen(panel(), max)));
        // A small one isn't blown up past full size.
        view.frame(panel(), Vec2::ZERO, Vec2::splat(10.0));
        assert_eq!(view.zoom, 1.0);
    }
}
