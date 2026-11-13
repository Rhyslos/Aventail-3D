//! Painting a Painted box in the Material editor (Phase 12 milestone 20,
//! [decision 0282](../../../../docs/decisions/0282-painted-box.md)), with
//! the painter's brushes.
//!
//! The painting view (`material::paint_canvas`) turns the mouse into
//! brush events; here they are painted with the brush in use (its size,
//! flow, hardness, spacing, tip, pen pressure, smoothing and mirror, the
//! same settings the Paint mode uses), flat on the box's texture, every
//! dab wrapping round the edges (`paint::flat::paint_wrapped_dab`) so the
//! texture stays seamless. The stroke shows in the view as it goes; when
//! it ends it becomes one `GraphEdit::SetPainting`: one undo step a
//! stroke.
//!
//! The painter's keys work over the view: 1 paint, 2 erase, 5 smudge, 7
//! blur, 8 the bucket (fills the whole painting), P picks a colour, L
//! mirror painting (left and right across the texture's middle, as in the
//! UV Editor), D smoothing, [ and ] the size, Shift + click a straight
//! line, Ctrl + drags the size, hardness and flow. Esc only cancels a
//! stroke under way. Projection (3), polygon fill (4) and clone (6) need
//! a model's surface or a source on it; on a painting they say what to
//! use instead.

use super::Workbench;
use super::brushing::{pick_up, stroke_label};
use crate::attributes::PaintTool;
use crate::material::paint_canvas::PaintPointer;
use crate::text::tr;
use crate::viewport::BrushEvent;
use egui::{Key, Modifiers, Ui};
use glam::Vec2;
use paint::{BrushKind, StrokeCanvas};

/// A stroke under way on a Painted box.
#[derive(Debug)]
pub(crate) struct BoxStroke {
    /// The box painted on.
    node: matgraph::BoxId,
    canvas: StrokeCanvas,
    /// Paint, erase or an effect: the brush's, or erasing with a pen's
    /// eraser end.
    kind: BrushKind,
    /// The mouse's last place: on the texture times the zoom (points).
    last: Vec2,
    since: Option<f32>,
    /// Where the lazy brush is, on its string (decision 0184).
    lazy: Vec2,
    /// Which way the stroke goes, in radians.
    angle: f32,
    /// The pen's pressure at `last` (none for a mouse).
    pressure: Option<f32>,
    /// Screen points per texture width when the stroke began.
    zoom: f32,
    /// Whether each dab is painted again flipped left and right.
    mirror: bool,
    /// Whether anything has been painted yet.
    painted: bool,
    /// Dabs painted so far: each dab's number, for tips that vary.
    dabs: u32,
}

impl BoxStroke {
    /// The stroke as it is now, for the view to show.
    pub(crate) fn image(&self) -> &paint::Image {
        self.canvas.image()
    }
}

/// The note when a tool needs a model's surface.
const NOT_ON_A_PAINTING: &str = crate::text!(
    "That tool works on a model. On a painting use the brush (1), the eraser (2), smudge (5), \
     blur (7) or the bucket (8), which fills the whole painting."
);

impl Workbench {
    /// The real-world size of the graph shown, in centimetres: how big a
    /// texture width is, for a brush sized on the surface.
    fn painted_units(&self) -> f32 {
        let registry = self.document.scene().graphs().registry().clone();
        self.material
            .current(&self.document)
            .map_or(matgraph::STANDARD_REAL_SIZE_CM, |g| {
                matgraph::real_size_cm(&registry, g)
            })
    }

    /// The brush's radius on a painting `size` pixels across shown at
    /// `zoom` points a texture width, in texture widths.
    fn painted_radius(&self, zoom: f32, size: u32) -> f32 {
        self.attributes
            .brush
            .flat_radius(zoom, self.painted_units(), Some(size as f32))
    }

