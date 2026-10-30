//! Painting with the brush in Paint mode ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)).
//!
//! A stroke begins where the mouse is pressed on a shape: it paints the
//! texture set of the material there, into its top layer (a "Paint" layer
//! is made if the set has none, and the set itself if the material has
//! none). Every frame of the stroke the layer is given to the document
//! through `Command::SetTextureSet`, merged into one undo step, so the
//! views show the paint as it goes on and undo takes the whole stroke.
//!
//! In the UV Editor (decision 0180) the same brush paints flat on the
//! texture: only inside the UV pieces of the texture set's material, on
//! every shown object that uses it.

use super::mirroring::{self, ModelMirror};
use super::{LayoutChoice, Workbench};
use crate::attributes::colour_picker::PaletteRequest;
use crate::attributes::{BrushSettings, PaintTool};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::transforming::Tool;
use crate::uv_editor::brushing::UvPointer;
use crate::viewport::{BrushEvent, Surface};
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Vec2, Vec3};
use paint::{
    BrushKind, Dab, DepthMap, Layer, PaintSurface, StrokeCanvas, Target, TextureSet, View,
};
use scene::{Mesh, ObjectId};
use std::collections::HashMap;
use std::sync::Arc;

/// Merge keys of paint strokes: apart from every other drag's.
const STROKE_KEYS: u64 = 0x5041_494E_0000_0000;

/// The status bar's hint after a press in the UV Editor outside the
/// pieces it can paint (cleared once a stroke starts).
const INSIDE_A_PIECE: &str = crate::text!(
    "Paint inside a UV piece of the selected object: press the left button over one, and drag. \
     To paint another object's pieces, select it."
);

/// How far round a smudge picks up colour, in texels.
const SMUDGE_REACH: f32 = 3.0;

/// How much of the colour under it a smudge picks up at each dab.
const SMUDGE_PICKUP: f32 = 0.3;

/// How much [ and ] change the brush's size.
const SIZE_STEP: f32 = 1.15;

/// The paint brush's state on the workbench.
#[derive(Debug, Default)]
pub(super) struct Brushing {
    /// Each object's shape as the brush sees it, made again when the
    /// shape changes.
    surfaces: HashMap<ObjectId, (Arc<Mesh>, Arc<PaintSurface>)>,
    /// The stroke under way.
    session: Option<BrushSession>,
    /// The vertex colour stroke under way (decision 0205).
    pub(super) vertex: Option<super::vertex_painting::VertexSession>,
    /// Strokes made so far, for their undo merge keys.
    strokes: u64,
    /// The surface the viewport showed before Paint mode, put back after.
    surface_before: Option<Surface>,
    /// Whether Paint mode was on last frame.
    on: bool,
    /// What the brush painted on last frame, to switch the views' surface
    /// when it changes (decision 0205).
    painted_on: Option<crate::attributes::PaintOn>,
    /// The layout before Paint mode switched to the Paint one, put back
    /// after (decision 0181).
    layout_before: Option<super::LayoutChoice>,
    /// The paint preview in use before a channel was shown on its own,
    /// for All to put back (decision 0213).
    preview_before: Option<crate::viewport::PaintPreview>,
    /// P was pressed: pick the colour under the mouse (decision 0183).
    pick_now: bool,
    /// The Paint page's select arrow is on (decision 0224): clicks select
    /// objects instead of painting.
    pub(super) selecting: bool,
    /// A press picked a colour: the drag that follows paints nothing.
    picked_press: bool,
    /// The tablet pen's state, from the app (decision 0184).
    tablet: Option<crate::PenState>,
    /// A pen's pressure through the system's touch events.
    touch: Option<f32>,
    /// Where the last stroke ended, for Shift + click lines: in a 3D view
    /// (screen points) or in the UV Editor (on the texture).
    last_end: Option<LastEnd>,
    /// Whether V is held: a click then sets where clone copies from
    /// (decision 0186).
    v_held: bool,
    /// Where clone copies from, on the texture.
    clone_source: Option<Vec2>,
    /// How far clone copies from, in texels, once the first stroke after
    /// setting the source has fixed it (aligned, as in Substance Painter).
    clone_offset: Option<(i32, i32)>,
    /// The picture projection paints, read by the app.
    stencil: Option<Arc<paint::Stencil>>,
    /// The picture as read, waiting to be shown over the views.
    stencil_pending: Option<(String, [usize; 2], Vec<u8>)>,
    /// The picture shown over the views.
    stencil_texture: Option<StencilTexture>,
}

impl Brushing {
    /// The projection brush's picture, as shown over the views, if one
    /// is chosen (for the shelf's Stencils tab, decision 0229).
    pub(super) fn stencil_picture(&self) -> Option<egui::TextureId> {
        self.stencil_texture.as_ref().map(|t| t.0.id())
    }

    /// Whether a stroke is under way.
    pub(super) fn stroking(&self) -> bool {
        self.session.is_some()
    }
}

/// The picture shown over the views, as an egui texture.
#[derive(Clone)]
struct StencilTexture(egui::TextureHandle);

impl std::fmt::Debug for StencilTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StencilTexture").field(&self.0.id()).finish()
    }
}

/// Where the last stroke ended (decision 0184).
#[derive(Debug, Clone, Copy, PartialEq)]
enum LastEnd {
    /// In a 3D view, in screen points, seen through this camera.
    Model(Vec2, tools::Camera),
    /// In the UV Editor, on the texture.
    Flat(Vec2),
}

/// A stroke under way.
#[derive(Debug)]
struct BrushSession {
    material: String,
    layer: paint::LayerPath,
    set: TextureSet,
    canvas: StrokeCanvas,
    /// Where it is painted: on the model in a 3D view, or flat in the UV
    /// Editor.
    place: StrokePlace,
    /// The mouse's last place, in screen points (in the UV Editor, the
    /// place on the texture times the zoom).
    last: Vec2,
    since: Option<f32>,
    /// The distance between dabs on the model, in screen points, from the
    /// last dab's size there (none before the first: the brush's own).
    /// Sized in the object or the texture, the brush's size on screen
    /// differs from its viewport size; spacing by that left dots apart (a
    /// review found it).
    step: Option<f32>,
    key: u64,
    /// The stroke as painted so far, kept with the layer (decision 0179).
    record: paint::StrokeRecord,
    /// The layer's strokes before this one.
    strokes_before: Vec<Arc<paint::StrokeRecord>>,
    /// Whether anything has been painted yet (the record is kept with the
    /// layer when the stroke ends, only then).
    painted: bool,
    /// Whether it paints the layer's mask (decision 0182).
    on_mask: bool,
    /// Paint or erase: the brush's, or erasing with a pen's eraser end
    /// (decision 0184).
    kind: BrushKind,
    /// The pen's pressure at `last` (none for a mouse).
    pressure: Option<f32>,
    /// Where the lazy brush is, following the pointer on its string
    /// (decision 0184), in the same points as `last`.
    lazy: Vec2,
    /// Which way the stroke goes, in radians, in the same points as
    /// `last` (decision 0185).
    angle: f32,
    /// The triangles the polygon fill tool has filled in this drag, by
    /// shape (decision 0182).
    filled: std::collections::HashSet<(usize, usize)>,
    /// The realism channels painted with the colour (decision 0207): each
    /// channel's image on the layer as the stroke goes, and its value.
    channels: Vec<(paint::Channel, StrokeCanvas, [u8; 4])>,
}

