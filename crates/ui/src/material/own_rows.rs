//! Attributes for your own boxes (decision 0263): a placed own box's
//! ways to open it, open it up, keep it and share it; inside one, its
//! name, words, picture, plugs and sliders; and, for a box inside, which
//! of its settings are sliders and which plugs it gives or takes.

use super::MaterialEditor;
use crate::attributes::section;
use crate::dialog::{help, row};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use matgraph::{BoxId, BoxInfo, OwnBox, OwnSlider, PlugRef};
use scene::{Command, Document};
use std::sync::Arc;

/// The button that opens an own box.
pub(crate) const OPEN_IT: &str = crate::text!("Open it");
/// The button that opens one back up.
pub(crate) const OPEN_UP: &str = crate::text!("Open it back up");
/// The button keeping one in your library.
pub(crate) const SAVE_TO_LIBRARY: &str = crate::text!("Keep in your library");
/// The button writing a box file.
pub(crate) const EXPORT_BOX: &str = crate::text!("Export as a box file…");

impl MaterialEditor {
    /// Replaces own box `changed.id` with `changed`: one undo step, or
    /// merged into the last under `key` (typing a name).
    fn change_own(
        &mut self,
        document: &mut Document,
        changed: OwnBox,
        key: Option<u64>,
        note: &mut Option<String>,
    ) {
        let command = Command::SetOwnBox {
            id: changed.id.clone(),
            own: Some(changed),
        };
        if let Err(why) = document.execute_labelled(command, tr("Change your own box"), key) {
            *note = Some(why.to_string());
        }
    }

