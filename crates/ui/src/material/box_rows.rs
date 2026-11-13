//! Attributes in the Material mode (decision 0258): the chosen box's
//! settings, its "What does this do?" and the boxes that usually come
//! next; a chosen comment frame's title, colour and note; or, with
//! nothing chosen, the graph's own name and seed.

use super::MaterialEditor;
use super::note::FixContext;
use crate::attributes::section;
use crate::dialog::{help, row};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use matgraph::{BoxId, FrameId, Graph, GraphEdit, SettingKind, SettingSpec, SettingValue};
use scene::Document;

/// Marks Attributes' merge keys apart from every other.
const FIELD_KEYS: u64 = 0x4d41_5441_0000_0000;

impl MaterialEditor {
    /// The Attributes panel in the Material mode.
    pub(crate) fn attributes(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        // Each press starts a new undo step; a drag or typing between
        // presses is one.
        if ui.input(|i| i.pointer.any_pressed()) {
            self.sessions += 1;
        }
        let Some(graph) = self.current(document).cloned() else {
            section(ui, tr("Material graph"));
            help(
                ui,
                tr("No material graph yet. New graph, above the canvas, makes one."),
            );
            return;
        };
        let selected: Vec<BoxId> = self
            .canvas
            .selected
            .iter()
            .copied()
            .filter(|id| graph.get(*id).is_some())
            .collect();
        match (selected.as_slice(), self.canvas.frame) {
            ([], Some(frame)) if graph.frame(frame).is_some() => {
                self.frame_rows(ui, &graph, frame, document, note);
            }
            ([], _) if !self.inside.is_empty() => self.own_rows(ui, document, note),
            ([], _) => self.graph_rows(ui, &graph, document, note),
            ([one], _) => self.box_rows(ui, &graph, *one, document, note),
            (many, _) => {
                section(ui, &crate::trf!("{} boxes selected", many.len()));
                help(
                    ui,
                    tr(
                        "Delete removes them; Backspace removes them and keeps the wires \
                        through; C puts a comment frame round them; Ctrl + C copies them.",
                    ),
                );
            }
        }
    }

    /// A Shape box's drawing: what is on it, and opening its canvas.
    fn drawn_rows(&mut self, ui: &mut Ui, b: &matgraph::GraphBox, id: BoxId) {
        ui.add_space(4.0);
        let pieces = b.drawing.as_ref().map_or(0, |d| d.pieces().len());
        help(
            ui,
            if pieces == 0 {
                tr("Nothing drawn yet.").to_string()
            } else {
                crate::trf!("{} pieces drawn.", pieces)
            },
        );
        let open = crate::dialog::main_button(ui, tr(super::shape_canvas::OPEN_LABEL))
            .on_hover_text(tr("Draw the shape (double-click the box does the same)"));
        if open.clicked() {
            self.open_shape_canvas(id);
        }
    }

    /// A Painted box's painting: how much is painted, and opening the
    /// painting view (decision 0282).
    fn painted_rows(&mut self, ui: &mut Ui, b: &matgraph::GraphBox, id: BoxId) {
        ui.add_space(4.0);
        help(
            ui,
            match &b.painting {
                None => tr("Nothing painted yet.").to_string(),
                Some(p) => crate::trf!(
                    "Painted at {} pixels across ({} of {} squares hold paint).",
                    p.size(),
                    p.painted_tiles(),
                    p.tiles().len()
                ),
            },
        );
        let open_now = self.paint.as_ref().is_some_and(|p| p.node == id);
        if !open_now {
            let open = crate::dialog::main_button(ui, tr(super::paint_canvas::PAINT_ON_IT_LABEL))
                .on_hover_text(tr(
                    "Paint on it with the painter's brushes (double-click the box, or its \
                     right-click menu, does the same)",
                ));
            if open.clicked() {
                self.open_paint_canvas(id);
            }
        }
    }

    /// A merge key for field `what` in this press's undo step.
    pub(super) fn field_key(&self, what: u64) -> u64 {
        FIELD_KEYS ^ (self.sessions << 24) ^ what
    }

