//! The colour picker in the Attributes panel in Paint mode ([decision 0183](../../../../docs/decisions/0183-colour.md)),
//! after the one in Nodelit (the project lead's choice): a hue ring
//! around a saturation and value square, or sliders; the new colour
//! beside the one before; its lightness for a value check; picking from
//! the views (P); the project's palettes; and the colour as hex and RGB.

use super::{label, section};
use crate::text::tr;
use crate::theme;
use egui::{Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};
use paint::palette::{Palette, from_hex, from_hsv, hex, lightness, to_hsv};
use std::f32::consts::TAU;

/// Side of the wheel, in points (kept small: the project lead found the
/// mockup's too big).
const WHEEL: f32 = 128.0;

/// Width of the hue ring.
const RING: f32 = 14.0;

/// Width of the column beside the wheel (the swatches, lightness and
/// Pick).
const SIDE_COLUMN: f32 = 90.0;

/// Side of a palette swatch.
const SWATCH: f32 = 18.0;

/// Accessible name of the wheel, and the label tests look for.
pub(crate) const WHEEL_LABEL: &str = crate::text!("Colour wheel");

/// How the colour is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PickerMode {
    /// The hue ring and the saturation and value square.
    #[default]
    Wheel,
    /// Hue, saturation and value sliders.
    Sliders,
}

/// The picker's state, kept between frames.
#[derive(Debug, Clone)]
pub(crate) struct ColourPicker {
    pub(crate) mode: PickerMode,
    /// The colour as hue, saturation and value: kept so the hue stays put
    /// while the colour is grey or black.
    hsv: [f32; 3],
    /// The colour `hsv` was made from, to notice changes from elsewhere.
    seen: [u8; 3],
    /// The colour last painted with, shown beside the new one (set when
    /// a stroke begins).
    pub(crate) before: [u8; 3],
    /// What the ring or square drag is moving.
    dragging: Option<WheelPart>,
    /// The hex field's text while it is typed in.
    hex_text: Option<String>,
    /// The palette shown, by its place in the project's list.
    pub(crate) palette: usize,
}

impl Default for ColourPicker {
    fn default() -> Self {
        let colour = [227, 170, 102];
        Self {
            mode: PickerMode::Wheel,
            hsv: to_hsv(colour),
            seen: colour,
            before: colour,
            dragging: None,
            hex_text: None,
            palette: 0,
        }
    }
}

/// The part of the wheel a drag moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WheelPart {
    Ring,
    Square,
}

/// What the picker asks of the workbench.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PaletteRequest {
    /// Replace the project's palettes (one undo step), labelled.
    Set(Vec<Palette>, &'static str),
    /// Read a palette file into the project.
    Import,
    /// Write the shown palette to a file.
    Export(usize),
}

