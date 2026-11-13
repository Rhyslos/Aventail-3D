//! The painting view (Phase 12 milestone 20, [decision 0282](../../../../docs/decisions/0282-painted-box.md)):
//! where a Painted box is painted with the painter's brushes. It opens
//! from the box (right-click → Paint on it, Paint on it in Attributes, or
//! a double-click) in place of the node canvas, and Tab goes back to the
//! graph.
//!
//! The view shows the box's texture flat, with a copy on every side
//! (three by three), so a stroke over an edge is seen coming back on the
//! other side: the paint wraps, and the texture stays seamless. Under the
//! paint it can show, faintly, what is plugged into the box or the
//! graph's result, to paint over.
//!
//! The view only turns the mouse into brush events (the painter's own
//! [`BrushCapture`]: a left drag paints, Shift + click a straight line,
//! Ctrl + right drag size and hardness, Ctrl + left drag flow); the
//! workbench paints them with the painter's brush and keys
//! (`workbench::box_painting`), and each stroke is one
//! `GraphEdit::SetPainting`, one undo step.

use super::MaterialEditor;
use super::shape_canvas::ShapeView;
use crate::text::tr;
use crate::theme;
use crate::viewport::{BrushCapture, BrushEvent};
use egui::{Color32, ColorImage, Context, Pos2, Rect, Sense, TextureHandle, TextureOptions, Ui};
use glam::Vec2;
use matgraph::boxes::painted::PaintedKind;
use matgraph::{BoxId, GraphEdit, PAINTING_SIZES, Painting, PlugRef, STANDARD_PAINTING_SIZE};
use scene::Document;
use std::sync::Arc;

/// The canvas panel's accessible name while the painting view is open.
pub(crate) const PAINT_VIEW_LABEL: &str = crate::text!("Painting view");
/// The box menu's entry and Attributes' button opening it.
pub(crate) const PAINT_ON_IT_LABEL: &str = crate::text!("Paint on it");
/// The button going back to the graph.
pub(crate) const BACK_LABEL: &str = crate::text!("Back to the graph (Tab)");
/// The button clearing the painting.
pub(crate) const CLEAR_LABEL: &str = crate::text!("Clear");

/// The bar's height.
const BAR: f32 = 34.0;
/// The largest underlay picture made, in pixels across.
const UNDERLAY_SIDE: u32 = 512;
/// How strongly the copies round the texture are dimmed.
const COPIES_DIM: u8 = 110;

/// What shows faintly under the paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Underlay {
    /// Nothing: the box's own plain colour.
    Nothing,
    /// What is plugged into the box's input of its kind.
    #[default]
    Input,
    /// The graph's result (Material out).
    Result,
}

impl Underlay {
    const ALL: [Self; 3] = [Self::Nothing, Self::Input, Self::Result];

    fn label(self) -> &'static str {
        match self {
            Self::Nothing => crate::text!("Nothing under"),
            Self::Input => crate::text!("Its input under"),
            Self::Result => crate::text!("The result under"),
        }
    }
}

/// Where the mouse is on the painting view, for a brush stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PaintPointer {
    /// The place on the texture, in texture widths: x right, y down, 0
    /// to 1 on the middle copy (past it on the others).
    pub(crate) at: Vec2,
    /// Screen points per texture width.
    pub(crate) zoom: f32,
}

/// The painting view open on one Painted box.
#[derive(Debug)]
pub(crate) struct PaintCanvas {
    /// The Painted box painted on, in the graph shown.
    pub(crate) node: BoxId,
    pub(crate) view: ShapeView,
    pub(crate) underlay: Underlay,
    /// How strongly the underlay shows, 0 to 1.
    pub(crate) underlay_strength: f32,
    /// The resolution a painting is started at when the box has none.
    pub(crate) new_size: u32,
    pub(crate) capture: BrushCapture,
    pub(crate) events: Vec<BrushEvent<PaintPointer>>,
    /// Where the mouse is over the view, last frame.
    pub(crate) hover: Option<PaintPointer>,
    /// The brush's radius on screen and its hardness, from the workbench.
    pub(crate) brush: Option<(f32, f32)>,
    /// The stroke under way as it is now (from the workbench), shown in
    /// place of the box's painting until it ends.
    pub(crate) live: Option<paint::Image>,
    /// Screen points per texture width, as last drawn.
    pub(crate) zoom: f32,
    /// Where the whole view and its painting area were drawn.
    pub(crate) rect: Option<Rect>,
    pub(crate) area: Option<Rect>,
    textures: PaintTextures,
}

