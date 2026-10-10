//! The 3D viewport panel: shows the rendered image, with its label on top,
//! and turns mouse input into camera navigation (Cinema 4D style: Alt +
//! left drag orbits, Alt + middle pans, Alt + right or the wheel zooms).
//!
//! Rendering itself happens elsewhere, behind the [`ViewportRenderer`]
//! trait, so this panel (and its tests) need no GPU. When no image can be
//! drawn, the panel shows the calm background and says why.

use egui::{Align2, Color32, CursorIcon, FontId, PointerButton, Rect, Response, Sense, Ui, pos2};
use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use tools::{Camera, DragKind, MouseButton, Navigator};

/// How the viewport is shown: display choices the user can switch, which
/// are remembered between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewOptions {
    /// Whether the grid shows its fine 10 cm lines as well as the 1 m and
    /// 10 m ones. Off by default, for a calmer ground.
    pub fine_grid: bool,
}

/// What a renderer needs to draw one viewport frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportView {
    /// Size of the image to draw, in physical pixels.
    pub size_px: [u32; 2],
    /// World-to-screen transform.
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// Background colour at the top of the view, sRGB 0 to 1.
    pub sky_top: [f32; 3],
    /// Background colour at the bottom of the view, sRGB 0 to 1.
    pub sky_bottom: [f32; 3],
    /// Whether to draw the grid's fine 10 cm lines.
    pub fine_grid: bool,
}

/// Draws viewport images. The app implements this with the GPU; tests and
/// machines without a usable GPU use [`NoRenderer`].
pub trait ViewportRenderer {
    /// Draws a frame and returns the egui texture showing it, or a
    /// plain-language reason why it could not.
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String>;
}

/// A renderer that never draws, with a reason to show instead.
#[derive(Debug, Clone)]
pub struct NoRenderer {
    /// Why there is no 3D view.
    pub reason: String,
}

impl Default for NoRenderer {
    fn default() -> Self {
        Self {
            reason: "no graphics device".to_string(),
        }
    }
}

impl ViewportRenderer for NoRenderer {
    fn render(&mut self, _view: &ViewportView) -> Result<egui::TextureId, String> {
        Err(self.reason.clone())
    }
}

/// Accessible name of the viewport, and the label tests look for.
pub const VIEWPORT_LABEL: &str = "3D viewport";

/// The mouse buttons navigation listens to, in egui's terms and ours.
const BUTTONS: [(PointerButton, MouseButton); 3] = [
    (PointerButton::Primary, MouseButton::Primary),
    (PointerButton::Middle, MouseButton::Middle),
    (PointerButton::Secondary, MouseButton::Secondary),
];

/// The viewport panel's state.
#[derive(Debug, Clone, Default)]
pub struct ViewportPanel {
    navigator: Navigator,
    /// The navigation drag in progress, if any. Chosen when the drag
    /// starts, so letting go of Alt mid-drag does not stop it.
    drag: Option<DragKind>,
    options: ViewOptions,
}

impl ViewportPanel {
    /// The camera the viewport looks through.
    pub fn camera(&self) -> &Camera {
        self.navigator.camera()
    }

    /// The camera and the pivot it orbits.
    pub fn navigator(&self) -> &Navigator {
        &self.navigator
    }

    /// Goes back to the opening view (View → Reset view).
    pub fn reset_view(&mut self) {
        self.navigator.reset();
        self.drag = None;
    }

    /// The navigation drag in progress, if any.
    pub fn active_drag(&self) -> Option<DragKind> {
        self.drag
    }

    /// The display choices (grid detail and so on).
    pub fn options(&self) -> ViewOptions {
        self.options
    }

    /// Changes the display choices.
    pub fn options_mut(&mut self) -> &mut ViewOptions {
        &mut self.options
    }