/// Paints `jobs` on a stroke's realism channels' canvases too, each with
/// its own value in place of the colour.
fn paint_channels(
    channels: &mut [(paint::Channel, StrokeCanvas, [u8; 4])],
    jobs: &[paint::DabJob<'_>],
) {
    {
        for (_, canvas, value) in channels.iter_mut() {
            let jobs: Vec<paint::DabJob<'_>> = jobs
                .iter()
                .map(|job| {
                    let mut job = *job;
                    job.brush.color = *value;
                    job
                })
                .collect();
            paint::paint_dabs(canvas, &jobs);
        }
    }
}

/// Where a stroke is painted. (Only one stroke is under way at a time,
/// so the 3D view's larger variant costs nothing worth boxing.)
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum StrokePlace {
    /// On the model, in a 3D view.
    Model {
        slots: Vec<u32>,
        surface: Arc<PaintSurface>,
        model: Mat4,
        view: View,
        depth: DepthMap,
        camera: tools::Camera,
        size: Vec2,
        /// The other side, when mirror painting (decision 0187).
        mirror: Option<ModelMirror>,
    },
    /// Flat on the texture, in the UV Editor.
    Flat {
        /// Each shown object's shape using the material, and its slots
        /// that do.
        targets: Vec<(Arc<PaintSurface>, Vec<u32>)>,
        /// Screen points per texture width when the stroke began.
        zoom: f32,
        /// Scene units per texture width where the stroke began, for a
        /// brush measured on the surface.
        units_per_uv: f32,
        /// Whether each dab is painted again flipped left and right
        /// (mirror painting, decision 0187).
        mirror: bool,
    },
}

/// What a ray from the mouse hits.
pub(super) struct Hit {
    pub(super) id: ObjectId,
    pub(super) world: Vec3,
    pub(super) triangle: usize,
    pub(super) surface: Arc<PaintSurface>,
    pub(super) model: Mat4,
}

impl Workbench {
    /// Paint mode on or off, each frame before the viewport is drawn: the
    /// brush shown in the views, its settings in Attributes, and the
    /// Painted surface while painting.
    pub(super) fn update_paint_mode(&mut self, drawing: &SceneDrawing) {
        let on = self.mode == Mode::Paint;
        if on && !self.brushing.on {
            self.brushing.surface_before = Some(self.viewport.options().surface);
            self.viewport.options_mut().surface = Surface::Painted;
            self.brushing.painted_on = None;
            // The Paint layout (decision 0181), and back after.
            if self.layout_choice != LayoutChoice::Paint {
                self.brushing.layout_before = Some(self.layout_choice);
                self.swap_layout(LayoutChoice::Paint);
            }
        }
        if !on && self.brushing.on {
            if let Some(before) = self.brushing.surface_before.take()
                && matches!(
                    self.viewport.options().surface,
                    Surface::Painted | Surface::VertexColours
                )
            {
                self.viewport.options_mut().surface = before;
            }
            self.end_stroke();
            self.brushing.vertex = None;
            let before = self.brushing.layout_before.take();
            if self.layout_choice == LayoutChoice::Paint {
                let back = before.unwrap_or(match self.mode {
                    Mode::Uv => LayoutChoice::UvEdit,
                    _ => LayoutChoice::Standard,
                });
                self.swap_layout(back);
            }
        }
        // What the views show follows what the brush paints on
        // (decision 0205): the painting, or the vertex colours.
        let paint_on = self.attributes.brush.paint_on;
        if on && self.brushing.painted_on != Some(paint_on) {
            if self.brushing.painted_on.is_some() || paint_on != crate::attributes::PaintOn::Texture
            {
                self.viewport.options_mut().surface = match paint_on {
                    crate::attributes::PaintOn::Texture => Surface::Painted,
                    crate::attributes::PaintOn::VertexColours => Surface::VertexColours,
                };
            }
            self.brushing.painted_on = Some(paint_on);
        }
        self.brushing.on = on;
        self.attributes.painting = on;
        // The floating brush bar shows while painting (decision 0213).
        self.attributes.brush_bar_shown = on;
        self.attributes.brush_in_paint_tab =
            self.dock.find_tab(&crate::panel::Panel::Paint).is_some();
        self.attributes.pen = self.pen();
        self.viewport.set_paint_page(on);
        // With the select arrow, move, rotate or scale chosen (decision
        // 0224), the brush steps aside: clicks select, drags move.
        let hands_busy = self.tool != Tool::Select || self.brushing.selecting;
        if !on || hands_busy {
            if !on {
                self.brushing.selecting = false;
            }
            self.viewport.set_paint_brush(None);
            self.uv_editor.set_paint_brush(None);
            self.viewport.set_stencil_overlay(None);
            self.viewport.set_mirror_mark(None);
            return;
        }
        // The picture projection paints, over the views while it's the
        // brush (decision 0186).
        let overlay = (self.attributes.brush.brush.kind == BrushKind::Projection)
            .then_some(())
            .and(self.brushing.stencil_texture.as_ref())
            .and_then(|texture| {
                let stencil = self.brushing.stencil.as_ref()?;
                let settings = self.attributes.projection;
                Some(crate::viewport::StencilOverlay {
                    texture: texture.0.id(),
                    size: Vec2::new(stencil.width as f32, stencil.height as f32)
                        * settings.scale.max(0.01),
                    offset: Vec2::from(settings.offset),
                    opacity: settings.shown,
                })
            });
        self.viewport.set_stencil_overlay(overlay);
        // What is under the mouse: the size on screen of a brush measured
        // on the surface, and what it would paint.
        let hover = self.viewport.pointer().and_then(|p| {
            let hit = self.brush_hit(drawing, &p.camera, p.size, p.at)?;
            Some((p, hit))
        });
        let settings = self.attributes.brush;
        let mut radius = settings.brush.radius;
        self.attributes.brush_target = Default::default();
        // The brush offers the realism channels while the material shown
        // uses them (decision 0207).
        self.attributes.realism = self
            .paint_layers
            .material(&self.document)
            .and_then(|m| self.document.scene().paint().set(&m).map(|s| s.realism))
            .unwrap_or(false);
        self.show_channel_asked();
        let mut mirror_mark = None;
        if let Some((pointer, hit)) = &hover {
            let per_point = pointer
                .camera
                .world_per_pixel(hit.world, pointer.size)
                .max(1e-6);
            let texels = self.texture_size().and_then(|size| {
                paint::surface::texels_per_unit(
                    &hit.surface.triangles()[hit.triangle],
                    hit.model,
                    size,
                )
            });
            radius = settings.screen_radius(per_point, texels);
            // Where the other side's dab would go (decision 0187).
            if self.mirroring() {
                let world = hit.model * settings.mirror_axis.reflection() * hit.model.inverse();
                mirror_mark = Some((world.transform_point3(hit.world), radius * per_point));
            }
            let triangle = hit.surface.triangles()[hit.triangle];
            self.attributes.brush_target =
                self.target_of(hit.id, &triangle, hit.model, radius * per_point);
        }
        // The fill tool has no size: a cross, not a circle.
        let sized = |r: f32| {
            if settings.tool == PaintTool::Fill {
                0.0
            } else {
                r.max(0.5)
            }
        };
        self.viewport
            .set_paint_brush(Some((sized(radius), settings.brush.hardness)));
        self.viewport.set_mirror_mark(mirror_mark);
        // The UV Editor's brush: its size on the texture, and what it
        // would paint when the mouse is there instead.
        let mut flat_radius = settings.brush.radius;
        if let Some(pointer) = self.uv_editor.brush_hover()
            && let Some((id, triangle, units_per_uv)) = self.flat_hit(drawing, pointer.uv)
        {
            let units_per_point = units_per_uv / pointer.zoom.max(1e-6);
            flat_radius = settings.flat_radius(pointer.zoom, units_per_uv, self.texture_size())
                * pointer.zoom;
            if hover.is_none() {
                let model = drawn_transform(drawing, id);
                self.attributes.brush_target =
                    self.target_of(id, &triangle, model, flat_radius * units_per_point);
            }
        }
        self.uv_editor
            .set_paint_brush(Some((sized(flat_radius), settings.brush.hardness)));
    }

    /// What the brush would paint on `triangle` of object `id` (placed by
    /// `model`), `radius` scene units across, for Attributes.
    fn target_of(
        &self,
        id: ObjectId,
        triangle: &paint::surface::PaintTriangle,
        model: Mat4,
        radius: f32,
    ) -> crate::attributes::BrushTarget {
        let material = self.material_at(id, triangle.slot);
        let set = self.document.scene().paint().set(&material).cloned();
        let size = set
            .as_ref()
            .map_or(self.document.scene().texture().size, |s| s.size);
        let layer = set
            .as_ref()
            .and_then(|s| {
                let path = self.paint_target(&material, s)?;
                s.layer(&path)
                    .filter(|l| l.image().is_some() && !l.is_made())
            })
            .map_or_else(|| tr("a new paint layer").to_string(), |l| l.name.clone());
        crate::attributes::BrushTarget {
            layer,
            material,
            texel: texel_width(triangle, model, size),
            radius: Some(radius),
        }
    }

    /// The brush's events from the views, after they are drawn.
    pub(super) fn carry_out_brush(&mut self, drawing: &SceneDrawing) {
        self.vertex_colours_from_texture();
        if std::mem::take(&mut self.brushing.pick_now) {
            let picked = match (self.viewport.pointer(), self.uv_editor.brush_hover()) {
                (Some(p), _) => self.pick_on_model(drawing, &p.camera, p.size, p.at),
                (None, Some(p)) => self.pick_flat(drawing, p.uv),
                (None, None) => false,
            };
            if !picked {
                self.note = Some(
                    tr("Point at a shape (or a UV piece) and press P to take its colour.")
                        .to_string(),
                );
            }
        }
        let on_vertices =
            self.attributes.brush.paint_on == crate::attributes::PaintOn::VertexColours;
        for event in self.viewport.take_brush_events() {
            if on_vertices {
                // Vertex colours (decision 0205): painted on the model only.
                match event {
                    BrushEvent::Begin(p) | BrushEvent::Line(p) if !self.attributes.picking => {
                        self.begin_vertex_stroke(drawing, &p.camera, p.size, p.at);
                    }
                    BrushEvent::Move(p) => {
                        self.continue_vertex_stroke(drawing, &p.camera, p.size, p.at);
                    }
                    BrushEvent::End => self.end_vertex_stroke(),
                    BrushEvent::Adjust {
                        size,
                        hardness,
                        flow,
                    } => self.adjust_brush(size, hardness, flow),
                    _ => {}
                }
                continue;
            }
            match event {
                BrushEvent::Begin(p) if self.attributes.picking => {
                    self.attributes.picking = false;
                    self.brushing.picked_press = true;
                    self.pick_on_model(drawing, &p.camera, p.size, p.at);
                }
                BrushEvent::Begin(p) if self.setting_clone_source() => {
                    self.brushing.picked_press = true;
                    if let Some(hit) = self.brush_hit(drawing, &p.camera, p.size, p.at) {
                        let uv = uv_at(
                            &hit.surface.triangles()[hit.triangle],
                            hit.model.inverse().transform_point3(hit.world),
                        );
                        self.set_clone_source(uv);
                    }
                }
                BrushEvent::Begin(p) => self.begin_stroke(drawing, &p.camera, p.size, p.at),
                BrushEvent::Line(p) => match self.brushing.last_end {
                    Some(LastEnd::Model(from, camera)) if camera == p.camera => {
                        self.begin_stroke(drawing, &p.camera, p.size, from);
                        self.stroke_to(p.at);
                    }
                    _ => self.begin_stroke(drawing, &p.camera, p.size, p.at),
                },
                BrushEvent::Move(_) if self.brushing.picked_press => {}
                BrushEvent::Move(p) => self.continue_stroke(p.at),
                BrushEvent::End => {
                    if let Some(session) = self.end_stroke()
                        && let StrokePlace::Model { camera, .. } = &session.place
                    {
                        self.brushing.last_end = Some(LastEnd::Model(session.last, *camera));
                    }
                    self.brushing.picked_press = false;
                }
                BrushEvent::Adjust {
                    size,
                    hardness,
                    flow,
                } => self.adjust_brush(size, hardness, flow),
            }
        }
        for event in self.uv_editor.take_brush_events() {
            if on_vertices {
                if matches!(event, BrushEvent::Begin(_)) {
                    self.note = Some(
                        tr(
                            "Vertex colours belong to the shape's corners: paint them on the \
                         model, not in the UV Editor.",
                        )
                        .to_string(),
                    );
                }
                continue;
            }
            match event {
                BrushEvent::Begin(p) if self.attributes.picking => {
                    self.attributes.picking = false;
                    self.brushing.picked_press = true;
                    self.pick_flat(drawing, p.uv);
                }
                BrushEvent::Begin(p) if self.setting_clone_source() => {
                    self.brushing.picked_press = true;
                    self.set_clone_source(p.uv);
                }
                BrushEvent::Begin(p) => self.begin_flat_stroke(drawing, p),
                BrushEvent::Line(p) => match self.brushing.last_end {
                    Some(LastEnd::Flat(from)) => {
                        self.begin_flat_stroke(drawing, UvPointer { uv: from, ..p });
                        let zoom = p.zoom.max(1e-6);
                        self.stroke_to(p.uv * zoom);
                    }
                    _ => self.begin_flat_stroke(drawing, p),
                },
                BrushEvent::Move(_) if self.brushing.picked_press => {}
                BrushEvent::Move(p) => self.continue_flat_stroke(p),
                BrushEvent::End => {
                    if let Some(session) = self.end_stroke()
                        && let StrokePlace::Flat { zoom, .. } = &session.place
                    {
                        self.brushing.last_end = Some(LastEnd::Flat(session.last / *zoom));
                    }
                    self.brushing.picked_press = false;
                }
                BrushEvent::Adjust {
                    size,
                    hardness,
                    flow,
                } => self.adjust_brush(size, hardness, flow),
            }
        }
    }

    /// How many texels across the texture of the material being painted
    /// is, for a brush measured on the texture (decision 0217).
    fn texture_size(&self) -> Option<f32> {
        let material = self.paint_layers.material(&self.document)?;
        let set = self.document.scene().paint().set(&material)?;
        Some(set.size as f32)
    }

    /// The Paint page's tool row's view of the tools in use (decision
    /// 0224).
    pub(super) fn paint_hands(&self) -> crate::paint_tools::Hands {
        crate::paint_tools::Hands {
            tool: self.tool,
            selecting: self.brushing.selecting,
            picking: self.attributes.picking,
        }
    }

    /// Takes what the tool row chose: the workbench's tool (through
    /// `set_tool`, so a session under way ends properly), the select arrow,
    /// and picking a colour.
    pub(super) fn set_paint_hands(&mut self, hands: crate::paint_tools::Hands) {
        if hands.tool != self.tool {
            self.set_tool(hands.tool);
        }
        self.brushing.selecting = hands.selecting;
        self.attributes.picking = hands.picking;
    }

    /// A Ctrl + drag's change to the brush: its size times `size`, its
    /// hardness and flow plus the others.
    fn adjust_brush(&mut self, size: f32, hardness: f32, flow: f32) {
        let settings = &mut self.attributes.brush;
        settings.scale_size(size);
        settings.brush.hardness += hardness;
        settings.brush.flow += flow;
        settings.brush = settings.brush.clamped();
    }

    /// Paint mode's keys: 1 paint, 2 erase and 4 polygon fill (3, 5 and
    /// 6 are the tools still to come), P picks a colour (decision 0183), [ and ] the size, Esc back to the Model mode.
    pub(super) fn paint_keys(&mut self, ui: &Ui) {
        if self.mode != Mode::Paint {
            return;
        }
        // A pen through the system's touch: its pressure this frame.
        if let Some(touch) = crate::pen::touch_pressure(ui.ctx()) {
            self.brushing.touch = touch;
        }
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |ui: &Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        // A paint tool's key puts down the select arrow and the transform
        // tools (decision 0224), back to painting.
        let paint_key = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
        ]
        .into_iter()
        .any(|k| ui.input(|i| i.key_pressed(k) && i.modifiers.is_none()));
        // Q (select, as everywhere) turns the select arrow on here.
        if ui.input(|i| i.key_pressed(Key::Q) && i.modifiers.is_none()) {
            self.brushing.selecting = true;
            self.attributes.picking = false;
        }
        if paint_key {
            self.brushing.selecting = false;
            self.attributes.picking = false;
            if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            }
        }
        if pressed(ui, Key::Num1) {
            self.attributes.brush.tool = PaintTool::Brush;
            self.attributes.brush.brush.kind = BrushKind::Paint;
        }
        if pressed(ui, Key::Num2) {
            self.attributes.brush.tool = PaintTool::Brush;
            self.attributes.brush.brush.kind = BrushKind::Erase;
        }
        if pressed(ui, Key::Num4) {
            self.attributes.brush.choose_fill();
        }
        if pressed(ui, Key::Num8) {
            self.attributes.brush.choose_bucket();
        }
        for (key, kind) in [
            (Key::Num3, BrushKind::Projection),
            (Key::Num5, BrushKind::Smudge),
            (Key::Num6, BrushKind::Clone),
            (Key::Num7, BrushKind::Blur),
        ] {
            if pressed(ui, key) {
                self.attributes.brush.tool = PaintTool::Brush;
                self.attributes.brush.brush.kind = kind;
            }
        }
        // V held: a click sets where clone copies from (V alone does
        // nothing else in Paint mode).
        pressed(ui, Key::V);
        self.brushing.v_held = ui.input(|i| i.key_down(Key::V));
        // A picture to project, read by the app: shown over the views.
        if let Some((name, size, rgba)) = self.brushing.stencil_pending.take() {
            let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
            self.brushing.stencil_texture = Some(StencilTexture(ui.ctx().load_texture(
                name,
                image,
                egui::TextureOptions::LINEAR,
            )));
        }
        if pressed(ui, Key::P) {
            self.brushing.pick_now = true;
        }
        if pressed(ui, Key::L) {
            let settings = &mut self.attributes.brush;
            settings.mirror_on = !settings.mirror_on;
            self.note = Some(if settings.mirror_on {
                crate::trf!(
                    "Mirror painting on, across the object's {} axis: each dab is painted on the \
                     other side too. L turns it off.",
                    settings.mirror_axis.name()
                )
            } else {
                tr("Mirror painting off. L turns it on.").to_string()
            });
        }
        if pressed(ui, Key::D) {
            let settings = &mut self.attributes.brush;
            settings.smoothing_on = !settings.smoothing_on;
            self.note = Some(if settings.smoothing_on {
                tr("Smoothing on: the brush follows the pointer on a string. D turns it off.")
                    .to_string()
            } else {
                tr("Smoothing off. D turns it on.").to_string()
            });
        }
        let settings = &mut self.attributes.brush;
        for (key, factor) in [
            (Key::OpenBracket, 1.0 / SIZE_STEP),
            (Key::CloseBracket, SIZE_STEP),
        ] {
            if pressed(ui, key) {
                settings.scale_size(factor);
            }
        }
        // Esc closing a menu or a pop-up only closes it.
        if self.brushing.session.is_none()
            && !egui::Popup::is_any_open(ui.ctx())
            && pressed(ui, Key::Escape)
        {
            self.mode = Mode::Model;
        }
    }

    /// Whether a press sets where clone copies from: V held with the clone
    /// brush (decision 0186).
    fn setting_clone_source(&self) -> bool {
        self.brushing.v_held && self.attributes.brush.brush.kind == BrushKind::Clone
    }

    /// Clone copies from `uv` from now on; the next stroke fixes how far.
    fn set_clone_source(&mut self, uv: Vec2) {
        self.brushing.clone_source = Some(uv);
        self.brushing.clone_offset = None;
        self.attributes.clone_source_set = true;
        self.note = Some(tr("Clone source set. Paint where the copy should go.").to_string());
    }

    /// Takes the picture `image` (read from a file called `name`) as the
    /// one projection paints (decision 0186), laid over the views.
    pub fn set_stencil(&mut self, name: &str, image: &fileio::image_file::ReadImage) {
        let Some(rgba) = (image.width > 0 && image.height > 0)
            .then(|| image.rgba.clone())
            .filter(|p| p.len() == (image.width * image.height * 4) as usize)
        else {
            self.note = Some(tr("That picture couldn't be used.").to_string());
            return;
        };
        let stencil = paint::Stencil {
            width: image.width,
            height: image.height,
            rgba: Arc::new(rgba.clone()),
            centre: Vec2::ZERO,
            scale: 1.0,
        };
        self.brushing.stencil = Some(Arc::new(stencil));
        self.brushing.stencil_pending = Some((
            format!("stencil_{name}"),
            [image.width as usize, image.height as usize],
            rgba,
        ));
        self.attributes.stencil_name = Some(name.to_string());
        self.attributes.brush.brush.kind = BrushKind::Projection;
        self.attributes.brush.tool = PaintTool::Brush;
        self.note = Some(crate::trf!(
            "{} is laid over the views: paint to project it onto the model.",
            name
        ));
    }

    /// Takes the colour shown on the shape under `at` (points, in a view
    /// of `size` seen through `camera`) as the brush's. Whether there was
    /// one.
    fn pick_on_model(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) -> bool {
        let Some(hit) = self.brush_hit(drawing, camera, size, at) else {
            return false;
        };
        let triangle = hit.surface.triangles()[hit.triangle];
        let local = hit.model.inverse().transform_point3(hit.world);
        let uv = uv_at(&triangle, local);
        self.take_colour(hit.id, triangle.slot, uv)
    }

    /// Takes the colour shown at `uv` in the UV Editor as the brush's.
    fn pick_flat(&mut self, drawing: &SceneDrawing, uv: Vec2) -> bool {
        let Some((id, triangle, _)) = self.flat_hit(drawing, uv) else {
            return false;
        };
        self.take_colour(id, triangle.slot, uv)
    }

    /// Takes the colour object `id`'s material in `slot` shows at `uv`:
    /// its painted layers mixed over its own colour, averaged over 3 × 3
    /// texels (decision 0183).
    fn take_colour(&mut self, id: ObjectId, slot: u32, uv: Vec2) -> bool {
        let material = self.material_at(id, slot);
        let scene = self.document.scene();
        let list = scene.material_list(id);
        let base = list
            .get(slot as usize)
            .or(list.first())
            .map_or([0.5; 3], |m| m.color)
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round());
        let mut sum = [0.0f32; 3];
        let mut count = 0.0;
        if let Some(set) = scene.paint().set(&material) {
            let size = set.size as f32;
            let x = (uv.x * size).floor() as i64;
            let y = ((1.0 - uv.y) * size).floor() as i64;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (px, py) = (x + dx, y + dy);
                    if px < 0 || py < 0 || px >= set.size as i64 || py >= set.size as i64 {
                        continue;
                    }
                    let p = paint::composite::pixel_at(set, px as u32, py as u32);
                    let a = p[3] as f32 / 255.0;
                    for c in 0..3 {
                        sum[c] += p[c] as f32 * a + base[c] * (1.0 - a);
                    }
                    count += 1.0;
                }
            }
        }
        let colour: [u8; 3] = if count > 0.0 {
            sum.map(|v| (v / count).round().clamp(0.0, 255.0) as u8)
        } else {
            base.map(|v| v as u8)
        };
        self.attributes.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        self.note = Some(crate::trf!(
            "Picked {} from {}.",
            paint::palette::hex(colour),
            material
        ));
        true
    }

    /// Carries out what the colour picker asked of the project's palettes
    /// (decision 0183).
    pub(super) fn carry_out_palette_requests(&mut self, actions: &mut Vec<super::WorkbenchAction>) {
        for request in std::mem::take(&mut self.attributes.palette_requests) {
            match request {
                PaletteRequest::Set(palettes, label) => {
                    let command = scene::Command::SetPalettes(palettes);
                    if let Err(why) = self.document.execute_labelled(command, tr(label), None) {
                        self.note = Some(crate::trf!("Couldn't change the palettes: {}.", why));
                    }
                }
                PaletteRequest::Import => {
                    actions.push(super::WorkbenchAction::ImportPalette { path: None });
                }
                PaletteRequest::Export(index) => {
                    actions.push(super::WorkbenchAction::ExportPalette { index, path: None });
                }
            }
        }
    }

    /// The layer the brush paints on in `set` (of `material`): the one
    /// chosen in the Paint layers panel, or else the topmost paint layer.
    /// `None` when neither is there (one is made on painting).
    fn paint_target(&self, material: &str, set: &TextureSet) -> Option<paint::LayerPath> {
        match self.paint_layers.active(material, set) {
            Some(path) => Some(path),
            None => set.top_paint_layer(),
        }
    }

    /// The name of the material in slot `slot` of object `id` (the first
    /// for a slot past the list's end, as the export does).
    fn material_at(&self, id: ObjectId, slot: u32) -> String {
        let list = self.document.scene().material_list(id);
        list.get(slot as usize)
            .or(list.first())
            .map(|m| m.name.clone())
            .unwrap_or_default()
    }

    /// Object `id`'s shape as the brush sees it.
    pub(super) fn brush_surface(&mut self, id: ObjectId, mesh: &Arc<Mesh>) -> Arc<PaintSurface> {
        match self.brushing.surfaces.get(&id) {
            Some((made_from, surface)) if Arc::ptr_eq(made_from, mesh) => Arc::clone(surface),
            _ => {
                let surface = Arc::new(PaintSurface::of(mesh));
                self.brushing
                    .surfaces
                    .insert(id, (Arc::clone(mesh), Arc::clone(&surface)));
                surface
            }
        }
    }

    /// The nearest shape under the mouse at `at` (points, in a view of
    /// `size` seen through `camera`), with UVs to paint through.
    pub(super) fn brush_hit(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) -> Option<Hit> {
        let ray = camera.ray_through(at, size);
        let mut best: Option<(f32, Hit)> = None;
        for item in &drawing.items {
            if item.outline || item.cage.is_some() || item.drawn.face_count() == 0 {
                continue;
            }
            let inverse = item.transform.inverse();
            if !inverse.is_finite() {
                continue;
            }
            let surface = self.brush_surface(item.id, &item.drawn);
            if !surface.has_uvs() {
                continue;
            }
            let origin = inverse.transform_point3(ray.origin);
            let direction = inverse.transform_vector3(ray.direction);
            let Some((t, triangle)) = surface.hit(origin, direction) else {
                continue;
            };
            let world = item.transform.transform_point3(origin + direction * t);
            let distance = world.distance(ray.origin);
            if best.as_ref().is_none_or(|(d, _)| distance < *d) {
                best = Some((
                    distance,
                    Hit {
                        id: item.id,
                        world,
                        triangle,
                        surface,
                        model: item.transform,
                    },
                ));
            }
        }
        best.map(|(_, hit)| hit)
    }

    /// Ends the stroke under way, if any: its record joins the layer's
    /// strokes (decision 0179), in the stroke's own undo step. Returns
    /// the ended stroke.
    fn end_stroke(&mut self) -> Option<BrushSession> {
        let mut session = self.brushing.session.take()?;
        if !session.painted {
            return Some(session);
        }
        let record = Arc::new(session.record.clone());
        if let Some(layer) = session.set.layer_mut(&session.layer) {
            let mut strokes = session.strokes_before.clone();
            strokes.push(record);
            match (&mut layer.mask, session.on_mask) {
                (Some(mask), true) => mask.strokes = strokes,
                _ => layer.strokes = strokes,
            }
        }
        let command = scene::Command::SetTextureSet {
            material: session.material.clone(),
            set: Some(session.set.clone()),
        };
        if let Err(why) =
            self.document
                .execute_labelled(command, stroke_label(session.kind), Some(session.key))
        {
            self.note = Some(crate::trf!("Couldn't paint: {}.", why));
        }
        Some(session)
    }

    /// Starts a stroke at `at`, on the shape there.
    fn begin_stroke(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) {
        self.end_stroke();
        let Some(hit) = self.brush_hit(drawing, camera, size, at) else {
            self.note = Some(
                tr("Paint on a shape with UVs: press the left button over one, and drag.")
                    .to_string(),
            );
            return;
        };
        let slot = hit.surface.triangles()[hit.triangle].slot;
        let material = self.material_at(hit.id, slot);
        let slots = self.slots_of(hit.id, &material);
        let view = View {
            view_projection: camera.view_projection(size),
            size,
            eye: camera.position,
            forward: camera.forward(),
            flat: camera.is_flat(),
        };
        let depth = DepthMap::of(&hit.surface, hit.model, &view);
        let mirror = self.mirroring().then(|| {
            let axis = self.attributes.brush.mirror_axis;
            ModelMirror::new(axis, &hit.surface, hit.model, &view)
        });
        let start_uv = uv_at(
            &hit.surface.triangles()[hit.triangle],
            hit.model.inverse().transform_point3(hit.world),
        );
        let place = StrokePlace::Model {
            slots,
            surface: hit.surface,
            model: hit.model,
            view,
            depth,
            camera: *camera,
            size,
            mirror,
        };
        if self.start_session(material, place, at, start_uv) {
            if self.note.as_deref() == Some(tr(INSIDE_A_PIECE)) {
                self.note = None;
            }
            self.note_colour_used();
            self.stroke_to(at);
        }
    }

    /// Starts a stroke in the UV Editor at `pointer`, on the piece there.
    fn begin_flat_stroke(&mut self, drawing: &SceneDrawing, pointer: UvPointer) {
        self.end_stroke();
        let Some((id, triangle, units_per_uv)) = self.flat_hit(drawing, pointer.uv) else {
            self.note = Some(tr(INSIDE_A_PIECE).to_string());
            return;
        };
        let material = self.material_at(id, triangle.slot);
        let mut targets = Vec::new();
        for shown in crate::uv_editor::shown_objects(&self.document) {
            let Some(mesh) = drawn_mesh(drawing, shown) else {
                continue;
            };
            let slots = self.slots_of(shown, &material);
            if !slots.is_empty() {
                let surface = self.brush_surface(shown, &mesh);
                targets.push((surface, slots));
            }
        }
        let zoom = pointer.zoom.max(1e-6);
        let at = pointer.uv * zoom;
        let start_uv = pointer.uv;
        let place = StrokePlace::Flat {
            targets,
            zoom,
            units_per_uv,
            mirror: self.mirroring(),
        };
        if self.start_session(material, place, at, start_uv) {
            if self.note.as_deref() == Some(tr(INSIDE_A_PIECE)) {
                self.note = None;
            }
            self.note_colour_used();
            self.stroke_to(at);
        }
    }

    /// Whether a stroke begun now mirrors (decision 0187): mirror painting
    /// is on and the brush is one that mirrors (the pen's eraser end
    /// does).
    fn mirroring(&self) -> bool {
        let settings = &self.attributes.brush;
        settings.mirror_on
            && (settings.tool == PaintTool::Fill || mirroring::mirrors(settings.brush.kind))
    }

    /// The slots of object `id` holding `material`.
    pub(super) fn slots_of(&self, id: ObjectId, material: &str) -> Vec<u32> {
        let list = self.document.scene().material_list(id);
        (0..list.len().max(1) as u32)
            .filter(|s| self.material_at(id, *s) == material)
            .collect()
    }

    /// The shown object with a UV piece at `uv` in the UV Editor: its id,
    /// the triangle there and how many scene units a texture width covers
    /// there.
    fn flat_hit(
        &mut self,
        drawing: &SceneDrawing,
        uv: Vec2,
    ) -> Option<(ObjectId, paint::surface::PaintTriangle, f32)> {
        for id in crate::uv_editor::shown_objects(&self.document) {
            let Some(mesh) = drawn_mesh(drawing, id) else {
                continue;
            };
            let surface = self.brush_surface(id, &mesh);
            let Some(triangle) = paint::flat::triangle_at(&surface, uv) else {
                continue;
            };
            let model = drawn_transform(drawing, id);
            let t = surface.triangles()[triangle];
            let world = t.points.map(|p| model.transform_point3(p));
            let area = (world[1] - world[0]).cross(world[2] - world[0]).length();
            let flat = (t.uvs[1] - t.uvs[0]).perp_dot(t.uvs[2] - t.uvs[0]).abs();
            let units_per_uv = if flat > 1e-12 && area > 1e-12 {
                (area / flat).sqrt()
            } else {
                1.0
            };
            return Some((id, t, units_per_uv));
        }
        None
    }

    /// Opens a stroke on `material`'s texture set (made if missing, with a
    /// paint layer on top if it has none), painted at `place`, starting at
    /// `at`. False when there is nothing to paint on.
    /// The texel rows of the trim row at `uv` (where the texture
    /// repeats), on a `size`-pixel texture, when the brush stays in its
    /// row (decision 0202).
    fn trim_row_at(&self, uv: Vec2, size: f32) -> Option<std::ops::Range<u32>> {
        if !self.attributes.brush.trim_row || !uv.y.is_finite() {
            return None;
        }
        let v = uv.y.rem_euclid(1.0);
        let trim = self.document.scene().trim();
        let row = trim.rows.iter().find(|r| (r.bottom..=r.top).contains(&v))?;
        let top = ((1.0 - row.top) * size).floor().max(0.0) as u32;
        let bottom = ((1.0 - row.bottom) * size).ceil().max(0.0) as u32;
        (top < bottom).then_some(top..bottom)
    }

    /// Shows the channel asked for in the Paint tab (decision 0213) by
    /// choosing its paint preview, and tells the tab which one shows.
    fn show_channel_asked(&mut self) {
        use crate::attributes::channel_view::ChannelView;
        if let Some(asked) = self.attributes.channel_asked.take() {
            let now = self.viewport.options().paint_preview;
            let shows_one = ChannelView::of(now) != ChannelView::All
                || (now == crate::viewport::PaintPreview::Unlit
                    && self.attributes.channel_shown == ChannelView::Colour);
            match asked.preview() {
                Some(preview) => {
                    if !shows_one {
                        self.brushing.preview_before = Some(now);
                    }
                    self.viewport.options_mut().paint_preview = preview;
                }
                None => {
                    let back = self.brushing.preview_before.take().unwrap_or_default();
                    self.viewport.options_mut().paint_preview = back;
                }
            }
            self.attributes.channel_shown = asked;
        }
        let now = self.viewport.options().paint_preview;
        let shown = self.attributes.channel_shown;
        // Colour shows as Unlit; any other change (the Shading button)
        // means the whole painting again.
        if !(shown == ChannelView::Colour && now == crate::viewport::PaintPreview::Unlit) {
            self.attributes.channel_shown = ChannelView::of(now);
        }
    }

    /// Keeps the brush's colour in Used lately (decision 0213) when the
    /// stroke starting paints it.
    fn note_colour_used(&mut self) {
        let brush = &self.attributes.brush;
        if brush.tool == PaintTool::Fill || brush.brush.kind == BrushKind::Paint {
            let [r, g, b, _] = brush.brush.color;
            crate::colour_shelf::note_used(&mut self.used_colours, [r, g, b]);
        }
    }

    fn start_session(
        &mut self,
        material: String,
        place: StrokePlace,
        at: Vec2,
        start_uv: Vec2,
    ) -> bool {
        let scene = self.document.scene();
        let mut set = match scene.paint().set(&material) {
            Some(set) => (**set).clone(),
            None => match TextureSet::new(scene.texture().size) {
                Some(set) => set,
                None => return false,
            },
        };
        // The chosen layer's mask when it is chosen (decision 0182); the
        // chosen layer when it's a paint layer; else a new one, above the
        // chosen layer or on top (decision 0181).
        let target = self.paint_target(&material, &set);
        // Not a smart mask: it's made from its settings (decision 0195).
        let on_mask = self.paint_layers.editing_mask()
            && target.as_ref().is_some_and(|p| {
                set.layer(p)
                    .and_then(|l| l.mask.as_ref())
                    .is_some_and(|m| m.smart.is_none())
            });
        // Not a gradient layer: it's made from its settings (decision
        // 0194), so a stroke goes on a new layer above it.
        let paintable = |l: &Layer| on_mask || (l.image().is_some() && !l.is_made());
        let erases = self.pen().is_some_and(|p| p.eraser)
            || self.attributes.brush.brush.kind == BrushKind::Erase;
        let layer =
            match target.filter(|p| set.layer(p).is_some_and(paintable)) {
                Some(path) => path,
                // The eraser on a fill, gradient or other made layer: a new
                // empty layer would take the stroke and nothing would change.
                None if erases => {
                    self.note = Some(
                    tr("Nothing to erase here: the eraser takes paint off a paint layer. Choose \
                        one in Paint layers, or lower this layer's opacity or paint its mask.")
                    .to_string(),
                );
                    return false;
                }
                None => {
                    let above = self.paint_layers.active(&material, &set);
                    match set.add_paint_layer(tr("Paint"), above.as_deref()) {
                        Some(path) => path,
                        None => return false,
                    }
                }
            };
        self.paint_layers.show_material(&material);
        self.paint_layers.choose(&material, layer.clone(), on_mask);
        let Some(chosen) = set.layer(&layer) else {
            return false;
        };
        let (image, strokes_before) = match (&chosen.mask, on_mask) {
            (Some(mask), true) => (mask.image.clone(), mask.strokes.clone()),
            _ => match chosen.image() {
                Some(image) => (image.clone(), chosen.strokes.clone()),
                None => return false,
            },
        };
        let mut brush = self.attributes.brush.brush;
        let pen_erases = self.pen().is_some_and(|p| p.eraser);
        // The realism channels painted with the colour (decision 0207):
        // the brush's ticked ones as it paints, every one the layer has as
        // it erases; not on masks, nor by effect brushes.
        let erasing = pen_erases || brush.kind == BrushKind::Erase;
        let mut channels = Vec::new();
        if set.realism && !on_mask && matches!(brush.kind, BrushKind::Paint | BrushKind::Erase) {
            let wanted: Vec<(paint::Channel, [u8; 4])> = if erasing {
                paint::Channel::ALL
                    .into_iter()
                    .filter(|c| chosen.channels.get(*c).is_some())
                    .map(|c| (c, [0; 4]))
                    .collect()
            } else {
                self.attributes.brush.channel_values()
            };
            for (channel, value) in wanted {
                let image = match chosen.channels.get(channel) {
                    Some(paint::ChannelPart::Paint(image)) => image.clone(),
                    _ => match paint::Image::new(set.size) {
                        Some(image) => image,
                        None => continue,
                    },
                };
                let mut canvas = StrokeCanvas::new(image);
                canvas.keep_to_rows(self.trim_row_at(start_uv, set.size as f32));
                channels.push((channel, canvas, value));
            }
        }
        let mut canvas = StrokeCanvas::new(image);
        let size = canvas.image().size() as f32;
        let texel = Vec2::new(start_uv.x * size, (1.0 - start_uv.y) * size);
        // Kept to the trim row it starts in (decision 0202).
        canvas.keep_to_rows(self.trim_row_at(start_uv, size));
        // Where the effect brushes take colours from (decision 0186).
        match brush.kind {
            BrushKind::Projection => match &self.brushing.stencil {
                Some(stencil) => {
                    let view_size = match &place {
                        StrokePlace::Model { size, .. } => *size,
                        StrokePlace::Flat { .. } => Vec2::ZERO,
                    };
                    let settings = self.attributes.projection;
                    let mut placed = (**stencil).clone();
                    placed.centre = view_size * 0.5 + Vec2::from(settings.offset);
                    placed.scale = settings.scale.max(0.01);
                    canvas.set_source(paint::Source::Stencil(Arc::new(placed)));
                }
                None => {
                    self.note = Some(
                        tr(
                            "Choose a picture to project first: Attributes → Projection → \
                            Picture…",
                        )
                        .to_string(),
                    );
                    return false;
                }
            },
            BrushKind::Clone => {
                let Some(source) = self.brushing.clone_source else {
                    self.note =
                        Some(tr("Hold V and click where to copy from, then paint.").to_string());
                    return false;
                };
                let offset = *self.brushing.clone_offset.get_or_insert_with(|| {
                    let from = Vec2::new(source.x * size, (1.0 - source.y) * size);
                    let d = from - texel;
                    (d.x.round() as i32, d.y.round() as i32)
                });
                canvas.set_source(paint::Source::Offset(offset.0, offset.1));
            }
            BrushKind::Smudge => {
                let picked = canvas.average(texel.x, texel.y, SMUDGE_REACH);
                canvas.set_carried(picked);
            }
            _ => {}
        }
        let pen = self.pen();
        if pen.is_some_and(|p| p.eraser) {
            brush.kind = BrushKind::Erase;
        }
        // The colour painted with, shown as Before in the picker.
        self.attributes.colour.before = [brush.color[0], brush.color[1], brush.color[2]];
        let record = paint::StrokeRecord {
            kind: brush.kind,
            color: brush.color,
            hardness: brush.hardness,
            flow: brush.flow,
            through: brush.through,
            dabs: Vec::new(),
            fills: Vec::new(),
            tip: brush.tip,
            alignment: brush.alignment,
        };
        self.brushing.strokes += 1;
        self.brushing.session = Some(BrushSession {
            material,
            layer,
            set,
            canvas,
            place,
            last: at,
            since: None,
            step: None,
            key: STROKE_KEYS | self.brushing.strokes,
            record,
            strokes_before,
            painted: false,
            on_mask,
            filled: Default::default(),
            kind: brush.kind,
            pressure: pen.map(|p| p.pressure),
            lazy: at,
            angle: 0.0,
            channels,
        });
        true
    }

    /// Carries the stroke on toward `at`: the lazy brush follows it on
    /// its string when smoothing is on (decision 0184).
    fn continue_stroke(&mut self, at: Vec2) {
        let settings = self.attributes.brush;
        let reach = if settings.smoothing_on {
            settings.smoothing.clamp(0.0, 1.0) * crate::pen::SMOOTHING_REACH
        } else {
            0.0
        };
        let Some(session) = self.brushing.session.as_mut() else {
            return;
        };
        if settings.tool == PaintTool::Fill {
            return self.stroke_to(at);
        }
        if crate::pen::follow(&mut session.lazy, at, reach) {
            let to = session.lazy;
            self.stroke_to(to);
        }
    }

    /// Carries a UV Editor stroke on to `pointer`.
    fn continue_flat_stroke(&mut self, pointer: UvPointer) {
        if let Some(session) = &self.brushing.session
            && let StrokePlace::Flat { zoom, .. } = session.place
        {
            self.continue_stroke(pointer.uv * zoom);
        }
    }

    /// Gives the workbench the tablet pen's state this frame, from the app
    /// (Wintab, decision 0184): `None` when no pen is near the tablet.
    pub fn set_tablet_pen(&mut self, pen: Option<crate::PenState>) {
        self.brushing.tablet = pen;
        self.attributes.pen = self.pen();
    }

    /// The pen's state now: a tablet's, or a pen through the system's
    /// touch; `None` for a mouse (decision 0184).
    pub(super) fn pen(&self) -> Option<crate::PenState> {
        self.brushing
            .tablet
            .or(self.brushing.touch.map(|pressure| crate::PenState {
                pressure,
                tilt: None,
                eraser: false,
            }))
    }

    /// Dabs from the stroke's last place to `at`, then gives the layer to
    /// the document.
    fn stroke_to(&mut self, at: Vec2) {
        let settings = self.attributes.brush;
        let pressure = self.pen().map(|p| p.pressure);
        let Some(session) = self.brushing.session.as_mut() else {
            return;
        };
        let from = session.last;
        let step = match &session.place {
            // Flat, the brush's size is known up front (texture widths,
            // shown `zoom` points a texture width).
            StrokePlace::Flat {
                zoom, units_per_uv, ..
            } => {
                let size = session.canvas.image().size() as f32;
                let mut brush = settings.brush;
                brush.radius = settings.flat_radius(*zoom, *units_per_uv, Some(size)) * zoom;
                brush.step()
            }
            StrokePlace::Model { .. } => session.step.unwrap_or(settings.brush.step()),
        };
        let spots: Vec<(Vec2, Option<f32>)> =
            paint::stroke::dabs_along(from, at, step, &mut session.since)
                .into_iter()
                .map(|spot| {
                    // The pressure along the way, between the last and now.
                    let length = (at - from).length();
                    let t = if length > 1e-6 {
                        ((spot - from).length() / length).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let p = match (session.pressure, pressure) {
                        (Some(a), Some(b)) => Some(a + (b - a) * t),
                        (_, b) => b,
                    };
                    (spot, p)
                })
                .collect();
        session.last = at;
        session.pressure = pressure;
        // Which way the stroke goes, for tips that follow it.
        let way = at - from;
        if way.length() > 1e-3 {
            session.angle = way.y.atan2(way.x);
        }
        let painted = match (&session.place, settings.tool) {
            (_, PaintTool::Fill) => fill_at(session, &settings, at),
            (StrokePlace::Model { .. }, PaintTool::Brush) => model_dabs(session, &settings, &spots),
            (StrokePlace::Flat { .. }, PaintTool::Brush) => flat_dabs(session, &settings, &spots),
        };
        if !painted {
            return;
        }
        session.painted = true;
        if let Some(layer) = session.set.layer_mut(&session.layer) {
            // The stroke's record joins the layer's when the stroke ends
            // (`end_stroke`): copying the record, which grows with every
            // dab, on every move made long strokes slower and slower (a
            // review found it).
            match (&mut layer.mask, session.on_mask) {
                (Some(mask), true) => {
                    mask.image = session.canvas.image().clone();
                }
                _ => {
                    if let Some(image) = layer.image_mut() {
                        *image = session.canvas.image().clone();
                    }
                    for (channel, canvas, _) in &session.channels {
                        *layer.channels.get_mut(*channel) =
                            Some(paint::ChannelPart::Paint(canvas.image().clone()));
                    }
                }
            }
        }
        let label = stroke_label(session.kind);
        let command = scene::Command::SetTextureSet {
            material: session.material.clone(),
            set: Some(session.set.clone()),
        };
        let key = session.key;
        if let Err(why) = self.document.execute_labelled(command, label, Some(key)) {
            self.note = Some(crate::trf!("Couldn't paint: {}.", why));
            self.brushing.session = None;
        }
    }
}

