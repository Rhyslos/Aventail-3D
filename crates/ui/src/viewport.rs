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
mod dice;
mod gizmo;
mod look;
mod mirror_plane;
mod picker;
mod select;
mod tool_drag;

use egui::{Color32, CursorIcon, PointerButton, Rect, Response, Sense, Ui, pos2};
use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use tools::{
    Bounds, Camera, DragKind, Modifiers, MouseButton, NavigationSettings, Navigator, ViewKind,
};

use crate::components::ViewPointer;
use crate::drawing::{ComponentOverlay, SceneDrawing, SceneSummary};
use crate::selecting::{
    BRUSH_MAX, BRUSH_MIN, BRUSH_RADIUS, BRUSH_STEP, Gesture, SelectKeys, SelectTool,
};
use crate::theme;
use crate::transforming::{Axes, SnapSettings};
pub(crate) use gizmo::GizmoEvent;
use std::sync::Arc;
pub(crate) use tool_drag::{ToolDragEvent, ToolPath};

#[cfg(test)]
pub(crate) use look::SHADING_LABEL;
#[cfg(test)]
pub(crate) use picker::AROUND_CURSOR_LABEL;
pub use picker::PICKER_LABEL;

/// How detailed the grid is: how far apart its finest lines are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GridDetail {
    /// Lines every metre (and stronger ones every 10 m): the calmest.
    #[default]
    Metre,
    /// Each metre square cut into four: lines every 50 cm as well.
    HalfMetre,
    /// Thin lines every 10 cm as well: for placing things precisely.
    TenCentimetres,
}

/// How shapes are drawn in the viewport ([decision 0063](../../../docs/decisions/0063-wireframe.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Shading {
    /// Solid, softly lit surfaces with faint edges: the calmest.
    #[default]
    Shaded,
    /// Solid surfaces with every edge drawn clearly on top, to see how a
    /// shape is built while still seeing its form.
    ShadedWithWires,
    /// Only the edges, seen through the shape: every edge at once, front
    /// and back.
    Wireframe,
}

impl Shading {
    /// Every look, in the order offered.
    pub const ALL: [Shading; 3] = [Self::Shaded, Self::ShadedWithWires, Self::Wireframe];

    /// Its name, in the menu and on the button.
    pub fn label(self) -> &'static str {
        match self {
            Self::Shaded => "Shaded",
            Self::ShadedWithWires => "Shaded with wires",
            Self::Wireframe => "Wireframe",
        }
    }

    /// What it shows, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Shaded => "Solid, softly lit surfaces with faint edges",
            Self::ShadedWithWires => {
                "Solid surfaces with every edge drawn clearly on top, to see how the shape \
                 is built"
            }
            Self::Wireframe => {
                "Only the edges, seen through the shape: front and back at once. Clicking \
                 still picks the shape."
            }
        }
    }

    /// Whether surfaces are drawn.
    pub fn shows_faces(self) -> bool {
        self != Self::Wireframe
    }
}

/// The angle from which edges shade hard while Automatic hard edges is on,
/// in degrees: the same as Hard edges by angle starts at.
pub const AUTO_HARD_ANGLE: f32 = mesh::hardness::DEFAULT_ANGLE;

impl ViewOptions {
    /// The angle edges shade hard from, while Automatic hard edges is on
    /// (`None`: by their marks).
    pub fn auto_hard_angle(&self) -> Option<f32> {
        (!self.hard_edges_by_hand).then_some(AUTO_HARD_ANGLE)
    }

    /// These options with an old "fine grid lines on" setting turned into
    /// the finest grid level.
    #[must_use]
    pub fn migrated(mut self) -> Self {
        if self.legacy_fine_grid {
            self.grid_detail = GridDetail::TenCentimetres;
            self.legacy_fine_grid = false;
        }
        self
    }
}

impl GridDetail {
    /// Every level, from calm to detailed.
    pub const ALL: [GridDetail; 3] = [Self::Metre, Self::HalfMetre, Self::TenCentimetres];

    /// The menu entry's name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Metre => "Every 1 m",
            Self::HalfMetre => "Every 50 cm (quarters)",
            Self::TenCentimetres => "Every 10 cm (fine)",
        }
    }

    /// The finest lines' spacing, as written in the viewport's corner.
    pub fn spacing_text(self) -> &'static str {
        match self {
            Self::Metre => "1 m",
            Self::HalfMetre => "50 cm",
            Self::TenCentimetres => "10 cm",
        }
    }

    /// The finest lines' spacing for the renderer, in centimetres, or
    /// `None` for only the 1 m and 10 m lines.
    pub fn fine_spacing(self) -> Option<f32> {
        match self {
            Self::Metre => None,
            Self::HalfMetre => Some(50.0),
            Self::TenCentimetres => Some(10.0),
        }
    }

    /// The next level, round again after the finest (the palette button).
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Metre => Self::HalfMetre,
            Self::HalfMetre => Self::TenCentimetres,
            Self::TenCentimetres => Self::Metre,
        }
    }

    /// A plain-language description, for tooltips.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Metre => "Grid lines every metre: the calmest ground",
            Self::HalfMetre => "Each metre square cut into four: lines every 50 cm",
            Self::TenCentimetres => "Thin lines every 10 cm, for placing things precisely",
        }
    }
}

/// How the viewport is shown and steered: choices the user can switch,
/// which are remembered between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewOptions {
    /// How detailed the grid is. Lines every metre as standard, for a calm
    /// ground.
    pub grid_detail: GridDetail,
    /// Settings saved before the grid had three levels said only whether
    /// the 10 cm lines were on; read so that choice carries over.
    #[serde(rename = "fine_grid", skip_serializing)]
    pub legacy_fine_grid: bool,
    /// Which mouse controls move the view.
    pub navigation: NavigationSettings,
    /// Whether all four views (perspective, top, front, right) are shown
    /// at once, rather than one filling the viewport.
    pub four_views: bool,
    /// Whether the box, lasso and brush only select what can be seen. Off
    /// as standard: they select through the shape, catching its far side
    /// too (decision 0029).
    pub select_visible_only: bool,
    /// Which keys add to and take from the selection.
    pub select_keys: SelectKeys,
    /// How the move, rotate and scale gizmos snap.
    pub snap: SnapSettings,
    /// Axes that can't change while moving, rotating or scaling (X, Y,
    /// Z).
    pub locked: [bool; 3],
    /// Whether the gizmo follows the world's axes or the object's own.
    pub axes: Axes,
    /// Whether hard edges are *not* marked in the Points, Edges and
    /// Polygons modes (they are as standard; decision 0053).
    pub hide_hard_edges: bool,
    /// Whether the selected objects show which way each polygon faces,
    /// as short lines (off as standard; decision 0053).
    pub show_normals: bool,
    /// The live mirror's settings, remembered for the next object
    /// mirrored (decision 0088).
    pub symmetry: Symmetry,
    /// Whether Dissolve goes ahead across a sharp corner without asking
    /// first ("Don't warn me again"; it asks as standard, decision 0057).
    pub dissolve_without_warning: bool,
    /// How shapes are drawn: shaded, shaded with wires, or wireframe.
    pub shading: Shading,
    /// Whether Cavity is *off*: ridges not brightened and creases not
    /// darkened. Cavity is on as standard (decision 0065).
    pub hide_cavity: bool,
    /// Whether shapes are shaded by their marked hard edges, with
    /// *Automatic hard edges* off. Off as standard: the angle decides
    /// ([`AUTO_HARD_ANGLE`], decision 0064).
    pub hard_edges_by_hand: bool,
    /// The keyboard the Keys window draws, once picked there (`None`:
    /// guessed from the system language; decision 0061).
    pub keyboard: Option<crate::KeyboardLayout>,
    /// Whether clicking the view dice (or snapping to it with Ctrl while
    /// orbiting) turns a perspective view orthographic, until it is
    /// orbited away (off as standard; decision 0084).
    pub flat_from_dice: bool,
}

