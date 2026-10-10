//! The 3D viewport panel: its own menu line along the top, the rendered
//! image with quiet labels over it, and the navigation controls picker in
//! the bottom-right corner. Mouse input over the image moves the camera,
//! with the controls the user picked (see `tools::controls`). Orbiting
//! turns around, and zooming moves towards, the point under the mouse:
//! the renderer is asked what is under the mouse while it hovers, so the
//! answer is ready when a drag or a wheel turn starts.
//!
//! Rendering itself happens elsewhere, behind the [`ViewportRenderer`]
//! trait, so this panel (and its tests) need no GPU. When no image can be
//! drawn, the panel shows the calm backdrop and says why.

pub(crate) mod chrome;
mod picker;

use egui::{Color32, CursorIcon, PointerButton, Rect, Response, Sense, Ui, pos2};
use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use tools::{Camera, DragKind, Modifiers, MouseButton, NavigationSettings, Navigator};

#[cfg(test)]
pub(crate) use picker::AROUND_CURSOR_LABEL;
pub use picker::PICKER_LABEL;

/// How the viewport is shown and steered: choices the user can switch,
/// which are remembered between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewOptions {
    /// Whether the grid shows its fine 10 cm lines as well as the 1 m and
    /// 10 m ones. Off by default, for a calmer ground.
    pub fine_grid: bool,
    /// Which mouse controls move the view.
    pub navigation: NavigationSettings,
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
    /// The point the camera looks at; the grid is drawn around it.
    pub focus: Vec3,
    /// The sky straight up, sRGB 0 to 1.
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground just below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// Whether to draw the grid's fine 10 cm lines.
    pub fine_grid: bool,
}

/// A question for the renderer: what is under this pixel of the image?
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickAsk {
    /// The camera's world-to-screen transform for the image.
    pub view_projection: Mat4,
    /// The pixel, in physical pixels from the image's top-left corner.
    pub pixel: [f32; 2],
    /// The image's size in physical pixels.
    pub size_px: [u32; 2],
}

/// What the renderer found under a pixel it was asked about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picked {
    /// The pixel that was asked about, in physical pixels.
    pub pixel: [f32; 2],
    /// The surface point there, or `None` for the background.
    pub point: Option<Vec3>,
}

/// The renderer's reply about picking, each frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PickReply {
    /// An answer that has arrived since the last frame, if any.
    pub answer: Option<Picked>,
    /// Whether a question is still waiting for its answer (so the
    /// interface should draw another frame soon to collect it).
    pub waiting: bool,
}

/// Draws viewport images. The app implements this with the GPU; tests and
/// machines without a usable GPU use [`NoRenderer`].
pub trait ViewportRenderer {
    /// Draws a frame and returns the egui texture showing it, or a
    /// plain-language reason why it could not.
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String>;

    /// Hands over any answer that has arrived about the point under the
    /// mouse, and asks about `ask`'s pixel if given. Never waits. A renderer
    /// that cannot pick answers nothing (the standard).
    fn pick(&mut self, ask: Option<PickAsk>) -> PickReply {
        let _ = ask;
        PickReply::default()
    }
}

/// How close (in logical pixels) a picked answer must be to where the
/// mouse is now to be used: the answer is usually a frame old.
const PICK_TOLERANCE: f32 = 6.0;

/// The latest answer about what is under the mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
struct UnderMouse {
    /// Where the mouse was, in logical screen coordinates.
    at: egui::Pos2,
    /// The surface point there, if any.
    point: Option<Vec3>,
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

/// Accessible name of the 3D image, and the label tests look for.
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
    /// starts, so letting go of a key mid-drag does not stop it.
    drag: Option<DragKind>,
    options: ViewOptions,
    /// Set by Options → Navigation controls, to open the picker once its
    /// button has been drawn this frame.
    open_picker: bool,
    /// What the renderer last found under the mouse.
    under_mouse: Option<UnderMouse>,
    /// Whether a question about the point under the mouse is in flight.
    asking: bool,
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

    /// The display and navigation choices.
    pub fn options(&self) -> ViewOptions {
        self.options
    }

    /// Changes the display and navigation choices.
    pub fn options_mut(&mut self) -> &mut ViewOptions {
        &mut self.options
    }