/// Paints `spots` (screen points in the stroke's view) on the model.
/// Whether any dab landed.
fn model_dabs(
    session: &mut BrushSession,
    settings: &BrushSettings,
    spots: &[(Vec2, Option<f32>)],
) -> bool {
    let StrokePlace::Model {
        slots,
        surface,
        model,
        view,
        depth,
        camera,
        size,
        mirror,
    } = &session.place
    else {
        return false;
    };
    let inverse = model.inverse();
    let mut brush = settings.brush;
    brush.kind = session.kind;
    let mut painted = false;
    // The frame's dabs, painted together after (decision 0188); smudge
    // paints each at once, as it picks up what the last one laid.
    let together = session.kind != BrushKind::Smudge;
    let mut jobs = Vec::new();
    for &(spot, pressure) in spots {
        let ray = camera.ray_through(spot, *size);
        let origin = inverse.transform_point3(ray.origin);
        let direction = inverse.transform_vector3(ray.direction);
        let Some((t, triangle)) = surface.hit(origin, direction) else {
            continue;
        };
        let world = model.transform_point3(origin + direction * t);
        let per_point = camera.world_per_pixel(world, *size).max(1e-6);
        if session.kind == BrushKind::Smudge {
            let uv = uv_at(&surface.triangles()[triangle], origin + direction * t);
            pick_up(&mut session.canvas, uv);
        }
        let texels = paint::surface::texels_per_unit(
            &surface.triangles()[triangle],
            *model,
            session.canvas.image().size() as f32,
        );
        let radius =
            settings.screen_radius(per_point, texels) * settings.size_pressure.share(pressure);
        let strength = settings.flow_pressure.share(pressure);
        brush.radius = radius;
        session.step = Some(brush.step());
        let scale = model
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element()
            .max(1e-6);
        // The dab, then its reflection on the other side (decision 0187):
        // the reflected ray, seen by the reflected view on the same spot.
        let mut sides = vec![(origin + direction * t, view, depth)];
        if let Some(mirror) = mirror {
            let (o, d) = (
                mirror.reflect.transform_point3(origin),
                mirror.reflect.transform_vector3(direction),
            );
            if let Some((t, _)) = surface.hit(o, d) {
                sides.push((o + d * t, &mirror.view, &mirror.depth));
            }
        }
        for (local, view, depth) in sides {
            let world = model.transform_point3(local);
            let target = Target {
                surface,
                slots,
                model: *model,
                view,
                depth: Some(depth),
            };
            let job = paint::DabJob {
                target,
                brush,
                dab: Dab {
                    screen: spot,
                    world,
                    radius,
                    world_radius: radius * per_point,
                    strength,
                    angle: session.angle,
                    seed: session.record.dabs.len() as u32,
                },
            };
            if together {
                jobs.push(job);
            } else {
                paint::paint_dabs(&mut session.canvas, &[job]);
                paint_channels(&mut session.channels, &[job]);
            }
            session.record.dabs.push(paint::DabRecord {
                at: local,
                toward: inverse
                    .transform_vector3(view.toward_camera(world))
                    .normalize_or_zero(),
                radius: radius * per_point / scale,
                strength,
            });
        }
        painted = true;
    }
    paint::paint_dabs(&mut session.canvas, &jobs);
    paint_channels(&mut session.channels, &jobs);
    painted
}

