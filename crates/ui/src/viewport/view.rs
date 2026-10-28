//! One view in the viewport: its camera, the navigation drag under way,
//! gliding into place when framing, and what it last learned about the
//! point under the mouse.

use super::input::{cursor_for, wheel_scroll};
use super::options::{GridDetail, Shading, Surface, ViewOptions};
use super::renderer::ViewportView;
use super::{select, tool_drag};
use crate::drawing::{SceneDrawing, SceneSummary};
use crate::text::tr;
use crate::theme;
use egui::{Color32, PointerButton, Rect, Response, Ui};
use glam::{Vec2, Vec3};
use std::sync::Arc;
use tools::{
    Bounds, Camera, DragKind, Modifiers, MouseButton, NavigationSettings, Navigator, ViewKind,
};

/// How close (in logical pixels) a picked answer must be to where the
/// mouse is now to be used: the answer is usually a frame old.
const PICK_TOLERANCE: f32 = 6.0;

/// The latest answer about what is under the mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct UnderMouse {
    /// Where the mouse was, in logical screen coordinates.
    pub(super) at: egui::Pos2,
    /// The surface point there, if any.
    pub(super) point: Option<Vec3>,
    /// The camera the question was asked with: once the camera moves, the
    /// answer no longer describes what is under the mouse.
    pub(super) camera: Camera,
}

/// What framing fits when nothing shown has a shape: a 1 m box standing
/// at the origin (the size of a standard shape), so the view goes back to
/// the middle of the scene.
pub const DEFAULT_FRAME: Bounds = Bounds {
    min: Vec3::new(-50.0, -50.0, 0.0),
    max: Vec3::new(50.0, 50.0, 100.0),
};

/// What to fit in the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// Everything in the scene (key `H`).
    All,
    /// The selection (key `F`); everything when nothing shown is selected.
    Selected,
}

impl Frame {
    /// The menu entry's name.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => tr("Frame all"),
            Self::Selected => tr("Frame selected"),
        }
    }

    /// The key that does it in the viewport.
    pub fn key(self) -> egui::Key {
        match self {
            Self::All => egui::Key::H,
            Self::Selected => egui::Key::Z,
        }
    }

    /// What the menu entry's tooltip says.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::All => tr("Fit everything in the scene in the view"),
            Self::Selected => {
                tr("Fit the selected objects in the view (everything, while nothing is selected)")
            }
        }
    }
}

/// How long the view takes to glide into place when framing, in seconds:
/// quick, but long enough to see where you went.
pub const GLIDE_SECONDS: f64 = 0.2;

/// A short, smooth move of the camera from one view to another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Glide {
    pub(super) from: Navigator,
    pub(super) to: Navigator,
    /// When it started, in egui's clock (seconds).
    pub(super) start: f64,
}

impl Glide {
    /// The view at time `now`, and whether the glide has finished. Eases
    /// out: fast at first, settling gently.
    pub(super) fn at(&self, now: f64) -> (Navigator, bool) {
        let t = ((now - self.start) / GLIDE_SECONDS).clamp(0.0, 1.0);
        let t = if t.is_finite() { t } else { 1.0 };
        let eased = 1.0 - (1.0 - t).powi(3);
        (self.from.blend(&self.to, eased as f32), t >= 1.0)
    }
}

/// The mouse buttons navigation listens to, in egui's terms and ours.
const BUTTONS: [(PointerButton, MouseButton); 3] = [
    (PointerButton::Primary, MouseButton::Primary),
    (PointerButton::Middle, MouseButton::Middle),
    (PointerButton::Secondary, MouseButton::Secondary),
];

