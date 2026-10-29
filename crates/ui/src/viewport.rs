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

mod brush;
pub(crate) mod chrome;
mod dice;
mod gizmo;
mod input;
pub(crate) mod look;
pub(crate) mod paint_row;
pub(crate) use paint_row::group_frame as paint_group_frame;
mod mirror_plane;
mod options;
mod picker;
mod player;
mod renderer;
pub(crate) mod scene_look;
mod select;
mod tool_drag;
mod view;

use crate::text::tr;
use egui::{Color32, PointerButton, Rect, Sense, Ui, pos2};
use glam::Vec2;
use tools::{Camera, DragKind, Navigator, ViewKind};

use crate::components::ViewPointer;
use crate::drawing::SceneSummary;
use crate::selecting::{BRUSH_RADIUS, Gesture, SelectTool};
use crate::theme;
pub(crate) use brush::{BrushCapture, BrushEvent};

/// The picture projection paints, as laid over each view (decision 0186).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StencilOverlay {
    /// Its texture.
    pub(crate) texture: egui::TextureId,
    /// Its size on screen, in points.
    pub(crate) size: Vec2,
    /// How far its middle is from the view's middle, in points.
    pub(crate) offset: Vec2,
    /// How strongly it shows, 0 to 1.
    pub(crate) opacity: f32,
}
pub(crate) use gizmo::GizmoEvent;
use std::sync::Arc;
pub(crate) use tool_drag::{ToolDragEvent, ToolPath};

pub(crate) use chrome::HEADER_HEIGHT;
#[cfg(test)]
pub(crate) use look::SHADING_LABEL;
#[cfg(test)]
pub(crate) use picker::AROUND_CURSOR_LABEL;
pub use picker::PICKER_LABEL;

#[cfg(test)]
pub(crate) use dice::DICE_LABEL;
use input::cursor_for;
pub use options::{
    AUTO_HARD_ANGLE, GRID_IMAGE_SIZE, GridDetail, PaintPreview, Shading, Surface, Symmetry,
    ViewOptions, checker_squares, grid_image,
};
pub use player::PlayerView;
pub(crate) use player::SCREEN_HEIGHTS;
pub use renderer::{NoRenderer, PickAsk, PickReply, Picked, ViewportRenderer, ViewportView};
pub use scene_look::{LookChoice, SceneLook};
pub use view::{DEFAULT_FRAME, Frame, GLIDE_SECONDS};
use view::{View, rgb, sane_scale};

/// Where to click on the view dice of the view at `rect` to look from
/// `side`, if it shows. For tests.
#[cfg(test)]
pub(crate) fn dice_point(camera: &Camera, rect: Rect, side: glam::IVec3) -> Option<egui::Pos2> {
    dice::point_for(camera, rect, side)
}

/// Accessible name of the 3D image, and the label tests look for.
pub const VIEWPORT_LABEL: &str = crate::text!("3D viewport");

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
    /// Image pixels per logical pixel: the screen's scale, times the
    /// player view's shrink (decision 0200).
    scale: f32,
}