/// Paints `spots` (places on the texture times the zoom) flat on the
/// texture. Whether any dab landed.
fn flat_dabs(
    session: &mut BrushSession,
    settings: &BrushSettings,
    spots: &[(Vec2, Option<f32>)],
) -> bool {
    let StrokePlace::Flat {
        targets,
        zoom,
        units_per_uv,
        mirror,
    } = &session.place
    else {
        return false;
    };
    // The brush's radius in texture widths.
    let size = session.canvas.image().size() as f32;
    let radius = settings.flat_radius(*zoom, *units_per_uv, Some(size));
    let mut brush = settings.brush;
    brush.kind = session.kind;
    for &(spot, pressure) in spots {
        let uv = spot / *zoom;
        let centre = Vec2::new(uv.x * size, (1.0 - uv.y) * size);
        if session.kind == BrushKind::Smudge {
            pick_up(&mut session.canvas, uv);
        }
        let radius = radius * settings.size_pressure.share(pressure);
        let strength = settings.flow_pressure.share(pressure);
        brush.radius = radius * size;
        // In texels, row 0 at the top: the other way up from the UVs.
        let angle = -session.angle;
        // The dab, then flipped left and right when mirroring (decision
        // 0187), its tip turned the mirrored way.
        let mut sides = vec![(uv, centre, angle)];
        if *mirror {
            let flipped = mirroring::flipped_uv(uv);
            let centre = Vec2::new(size - centre.x, centre.y);
            sides.push((flipped, centre, std::f32::consts::PI - angle));
        }
        for (uv, centre, angle) in sides {
            let seed = session.record.dabs.len() as u32;
            for (surface, slots) in targets {
                paint::flat::paint_flat_dab_turned(
                    &mut session.canvas,
                    surface,
                    slots,
                    &brush,
                    (centre, radius * size, strength),
                    (angle, seed),
                );
                // The realism channels too (decision 0207).
                for (_, canvas, value) in &mut session.channels {
                    let mut brush = brush;
                    brush.color = *value;
                    paint::flat::paint_flat_dab_turned(
                        canvas,
                        surface,
                        slots,
                        &brush,
                        (centre, radius * size, strength),
                        (angle, seed),
                    );
                }
            }
            session
                .record
                .dabs
                .push(paint::DabRecord::flat(uv, radius, strength));
        }
    }
    !spots.is_empty()
}