/// One view: its camera and what the mouse is doing in it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct View {
    pub(super) navigator: Navigator,
    /// The navigation drag in progress, if any. Chosen when the drag
    /// starts, so letting go of a key mid-drag does not stop it.
    pub(super) drag: Option<DragKind>,
    /// What the renderer last found under the mouse in this view.
    pub(super) under_mouse: Option<UnderMouse>,
    /// A framing move in progress.
    pub(super) glide: Option<Glide>,
    /// The image's size last frame, in logical pixels (for framing from a
    /// menu, before this frame's size is known).
    pub(super) image_size: Vec2,
    /// A selection drag (box, lasso or brush) in progress, if any.
    pub(super) select_drag: Option<select::SelectDrag>,
    /// Where and when the last single click was, so a double-click is
    /// two clicks close in time and place, whatever came before (egui
    /// counts a third quick click as a triple, not a double).
    pub(super) last_click: Option<(egui::Pos2, f64)>,
    /// Where and when the last click of a double-click (or a quick click
    /// straight after one) was: a further quick click there is part of the
    /// same burst and is ignored, so it can't undo the loop just selected.
    pub(super) last_double: Option<(egui::Pos2, f64)>,
    /// A drag of the gizmo in progress, if any.
    pub(super) gizmo_drag: Option<tools::Drag>,
    /// The gizmo handle the left button went down on, if any.
    pub(super) pressed_handle: Option<tools::Handle>,
    /// The gizmo handle under the mouse, to light up.
    pub(super) hovered_handle: Option<tools::Handle>,
    /// A modelling tool's drag (extrude, inset) in progress, if any.
    pub(super) tool_drag: Option<tool_drag::ToolDrag>,
    /// Whether the pivot is a point on a surface you pointed at (rather
    /// than a point in mid-air left by zooming over empty space), so an
    /// orbit started over empty space may keep turning around it.
    pub(super) pivot_on_surface: bool,
    /// While orbiting with Ctrl held, the view as the orbit would leave it
    /// without snapping; what is shown is it snapped to the nearest side
    /// of the view dice (decision 0084).
    pub(super) free: Option<Navigator>,
    /// Whether the view was turned flat by the view dice (or snapping)
    /// with *Switch to orthographic when clicking the dice* on: orbiting
    /// away freely turns it back into a perspective view (decision 0084).
    pub(super) auto_flat: bool,
}

impl View {
    /// A view of the given kind, at its opening camera.
    pub(super) fn new(kind: ViewKind) -> Self {
        Self {
            navigator: Navigator::for_view(kind),
            drag: None,
            under_mouse: None,
            glide: None,
            image_size: Vec2::ZERO,
            select_drag: None,
            last_click: None,
            last_double: None,
            gizmo_drag: None,
            pressed_handle: None,
            hovered_handle: None,
            tool_drag: None,
            pivot_on_surface: false,
            free: None,
            auto_flat: false,
        }
    }

    pub(super) fn camera(&self) -> &Camera {
        self.navigator.camera()
    }

    pub(super) fn kind(&self) -> ViewKind {
        self.navigator.kind()
    }

    /// Switches between perspective and orthographic, stopping a glide.
    pub(super) fn toggle_projection(&mut self) {
        self.glide = None;
        self.auto_flat = false;
        self.navigator = self.navigator.toggled_projection();
    }

    /// Goes back to the opening camera for this kind of view.
    pub(super) fn reset(&mut self) {
        *self = Self {
            image_size: self.image_size,
            ..Self::new(self.kind())
        };
    }

    /// Starts gliding to fit `bounds` in the view, at time `now`.
    pub(super) fn start_frame(&mut self, bounds: Bounds, now: f64) {
        let aspect = if self.image_size.y > 0.0 {
            self.image_size.x / self.image_size.y
        } else {
            1.0
        };
        let to = self.navigator.framed(&bounds, aspect);
        self.glide = Some(Glide {
            from: self.navigator,
            to,
            start: now,
        });
        self.drag = None;
    }