    /// Draws the viewport into all of `ui`'s space: the menu line along the
    /// top, the 3D image below it.
    pub fn show(&mut self, ui: &mut Ui, renderer: &mut dyn ViewportRenderer) {
        let full = ui.available_rect_before_wrap();
        let header = Rect::from_min_size(full.min, egui::vec2(full.width(), chrome::HEADER_HEIGHT));
        let body = Rect::from_min_max(pos2(full.min.x, header.max.y), full.max);
        ui.allocate_rect(full, Sense::hover());

        let size = Vec2::new(body.width(), body.height());
        chrome::header(ui, header, self, size);

        let response = ui.interact(body, ui.id().with("viewport_body"), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, VIEWPORT_LABEL)
        });
        self.navigate(ui, &response, body);

        let ppp = pixels_per_point(ui);
        let view = self.view(body, ppp);
        let painter = ui.painter_at(body);
        match renderer.render(&view) {
            Ok(texture) => {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(texture, body, uv, Color32::WHITE);
            }
            Err(reason) => chrome::no_image(ui, body, &reason),
        }
        self.ask_what_is_under_the_mouse(ui, &response, body, &view, renderer);
        chrome::overlays(ui, body, self);
        picker::show(ui, body, self);
    }

    /// Collects the renderer's latest answer about the point under the
    /// mouse, and asks again while the mouse is over the image, so an
    /// answer is ready when an orbit or zoom starts.
    fn ask_what_is_under_the_mouse(
        &mut self,
        ui: &Ui,
        response: &Response,
        body: Rect,
        view: &ViewportView,
        renderer: &mut dyn ViewportRenderer,
    ) {
        let ppp = pixels_per_point(ui);
        let ask = self
            .options
            .navigation
            .around_cursor
            .then(|| response.hover_pos())
            .flatten()
            // One question at a time, and only when the mouse has moved
            // since the last answer, so a resting mouse costs nothing.
            .filter(|at| {
                self.drag.is_none()
                    && !self.asking
                    && self.under_mouse.is_none_or(|u| u.at.distance(*at) > 0.5)
            })
            .map(|at| PickAsk {
                view_projection: view.view_projection,
                pixel: [(at.x - body.min.x) * ppp, (at.y - body.min.y) * ppp],
                size_px: view.size_px,
            });
        let reply = renderer.pick(ask);
        if let Some(answer) = reply.answer {
            self.under_mouse = Some(UnderMouse {
                at: body.min + egui::vec2(answer.pixel[0], answer.pixel[1]) / ppp,
                point: answer.point,
            });
        }
        self.asking = reply.waiting;
        if reply.waiting {
            ui.ctx().request_repaint();
        }
    }

    /// The surface point under `at` (logical screen coordinates), if the
    /// renderer's latest answer is about that spot.
    fn surface_at(&self, at: egui::Pos2) -> Option<Vec3> {
        self.under_mouse
            .filter(|u| u.at.distance(at) <= PICK_TOLERANCE)
            .and_then(|u| u.point)
    }

    /// The point to zoom towards for the mouse at `at`: the surface there,
    /// or else the ground under it, or else a point along the mouse's line
    /// of sight as far away as the pivot is now.
    fn zoom_target(&self, at: egui::Pos2, rect: Rect) -> Vec3 {
        if let Some(point) = self.surface_at(at) {
            return point;
        }
        let size = Vec2::new(rect.width(), rect.height());
        let ray = self
            .camera()
            .ray_through(Vec2::new(at.x - rect.min.x, at.y - rect.min.y), size);
        let reach = self.navigator.distance();
        match ray.hit_ground(0.0) {
            // Only ground in front and not much farther than the pivot, so
            // zooming at the horizon doesn't leap towards it.
            Some(distance) if distance <= reach * 4.0 => ray.at(distance),
            _ => ray.at(reach),
        }
    }

    /// Moves the camera for this frame's mouse input over the image.
    fn navigate(&mut self, ui: &Ui, response: &Response, rect: Rect) {
        let size = Vec2::new(rect.width(), rect.height());
        let settings = self.options.navigation;
        if response.drag_started() {
            let keys = ui.input(|i| Modifiers {
                alt: i.modifiers.alt,
                shift: i.modifiers.shift,
            });
            self.drag = BUTTONS
                .iter()
                .find(|(egui_button, _)| response.drag_started_by(*egui_button))
                .and_then(|(_, button)| settings.drag(*button, keys));
            let pressed_at = ui.input(|i| i.pointer.press_origin());
            if let (Some(kind), Some(at), true) = (self.drag, pressed_at, settings.around_cursor) {
                match kind {
                    // Turn around what was clicked; over the background,
                    // keep turning around the last pivot.
                    DragKind::Orbit => {
                        if let Some(point) = self.surface_at(at) {
                            self.navigator.set_pivot(point);
                        }
                    }
                    DragKind::Dolly => {
                        let target = self.zoom_target(at, rect);
                        self.navigator.set_pivot(target);
                    }
                    DragKind::Pan => {}
                }
            }
        }
        if let Some(kind) = self.drag {
            if response.dragged() {
                let delta = response.drag_delta();
                let delta = settings.movement(kind, Vec2::new(delta.x, delta.y));
                self.navigator.drag(kind, delta, size);
                ui.ctx().set_cursor_icon(cursor_for(kind));
            }
            if response.drag_stopped() || !response.dragged() {
                self.drag = None;
            }
        }
        if response.hovered() {
            let scroll = wheel_scroll(ui, rect.height());
            if scroll != 0.0 {
                match response.hover_pos().filter(|_| settings.around_cursor) {
                    Some(at) => {
                        let target = self.zoom_target(at, rect);
                        self.navigator.wheel_towards(scroll, target);
                    }
                    None => self.navigator.wheel(scroll),
                }
            }
        }
    }

    /// Applies a drag on one of the menu line's navigation buttons.
    fn button_drag(&mut self, kind: DragKind, delta: egui::Vec2, size: Vec2) {
        let delta = self
            .options
            .navigation
            .movement(kind, Vec2::new(delta.x, delta.y));
        self.navigator.drag(kind, delta, size);
    }

    /// What to ask the renderer for, for an image at `rect`.
    fn view(&self, rect: Rect, pixels_per_point: f32) -> ViewportView {
        let ppp = sane_scale(pixels_per_point);
        let px = |v: f32| (v * ppp).round().clamp(1.0, 16_384.0) as u32;
        let size_px = [px(rect.width()), px(rect.height())];
        let size = Vec2::new(size_px[0] as f32, size_px[1] as f32);
        use crate::theme::backdrop;
        ViewportView {
            size_px,
            view_projection: self.camera().view_projection(size),
            camera_position: self.camera().position,
            focus: self.navigator.pivot(),
            sky_top: rgb(backdrop::SKY_TOP),
            sky_horizon: rgb(backdrop::HORIZON),
            ground: rgb(backdrop::GROUND),
            fine_grid: self.options.fine_grid,
        }
    }
}

