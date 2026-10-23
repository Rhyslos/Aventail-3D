//! The Materials section of the Attributes panel
//! ([decision 0074](../../../../docs/decisions/0074-material-slots.md)): an
//! object's list of materials, each with its colour and name, how many
//! polygons use it, and an Assign button that gives the selected polygons
//! that material. Add and Remove last change the list.
//!
//! Like the rest of the panel it never changes the scene itself: it hands
//! back commands, and asks the workbench to assign (which needs the
//! selected polygons it knows about).

use super::{AttributesPanel, label, section};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui, vec2};
use scene::{Command, Document, Material, ObjectId, ObjectKind};

/// Width of a material's name field.
const NAME_WIDTH: f32 = 120.0;

/// Above this many polygons the panel doesn't count each material's
/// polygons every frame.
const COUNT_UP_TO: usize = 50_000;

/// Colours given to new materials in turn: clearly different from each
/// other and from the standard grey.
const NEW_COLOURS: [[f32; 3]; 8] = [
    [0.86, 0.42, 0.25],
    [0.30, 0.55, 0.85],
    [0.45, 0.72, 0.36],
    [0.90, 0.75, 0.30],
    [0.62, 0.42, 0.78],
    [0.30, 0.70, 0.70],
    [0.85, 0.45, 0.60],
    [0.55, 0.45, 0.35],
];

/// The Materials section's own state.
#[derive(Debug, Clone, Default)]
pub(crate) struct MaterialsState {
    /// Names being typed, for which object.
    drafts: Option<(ObjectId, Vec<String>)>,
    /// A message under the list (why Remove last didn't), for which object.
    note: Option<(ObjectId, String)>,
}

/// A name for a new material that the list doesn't have yet:
/// "Material 2", "Material 3" and so on.
pub(crate) fn new_name(list: &[Material]) -> String {
    (2..)
        .map(|n| crate::trf!("Material {}", n))
        .find(|name| !list.iter().any(|m| m.name == *name))
        .unwrap_or_default()
}

/// The colour the next new material gets in a list of `count`.
pub(crate) fn new_colour(count: usize) -> [f32; 3] {
    NEW_COLOURS[(count + NEW_COLOURS.len() - 1) % NEW_COLOURS.len()]
}