/// The polygon fill tool at `at` (screen points in the stroke's view, or
/// the place on the texture times the zoom): fills the polygon, UV piece
/// or material there, once per drag (decision 0182). Whether it filled
/// anything new.
fn fill_at(session: &mut BrushSession, settings: &BrushSettings, at: Vec2) -> bool {
    let mut brush = settings.brush;
    brush.hardness = 1.0;
    brush.flow = 1.0;
    // The shapes and the triangle under `at` on each.
    let mut hits: Vec<(usize, Arc<PaintSurface>, Vec<u32>, usize)> = Vec::new();
    match &session.place {
        StrokePlace::Model {
            slots,
            surface,
            model,
            camera,
            size,
            mirror,
            ..
        } => {
            let inverse = model.inverse();
            let ray = camera.ray_through(at, *size);
            let origin = inverse.transform_point3(ray.origin);
            let direction = inverse.transform_vector3(ray.direction);
            let mut rays = vec![(origin, direction)];
            // And the reflected ray, when mirroring (decision 0187).
            if let Some(mirror) = mirror {
                rays.push((
                    mirror.reflect.transform_point3(origin),
                    mirror.reflect.transform_vector3(direction),
                ));
            }
            for (origin, direction) in rays {
                if let Some((_, triangle)) = surface.hit(origin, direction) {
                    hits.push((0, Arc::clone(surface), slots.clone(), triangle));
                }
            }
        }
        StrokePlace::Flat {
            targets,
            zoom,
            mirror,
            ..
        } => {
            let uv = at / *zoom;
            let mut uvs = vec![uv];
            if *mirror {
                uvs.push(mirroring::flipped_uv(uv));
            }
            for uv in uvs {
                for (i, (surface, slots)) in targets.iter().enumerate() {
                    if let Some(triangle) = paint::flat::triangle_at(surface, uv)
                        && slots.contains(&surface.triangles()[triangle].slot)
                    {
                        hits.push((i, Arc::clone(surface), slots.clone(), triangle));
                    }
                }
            }
        }
    }
    let mut filled = false;
    for (shape, surface, slots, triangle) in hits {
        if session.filled.contains(&(shape, triangle)) {
            continue;
        }
        let area = paint::fill::fill_area(&surface, triangle, settings.fill_area(), &slots);
        paint::fill::fill_triangles(&mut session.canvas, &surface, &area, &brush);
        for (_, canvas, value) in &mut session.channels {
            let mut brush = brush;
            brush.color = *value;
            paint::fill::fill_triangles(canvas, &surface, &area, &brush);
        }
        session.filled.extend(area.iter().map(|t| (shape, *t)));
        session.record.fills.push(paint::FillRecord {
            face: surface.triangles()[triangle].face,
            by: settings.fill_area(),
        });
        filled = true;
    }
    filled
}

