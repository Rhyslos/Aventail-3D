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
//!
//! In Paint mode the left button paints on the texture instead of
//! selecting (see [`brushing`]).

use crate::palette::Mode;
use crate::selecting::SelectKeys;
use crate::text::tr;
use crate::theme;
use crate::transforming::{SnapSettings, Tool};
use egui::{Align2, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType, vec2};
use glam::Vec2;
pub(crate) use menus::{density_text, texture_menu, trim_menu};
pub(crate) use navigation::UvFrame;
use painting::{GridTexture, add_line, add_polygon, stretch_note};
pub(crate) use picking::element_kind;
use picking::{Hit, PICK_DISTANCE};
use scene::{Combine, Document, Element, ElementKind, ObjectId, ObjectKind};
use shape::{UvShape, inside, segment_distance};
use std::collections::HashMap;
use std::sync::Arc;
use tools::{DragKind, NavigationSettings};

/// Accessible name of the UV Editor's drawing, and the label tests look for.
pub(crate) const UV_EDITOR_LABEL: &str = crate::text!("UV Editor view");

/// More polygons than this in the shown objects together, and their UVs
/// aren't drawn (a heavy imported shape): drawing them would freeze the
/// window.
pub(crate) const UV_POLYGONS_UP_TO: usize = 300_000;

pub(crate) mod brushing;
pub(crate) mod checking;
pub(crate) mod explain;
pub(crate) mod gizmo;
mod menus;
mod navigation;
mod painted;
mod painting;
mod picking;
pub(crate) mod placing;
mod shape;
pub(crate) mod snapshots;

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

/// The UV Editor's own selection, while it doesn't follow the viewport's
/// (decision 0239).
pub(crate) type OwnSelection = std::collections::BTreeMap<ObjectId, scene::ElementSelection>;

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
    /// Where the mouse was last seen in the window. On Windows, pressing
    /// Alt can make the window report the mouse gone until it moves again;
    /// Alt + A over the editor must still mean the pieces then.
    last_pointer: Option<Pos2>,
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
    /// The texture picture shown behind the UVs (decision 0244): the
    /// picture (as a pointer) and mip level it was made from.
    picture: Option<((usize, u32), GridTexture)>,
    /// The painted texture shown behind the UVs (decision 0177).
    painted: Option<painted::PaintedShown>,
    /// The layout snapshot drawn faintly under the UVs, by name
    /// (decision 0114).
    compare: Option<String>,
    /// What the Snapshots menu last did, to say in it.
    note: Option<String>,
    /// Whether numbers are written on pieces (decision 0115).
    numbers: bool,
    /// Drawn plain, for painting (decision 0225): the objects' names at
    /// the top, no 0 and 1 beside the square, no notes along the bottom.
    plain: bool,
    /// The name drawn at the top while plain, as last drawn.
    title: Option<String>,
    /// Measuring, while it's on (decision 0115).
    measure: Option<checking::Measuring>,
    /// The always-on warnings, for the document as it was last drawn.
    warnings: Option<checking::WarningCache>,
    /// Whether choices are explained while pointing (decision 0116).
    explain: bool,
    /// Why Suggest seams would put each shape's seams where it would.
    reasons: HashMap<ObjectId, explain::SeamReasons>,
    /// What was pointed at and its explanation, while explaining.
    pointed: Option<(explain::Pointed, String)>,
    /// The paint brush, in Paint mode (decision 0180).
    brush: brushing::UvBrush,
    /// Where the notes along the bottom were last drawn (the pieces on
    /// the left, the texture on the right): the paint tools keep off them
    /// (decision 0215).
    notes: [Option<Rect>; 2],
    /// What moves snap to (decision 0237).
    snap_to: placing::UvSnap,
    /// What the gizmo turns and scales about.
    pivot: placing::UvPivot,
    /// The point of your own, placed with Ctrl + click.
    own_pivot: Option<Vec2>,
    /// How the Exact menu shows sizes, and the turn typed there.
    exact_units: placing::ExactUnits,
    exact_turn: f32,
    /// Whether only the pieces holding the selection are shown, and
    /// those shapes as last worked out (decision 0239).
    isolate: bool,
    isolated: HashMap<ObjectId, (Arc<scene::Mesh>, scene::ElementSelection, Option<UvShape>)>,
    /// The editor's own selection, while it doesn't follow the
    /// viewport's, and a count that goes up as it changes.
    own: Option<OwnSelection>,
}