    /// Draws the viewport into all of `ui`'s space.
    pub fn show(&mut self, ui: &mut Ui, renderer: &mut dyn ViewportRenderer) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, VIEWPORT_LABEL)
        });
        self.navigate(ui, &response, rect);
        let painter = ui.painter_at(rect);
        let view = self.view(rect, ui.ctx().pixels_per_point());
        match renderer.render(&view) {
            Ok(texture) => {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(texture, rect, uv, Color32::WHITE);
            }
            Err(reason) => {
                painter.add(crate::panels::vertical_gradient(
                    rect,
                    crate::panels::VIEWPORT_TOP,
                    crate::panels::VIEWPORT_BOTTOM,
                ));
                // A real label (not just painted text), so screen readers
                // and tests can read the reason.
                ui.put(
                    Rect::from_center_size(rect.center(), egui::vec2(rect.width(), 40.0)),
                    egui::Label::new(
                        egui::RichText::new(format!("The 3D view can't be shown: {reason}."))
                            .color(crate::theme::TEXT),
                    )
                    // Not selectable, so dragging across it still moves
                    // the camera underneath.
                    .selectable(false),
                );
            }
        }
        painter.text(
            rect.left_top() + egui::vec2(14.0, 10.0),
            Align2::LEFT_TOP,
            "Perspective",
            FontId::proportional(13.0),
            crate::theme::TEXT_STRONG,
        );
    }

    /// Moves the camera for this frame's mouse input over the viewport.
    fn navigate(&mut self, ui: &Ui, response: &Response, rect: Rect) {
        let size = Vec2::new(rect.width(), rect.height());
        if response.drag_started() {
            let alt = ui.input(|i| i.modifiers.alt);
            self.drag = BUTTONS
                .iter()
                .find(|(egui_button, _)| response.drag_started_by(*egui_button))
                .and_then(|(_, button)| DragKind::for_drag(*button, alt));
        }
        if let Some(kind) = self.drag {
            if response.dragged() {
                let delta = response.drag_delta();
                self.navigator.drag(kind, Vec2::new(delta.x, delta.y), size);
                ui.ctx().set_cursor_icon(match kind {
                    DragKind::Orbit => CursorIcon::Grabbing,
                    DragKind::Pan => CursorIcon::Move,
                    DragKind::Dolly => CursorIcon::ZoomIn,
                });
            }
            if response.drag_stopped() || !response.dragged() {
                self.drag = None;
            }
        }
        if response.hovered() {
            let scroll = wheel_scroll(ui, rect.height());
            if scroll != 0.0 {
                self.navigator.wheel(scroll);
            }
        }
    }

    /// What to ask the renderer for, for a panel at `rect`.
    fn view(&self, rect: Rect, pixels_per_point: f32) -> ViewportView {
        let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
            pixels_per_point
        } else {
            1.0
        };
        let px = |v: f32| (v * ppp).round().clamp(1.0, 16_384.0) as u32;
        let size_px = [px(rect.width()), px(rect.height())];
        let size = Vec2::new(size_px[0] as f32, size_px[1] as f32);
        ViewportView {
            size_px,
            view_projection: self.camera().view_projection(size),
            camera_position: self.camera().position,
            sky_top: rgb(crate::panels::VIEWPORT_TOP),
            sky_bottom: rgb(crate::panels::VIEWPORT_BOTTOM),
            fine_grid: self.options.fine_grid,
        }
    }
}

/// How far the mouse wheel turned this frame, in points; positive when
/// turned away from you. Read straight from the wheel events rather than
/// egui's smoothed scrolling, so zooming responds at once (DESIGN.md §5.2).
fn wheel_scroll(ui: &Ui, page_height: f32) -> f32 {
    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed);
    ui.input(|input| {
        input
            .events
            .iter()
            .map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => match unit {
                    egui::MouseWheelUnit::Point => delta.y,
                    egui::MouseWheelUnit::Line => delta.y * line,
                    egui::MouseWheelUnit::Page => delta.y * page_height,
                },
                _ => 0.0,
            })
            .sum::<f32>()
    })
}

/// An egui colour as sRGB 0 to 1.
fn rgb(color: Color32) -> [f32; 3] {
    [
        f32::from(color.r()) / 255.0,
        f32::from(color.g()) / 255.0,
        f32::from(color.b()) / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_is_sized_in_physical_pixels() {
        let panel = ViewportPanel::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(800.0, 500.0));
        let view = panel.view(rect, 1.5);
        assert_eq!(view.size_px, [1200, 750]);
    }

    #[test]
    fn empty_or_broken_sizes_still_make_a_valid_view() {
        let panel = ViewportPanel::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(0.0, 0.0));
        for ppp in [0.0, f32::NAN, 2.0] {
            let view = panel.view(rect, ppp);
            assert_eq!(view.size_px, [1, 1]);
            assert!(view.view_projection.is_finite());
        }
    }

    #[test]
    fn the_view_uses_the_camera_and_the_theme_colours() {
        let panel = ViewportPanel::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(640.0, 400.0));
        let view = panel.view(rect, 1.0);
        assert_eq!(view.camera_position, panel.camera().position);
        assert_eq!(
            view.view_projection,
            panel.camera().view_projection(Vec2::new(640.0, 400.0))
        );
        assert_eq!(view.sky_top, rgb(crate::panels::VIEWPORT_TOP));
        assert!(view.sky_top[0] > view.sky_bottom[0], "lighter at the top");
    }

    #[test]
    fn the_view_carries_the_grid_detail_choice() {
        let mut panel = ViewportPanel::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(640.0, 400.0));
        assert!(!panel.view(rect, 1.0).fine_grid, "calm by default");
        panel.options_mut().fine_grid = true;
        assert!(panel.view(rect, 1.0).fine_grid);
    }

    #[test]
    fn resetting_the_view_restores_the_opening_camera() {
        let mut panel = ViewportPanel::default();
        panel.navigator.orbit(Vec2::new(200.0, 50.0));
        panel.drag = Some(DragKind::Orbit);
        panel.reset_view();
        assert_eq!(*panel.navigator(), Navigator::default());
        assert_eq!(panel.active_drag(), None);
    }
}
