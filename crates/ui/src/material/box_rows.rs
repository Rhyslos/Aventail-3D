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
use matgraph::{
    BoxId, FrameId, Graph, GraphEdit, Registry, SettingKind, SettingSpec, SettingValue,
};
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

    /// A merge key for field `what` in this press's undo step.
    fn field_key(&self, what: u64) -> u64 {
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
        let registry = Registry::standard();
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
        help(ui, crate::trf!("A {} box", info.group.name()));
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
                if let Some(value) = setting_row(ui, spec, now)
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

/// One setting's control; the value it was changed to, if it was.
fn setting_row(ui: &mut Ui, spec: &SettingSpec, now: SettingValue) -> Option<SettingValue> {
    let name = spec.name.show();
    let about = spec.about.show();
    row(ui, &name, |ui| match (&spec.kind, now) {
        (SettingKind::Number { min, max, .. }, SettingValue::Number(mut n)) => {
            let r = ui
                .add(egui::Slider::new(&mut n, *min..=*max))
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
        _ => {
            help(ui, &about);
            None
        }
    })
}