    /// The graph's name and seed, and how to start.
    fn graph_rows(
        &mut self,
        ui: &mut Ui,
        graph: &Graph,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        section(ui, tr("Material graph"));
        let mut name = graph.name().to_string();
        let typed = row(ui, tr("Name"), |ui| ui.text_edit_singleline(&mut name));
        if typed.changed() && !name.trim().is_empty() {
            let key = self.field_key(1);
            if let Err(why) = self.run(
                document,
                vec![GraphEdit::Rename(name)],
                tr("Rename the graph"),
                Some(key),
                false,
            ) {
                *note = Some(why.reason);
            }
        }
        let mut seed = graph.seed();
        let changed = row(ui, tr("Seed"), |ui| {
            ui.add(egui::DragValue::new(&mut seed).range(0..=999_999))
                .on_hover_text(tr(
                    "Changes every random pattern in the graph at once, keeping its look",
                ))
        });
        if changed.changed() {
            let key = self.field_key(2);
            let edit = GraphEdit::SetSeed(seed);
            let label = edit.describe();
            let _ = self.run(document, vec![edit], &label, Some(key), false);
        }
        help(
            ui,
            crate::trf!(
                "{} boxes, {} wires, {} comment frames.",
                graph.box_count(),
                graph.wires().len(),
                graph.frames().count()
            ),
        );
        self.graph_motion_rows(ui, graph, document);
        self.variation_rows(ui, graph, document, note);
        ui.add_space(6.0);
        help(
            ui,
            tr(
                "Space or a right-click on the canvas adds a box. Drag from a box's dot to \
                another's to connect them; click a box to see its settings here.",
            ),
        );
    }

    /// A comment frame's title, colour and note.
    fn frame_rows(
        &mut self,
        ui: &mut Ui,
        graph: &Graph,
        id: FrameId,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(frame) = graph.frame(id) else { return };
        section(ui, tr("Comment frame"));
        let mut changed = frame.clone();
        let title = row(ui, tr("Title"), |ui| {
            ui.text_edit_singleline(&mut changed.title)
        });
        let colour = row(ui, tr("Colour"), |ui| {
            ui.color_edit_button_srgb(&mut changed.colour)
        });
        ui.label(RichText::new(tr("Note")).color(theme::TEXT_WEAK));
        let words = ui.add(
            egui::TextEdit::multiline(&mut changed.note)
                .desired_rows(5)
                .desired_width(f32::INFINITY)
                .hint_text(tr("What this part of the graph does, for later")),
        );
        if (title.changed() || colour.changed() || words.changed())
            && changed != *frame
            && !changed.title.trim().is_empty()
        {
            let key = self.field_key(id.0 << 4 | 3);
            let edit = GraphEdit::ChangeFrame { id, frame: changed };
            if let Err(why) = self.run(
                document,
                vec![edit],
                tr("Change a comment frame"),
                Some(key),
                false,
            ) {
                *note = Some(why.reason);
            }
        }
        help(
            ui,
            tr(
                "Drag the title to move the frame with everything inside; drag its edges to \
                size it; double-click the title to rename it. Delete removes the frame, \
                not its boxes.",
            ),
        );
    }