/// The viewport panel's state.
#[derive(Debug, Clone)]
pub struct ViewportPanel {
    /// Where the Shading and navigation buttons were last drawn, at the
    /// bottom left and right: the paint tools sit between them (decision
    /// 0213).
    bottom_buttons: Option<(Rect, Rect)>,
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
    /// Set by the Display and Options menus' Settings entry, for the
    /// workbench to open the Settings window (decision 0126: every setting
    /// has its home there).
    open_settings: bool,
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
    /// A note across the top of the views while something unusual is
    /// edited (decision 0165: a level made by hand), with a button.
    banner: Option<String>,
    /// Whether the banner's button was clicked this frame.
    banner_clicked: bool,
    /// A line a tool shows over the views (Slit's).
    tool_path: Option<ToolPath>,
    /// Modelling tool drag events this frame, for the workbench.
    tool_events: Vec<ToolDragEvent>,
    /// Where the right mouse button was clicked (pressed and let go
    /// without dragging) in a view, if it was since last asked: the
    /// quad menu (decision 0094).
    right_click: Option<egui::Pos2>,
    /// Whether a right click finishes the tool in use (Draw curve,
    /// decision 0138), set by the workbench each frame.
    finish_clicks: bool,
    /// Whether a right click finished it since last asked.
    finish_click: bool,
    /// The paint brush in Paint mode (decision 0179): its radius and
    /// hardness, set by the workbench each frame; `None` in other modes.
    paint_brush: Option<(f32, f32)>,
    /// The Paint page is showing (decision 0223).
    paint_page: bool,
    /// Where its view controls were drawn last frame (decision 0224).
    paint_controls: Option<Rect>,
    /// The paint brush's state in the views.
    brush: brush::BrushCapture,
    /// The picture projection paints, laid over each view (decision 0186).
    stencil: Option<StencilOverlay>,
    /// Where mirror painting's other dab would go (decision 0187): its
    /// middle in the world and its radius in scene units.
    mirror_mark: Option<(glam::Vec3, f32)>,
    /// Brush events this frame, for the workbench.
    brush_events: Vec<BrushEvent>,
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self {
            bottom_buttons: None,
            views: ViewKind::ALL.map(View::new),
            active: 0,
            options: ViewOptions::default(),
            open_picker: false,
            open_settings: false,
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
            banner: None,
            banner_clicked: false,
            tool_path: None,
            tool_events: Vec::new(),
            right_click: None,
            finish_clicks: false,
            finish_click: false,
            paint_brush: None,
            paint_page: false,
            paint_controls: None,
            brush: brush::BrushCapture::default(),
            stencil: None,
            mirror_mark: None,
            brush_events: Vec::new(),
        }
    }
}

impl ViewportPanel {
    /// Where the Shading and the navigation buttons were last drawn
    /// (bottom left, bottom right), once the panel has been shown.
    pub(crate) fn bottom_buttons(&self) -> Option<(Rect, Rect)> {
        self.bottom_buttons
    }

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

    /// Whether a right click (not a drag) finishes the tool in use rather
    /// than opening the quad menu (Draw curve, decision 0138), from this
    /// frame on.
    pub(crate) fn set_finish_clicks(&mut self, active: bool) {
        self.finish_clicks = active;
    }