/// The live mirror's settings ([decision 0088](../../../docs/decisions/0088-live-mirror.md)):
/// what the Symmetry button gives the selected objects, and what its menu
/// shows. Each object keeps its own mirror; these are remembered for the
/// next one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Symmetry {
    /// Across which of the world's axes, X, Y and Z (X as standard: left
    /// and right).
    pub axes: [bool; 3],
    /// How far the mirror is moved from the world centre along X, Y and Z,
    /// in centimetres.
    pub offset: [f32; 3],
    /// How near points at the seam must be to become one, in centimetres
    /// (0.1 as standard).
    pub weld_distance: f32,
}

/// The standard weld distance, in centimetres.
pub(crate) const WELD_DISTANCE: f32 = scene::LiveMirror::WELD_DISTANCE;

impl Default for Symmetry {
    fn default() -> Self {
        Self {
            axes: [true, false, false],
            offset: [0.0; 3],
            weld_distance: WELD_DISTANCE,
        }
    }
}

impl Symmetry {
    /// The mirror these settings make, if any axis is chosen.
    pub fn mirror(&self) -> Option<scene::LiveMirror> {
        let mirror = scene::LiveMirror {
            axes: self.axes,
            centre: glam::Vec3::from_array(self.offset),
            weld_distance: self
                .weld_distance
                .clamp(0.0, scene::LiveMirror::MAX_WELD_DISTANCE),
        };
        mirror.is_valid().then_some(mirror)
    }

    /// The settings of an object's `mirror`.
    pub fn of(mirror: scene::LiveMirror) -> Self {
        Self {
            axes: mirror.axes,
            offset: mirror.centre.to_array(),
            weld_distance: mirror.weld_distance,
        }
    }
}

/// What a renderer needs to draw one viewport frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportView {
    /// Which of the four views this is (0 to 3), so a renderer can keep a
    /// separate image for each.
    pub slot: usize,
    /// Size of the image to draw, in physical pixels.
    pub size_px: [u32; 2],
    /// World-to-screen transform.
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// The point the camera looks at; the grid is drawn around it.
    pub focus: Vec3,
    /// For a flat (orthographic) view, the height it shows in centimetres;
    /// `None` for perspective.
    pub flat_height: Option<f32>,
    /// The plane the grid lies in, as two directions across it.
    pub grid_plane: [Vec3; 2],
    /// The sky straight up, sRGB 0 to 1.
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground just below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// How detailed the grid is.
    pub grid_detail: GridDetail,
    /// The scene's objects to draw.
    pub scene: Arc<SceneDrawing>,
    /// The colour selected objects are outlined in (the accent colour),
    /// sRGB 0 to 1.
    pub selection_color: [f32; 3],
    /// What to draw over the edited objects in the Points, Edges and
    /// Polygons modes.
    pub overlay: Arc<ComponentOverlay>,
    /// How shapes are drawn.
    pub shading: Shading,
    /// Shade by angle from this many degrees, whatever edges are marked
    /// (Automatic hard edges), or by the marks (`None`).
    pub auto_hard_angle: Option<f32>,
    /// Whether ridges are brightened and creases darkened (Cavity).
    pub cavity: bool,
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

    /// Whether a heavy shape is still being made ready to draw in the
    /// background (the viewport says so, and keeps drawing until it is).
    fn preparing(&self) -> bool {
        false
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
    /// The camera the question was asked with: once the camera moves, the
    /// answer no longer describes what is under the mouse.
    camera: Camera,
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
            Self::All => "Frame all",
            Self::Selected => "Frame selected",
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
            Self::All => "Fit everything in the scene in the view",
            Self::Selected => {
                "Fit the selected objects in the view (everything, while nothing is selected)"
            }
        }
    }
}

/// How long the view takes to glide into place when framing, in seconds:
/// quick, but long enough to see where you went.
pub const GLIDE_SECONDS: f64 = 0.2;

/// A short, smooth move of the camera from one view to another.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Glide {
    from: Navigator,
    to: Navigator,
    /// When it started, in egui's clock (seconds).
    start: f64,
}

impl Glide {
    /// The view at time `now`, and whether the glide has finished. Eases
    /// out: fast at first, settling gently.
    fn at(&self, now: f64) -> (Navigator, bool) {
        let t = ((now - self.start) / GLIDE_SECONDS).clamp(0.0, 1.0);
        let t = if t.is_finite() { t } else { 1.0 };
        let eased = 1.0 - (1.0 - t).powi(3);
        (self.from.blend(&self.to, eased as f32), t >= 1.0)
    }
}

#[cfg(test)]
pub(crate) use dice::DICE_LABEL;

/// Where to click on the view dice of the view at `rect` to look from
/// `side`, if it shows. For tests.
#[cfg(test)]
pub(crate) fn dice_point(camera: &Camera, rect: Rect, side: glam::IVec3) -> Option<egui::Pos2> {
    dice::point_for(camera, rect, side)
}

/// Accessible name of the 3D image, and the label tests look for.
pub const VIEWPORT_LABEL: &str = "3D viewport";

/// The mouse buttons navigation listens to, in egui's terms and ours.
const BUTTONS: [(PointerButton, MouseButton); 3] = [
    (PointerButton::Primary, MouseButton::Primary),
    (PointerButton::Middle, MouseButton::Middle),
    (PointerButton::Secondary, MouseButton::Secondary),
];

/// How many views the viewport holds: perspective, top, front and right,
/// shown one at a time or all four at once.
pub const VIEW_COUNT: usize = 4;

/// The keys that show one view on its own (`F1` to `F4`), and the views
/// they show, as DESIGN.md §5.3 planned.
pub const VIEW_KEYS: [(egui::Key, ViewKind); 4] = [
    (egui::Key::F1, ViewKind::Perspective),
    (egui::Key::F2, ViewKind::Top),
    (egui::Key::F3, ViewKind::Right),
    (egui::Key::F4, ViewKind::Front),
];

/// The key that shows all four views at once.
pub const FOUR_VIEWS_KEY: egui::Key = egui::Key::F5;

/// The key that switches the active view between perspective and
/// orthographic (the project lead's choice, [decision 0069](../../../docs/decisions/0069-orthographic-view.md)).
pub(crate) const PROJECTION_KEY: egui::Key = egui::Key::P;

/// Gap between the four views, in logical pixels: a thin dark line.
const VIEW_GAP: f32 = 2.0;