/// The shape drawn for object `id`, if it has polygons.
pub(super) fn drawn_mesh(drawing: &SceneDrawing, id: ObjectId) -> Option<Arc<Mesh>> {
    drawing
        .items
        .iter()
        .find(|i| i.id == id && !i.outline && i.cage.is_none() && i.drawn.face_count() > 0)
        .map(|i| Arc::clone(&i.drawn))
}

/// Where object `id` is drawn in the world.
fn drawn_transform(drawing: &SceneDrawing, id: ObjectId) -> Mat4 {
    drawing
        .items
        .iter()
        .find(|i| i.id == id && !i.outline && i.cage.is_none())
        .map_or(Mat4::IDENTITY, |i| i.transform)
}

/// A smudge picks up some of the colour under it at `uv`, carried on to
/// the next dabs (decision 0186).
fn pick_up(canvas: &mut StrokeCanvas, uv: Vec2) {
    let size = canvas.image().size() as f32;
    let under = canvas.average(uv.x * size, (1.0 - uv.y) * size, SMUDGE_REACH);
    let carried = canvas.carried();
    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = carried[c] as f32 + (under[c] as f32 - carried[c] as f32) * SMUDGE_PICKUP;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    canvas.set_carried(out);
}

/// The UV at `point` (in the shape's own space, on `triangle`'s plane).
fn uv_at(triangle: &paint::surface::PaintTriangle, point: Vec3) -> Vec2 {
    let [a, b, c] = triangle.points;
    let (v0, v1, v2) = (b - a, c - a, point - a);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return triangle.uvs[0];
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    triangle.uvs[0] * u + triangle.uvs[1] * v + triangle.uvs[2] * w
}