    /// Whether a right click finished the tool since last asked.
    pub(crate) fn take_finish_click(&mut self) -> bool {
        std::mem::take(&mut self.finish_click)
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

    /// The note across the top of the views (decision 0165), or none.
    pub(crate) fn set_banner(&mut self, banner: Option<String>) {
        self.banner = banner;
    }

    /// Whether the banner's button was clicked since last asked.
    pub(crate) fn take_banner_click(&mut self) -> bool {
        std::mem::take(&mut self.banner_clicked)
    }

    /// Whether a viewport menu's "All settings…" was clicked since last
    /// asked.
    pub(crate) fn take_open_settings(&mut self) -> bool {
        std::mem::take(&mut self.open_settings)
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

    /// Shows the paint brush (Paint mode, decision 0179) with `radius`
    /// (points) and `hardness`, or not (`None`): a left drag paints
    /// instead of selecting.
    /// The paint brush shown: its radius in points (0: a cross, for a
    /// tool with no size) and hardness.
    #[cfg(test)]
    pub(crate) fn paint_brush(&self) -> Option<(f32, f32)> {
        self.paint_brush
    }

    pub(crate) fn set_paint_brush(&mut self, brush: Option<(f32, f32)>) {
        self.paint_brush = brush;
    }

    /// Tells the viewport whether the Paint page is showing (decisions
    /// 0223 and 0224): one view, no labels, the grid only when asked for,
    /// and the view's controls in the tool row along its bottom.
    pub(crate) fn set_paint_page(&mut self, on: bool) {
        self.paint_page = on;
    }

    /// Whether the Paint page is showing.
    pub(crate) fn painting(&self) -> bool {
        self.paint_page
    }

    /// Where the Paint page's view controls are (decision 0224), for the
    /// tools to follow them.
    pub(crate) fn paint_controls(&self) -> Option<Rect> {
        self.paint_controls.filter(|_| self.paint_page)
    }

    /// Lays the picture projection paints over each view, or takes it
    /// away (decision 0186).
    pub(crate) fn set_stencil_overlay(&mut self, overlay: Option<StencilOverlay>) {
        self.stencil = overlay;
    }

    /// Shows where mirror painting's other dab would go, or not
    /// (decision 0187): its middle in the world and its radius in scene
    /// units.
    pub(crate) fn set_mirror_mark(&mut self, mark: Option<(glam::Vec3, f32)>) {
        self.mirror_mark = mark;
    }

    /// The brush events since last asked.
    pub(crate) fn take_brush_events(&mut self) -> Vec<BrushEvent> {
        std::mem::take(&mut self.brush_events)
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
        // Painting always shows one view (decision 0223).
        if !self.options.four_views || self.painting() {
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
        // The Paint page has no menu line: its menus are in the tool row's
        // ☰ (decision 0224).
        let header_height = if self.painting() {
            0.0
        } else {
            chrome::HEADER_HEIGHT
        };
        let header = Rect::from_min_size(full.min, egui::vec2(full.width(), header_height));
        let body = Rect::from_min_max(pos2(full.min.x, header.max.y), full.max);
        ui.allocate_rect(full, Sense::hover());

        self.views_from_keys(ui);
        let shown = self.layout(body);
        for (slot, rect) in &shown {
            self.views[*slot].image_size = Vec2::new(rect.width(), rect.height());
        }
        self.frame_from_keys_and_menus(ui, scene);
        if !self.painting() {
            chrome::header(ui, header, self);
        }

        let four = self.options.four_views && !self.painting();
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
                crate::trf!(
                    "{} ({})",
                    tr(VIEWPORT_LABEL),
                    self.views[slot].kind().name()
                )
            } else {
                tr(VIEWPORT_LABEL).to_string()
            };
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &label));
            // Pressing any button in a view makes it the active one.
            if response.contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
                self.active = slot;
            }
            // A middle click (not a drag) switches between one view and
            // four; not while painting, which shows one (decision 0223).
            if response.clicked_by(PointerButton::Middle) && !self.painting() {
                self.active = slot;
                toggle_layout = true;
            }
            let slide_gizmo = self
                .gizmo
                .filter(|g| matches!(g.tool, tools::GizmoTool::Move | tools::GizmoTool::All));
            // Ctrl + drag changes the paint brush instead of navigating.
            let brush_drag = self.paint_brush.is_some()
                && (self.brush.adjusting() || brush::BrushCapture::wants(ui, &response));
            if !brush_drag {
                self.views[slot].navigate(
                    ui,
                    &response,
                    rect,
                    &self.options,
                    slide_gizmo,
                    Some(scene),
                );
            }
            if self.paint_brush.is_some() {
                let camera = *self.views[slot].camera();
                if !self.views[slot].navigating() || self.brush.adjusting() {
                    let size = Vec2::new(rect.width(), rect.height());
                    let pointer_at = |p: egui::Pos2| ViewPointer {
                        camera,
                        size,
                        at: Vec2::new(p.x - rect.min.x, p.y - rect.min.y),
                    };
                    self.brush
                        .capture(ui, &response, pointer_at, &mut self.brush_events);
                }
            }
            let tool_busy = self.views[slot].capture_tool_drag(
                ui,
                &response,
                rect,
                (self.drag_tool, self.click_tool),
                &mut self.tool_events,
            );
            if self.finish_clicks
                && !self.views[slot].dragging_tool()
                && response.clicked_by(egui::PointerButton::Secondary)
            {
                self.finish_click = true;
            }
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
            if !gizmo_busy && self.paint_brush.is_none() {
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
            view.grid_colors = theme::grid_colors();
            view.overlay = Arc::clone(&scene.overlay);
            view.shading = self.options.shading;
            view.show_edges = self.options.show_edges;
            // The grid while painting only when asked for (decision 0223).
            view.show_grid = !self.painting() || self.options.paint_grid;
            view.surface = self.options.surface;
            view.paint_preview = self.options.paint_preview;
            view.light_turn = self.options.light_turn;
            // Cavity's shading would change what a preview other than Lit
            // shows (decision 0198).
            view.cavity = !self.options.hide_cavity
                && (view.surface != Surface::Painted || view.paint_preview == PaintPreview::Lit);
            view.high_contrast = self.options.high_contrast;
            // The look chosen in Settings → Viewport (decision 0230).
            let look = self.options.look.look(&self.options.custom_look);
            [view.sky_top, view.sky_horizon, view.ground] = look.backdrop();
            view.look = look;
            // See it as the player will (decision 0200): a smaller image,
            // enlarged with its pixels kept.
            let shrink = if self.options.player_view {
                self.options.player.scale(
                    self.views[slot].camera(),
                    view.focus,
                    view.size_px[1] as f32,
                )
            } else {
                1.0
            };
            if shrink < 1.0 {
                view.size_px = view
                    .size_px
                    .map(|v| ((v as f32 * shrink).round() as u32).max(1));
                // Its lines as wide as the image is enlarged makes them.
                view.pixels_per_point *= shrink;
            }
            match renderer.render(&view) {
                Ok(texture) => {
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    ui.painter_at(rect).image(texture, rect, uv, Color32::WHITE);
                }
                Err(reason) => chrome::no_image(ui, rect, &reason, &view.look),
            }
            if self.options.player_view {
                player::label(ui, rect, &self.options.player, shrink >= 1.0);
            }
            if response.hovered() {
                hovered = Some((slot, rect, response.hover_pos(), view));
            }
            mirror_plane::paint(ui, rect, self.views[slot].camera(), &scene.drawing.mirrors);
            self.views[slot].paint_gizmo(ui, rect, self.gizmo, self.options.locked);
            self.views[slot].paint_tool_path(ui, rect, self.tool_path.as_ref());
            if let Some(stencil) = &self.stencil {
                let middle = rect.center() + egui::vec2(stencil.offset.x, stencil.offset.y);
                let area =
                    Rect::from_center_size(middle, egui::vec2(stencil.size.x, stencil.size.y));
                ui.painter_at(rect).image(
                    stencil.texture,
                    area,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE.gamma_multiply(stencil.opacity.clamp(0.0, 1.0)),
                );
            }
            if let Some((radius, hardness)) = self.paint_brush
                && !self.views[slot].navigating()
            {
                self.brush
                    .paint(ui, rect, response.hover_pos(), radius, hardness);
                // Mirror painting's other dab, in the view the mouse is
                // over.
                if let Some((world, world_radius)) = self.mirror_mark
                    && response.hover_pos().is_some()
                {
                    let camera = self.views[slot].camera();
                    let size = glam::Vec2::new(rect.width(), rect.height());
                    if let Some(at) = camera.project(world, size) {
                        let per_point = camera.world_per_pixel(world, size).max(1e-6);
                        brush::paint_mirror_mark(
                            ui,
                            rect,
                            rect.min + egui::vec2(at.x, at.y),
                            world_radius / per_point,
                        );
                    }
                }
            }
            self.views[slot].paint_tool_label(ui, rect, self.tool_label.as_deref());
            self.views[slot].paint_selection(
                ui,
                rect,
                self.select_tool,
                self.brush_radius,
                response.hover_pos(),
            );
            if !self.painting() {
                chrome::overlays(ui, rect, &self.views[slot], &self.options);
            }
            // The view dice turns the view to look from the side clicked
            // (a flat Top, Front or Right view becomes a free Orthographic
            // one, as when orbited).
            if !self.painting()
                && let Some(side) = dice::show(ui, rect, slot, self.views[slot].camera())
            {
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
        if let Some(banner) = self.banner.clone()
            && chrome::banner(ui, body, &banner)
        {
            self.banner_clicked = true;
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
        // The Paint page's controls in place of the shading and navigation
        // buttons (decision 0224).
        if self.painting() {
            let controls = paint_row::show(ui, body, self);
            self.paint_controls = Some(controls);
            self.bottom_buttons = Some((controls, controls));
            return;
        }
        self.paint_controls = None;
        let navigation = picker::show(ui, body, self);
        let shading = look::show(ui, body, self);
        self.bottom_buttons = Some((shading, navigation));
    }
}

/// Physical pixels per logical pixel, never zero or broken.
fn pixels_per_point(ui: &Ui) -> f32 {
    sane_scale(ui.ctx().pixels_per_point())
}

#[cfg(test)]
mod tests {
    use super::view::{Glide, UnderMouse};
    use super::*;
    use crate::theme::backdrop;
    use glam::Vec3;
    use tools::NavigationSettings;

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