impl PaintCanvas {
    pub(crate) fn new(node: BoxId) -> Self {
        Self {
            node,
            view: ShapeView::default(),
            underlay: Underlay::default(),
            underlay_strength: 0.35,
            new_size: STANDARD_PAINTING_SIZE,
            capture: BrushCapture::default(),
            events: Vec::new(),
            hover: None,
            brush: None,
            live: None,
            zoom: 1.0,
            rect: None,
            area: None,
            textures: PaintTextures::default(),
        }
    }
}

/// The view's textures: the painting (sent again tile by tile as strokes
/// change it) and the underlay.
#[derive(Default)]
struct PaintTextures {
    painting: Option<TextureHandle>,
    /// What the painting texture shows: its size, greys or colours, and
    /// each tile's memory (0 for none), to send only changed tiles.
    shown: (u32, bool, Vec<usize>),
    underlay: Option<(usize, TextureHandle)>,
}

impl std::fmt::Debug for PaintTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintTextures")
            .field("shown", &(self.shown.0, self.shown.1))
            .finish()
    }
}

/// One tile of `image` as egui pixels: as painted, or (for a mask or a
/// height) its greys by lightness.
fn tile_pixels(tile: Option<&paint::Tile>, grey: bool) -> Vec<Color32> {
    let n = (paint::TILE * paint::TILE) as usize;
    let Some(tile) = tile else {
        return vec![Color32::TRANSPARENT; n];
    };
    tile.chunks_exact(4)
        .map(|p| {
            if grey {
                let l = matgraph::painting::lightness([p[0], p[1], p[2]].map(|v| v as f32 / 255.0));
                let v = (l * 255.0).round() as u8;
                Color32::from_rgba_unmultiplied(v, v, v, p[3])
            } else {
                Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])
            }
        })
        .collect()
}

impl PaintTextures {
    /// The painting's texture, brought up to date with `image`.
    fn painting(&mut self, ctx: &Context, image: &paint::Image, grey: bool) -> egui::TextureId {
        let size = image.size();
        let across = image.tiles_across() as usize;
        let t = paint::TILE as usize;
        let ptrs: Vec<usize> = image
            .tiles()
            .iter()
            .map(|tile| tile.as_ref().map_or(0, |a| Arc::as_ptr(a) as usize))
            .collect();
        let whole = self.painting.is_none() || self.shown.0 != size || self.shown.1 != grey;
        if whole {
            let n = size as usize;
            let mut pixels = vec![Color32::TRANSPARENT; n * n];
            for (index, tile) in image.tiles().iter().enumerate() {
                let Some(tile) = tile else { continue };
                let (tx, ty) = (index % across, index / across);
                let px = tile_pixels(Some(tile), grey);
                for row in 0..t {
                    let to = (ty * t + row) * n + tx * t;
                    pixels[to..to + t].copy_from_slice(&px[row * t..(row + 1) * t]);
                }
            }
            let picture = ColorImage::new([n, n], pixels);
            match &mut self.painting {
                Some(handle) => handle.set(picture, TextureOptions::LINEAR),
                None => {
                    self.painting =
                        Some(ctx.load_texture("painted box", picture, TextureOptions::LINEAR));
                }
            }
        } else if let Some(handle) = &mut self.painting {
            for (index, (now, was)) in ptrs.iter().zip(&self.shown.2).enumerate() {
                if now != was {
                    let (tx, ty) = (index % across, index / across);
                    let picture = ColorImage::new([t, t], tile_pixels(image.tile(index), grey));
                    handle.set_partial([tx * t, ty * t], picture, TextureOptions::LINEAR);
                }
            }
        }
        self.shown = (size, grey, ptrs);
        self.painting
            .as_ref()
            .map_or(egui::TextureId::default(), |h| h.id())
    }
}

impl MaterialEditor {
    /// Opens the painting view on Painted box `node` of the graph shown.
    pub(crate) fn open_paint_canvas(&mut self, node: BoxId) {
        self.shape = None;
        self.paint = Some(PaintCanvas::new(node));
        self.canvas.search = None;
        self.canvas.note = None;
        self.canvas.menu = None;
        self.canvas.selected = [node].into();
    }