/// How wide one texture pixel is on `triangle` (placed by `model`) in a
/// texture `size` pixels across, in centimetres.
fn texel_width(triangle: &paint::surface::PaintTriangle, model: Mat4, size: u32) -> Option<f32> {
    let world = triangle.points.map(|p| model.transform_point3(p));
    let mut most = 0.0f32;
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let length = world[a].distance(world[b]);
        let texels = (triangle.uvs[a] - triangle.uvs[b]).length() * size as f32;
        if length > 1e-6 {
            most = most.max(texels / length);
        }
    }
    (most > 0.0).then(|| 1.0 / most)
}

/// The undo step's name for a stroke of `kind`.
fn stroke_label(kind: BrushKind) -> &'static str {
    match kind {
        BrushKind::Paint => tr("Paint stroke"),
        BrushKind::Erase => tr("Erase stroke"),
        BrushKind::Smudge => tr("Smudge stroke"),
        BrushKind::Blend => tr("Blend stroke"),
        BrushKind::Blur => tr("Blur stroke"),
        BrushKind::HueShift => tr("Hue shift stroke"),
        BrushKind::Projection => tr("Projection stroke"),
        BrushKind::Clone => tr("Clone stroke"),
    }
}

#[cfg(test)]
mod uv_at_tests {
    use super::*;

    #[test]
    fn the_uv_at_a_point_follows_the_triangle() {
        let surface = PaintSurface::of(&mesh::PrimitiveKind::Cube.standard().mesh());
        for t in surface.triangles() {
            let middle = (t.points[0] + t.points[1] + t.points[2]) / 3.0;
            let uv = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
            assert!(uv_at(t, middle).distance(uv) < 1e-4);
            assert!(uv_at(t, t.points[1]).distance(t.uvs[1]) < 1e-4);
        }
    }
}
