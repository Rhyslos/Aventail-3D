//! The UV Editor panel ([decision 0096](../../../docs/decisions/0096-uv-editor.md)):
//! the selected objects' UVs, laid flat on the texture square.
//!
//! It shows the UVs of every selected object with a shape, each polygon's
//! outline, island borders a little brighter, and in the Points, Edges,
//! Polygons and UV modes what is selected, in the accent colour. Clicks and
//! box drags select the same points, edges and polygons as the viewport
//! (one selection, seen in both); in the UV mode a click takes the whole
//! island. It moves like the viewport (the project lead's choice): the
//! navigation controls pan (a flat view has nothing to orbit), the wheel
//! zooms at the mouse, H frames the texture square and Z the selection.
//!
//! With the Move, Rotate, Scale or all-gizmos tool, a flat gizmo moves,
//! turns and scales the selected UVs (see [`gizmo`]).

use crate::palette::Mode;
use crate::selecting::SelectKeys;
use crate::theme;
use crate::transforming::{SnapSettings, Tool};
use egui::{
    Align2, Color32, FontId, Key, Pos2, Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};
use glam::Vec2;
use mesh::uvs::Uvs;
use scene::{
    Combine, Document, EdgeKey, Element, ElementKind, ElementSelection, FaceId, Mesh, ObjectId,
    ObjectKind, VertexId,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tools::{DragKind, Modifiers, MouseButton, NavigationSettings};

/// Accessible name of the UV Editor's drawing, and the label tests look for.
pub(crate) const UV_EDITOR_LABEL: &str = "UV Editor view";

/// More polygons than this in the shown objects together, and their UVs
/// aren't drawn (a heavy imported shape): drawing them would freeze the
/// window.
pub(crate) const UV_POLYGONS_UP_TO: usize = 300_000;

/// How close, in screen points, a click must be to a point or an edge.
const PICK_DISTANCE: f32 = 9.0;

/// How many screen points a texture pixel must cover before the pixels
/// are drawn.
const PIXELS_FROM: f32 = 8.0;

/// Zoom limits, in screen points per texture width.
const MIN_ZOOM: f32 = 20.0;
const MAX_ZOOM: f32 = 200_000.0;

/// One object's UVs, as drawn and picked.
#[derive(Debug, Clone)]
struct UvShape {
    /// The shape they were worked out from, to tell when it changes.
    mesh: Arc<Mesh>,
    /// Each polygon's corners: its point and UV.
    polygons: Vec<(FaceId, Vec<(VertexId, Vec2)>)>,
    /// Edges as drawn once each: the two UVs, the mesh edge, and whether it
    /// borders an island (a seam or the shape's open edge).
    edges: Vec<([Vec2; 2], EdgeKey, bool)>,
    /// The island of each polygon, by slot.
    island_of: Vec<usize>,
    /// The polygons of each island.
    islands: Vec<Vec<FaceId>>,
    /// The edges marked as seams, drawn orange (decision 0098).
    seams: BTreeSet<EdgeKey>,
    /// Each polygon's UV area and real area, in the order of `polygons`,
    /// for its texel density (decision 0100).
    areas: Vec<(f32, f32)>,
    /// How evenly each polygon lies, in the order of `polygons`
    /// (decision 0102).
    stretch: Vec<uv::stretch::Stretch>,
    /// The spots of pinned UVs, each once (decision 0109).
    pins: Vec<Vec2>,
    /// The edges with a seam preference (decision 0111).
    preferences: HashMap<EdgeKey, scene::SeamPreference>,
    /// Whether each polygon, in the order of `polygons`, is locked
    /// (decision 0112).
    locked: Vec<bool>,
}

impl UvShape {
    fn of(mesh: Arc<Mesh>) -> Option<Self> {
        let uvs = Uvs::of(&mesh)?;
        let (starts, corners) = mesh.face_layout();
        let mut polygons = Vec::new();
        // Each side of each polygon, by its edge: the UVs from both sides.
        let mut sides: HashMap<EdgeKey, Vec<[Vec2; 2]>> = HashMap::new();
        for (face, list) in mesh.faces() {
            let start = starts[face.get() as usize] as usize;
            let points: Vec<(VertexId, Vec2)> = (0..list.len())
                .map(|k| (corners[start + k], uvs.at(start + k)))
                .collect();
            for k in 0..points.len() {
                let (a, ua) = points[k];
                let (b, ub) = points[(k + 1) % points.len()];
                let key = EdgeKey::new(a, b);
                let pair = if a < b { [ua, ub] } else { [ub, ua] };
                sides.entry(key).or_default().push(pair);
            }
            polygons.push((face, points));
        }
        let mut edges = Vec::new();
        for (key, uses) in sides {
            let joined = |p: &[Vec2; 2], q: &[Vec2; 2]| {
                p[0].distance(q[0]) <= mesh::uvs::SAME_UV
                    && p[1].distance(q[1]) <= mesh::uvs::SAME_UV
            };
            match uses.as_slice() {
                [only] => edges.push((*only, key, true)),
                [p, q] if joined(p, q) => edges.push((*p, key, false)),
                many => {
                    for p in many {
                        edges.push((*p, key, true));
                    }
                }
            }
        }
        edges.sort_by_key(|(_, key, _)| *key);
        let (island_of, count) = mesh::uvs::island_of_each(&mesh);
        let mut islands = vec![Vec::new(); count];
        for (face, _) in &polygons {
            if let Some(i) = island_of.get(face.get() as usize).copied()
                && i < count
            {
                islands[i].push(*face);
            }
        }
        let seams = mesh::uvs::seams(&mesh).into_iter().collect();
        let stretch = uv::stretch::stretch_of(&mesh)
            .map(|list| list.into_iter().map(|(_, s)| s).collect())
            .unwrap_or_default();
        let areas = polygons
            .iter()
            .map(|(face, _)| uv::density::areas(&mesh, &[*face], &uvs))
            .collect();
        let mut pins: Vec<Vec2> = Vec::new();
        if let Some(pinned) = mesh::uvs::pins(&mesh) {
            for (k, pinned) in pinned.iter().enumerate() {
                let spot = uvs.at(k);
                if *pinned && !pins.iter().any(|p| p.distance(spot) <= mesh::uvs::SAME_UV) {
                    pins.push(spot);
                }
            }
        }
        let preferences = mesh::uvs::seam_preferences(&mesh).into_iter().collect();
        let lock = mesh::uvs::locked(&mesh);
        let locked = polygons
            .iter()
            .map(|(f, _)| {
                lock.as_ref()
                    .is_some_and(|l| l.get(f.get() as usize).copied().unwrap_or(false))
            })
            .collect();
        Some(Self {
            mesh,
            polygons,
            edges,
            island_of,
            islands,
            seams,
            areas,
            stretch,
            pins,
            preferences,
            locked,
        })
    }
}

pub(crate) mod gizmo;
pub(crate) mod snapshots;

/// What a click in the editor found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hit {
    id: ObjectId,
    element: Element,
}

/// A drag in the editor.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EditorDrag {
    /// Moving the view.
    Navigate(DragKind),
    /// A selection box, from where it started.
    Box { from: Pos2, how: Combine },
    /// The gizmo (the drag itself is in `UvEditor::gizmo`).
    Gizmo,
}