    /// Whether box `id` of the graph shown is painted on.
    pub(crate) fn is_painted(&self, document: &Document, id: BoxId) -> bool {
        self.current(document)
            .and_then(|g| g.get(id))
            .and_then(|b| self.registry_of(document).get(&b.kind).cloned())
            .is_some_and(|k| k.takes_painting())
    }

    /// The open view's box as it is in `document`: its kind and its
    /// painting; `None` when the box is gone.
    pub(crate) fn painted_box(
        &self,
        document: &Document,
    ) -> Option<(PaintedKind, Option<Arc<Painting>>)> {
        let node = self.paint.as_ref()?.node;
        let b = self.current(document)?.get(node)?;
        Some((PaintedKind::of(&b.settings), b.painting.clone()))
    }

    /// The image a stroke on the open view starts from: the box's
    /// painting, or an empty one at the chosen resolution.
    pub(crate) fn painting_image(&self, document: &Document) -> Option<paint::Image> {
        let (_, painting) = self.painted_box(document)?;
        match painting {
            Some(p) => Some(paint::Image::from_painting(&p)),
            None => paint::Image::new(self.paint.as_ref()?.new_size),
        }
    }

    /// Makes `painting` the open view's box's painting (`None` clears
    /// it): one undo step labelled `label`, or why not.
    pub(crate) fn set_painting(
        &mut self,
        document: &mut Document,
        painting: Option<Painting>,
        label: &str,
    ) -> Result<(), matgraph::Refusal> {
        let Some(node) = self.paint.as_ref().map(|p| p.node) else {
            return Ok(());
        };
        let edit = GraphEdit::SetPainting {
            node,
            painting: painting.map(Arc::new),
        };
        self.run(document, vec![edit], label, None, false)
    }

    /// Takes the painting view's keys while it is open: none of the node
    /// canvas's (the workbench takes the painter's keys over the view).
    /// True when the view is open.
    pub(crate) fn take_paint_keys(&mut self) -> bool {
        self.paint.is_some()
    }

    /// Whether the mouse is over the painting view's painting area.
    pub(crate) fn over_paint_area(&self, ctx: &Context) -> bool {
        self.paint
            .as_ref()
            .and_then(|p| p.area)
            .zip(ctx.pointer_hover_pos())
            .is_some_and(|(area, at)| area.contains(at))
    }

    /// Tab on the painting view: back to the graph, or why not yet.
    pub(crate) fn paint_step_back(&mut self, stroking: bool, note: &mut Option<String>) {
        if self.paint.is_none() {
            return;
        }
        if stroking {
            *note = Some(tr("Finish the stroke (or press Esc), then Tab.").into());
        } else {
            self.paint = None;
            *note = Some(tr("Back to the graph.").into());
        }
    }

