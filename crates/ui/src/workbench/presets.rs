//! Preset files for the workbench ([decision 0206](../../../../docs/decisions/0206-presets-and-starting-layouts.md)):
//! what saving one writes, and what loading one does. The app shows the
//! file windows and reads and writes the files.

use super::{Workbench, WorkbenchAction};
use crate::paint_layers::presets::{PresetAsk, PresetKind};
use crate::text::tr;
use fileio::preset::{BrushRecipe, Preset};
use std::path::Path;

impl Workbench {
    /// Turns what the Presets row or the brush bar asked for into
    /// actions for the app.
    pub(super) fn carry_out_preset_asks(&mut self, actions: &mut Vec<WorkbenchAction>) {
        match self.paint_layers.preset_asked.take() {
            Some(PresetAsk::Save(kind)) => {
                actions.push(WorkbenchAction::SavePreset { kind, path: None });
            }
            Some(PresetAsk::Load) => actions.push(WorkbenchAction::LoadPreset { path: None }),
            None => {}
        }
    }

    /// A preset file of `kind` as it stands now: a name for the file and
    /// its text. `None` when there is nothing to keep (no brushes of your
    /// own, no chosen layer, or no layers).
    pub fn preset_file(&self, kind: PresetKind) -> Option<(String, String)> {
        let preset = match kind {
            PresetKind::Brushes => {
                if self.saved_brushes.is_empty() {
                    return None;
                }
                Preset::Brushes(self.saved_brushes.iter().map(recipe_of).collect())
            }
            PresetKind::Style => {
                let material = self.paint_layers.material(&self.document)?;
                let set = self.document.scene().paint().set(&material)?;
                let path = self.paint_layers.active(&material, set)?;
                let layer = set.layer(&path)?.clone();
                Preset::Style {
                    name: layer.name.clone(),
                    layer: Box::new(layer),
                }
            }
            PresetKind::Material => {
                let material = self.paint_layers.material(&self.document)?;
                let set = self.document.scene().paint().set(&material)?;
                Preset::Material {
                    name: material.clone(),
                    colour: crate::paint_layers::presets::material_colour(
                        &self.document,
                        &material,
                    ),
                    layers: set.layers.clone(),
                }
            }
        };
        let name = match &preset {
            Preset::Style { name, .. } | Preset::Material { name, .. } => name.clone(),
            Preset::Brushes(_) => kind.fallback_name().to_string(),
        };
        Some((name, fileio::preset::write_preset(&preset)))
    }

    /// Says the preset was written to `path`, or why it wasn't.
    pub fn preset_saved(&mut self, path: &Path, result: Result<(), String>) {
        let file = super::files::file_name(path);
        self.note = Some(match result {
            Ok(()) => crate::trf!("Saved the preset {}.", file),
            Err(why) => crate::trf!("Couldn't save the preset {}: {}.", file, why),
        });
    }