/// The UV Editor's view and state.
#[derive(Debug, Clone)]
pub(crate) struct UvEditor {
    /// The UV at the middle of the panel.
    centre: Vec2,
    /// Screen points per texture width.
    zoom: f32,
    /// Whether the view has been fitted to the panel yet.
    fitted: bool,
    /// Where it was drawn last frame, to tell whether the mouse is over it.
    rect: Option<Rect>,
    drag: Option<EditorDrag>,
    cache: HashMap<ObjectId, Option<UvShape>>,
    /// The frame asked for from the menu, for the next frame.
    frame_request: Option<UvFrame>,
    /// The pass it was last drawn in: a panel not drawn lately isn't
    /// under the mouse, wherever it was.
    shown_pass: Option<u64>,
    /// A drag of the gizmo, if one is going on.
    gizmo: Option<gizmo::GizmoDrag>,
    /// Whether the left button went down on the gizmo (so letting go
    /// without moving selects nothing).
    pressed_on_gizmo: bool,
    /// Counts gizmo drags, for their undo merge keys.
    sessions: u64,
    /// What was under the mouse here last frame, for the viewport to
    /// light up (decision 0103).
    under_mouse: Option<(ObjectId, Element)>,
    /// The numbered checker image shown behind the UVs, and the colours
    /// it was made in (decision 0107).
    grid: Option<([[u8; 3]; 2], GridTexture)>,
    /// The layout snapshot drawn faintly under the UVs, by name
    /// (decision 0114).
    compare: Option<String>,
    /// What the Snapshots menu last did, to say in it.
    note: Option<String>,
}

/// The numbered checker's texture in egui.
#[derive(Clone)]
struct GridTexture(egui::TextureHandle);

impl std::fmt::Debug for GridTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GridTexture").field(&self.0.id()).finish()
    }
}

impl Default for UvEditor {
    fn default() -> Self {
        Self {
            centre: Vec2::splat(0.5),
            zoom: 300.0,
            fitted: false,
            rect: None,
            drag: None,
            cache: HashMap::new(),
            frame_request: None,
            shown_pass: None,
            gizmo: None,
            pressed_on_gizmo: false,
            sessions: 0,
            under_mouse: None,
            grid: None,
            compare: None,
            note: None,
        }
    }
}

/// What to fit in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvFrame {
    /// The texture square (H, as Frame all in the viewport).
    Square,
    /// The selection's UVs (Z, as Frame selected).
    Selected,
}

impl UvFrame {
    /// The menu entry's name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Square => "Frame the texture square",
            Self::Selected => "Frame selected",
        }
    }

    /// Its key, the same as the viewport's framing key.
    pub(crate) fn key(self) -> Key {
        match self {
            Self::Square => crate::Frame::All.key(),
            Self::Selected => crate::Frame::Selected.key(),
        }
    }
}

/// Everything the editor needs from the rest of the window this frame.
pub(crate) struct UvContext<'a> {
    pub(crate) mode: Mode,
    pub(crate) navigation: &'a NavigationSettings,
    pub(crate) select_keys: SelectKeys,
    /// The tool in use: the move, rotate and scale tools show the gizmo.
    pub(crate) tool: Tool,
    /// How snapping works.
    pub(crate) snap: SnapSettings,
    /// Whether polygons are coloured by how evenly they lie (the
    /// viewport's Stretch view, decision 0102).
    pub(crate) stretch: bool,
    /// What is under the mouse in the viewport, lit up here too
    /// (decision 0103).
    pub(crate) outside_hover: Option<(ObjectId, Element)>,
    /// The numbered checker's colours while the viewport shows it
    /// (decision 0107): the editor shows it behind the UVs.
    pub(crate) grid: Option<[[u8; 3]; 2]>,
}

/// The objects whose UVs are shown: the selected ones with a shape, in
/// selection order (an instance shows its original's).
pub(crate) fn shown_objects(doc: &Document) -> Vec<ObjectId> {
    doc.selection()
        .into_iter()
        .filter(|id| {
            doc.scene()
                .object(*id)
                .is_some_and(|o| o.kind.has_shape() && !matches!(o.kind, ObjectKind::Group))
        })
        .collect()
}

impl UvEditor {
    /// Whether the mouse is over the editor (as drawn last frame).
    pub(crate) fn hovered(&self, ctx: &egui::Context) -> bool {
        let pass = ctx.cumulative_pass_nr();
        if self.shown_pass.is_none_or(|p| p + 1 < pass) {
            return false;
        }
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        self.rect
            .zip(pointer)
            .is_some_and(|(rect, at)| rect.contains(at))
    }

    /// Where `uv` is on screen, as drawn last frame (for tests).
    #[cfg(test)]
    pub(crate) fn screen_of(&self, uv: Vec2) -> Option<Pos2> {
        Some(self.to_screen(self.rect?, uv))
    }

    /// What is under the mouse in the editor, if the mouse is over it.
    pub(crate) fn under_mouse(&self, ctx: &egui::Context) -> Option<(ObjectId, Element)> {
        self.hovered(ctx).then_some(self.under_mouse).flatten()
    }

    /// Whether the gizmo is being dragged.
    pub(crate) fn dragging_gizmo(&self) -> bool {
        self.gizmo.is_some()
    }

    /// Cancels a drag of the gizmo, putting everything back (Esc).
    pub(crate) fn cancel_gizmo(&mut self, doc: &mut Document) {
        if let Some(mut drag) = self.gizmo.take() {
            drag.cancel(doc);
        }
        if self.drag == Some(EditorDrag::Gizmo) {
            self.drag = None;
        }
    }

    /// Where the gizmo's middle is drawn last frame, if it is (for tests).
    #[cfg(test)]
    pub(crate) fn gizmo_centre(&self, doc: &Document, cx_mode: Mode) -> Option<Pos2> {
        let (_, bounds) = gizmo::uv_targets(doc, cx_mode);
        let (min, max) = bounds?;
        Some(self.to_screen(self.rect?, (min + max) / 2.0))
    }

    /// Asks for a frame on the next frame (from a menu).
    pub(crate) fn frame(&mut self, what: UvFrame) {
        self.frame_request = Some(what);
    }