/// One view: its camera and what the mouse is doing in it.
#[derive(Debug, Clone, PartialEq)]
struct View {
    navigator: Navigator,
    /// The navigation drag in progress, if any. Chosen when the drag
    /// starts, so letting go of a key mid-drag does not stop it.
    drag: Option<DragKind>,
    /// What the renderer last found under the mouse in this view.
    under_mouse: Option<UnderMouse>,
    /// A framing move in progress.
    glide: Option<Glide>,
    /// The image's size last frame, in logical pixels (for framing from a
    /// menu, before this frame's size is known).
    image_size: Vec2,
    /// A selection drag (box, lasso or brush) in progress, if any.
    select_drag: Option<select::SelectDrag>,
    /// Where the last click was, so a double-click can be told from two
    /// quick clicks in different places.
    last_click: Option<egui::Pos2>,
    /// A drag of the gizmo in progress, if any.
    gizmo_drag: Option<tools::Drag>,
    /// The gizmo handle the left button went down on, if any.
    pressed_handle: Option<tools::Handle>,
    /// The gizmo handle under the mouse, to light up.
    hovered_handle: Option<tools::Handle>,
    /// A modelling tool's drag (extrude, inset) in progress, if any.
    tool_drag: Option<tool_drag::ToolDrag>,
    /// Whether the pivot is a point on a surface you pointed at (rather
    /// than a point in mid-air left by zooming over empty space), so an
    /// orbit started over empty space may keep turning around it.
    pivot_on_surface: bool,
    /// While orbiting with Ctrl held, the view as the orbit would leave it
    /// without snapping; what is shown is it snapped to the nearest side
    /// of the view dice (decision 0084).
    free: Option<Navigator>,
    /// Whether the view was turned flat by the view dice (or snapping)
    /// with *Switch to orthographic when clicking the dice* on: orbiting
    /// away freely turns it back into a perspective view (decision 0084).
    auto_flat: bool,
}

impl View {
    /// A view of the given kind, at its opening camera.
    fn new(kind: ViewKind) -> Self {
        Self {
            navigator: Navigator::for_view(kind),
            drag: None,
            under_mouse: None,
            glide: None,
            image_size: Vec2::ZERO,
            select_drag: None,
            last_click: None,
            gizmo_drag: None,
            pressed_handle: None,
            hovered_handle: None,
            tool_drag: None,
            pivot_on_surface: false,
            free: None,
            auto_flat: false,
        }
    }

    fn camera(&self) -> &Camera {
        self.navigator.camera()
    }

    fn kind(&self) -> ViewKind {
        self.navigator.kind()
    }

    /// Switches between perspective and orthographic, stopping a glide.
    fn toggle_projection(&mut self) {
        self.glide = None;
        self.auto_flat = false;
        self.navigator = self.navigator.toggled_projection();
    }

    /// Goes back to the opening camera for this kind of view.
    fn reset(&mut self) {
        *self = Self {
            image_size: self.image_size,
            ..Self::new(self.kind())
        };
    }