/// An sRGB colour (0 to 1) as bytes, and back.
fn to_bytes(c: [f32; 3]) -> [u8; 3] {
    c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn from_bytes(c: [u8; 3]) -> [f32; 3] {
    c.map(|x| f32::from(x) / 255.0)
}

impl AttributesPanel {
    /// The message under the Materials list (why Remove last didn't), if
    /// any.
    #[cfg(test)]
    pub(crate) fn material_note(&self) -> Option<&str> {
        self.materials.note.as_ref().map(|(_, n)| n.as_str())
    }

    /// The Materials section for object `id`.
    pub(super) fn materials_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(object) = doc.scene().object(id) else {
            return;
        };
        let instance = matches!(object.kind, ObjectKind::Instance(_));
        let list = doc.scene().material_list(id);
        section(ui, tr("Materials"));
        if instance {
            ui.label(
                RichText::new(tr(
                    "An instance uses its original's materials: change them on the original.",
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        }
        // Keep the names being typed for this object.
        let drafts_fit = self
            .materials
            .drafts
            .as_ref()
            .is_some_and(|(at, names)| *at == id && names.len() == list.len());
        if !drafts_fit {
            self.materials.drafts = Some((id, list.iter().map(|m| m.name.clone()).collect()));
        }
        let counts = doc.scene().evaluated(id).and_then(|shape| {
            (shape.mesh.face_count() <= COUNT_UP_TO).then(|| {
                let mut counts = vec![0usize; list.len()];
                for (f, _) in shape.mesh.faces() {
                    let slot = mesh::material::slot_of(&shape.mesh, f) as usize;
                    let slot = if slot < counts.len() { slot } else { 0 };
                    counts[slot] += 1;
                }
                counts
            })
        });
        let selected = self.polygons_selected.unwrap_or(0);
        let mut changed: Option<(Vec<Material>, bool)> = None;
        egui::Grid::new("attributes_materials")
            .num_columns(4)
            .spacing(vec2(8.0, 6.0))
            .show(ui, |ui| {
                for (slot, material) in list.iter().enumerate() {
                    // Its colour.
                    let mut bytes = to_bytes(material.color);
                    let swatch = ui.add_enabled_ui(!instance, |ui| {
                        egui::color_picker::color_edit_button_srgb(ui, &mut bytes)
                    });
                    let swatch = swatch.inner;
                    let name = material.name.clone();
                    swatch.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::ColorButton,
                            !instance,
                            crate::trf!("{} colour", name),
                        )
                    });
                    if swatch.clicked() {
                        // A new colour picked: its own undo step.
                        self.gesture += 1;
                    }
                    if swatch.changed() && from_bytes(bytes) != material.color {
                        let mut next = list.clone();
                        next[slot].color = from_bytes(bytes);
                        changed = Some((next, true));
                    }
                    // Its name.
                    if let Some((_, drafts)) = self.materials.drafts.as_mut()
                        && let Some(draft) = drafts.get_mut(slot)
                    {
                        let response = ui
                            .add_enabled_ui(!instance, |ui| {
                                ui.add_sized(
                                    [NAME_WIDTH, ui.spacing().interact_size.y],
                                    egui::TextEdit::singleline(draft),
                                )
                            })
                            .inner;
                        let label_name = material.name.clone();
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::TextEdit,
                                !instance,
                                crate::trf!("Material {} name ({})", slot + 1, label_name),
                            )
                        });
                        if response.lost_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::Escape))
                                || draft.trim().is_empty()
                            {
                                *draft = material.name.clone();
                            } else if draft.trim() != material.name {
                                let mut next = list.clone();
                                next[slot].name = draft.trim().to_string();
                                changed = Some((next, false));
                            }
                        } else if !response.has_focus() && *draft != material.name {
                            *draft = material.name.clone();
                        }
                    }
                    // How many polygons use it.
                    match counts.as_ref().and_then(|c| c.get(slot)) {
                        Some(n) => ui.label(
                            RichText::new(match n {
                                1 => tr("1 polygon").to_string(),
                                n => crate::trf!("{} polygons", n),
                            })
                            .small()
                            .color(theme::TEXT_WEAK),
                        ),
                        None => ui.label(""),
                    };
                    // Assign it to the selected polygons.
                    let assign = ui
                        .add_enabled(
                            !instance && selected > 0,
                            egui::Button::new(tr("Assign")).small(),
                        )
                        .on_hover_text(crate::trf!(
                            "Give the selected polygons the material {}",
                            material.name
                        ))
                        .on_disabled_hover_text(if instance {
                            tr("An instance uses its original's materials").to_string()
                        } else {
                            tr(
                                "In the Polygons mode (3), select the polygons to give this \
                             material, then click Assign",
                            )
                            .to_string()
                        });
                    let name = material.name.clone();
                    assign.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            !instance && selected > 0,
                            crate::trf!("Assign {}", name),
                        )
                    });
                    if assign.clicked() {
                        self.assign = Some(slot as u32);
                    }
                    ui.end_row();
                }
            });
        if !instance {
            ui.horizontal(|ui| {
                let full = list.len() >= mesh::material::MAX_SLOTS as usize;
                if ui
                    .add_enabled(!full, egui::Button::new(tr("Add material")))
                    .on_hover_text(tr("Add a material to the list, to assign to polygons"))
                    .on_disabled_hover_text(crate::trf!(
                        "An object can have at most {} materials",
                        mesh::material::MAX_SLOTS
                    ))
                    .clicked()
                {
                    let mut next = list.clone();
                    next.push(Material {
                        name: new_name(&list),
                        color: new_colour(list.len()),
                    });
                    changed = Some((next, false));
                    self.materials.note = None;
                }
                let can_remove = list.len() > 1;
                if ui
                    .add_enabled(can_remove, egui::Button::new(tr("Remove last")))
                    .on_hover_text(tr(
                        "Remove the last material in the list, if no polygon uses it",
                    ))
                    .on_disabled_hover_text(tr("The first material can't be removed"))
                    .clicked()
                {
                    let last = list.len() - 1;
                    let used = doc.scene().evaluated(id).map_or(0, |shape| {
                        mesh::material::faces_with_slot(&shape.mesh, last as u32).len()
                    });
                    if used > 0 {
                        self.materials.note = Some((
                            id,
                            crate::trf!(
                                "{} is used by {} polygon{}: assign {} another material \
                                 first (Select → Select by material finds {}).",
                                list[last].name,
                                used,
                                if used == 1 { "" } else { "s" },
                                if used == 1 { tr("it") } else { tr("them") },
                                if used == 1 { tr("it") } else { tr("them") }
                            ),
                        ));
                    } else {
                        changed = Some((list[..last].to_vec(), false));
                        self.materials.note = None;
                    }
                }
            });
        }
        if let Some((at, note)) = &self.materials.note
            && *at == id
        {
            ui.label(RichText::new(note).color(theme::WARNING));
        }
        if self.polygons_selected.is_none() && !instance {
            label(
                ui,
                tr("Polygons mode (3): select polygons, then Assign."),
                tr(
                    "Materials are given to polygons: switch to the Polygons mode, select the \
                 polygons, then click a material's Assign button",
                ),
            );
        }
        if let Some((materials, merging)) = changed {
            let command = Command::SetMaterials { id, materials };
            out.push(if merging {
                DocRequest::RunMerging(command, self.gesture)
            } else {
                DocRequest::Run(command)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_materials_get_unused_names_and_different_colours() {
        let list = vec![Material::standard()];
        assert_eq!(new_name(&list), "Material 2");
        let more = vec![
            Material::standard(),
            Material {
                name: "Material 2".into(),
                color: new_colour(1),
            },
        ];
        assert_eq!(new_name(&more), "Material 3");
        assert_ne!(new_colour(1), new_colour(2));
        assert_ne!(new_colour(1), Material::standard().color);
    }

    #[test]
    fn colours_survive_the_trip_through_bytes() {
        let c = [0.2, 0.4, 1.0];
        assert_eq!(to_bytes(from_bytes(to_bytes(c))), to_bytes(c));
        assert_eq!(to_bytes([-1.0, 2.0, 0.5]), [0, 255, 128]);
    }
}