    /// Starts gliding round the pivot to look from `side` of the view
    /// dice, at time `now` ([decision 0084](../../../../docs/decisions/0084-view-dice.md)).
    /// With `flat` (Display → Switch to orthographic when clicking the
    /// dice), a perspective view ends up orthographic, and turns back into
    /// perspective when orbited away.
    pub(super) fn start_look_from(&mut self, side: glam::IVec3, flat: bool, now: f64) {
        let mut to = self.navigator.looking_from(side);
        if flat && !to.camera().is_flat() {
            to = to.toggled_projection();
            self.auto_flat = true;
        }
        self.glide = Some(Glide {
            from: self.navigator,
            to,
            start: now,
        });
        self.drag = None;
    }

    /// Orbits by `delta` (already scaled): with `snap` (Ctrl held), the
    /// view shows the orbit snapped to the nearest side of the view dice,
    /// turned flat if `flat`; without, a view the dice turned flat goes
    /// back to perspective first (decision 0084).
    fn orbit(&mut self, delta: Vec2, size: Vec2, snap: bool, flat: bool) {
        if snap {
            let free = self.free.get_or_insert(self.navigator);
            free.drag(DragKind::Orbit, delta, size);
            let mut shown = free.snapped();
            if flat && !shown.camera().is_flat() {
                shown = shown.toggled_projection();
                self.auto_flat = true;
            }
            self.navigator = shown;
            return;
        }
        self.free = None;
        if self.auto_flat {
            self.auto_flat = false;
            if self.navigator.camera().is_flat() {
                self.navigator = self.navigator.toggled_projection();
            }
        }
        self.navigator.drag(DragKind::Orbit, delta, size);
    }

    /// Moves the camera along a glide in progress, at time `now`; returns
    /// whether it is still going.
    pub(super) fn follow_glide(&mut self, now: f64) -> bool {
        let Some(glide) = self.glide else {
            return false;
        };
        let (view, done) = glide.at(now);
        self.navigator = view;
        if done {
            self.glide = None;
        }
        !done
    }

    /// The renderer's answer about `at` (logical screen coordinates):
    /// `Some(Some(point))` for a surface, `Some(None)` for background, or
    /// `None` when there is no up-to-date answer for that spot (the mouse
    /// or the camera has moved since).
    pub(super) fn answer_at(&self, at: egui::Pos2) -> Option<Option<Vec3>> {
        self.under_mouse
            .filter(|u| u.at.distance(at) <= PICK_TOLERANCE && u.camera == *self.camera())
            .map(|u| u.point)
    }

    /// The surface point under `at`, if an up-to-date answer says so.
    pub(super) fn surface_at(&self, at: egui::Pos2) -> Option<Vec3> {
        self.answer_at(at).flatten()
    }

    /// The point to zoom towards for the mouse at `at`: the surface there;
    /// over background, the ground under it (if not much farther than the
    /// pivot); otherwise, or without an up-to-date answer, a point along
    /// the mouse's line of sight as far away as the pivot is now, so the
    /// zoom never leaps.
    /// Whether it is a point on a surface.
    ///
    /// The ground is used only when it is no farther than the pivot: a
    /// far-off pivot makes every later zoom step, pan and orbit huge.
    pub(super) fn zoom_target(&self, at: egui::Pos2, rect: Rect) -> (Vec3, bool) {
        let size = Vec2::new(rect.width(), rect.height());
        let ray = self
            .camera()
            .ray_through(Vec2::new(at.x - rect.min.x, at.y - rect.min.y), size);
        let reach = self.navigator.distance();
        match self.answer_at(at) {
            Some(Some(point)) => (point, true),
            Some(None) => match ray.hit_ground(0.0) {
                Some(distance) if distance <= reach => (ray.at(distance), false),
                _ => (ray.at(reach), false),
            },
            None => (ray.at(reach), false),
        }
    }

