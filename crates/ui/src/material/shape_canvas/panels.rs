//! The shape canvas's panel: the bar over it (back to the graph, the
//! tools, Add / Cut, the mirror line, the reference image), the drawing
//! area, the pieces list beside it, the strip under it showing what the
//! box gives, and the basic shapes list and notes over it.

use super::paint::grey_image;
use super::{BACK_LABEL, SHAPE_CANVAS_LABEL, ShapeTool};
use crate::dialog::help;
use crate::material::MaterialEditor;
use crate::text::tr;
use crate::theme;
use egui::{Rect, RichText, Sense, Ui};
use glam::Vec2;
use matgraph::boxes::drawn_shape::{Look, drawn};
use matgraph::{SettingValue, Size};
use scene::Document;
use shapes::{Basic, Drawing, PieceId, PieceKind};
use std::sync::Arc;

/// The bar's height, the pieces list's width and the strip's height.
const BAR: f32 = 34.0;
const LIST: f32 = 250.0;
const STRIP: f32 = 132.0;
/// The strip's pictures are this many pixels across.
const STRIP_SIDE: u32 = 128;
/// How big a new basic shape is, from its middle (a share of the texture).
const BASIC_RADIUS: f32 = 0.2;

/// The tools on the bar, in order.
const TOOLS: [ShapeTool; 6] = [
    ShapeTool::Select,
    ShapeTool::Move,
    ShapeTool::Rotate,
    ShapeTool::Scale,
    ShapeTool::All,
    ShapeTool::Pen,
];