    /// One box's name, what it does, why it can't run, its settings and
    /// the boxes that usually come next.
    fn box_rows(
        &mut self,
        ui: &mut Ui,
        graph: &Graph,
        id: BoxId,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let registry = self.registry_of(document);
        let registry = &*registry;
        let Some(b) = graph.get(id) else { return };
        let Some(info) = registry.get(&b.kind).map(|t| t.info()) else {
            section(ui, &b.kind);
            help(
                ui,
                tr(
                    "This box comes from a newer version of the app, which this one doesn't \
                    know. It is kept as it is, and doesn't run.",
                ),
            );
            return;
        };
        section(ui, &info.name.show());
        let own = registry.get(&b.kind).and_then(|k| k.own().cloned());
        let what = if own.is_some() {
            tr("One of your own boxes").to_string()
        } else {
            crate::trf!("A {} box", info.group.name())
        };
        // Its kind, and the Bypassed tick at the end of the same line.
        ui.horizontal(|ui| {
            help(ui, what);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.bypass_row(ui, graph, id, info, document, note);
            });
        });
        if b.bypassed {
            crate::dialog::help(
                ui,
                tr(
                    "Bypassed: what goes in passes straight through, and its settings do \
                    nothing until it is switched back on (the Bypassed tick above).",
                ),
            );
        }
        if registry.get(&b.kind).is_some_and(|k| k.takes_drawing()) {
            self.drawn_rows(ui, b, id);
        }
        if registry.get(&b.kind).is_some_and(|k| k.takes_painting()) {
            self.painted_rows(ui, b, id);
        }
        if registry.get(&b.kind).is_some_and(|k| k.takes_photo()) {
            self.photo_rows(ui, graph, b, id, document, note);
        }
        if b.kind == matgraph::boxes::seamless::SEAMLESS {
            ui.add_space(4.0);
            self.seam_row(ui, id, false);
        }
        if let Some(why) = self.problems(graph).remove(&id) {
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(&why.reason).color(super::look::REFUSED)).wrap());
            let rect = self.canvas.last_rect.unwrap_or(ui.max_rect());
            let context = FixContext::problem(graph, registry, id);
            for fix in &why.fixes {
                if ui.button(fix.describe(registry, graph)).clicked() {
                    self.carry_out_fix(document, fix, &context, rect, note);
                }
            }
        }
        section(ui, tr(super::WHAT_IT_DOES));
        help(ui, info.about.show());
        if !info.settings.is_empty() {
            section(ui, tr("Settings"));
            for (index, spec) in info.settings.iter().enumerate() {
                let now = b.settings.get(&spec.key).unwrap_or_else(|| spec.standard());
                if let Some(value) = setting_row(ui, spec, now.clone(), registry.palettes())
                    && value != now
                {
                    let key = self.field_key(id.0 << 8 | index as u64);
                    let edit = GraphEdit::SetSetting {
                        node: id,
                        key: spec.key.to_string(),
                        value,
                    };
                    let label = edit.describe();
                    if let Err(why) = self.run(document, vec![edit], &label, Some(key), false) {
                        *note = Some(why.reason);
                    }
                }
            }
        }
        if !info.settings.is_empty() && self.inside.is_empty() {
            self.shown_rows(ui, b, id, info, document, note);
            self.game_rows(ui, b, id, info, document, note);
        }
        if let Some(own) = &own {
            self.placed_rows(ui, id, own, document, note);
        }
        if !self.inside.is_empty() {
            self.inner_rows(ui, id, info, document, note);
        }
        if !info.comes_next.is_empty() {
            section(ui, tr(super::COMES_NEXT));
            ui.horizontal_wrapped(|ui| {
                for kind in &info.comes_next {
                    let Some(next) = registry.get(kind).map(|t| t.info()) else {
                        continue;
                    };
                    if ui
                        .button(next.name.show())
                        .on_hover_text(next.about.show())
                        .clicked()
                    {
                        self.add_next(document, id, kind, note);
                    }
                }
            });
        }
        ui.add_space(8.0);
        let looking = self.canvas.looking_at == Some(id);
        let words = if looking {
            tr("Back to the material")
        } else {
            tr("Look through it in the preview")
        };
        if ui.button(words).clicked() {
            self.canvas.looking_at = (!looking).then_some(id);
        }
    }
}