    /// Starts gliding to fit `bounds` in the view, at time `now`.
    fn start_frame(&mut self, bounds: Bounds, now: f64) {
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
    /// dice, at time `now` ([decision 0084](../../../docs/decisions/0084-view-dice.md)).
    /// With `flat` (Display → Switch to orthographic when clicking the
    /// dice), a perspective view ends up orthographic, and turns back into
    /// perspective when orbited away.
    fn start_look_from(&mut self, side: glam::IVec3, flat: bool, now: f64) {
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
    fn follow_glide(&mut self, now: f64) -> bool {
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
    fn answer_at(&self, at: egui::Pos2) -> Option<Option<Vec3>> {
        self.under_mouse
            .filter(|u| u.at.distance(at) <= PICK_TOLERANCE && u.camera == *self.camera())
            .map(|u| u.point)
    }

    /// The surface point under `at`, if an up-to-date answer says so.
    fn surface_at(&self, at: egui::Pos2) -> Option<Vec3> {
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
    fn zoom_target(&self, at: egui::Pos2, rect: Rect) -> (Vec3, bool) {
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
    /// starts over empty space ([decision 0080](../../../docs/decisions/0080-navigation-feel.md)):
    /// the first of these that is in front of the camera and on screen,
    /// so the view never swings around something you can't see:
    ///
    /// 1. the pivot, if it is a surface point you pointed at before;
    /// 2. the middle of the selection;
    /// 3. the middle of everything shown;
    ///
    /// otherwise the middle of the view, at the pivot's depth.
    fn pivot_over_empty_space(&mut self, size: Vec2, scene: Option<&SceneSummary>) {
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
    fn navigate(
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
        // Alt + left drag on the Move gizmo slides points (decision 0055),
        // so it doesn't orbit there.
        let sliding = response.drag_started_by(PointerButton::Primary)
            && ui.input(|i| i.modifiers.alt)
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
    fn button_drag(&mut self, kind: DragKind, delta: egui::Vec2, settings: NavigationSettings) {
        self.glide = None;
        if !settings.around_cursor {
            self.navigator.centre_pivot();
        }
        let delta = settings.movement(kind, Vec2::new(delta.x, delta.y));
        self.navigator.drag(kind, delta, self.image_size);
    }

    /// What to ask the renderer for, for this view's image at `rect`.
    fn view(
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
            overlay: Arc::default(),
            shading: Shading::default(),
            auto_hard_angle: Some(AUTO_HARD_ANGLE),
            cavity: true,
        }
    }
}

/// The question about the point under the mouse that is waiting for its
/// answer.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Asked {
    /// Which view asked.
    slot: usize,
    /// The camera it asked with.
    camera: Camera,
    /// The view's top-left corner on screen, in logical pixels.
    corner: egui::Pos2,
}

/// The viewport panel's state.
#[derive(Debug, Clone)]
pub struct ViewportPanel {
    /// The four views, in layout order: perspective top left, top top
    /// right, front bottom left, right bottom right (each can be switched
    /// to another kind from the Cameras menu).
    views: [View; VIEW_COUNT],
    /// The view the menus and keys act on: the last one clicked. Shown on
    /// its own when the four-view layout is off.
    active: usize,
    options: ViewOptions,
    /// Set by Options → Navigation controls, to open the picker once its
    /// button has been drawn this frame.
    open_picker: bool,
    /// The question about the point under the mouse in flight, if any.
    asked: Option<Asked>,
    /// Whether the renderer is still working on a question.
    asking: bool,
    /// A framing asked for from a menu, done on the next frame.
    frame_request: Option<Frame>,
    /// The mouse over a view this frame, while not navigating.
    pointer: Option<ViewPointer>,
    /// What a left drag selects with.
    select_tool: SelectTool,
    /// The selection brush's radius, in screen points.
    brush_radius: f32,
    /// The last selection drag's number.
    strokes: u64,
    /// Ctrl + wheel turned but not yet a whole step.
    ctrl_scroll: f32,
    /// Selection gestures this frame, for the workbench.
    gestures: Vec<Gesture>,
    /// The gizmo to show and handle, if a transform tool is in use and
    /// something can move (set by the workbench each frame).
    gizmo: Option<tools::Gizmo>,
    /// Gizmo events this frame, for the workbench.
    gizmo_events: Vec<GizmoEvent>,
    /// Whether a modelling tool that is dragged anywhere (extrude, inset)
    /// is in use (set by the workbench each frame).
    drag_tool: bool,
    /// Whether a modelling tool used by clicking on shapes is in use.
    click_tool: bool,
    /// Whether the tool in use has a count Ctrl + wheel changes.
    wheel_tool: bool,
    /// Ctrl + wheel notches for that tool since last asked (up is more).
    tool_steps: i32,
    /// What the modelling tool's drag is doing, written by the mouse.
    tool_label: Option<String>,
    /// A line a tool shows over the views (Slit's).
    tool_path: Option<ToolPath>,
    /// Modelling tool drag events this frame, for the workbench.
    tool_events: Vec<ToolDragEvent>,
    /// Where the right mouse button was clicked (pressed and let go
    /// without dragging) in a view, if it was since last asked: the
    /// quad menu (decision 0094).
    right_click: Option<egui::Pos2>,
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self {
            views: ViewKind::ALL.map(View::new),
            active: 0,
            options: ViewOptions::default(),
            open_picker: false,
            asked: None,
            asking: false,
            frame_request: None,
            pointer: None,
            select_tool: SelectTool::default(),
            brush_radius: BRUSH_RADIUS,
            strokes: 0,
            ctrl_scroll: 0.0,
            gestures: Vec::new(),
            gizmo: None,
            gizmo_events: Vec::new(),
            drag_tool: false,
            click_tool: false,
            wheel_tool: false,
            tool_steps: 0,
            tool_label: None,
            tool_path: None,
            tool_events: Vec::new(),
            right_click: None,
        }
    }
}

impl ViewportPanel {
    fn active_view(&self) -> &View {
        &self.views[self.active.min(VIEW_COUNT - 1)]
    }

    fn active_view_mut(&mut self) -> &mut View {
        &mut self.views[self.active.min(VIEW_COUNT - 1)]
    }

    /// The camera of the active view (the one the menus act on).
    pub fn camera(&self) -> &Camera {
        self.active_view().camera()
    }

    /// The active view's camera and the pivot it orbits.
    pub fn navigator(&self) -> &Navigator {
        &self.active_view().navigator
    }

    /// The active view's camera and pivot, for moving them in tests.
    #[cfg(test)]
    pub(crate) fn navigator_mut(&mut self) -> &mut Navigator {
        &mut self.active_view_mut().navigator
    }

    /// Which kind of view the active view is.
    pub fn active_kind(&self) -> ViewKind {
        self.active_view().kind()
    }

    /// The kind of each of the four views, in layout order.
    pub fn view_kinds(&self) -> [ViewKind; VIEW_COUNT] {
        std::array::from_fn(|slot| self.views[slot].kind())
    }

    /// Which of the four views is active (0 top left, 1 top right, 2
    /// bottom left, 3 bottom right).
    pub fn active_slot(&self) -> usize {
        self.active
    }

    /// Whether all four views are shown at once.
    pub fn four_views(&self) -> bool {
        self.options.four_views
    }

    /// Shows all four views, or only the active one.
    pub fn set_four_views(&mut self, four: bool) {
        self.options.four_views = four;
    }

    /// Turns the active view into a view of `kind`, at its opening camera
    /// (Cameras menu). Nothing changes if it already is one.
    pub fn set_kind(&mut self, kind: ViewKind) {
        let view = self.active_view_mut();
        if view.kind() != kind {
            *view = View {
                image_size: view.image_size,
                ..View::new(kind)
            };
        }
    }

    /// Shows a view of `kind` on its own (keys `F1` to `F4`): the view of
    /// that kind if there is one, otherwise the active view turned into one.
    pub fn show_alone(&mut self, kind: ViewKind) {
        match self.views.iter().position(|v| v.kind() == kind) {
            Some(slot) => self.active = slot,
            None => self.set_kind(kind),
        }
        self.options.four_views = false;
    }

    /// Switches the active view between perspective and orthographic (key
    /// P, or the Cameras menu), keeping where it looks.
    pub fn toggle_projection(&mut self) {
        self.active_view_mut().toggle_projection();
    }

    /// Goes back to the opening camera of the active view (View → Reset
    /// view), keeping its kind.
    pub fn reset_view(&mut self) {
        self.active_view_mut().reset();
    }

    /// Fits everything, or the selection, in the active view (View → Frame
    /// all, Frame selected, or keys `H` and `F`): the camera keeps its
    /// direction and glides forwards or back until it fits. Nothing can be
    /// selected yet, so both frame the whole scene.
    pub fn frame(&mut self, what: Frame) {
        self.frame_request = Some(what);
    }

    /// The mouse over a view this frame (its camera, size and position),
    /// if it rests there and nothing is being navigated.
    pub(crate) fn pointer(&self) -> Option<ViewPointer> {
        self.pointer
    }

    /// What a left drag selects with.
    pub fn select_tool(&self) -> SelectTool {
        self.select_tool
    }

    /// Changes what a left drag selects with.
    pub fn set_select_tool(&mut self, tool: SelectTool) {
        self.select_tool = tool;
    }

    /// The selection brush's radius, in screen points.
    pub fn brush_radius(&self) -> f32 {
        self.brush_radius
    }

    /// Shows `gizmo` in the views (or none), from this frame on.
    pub(crate) fn set_gizmo(&mut self, gizmo: Option<tools::Gizmo>) {
        self.gizmo = gizmo;
    }

    /// The gizmo shown, if any.
    pub fn gizmo(&self) -> Option<tools::Gizmo> {
        self.gizmo
    }

    /// The gizmo events since last asked.
    pub(crate) fn take_gizmo_events(&mut self) -> Vec<GizmoEvent> {
        std::mem::take(&mut self.gizmo_events)
    }

    /// Whether the gizmo is being dragged in any view.
    pub fn dragging_gizmo(&self) -> bool {
        self.views.iter().any(View::dragging_gizmo)
    }

    /// Cancels a gizmo drag in progress (Esc). Returns whether there was
    /// one.
    pub(crate) fn cancel_gizmo_drag(&mut self) -> bool {
        let mut cancelled = false;
        for view in &mut self.views {
            if view.gizmo_drag.take().is_some() {
                view.pressed_handle = None;
                cancelled = true;
            }
        }
        if cancelled {
            self.gizmo_events.push(GizmoEvent::Cancel);
        }
        cancelled
    }

    /// Whether a modelling tool that is dragged anywhere is in use, from
    /// this frame on.
    pub(crate) fn set_drag_tool(&mut self, active: bool) {
        self.drag_tool = active;
    }

    /// Whether a modelling tool used by clicking on shapes is in use (it
    /// takes drags too), from this frame on.
    pub(crate) fn set_click_tool(&mut self, active: bool) {
        self.click_tool = active;
    }

    /// Whether the tool in use has a count that Ctrl + wheel changes
    /// (instead of growing the selection), from this frame on.
    pub(crate) fn set_wheel_tool(&mut self, active: bool) {
        self.wheel_tool = active;
    }

    /// The Ctrl + wheel notches for the tool's count since last asked (up
    /// is positive).
    pub(crate) fn take_tool_steps(&mut self) -> i32 {
        std::mem::take(&mut self.tool_steps)
    }

    /// The line a tool shows over the views, if any.
    pub(crate) fn set_tool_path(&mut self, path: Option<ToolPath>) {
        self.tool_path = path;
    }

    /// The line a tool shows over the views, if any.
    #[cfg(test)]
    pub(crate) fn tool_path(&self) -> Option<&ToolPath> {
        self.tool_path.as_ref()
    }

    /// What to write beside the mouse while a modelling tool is dragged.
    pub(crate) fn set_tool_label(&mut self, label: Option<String>) {
        self.tool_label = label;
    }

    /// The modelling tool drag events since last asked.
    pub(crate) fn take_tool_events(&mut self) -> Vec<ToolDragEvent> {
        std::mem::take(&mut self.tool_events)
    }

    /// Whether a modelling tool is being dragged in any view.
    pub fn dragging_tool(&self) -> bool {
        self.views.iter().any(View::dragging_tool)
    }

    /// Cancels a modelling tool's drag in progress (Esc). Returns whether
    /// there was one.
    pub(crate) fn cancel_tool_drag(&mut self) -> bool {
        let mut cancelled = false;
        for view in &mut self.views {
            cancelled |= view.tool_drag.take().is_some();
        }
        if cancelled {
            self.tool_events.push(ToolDragEvent::Cancel);
        }
        cancelled
    }

    /// Where the right mouse button was clicked in a view since last
    /// asked, if it was (decision 0094).
    pub(crate) fn take_right_click(&mut self) -> Option<egui::Pos2> {
        self.right_click.take()
    }

    /// The selection gestures made since last asked.
    pub(crate) fn take_gestures(&mut self) -> Vec<Gesture> {
        std::mem::take(&mut self.gestures)
    }

    /// Whether a framing glide is in progress in the active view.
    pub fn is_gliding(&self) -> bool {
        self.active_view().glide.is_some()
    }

    /// The navigation drag in progress in the active view, if any.
    pub fn active_drag(&self) -> Option<DragKind> {
        self.active_view().drag
    }

    /// The display and navigation choices.
    pub fn options(&self) -> ViewOptions {
        self.options
    }

    /// Changes the display and navigation choices.
    pub fn options_mut(&mut self) -> &mut ViewOptions {
        &mut self.options
    }

    /// Where each view shown goes in `body`, with its slot: the active view
    /// filling it, or all four in a 2 × 2 grid.
    fn layout(&self, body: Rect) -> Vec<(usize, Rect)> {
        if !self.options.four_views {
            return vec![(self.active.min(VIEW_COUNT - 1), body)];
        }
        let half = VIEW_GAP / 2.0;
        let mid = body.center();
        let columns = [(body.left(), mid.x - half), (mid.x + half, body.right())];
        let rows = [(body.top(), mid.y - half), (mid.y + half, body.bottom())];
        (0..VIEW_COUNT)
            .map(|slot| {
                let (left, right) = columns[slot % 2];
                let (top, bottom) = rows[slot / 2];
                let rect =
                    Rect::from_min_max(pos2(left, top), pos2(right.max(left), bottom.max(top)));
                (slot, rect)
            })
            .collect()
    }

    /// Draws the viewport into all of `ui`'s space: the menu line along the
    /// top, and below it one 3D image, or four.
    pub fn show(&mut self, ui: &mut Ui, renderer: &mut dyn ViewportRenderer, scene: &SceneSummary) {
        let full = ui.available_rect_before_wrap();
        let header = Rect::from_min_size(full.min, egui::vec2(full.width(), chrome::HEADER_HEIGHT));
        let body = Rect::from_min_max(pos2(full.min.x, header.max.y), full.max);
        ui.allocate_rect(full, Sense::hover());

        self.views_from_keys(ui);
        let shown = self.layout(body);
        for (slot, rect) in &shown {
            self.views[*slot].image_size = Vec2::new(rect.width(), rect.height());
        }
        self.frame_from_keys_and_menus(ui, scene);
        chrome::header(ui, header, self);

        let four = self.options.four_views;
        if four {
            ui.painter().rect_filled(body, 0.0, theme::BORDER);
        }
        let ppp = pixels_per_point(ui);
        let mut hovered = None;
        let mut toggle_layout = false;
        for (slot, rect) in shown.iter().copied() {
            let response = ui.interact(
                rect,
                ui.id().with(("viewport_body", slot)),
                Sense::click_and_drag(),
            );
            let label = if four {
                format!("{VIEWPORT_LABEL} ({})", self.views[slot].kind().name())
            } else {
                VIEWPORT_LABEL.to_string()
            };
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &label));
            // Pressing any button in a view makes it the active one.
            if response.contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
                self.active = slot;
            }
            // A middle click (not a drag) switches between one view and four.
            if response.clicked_by(PointerButton::Middle) {
                self.active = slot;
                toggle_layout = true;
            }
            let slide_gizmo = self
                .gizmo
                .filter(|g| matches!(g.tool, tools::GizmoTool::Move | tools::GizmoTool::All));
            self.views[slot].navigate(ui, &response, rect, &self.options, slide_gizmo, Some(scene));
            let tool_busy = self.views[slot].capture_tool_drag(
                ui,
                &response,
                rect,
                (self.drag_tool, self.click_tool),
                &mut self.tool_events,
            );
            let gizmo_busy = tool_busy
                || self.views[slot].capture_gizmo(
                    ui,
                    &response,
                    rect,
                    gizmo::GizmoInput {
                        gizmo: self.gizmo,
                        locked: self.options.locked,
                        out: &mut self.gizmo_events,
                    },
                );
            if !gizmo_busy
                && response.clicked_by(egui::PointerButton::Secondary)
                && let Some(at) = response.interact_pointer_pos()
            {
                self.right_click = Some(at);
            }
            if !gizmo_busy {
                self.views[slot].capture_selection(
                    ui,
                    &response,
                    rect,
                    select::Capture {
                        tool: self.select_tool,
                        keys: self.options.select_keys,
                        brush_radius: self.brush_radius,
                        strokes: &mut self.strokes,
                        out: &mut self.gestures,
                    },
                );
            }

            let mut view =
                self.views[slot].view(slot, rect, ppp, self.options.grid_detail, &scene.drawing);
            view.selection_color = rgb(theme::accent(ui.ctx()).colors().light);
            view.overlay = Arc::clone(&scene.overlay);
            view.shading = self.options.shading;
            view.auto_hard_angle = self.options.auto_hard_angle();
            view.cavity = !self.options.hide_cavity;
            match renderer.render(&view) {
                Ok(texture) => {
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    ui.painter_at(rect).image(texture, rect, uv, Color32::WHITE);
                }
                Err(reason) => chrome::no_image(ui, rect, &reason),
            }
            if response.hovered() {
                hovered = Some((slot, rect, response.hover_pos(), view));
            }
            mirror_plane::paint(ui, rect, self.views[slot].camera(), &scene.drawing.mirrors);
            self.views[slot].paint_gizmo(ui, rect, self.gizmo, self.options.locked);
            self.views[slot].paint_tool_path(ui, rect, self.tool_path.as_ref());
            self.views[slot].paint_tool_label(ui, rect, self.tool_label.as_deref());
            self.views[slot].paint_selection(
                ui,
                rect,
                self.select_tool,
                self.brush_radius,
                response.hover_pos(),
            );
            chrome::overlays(ui, rect, &self.views[slot], &self.options);
            // The view dice turns the view to look from the side clicked
            // (a flat Top, Front or Right view becomes a free Orthographic
            // one, as when orbited).
            if let Some(side) = dice::show(ui, rect, slot, self.views[slot].camera()) {
                self.active = slot;
                let now = ui.input(|i| i.time);
                let flat = self.options.flat_from_dice;
                self.views[slot].start_look_from(side, flat, now);
            }
            if renderer.preparing() {
                chrome::preparing(ui, rect);
                // Keep drawing, so the shape shows as soon as it is ready.
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            if four && slot == self.active {
                chrome::active_border(ui, rect);
            }
        }
        // Where the mouse is, for finding points, edges and polygons under
        // it: only while it rests over a view, not while navigating.
        self.pointer = hovered.as_ref().and_then(|(slot, rect, at, _)| {
            let view = &self.views[*slot];
            let at = (*at)?;
            (view.drag.is_none() && view.glide.is_none()).then(|| ViewPointer {
                camera: *view.camera(),
                size: Vec2::new(rect.width(), rect.height()),
                at: Vec2::new(at.x - rect.min.x, at.y - rect.min.y),
            })
        });
        if hovered.is_some() {
            self.ctrl_wheel(ui);
        }
        self.ask_what_is_under_the_mouse(ui, hovered, renderer);
        if toggle_layout {
            self.options.four_views = !four;
        }
        picker::show(ui, body, self);
        look::show(ui, body, self);
    }

    /// Ctrl + wheel over a view: with the brush, changes its size;
    /// otherwise grows or shrinks the selection, a step for each notch of
    /// the wheel.
    fn ctrl_wheel(&mut self, ui: &Ui) {
        if !ui.input(|i| i.modifiers.command) {
            self.ctrl_scroll = 0.0;
            return;
        }
        let scroll = wheel_scroll(ui, 400.0);
        if scroll == 0.0 {
            return;
        }
        // One notch of a mouse wheel; touchpads send many small turns.
        let notch = ui
            .ctx()
            .options(|o| o.input_options.line_scroll_speed)
            .max(1.0);
        self.ctrl_scroll += scroll;
        let steps = (self.ctrl_scroll / notch).trunc();
        if steps == 0.0 {
            return;
        }
        self.ctrl_scroll -= steps * notch;
        if self.wheel_tool {
            // A tool with a count (Bevel's segments, Loop cut's cuts) takes
            // the turns while it is in use (decision 0048).
            self.tool_steps += steps as i32;
        } else if self.select_tool == SelectTool::Brush {
            self.brush_radius =
                (self.brush_radius * BRUSH_STEP.powf(steps)).clamp(BRUSH_MIN, BRUSH_MAX);
        } else {
            self.gestures.push(Gesture::Grow(steps as i32));
        }
    }

    /// Shows one view or four, for keys `F1` to `F5` (unless typing in a
    /// field).
    fn views_from_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        for (key, kind) in VIEW_KEYS {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.show_alone(kind);
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, FOUR_VIEWS_KEY)) {
            self.options.four_views = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, PROJECTION_KEY)) {
            self.toggle_projection();
        }
    }

    /// Collects the renderer's latest answer about the point under the
    /// mouse, and asks again while the mouse is over a view, so an answer
    /// is ready when an orbit or zoom starts. `hovered` is the view under
    /// the mouse, if any: its slot, place, the mouse position and what was
    /// drawn.
    fn ask_what_is_under_the_mouse(
        &mut self,
        ui: &Ui,
        hovered: Option<(usize, Rect, Option<egui::Pos2>, ViewportView)>,
        renderer: &mut dyn ViewportRenderer,
    ) {
        let ppp = pixels_per_point(ui);
        let around_cursor = self.options.navigation.around_cursor;
        let mut ask = None;
        if let Some((slot, rect, Some(at), view)) = hovered
            && around_cursor
        {
            let it = &self.views[slot];
            // One question at a time, and only when the mouse has moved
            // since the last answer, so a resting mouse costs nothing.
            let fresh_needed = it
                .under_mouse
                .is_none_or(|u| u.at.distance(at) > 0.5 || u.camera != *it.camera());
            if it.drag.is_none() && !self.asking && fresh_needed {
                ask = Some(PickAsk {
                    view_projection: view.view_projection,
                    pixel: [(at.x - rect.min.x) * ppp, (at.y - rect.min.y) * ppp],
                    size_px: view.size_px,
                });
                self.asked = Some(Asked {
                    slot,
                    camera: *it.camera(),
                    corner: rect.min,
                });
            }
        }
        let reply = renderer.pick(ask);
        if let (Some(answer), Some(asked)) = (reply.answer, self.asked)
            && let Some(view) = self.views.get_mut(asked.slot)
        {
            view.under_mouse = Some(UnderMouse {
                at: asked.corner + egui::vec2(answer.pixel[0], answer.pixel[1]) / ppp,
                point: answer.point,
                camera: asked.camera,
            });
        }
        self.asking = reply.waiting;
        if reply.waiting {
            ui.ctx().request_repaint();
        }
    }

    /// Starts framing the active view for the `H` and `Z` keys (unless
    /// typing in a field) or a menu entry, and moves every view along a
    /// glide in progress.
    fn frame_from_keys_and_menus(&mut self, ui: &Ui, scene: &SceneSummary) {
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let now = ui.input(|i| i.time);
        let mut wanted = self.frame_request.take();
        if !typing {
            for what in [Frame::All, Frame::Selected] {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, what.key())) {
                    wanted = Some(what);
                }
            }
        }
        if let Some(what) = wanted {
            self.active_view_mut()
                .start_frame(scene.bounds_for(what), now);
        }
        let mut gliding = false;
        for view in &mut self.views {
            gliding |= view.follow_glide(now);
        }
        if gliding {
            ui.ctx().request_repaint();
        }
    }

    /// Applies a drag on one of the menu line's navigation buttons to the
    /// active view.
    fn button_drag(&mut self, kind: DragKind, delta: egui::Vec2) {
        let settings = self.options.navigation;
        self.active_view_mut().button_drag(kind, delta, settings);
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

    fn rect(w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(w, h))
    }

    fn perspective() -> View {
        View::new(ViewKind::Perspective)
    }

    #[test]
    fn the_view_is_sized_in_physical_pixels() {
        let view = perspective().view(
            0,
            rect(800.0, 500.0),
            1.5,
            GridDetail::Metre,
            &Arc::default(),
        );
        assert_eq!(view.size_px, [1200, 750]);
    }

    #[test]
    fn empty_or_broken_sizes_still_make_a_valid_view() {
        for ppp in [0.0, f32::NAN, 2.0] {
            let view =
                perspective().view(0, rect(0.0, 0.0), ppp, GridDetail::Metre, &Arc::default());
            assert_eq!(view.size_px, [1, 1]);
            assert!(view.view_projection.is_finite());
        }
    }

    #[test]
    fn the_view_uses_the_camera_pivot_and_backdrop() {
        let it = perspective();
        let view = it.view(
            0,
            rect(640.0, 400.0),
            1.0,
            GridDetail::Metre,
            &Arc::default(),
        );
        assert_eq!(view.camera_position, it.camera().position);
        assert_eq!(view.focus, it.navigator.pivot());
        assert_eq!(
            view.view_projection,
            it.camera().view_projection(Vec2::new(640.0, 400.0))
        );
        assert_eq!(view.sky_top, rgb(backdrop::SKY_TOP));
        assert_eq!(view.ground, rgb(backdrop::GROUND));
        assert_eq!(view.flat_height, None);
        assert_eq!(view.grid_plane, [Vec3::X, Vec3::Y]);
    }

    #[test]
    fn flat_views_tell_the_renderer_their_height_and_grid() {
        let it = View::new(ViewKind::Front);
        let view = it.view(
            2,
            rect(640.0, 400.0),
            1.0,
            GridDetail::TenCentimetres,
            &Arc::default(),
        );
        assert_eq!(view.slot, 2);
        assert_eq!(view.flat_height, it.camera().ortho_height);
        assert!(view.flat_height.is_some());
        assert_eq!(view.grid_plane, [Vec3::X, Vec3::Z]);
        assert_eq!(view.grid_detail, GridDetail::TenCentimetres);
    }

    #[test]
    fn the_four_views_open_as_perspective_top_front_and_right() {
        let panel = ViewportPanel::default();
        assert_eq!(panel.view_kinds(), ViewKind::ALL);
        assert_eq!(panel.active_kind(), ViewKind::Perspective);
        assert!(!panel.four_views(), "one view to start with");
        assert_eq!(*panel.navigator(), Navigator::default());
    }

    #[test]
    fn the_layout_shows_one_view_or_four() {
        let mut panel = ViewportPanel::default();
        let body = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(800.0, 600.0));
        assert_eq!(panel.layout(body), vec![(0, body)]);
        panel.active = 3;
        assert_eq!(panel.layout(body), vec![(3, body)], "the active view");
        panel.set_four_views(true);
        let four = panel.layout(body);
        assert_eq!(four.len(), 4);
        let (_, top_left) = four[0];
        let (_, bottom_right) = four[3];
        assert_eq!(top_left.min, body.min);
        assert_eq!(bottom_right.max, body.max);
        assert!(four[1].1.min.x > top_left.max.x, "a gap between them");
        assert!(four[2].1.min.y > top_left.max.y);
        for (_, r) in &four {
            assert!(body.contains_rect(*r));
            assert!((r.width() - (800.0 - VIEW_GAP) / 2.0).abs() < 1e-3);
        }
        // A body too small for the gap still gives valid (empty) rects.
        let tiny = panel.layout(rect(1.0, 1.0));
        assert!(
            tiny.iter()
                .all(|(_, r)| r.width() >= 0.0 && r.height() >= 0.0)
        );
    }

    #[test]
    fn view_keys_show_that_view_alone() {
        let mut panel = ViewportPanel::default();
        panel.set_four_views(true);
        panel.show_alone(ViewKind::Right);
        assert_eq!(panel.active_slot(), 3);
        assert_eq!(panel.active_kind(), ViewKind::Right);
        assert!(!panel.four_views());
        panel.show_alone(ViewKind::Top);
        assert_eq!(panel.active_slot(), 1);
    }

    #[test]
    fn a_missing_view_kind_takes_over_the_active_view() {
        let mut panel = ViewportPanel {
            active: 1,
            ..ViewportPanel::default()
        };
        panel.set_kind(ViewKind::Front);
        // Now two front views and no top view: F2 turns the active view
        // back into a top view.
        assert_eq!(panel.view_kinds()[1], ViewKind::Front);
        panel.active = 2;
        panel.show_alone(ViewKind::Top);
        assert_eq!(panel.active_slot(), 2);
        assert_eq!(panel.view_kinds()[2], ViewKind::Top);
    }

    #[test]
    fn changing_a_views_kind_starts_it_fresh_and_keeps_others() {
        let mut panel = ViewportPanel::default();
        panel.views[1].navigator.pan(Vec2::new(40.0, 0.0), 400.0);
        let top = panel.views[1].clone();
        panel.navigator_mut().orbit(Vec2::new(50.0, 0.0));
        panel.set_kind(ViewKind::Right);
        assert_eq!(*panel.navigator(), Navigator::for_view(ViewKind::Right));
        assert_eq!(panel.views[1], top, "the other views are untouched");
        let before = *panel.navigator();
        panel.set_kind(ViewKind::Right);
        assert_eq!(*panel.navigator(), before, "same kind: nothing changes");
    }

    #[test]
    fn the_view_carries_the_grid_detail_choice() {
        let mut panel = ViewportPanel::default();
        let r = rect(640.0, 400.0);
        let detail = |p: &ViewportPanel| {
            p.active_view()
                .view(0, r, 1.0, p.options.grid_detail, &Arc::default())
                .grid_detail
        };
        assert_eq!(detail(&panel), GridDetail::Metre, "calm by default");
        for level in GridDetail::ALL {
            panel.options_mut().grid_detail = level;
            assert_eq!(detail(&panel), level);
        }
    }

    #[test]
    fn resetting_the_view_restores_its_opening_camera() {
        let mut panel = ViewportPanel::default();
        panel.navigator_mut().orbit(Vec2::new(200.0, 50.0));
        panel.active_view_mut().drag = Some(DragKind::Orbit);
        panel.reset_view();
        assert_eq!(*panel.navigator(), Navigator::default());
        assert_eq!(panel.active_drag(), None);
        // A flat view resets to its own opening view, staying flat.
        panel.active = 1;
        panel.navigator_mut().pan(Vec2::new(30.0, 10.0), 400.0);
        panel.reset_view();
        assert_eq!(*panel.navigator(), Navigator::for_view(ViewKind::Top));
    }

    #[test]
    fn button_drags_respect_the_invert_setting() {
        let mut normal = ViewportPanel::default();
        normal.active_view_mut().image_size = Vec2::new(640.0, 400.0);
        normal.button_drag(DragKind::Orbit, egui::vec2(30.0, 0.0));
        let mut inverted = ViewportPanel::default();
        inverted.active_view_mut().image_size = Vec2::new(640.0, 400.0);
        inverted.options_mut().navigation.invert_orbit = true;
        inverted.button_drag(DragKind::Orbit, egui::vec2(30.0, 0.0));
        let start = Navigator::default().camera().yaw;
        assert!(normal.camera().yaw < start);
        assert!(inverted.camera().yaw > start);
    }

    #[test]
    fn view_options_survive_saving_and_old_files() {
        let mut options = ViewOptions {
            grid_detail: GridDetail::HalfMetre,
            four_views: true,
            ..ViewOptions::default()
        };
        options.navigation.scheme = tools::Scheme::MouseOnly;
        let text = ron::to_string(&options).unwrap();
        assert_eq!(ron::from_str::<ViewOptions>(&text).unwrap(), options);
        // Files saved before a setting existed still load, with defaults.
        let old: ViewOptions = ron::from_str("(four_views: false)").unwrap();
        assert_eq!(old.grid_detail, GridDetail::Metre);
        assert!(!old.four_views);
        assert_eq!(old.navigation, NavigationSettings::default());
    }

    #[test]
    fn an_old_fine_grid_setting_becomes_the_finest_grid() {
        let old: ViewOptions = ron::from_str("(fine_grid: true)").unwrap();
        let options = old.migrated();
        assert_eq!(options.grid_detail, GridDetail::TenCentimetres);
        let text = ron::to_string(&options).unwrap();
        assert!(
            !text.contains("fine_grid"),
            "the old name is not written back"
        );
        let off: ViewOptions = ron::from_str("(fine_grid: false)").unwrap();
        assert_eq!(off.migrated().grid_detail, GridDetail::Metre);
    }

    #[test]
    fn grid_levels_go_round_from_calm_to_detailed() {
        let mut level = GridDetail::default();
        let mut seen = vec![level];
        for _ in 0..2 {
            level = level.next();
            seen.push(level);
        }
        assert_eq!(seen, GridDetail::ALL);
        assert_eq!(level.next(), GridDetail::Metre);
        assert_eq!(GridDetail::Metre.fine_spacing(), None);
        assert_eq!(GridDetail::HalfMetre.fine_spacing(), Some(50.0));
        assert_eq!(GridDetail::TenCentimetres.fine_spacing(), Some(10.0));
    }

    #[test]
    fn answers_go_stale_when_the_camera_moves() {
        let mut it = perspective();
        let at = pos2(100.0, 100.0);
        it.under_mouse = Some(UnderMouse {
            at,
            point: Some(Vec3::new(1.0, 2.0, 3.0)),
            camera: *it.camera(),
        });
        assert_eq!(it.surface_at(at), Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(it.answer_at(pos2(140.0, 100.0)), None, "another spot");
        it.navigator.orbit(Vec2::new(10.0, 0.0));
        assert_eq!(it.answer_at(at), None, "the camera has moved");
    }

    #[test]
    fn without_an_answer_zooming_keeps_the_current_distance() {
        let it = perspective();
        let (target, on_surface) = it.zoom_target(pos2(500.0, 350.0), rect(640.0, 400.0));
        let distance = it.camera().position.distance(target);
        assert!((distance - it.navigator.distance()).abs() < 0.01);
        assert!(!on_surface);
    }

    #[test]
    fn zooming_over_empty_ground_never_moves_the_pivot_farther_away() {
        let mut it = perspective();
        let at = pos2(320.0, 300.0);
        it.under_mouse = Some(UnderMouse {
            at,
            point: None,
            camera: *it.camera(),
        });
        let (target, on_surface) = it.zoom_target(at, rect(640.0, 400.0));
        assert!(!on_surface);
        let distance = it.camera().position.distance(target);
        assert!(distance <= it.navigator.distance() + 0.01, "{distance}");
    }

    /// A summary whose selection is a box around `centre`.
    fn selected_at(centre: Vec3) -> SceneSummary {
        let half = Vec3::splat(10.0);
        SceneSummary {
            selected: scene::Bounds::new(centre - half, centre + half),
            all: scene::Bounds::new(Vec3::splat(-5000.0), Vec3::splat(5000.0)),
            ..SceneSummary::default()
        }
    }

    #[test]
    fn over_empty_space_the_view_turns_around_something_you_can_see() {
        let size = Vec2::new(640.0, 400.0);
        let mut it = perspective();
        // A pivot left in mid-air by zooming over nothing is let go of:
        // the selection on screen is used instead.
        it.navigator
            .set_pivot(it.camera().position + it.camera().forward() * 5000.0);
        it.pivot_on_surface = false;
        let selection = tools::camera::DEFAULT_TARGET;
        it.pivot_over_empty_space(size, Some(&selected_at(selection)));
        assert!(it.navigator.pivot().distance(selection) < 1e-3);
        // A surface point you pointed at, still on screen, is kept.
        it.pivot_on_surface = true;
        let kept = it.navigator.pivot();
        it.pivot_over_empty_space(size, Some(&selected_at(Vec3::new(9e6, 0.0, 0.0))));
        assert_eq!(it.navigator.pivot(), kept);
        // Nothing on screen at all: the middle of the view.
        it.pivot_on_surface = false;
        let behind = it.camera().position - it.camera().forward() * 500.0;
        let summary = SceneSummary {
            selected: scene::Bounds::new(behind, behind + Vec3::ONE),
            all: scene::Bounds::new(behind, behind + Vec3::ONE),
            ..SceneSummary::default()
        };
        it.pivot_over_empty_space(size, Some(&summary));
        let on = it.camera().project(it.navigator.pivot(), size).unwrap();
        assert!((on - size / 2.0).length() < 0.5, "{on}");
    }

    #[test]
    fn framing_glides_into_place_and_stops() {
        let mut it = View {
            image_size: Vec2::new(640.0, 400.0),
            ..perspective()
        };
        let start = it.navigator;
        it.start_frame(DEFAULT_FRAME, 10.0);
        let end = start.framed(&DEFAULT_FRAME, 1.6);
        assert!(it.follow_glide(10.05), "still going");
        let partway = it.camera().position;
        assert_ne!(partway, start.camera().position);
        assert_ne!(partway, end.camera().position);
        assert!(!it.follow_glide(10.0 + GLIDE_SECONDS + 0.01), "finished");
        assert_eq!(it.navigator, end);
        assert!(it.glide.is_none());
    }

    #[test]
    fn framing_a_flat_view_keeps_it_flat() {
        let mut it = View {
            image_size: Vec2::new(640.0, 400.0),
            ..View::new(ViewKind::Front)
        };
        it.start_frame(DEFAULT_FRAME, 0.0);
        it.follow_glide(1.0);
        assert_eq!(it.kind(), ViewKind::Front);
        assert_eq!(it.navigator.pivot(), DEFAULT_FRAME.centre());
    }

    #[test]
    fn taking_hold_of_the_view_stops_a_glide() {
        let mut panel = ViewportPanel::default();
        panel.active_view_mut().image_size = Vec2::new(640.0, 400.0);
        panel.active_view_mut().start_frame(DEFAULT_FRAME, 0.0);
        panel.active_view_mut().follow_glide(0.05);
        panel.button_drag(DragKind::Pan, egui::vec2(5.0, 0.0));
        assert!(!panel.is_gliding());
        panel.active_view_mut().start_frame(DEFAULT_FRAME, 0.0);
        panel.reset_view();
        assert!(!panel.is_gliding());
    }

    #[test]
    fn the_glide_eases_out() {
        let a = Navigator::default();
        let b = a.framed(&DEFAULT_FRAME, 1.6);
        let glide = Glide {
            from: a,
            to: b,
            start: 0.0,
        };
        let travelled = |t: f64| {
            glide
                .at(t)
                .0
                .camera()
                .position
                .distance(a.camera().position)
                / b.camera().position.distance(a.camera().position)
        };
        assert!(
            travelled(GLIDE_SECONDS / 2.0) > 0.6,
            "most of the way by half time"
        );
        assert!(!glide.at(-1.0).1, "not started yet");
        assert!(glide.at(f64::NAN).1, "broken clocks end the glide");
    }
}