impl ColourPicker {
    /// Draws the picker for `colour`, changing it. `palettes` are the
    /// project's; changes to them go to `requests`. `picking` is whether
    /// the eyedropper is waiting for a click; the button toggles it.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        colour: &mut [u8; 3],
        palettes: &[Palette],
        picking: &mut bool,
        requests: &mut Vec<PaletteRequest>,
    ) {
        if *colour != self.seen {
            // Changed elsewhere (picked, a swatch, a key): follow it.
            self.set_from(*colour);
        }
        section(ui, tr("Colour"));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, PickerMode::Wheel, tr("Wheel"));
            ui.selectable_value(&mut self.mode, PickerMode::Sliders, tr("Sliders"));
        });
        let start = *colour;
        // Beside the wheel when the panel is wide enough, else under it.
        let side_by_side = ui.available_width() >= WHEEL + SIDE_COLUMN + 12.0;
        let mut body = |ui: &mut Ui| {
            match self.mode {
                PickerMode::Wheel => self.wheel(ui),
                PickerMode::Sliders => self.sliders(ui),
            }
            ui.vertical(|ui| {
                self.swatches(ui, colour);
                ui.label(
                    egui::RichText::new(crate::trf!(
                        "Lightness {:.0}%",
                        lightness(from_hsv(self.hsv)) * 100.0
                    ))
                    .small()
                    .color(theme::TEXT_WEAK),
                )
                .on_hover_text(tr(
                    "How light the colour looks, without its hue: a value check. Shapes read \
                     well when their lights and shades differ in lightness, not only in hue",
                ));
                if ui
                    .add(egui::Button::selectable(*picking, tr("Pick (P)")))
                    .on_hover_text(tr(
                        "Then click on the model or in the UV Editor to take the colour shown \
                         there. P picks under the mouse at once",
                    ))
                    .clicked()
                {
                    *picking = !*picking;
                }
            });
        };
        // The same ids either way, so a click lands while the panel's
        // width changes.
        ui.push_id("colour_picker_body", |ui| {
            if side_by_side {
                ui.horizontal(|ui| body(ui));
            } else {
                ui.vertical(|ui| body(ui));
            }
        });
        let made = from_hsv(self.hsv);
        if made != start && *colour == start {
            *colour = made;
        }
        self.numbers(ui, colour);
        self.palettes(ui, colour, palettes, requests);
        if from_hsv(self.hsv) != *colour {
            self.set_from(*colour);
        }
        self.seen = *colour;
    }

    /// Takes `colour` as the picker's, keeping the hue when it has none.
    fn set_from(&mut self, colour: [u8; 3]) {
        let [h, s, v] = to_hsv(colour);
        let keep_hue = s <= 0.0 || v <= 0.0;
        let keep_sat = v <= 0.0;
        self.hsv = [
            if keep_hue { self.hsv[0] } else { h },
            if keep_sat { self.hsv[1] } else { s },
            v,
        ];
        self.seen = colour;
        self.hex_text = None;
    }

    /// The hue ring around the saturation and value square.
    fn wheel(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(vec2(WHEEL, WHEEL), Sense::drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(WHEEL_LABEL))
        });
        let centre = rect.center();
        let outer = WHEEL / 2.0;
        let inner = outer - RING;
        let side = inner * std::f32::consts::SQRT_2 - 6.0;
        let square = Rect::from_center_size(centre, vec2(side, side));
        if let Some(at) = response.interact_pointer_pos() {
            if response.drag_started() || self.dragging.is_none() {
                let d = at.distance(centre);
                self.dragging = if d >= inner - 2.0 && d <= outer + 4.0 {
                    Some(WheelPart::Ring)
                } else if square.expand(4.0).contains(at) {
                    Some(WheelPart::Square)
                } else {
                    None
                };
            }
            match self.dragging {
                Some(WheelPart::Ring) => {
                    let v = at - centre;
                    self.hsv[0] = (v.y.atan2(v.x) / TAU).rem_euclid(1.0);
                }
                Some(WheelPart::Square) => {
                    let s = ((at.x - square.left()) / square.width()).clamp(0.0, 1.0);
                    let v = 1.0 - ((at.y - square.top()) / square.height()).clamp(0.0, 1.0);
                    self.hsv[1] = s;
                    self.hsv[2] = v;
                }
                None => {}
            }
        }
        if !response.dragged() && !response.drag_started() {
            self.dragging = None;
        }
        let painter = ui.painter_at(rect.expand(2.0));
        // The ring, in small quads round the circle.
        let mut ring = Mesh::default();
        let steps = 96;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let angle = t * TAU;
            let (sin, cos) = angle.sin_cos();
            let c = to_color(from_hsv([t, 1.0, 1.0]));
            ring.colored_vertex(centre + vec2(cos, sin) * inner, c);
            ring.colored_vertex(centre + vec2(cos, sin) * outer, c);
            if i > 0 {
                let n = ring.vertices.len() as u32;
                ring.add_triangle(n - 4, n - 3, n - 2);
                ring.add_triangle(n - 3, n - 1, n - 2);
            }
        }
        painter.add(Shape::mesh(ring));
        // The square: white to the hue across, to black down, in a grid
        // so the colours blend as they should.
        let hue = from_hsv([self.hsv[0], 1.0, 1.0]);
        let mut grid = Mesh::default();
        let cells = 8;
        for j in 0..=cells {
            for i in 0..=cells {
                let s = i as f32 / cells as f32;
                let v = 1.0 - j as f32 / cells as f32;
                let c = from_hsv([self.hsv[0], s, v]);
                grid.colored_vertex(
                    pos2(
                        square.left() + s * square.width(),
                        square.top() + (1.0 - v) * square.height(),
                    ),
                    to_color(c),
                );
            }
        }
        let row = cells + 1;
        for j in 0..cells {
            for i in 0..cells {
                let a = (j * row + i) as u32;
                grid.add_triangle(a, a + 1, a + row as u32);
                grid.add_triangle(a + 1, a + row as u32 + 1, a + row as u32);
            }
        }
        painter.add(Shape::mesh(grid));
        // The marks: on the ring at the hue, in the square at the colour.
        let (sin, cos) = (self.hsv[0] * TAU).sin_cos();
        let on_ring = centre + vec2(cos, sin) * (inner + RING / 2.0);
        mark(&painter, on_ring, 6.0, to_color(hue));
        let in_square = pos2(
            square.left() + self.hsv[1] * square.width(),
            square.top() + (1.0 - self.hsv[2]) * square.height(),
        );
        mark(&painter, in_square, 5.0, to_color(from_hsv(self.hsv)));
    }

    /// Hue, saturation and value sliders.
    fn sliders(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.set_width(WHEEL);
            for (i, name) in [tr("Hue"), tr("Saturation"), tr("Value")]
                .iter()
                .enumerate()
            {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(*name).small());
                    let mut percent = self.hsv[i] * if i == 0 { 360.0 } else { 100.0 };
                    let range = if i == 0 { 0.0..=360.0 } else { 0.0..=100.0 };
                    let response = ui.add(
                        egui::Slider::new(&mut percent, range)
                            .show_value(false)
                            .trailing_fill(true),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, *name)
                    });
                    if response.changed() {
                        self.hsv[i] = percent / if i == 0 { 360.0 } else { 100.0 };
                    }
                });
            }
        });
    }

    /// The new colour over the one before; clicking Before takes it back.
    fn swatches(&mut self, ui: &mut Ui, colour: &mut [u8; 3]) {
        let width = SIDE_COLUMN - 6.0;
        let (rect, _) = ui.allocate_exact_size(vec2(width, 40.0), Sense::hover());
        let painter = ui.painter_at(rect);
        let new = Rect::from_min_size(rect.min, vec2(width, 24.0));
        let old = Rect::from_min_size(rect.min + vec2(0.0, 24.0), vec2(width, 16.0));
        let now = from_hsv(self.hsv);
        for (area, c, text) in [(new, now, tr("New")), (old, self.before, tr("Before"))] {
            painter.rect_filled(area, 0.0, to_color(c));
            let ink = if lightness(c) > 0.5 {
                Color32::from_gray(30)
            } else {
                Color32::from_gray(225)
            };
            painter.text(
                area.left_center() + vec2(6.0, 0.0),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(11.0),
                ink,
            );
        }
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, theme::TEXT_WEAK),
            egui::StrokeKind::Inside,
        );
        let back = ui.interact(old, ui.id().with("colour_before"), Sense::click());
        back.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr("Colour before"))
        });
        if back
            .on_hover_text(tr("The colour last painted with: click to go back to it"))
            .clicked()
        {
            *colour = self.before;
        }
    }

    /// The colour as hex and as red, green and blue, with a copy button.
    fn numbers(&mut self, ui: &mut Ui, colour: &mut [u8; 3]) {
        ui.horizontal(|ui| {
            let mut text = self.hex_text.clone().unwrap_or_else(|| hex(*colour));
            let response = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(64.0)
                    .char_limit(7),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, tr("Colour as hex"))
            });
            if response.changed() {
                if let Some(c) = from_hex(&text) {
                    *colour = c;
                }
                self.hex_text = Some(text);
            }
            if response.lost_focus() {
                self.hex_text = None;
            }
            for (i, name) in ["R", "G", "B"].into_iter().enumerate() {
                ui.label(egui::RichText::new(name).small().color(theme::TEXT_WEAK));
                let mut value = colour[i];
                let response = ui.add(egui::DragValue::new(&mut value).range(0..=255));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::DragValue,
                        true,
                        crate::trf!("{} {}", tr("Colour"), name),
                    )
                });
                if response.changed() {
                    colour[i] = value;
                }
            }
            if ui
                .small_button(tr("Copy"))
                .on_hover_text(tr("Copy the colour as hex"))
                .clicked()
            {
                ui.ctx().copy_text(hex(*colour));
            }
        });
    }

    /// The project's palettes: one shown, its swatches (click to use, + to
    /// add the colour, right click to remove), and the palette menu.
    fn palettes(
        &mut self,
        ui: &mut Ui,
        colour: &mut [u8; 3],
        palettes: &[Palette],
        requests: &mut Vec<PaletteRequest>,
    ) {
        if self.palette >= palettes.len() {
            self.palette = 0;
        }
        ui.horizontal(|ui| {
            label(
                ui,
                tr("Palette"),
                tr(
                    "Colours kept with the project. Click a swatch to use it, + to add the \
                    colour, right click a swatch to remove it",
                ),
            );
            let shown = palettes
                .get(self.palette)
                .map_or_else(|| tr("None yet").to_string(), |p| p.name.clone());
            let palette_label = ui.label("");
            egui::ComboBox::from_id_salt("colour_palette")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (i, p) in palettes.iter().enumerate() {
                        ui.selectable_value(&mut self.palette, i, &p.name);
                    }
                    ui.separator();
                    if ui.button(tr("New palette")).clicked() {
                        let mut all = palettes.to_vec();
                        let name = free_palette_name(palettes);
                        all.push(Palette::new(name));
                        self.palette = all.len() - 1;
                        requests.push(PaletteRequest::Set(all, crate::text!("Add a palette")));
                    }
                    if ui.button(tr("Import palette…")).clicked() {
                        requests.push(PaletteRequest::Import);
                    }
                    if !palettes.is_empty() {
                        if ui.button(tr("Export palette…")).clicked() {
                            requests.push(PaletteRequest::Export(self.palette));
                        }
                        if ui.button(tr("Delete palette")).clicked() {
                            let mut all = palettes.to_vec();
                            all.remove(self.palette);
                            self.palette = 0;
                            requests
                                .push(PaletteRequest::Set(all, crate::text!("Delete a palette")));
                        }
                    }
                })
                .response
                .labelled_by(palette_label.id);
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
            if let Some(palette) = palettes.get(self.palette) {
                for (i, c) in palette.colours.iter().enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(SWATCH, SWATCH), Sense::click());
                    ui.painter().rect_filled(rect, 2.0, to_color(*c));
                    if *c == *colour {
                        ui.painter().rect_stroke(
                            rect,
                            2.0,
                            Stroke::new(2.0, Color32::WHITE),
                            egui::StrokeKind::Outside,
                        );
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, hex(*c))
                    });
                    let response = response.on_hover_text(hex(*c));
                    if response.clicked() {
                        *colour = *c;
                    }
                    if response.secondary_clicked() {
                        let mut all = palettes.to_vec();
                        all[self.palette].colours.remove(i);
                        requests.push(PaletteRequest::Set(
                            all,
                            crate::text!("Remove a colour from a palette"),
                        ));
                    }
                }
            }
            if ui
                .small_button("+")
                .on_hover_text(tr("Add the colour to the palette"))
                .clicked()
            {
                let mut all = palettes.to_vec();
                if all.is_empty() {
                    all.push(Palette::new(tr("Palette")));
                    self.palette = 0;
                }
                let palette = &mut all[self.palette];
                if palette.colours.len() < paint::palette::MAX_COLOURS {
                    palette.colours.push(*colour);
                    requests.push(PaletteRequest::Set(
                        all,
                        crate::text!("Add a colour to a palette"),
                    ));
                }
            }
        });
    }
}

/// A name for a new palette: "Palette", or "Palette 2"... if taken.
fn free_palette_name(palettes: &[Palette]) -> String {
    let base = tr("Palette");
    let taken = |n: &str| palettes.iter().any(|p| p.name == n);
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| base.to_string())
}

fn to_color(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A round mark at `at`, filled with `fill`, ringed white and black so it
/// shows on any colour.
fn mark(painter: &egui::Painter, at: Pos2, radius: f32, fill: Color32) {
    painter.circle_filled(at, radius, fill);
    painter.circle_stroke(at, radius, Stroke::new(2.0, Color32::WHITE));
    painter.circle_stroke(at, radius + 1.5, Stroke::new(1.0, Color32::BLACK));
}