impl MaterialEditor {
    /// "Shown in the painter" (decision 0261): which of the box's
    /// settings every graph fill using the graph shows, to set its own
    /// way.
    fn shown_rows(
        &mut self,
        ui: &mut Ui,
        b: &matgraph::GraphBox,
        id: BoxId,
        info: &matgraph::BoxInfo,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        section(ui, tr(SHOWN_IN_PAINTER));
        help(
            ui,
            tr(
                "Ticked settings show on every paint layer using this graph, so each layer \
                can set them its own way (a different colour, a smaller pattern).",
            ),
        );
        ui.horizontal_wrapped(|ui| {
            for spec in &info.settings {
                let mut on = b.shown.contains(spec.key.as_ref());
                let name = spec.name.show();
                let switch = ui
                    .checkbox(&mut on, &name)
                    .on_hover_text(tr("Show this setting on paint layers using the graph"));
                // Told apart from the setting's own control of that name.
                switch.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        true,
                        on,
                        crate::trf!("{} {}", tr(SHOW_SWITCH), name),
                    )
                });
                if switch.changed() {
                    let edit = GraphEdit::ShowInPainter {
                        node: id,
                        key: spec.key.to_string(),
                        shown: on,
                    };
                    let label = edit.describe();
                    if let Err(why) = self.run(document, vec![edit], &label, None, false) {
                        *note = Some(why.reason);
                    }
                }
            }
        });
    }
}

/// The start of each such switch's accessible name ("Show in the
/// painter: Lightest").
pub(crate) const SHOW_SWITCH: &str = crate::text!("Show in the painter:");

/// The heading over the switches choosing the settings shown in the
/// painter.
pub(crate) const SHOWN_IN_PAINTER: &str = crate::text!("Shown in the painter");

/// One setting's control; the value it was changed to, if it was.
pub(crate) fn setting_row(
    ui: &mut Ui,
    spec: &SettingSpec,
    now: SettingValue,
    palettes: &matgraph::Palettes,
) -> Option<SettingValue> {
    let name = spec.name.show();
    let about = spec.about.show();
    row(ui, &name, |ui| match (&spec.kind, now) {
        (SettingKind::Number { min, max, .. }, SettingValue::Number(mut n)) => {
            // A wide range (a speed, seconds, a real-world size): a
            // slider that slows near its small end.
            let wide = *min >= 0.0 && *max >= 500.0;
            let r = ui
                .add(
                    egui::Slider::new(&mut n, *min..=*max)
                        .logarithmic(wide)
                        .smallest_positive(0.01),
                )
                .on_hover_text(&about);
            r.changed().then_some(SettingValue::Number(n))
        }
        (SettingKind::Whole { min, max, .. }, SettingValue::Whole(mut n)) => {
            let r = ui
                .add(egui::DragValue::new(&mut n).range(*min..=*max))
                .on_hover_text(&about);
            r.changed().then_some(SettingValue::Whole(n))
        }
        (SettingKind::Seed, SettingValue::Whole(mut n)) => {
            let r = ui
                .add(egui::DragValue::new(&mut n).range(0..=999_999))
                .on_hover_text(&about);
            r.changed().then_some(SettingValue::Whole(n))
        }
        (SettingKind::Colour { .. }, SettingValue::Colour(c)) => {
            let mut bytes = c.map(super::pictures::byte);
            let r = ui.color_edit_button_srgb(&mut bytes).on_hover_text(&about);
            r.changed()
                .then(|| SettingValue::Colour(bytes.map(|b| f32::from(b) / 255.0)))
        }
        (SettingKind::Choice { options, .. }, SettingValue::Choice(chosen)) => {
            let mut picked = chosen;
            let shown = options
                .get(chosen as usize)
                .map_or_else(String::new, |o| o.show());
            egui::ComboBox::from_id_salt(("material setting", spec.key.as_ref()))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (i, option) in options.iter().enumerate() {
                        ui.selectable_value(&mut picked, i as u32, option.show());
                    }
                })
                .response
                .on_hover_text(&about);
            (picked != chosen).then_some(SettingValue::Choice(picked))
        }
        (SettingKind::Switch { .. }, SettingValue::Switch(mut on)) => {
            let r = ui.checkbox(&mut on, "").on_hover_text(&about);
            r.changed().then_some(SettingValue::Switch(on))
        }
        (SettingKind::Length { min, max, .. }, SettingValue::Number(mut n)) => {
            // Lengths span small and large: a slider that slows near 0.
            let r = ui
                .add(
                    egui::Slider::new(&mut n, *min..=*max)
                        .logarithmic(true)
                        .smallest_positive(0.01)
                        .suffix(" cm"),
                )
                .on_hover_text(format!(
                    "{about} {}",
                    tr(
                        "The graph's real-world size, on Material out, says how many \
                        centimetres the texture covers."
                    )
                ));
            r.changed().then_some(SettingValue::Number(n))
        }
        (SettingKind::Gradient { .. }, SettingValue::Gradient(g)) => {
            gradient_editor(ui, spec.key.as_ref(), &about, g).map(SettingValue::Gradient)
        }
        (SettingKind::Palette { .. }, SettingValue::Palette(name)) => {
            super::palette_rows::palette_picker(ui, spec.key.as_ref(), &about, palettes, &name)
                .map(SettingValue::Palette)
        }
        _ => {
            help(ui, &about);
            None
        }
    })
}