    /// Takes H and Z while the mouse is over the editor (before the
    /// viewport, which would take them otherwise), so they frame here.
    pub(crate) fn take_keys(&mut self, ctx: &egui::Context) {
        if !self.hovered(ctx) || ctx.egui_wants_keyboard_input() {
            return;
        }
        for what in [UvFrame::Square, UvFrame::Selected] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, what.key())) {
                self.frame_request = Some(what);
            }
        }
    }

    fn to_screen(&self, rect: Rect, uv: Vec2) -> Pos2 {
        let c = rect.center();
        pos2(
            c.x + (uv.x - self.centre.x) * self.zoom,
            c.y - (uv.y - self.centre.y) * self.zoom,
        )
    }

    fn to_uv(&self, rect: Rect, at: Pos2) -> Vec2 {
        let c = rect.center();
        Vec2::new(
            self.centre.x + (at.x - c.x) / self.zoom,
            self.centre.y - (at.y - c.y) / self.zoom,
        )
    }

    /// Fits `min`..`max` (UVs) in the panel, with a margin.
    fn fit(&mut self, rect: Rect, min: Vec2, max: Vec2) {
        let size = (max - min).max(Vec2::splat(1e-3));
        let room = (rect.width().min(rect.height()) * 0.85).max(10.0);
        self.zoom = (room / size.x.max(size.y)).clamp(MIN_ZOOM, MAX_ZOOM);
        self.centre = (min + max) / 2.0;
    }

    /// The shown objects' UVs, worked out again where their shapes changed.
    fn shapes(&mut self, doc: &Document) -> Vec<(ObjectId, UvShape)> {
        let ids = shown_objects(doc);
        self.cache.retain(|id, _| ids.contains(id));
        let mut out = Vec::new();
        for id in ids {
            let Some(evaluated) = doc.scene().evaluated(id) else {
                continue;
            };
            let fresh = match self.cache.get(&id) {
                Some(Some(shape)) => !Arc::ptr_eq(&shape.mesh, &evaluated.mesh),
                Some(None) => true,
                None => true,
            };
            if fresh {
                let shape = if evaluated.mesh.face_count() > UV_POLYGONS_UP_TO {
                    None
                } else {
                    UvShape::of(Arc::clone(&evaluated.mesh))
                };
                self.cache.insert(id, shape);
            }
            if let Some(Some(shape)) = self.cache.get(&id) {
                out.push((id, shape.clone()));
            }
        }
        out
    }

    /// Draws the editor and handles the mouse. Selection changes go
    /// straight to the document, as one undo step each, like the
    /// viewport's.
    pub(crate) fn show(&mut self, ui: &mut Ui, doc: &mut Document, cx: &UvContext<'_>) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, UV_EDITOR_LABEL));
        self.rect = Some(rect);
        self.shown_pass = Some(ui.ctx().cumulative_pass_nr());
        if !self.fitted && rect.width() > 10.0 {
            self.fit(rect, Vec2::ZERO, Vec2::ONE);
            self.fitted = true;
        }
        let shapes = self.shapes(doc);
        self.frame_from_request(rect, &shapes, doc);
        let lit = self.use_gizmo(ui, &response, rect, doc, cx);
        self.navigate(ui, &response, rect, cx.navigation);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::UV_BACKGROUND);
        let grid = cx.grid.map(|colors| self.grid_texture(ui.ctx(), colors));
        self.paint_square(&painter, rect, doc.scene().texture().size, grid);
        self.paint_trim(&painter, rect, doc.scene().trim());
        self.paint_compared(&painter, rect, doc);
        let accent = theme::accent(ui.ctx()).colors();
        let kind = element_kind(cx.mode);
        let own = response
            .hover_pos()
            .filter(|_| self.drag.is_none() && kind.is_some())
            .and_then(|at| self.pick(rect, &shapes, at, cx.mode));
        self.under_mouse = own.map(|h| (h.id, h.element));
        // What the viewport has under the mouse lights up here too.
        let hover = own.or_else(|| {
            let (id, element) = cx.outside_hover?;
            (response.hover_pos().is_none() && Some(element.kind()) == kind)
                .then_some(Hit { id, element })
        });
        let too_many = shown_objects(doc).len() > shapes.len();
        let mut polygons = 0;
        let mut islands = 0;
        for (id, shape) in &shapes {
            polygons += shape.polygons.len();
            islands += shape.islands.len();
            let selected = doc.elements(*id).cloned().unwrap_or_default();
            self.paint_shape(
                &painter,
                rect,
                shape,
                &selected,
                kind,
                hover.filter(|h| h.id == *id).map(|h| h.element),
                accent.fill,
                accent.light,
                cx.mode,
                cx.stretch,
            );
        }
        if let Some((centre, lit)) = lit {
            gizmo::paint(&painter, self.to_screen(rect, centre), cx.tool, lit);
        }
        self.select(ui, &response, rect, &shapes, doc, cx);
        if let Some(EditorDrag::Box { from, .. }) = self.drag
            && let Some(now) = ui.input(|i| i.pointer.latest_pos())
        {
            let area = Rect::from_two_pos(from, now);
            painter.rect_filled(area, 0.0, accent.fill.gamma_multiply(0.12));
            painter.rect_stroke(
                area,
                0.0,
                Stroke::new(1.0, accent.light),
                egui::StrokeKind::Inside,
            );
        }
        // What is shown, bottom left; a hint when nothing is.
        // With the stretch view, what is wrong with the polygon under the
        // mouse (or how many are off), above the other notes, wrapped.
        if let Some(text) = cx.stretch.then(|| stretch_note(&shapes, hover)).flatten() {
            let galley = painter.layout(
                text,
                FontId::proportional(12.0),
                theme::TEXT,
                (rect.width() - 20.0).max(50.0),
            );
            let at = rect.left_bottom() + vec2(10.0, -28.0 - galley.size().y);
            let behind = Rect::from_min_size(at, galley.size()).expand(4.0);
            painter.rect_filled(behind, 3.0, theme::UV_BACKGROUND.gamma_multiply(0.92));
            painter.galley(at, galley, theme::TEXT);
        }
        let note = if shapes.is_empty() && !too_many {
            if shown_objects(doc).is_empty() {
                "Select an object to see its UVs.".to_string()
            } else {
                "The selected objects have no UVs.".to_string()
            }
        } else if too_many {
            format!(
                "Some selected objects have more than {} polygons: their UVs aren't drawn.",
                UV_POLYGONS_UP_TO
            )
        } else {
            let hint = match cx.mode {
                Mode::Model | Mode::Object => " · press 1, 2, 3 or pick UV mode to select",
                _ => "",
            };
            format!(
                "{islands} {} · {polygons} {}{hint}",
                if islands == 1 { "island" } else { "islands" },
                if polygons == 1 { "polygon" } else { "polygons" },
            )
        };
        painter.text(
            rect.left_bottom() + vec2(10.0, -10.0),
            Align2::LEFT_BOTTOM,
            note,
            FontId::proportional(12.0),
            theme::TEXT_WEAK,
        );
        // The texture, and the texel density shown (of the selected
        // polygons, or of everything shown), bottom right.
        let texture = doc.scene().texture();
        let mut flat = 0.0;
        let mut real = 0.0;
        let mut chosen = 0;
        for (id, shape) in &shapes {
            let selected = doc.elements(*id).cloned().unwrap_or_default();
            for ((face, _), (f, r)) in shape.polygons.iter().zip(&shape.areas) {
                if kind == Some(ElementKind::Polygons) && selected.contains(Element::Polygon(*face))
                {
                    if chosen == 0 {
                        flat = 0.0;
                        real = 0.0;
                    }
                    chosen += 1;
                    flat += f;
                    real += r;
                } else if chosen == 0 {
                    flat += f;
                    real += r;
                }
            }
        }
        let density =
            (flat > 1e-20 && real > 1e-20).then(|| texture.pixels_per_metre((flat / real).sqrt()));
        let text = density_text(texture, density, chosen > 0);
        painter.text(
            rect.right_bottom() + vec2(-10.0, -10.0),
            Align2::RIGHT_BOTTOM,
            text,
            FontId::proportional(12.0),
            theme::TEXT_WEAK,
        );
    }

    fn frame_from_request(&mut self, rect: Rect, shapes: &[(ObjectId, UvShape)], doc: &Document) {
        let Some(what) = self.frame_request.take() else {
            return;
        };
        match what {
            UvFrame::Square => self.fit(rect, Vec2::ZERO, Vec2::ONE),
            UvFrame::Selected => {
                let mut min = Vec2::splat(f32::INFINITY);
                let mut max = Vec2::splat(f32::NEG_INFINITY);
                let mut any_selected = false;
                for (id, shape) in shapes {
                    let selected = doc.elements(*id).cloned().unwrap_or_default();
                    for (face, points) in &shape.polygons {
                        let chosen = selected.contains(Element::Polygon(*face))
                            || points
                                .iter()
                                .any(|(v, _)| selected.contains(Element::Point(*v)));
                        if chosen {
                            any_selected = true;
                            for (_, uv) in points {
                                min = min.min(*uv);
                                max = max.max(*uv);
                            }
                        }
                    }
                }
                if !any_selected {
                    for (_, shape) in shapes {
                        for (_, points) in &shape.polygons {
                            for (_, uv) in points {
                                min = min.min(*uv);
                                max = max.max(*uv);
                            }
                        }
                    }
                }
                if min.is_finite() {
                    self.fit(rect, min, max);
                } else {
                    self.fit(rect, Vec2::ZERO, Vec2::ONE);
                }
            }
        }
    }

    /// The gizmo: starts, follows, ends and cancels its drags. Returns
    /// where to draw it (in UVs) and which handle to light, when it shows.
    fn use_gizmo(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        doc: &mut Document,
        cx: &UvContext<'_>,
    ) -> Option<(Vec2, Option<gizmo::UvHandle>)> {
        if ui.input(|i| i.pointer.primary_pressed()) {
            self.pressed_on_gizmo = false;
        }
        if !gizmo::has_gizmo(cx.tool) || element_kind(cx.mode).is_none() {
            self.cancel_gizmo(doc);
            return None;
        }
        let snap_now = |ui: &Ui| {
            cx.snap
                .active(ui.input(|i| i.modifiers.command))
                .then_some(&cx.snap)
        };
        if let Some(mut drag) = self.gizmo.take() {
            let (right, left_down, at, fine) = ui.input(|i| {
                (
                    i.pointer.secondary_pressed(),
                    i.pointer.primary_down(),
                    i.pointer.latest_pos(),
                    i.modifiers.shift,
                )
            });
            if right {
                drag.cancel(doc);
                self.drag = None;
                return None;
            }
            let snap = snap_now(ui);
            if let Some(at) = at {
                let centre = self.to_screen(rect, drag.pivot);
                drag.follow(at, centre, self.zoom, fine);
                if !drag.show(doc, snap) {
                    // Something else changed the scene meanwhile: keep it
                    // all, and stop.
                    self.drag = None;
                    return None;
                }
            }
            let shown = (drag.pivot_now(snap), Some(drag.handle));
            if left_down {
                self.gizmo = Some(drag);
            } else {
                self.drag = None;
            }
            return Some(shown);
        }
        let (targets, bounds) = gizmo::uv_targets(doc, cx.mode);
        let (min, max) = bounds?;
        let centre = (min + max) / 2.0;
        let on_screen = self.to_screen(rect, centre);
        if ui.input(|i| i.pointer.primary_pressed()) {
            self.pressed_on_gizmo = ui.input(|i| i.pointer.press_origin()).is_some_and(|at| {
                rect.contains(at) && gizmo::hit(on_screen, cx.tool, at).is_some()
            });
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && self.drag.is_none()
            && let Some(from) = ui.input(|i| i.pointer.press_origin())
            && let Some(handle) = gizmo::hit(on_screen, cx.tool, from)
        {
            self.sessions += 1;
            let key = gizmo::UV_GIZMO_KEY | self.sessions;
            let mut drag = gizmo::GizmoDrag::new(handle, centre, targets, from, key);
            if let Some(at) = ui.input(|i| i.pointer.latest_pos()) {
                drag.follow(at, on_screen, self.zoom, ui.input(|i| i.modifiers.shift));
                drag.show(doc, snap_now(ui));
            }
            self.drag = Some(EditorDrag::Gizmo);
            let shown = (drag.pivot_now(snap_now(ui)), Some(handle));
            self.gizmo = Some(drag);
            return Some(shown);
        }
        let hover = response
            .hover_pos()
            .filter(|_| self.drag.is_none())
            .and_then(|at| gizmo::hit(on_screen, cx.tool, at));
        Some((centre, hover))
    }

    /// Panning and zooming, with the viewport's controls (orbiting pans).
    fn navigate(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        settings: &NavigationSettings,
    ) {
        if response.drag_started() && self.drag.is_none() {
            let keys = ui.input(|i| Modifiers {
                alt: i.modifiers.alt,
                shift: i.modifiers.shift,
            });
            let button = [
                (egui::PointerButton::Primary, MouseButton::Primary),
                (egui::PointerButton::Middle, MouseButton::Middle),
                (egui::PointerButton::Secondary, MouseButton::Secondary),
            ]
            .into_iter()
            .find(|(b, _)| response.drag_started_by(*b))
            .map(|(_, b)| b);
            if let Some(kind) = button.and_then(|b| settings.drag(b, keys)) {
                self.drag = Some(EditorDrag::Navigate(kind));
            }
        }
        if let Some(EditorDrag::Navigate(kind)) = self.drag {
            let delta = response.drag_delta();
            match kind {
                DragKind::Pan | DragKind::Orbit => {
                    self.centre -= Vec2::new(delta.x, -delta.y) / self.zoom;
                }
                DragKind::Dolly => {
                    let factor = (-delta.y * 0.01).exp();
                    self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                }
            }
            if response.drag_stopped() || !ui.input(|i| i.pointer.any_down()) {
                self.drag = None;
            }
        }
        // The wheel zooms at the mouse (Ctrl + wheel is left alone, as in
        // the viewport, where it grows and shrinks the selection).
        if let Some(at) = response.hover_pos() {
            let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
            if scroll != 0.0 && !ctrl {
                let before = self.to_uv(rect, at);
                self.zoom = (self.zoom * (scroll * 0.0025).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
                let after = self.to_uv(rect, at);
                self.centre += before - after;
            }
        }
    }

    /// The texture's pixels, when zoomed in far enough to see them
    /// (decision 0100): faint lines between them, over what is in view.
    fn paint_pixels(&self, painter: &egui::Painter, rect: Rect, square: Rect, size: u32) {
        let step = self.zoom / size as f32;
        if step < PIXELS_FROM {
            return;
        }
        let shown = square.intersect(rect);
        if !shown.is_positive() {
            return;
        }
        let low = self.to_uv(rect, shown.left_bottom());
        let high = self.to_uv(rect, shown.right_top());
        let n = size as f32;
        let stroke = Stroke::new(1.0, theme::UV_GRID.gamma_multiply(0.6));
        let first = |v: f32| (v * n).ceil() as i64;
        let last = |v: f32| (v * n).floor() as i64;
        for i in first(low.x)..=last(high.x) {
            let x = self.to_screen(rect, Vec2::new(i as f32 / n, 0.0)).x;
            painter.vline(x, shown.y_range(), stroke);
        }
        for j in first(low.y)..=last(high.y) {
            let y = self.to_screen(rect, Vec2::new(0.0, j as f32 / n)).y;
            painter.hline(shown.x_range(), y, stroke);
        }
    }

    /// The trim sheet's rows (decision 0106): faint bands across the
    /// square, every other one stronger, with their edges and names, and
    /// a fainter continuation to the right, where the rows repeat.
    fn paint_trim(&self, painter: &egui::Painter, rect: Rect, trim: &scene::TrimSheet) {
        for (i, row) in trim.rows.iter().enumerate() {
            let a = self.to_screen(rect, Vec2::new(0.0, row.bottom));
            let b = self.to_screen(rect, Vec2::new(1.0, row.top));
            let band = Rect::from_two_pos(a, b);
            let fill = if i % 2 == 0 {
                theme::TRIM_ROW
            } else {
                theme::TRIM_ROW.gamma_multiply(1.8)
            };
            painter.rect_filled(band, 0.0, fill);
            let beyond = Rect::from_min_max(
                pos2(band.right(), band.top()),
                pos2(band.right() + band.width() * 0.5, band.bottom()),
            );
            painter.rect_filled(beyond, 0.0, fill.gamma_multiply(0.4));
            let edge = Stroke::new(1.0, theme::TRIM_ROW_EDGE.gamma_multiply(0.7));
            painter.hline(band.x_range(), band.top(), edge);
            painter.hline(band.x_range(), band.bottom(), edge);
            if band.height() >= 14.0 {
                painter.text(
                    band.left_top() + vec2(6.0, 3.0),
                    Align2::LEFT_TOP,
                    &row.name,
                    FontId::proportional(11.0),
                    theme::TRIM_ROW_EDGE,
                );
            }
        }
    }

    /// The colours of the numbered checker image shown, if one has been.
    #[cfg(test)]
    pub(crate) fn grid_colors(&self) -> Option<[[u8; 3]; 2]> {
        self.grid.as_ref().map(|(c, _)| *c)
    }

    /// The numbered checker image in `colors`, made again when they
    /// change.
    fn grid_texture(&mut self, ctx: &egui::Context, colors: [[u8; 3]; 2]) -> egui::TextureId {
        if let Some((made, handle)) = &self.grid
            && *made == colors
        {
            return handle.0.id();
        }
        let size = crate::viewport::GRID_IMAGE_SIZE;
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [size, size],
            &crate::viewport::grid_image(colors),
        );
        let handle = ctx.load_texture("numbered checker", image, egui::TextureOptions::LINEAR);
        let id = handle.id();
        self.grid = Some((colors, GridTexture(handle)));
        id
    }

    /// The texture square: a lighter tile (or the numbered checker,
    /// dimmed so the UVs stand out over it), a grid every tenth and its
    /// edges.
    fn paint_square(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        texture_size: u32,
        grid: Option<egui::TextureId>,
    ) {
        let a = self.to_screen(rect, Vec2::ZERO);
        let b = self.to_screen(rect, Vec2::ONE);
        let square = Rect::from_two_pos(a, b);
        painter.rect_filled(square, 0.0, theme::UV_SQUARE);
        if let Some(image) = grid {
            // The image's first row is the top of the square.
            painter.image(
                image,
                square,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                theme::UV_GRID_IMAGE_TINT,
            );
        }
        for i in (1..10).filter(|_| grid.is_none()) {
            let t = i as f32 / 10.0;
            let color = if i == 5 {
                theme::UV_GRID.gamma_multiply(1.8)
            } else {
                theme::UV_GRID
            };
            let x = self.to_screen(rect, Vec2::new(t, 0.0)).x;
            let y = self.to_screen(rect, Vec2::new(0.0, t)).y;
            painter.vline(x, square.y_range(), Stroke::new(1.0, color));
            painter.hline(square.x_range(), y, Stroke::new(1.0, color));
        }
        self.paint_pixels(painter, rect, square, texture_size);
        painter.rect_stroke(
            square,
            0.0,
            Stroke::new(1.0, theme::UV_SQUARE_EDGE),
            egui::StrokeKind::Middle,
        );
        let label = |at: Pos2, text: &str, align| {
            painter.text(
                at,
                align,
                text,
                FontId::proportional(11.0),
                theme::TEXT_WEAK,
            );
        };
        label(
            square.left_bottom() + vec2(-4.0, 4.0),
            "0",
            Align2::RIGHT_TOP,
        );
        label(
            square.right_bottom() + vec2(0.0, 4.0),
            "1",
            Align2::CENTER_TOP,
        );
        label(
            square.left_top() + vec2(-4.0, 0.0),
            "1",
            Align2::RIGHT_CENTER,
        );
    }

    /// One object's UVs: selected and hovered polygons tinted, every edge,
    /// island borders brighter, and selected edges and points on top.
    #[allow(clippy::too_many_arguments)]
    fn paint_shape(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        shape: &UvShape,
        selected: &ElementSelection,
        kind: Option<ElementKind>,
        hover: Option<Element>,
        fill: Color32,
        light: Color32,
        mode: Mode,
        stretch: bool,
    ) {
        let screen = |uv: Vec2| self.to_screen(rect, uv);
        // Tinted polygons.
        let hovered_faces: BTreeSet<FaceId> = match (hover, mode) {
            (Some(Element::Polygon(f)), Mode::Uv) => shape
                .island_of
                .get(f.get() as usize)
                .and_then(|i| shape.islands.get(*i))
                .map(|faces| faces.iter().copied().collect())
                .unwrap_or_default(),
            (Some(Element::Polygon(f)), _) => [f].into_iter().collect(),
            _ => BTreeSet::new(),
        };
        let mut tint = egui::Mesh::default();
        // Locked pieces: faint stripes across them.
        let mut stripes = Vec::new();
        for ((_, points), locked) in shape.polygons.iter().zip(&shape.locked) {
            if *locked {
                let corners: Vec<Pos2> = points.iter().map(|(_, uv)| screen(*uv)).collect();
                add_polygon(&mut tint, &corners, theme::UV_LOCKED);
                stripes.push(corners);
            }
        }
        if stretch {
            for ((_, points), s) in shape.polygons.iter().zip(&shape.stretch) {
                let corners: Vec<Pos2> = points.iter().map(|(_, uv)| screen(*uv)).collect();
                add_polygon(&mut tint, &corners, stretch_color(s).gamma_multiply(0.55));
            }
        }
        if matches!(kind, Some(ElementKind::Polygons)) {
            for (face, points) in &shape.polygons {
                let color = if selected.contains(Element::Polygon(*face)) {
                    fill.gamma_multiply(0.55)
                } else if hovered_faces.contains(face) {
                    light.gamma_multiply(0.25)
                } else {
                    continue;
                };
                let corners: Vec<Pos2> = points.iter().map(|(_, uv)| screen(*uv)).collect();
                add_polygon(&mut tint, &corners, color);
            }
        }
        painter.add(egui::Shape::mesh(tint));
        for corners in &stripes {
            let (min, max) = corners.iter().fold(
                (
                    Pos2::new(f32::INFINITY, f32::INFINITY),
                    Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                ),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            let clip = painter.with_clip_rect(Rect::from_min_max(min, max).intersect(rect));
            let mut x = min.x - (max.y - min.y);
            while x < max.x {
                clip.line_segment(
                    [Pos2::new(x, max.y), Pos2::new(x + (max.y - min.y), min.y)],
                    egui::Stroke::new(1.0, theme::UV_LOCKED.gamma_multiply(2.0)),
                );
                x += 8.0;
            }
        }
        // Every edge.
        let mut lines = egui::Mesh::default();
        let mut borders = egui::Mesh::default();
        let mut seams = egui::Mesh::default();
        for (uvs, key, border) in &shape.edges {
            let (a, b) = (screen(uvs[0]), screen(uvs[1]));
            if !rect.expand(2.0).intersects(Rect::from_two_pos(a, b)) {
                continue;
            }
            if shape.seams.contains(key) {
                add_line(&mut seams, a, b, 2.0, theme::UV_SEAM);
            } else if let Some(preference) = shape.preferences.get(key) {
                match preference {
                    scene::SeamPreference::Prefer => {
                        add_dashes(&mut seams, a, b, 2.0, theme::UV_SEAM_PREFER);
                    }
                    scene::SeamPreference::Never => {
                        add_line(&mut seams, a, b, 2.0, theme::UV_SEAM_NEVER);
                    }
                    scene::SeamPreference::Free => {
                        add_line(&mut lines, a, b, 1.0, theme::UV_EDGE);
                    }
                }
            } else if *border {
                add_line(&mut borders, a, b, 1.6, theme::UV_BORDER);
            } else {
                add_line(&mut lines, a, b, 1.0, theme::UV_EDGE);
            }
        }
        painter.add(egui::Shape::mesh(lines));
        painter.add(egui::Shape::mesh(borders));
        painter.add(egui::Shape::mesh(seams));
        // Selected and hovered edges and points.
        let mut marks = egui::Mesh::default();
        match kind {
            Some(ElementKind::Edges) => {
                for (uvs, key, _) in &shape.edges {
                    let color = if selected.contains(Element::Edge(*key)) {
                        fill
                    } else if hover == Some(Element::Edge(*key)) {
                        light
                    } else {
                        continue;
                    };
                    add_line(&mut marks, screen(uvs[0]), screen(uvs[1]), 3.0, color);
                }
            }
            Some(ElementKind::Points) => {
                for (_, points) in &shape.polygons {
                    for (v, uv) in points {
                        let (radius, color) = if selected.contains(Element::Point(*v)) {
                            (3.5, fill)
                        } else if hover == Some(Element::Point(*v)) {
                            (4.5, light)
                        } else {
                            (2.0, theme::UV_EDGE)
                        };
                        add_dot(&mut marks, screen(*uv), radius, color);
                    }
                }
            }
            _ => {}
        }
        // Pinned UVs, in every mode: a dark ring with a bright middle.
        for pin in &shape.pins {
            let at = screen(*pin);
            if rect.expand(6.0).contains(at) {
                add_dot(&mut marks, at, 5.0, theme::PANEL);
                add_dot(&mut marks, at, 3.5, theme::UV_PIN);
            }
        }
        painter.add(egui::Shape::mesh(marks));
    }

    /// What is under `at`, by mode: a point or an edge close by, or the
    /// polygon it is inside (the smallest, where they overlap).
    fn pick(
        &self,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        at: Pos2,
        mode: Mode,
    ) -> Option<Hit> {
        let kind = element_kind(mode)?;
        let uv = self.to_uv(rect, at);
        match kind {
            ElementKind::Points => {
                let mut best: Option<(f32, Hit)> = None;
                for (id, shape) in shapes {
                    for (_, points) in &shape.polygons {
                        for (v, p) in points {
                            let d = self.to_screen(rect, *p).distance(at);
                            if d <= PICK_DISTANCE && best.is_none_or(|(b, _)| d < b) {
                                best = Some((
                                    d,
                                    Hit {
                                        id: *id,
                                        element: Element::Point(*v),
                                    },
                                ));
                            }
                        }
                    }
                }
                best.map(|(_, h)| h)
            }
            ElementKind::Edges => {
                let mut best: Option<(f32, Hit)> = None;
                for (id, shape) in shapes {
                    for (uvs, key, _) in &shape.edges {
                        let d = segment_distance(
                            at,
                            self.to_screen(rect, uvs[0]),
                            self.to_screen(rect, uvs[1]),
                        );
                        if d <= PICK_DISTANCE && best.is_none_or(|(b, _)| d < b) {
                            best = Some((
                                d,
                                Hit {
                                    id: *id,
                                    element: Element::Edge(*key),
                                },
                            ));
                        }
                    }
                }
                best.map(|(_, h)| h)
            }
            ElementKind::Polygons => {
                let mut best: Option<(f32, Hit)> = None;
                for (id, shape) in shapes {
                    for (face, points) in &shape.polygons {
                        let corners: Vec<Vec2> = points.iter().map(|(_, p)| *p).collect();
                        if inside(&corners, uv) {
                            let area = polygon_area(&corners).abs();
                            if best.is_none_or(|(b, _)| area < b) {
                                best = Some((
                                    area,
                                    Hit {
                                        id: *id,
                                        element: Element::Polygon(*face),
                                    },
                                ));
                            }
                        }
                    }
                }
                best.map(|(_, h)| h)
            }
        }
    }

    /// Clicks and box drags select, as in the viewport.
    fn select(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        doc: &mut Document,
        cx: &UvContext<'_>,
    ) {
        let Some(kind) = element_kind(cx.mode) else {
            return;
        };
        let keys = ui.input(|i| i.modifiers);
        if response.drag_started_by(egui::PointerButton::Primary)
            && self.drag.is_none()
            && let Some(from) = ui.input(|i| i.pointer.press_origin())
        {
            self.drag = Some(EditorDrag::Box {
                from,
                how: cx.select_keys.combine(keys, false),
            });
        }
        if let Some(EditorDrag::Box { from, how }) = self.drag
            && (response.drag_stopped() || !ui.input(|i| i.pointer.primary_down()))
        {
            self.drag = None;
            if let Some(to) = ui.input(|i| i.pointer.latest_pos()) {
                let area = Rect::from_two_pos(from, to);
                let mut hits = Vec::new();
                for (id, shape) in shapes {
                    for element in self.in_box(rect, shape, area, kind) {
                        hits.push(Hit { id: *id, element });
                    }
                }
                apply(doc, shapes, &hits, kind, how, cx.mode);
            }
            return;
        }
        if response.clicked_by(egui::PointerButton::Primary)
            && !self.pressed_on_gizmo
            && let Some(at) = response.interact_pointer_pos()
        {
            let how = cx.select_keys.combine(keys, true);
            let hits: Vec<Hit> = self.pick(rect, shapes, at, cx.mode).into_iter().collect();
            apply(doc, shapes, &hits, kind, how, cx.mode);
        }
    }

    /// The elements of `kind` inside `area` (screen): points, edges with
    /// both ends inside, polygons with their middle inside.
    fn in_box(&self, rect: Rect, shape: &UvShape, area: Rect, kind: ElementKind) -> Vec<Element> {
        let screen = |uv: Vec2| self.to_screen(rect, uv);
        let mut out = BTreeSet::new();
        match kind {
            ElementKind::Points => {
                for (_, points) in &shape.polygons {
                    for (v, uv) in points {
                        if area.contains(screen(*uv)) {
                            out.insert(Element::Point(*v));
                        }
                    }
                }
            }
            ElementKind::Edges => {
                for (uvs, key, _) in &shape.edges {
                    if area.contains(screen(uvs[0])) && area.contains(screen(uvs[1])) {
                        out.insert(Element::Edge(*key));
                    }
                }
            }
            ElementKind::Polygons => {
                for (face, points) in &shape.polygons {
                    let middle = points.iter().map(|(_, p)| *p).sum::<Vec2>() / points.len() as f32;
                    if area.contains(screen(middle)) {
                        out.insert(Element::Polygon(*face));
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

/// Marks texture setting changes apart from every other undo step, so a
/// drag of the density field is one step.
const TEXTURE_KEY: u64 = 0x5458_5452_0000_0001;

/// The menu entries for the project's texture settings (decision 0100):
/// its size, and the texel density aimed for.
pub(crate) fn texture_menu(ui: &mut Ui, doc: &mut Document) {
    let mut texture = doc.scene().texture();
    let mut changed = false;
    ui.label(egui::RichText::new("Texture size").strong());
    for size in scene::TEXTURE_SIZES {
        if ui
            .add(egui::Button::selectable(
                texture.size == size,
                format!("{size} × {size} px"),
            ))
            .on_hover_text("The size of the texture this project's UVs are made for")
            .clicked()
        {
            texture.size = size;
            changed = true;
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(TEXEL_DENSITY_LABEL);
        changed |= ui
            .add(
                egui::DragValue::new(&mut texture.density)
                    .range(1.0..=scene::MAX_DENSITY)
                    .speed(1.0)
                    .suffix(" px/m"),
            )
            .on_hover_text(
                "The texel density aimed for: texture pixels per metre of surface. 512 px/m \
                 is common for props seen up close in third-person games.",
            )
            .changed();
    });
    if changed && texture.is_valid() {
        let _ = doc.execute_labelled(
            scene::Command::SetTexture(texture),
            "Change the texture settings",
            Some(TEXTURE_KEY),
        );
    }
}

/// Marks trim sheet changes' undo merge keys, so a drag or a run of
/// typing in the Trim menu is one undo step.
const TRIM_KEY: u64 = 0x5452_494d_0000_0001;

/// The UV Editor's Trim menu (decision 0106): the trim sheet's rows, top
/// to bottom, each with its name, where it starts from the top of the
/// texture and its height (in the texture's pixels, as an image editor
/// shows them), to change or remove; quick even layouts; adding a row.
pub(crate) fn trim_menu(ui: &mut Ui, doc: &mut Document) {
    let size = doc.scene().texture().size as f32;
    let mut sheet = doc.scene().trim().clone();
    let mut changed = false;
    ui.set_max_width(340.0);
    ui.label(egui::RichText::new("Trim sheet").strong());
    ui.label(egui::RichText::new("Rows of a shared texture, repeating left to right.").weak());
    ui.label(egui::RichText::new("Lay strips on them: UV → Put on trim row.").weak());
    ui.separator();
    if sheet.rows.is_empty() {
        ui.label("No rows yet.");
    }
    // Top to bottom, as the texture looks in an image editor.
    let mut order: Vec<usize> = (0..sheet.rows.len()).collect();
    order.sort_by(|a, b| sheet.rows[*b].top.total_cmp(&sheet.rows[*a].top));
    let mut remove = None;
    egui::Grid::new("trim_rows")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            if !sheet.rows.is_empty() {
                ui.label(egui::RichText::new("Name").weak());
                ui.label(egui::RichText::new("From top").weak());
                ui.label(egui::RichText::new("Height").weak());
                ui.label("");
                ui.end_row();
            }
            for i in order {
                let row = &mut sheet.rows[i];
                let mut name = row.name.clone();
                if ui
                    .add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut name))
                    .on_hover_text("What the row holds: bolts, a panel edge, a seal")
                    .changed()
                    && !name.trim().is_empty()
                {
                    name.truncate(scene::MAX_ROW_NAME);
                    row.name = name;
                    changed = true;
                }
                let mut from_top = ((1.0 - row.top) * size).round();
                let mut height = (row.height() * size).round();
                if ui
                    .add(
                        egui::DragValue::new(&mut from_top)
                            .range(0.0..=size - 1.0)
                            .speed(1.0)
                            .suffix(" px"),
                    )
                    .on_hover_text("Where the row starts, in pixels from the top of the texture")
                    .changed()
                {
                    let h = row.height();
                    row.top = 1.0 - from_top / size;
                    row.bottom = row.top - h;
                    changed = true;
                }
                if ui
                    .add(
                        egui::DragValue::new(&mut height)
                            .range(1.0..=size)
                            .speed(1.0)
                            .suffix(" px"),
                    )
                    .on_hover_text("How tall the row is, in pixels")
                    .changed()
                {
                    row.bottom = row.top - height / size;
                    changed = true;
                }
                if ui
                    .small_button("×")
                    .on_hover_text("Remove this row")
                    .clicked()
                {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        sheet.rows.remove(i);
        changed = true;
    }
    ui.separator();
    // A new row below the lowest, as tall as it, if there is room.
    let lowest = sheet
        .rows
        .iter()
        .map(|r| (r.bottom, r.height()))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let (below, tall) = lowest.unwrap_or((1.0, 0.125));
    let room = below >= tall - 1e-6 && sheet.rows.len() < scene::MAX_TRIM_ROWS;
    if ui
        .add_enabled(room, egui::Button::new("Add a row"))
        .on_hover_text("A new row under the lowest one, as tall as it")
        .on_disabled_hover_text("There's no room under the lowest row: make it shorter first")
        .clicked()
    {
        sheet.rows.push(scene::TrimRow {
            name: format!("Row {}", sheet.rows.len() + 1),
            top: below,
            bottom: (below - tall).max(0.0),
        });
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label("Even rows:");
        for n in [2, 4, 8, 16] {
            if ui
                .small_button(n.to_string())
                .on_hover_text(format!("{n} rows of the same height, filling the texture"))
                .clicked()
            {
                sheet = scene::TrimSheet::even(n);
                changed = true;
            }
        }
    });
    if !sheet.rows.is_empty()
        && ui
            .button("Remove every row")
            .on_hover_text("No trim sheet. Pieces already on its rows stay where they are.")
            .clicked()
    {
        sheet.rows.clear();
        changed = true;
    }
    // A change that would make rows overlap isn't made: the numbers spring
    // back.
    if changed && sheet.is_valid() {
        let _ = doc.execute_labelled(
            scene::Command::SetTrim(sheet),
            "Change the trim sheet",
            Some(TRIM_KEY),
        );
    }
}

/// The label of the texel density field.
pub(crate) const TEXEL_DENSITY_LABEL: &str = "Texel density";

/// The colour of a polygon in the stretch view.
fn stretch_color(s: &uv::stretch::Stretch) -> Color32 {
    let off = s.off();
    if !off.is_finite() {
        return theme::STRETCH_NO_ROOM;
    }
    let half = uv::stretch::SOME.log2();
    let mix = |a: Color32, b: Color32, t: f32| {
        let t = t.clamp(0.0, 1.0);
        let c = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
        Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
    };
    if off < half {
        mix(theme::STRETCH_EVEN, theme::STRETCH_SOME, off / half)
    } else {
        mix(
            theme::STRETCH_SOME,
            theme::STRETCH_FAR,
            (off - half) / (1.0 - half),
        )
    }
}

/// The stretch view's note: what is wrong with the polygon under the
/// mouse, or how many polygons are off.
fn stretch_note(shapes: &[(ObjectId, UvShape)], hover: Option<Hit>) -> Option<String> {
    if let Some(Hit {
        id,
        element: Element::Polygon(face),
    }) = hover
    {
        let (_, shape) = shapes.iter().find(|(i, _)| *i == id)?;
        let at = shape.polygons.iter().position(|(f, _)| *f == face)?;
        let s = shape.stretch.get(at)?;
        return Some(match s.explain() {
            Some(why) => format!("This polygon {why}"),
            None => "This polygon lies evenly on the texture.".to_string(),
        });
    }
    let off = shapes
        .iter()
        .flat_map(|(_, s)| &s.stretch)
        .filter(|s| s.off() >= uv::stretch::SOME.log2())
        .count();
    Some(match off {
        0 => "Stretch: every polygon lies evenly.".to_string(),
        1 => "Stretch: 1 polygon is 1.5 times off or more. Point at it to see why.".to_string(),
        n => format!("Stretch: {n} polygons are 1.5 times off or more. Point at one to see why."),
    })
}

/// The texture and texel density line: "2048 px · 498 px/m (aim 512)".
pub(crate) fn density_text(
    texture: scene::TextureSettings,
    density: Option<f32>,
    selected: bool,
) -> String {
    let aim = texture.density.round();
    match density {
        Some(d) => format!(
            "{} px · {}{} px/m (aim {aim})",
            texture.size,
            if selected { "selected " } else { "" },
            d.round()
        ),
        None => format!("{} px · aim {aim} px/m", texture.size),
    }
}

/// The kind of element the editor selects in `mode`: the UV mode works on
/// polygons, a whole island at a time.
pub(crate) fn element_kind(mode: Mode) -> Option<ElementKind> {
    match mode {
        Mode::Points => Some(ElementKind::Points),
        Mode::Edges => Some(ElementKind::Edges),
        Mode::Polygons | Mode::Uv => Some(ElementKind::Polygons),
        Mode::Model | Mode::Object => None,
    }
}

/// Changes the selection by `hits`, combined `how`, as one undo step; in
/// the UV mode each polygon brings its whole island.
fn apply(
    doc: &mut Document,
    shapes: &[(ObjectId, UvShape)],
    hits: &[Hit],
    kind: ElementKind,
    how: Combine,
    mode: Mode,
) {
    let mut changes = Vec::new();
    for (id, shape) in shapes {
        let mine: Vec<Element> = hits
            .iter()
            .filter(|h| h.id == *id)
            .map(|h| h.element)
            .collect();
        if mine.is_empty() && how != Combine::Replace {
            continue;
        }
        let mut selection = doc.elements(*id).cloned().unwrap_or_default();
        if mode == Mode::Uv {
            whole_islands(&mut selection, shape, &mine, how);
        } else {
            selection.combine(kind, mine, how);
        }
        changes.push((*id, selection));
    }
    let count: usize = hits.len();
    let label = match (mode, count) {
        (Mode::Uv, _) => "Select islands".to_string(),
        (_, 0) => "Deselect".to_string(),
        _ => format!("Select {count} {}", kind_name(kind, count)),
    };
    doc.select_elements(changes, label, None);
}

/// Combines whole islands: the islands of `hits` are added, taken away or
/// flipped as one.
fn whole_islands(
    selection: &mut ElementSelection,
    shape: &UvShape,
    hits: &[Element],
    how: Combine,
) {
    let islands: BTreeSet<usize> = hits
        .iter()
        .filter_map(|e| match e {
            Element::Polygon(f) => shape.island_of.get(f.get() as usize).copied(),
            _ => None,
        })
        .filter(|i| *i < shape.islands.len())
        .collect();
    if how == Combine::Replace {
        selection.clear_kind(ElementKind::Polygons);
    }
    for i in islands {
        let faces = &shape.islands[i];
        let all = faces
            .iter()
            .all(|f| selection.contains(Element::Polygon(*f)));
        let add = match how {
            Combine::Replace | Combine::Add => true,
            Combine::Remove => false,
            Combine::Toggle => !all,
        };
        for f in faces {
            if add {
                selection.insert(Element::Polygon(*f));
            } else {
                selection.remove(Element::Polygon(*f));
            }
        }
    }
}

fn kind_name(kind: ElementKind, n: usize) -> &'static str {
    match (kind, n == 1) {
        (ElementKind::Points, true) => "point",
        (ElementKind::Points, false) => "points",
        (ElementKind::Edges, true) => "edge",
        (ElementKind::Edges, false) => "edges",
        (ElementKind::Polygons, true) => "polygon",
        (ElementKind::Polygons, false) => "polygons",
    }
}

/// Twice the signed area of a polygon (counter-clockwise positive).
fn polygon_area(corners: &[Vec2]) -> f32 {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .map(|(a, b)| a.perp_dot(*b))
        .sum::<f32>()
        / 2.0
}

/// Whether `p` is inside the polygon (by the winding of its edges around
/// it, so it works for any shape).
fn inside(corners: &[Vec2], p: Vec2) -> bool {
    let mut winding = 0i32;
    for (a, b) in corners.iter().zip(corners.iter().cycle().skip(1)) {
        if a.y <= p.y {
            if b.y > p.y && (*b - *a).perp_dot(p - *a) > 0.0 {
                winding += 1;
            }
        } else if b.y <= p.y && (*b - *a).perp_dot(p - *a) < 0.0 {
            winding -= 1;
        }
    }
    winding != 0
}

/// How far `p` is from the segment `a`–`b`.
fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0.0 {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Adds a line of `width` to `mesh`, as a thin quad.
fn add_line(mesh: &mut egui::Mesh, a: Pos2, b: Pos2, width: f32, color: Color32) {
    let along = b - a;
    let length = along.length();
    if length < 1e-3 {
        return;
    }
    let side = vec2(-along.y, along.x) / length * (width / 2.0);
    let i = mesh.vertices.len() as u32;
    for p in [a + side, a - side, b - side, b + side] {
        mesh.colored_vertex(p, color);
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

/// Adds a round dot to `mesh`.
/// Adds a dashed line from `a` to `b` to `mesh`: dashes of 6 points with
/// gaps of 4.
fn add_dashes(mesh: &mut egui::Mesh, a: Pos2, b: Pos2, width: f32, color: Color32) {
    let length = a.distance(b);
    if length < 1e-3 {
        return;
    }
    let way = (b - a) / length;
    let mut at = 0.0;
    while at < length {
        let end = (at + 6.0).min(length);
        add_line(mesh, a + way * at, a + way * end, width, color);
        at += 10.0;
    }
}

fn add_dot(mesh: &mut egui::Mesh, at: Pos2, radius: f32, color: Color32) {
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(at, color);
    const STEPS: u32 = 10;
    for k in 0..STEPS {
        let a = k as f32 / STEPS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(at + vec2(a.cos(), a.sin()) * radius, color);
    }
    for k in 0..STEPS {
        mesh.add_triangle(i, i + 1 + k, i + 1 + (k + 1) % STEPS);
    }
}

/// Adds a filled polygon to `mesh` (cut into triangles, so any shape
/// fills right).
fn add_polygon(mesh: &mut egui::Mesh, corners: &[Pos2], color: Color32) {
    if corners.len() < 3 {
        return;
    }
    let i = mesh.vertices.len() as u32;
    for p in corners {
        mesh.colored_vertex(*p, color);
    }
    mesh::triangulate::triangulate(
        corners.len(),
        |k| glam::Vec3::new(corners[k].x, corners[k].y, 0.0),
        |[a, b, c]| mesh.add_triangle(i + a as u32, i + b as u32, i + c as u32),
    );
}

#[cfg(test)]
mod tests;