/// Physical pixels per logical pixel, never zero or broken.
fn pixels_per_point(ui: &Ui) -> f32 {
    sane_scale(ui.ctx().pixels_per_point())
}

/// `scale` if it is a usable size factor, otherwise 1.
fn sane_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// The mouse pointer shown while navigating.
fn cursor_for(kind: DragKind) -> CursorIcon {
    match kind {
        DragKind::Orbit => CursorIcon::Grabbing,
        DragKind::Pan => CursorIcon::Move,
        DragKind::Dolly => CursorIcon::ZoomIn,
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
    use crate::theme::backdrop;

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
    fn the_view_uses_the_camera_pivot_and_backdrop() {
        let panel = ViewportPanel::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(640.0, 400.0));
        let view = panel.view(rect, 1.0);
        assert_eq!(view.camera_position, panel.camera().position);
        assert_eq!(view.focus, panel.navigator().pivot());
        assert_eq!(
            view.view_projection,
            panel.camera().view_projection(Vec2::new(640.0, 400.0))
        );
        assert_eq!(view.sky_top, rgb(backdrop::SKY_TOP));
        assert_eq!(view.ground, rgb(backdrop::GROUND));
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

    #[test]
    fn button_drags_respect_the_invert_setting() {
        let size = Vec2::new(640.0, 400.0);
        let mut normal = ViewportPanel::default();
        normal.button_drag(DragKind::Orbit, egui::vec2(30.0, 0.0), size);
        let mut inverted = ViewportPanel::default();
        inverted.options_mut().navigation.invert_orbit = true;
        inverted.button_drag(DragKind::Orbit, egui::vec2(30.0, 0.0), size);
        let start = Navigator::default().camera().yaw;
        assert!(normal.camera().yaw < start);
        assert!(inverted.camera().yaw > start);
    }

    #[test]
    fn view_options_survive_saving_and_old_files() {
        let mut options = ViewOptions {
            fine_grid: true,
            ..ViewOptions::default()
        };
        options.navigation.scheme = tools::Scheme::MouseOnly;
        let text = ron::to_string(&options).unwrap();
        assert_eq!(ron::from_str::<ViewOptions>(&text).unwrap(), options);
        // Files saved before a setting existed still load, with defaults.
        let old: ViewOptions = ron::from_str("(fine_grid: true)").unwrap();
        assert!(old.fine_grid);
        assert_eq!(old.navigation, NavigationSettings::default());
    }
}