/// A gradient's editor: the gradient drawn as a strip, then each colour
/// with where it sits (0 at the left, 1 at the right) and a button to
/// take it out, and a button to add one. The changed gradient, if it was.
fn gradient_editor(
    ui: &mut Ui,
    key: &str,
    about: &str,
    gradient: matgraph::ColourGradient,
) -> Option<matgraph::ColourGradient> {
    let mut changed = None;
    ui.vertical(|ui| {
        let width = ui.available_width().clamp(80.0, 260.0);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
        response.on_hover_text(about);
        let painter = ui.painter_at(rect);
        let bands = 64;
        for i in 0..bands {
            let t = (i as f32 + 0.5) / bands as f32;
            let c = gradient.colour_at(t).map(super::pictures::byte);
            let x0 = rect.left() + rect.width() * i as f32 / bands as f32;
            let x1 = rect.left() + rect.width() * (i + 1) as f32 / bands as f32;
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x0, rect.top()),
                    egui::pos2(x1 + 0.5, rect.bottom()),
                ),
                0.0,
                egui::Color32::from_rgb(c[0], c[1], c[2]),
            );
        }
        // Each colour's place, marked under the strip.
        for stop in gradient.stops() {
            let x = rect.left() + rect.width() * stop.at.clamp(0.0, 1.0);
            painter.line_segment(
                [
                    egui::pos2(x, rect.bottom() - 5.0),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(2.0, theme::TEXT),
            );
        }
        let count = gradient.stops().len();
        for (i, stop) in gradient.stops().iter().enumerate() {
            ui.horizontal(|ui| {
                ui.push_id((key, i), |ui| {
                    let mut bytes = stop.colour.map(super::pictures::byte);
                    if ui
                        .color_edit_button_srgb(&mut bytes)
                        .on_hover_text(tr("This colour"))
                        .changed()
                    {
                        let colour = bytes.map(|b| f32::from(b) / 255.0);
                        let next = matgraph::GradientStop { colour, ..*stop };
                        changed = Some(gradient.with_stop(i, next));
                    }
                    let mut at = stop.at;
                    if ui
                        .add(egui::Slider::new(&mut at, 0.0..=1.0).max_decimals(2))
                        .on_hover_text(tr("Where this colour sits: 0 is for black, 1 for white"))
                        .changed()
                    {
                        let next = matgraph::GradientStop { at, ..*stop };
                        changed = Some(gradient.with_stop(i, next));
                    }
                    if count > 2
                        && ui
                            .small_button("×")
                            .on_hover_text(tr("Take this colour out of the gradient"))
                            .clicked()
                    {
                        changed = gradient.with_stop_removed(i);
                    }
                });
            });
        }
        if let Some(more) = gradient.with_stop_added()
            && ui
                .button(tr("Add a colour"))
                .on_hover_text(tr("Adds a colour in the widest gap of the gradient"))
                .clicked()
        {
            changed = Some(more);
        }
    });
    changed
}