    /// The point to turn around (or pan at the depth of) when a drag
    /// starts over empty space ([decision 0080](../../../../docs/decisions/0080-navigation-feel.md)):
    /// the first of these that is in front of the camera and on screen,
    /// so the view never swings around something you can't see:
    ///
    /// 1. the pivot, if it is a surface point you pointed at before;
    /// 2. the middle of the selection;
    /// 3. the middle of everything shown;
    ///
    /// otherwise the middle of the view, at the pivot's depth.
    pub(super) fn pivot_over_empty_space(&mut self, size: Vec2, scene: Option<&SceneSummary>) {
        let on_screen = |camera: &Camera, point: Vec3| {
            camera
                .project(point, size)
                .is_some_and(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y)
        };
        let camera = *self.camera();
        if self.pivot_on_surface && on_screen(&camera, self.navigator.pivot()) {
            return;
        }
        let candidates = scene
            .into_iter()
            .flat_map(|s| [s.selected, s.all])
            .flatten()
            .map(|b| b.centre());
        for point in candidates {
            if on_screen(&camera, point) && self.navigator.set_pivot(point) {
                self.pivot_on_surface = false;
                return;
            }
        }
        self.navigator.centre_pivot();
        self.pivot_on_surface = false;
    }

    /// Moves the camera for this frame's mouse input over the image.
    pub(super) fn navigate(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        options: &ViewOptions,
        slide_gizmo: Option<tools::Gizmo>,
        scene: Option<&SceneSummary>,
    ) {
        let settings = options.navigation;
        let flat_from_dice = options.flat_from_dice;
        let size = Vec2::new(rect.width(), rect.height());
        // Shift + Alt + left drag on the Move gizmo slides points
        // (decisions 0055 and 0133), so it doesn't orbit there. Plain
        // Alt + drag orbits, even over the gizmo.
        let sliding = response.drag_started_by(PointerButton::Primary)
            && ui.input(|i| i.modifiers.alt && i.modifiers.shift)
            && slide_gizmo.is_some_and(|gizmo| {
                ui.input(|i| i.pointer.press_origin()).is_some_and(|at| {
                    let local = Vec2::new(at.x - rect.min.x, at.y - rect.min.y);
                    gizmo
                        .handle_at(self.camera(), size, local, [false; 3])
                        .is_some()
                })
            });
        if response.drag_started() && !sliding {
            // Taking hold of the view stops a framing glide where it is.
            self.glide = None;
            let keys = ui.input(|i| Modifiers {
                alt: i.modifiers.alt,
                shift: i.modifiers.shift,
            });
            self.drag = BUTTONS
                .iter()
                .find(|(egui_button, _)| response.drag_started_by(*egui_button))
                .and_then(|(_, button)| settings.drag(*button, keys));
            let pressed_at = ui.input(|i| i.pointer.press_origin());
            if !settings.around_cursor
                && matches!(self.drag, Some(DragKind::Orbit | DragKind::Dolly))
            {
                // Around the middle of the view, whatever was picked before.
                self.navigator.centre_pivot();
            }
            if let (Some(kind), Some(at), true) = (self.drag, pressed_at, settings.around_cursor) {
                match kind {
                    // Turn around what was clicked, and pan at its depth so
                    // it stays under the mouse; over empty space, around
                    // something you can see.
                    DragKind::Orbit | DragKind::Pan => match self.surface_at(at) {
                        Some(point) if self.navigator.set_pivot(point) => {
                            self.pivot_on_surface = true;
                        }
                        _ => self.pivot_over_empty_space(size, scene),
                    },
                    DragKind::Dolly => {
                        let (target, on_surface) = self.zoom_target(at, rect);
                        if self.navigator.set_pivot(target) {
                            self.pivot_on_surface = on_surface;
                        }
                    }
                }
            }
        }
        if let Some(kind) = self.drag {
            if response.dragged() {
                // The drag only counts as one once the mouse has moved a
                // few pixels; on its first frame, use the whole way from
                // where the button went down, so no movement is lost.
                let delta = match (
                    response.drag_started(),
                    ui.input(|i| i.pointer.press_origin()),
                    response.interact_pointer_pos(),
                ) {
                    (true, Some(from), Some(to)) => to - from,
                    _ => response.drag_delta(),
                };
                let delta = settings.movement(kind, Vec2::new(delta.x, delta.y));
                if kind == DragKind::Orbit {
                    // Ctrl while orbiting snaps to the dice's sides.
                    let snap = ui.input(|i| i.modifiers.command);
                    self.orbit(delta, size, snap, flat_from_dice);
                } else {
                    self.navigator.drag(kind, delta, size);
                }
                ui.ctx().set_cursor_icon(cursor_for(kind));
            }
            if response.drag_stopped() || !response.dragged() {
                self.drag = None;
                self.free = None;
            }
        }
        // Ctrl + wheel grows and shrinks the selection instead.
        if response.hovered() && !ui.input(|i| i.modifiers.command) {
            let scroll = wheel_scroll(ui, rect.height());
            if scroll != 0.0 {
                self.glide = None;
                // While a button is held (mid-orbit or mid-pan), the point
                // under the mouse is not being looked up, so keep zooming
                // towards the pivot already in use instead of jumping to a
                // stale or guessed spot.
                let busy = self.drag.is_some() || ui.input(|i| i.pointer.any_down());
                match response
                    .hover_pos()
                    .filter(|_| settings.around_cursor && !busy)
                {
                    Some(at) => {
                        let (target, on_surface) = self.zoom_target(at, rect);
                        self.navigator.wheel_towards(scroll, target);
                        if self.navigator.pivot() == target {
                            self.pivot_on_surface = on_surface;
                        }
                    }
                    None => {
                        if !settings.around_cursor {
                            self.navigator.centre_pivot();
                        }
                        self.navigator.wheel(scroll);
                    }
                }
            }
        }
    }