impl MaterialEditor {
    /// The Material graph panel while the shape canvas is open.
    pub(crate) fn shape_tab(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let full = ui.max_rect();
        ui.advance_cursor_after_rect(full);
        let Some(drawing) = self.shape_drawing(document) else {
            // The box is gone (undone, removed): back to the graph.
            self.shape = None;
            return;
        };
        let bar = Rect::from_min_size(full.min, egui::vec2(full.width(), BAR));
        let list = Rect::from_min_max(egui::pos2(full.right() - LIST, bar.bottom()), full.max);
        let strip = Rect::from_min_max(
            egui::pos2(full.left(), full.bottom() - STRIP),
            egui::pos2(list.left(), full.bottom()),
        );
        let area = Rect::from_min_max(
            egui::pos2(full.left(), bar.bottom()),
            egui::pos2(list.left(), strip.top()),
        );
        if let Some(s) = self.shape.as_mut() {
            s.rect = Some(full);
            s.area = Some(area);
            // Pieces gone (undone) can't stay chosen.
            s.selected.retain(|id| drawing.piece(*id).is_some());
            if s.editing.is_some_and(|id| drawing.piece(id).is_none()) {
                s.editing = None;
                s.point = None;
            }
        }
        let response = ui.interact(area, ui.id().with("shape_canvas"), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(SHAPE_CANVAS_LABEL))
        });
        self.navigate_shapes(ui, area, &response);
        self.use_shape_keys(ui.ctx(), document, note);
        let drawing = self.shape_drawing(document).unwrap_or(drawing);
        self.shape_pointer(ui, area, &drawing, document, note);
        let drawing = self.shape_drawing(document).unwrap_or(drawing);
        let painter = ui.painter_at(area);
        let pointer = ui.ctx().pointer_hover_pos();
        self.paint_shape_canvas(ui.ctx(), &painter, area, &drawing, pointer);
        if let Some(s) = self.shape.as_mut() {
            s.overlays.clear();
        }
        self.shape_note_overlay(ui, area, document, note);
        self.basic_list(ui, area, document, note);
        self.shape_bar(ui, bar, &drawing, document, note);
        self.pieces_list(ui, list, &drawing, document, note);
        self.what_it_gives(ui, strip, &drawing, document);
    }

    /// Middle drag pans; the wheel zooms round the pointer.
    fn navigate_shapes(&mut self, ui: &Ui, area: Rect, response: &egui::Response) {
        let Some(s) = self.shape.as_mut() else { return };
        if response.dragged_by(egui::PointerButton::Middle) {
            let d = response.drag_delta();
            s.view.pan -= Vec2::new(d.x, d.y) / s.view.scale(area);
        }
        if response.hovered() && !egui::Popup::is_any_open(ui.ctx()) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
            if scroll != 0.0
                && let Some(at) = response.hover_pos()
            {
                s.view.wheel(area, at, scroll);
            }
        }
    }

    /// The bar over the canvas.
    fn shape_bar(
        &mut self,
        ui: &mut Ui,
        bar: Rect,
        drawing: &Arc<Drawing>,
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
            self.shape = None;
            *note = Some(tr("Back to the graph.").into());
            return;
        }
        ui.separator();
        let Some(tool) = self.shape.as_ref().map(|s| s.tool) else {
            return;
        };
        for t in TOOLS {
            if ui.selectable_label(tool == t, tr(t.label())).clicked() {
                self.choose_tool(t);
            }
        }
        if ui
            .button(tr("Basic shapes (B)"))
            .on_hover_text(tr("A circle, square, polygon or star"))
            .clicked()
            && let Some(s) = self.shape.as_mut()
        {
            s.list = s.area.map(|a| a.center());
        }
        ui.separator();
        let cut = self.shape.as_ref().is_some_and(|s| s.cut);
        let add_hint = tr("New shapes add to the shape");
        let cut_hint = tr("New shapes cut out of those under them (or hold Alt while drawing)");
        if ui
            .selectable_label(!cut, tr("Add"))
            .on_hover_text(add_hint)
            .clicked()
            && let Some(s) = self.shape.as_mut()
        {
            s.cut = false;
        }
        if ui
            .selectable_label(cut, tr("Cut"))
            .on_hover_text(cut_hint)
            .clicked()
            && let Some(s) = self.shape.as_mut()
        {
            s.cut = true;
        }
        ui.separator();
        if ui
            .selectable_label(drawing.mirror().on, tr("Mirror line (M)"))
            .on_hover_text(tr(
                "Draw one half; the other is mirrored. Drag the line to move it.",
            ))
            .clicked()
        {
            self.toggle_mirror(document, note);
        }
    }

    /// The pieces list beside the canvas, and the chosen piece's settings.
    fn pieces_list(
        &mut self,
        ui: &mut Ui,
        list: Rect,
        drawing: &Arc<Drawing>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        ui.painter().rect_filled(list, 0.0, theme::PANEL);
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list.shrink(8.0))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        crate::attributes::section(&mut ui, tr("Pieces"));
        if drawing.is_empty() {
            help(
                &mut ui,
                tr("Nothing drawn yet. P draws with the pen; B adds a basic shape."),
            );
        }
        let modifiers = ui.input(|i| i.modifiers);
        egui::ScrollArea::vertical()
            .max_height(list.height() * 0.45)
            .show(&mut ui, |ui| {
                for piece in drawing.pieces().iter().rev() {
                    let chosen = self
                        .shape
                        .as_ref()
                        .is_some_and(|s| s.selected.contains(&piece.id));
                    let kind = tr(piece.kind.name());
                    let text = crate::trf!("{} · {}", piece.name, kind);
                    if ui.selectable_label(chosen, text).clicked()
                        && let Some(s) = self.shape.as_mut()
                    {
                        if modifiers.command {
                            s.selected.insert(piece.id);
                        } else if modifiers.shift {
                            s.selected.remove(&piece.id);
                        } else {
                            s.selected = [piece.id].into();
                        }
                    }
                }
            });
        let chosen: Vec<PieceId> = self
            .shape
            .as_ref()
            .map(|s| s.selected.iter().copied().collect())
            .unwrap_or_default();
        if let [one] = chosen.as_slice()
            && let Some(piece) = drawing.piece(*one)
        {
            ui.add_space(6.0);
            self.piece_rows(&mut ui, drawing, piece.clone(), document, note);
        }
        crate::attributes::section(&mut ui, tr("Reference image"));
        self.reference_controls(&mut ui, drawing, document, note);
        ui.add_space(8.0);
        help(
            &mut ui,
            tr(
                "Q select, W move, R rotate, S scale, T all; P pen, B basic shapes, M mirror \
                 line. Double-click a piece to edit its points. Tab goes back; Esc cancels.",
            ),
        );
    }

    /// The strip under the canvas: the box's three outputs, as it gives
    /// them with its settings now.
    fn what_it_gives(
        &mut self,
        ui: &mut Ui,
        strip: Rect,
        drawing: &Arc<Drawing>,
        document: &Document,
    ) {
        ui.painter().rect_filled(strip, 0.0, theme::HEADER);
        let look = self.shape_look(document);
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            (
                drawing.key(),
                look.puff.to_bits(),
                look.top.to_bits(),
                look.outline.to_bits(),
            )
                .hash(&mut h);
            h.finish()
        };
        let Some(s) = self.shape.as_mut() else { return };
        if s.textures.strip.as_ref().is_none_or(|t| t.0 != key) {
            let size = Size::nearest(STRIP_SIDE);
            let made = drawn(drawing, size, look, 2);
            let ctx = ui.ctx();
            let handles = [0, 1, 2].map(|i| {
                let image = grey_image(STRIP_SIDE, made[i].pixels());
                ctx.load_texture(
                    format!("shape_gives_{i}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
            s.textures.strip = Some((key, handles));
        }
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(strip.shrink(8.0))
                .layout(egui::Layout::left_to_right(egui::Align::Min)),
        );
        ui.vertical(|ui| {
            ui.label(RichText::new(tr("What the box gives")).color(theme::TEXT_STRONG));
            help(
                ui,
                tr("Sharp at any texture size; Puff and Outline are in Attributes."),
            );
        });
        if let Some((_, handles)) = &s.textures.strip {
            let side = (strip.height() - 36.0).max(32.0);
            for (handle, name) in handles
                .iter()
                .zip([tr("Mask"), tr("Height"), tr("Outline")])
            {
                ui.vertical(|ui| {
                    ui.add(egui::Image::new((handle.id(), egui::vec2(side, side))));
                    ui.label(RichText::new(name).color(theme::TEXT_WEAK).size(11.0));
                });
            }
        }
    }

    /// The open Shape box's settings in pixels at the strip's size.
    fn shape_look(&self, document: &Document) -> Look {
        let standard = Look {
            puff: 0.0,
            top: 1.0,
            outline: 1.0,
        };
        let Some((graph, node)) = self
            .current(document)
            .zip(self.shape.as_ref().map(|s| s.node))
        else {
            return standard;
        };
        let registry = self.registry_of(document);
        let Some(b) = graph.get(node) else {
            return standard;
        };
        let Some(info) = registry.get(&b.kind).map(|k| k.info()) else {
            return standard;
        };
        let values = b.settings.resolved(&info.settings);
        let px_per_cm = STRIP_SIDE as f32 / matgraph::real_size_cm(&registry, graph);
        let get = |key: &str| {
            info.settings
                .iter()
                .position(|s| s.key == key)
                .and_then(|i| match values.get(i) {
                    Some(SettingValue::Number(n)) => Some(*n),
                    _ => None,
                })
        };
        Look {
            puff: get("puff").map_or(0.0, |cm| cm * px_per_cm),
            top: get("height").unwrap_or(1.0),
            outline: get("outline").map_or(1.0, |cm| (cm * px_per_cm).max(0.5)),
        }
    }

    /// The note over the canvas, with its fixes as buttons.
    fn shape_note_overlay(
        &mut self,
        ui: &mut Ui,
        area: Rect,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(shown) = self.shape.as_ref().and_then(|s| s.note.clone()) else {
            return;
        };
        let place = Rect::from_min_size(
            area.left_top() + egui::vec2(16.0, 16.0),
            egui::vec2(380.0, 140.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(place)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let mut chosen = None;
        let frame = egui::Frame::new()
            .fill(theme::PANEL)
            .stroke(egui::Stroke::new(1.0, crate::material::look::REFUSED))
            .inner_margin(10.0)
            .corner_radius(4.0)
            .show(&mut child, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&shown.reason).color(theme::TEXT_STRONG)).wrap(),
                );
                ui.horizontal_wrapped(|ui| {
                    for fix in &shown.fixes {
                        if ui.button(tr(fix.describe())).clicked() {
                            chosen = Some(*fix);
                        }
                    }
                });
            });
        if let Some(s) = self.shape.as_mut() {
            s.overlays.push(frame.response.rect);
        }
        if let Some(fix) = chosen {
            self.carry_out_note_fix(document, fix, note);
        }
    }

    /// The basic shapes list (B), where it was opened.
    fn basic_list(
        &mut self,
        ui: &mut Ui,
        area: Rect,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some((at, sides)) = self.shape.as_ref().and_then(|s| Some((s.list?, s.sides))) else {
            return;
        };
        let place = Rect::from_min_size(at, egui::vec2(170.0, 190.0));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(place)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let mut sides_now = sides;
        let mut chosen = None;
        let frame = egui::Frame::new()
            .fill(theme::PANEL)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .inner_margin(8.0)
            .corner_radius(4.0)
            .show(&mut child, |ui| {
                ui.label(RichText::new(tr("Basic shapes")).color(theme::TEXT_STRONG));
                for shape in [
                    Basic::Circle,
                    Basic::Square,
                    Basic::Polygon(sides),
                    Basic::Star(sides),
                ] {
                    if ui.button(tr(shape.name())).clicked() {
                        chosen = Some(shape);
                    }
                }
                crate::dialog::row(ui, tr("Sides"), |ui| {
                    ui.add(egui::DragValue::new(&mut sides_now).range(shapes::basic::SIDES))
                        .on_hover_text(tr("Sides of a polygon, points of a star"))
                });
            });
        let alt = ui.input(|i| i.modifiers.alt);
        if let Some(s) = self.shape.as_mut() {
            s.overlays.push(frame.response.rect);
            s.sides = sides_now;
        }
        if let Some(shape) = chosen {
            let Some(s) = self.shape.as_mut() else { return };
            let view = s.view;
            let middle = view.on_canvas(area, at);
            let middle = if (0.0..=1.0).contains(&middle.x) && (0.0..=1.0).contains(&middle.y) {
                middle
            } else {
                Vec2::splat(0.5)
            };
            let kind = if s.cut != alt {
                PieceKind::Cut
            } else {
                PieceKind::Add
            };
            s.list = None;
            if matches!(s.tool, ShapeTool::Pen | ShapeTool::Select) {
                s.tool = ShapeTool::All;
            }
            self.add_piece(document, shape.path(middle, BASIC_RADIUS), kind, note);
        }
    }
}