    /// Each frame before the panels: the brush settings show while the
    /// painting view is open (the floating bar, the Paint section in
    /// Attributes), and the view gets the brush's size and the stroke so
    /// far.
    pub(super) fn update_box_painting(&mut self) {
        let open = self.mode == crate::palette::Mode::Material && self.material.paint.is_some();
        if !open {
            if self.brushing.box_stroke.is_some() {
                self.end_box_stroke();
            }
            return;
        }
        self.attributes.painting = true;
        self.attributes.brush_bar_shown = true;
        self.attributes.realism = false;
        self.attributes.pen = self.pen();
        let size = self
            .material
            .painting_image(&self.document)
            .map_or(matgraph::STANDARD_PAINTING_SIZE, |i| i.size());
        let zoom = self.material.paint.as_ref().map_or(1.0, |p| p.zoom);
        let radius = if self.attributes.brush.tool != PaintTool::Brush {
            0.0
        } else {
            (self.painted_radius(zoom, size) * zoom).max(0.5)
        };
        let hardness = self.attributes.brush.brush.hardness;
        let live = self.brushing.box_stroke.as_ref().map(|s| s.image().clone());
        if let Some(p) = self.material.paint.as_mut() {
            p.brush = Some((radius, hardness));
            p.live = live;
        }
    }

    /// The painter's keys over the painting view (`paint_keys` sends them
    /// here while it is open).
    pub(super) fn box_paint_keys(&mut self, ui: &Ui) {
        if let Some(touch) = crate::pen::touch_pressure(ui.ctx()) {
            self.brushing.touch = touch;
        }
        if ui.ctx().memory(|m| m.focused().is_some()) || !self.material.over_paint_area(ui.ctx()) {
            return;
        }
        let pressed = |ui: &Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        let settings = &mut self.attributes.brush;
        for (key, kind) in [
            (Key::Num1, BrushKind::Paint),
            (Key::Num2, BrushKind::Erase),
            (Key::Num5, BrushKind::Smudge),
            (Key::Num7, BrushKind::Blur),
        ] {
            if pressed(ui, key) {
                settings.tool = PaintTool::Brush;
                settings.brush.kind = kind;
                self.attributes.picking = false;
            }
        }
        if pressed(ui, Key::Num8) {
            self.attributes.brush.choose_bucket();
            self.attributes.picking = false;
        }
        for key in [Key::Num3, Key::Num4, Key::Num6, Key::Num9] {
            if pressed(ui, key) {
                self.note = Some(tr(NOT_ON_A_PAINTING).to_string());
            }
        }
        // L mirrors across the texture's middle here, as in the UV Editor.
        if pressed(ui, Key::L) {
            let settings = &mut self.attributes.brush;
            settings.mirror_on = !settings.mirror_on;
            self.note = Some(if settings.mirror_on {
                tr(
                    "Mirror painting on: each dab is painted again flipped left and right across \
                    the texture's middle. L turns it off.",
                )
                .to_string()
            } else {
                tr("Mirror painting off. L turns it on.").to_string()
            });
        }
        self.feel_keys(ui);
        // Esc only cancels a stroke under way.
        if self.brushing.box_stroke.is_some() && pressed(ui, Key::Escape) {
            self.brushing.box_stroke = None;
            self.note = Some(tr("Stroke cancelled.").to_string());
        }
    }