    /// Applies a drag on one of the menu line's navigation buttons.
    pub(super) fn button_drag(
        &mut self,
        kind: DragKind,
        delta: egui::Vec2,
        settings: NavigationSettings,
    ) {
        self.glide = None;
        if !settings.around_cursor {
            self.navigator.centre_pivot();
        }
        let delta = settings.movement(kind, Vec2::new(delta.x, delta.y));
        self.navigator.drag(kind, delta, self.image_size);
    }

    /// What to ask the renderer for, for this view's image at `rect`.
    pub(super) fn view(
        &self,
        slot: usize,
        rect: Rect,
        pixels_per_point: f32,
        grid_detail: GridDetail,
        scene: &Arc<SceneDrawing>,
    ) -> ViewportView {
        let ppp = sane_scale(pixels_per_point);
        let px = |v: f32| (v * ppp).round().clamp(1.0, 16_384.0) as u32;
        let size_px = [px(rect.width()), px(rect.height())];
        let size = Vec2::new(size_px[0] as f32, size_px[1] as f32);
        use crate::theme::backdrop;
        ViewportView {
            slot,
            size_px,
            view_projection: self.camera().view_projection(size),
            camera_position: self.camera().position,
            focus: self.navigator.pivot(),
            flat_height: self.camera().ortho_height,
            grid_plane: self.kind().grid_plane(),
            sky_top: rgb(backdrop::SKY_TOP),
            sky_horizon: rgb(backdrop::HORIZON),
            ground: rgb(backdrop::GROUND),
            grid_detail,
            scene: Arc::clone(scene),
            selection_color: rgb(theme::Accent::default().colors().light),
            grid_colors: theme::grid_colors(),
            overlay: Arc::default(),
            shading: Shading::default(),
            cavity: true,
            surface: Surface::Plain,
            paint_preview: super::PaintPreview::Lit,
            high_contrast: false,
        }
    }
}

/// `scale` if it is a usable size factor, otherwise 1.
pub(super) fn sane_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// An egui colour as sRGB 0 to 1.
pub(super) fn rgb(color: Color32) -> [f32; 3] {
    [
        f32::from(color.r()) / 255.0,
        f32::from(color.g()) / 255.0,
        f32::from(color.b()) / 255.0,
    ]
}