    /// A placed own box's rows: what it is, and opening, opening up,
    /// keeping and sharing it.
    pub(super) fn placed_rows(
        &mut self,
        ui: &mut Ui,
        id: BoxId,
        own: &Arc<OwnBox>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        section(ui, tr("Your own box"));
        let placed = document
            .scene()
            .graphs()
            .iter()
            .flat_map(|(_, g)| g.boxes().map(|(_, b)| b.kind.clone()).collect::<Vec<_>>())
            .filter(|k| *k == own.id)
            .count();
        help(
            ui,
            crate::trf!(
                "Made of {} boxes, placed {} times in this project. Every one is linked: \
                 change the boxes inside and they all change. Its sliders are each one's own.",
                own.inner.box_count(),
                placed
            ),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr(OPEN_IT))
                .on_hover_text(tr(
                    "See and change the boxes inside (double-click does the same)",
                ))
                .clicked()
            {
                self.open_inside(own.id.clone(), Some(id));
            }
            if ui
                .button(tr(OPEN_UP))
                .on_hover_text(tr(
                    "Put its boxes back in the graph in its place (Shift + G)",
                ))
                .clicked()
            {
                self.canvas.selected = [id].into();
                self.open_up_selected(document, note);
            }
        });
        self.keep_and_share(ui, &own.id, document, note);
    }

    /// Keep in your library, and export as a box file.
    fn keep_and_share(
        &mut self,
        ui: &mut Ui,
        own: &str,
        document: &Document,
        note: &mut Option<String>,
    ) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr(SAVE_TO_LIBRARY))
                .on_hover_text(tr("Keep it on the shelf for every project"))
                .clicked()
            {
                self.save_to_library(document, own, note);
            }
            if ui
                .button(tr(EXPORT_BOX))
                .on_hover_text(tr("Write it to a file to share or keep"))
                .clicked()
                && let Some(file) = self.box_file_of(document, own)
            {
                self.box_file_asked = Some(super::own_boxes::BoxFileAsk::Save(file));
            }
        });
    }

    /// Inside an own box with nothing selected: its name, words, picture,
    /// plugs and sliders.
    pub(super) fn own_rows(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(inside) = self.inside.last().cloned() else {
            return;
        };
        let Some(own) = document.scene().graphs().own(&inside.own).cloned() else {
            return;
        };
        section(ui, tr("Your own box"));
        help(
            ui,
            tr(
                "You are inside it: changes here change every box of it in the project. Tab or \
                Back out goes back.",
            ),
        );
        let mut changed = (*own).clone();
        let name = row(ui, tr("Name"), |ui| {
            ui.text_edit_singleline(&mut changed.name)
        });
        ui.label(RichText::new(tr(super::WHAT_IT_DOES)).color(theme::TEXT_WEAK));
        let about = ui.add(
            egui::TextEdit::multiline(&mut changed.about)
                .desired_rows(4)
                .desired_width(f32::INFINITY)
                .hint_text(tr(
                    "In your own words, for later and for whoever you share it with",
                )),
        );
        if (name.changed() || about.changed()) && !changed.name.trim().is_empty() {
            let key = self.own_key(&own.id, 1);
            self.change_own(document, changed, Some(key), note);
        }
        section(ui, tr("Picture"));
        ui.horizontal(|ui| {
            if let Some(texture) = self.own_texture(ui.ctx(), &own) {
                ui.image((texture, egui::vec2(48.0, 48.0)));
            }
            if ui
                .button(tr("Take its picture again"))
                .on_hover_text(tr("Use what its first output makes now, for the shelf"))
                .clicked()
            {
                self.take_picture(document, note);
            }
        });
        section(ui, tr("Plugs"));
        help(
            ui,
            tr("Select a box inside to choose what each plug feeds or comes from."),
        );
        let mut renamed = (*own).clone();
        let mut any = false;
        for (i, input) in renamed.inputs.iter_mut().enumerate() {
            ui.push_id(("own input", i), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("In")).color(theme::TEXT_WEAK));
                    any |= ui
                        .add(egui::TextEdit::singleline(&mut input.name).desired_width(140.0))
                        .changed();
                    let feeds = if input.to.is_empty() {
                        tr("feeds nothing").to_string()
                    } else {
                        crate::trf!("feeds {}", input.to.len())
                    };
                    ui.label(RichText::new(feeds).color(theme::TEXT_WEAK).small());
                });
            });
        }
        for (i, output) in renamed.outputs.iter_mut().enumerate() {
            ui.push_id(("own output", i), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Out")).color(theme::TEXT_WEAK));
                    any |= ui
                        .add(egui::TextEdit::singleline(&mut output.name).desired_width(140.0))
                        .changed();
                    if output.from.is_none() {
                        ui.label(
                            RichText::new(tr("nothing gives it"))
                                .color(super::look::REFUSED)
                                .small(),
                        );
                    }
                });
            });
        }
        section(ui, tr("Sliders"));
        if renamed.sliders.is_empty() {
            help(
                ui,
                tr("None yet: select a box inside and tick its settings."),
            );
        }
        let mut remove = None;
        for (i, slider) in renamed.sliders.iter_mut().enumerate() {
            ui.push_id(("own slider", i), |ui| {
                ui.horizontal(|ui| {
                    any |= ui
                        .add(egui::TextEdit::singleline(&mut slider.name).desired_width(160.0))
                        .changed();
                    if ui
                        .small_button("×")
                        .on_hover_text(tr("Take this slider off the box"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            renamed.sliders.remove(i);
            self.change_own(document, renamed, None, note);
        } else if any {
            let key = self.own_key(&own.id, 2);
            self.change_own(document, renamed, Some(key), note);
        }
        ui.add_space(6.0);
        self.keep_and_share(ui, &own.id, document, note);
    }

    /// A merge key for typing in field `what` of own box `id`, in this
    /// press's undo step.
    fn own_key(&self, id: &str, what: u64) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        (id, what, self.sessions).hash(&mut h);
        0x4d41_544f_0000_0000 | (h.finish() >> 32)
    }

    /// Inside an own box, box `id` selected: which of its settings are the
    /// own box's sliders, and the own box's plugs it could give or take.
    pub(super) fn inner_rows(
        &mut self,
        ui: &mut Ui,
        id: BoxId,
        info: &BoxInfo,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(inside) = self.inside.last().cloned() else {
            return;
        };
        let Some(own) = document.scene().graphs().own(&inside.own).cloned() else {
            return;
        };
        if !info.settings.is_empty() {
            section(ui, &crate::trf!("Sliders on {}", own.name));
            help(
                ui,
                tr("Ticked settings show on every box of it, so each can set them its own way."),
            );
            ui.horizontal_wrapped(|ui| {
                for spec in &info.settings {
                    let at = own
                        .sliders
                        .iter()
                        .position(|s| s.node == id && s.setting == spec.key);
                    let mut on = at.is_some();
                    let name = spec.name.show();
                    let switch = ui.checkbox(&mut on, &name);
                    switch.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Checkbox,
                            true,
                            on,
                            crate::trf!("{} {}", tr(super::own_boxes::SLIDER_SWITCH), name),
                        )
                    });
                    if switch.changed() {
                        let mut changed = (*own).clone();
                        match at {
                            Some(at) => {
                                changed.sliders.remove(at);
                            }
                            None => {
                                let n = (1..)
                                    .find(|n| {
                                        !changed.sliders.iter().any(|s| s.key == format!("s{n}"))
                                    })
                                    .unwrap_or(1);
                                changed.sliders.push(OwnSlider {
                                    key: format!("s{n}"),
                                    name: name.clone(),
                                    node: id,
                                    setting: spec.key.to_string(),
                                });
                            }
                        }
                        self.change_own(document, changed, None, note);
                    }
                }
            });
        }
        // The own box's plugs this box could give or take.
        let mut buttons: Vec<(String, OwnBox)> = Vec::new();
        for (i, output) in own.outputs.iter().enumerate() {
            let Some(plug) = info.outputs.iter().find(|p| p.kind == output.kind) else {
                continue;
            };
            let from = PlugRef::new(id, plug.key.to_string());
            if output.from.as_ref() == Some(&from) {
                continue;
            }
            let mut changed = (*own).clone();
            changed.outputs[i].from = Some(from);
            buttons.push((crate::trf!("Give {} from this box", output.name), changed));
        }
        let graph = own.inner.clone();
        for (i, input) in own.inputs.iter().enumerate() {
            let free = info.inputs.iter().find(|p| {
                let to = PlugRef::new(id, p.key.to_string());
                graph.wire_into(&to).is_none()
                    && !input.to.contains(&to)
                    && matgraph::kind::fit(input.kind, p.kind).is_ok()
            });
            let Some(plug) = free else { continue };
            let mut changed = (*own).clone();
            changed.inputs[i]
                .to
                .push(PlugRef::new(id, plug.key.to_string()));
            buttons.push((
                crate::trf!("Feed {} into its {}", input.name, plug.name.show()),
                changed,
            ));
        }
        if !buttons.is_empty() {
            section(ui, &crate::trf!("Plugs of {}", own.name));
            for (words, changed) in buttons {
                if ui.button(words).clicked() {
                    self.change_own(document, changed, None, note);
                }
            }
        }
    }

    /// Own box `own`'s picture as a texture, made when it changes.
    pub(crate) fn own_texture(
        &mut self,
        ctx: &egui::Context,
        own: &OwnBox,
    ) -> Option<egui::TextureId> {
        let picture = own.picture.as_ref()?;
        let key = {
            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut h = DefaultHasher::new();
            picture.hash(&mut h);
            h.finish()
        };
        if let Some((k, handle)) = self.own_textures.get(&own.id)
            && *k == key
        {
            return Some(handle.id());
        }
        let side = matgraph::own::PICTURE_SIDE as usize;
        let image = egui::ColorImage::from_rgb([side, side], picture);
        let handle = ctx.load_texture(
            format!("own box {}", own.id),
            image,
            egui::TextureOptions::LINEAR,
        );
        let id = handle.id();
        self.own_textures.insert(own.id.clone(), (key, handle));
        Some(id)
    }
}