    /// The painting view's brush events, after the panels are drawn.
    pub(super) fn carry_out_box_brush(&mut self) {
        let open = self.mode == crate::palette::Mode::Material && self.material.paint.is_some();
        if !open {
            return;
        }
        if std::mem::take(&mut self.brushing.pick_now) {
            let hover = self.material.paint.as_ref().and_then(|p| p.hover);
            match hover {
                Some(p) => self.pick_on_painting(p.at),
                None => {
                    self.note = Some(
                        tr("Point at the painting and press P to take its colour.").to_string(),
                    );
                }
            }
        }
        let events = self
            .material
            .paint
            .as_mut()
            .map(|p| std::mem::take(&mut p.events))
            .unwrap_or_default();
        for event in events {
            match event {
                BrushEvent::Begin(p) if self.attributes.picking => {
                    self.attributes.picking = false;
                    self.brushing.picked_press = true;
                    self.pick_on_painting(p.at);
                }
                BrushEvent::Begin(p) => self.begin_box_stroke(p, None),
                BrushEvent::Line(p) => {
                    let from = self.brushing.box_last_end;
                    self.begin_box_stroke(p, from);
                }
                BrushEvent::Move(_) if self.brushing.picked_press => {}
                BrushEvent::Move(p) => self.continue_box_stroke(p),
                BrushEvent::End => {
                    self.end_box_stroke();
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

    /// Takes the colour the box shows at `at` (on the texture) as the
    /// brush's.
    fn pick_on_painting(&mut self, at: Vec2) {
        let Some(node) = self.material.paint.as_ref().map(|p| p.node) else {
            return;
        };
        let Some(value) = self.material.pictures.value(node).cloned() else {
            self.note =
                Some(tr("The box's picture isn't ready yet: try again in a moment.").into());
            return;
        };
        let n = value.size() as f32;
        let (x, y) = ((at.x * n).floor() as i64, (at.y * n).floor() as i64);
        let colour =
            crate::material::pictures::colour_at(&value, x, y).map(crate::material::pictures::byte);
        self.attributes.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        self.note = Some(crate::trf!("Picked {}.", paint::palette::hex(colour)));
    }

    /// Starts a stroke at `p` (from `from`, a straight line, for Shift +
    /// click).
    fn begin_box_stroke(&mut self, p: PaintPointer, from: Option<Vec2>) {
        self.end_box_stroke();
        let Some(node) = self.material.paint.as_ref().map(|p| p.node) else {
            return;
        };
        let settings = self.attributes.brush;
        let mut kind = settings.brush.kind;
        if (settings.tool == PaintTool::Fill && !settings.bucket)
            || settings.tool == PaintTool::Seams
        {
            self.note = Some(tr(NOT_ON_A_PAINTING).to_string());
            return;
        }
        if settings.tool == PaintTool::Brush
            && matches!(kind, BrushKind::Projection | BrushKind::Clone)
        {
            self.note = Some(tr(NOT_ON_A_PAINTING).to_string());
            return;
        }
        if self.pen().is_some_and(|p| p.eraser) {
            kind = BrushKind::Erase;
        }
        let Some(image) = self.material.painting_image(&self.document) else {
            return;
        };
        let mut canvas = StrokeCanvas::new(image);
        let zoom = p.zoom.max(1e-6);
        let start = from.unwrap_or(p.at);
        if kind == BrushKind::Smudge {
            let size = canvas.image().size() as f32;
            let at = wrapped(start);
            let picked = canvas.average(at.x * size, at.y * size, 3.0);
            canvas.set_carried(picked);
        }
        self.note_colour_used();
        self.attributes.colour.before = {
            let [r, g, b, _] = settings.brush.color;
            [r, g, b]
        };
        self.brushing.box_stroke = Some(BoxStroke {
            node,
            canvas,
            kind,
            last: start * zoom,
            since: None,
            lazy: start * zoom,
            angle: 0.0,
            pressure: self.pen().map(|p| p.pressure),
            zoom,
            mirror: settings.mirror_on,
            painted: false,
            dabs: 0,
        });
        if settings.tool == PaintTool::Fill {
            self.fill_box_painting();
            return;
        }
        self.box_stroke_to(p.at * zoom);
    }

    /// The bucket: the whole painting covered with the brush's colour.
    fn fill_box_painting(&mut self) {
        let mut brush = self.attributes.brush.brush;
        brush.color = self.attributes.stroke_colour();
        brush.kind = BrushKind::Paint;
        brush.hardness = 1.0;
        let Some(stroke) = self.brushing.box_stroke.as_mut() else {
            return;
        };
        let size = stroke.canvas.image().size() as f32;
        brush.radius = size * 2.0;
        paint::flat::paint_wrapped_dab(
            &mut stroke.canvas,
            &brush,
            (Vec2::splat(size / 2.0), size * 2.0, 1.0),
            (0.0, 0),
        );
        stroke.painted = true;
    }

    /// Carries the stroke on to `p`, the lazy brush following on its
    /// string when smoothing is on.
    fn continue_box_stroke(&mut self, p: PaintPointer) {
        let settings = self.attributes.brush;
        if settings.tool == PaintTool::Fill {
            return;
        }
        let reach = if settings.smoothing_on {
            settings.smoothing.clamp(0.0, 1.0) * crate::pen::SMOOTHING_REACH
        } else {
            0.0
        };
        let Some(stroke) = self.brushing.box_stroke.as_mut() else {
            return;
        };
        if crate::pen::follow(&mut stroke.lazy, p.at * stroke.zoom, reach) {
            let to = stroke.lazy;
            self.box_stroke_to(to);
        }
    }

    /// Dabs from the stroke's last place to `at` (points: the place on
    /// the texture times the zoom), each wrapping round the edges.
    fn box_stroke_to(&mut self, at: Vec2) {
        let settings = self.attributes.brush;
        let colour = self.attributes.stroke_colour();
        let pressure = self.pen().map(|p| p.pressure);
        let Some((zoom, size)) = self
            .brushing
            .box_stroke
            .as_ref()
            .map(|s| (s.zoom, s.canvas.image().size()))
        else {
            return;
        };
        let radius = self.painted_radius(zoom, size);
        let Some(stroke) = self.brushing.box_stroke.as_mut() else {
            return;
        };
        let mut brush = settings.brush;
        brush.kind = stroke.kind;
        brush.color = colour;
        brush.radius = radius * zoom;
        let step = brush.step();
        let from = stroke.last;
        let spots = paint::stroke::dabs_along(from, at, step, &mut stroke.since);
        let length = (at - from).length();
        let way = at - from;
        if way.length() > 1e-3 {
            stroke.angle = way.y.atan2(way.x);
        }
        let n = size as f32;
        for spot in spots {
            let t = if length > 1e-6 {
                ((spot - from).length() / length).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let p = match (stroke.pressure, pressure) {
                (Some(a), Some(b)) => Some(a + (b - a) * t),
                (_, b) => b,
            };
            let place = spot / zoom;
            if stroke.kind == BrushKind::Smudge {
                let w = wrapped(place);
                pick_up(&mut stroke.canvas, Vec2::new(w.x, 1.0 - w.y));
            }
            let r = radius * n * settings.size_pressure.share(p);
            let strength = settings.flow_pressure.share(p);
            let mut sides = vec![(place * n, stroke.angle)];
            if stroke.mirror {
                let flipped = Vec2::new(n - place.x * n, place.y * n);
                sides.push((flipped, std::f32::consts::PI - stroke.angle));
            }
            for (centre, angle) in sides {
                paint::flat::paint_wrapped_dab(
                    &mut stroke.canvas,
                    &brush,
                    (centre, r, strength),
                    (angle, stroke.dabs),
                );
                stroke.dabs += 1;
            }
            stroke.painted = true;
        }
        stroke.last = at;
        stroke.pressure = pressure;
    }

    /// Ends the stroke under way: the painting it made becomes the box's,
    /// one undo step.
    pub(super) fn end_box_stroke(&mut self) {
        let Some(stroke) = self.brushing.box_stroke.take() else {
            return;
        };
        self.brushing.box_last_end = Some(stroke.last / stroke.zoom);
        if !stroke.painted {
            return;
        }
        let Some(painting) = stroke.canvas.image().to_painting() else {
            return;
        };
        // The box may be gone, or the view moved to another box.
        if self.material.paint.as_ref().map(|p| p.node) != Some(stroke.node) {
            return;
        }
        let label = stroke_label(stroke.kind);
        let painting = (!painting.is_empty()).then_some(painting);
        if let Err(why) = self
            .material
            .set_painting(&mut self.document, painting, label)
        {
            self.note = Some(why.reason);
        }
    }
}

/// `at` brought onto the middle texture (0 to 1 each way).
fn wrapped(at: Vec2) -> Vec2 {
    Vec2::new(at.x.rem_euclid(1.0), at.y.rem_euclid(1.0))
}