    /// Uses the preset file read from `path`: brushes join your own, a
    /// style is added above the chosen layer of the shown material, and a
    /// material preset replaces the shown material's colour and layers
    /// (one undo step).
    pub fn load_preset(&mut self, path: &Path, bytes: &[u8]) {
        let size = self.document.scene().texture().size;
        let preset = match fileio::preset::read_preset(bytes, size) {
            Ok(preset) => preset,
            Err(why) => return self.import_failed(path, &why),
        };
        let file = super::files::file_name(path);
        match preset {
            Preset::Brushes(brushes) => {
                let mut added = 0;
                for brush in brushes {
                    let preset = brush_of(brush);
                    if self.saved_brushes.len() >= crate::brush_bar::MAX_SAVED {
                        break;
                    }
                    if preset.is_valid() && !self.saved_brushes.contains(&preset) {
                        self.saved_brushes.push(preset);
                        added += 1;
                    }
                }
                self.note = Some(crate::trf!("Added {} brushes from {}.", added, file));
            }
            Preset::Style { name, layer } => {
                let layer = *layer;
                let Some(material) = self.paint_layers.material(&self.document) else {
                    return self.import_failed(
                        path,
                        tr("select an object to add the style to its material"),
                    );
                };
                let material = self.own_for_change(&material);
                let Some(mut set) = self
                    .document
                    .scene()
                    .paint()
                    .set(&material)
                    .map(|s| (**s).clone())
                    .or_else(|| paint::TextureSet::new(size))
                else {
                    return self.import_failed(path, tr("the texture size is out of range"));
                };
                let active = self.paint_layers.active(&material, &set);
                set.realism |= paint::channel::any_channels(std::slice::from_ref(&layer));
                let Some(at) = set.insert_layer(layer, active.as_deref()) else {
                    return self
                        .import_failed(path, tr("the material has as many layers as it can"));
                };
                self.paint_layers.choose(&material, at, false);
                self.set_layers(&material, set, crate::trf!("Add the style {}", name));
            }
            Preset::Material { colour, layers, .. } => {
                let Some(material) = self.paint_layers.material(&self.document) else {
                    return self.import_failed(
                        path,
                        tr("select an object to load the preset onto its material"),
                    );
                };
                let material = self.own_for_change(&material);
                let key = self.new_step_key();
                let label = crate::trf!("Load the material {}", file);
                // The colour, on every object that has the material.
                let scene = self.document.scene();
                let changes: Vec<_> = scene
                    .walk()
                    .iter()
                    .filter_map(|(id, _)| {
                        let mut list = scene.material_list(*id);
                        let found = list.iter_mut().find(|m| m.name == material)?;
                        found.color = colour;
                        Some((*id, list))
                    })
                    .collect();
                for (id, materials) in changes {
                    let command = scene::Command::SetMaterials { id, materials };
                    if let Err(why) =
                        self.document
                            .execute_labelled(command, label.clone(), Some(key))
                    {
                        return self.import_failed(path, &why.to_string());
                    }
                }
                let Some(mut set) = paint::TextureSet::new(size) else {
                    return self.import_failed(path, tr("the texture size is out of range"));
                };
                set.realism = paint::channel::any_channels(&layers);
                set.layers = layers;
                if let Some(top) = set.listed(false).first().map(|(p, _)| p.clone()) {
                    self.paint_layers.choose(&material, top, false);
                }
                let command = scene::Command::SetTextureSet {
                    material: material.clone(),
                    set: (!set.layers.is_empty()).then_some(set),
                };
                if let Err(why) = self.document.execute_labelled(command, label, Some(key)) {
                    return self.import_failed(path, &why.to_string());
                }
                self.ask_to_make(&material);
                self.note = Some(crate::trf!("Loaded {} onto {}.", file, material));
            }
        }
    }

    /// Gives `material` the layers `set`, one undo step called `label`,
    /// and makes its made layers on the shapes.
    fn set_layers(&mut self, material: &str, set: paint::TextureSet, label: String) {
        let command = scene::Command::SetTextureSet {
            material: material.to_string(),
            set: Some(set),
        };
        match self.document.execute_labelled(command, label, None) {
            Ok(()) => self.ask_to_make(material),
            Err(why) => self.note = Some(crate::trf!("Couldn't change the layers: {}.", why)),
        }
    }

    /// Asks for `material`'s gradients, light and shade and smart masks
    /// to be made on the shapes.
    fn ask_to_make(&mut self, material: &str) {
        if !self
            .paint_layers
            .gradients_to_make
            .iter()
            .any(|m| m == material)
        {
            self.paint_layers
                .gradients_to_make
                .push(material.to_string());
        }
    }
}

/// A kept brush as a preset file holds it.
fn recipe_of(brush: &crate::brush_bar::BrushPreset) -> BrushRecipe {
    BrushRecipe {
        name: brush.name.clone(),
        tip: brush.tip,
        hardness: brush.hardness,
        spacing: brush.spacing,
        flow: brush.flow,
        size_pressure: brush.size_pressure,
        flow_pressure: brush.flow_pressure,
        kind: brush.kind,
        alignment: brush.alignment,
    }
}

/// A brush from a preset file, as the brush bar keeps it.
fn brush_of(recipe: BrushRecipe) -> crate::brush_bar::BrushPreset {
    crate::brush_bar::BrushPreset {
        name: recipe.name,
        tip: recipe.tip,
        hardness: recipe.hardness,
        spacing: recipe.spacing,
        flow: recipe.flow,
        size_pressure: recipe.size_pressure,
        flow_pressure: recipe.flow_pressure,
        kind: recipe.kind,
        alignment: recipe.alignment,
    }
}