impl Default for UvEditor {
    fn default() -> Self {
        Self {
            snap_to: placing::UvSnap::default(),
            pivot: placing::UvPivot::default(),
            own_pivot: None,
            exact_units: placing::ExactUnits::default(),
            exact_turn: 90.0,
            isolate: false,
            isolated: HashMap::new(),
            own: None,
            centre: Vec2::splat(0.5),
            zoom: 300.0,
            fitted: false,
            rect: None,
            drag: None,
            cache: HashMap::new(),
            frame_request: None,
            shown_pass: None,
            last_pointer: None,
            gizmo: None,
            pressed_on_gizmo: false,
            sessions: 0,
            under_mouse: None,
            grid: None,
            picture: None,
            painted: None,
            compare: None,
            note: None,
            numbers: false,
            plain: false,
            title: None,
            measure: None,
            warnings: None,
            explain: false,
            reasons: HashMap::new(),
            pointed: None,
            brush: brushing::UvBrush::default(),
            notes: [None, None],
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
    /// The texture picture and its mip level while the viewport shows
    /// the Texture picture surface (decision 0244): the editor shows it
    /// behind the UVs, as it is.
    pub(crate) picture: Option<(&'a std::sync::Arc<crate::viewport::Picture>, u32)>,
    /// The painted textures while the viewport shows the Painted surface
    /// (decision 0177): the editor shows its shape's behind the UVs.
    pub(crate) painted: Option<&'a [crate::paint_view::PaintedTexture]>,
    /// Which of the painting's channels the views show (decision 0213):
    /// the editor shows the same.
    pub(crate) channel: crate::attributes::channel_view::ChannelView,
    /// How pieces are grouped onto textures, for the overlap warnings.
    pub(crate) texture_groups: checking::TextureGroups,
    /// What explaining choices needs (decision 0116).
    pub(crate) explain: explain::ExplainContext,
    /// Whether the selection is drawn larger with a dark rim (decision
    /// 0126).
    pub(crate) high_contrast: bool,
}

/// The objects whose UVs are shown: the selected ones with a shape, in
/// selection order (an instance shows its original's).
pub(crate) fn shown_objects(doc: &Document) -> Vec<ObjectId> {
    doc.selection()
        .into_iter()
        .filter(|id| {
            doc.scene().object(*id).is_some_and(|o| {
                o.kind.has_shape() && !matches!(o.kind, ObjectKind::Group | ObjectKind::Curve(_))
            })
        })
        .collect()
}

impl UvEditor {
    /// Every UV edge of the objects shown here, once each, in the
    /// texture's units: the layout export (decision 0244).
    pub(crate) fn layout_lines(&mut self, doc: &Document) -> Vec<[Vec2; 2]> {
        self.shapes(doc)
            .iter()
            .flat_map(|(_, shape)| shape.edges.iter().map(|(line, _, _)| *line))
            .collect()
    }

    /// Whether the mouse is over the editor (as drawn last frame).
    pub(crate) fn hovered(&self, ctx: &egui::Context) -> bool {
        let pass = ctx.cumulative_pass_nr();
        if self.shown_pass.is_none_or(|p| p + 1 < pass) {
            return false;
        }
        let pointer = ctx.input(|i| i.pointer.hover_pos()).or(self.last_pointer);
        self.rect
            .zip(pointer)
            .is_some_and(|(rect, at)| rect.contains(at))
    }

    /// Where `uv` is on screen, as drawn last frame (for tests).
    #[cfg(test)]
    pub(crate) fn screen_of(&self, uv: Vec2) -> Option<Pos2> {
        Some(self.to_screen(self.rect?, uv))
    }

    /// Whether only the pieces holding the selection are shown.
    pub(crate) fn isolating(&self) -> bool {
        self.isolate
    }

    /// Shows only the pieces holding the selection (or everything).
    pub(crate) fn set_isolating(&mut self, isolate: bool, _doc: &Document) {
        self.isolate = isolate;
        self.isolated.clear();
    }

    /// Whether the editor keeps its own selection.
    pub(crate) fn own_selection(&self) -> bool {
        self.own.is_some()
    }

    /// Keeps a selection of its own, starting from the viewport's (or
    /// follows the viewport's again).
    pub(crate) fn set_own_selection(&mut self, own: bool, doc: &Document) {
        self.own = own.then(|| {
            shown_objects(doc)
                .into_iter()
                .filter_map(|id| Some((id, doc.elements(id)?.clone())))
                .collect()
        });
        self.isolated.clear();
    }

    /// The editor's own selection, while it keeps one.
    pub(crate) fn own(&self) -> Option<&OwnSelection> {
        self.own.as_ref()
    }

    /// Object `id`'s selected elements, as the editor uses them.
    pub(crate) fn selected(&self, doc: &Document, id: ObjectId) -> scene::ElementSelection {
        match &self.own {
            Some(own) => own.get(&id).cloned().unwrap_or_default(),
            None => doc.elements(id).cloned().unwrap_or_default(),
        }
    }

    /// `shape` cut down to the pieces holding object `id`'s selection,
    /// while isolating (worked out again when the shape or the selection
    /// changes).
    fn isolated_shape(&mut self, doc: &Document, id: ObjectId, shape: &UvShape) -> Option<UvShape> {
        let selected = self.selected(doc, id);
        if let Some((mesh, was, cut)) = self.isolated.get(&id)
            && Arc::ptr_eq(mesh, &shape.mesh)
            && *was == selected
        {
            return cut.clone();
        }
        let mesh = &shape.mesh;
        let touched: Vec<scene::FaceId> = mesh
            .faces()
            .filter(|(f, list)| {
                selected.contains(Element::Polygon(*f))
                    || list.iter().any(|v| selected.contains(Element::Point(*v)))
                    || mesh
                        .topology()
                        .face_edges(*f)
                        .iter()
                        .any(|e| selected.contains(Element::Edge(*e)))
            })
            .map(|(f, _)| f)
            .collect();
        let cut = if touched.is_empty() {
            None
        } else {
            let keep: std::collections::BTreeSet<scene::FaceId> =
                uv::pieces::pieces_holding(mesh, &touched).into_iter().flatten().collect();
            let gone: Vec<scene::FaceId> = mesh
                .faces()
                .map(|(f, _)| f)
                .filter(|f| !keep.contains(f))
                .collect();
            let mut sub = (**mesh).clone();
            sub.delete_faces(&gone);
            UvShape::of(Arc::new(sub))
        };
        self.isolated
            .insert(id, (Arc::clone(&shape.mesh), selected, cut.clone()));
        cut
    }

    /// What moves snap to, for the Snap menu.
    pub(crate) fn snap_to_mut(&mut self) -> &mut placing::UvSnap {
        &mut self.snap_to
    }

    /// What the gizmo turns and scales about, for the Pivot menu, and the
    /// point of your own.
    pub(crate) fn pivot_mut(&mut self) -> (&mut placing::UvPivot, Option<Vec2>) {
        (&mut self.pivot, self.own_pivot)
    }

    /// Places your own pivot point (Ctrl + click; for tests).
    pub(crate) fn set_own_pivot(&mut self, at: Option<Vec2>) {
        self.own_pivot = at;
    }

    /// The Exact menu, for the selection in `mode`: a typed change is
    /// recorded at once, one undo step.
    pub(crate) fn exact_menu(&mut self, ui: &mut Ui, doc: &mut Document, mode: Mode) {
        let (targets, bounds) = gizmo::uv_targets_with(doc, mode, self.own.as_ref());
        let pixels = doc.scene().texture().size;
        let change = placing::exact_menu(
            ui,
            &mut self.exact_units,
            &mut self.exact_turn,
            bounds,
            pixels,
        );
        let (Some(change), Some((lo, hi))) = (change, bounds) else {
            return;
        };
        let (transform, label) = match change {
            placing::ExactChange::Move(by) => (glam::Affine2::from_translation(by), tr("Move UVs")),
            placing::ExactChange::Scale(f) => (
                gizmo::about((lo + hi) / 2.0, glam::Affine2::from_scale(f)),
                tr("Scale UVs"),
            ),
            placing::ExactChange::Turn(degrees) => (
                gizmo::about(
                    self.pivot.spot(lo, hi, self.own_pivot),
                    glam::Affine2::from_angle(degrees.to_radians()),
                ),
                tr("Rotate UVs"),
            ),
        };
        self.sessions += 1;
        let key = gizmo::UV_GIZMO_KEY | (1 << 40) | self.sessions;
        let _ = gizmo::record(doc, &targets, transform, label, key);
    }

    /// The texture place under screen point `at`, as last drawn.
    pub(crate) fn uv_of(&self, at: Pos2) -> Option<Vec2> {
        Some(self.to_uv(self.rect?, at))
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
        let (_, bounds) = gizmo::uv_targets_with(doc, cx_mode, self.own.as_ref());
        let (min, max) = bounds?;
        Some(self.to_screen(self.rect?, (min + max) / 2.0))
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
        if self.isolate {
            let mut shown = Vec::new();
            for (id, shape) in out {
                let cut = self.isolated_shape(doc, id, &shape);
                shown.push((id, cut.unwrap_or(shape)));
            }
            return shown;
        }
        out
    }

    /// Draws the editor and handles the mouse. Selection changes go
    /// straight to the document, as one undo step each, like the
    /// viewport's.
    /// Where the editor was last drawn.
    pub(crate) fn drawn(&self) -> Option<Rect> {
        self.rect
    }

    /// The name written at its top while painting (decision 0225), as
    /// last drawn.
    #[cfg(test)]
    pub(crate) fn plain_title(&self) -> Option<&str> {
        self.title.as_deref().filter(|_| self.plain)
    }

    /// Where its notes along the bottom were last drawn (decision 0215).
    pub(crate) fn bottom_notes(&self) -> [Option<Rect>; 2] {
        self.notes
    }

    pub(crate) fn show(&mut self, ui: &mut Ui, doc: &mut Document, cx: &UvContext<'_>) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(UV_EDITOR_LABEL)));
        self.rect = Some(rect);
        self.notes = [None, None];
        self.plain = cx.mode == Mode::Paint;
        self.shown_pass = Some(ui.ctx().cumulative_pass_nr());
        if let Some(at) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            self.last_pointer = Some(at);
        }
        if !self.fitted && rect.width() > 10.0 {
            self.fit(rect, Vec2::ZERO, Vec2::ONE);
            self.fitted = true;
        }
        let shapes = self.shapes(doc);
        self.frame_from_request(rect, &shapes, doc);
        // Ctrl + drag changes the paint brush instead of moving the view.
        if !self.brush_wants(ui, &response) {
            self.navigate(ui, &response, rect, cx.navigation);
        }
        // Measuring takes the left button from the brush, the gizmo and
        // selecting.
        let measuring = self.drag.is_none() && self.use_measure(ui, &response, rect, doc);
        let painting = self.brush.shown.is_some() && !measuring;
        if painting {
            self.use_brush(ui, &response, rect);
        } else {
            self.brush.hover = None;
        }
        let lit = if measuring || painting {
            None
        } else {
            self.use_gizmo(ui, &response, rect, doc, cx)
        };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::UV_BACKGROUND);
        let (image, dim) = match cx.painted {
            Some(textures) => (
                self.painted_texture(ui.ctx(), doc, textures, cx.channel),
                false,
            ),
            None => match cx.picture {
                Some((picture, mip)) => (Some(self.picture_texture(ui.ctx(), picture, mip)), false),
                None => (
                    cx.grid.map(|colors| self.grid_texture(ui.ctx(), colors)),
                    true,
                ),
            },
        };
        self.paint_square(&painter, rect, doc.scene().texture().size, image, dim);
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
        let warnings = self.warnings(doc, &shapes, cx.texture_groups).clone();
        for (id, shape) in &shapes {
            if let Some(w) = warnings.get(id) {
                self.paint_warnings(&painter, rect, shape, w);
            }
        }
        let mut polygons = 0;
        let mut islands = 0;
        for (id, shape) in &shapes {
            polygons += shape.polygons.len();
            islands += shape.islands.len();
            let selected = self.selected(doc, *id);
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
                cx.high_contrast,
            );
        }
        if let Some((centre, lit)) = lit {
            gizmo::paint(&painter, self.to_screen(rect, centre), cx.tool, lit);
        }
        // Your own pivot point, while chosen (decision 0237).
        if self.pivot == placing::UvPivot::Own
            && let Some(at) = self.own_pivot
        {
            let p = self.to_screen(rect, at);
            let stroke = Stroke::new(1.5, accent.light);
            painter.line_segment([p - vec2(6.0, 0.0), p + vec2(6.0, 0.0)], stroke);
            painter.line_segment([p - vec2(0.0, 6.0), p + vec2(0.0, 6.0)], stroke);
            painter.circle_stroke(p, 3.5, stroke);
        }
        if self.numbers {
            for (_, shape) in &shapes {
                self.paint_numbers(&painter, rect, shape, doc.scene().texture());
            }
        }
        self.paint_measure(&painter, rect, doc.scene().texture());
        if let Some(text) = checking::warning_text(checking::WarningCounts::of(warnings.values())) {
            painter.text(
                rect.right_top() + vec2(-10.0, 10.0),
                Align2::RIGHT_TOP,
                text,
                FontId::proportional(12.0),
                theme::UV_WARNING_TEXT,
            );
        }
        if !measuring && !painting {
            self.select(ui, &response, rect, &shapes, doc, cx);
        }
        if painting {
            self.paint_brush(ui, rect, &response);
        }
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
        if self.plain {
            self.paint_name(&painter, rect, doc, &shapes, too_many);
        } else {
            let note = if shapes.is_empty() && !too_many {
                if shown_objects(doc).is_empty() {
                    tr("Select an object to see its UVs.").to_string()
                } else {
                    tr("The selected objects have no UVs.").to_string()
                }
            } else if too_many {
                crate::trf!(
                    "Some selected objects have more than {} polygons: their UVs aren't drawn.",
                    UV_POLYGONS_UP_TO
                )
            } else {
                let hint = match cx.mode {
                    Mode::Model | Mode::Object => tr(" · press 1, 2, 3 or pick UV mode to select"),
                    _ => "",
                };
                crate::trf!(
                    "{} {} · {} {}{}",
                    islands,
                    if islands == 1 {
                        tr("piece")
                    } else {
                        tr("pieces")
                    },
                    polygons,
                    if polygons == 1 {
                        tr("polygon")
                    } else {
                        tr("polygons")
                    },
                    hint
                )
            };
            // Drawn after the texture's note on the right, in the room that
            // leaves (wrapping onto more lines rather than running under it).
            let left_note = note;
            // The texture, and the texel density shown (of the selected
            // polygons, or of everything shown), bottom right.
            let texture = doc.scene().texture();
            let mut flat = 0.0;
            let mut real = 0.0;
            let mut chosen = 0;
            for (id, shape) in &shapes {
                let selected = self.selected(doc, *id);
                for ((face, _), (f, r)) in shape.polygons.iter().zip(&shape.areas) {
                    if kind == Some(ElementKind::Polygons)
                        && selected.contains(Element::Polygon(*face))
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
            let density = (flat > 1e-20 && real > 1e-20)
                .then(|| texture.pixels_per_metre((flat / real).sqrt()));
            let text = density_text(texture, density, chosen > 0);
            self.notes[1] = Some(painter.text(
                rect.right_bottom() + vec2(-10.0, -10.0),
                Align2::RIGHT_BOTTOM,
                text,
                FontId::proportional(12.0),
                theme::TEXT_WEAK,
            ));
            let room = self.notes[1].map_or(rect.right() - 10.0, |r| r.left() - 16.0)
                - (rect.left() + 10.0);
            let galley = painter.layout(
                left_note,
                FontId::proportional(12.0),
                theme::TEXT_WEAK,
                room.max(60.0),
            );
            let at = rect.left_bottom() + vec2(10.0, -10.0 - galley.size().y);
            self.notes[0] = Some(egui::Rect::from_min_size(at, galley.size()));
            painter.galley(at, galley, theme::TEXT_WEAK);
        }
        // Why what the mouse points at is there (decision 0116), on top.
        self.pointed = None;
        if self.explain
            && self.drag.is_none()
            && let Some(at) = response.hover_pos()
        {
            let pointed = self.point_at(rect, &shapes, at);
            if let Some(pointed) = pointed
                && let Some(text) = self.explanation(&pointed, &shapes, doc, &warnings, cx.explain)
            {
                self.pointed = Some((pointed, text));
                self.paint_explanation(&painter, rect, &shapes, at);
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
        let (targets, bounds) = gizmo::uv_targets_with(doc, cx.mode, self.own.as_ref());
        let (min, max) = bounds?;
        let centre = self.pivot.spot(min, max, self.own_pivot);
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
            let (moving, points, segments) = placing::gather(doc, &targets);
            let rows = doc
                .scene()
                .trim()
                .rows
                .iter()
                .flat_map(|r| [r.bottom, r.top])
                .collect();
            let snapping = placing::UvSnapping {
                settings: self.snap_to,
                pixels: doc.scene().texture().size,
                reach: placing::SNAP_REACH / self.zoom.max(1e-3),
                moving,
                points,
                segments,
                rows,
            };
            let pieces = if self.pivot == placing::UvPivot::EachPiece {
                placing::per_piece(doc, &targets)
            } else {
                Vec::new()
            };
            let mut drag = gizmo::GizmoDrag::new(handle, centre, targets, from, key)
                .with_snapping(snapping)
                .with_pieces(pieces);
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
}

#[cfg(test)]
mod tests;