    /// The Material graph panel while the painting view is open.
    pub(crate) fn paint_tab(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let full = ui.max_rect();
        ui.advance_cursor_after_rect(full);
        let Some((kind, painting)) = self.painted_box(document) else {
            // The box is gone (undone, removed): back to the graph.
            self.paint = None;
            return;
        };
        let bar = Rect::from_min_size(full.min, egui::vec2(full.width(), BAR));
        let area = Rect::from_min_max(egui::pos2(full.left(), bar.bottom()), full.max);
        let response = ui.interact(area, ui.id().with("paint_canvas"), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(PAINT_VIEW_LABEL))
        });
        let Some(p) = self.paint.as_mut() else { return };
        p.rect = Some(full);
        p.area = Some(area);
        // Middle drag pans; the wheel zooms round the pointer (as on the
        // shape canvas and in the UV Editor).
        if response.dragged_by(egui::PointerButton::Middle) {
            let d = response.drag_delta();
            p.view.pan -= Vec2::new(d.x, d.y) / p.view.scale(area);
        }
        if response.hovered() && !egui::Popup::is_any_open(ui.ctx()) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
            if scroll != 0.0
                && let Some(at) = response.hover_pos()
            {
                p.view.wheel(area, at, scroll);
            }
        }
        let (view, zoom) = (p.view, p.view.scale(area));
        p.zoom = zoom;
        let to_place = move |at: Pos2| PaintPointer {
            at: view.on_canvas(area, at),
            zoom,
        };
        p.hover = response.hover_pos().map(to_place);
        let mut events = std::mem::take(&mut p.events);
        p.capture.capture(ui, &response, to_place, &mut events);
        p.events = events;
        // The picture: the copies round the middle one, the plain colour
        // (or the underlay) under the paint.
        let painter = ui.painter_at(area);
        painter.rect_filled(area, 0.0, super::canvas::BACKGROUND);
        let square = Rect::from_two_pos(
            view.on_screen(area, Vec2::ZERO),
            view.on_screen(area, Vec2::ONE),
        );
        let side = square.width();
        let base = kind.standard().map(|v| (v * 255.0).round() as u8);
        let base = Color32::from_rgb(base[0], base[1], base[2]);
        let underlay = self.underlay_texture(ui.ctx(), document, kind);
        let Some(p) = self.paint.as_mut() else { return };
        let image = match (&p.live, &painting) {
            (Some(live), _) => Some(live.clone()),
            (None, Some(painting)) => Some(paint::Image::from_painting(painting)),
            (None, None) => None,
        };
        let texture = image
            .as_ref()
            .map(|image| p.textures.painting(ui.ctx(), image, kind.is_grey()));
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let faint = Color32::from_white_alpha((p.underlay_strength.clamp(0.0, 1.0) * 255.0) as u8);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let r = square.translate(egui::vec2(dx as f32 * side, dy as f32 * side));
                if !r.intersects(area) {
                    continue;
                }
                painter.rect_filled(r, 0.0, base);
                if let Some(t) = underlay {
                    painter.image(t, r, uv, faint);
                }
                if let Some(t) = texture {
                    painter.image(t, r, uv, Color32::WHITE);
                }
                if dx != 0 || dy != 0 {
                    painter.rect_filled(r, 0.0, Color32::from_black_alpha(COPIES_DIM));
                }
            }
        }
        painter.rect_stroke(
            square,
            0.0,
            egui::Stroke::new(1.0, theme::UV_SQUARE_EDGE),
            egui::StrokeKind::Outside,
        );
        if let Some((radius, hardness)) = p.brush {
            p.capture
                .paint(ui, area, response.hover_pos(), radius, hardness);
        }
        self.paint_bar(ui, bar, kind, painting.as_deref(), document, note);
    }

    /// What shows under the paint, as a texture: what is plugged into the
    /// box's input of its kind, or the graph's result.
    fn underlay_texture(
        &mut self,
        ctx: &Context,
        document: &Document,
        kind: PaintedKind,
    ) -> Option<egui::TextureId> {
        let p = self.paint.as_ref()?;
        let graph = self.current(document)?.clone();
        let value = match p.underlay {
            Underlay::Nothing => None,
            Underlay::Input => {
                let wire = graph.wire_into(&PlugRef::new(p.node, kind.input()))?;
                let from = wire.from.clone();
                let registry = self.registry_of(document);
                let source = graph.get(from.node)?;
                let index = registry
                    .get(&source.kind)?
                    .info()
                    .outputs
                    .iter()
                    .position(|o| o.key == from.plug)?;
                let done = self.pictures.shown()?;
                let result = done.evaluation.outcome(from.node)?.as_ref().ok()?;
                result.outputs.get(index).cloned().flatten()
            }
            Underlay::Result => {
                let out = graph.first_of(matgraph::eval::MATERIAL_OUT)?;
                self.pictures.value(out).cloned()
            }
        }?;
        let key = super::pictures::picture_key(&value);
        let p = self.paint.as_mut()?;
        if p.textures.underlay.as_ref().is_none_or(|(k, _)| *k != key) {
            let image = super::pictures::colour_image(&value, UNDERLAY_SIDE);
            let handle = ctx.load_texture("painted box underlay", image, TextureOptions::LINEAR);
            p.textures.underlay = Some((key, handle));
        }
        p.textures.underlay.as_ref().map(|(_, h)| h.id())
    }

    /// The bar over the painting view: back to the graph, the box's
    /// kind, the resolution, what shows under the paint, and Clear.
    fn paint_bar(
        &mut self,
        ui: &mut Ui,
        bar: Rect,
        kind: PaintedKind,
        painting: Option<&Painting>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        ui.painter().rect_filled(bar, 0.0, theme::HEADER);
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar.shrink2(egui::vec2(8.0, 3.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        if ui.button(tr(BACK_LABEL)).clicked() {
            self.paint = None;
            *note = Some(tr("Back to the graph.").into());
            return;
        }
        ui.separator();
        for k in PaintedKind::ALL {
            if ui
                .selectable_label(k == kind, k.name())
                .on_hover_text(tr(
                    "What the box paints: its output of this kind gives the paint",
                ))
                .clicked()
                && k != kind
            {
                self.set_painted_kind(document, k, note);
            }
        }
        ui.separator();
        let size = painting.map_or_else(
            || self.paint.as_ref().map_or(0, |p| p.new_size),
            Painting::size,
        );
        egui::ComboBox::from_id_salt("painted box resolution")
            .selected_text(crate::trf!("{} pixels", size))
            .show_ui(&mut ui, |ui| {
                for s in PAINTING_SIZES {
                    if ui
                        .selectable_label(s == size, crate::trf!("{} pixels", s))
                        .clicked()
                        && s != size
                    {
                        self.set_paint_resolution(document, s, note);
                    }
                }
            })
            .response
            .on_hover_text(tr(
                "How many pixels across the paint is kept at. It is scaled to the graph's size \
                 when the graph is worked out; more pixels keep finer detail and take more memory",
            ));
        ui.separator();
        let Some(p) = self.paint.as_mut() else { return };
        egui::ComboBox::from_id_salt("painted box underlay")
            .selected_text(tr(p.underlay.label()))
            .show_ui(&mut ui, |ui| {
                for u in Underlay::ALL {
                    ui.selectable_value(&mut p.underlay, u, tr(u.label()));
                }
            })
            .response
            .on_hover_text(tr(
                "Shown faintly under the paint, to paint over: what is plugged into the box, or \
                 the graph's result. It is only shown here; the box itself paints over its input",
            ));
        if p.underlay != Underlay::Nothing {
            ui.add(
                egui::Slider::new(&mut p.underlay_strength, 0.0..=1.0)
                    .show_value(false)
                    .text(tr("Faint")),
            );
        }
        ui.separator();
        if ui
            .add_enabled(painting.is_some(), egui::Button::new(tr(CLEAR_LABEL)))
            .on_hover_text(tr("Take all the paint off this box (undo brings it back)"))
            .clicked()
        {
            let done = self.set_painting(document, None, tr("Clear a Painted box"));
            super::shape_canvas::report(done, note);
        }
    }

    /// Sets the open view's box's Kind to `kind`, as one undo step.
    pub(crate) fn set_painted_kind(
        &mut self,
        document: &mut Document,
        kind: PaintedKind,
        note: &mut Option<String>,
    ) {
        let Some(node) = self.paint.as_ref().map(|p| p.node) else {
            return;
        };
        let edit = GraphEdit::SetSetting {
            node,
            key: matgraph::boxes::painted::KIND.into(),
            value: matgraph::SettingValue::Choice(kind.index()),
        };
        let done = self.run(
            document,
            vec![edit],
            tr("Change what a Painted box paints"),
            None,
            false,
        );
        super::shape_canvas::report(done, note);
    }

    /// Keeps the open view's paint at `size` pixels across: the painting
    /// scaled (one undo step), or the size a new one starts at.
    pub(crate) fn set_paint_resolution(
        &mut self,
        document: &mut Document,
        size: u32,
        note: &mut Option<String>,
    ) {
        if let Some(p) = self.paint.as_mut() {
            p.new_size = size;
        }
        let Some((_, Some(painting))) = self.painted_box(document) else {
            return;
        };
        let Some(resized) = painting.resized(size) else {
            return;
        };
        let done = self.set_painting(
            document,
            Some(resized),
            tr("Change a painting's resolution"),
        );
        super::shape_canvas::report(done, note);
        if note.is_none() {
            *note = Some(crate::trf!(
                "The paint is kept at {} pixels across now. Undo puts it back as it was.",
                size
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grey_tile_shows_the_paint_by_lightness() {
        let mut tile = vec![0u8; paint::TILE_BYTES];
        tile[..4].copy_from_slice(&[255, 0, 0, 200]);
        let tile = Arc::new(tile);
        let grey = tile_pixels(Some(&tile), true);
        let [r, g, b, _] = grey[0].to_srgba_unmultiplied();
        assert!(r == g && g == b && r > 0 && r < 255);
        let colour = tile_pixels(Some(&tile), false);
        let [r, g, _, a] = colour[0].to_srgba_unmultiplied();
        assert!(r > 250 && g == 0 && a == 200);
        assert!(tile_pixels(None, true).iter().all(|c| c.a() == 0));
    }
}
