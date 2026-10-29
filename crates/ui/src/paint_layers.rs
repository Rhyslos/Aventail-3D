//! The Paint layers panel ([decision 0181](../../../docs/decisions/0181-paint-layers.md)):
//! the layers of one material's texture set, top first as Substance
//! Painter and other paint programs list them, and what can be done with
//! them: add paint and fill layers and groups, show and hide, rename,
//! reorder, set opacity and blend mode, clip to the layer below, merge
//! down, group and ungroup, duplicate and delete; give a layer a mask,
//! invert it and choose it to paint on (decision 0182).
//!
//! Every change gives the material a new texture set through
//! `Command::SetTextureSet`, one undo step each (a slider's drag is one).
//! The chosen layer is where the brush paints.

pub(crate) mod bakes;
pub(crate) mod channel_rows;
pub(crate) mod presets;
pub(crate) mod shared_note;

use crate::icons::{self, Icon};
use crate::text::tr;
use crate::theme;
use egui::{Color32, Rect, Sense, Ui, vec2};
use paint::{Blend, Content, Layer, LayerPath, TextureSet};
use scene::{Command, Document};
use std::collections::HashMap;
use std::sync::Arc;

/// Accessible name of the panel's list of layers, and the label tests
/// look for.
pub(crate) const LAYER_LIST_LABEL: &str = crate::text!("Paint layer list");

/// Merge keys of the panel's slider drags: apart from every other drag's.
const PANEL_KEYS: u64 = 0x504C_4159_0000_0000;

/// Height of one row of the list.
const ROW_HEIGHT: f32 = 24.0;

/// How far each level of group is indented.
const INDENT: f32 = 14.0;

/// The Paint layers panel's state.
#[derive(Debug, Default)]
pub(crate) struct PaintLayersPanel {
    /// The material whose layers are shown, once one has been chosen or
    /// painted.
    material: Option<String>,
    /// The chosen layer of each material's set.
    active: HashMap<String, LayerPath>,
    /// Counts slider drags, for their undo merge keys.
    drags: u64,
    /// Whether a slider was being dragged last frame.
    dragging: bool,
    /// Whether the chosen layer's mask is chosen, to paint on, rather
    /// than the layer.
    mask: bool,
    /// Which toolbar buttons can be used, worked out for one layout of
    /// layers and chosen layer: trying each on a copy of a big set every
    /// frame is slow (decision 0188).
    buttons: Option<(Vec<LayoutEntry>, LayerPath, Vec<bool>)>,
    /// The Bakes section (decision 0192), filled in by the workbench.
    pub(crate) bakes: bakes::BakesSection,
    /// Materials whose gradient layers need making again (decision 0194):
    /// the workbench makes them on the shapes and adds them to the same
    /// undo step.
    pub(crate) gradients_to_make: Vec<String>,
    /// Whether "Why does this look wrong?" was clicked (decision 0203):
    /// the workbench opens its window.
    pub(crate) look_wrong_asked: bool,
    /// What the Presets row asked for this frame (decision 0206): the
    /// workbench asks for a file.
    pub(crate) preset_asked: Option<presets::PresetAsk>,
    /// Paint shared by accident with other shapes (decision 0214), found
    /// by the workbench for the shape being painted.
    pub(crate) shared: Option<shared_note::SharedPaint>,
    /// What the shared-paint note asked for this frame.
    pub(crate) shared_fix: Option<shared_note::SharedFix>,
}

/// One layer's place and what the toolbar's buttons depend on: what it
/// holds (paint, fill or group), whether it is clipped, whether it has
/// a mask.
type LayoutEntry = (LayerPath, u8, bool, bool);

/// How `set`'s layers are laid out, for the toolbar.
fn layout_of(set: &TextureSet) -> Vec<LayoutEntry> {
    set.listed(true)
        .into_iter()
        .filter_map(|(path, _)| {
            let layer = set.layer(&path)?;
            let kind = match layer.content {
                paint::Content::Paint(_) => 0,
                paint::Content::Fill(_) => 1,
                paint::Content::Group(_) => 2,
            };
            Some((path, kind, layer.clipped, layer.mask.is_some()))
        })
        .collect()
}

/// A change to the shown set, from the panel.
enum Change {
    /// Replace the set (labelled for undo, merged under the key if any),
    /// choosing a layer.
    Set {
        set: TextureSet,
        label: &'static str,
        key: Option<u64>,
        choose: Option<LayerPath>,
        /// Whether a gradient layer is new or changed, to be made again.
        remake: bool,
    },
    /// Choose a layer, or its mask (`true`).
    Choose(LayerPath, bool),
    /// Show another material.
    Material(String),
}

impl PaintLayersPanel {
    /// The material shown: the one chosen or painted last if still
    /// around, else the first of the selected objects'.
    pub(crate) fn material(&self, doc: &Document) -> Option<String> {
        let materials = materials(doc);
        self.material
            .clone()
            .filter(|m| materials.contains(m))
            .or_else(|| materials.first().cloned())
    }

    /// Shows `material`'s layers (painting on it chooses it).
    pub(crate) fn show_material(&mut self, material: &str) {
        self.material = Some(material.to_string());
    }

    /// The chosen layer of `set` (of `material`), if it is still there.
    pub(crate) fn active(&self, material: &str, set: &TextureSet) -> Option<LayerPath> {
        self.active
            .get(material)
            .filter(|path| set.layer(path).is_some())
            .cloned()
    }

    /// Chooses the layer at `path` of `material`'s set, or its mask.
    pub(crate) fn choose(&mut self, material: &str, path: LayerPath, mask: bool) {
        self.active.insert(material.to_string(), path);
        self.mask = mask;
    }

    /// Whether the chosen layer's mask is what the brush paints on.
    pub(crate) fn editing_mask(&self) -> bool {
        self.mask
    }

    /// Draws the panel; changes go straight to the document.
    pub(crate) fn show(&mut self, ui: &mut Ui, doc: &mut Document, note: &mut Option<String>) {
        let materials = materials(doc);
        let Some(material) = self.material(doc) else {
            ui.label(
                egui::RichText::new(tr("Select an object to see its materials' paint layers."))
                    .color(theme::TEXT_WEAK),
            );
            return;
        };
        let size = doc.scene().texture().size;
        // The set as it is, shared (copying a big set every frame is slow,
        // decision 0188), or an empty one.
        let shared = doc
            .scene()
            .paint()
            .set(&material)
            .cloned()
            .or_else(|| TextureSet::new(size).map(Arc::new));
        let Some(shared) = shared else { return };
        let set: &TextureSet = &shared;
        let active = self
            .active(&material, set)
            .or_else(|| set.listed(false).first().map(|(p, _)| p.clone()));
        let mut changes = Vec::new();
        material_row(ui, &materials, &material, &mut changes);
        if let Some(shared) = &self.shared
            && let Some(fix) = shared_note::show(ui, shared, &material)
        {
            self.shared_fix = Some(fix);
        }
        realism_row(ui, set, &mut changes);
        ui.add_space(4.0);
        self.toolbar(ui, &shared, active.as_deref(), &mut changes);
        ui.add_space(4.0);
        if set.layers.is_empty()
            && let Some(start) = presets::start_row(ui)
        {
            let base = presets::bytes(presets::material_colour(doc, &material));
            if let Some(layers) = presets::starting_layers(start, size, base) {
                let mut started = set.clone();
                started.layers = layers;
                started.realism |= start == presets::Start::Realistic;
                let top = started.layers.len() - 1;
                changes.push(Change::Set {
                    set: started,
                    label: start.label(),
                    key: None,
                    choose: Some(vec![top]),
                    remake: true,
                });
            }
            ui.add_space(4.0);
        }
        if let Some(asked) = presets::presets_row(ui, set, active.is_some()) {
            self.preset_asked = Some(asked);
        }
        ui.add_space(4.0);
        if let Some(path) = &active
            && let Some(layer) = set.layer(path)
        {
            self.settings(ui, set, path, layer, &mut changes);
            ui.add_space(4.0);
        }
        let mask_chosen = self.mask
            && active
                .as_ref()
                .and_then(|p| set.layer(p))
                .is_some_and(|l| l.mask.is_some());
        layer_list(ui, set, active.as_deref(), mask_chosen, &mut changes);
        ui.add_space(6.0);
        self.bakes.show(ui);
        ui.add_space(4.0);
        if ui
            .button(tr(crate::text!("Why does this look wrong?")))
            .on_hover_text(tr(
                "Look for the usual reasons a painted model looks off (seams that show, \
                 stretched or too few pixels, flat lighting), each explained, with fixes",
            ))
            .clicked()
        {
            self.look_wrong_asked = true;
        }
        if !ui.ctx().input(|i| i.pointer.any_down()) {
            self.dragging = false;
        }
        for change in changes {
            match change {
                Change::Choose(path, mask) => self.choose(&material, path, mask),
                Change::Material(m) => self.material = Some(m),
                Change::Set {
                    set,
                    label,
                    key,
                    choose,
                    remake,
                } => {
                    if remake && !self.gradients_to_make.contains(&material) {
                        self.gradients_to_make.push(material.clone());
                        ui.ctx().request_repaint();
                    }
                    if let Some(path) = choose {
                        // A new mask is chosen at once, to paint on.
                        let mask = label == crate::text!("Add a mask");
                        self.choose(&material, path, mask);
                    }
                    let set = (!set.layers.is_empty() || set.realism).then_some(set);
                    let command = Command::SetTextureSet {
                        material: material.clone(),
                        set,
                    };
                    if let Err(why) = doc.execute_labelled(command, tr(label), key) {
                        *note = Some(crate::trf!("Couldn't change the layers: {}.", why));
                    }
                }
            }
        }
    }

    /// The buttons over the list.
    fn toolbar(
        &mut self,
        ui: &mut Ui,
        shared: &Arc<TextureSet>,
        active: Option<&[usize]>,
        out: &mut Vec<Change>,
    ) {
        let set: &TextureSet = shared;
        ui.horizontal_wrapped(|ui| {
            let mut add = |ui: &mut Ui,
                           text: &'static str,
                           hint: &'static str,
                           label: &'static str,
                           layer: Layer| {
                if ui.button(tr(text)).on_hover_text(tr(hint)).clicked() {
                    let mut set = set.clone();
                    // Made on the shape: a gradient, light and shade, or a smart mask.
                    let remake =
                        layer.is_made() || layer.mask.as_ref().is_some_and(|m| m.smart.is_some());
                    if let Some(path) = set.insert_layer(layer, active) {
                        out.push(Change::Set {
                            set,
                            label,
                            key: None,
                            choose: Some(path),
                            remake,
                        });
                    }
                }
            };
            if let Some(image) = paint::Image::new(set.size) {
                add(
                    ui,
                    crate::text!("+ Paint"),
                    crate::text!("A new paint layer above the chosen one, to paint on"),
                    crate::text!("Add a paint layer"),
                    Layer::paint(tr("Paint"), image),
                );
            }
            add(
                ui,
                crate::text!("+ Fill"),
                crate::text!(
                    "A new fill layer above the chosen one: one colour everywhere. Clip it to \
                     the layer below to colour only what that layer covers"
                ),
                crate::text!("Add a fill layer"),
                Layer::fill(tr("Fill"), [128, 128, 128, 255]),
            );
            if let Some(image) = paint::Image::new(set.size) {
                let mut layer = Layer::paint(tr("Gradient"), image);
                layer.gradient = Some(paint::Gradient::default());
                add(
                    ui,
                    crate::text!("+ Gradient"),
                    crate::text!(
                        "A new gradient above the chosen layer: dark at the bottom of the object \
                         to light at the top, to give it form. Change its colours and direction \
                         in its settings"
                    ),
                    crate::text!("Add a gradient layer"),
                    layer,
                );
            }
            if let Some(layer) = presets::light_and_shade(set.size) {
                add(
                    ui,
                    crate::text!("+ Light and shade"),
                    crate::text!(
                        "A new layer above the chosen one with soft light and shade from a sun: \
                         warm where the sun reaches, cool in the shade, as a start to paint \
                         over. Hide it with its eye to paint without lighting"
                    ),
                    crate::text!("Add a light and shade layer"),
                    layer,
                );
            }
            if let Some(layer) = presets::edge_highlight(set.size) {
                add(
                    ui,
                    crate::text!("+ Edge highlight"),
                    crate::text!(
                        "A new layer above the chosen one that catches the light along the \
                         object's outer edges, as hand-painted textures do. Change its colour, \
                         and the edges' width and breakup in its smart mask"
                    ),
                    crate::text!("Add an edge highlight"),
                    layer,
                );
            }
            if let Some(image) = paint::Image::new(set.size) {
                let mut layer = Layer::paint(tr("Material"), image);
                layer.material = Some(paint::MaterialFill::of(paint::MaterialKind::Metal));
                add(
                    ui,
                    crate::text!("+ Material"),
                    crate::text!(
                        "A new material layer above the chosen one: brushed metal, wood, stone \
                         or a coat of paint, laid on from every side of the object so it needs \
                         no particular UVs. Choose which and its colours in its settings"
                    ),
                    crate::text!("Add a material layer"),
                    layer,
                );
            }
            add(
                ui,
                crate::text!("+ Group"),
                crate::text!("A new empty group above the chosen layer"),
                crate::text!("Add a paint layer group"),
                Layer::group(tr("Group"), Vec::new()),
            );
            // Ready-made realism masks (decision 0209), with the realism
            // channels on.
            if set.realism {
                for (kind, text, hint, label) in [
                    (
                        paint::SmartKind::Dirt,
                        crate::text!("+ Dirt"),
                        crate::text!(
                            "Rough, dark grime in the creases and over the lower part of the \
                             object, through a Dirt smart mask"
                        ),
                        crate::text!("Add dirt"),
                    ),
                    (
                        paint::SmartKind::Wear,
                        crate::text!("+ Wear"),
                        crate::text!(
                            "Bare, shiny metal showing through on worn, chipped edges, through \
                             a Wear smart mask. Put it above the paint it wears through"
                        ),
                        crate::text!("Add wear"),
                    ),
                    (
                        paint::SmartKind::Rust,
                        crate::text!("+ Rust"),
                        crate::text!(
                            "Rough orange-brown rust spreading from edges and creases, with \
                             streaks running down, through a Rust smart mask"
                        ),
                        crate::text!("Add rust"),
                    ),
                ] {
                    if let Some(layer) = presets::realism_mask_layer(kind, set.size) {
                        add(ui, text, hint, label, layer);
                    }
                }
            }
        });
        let Some(path) = active else { return };
        let Some(layer) = set.layer(path) else {
            return;
        };
        // The buttons' states from last frame, if the layers are laid out
        // the same and the same one is chosen (painting changes only
        // pixels).
        let layout = layout_of(set);
        let known = match &self.buttons {
            Some((was, at, enabled)) if *was == layout && at == path => Some(enabled.clone()),
            _ => None,
        };
        let mut enabled_now = Vec::new();
        ui.horizontal_wrapped(|ui| {
            type Op = fn(&mut TextureSet, &[usize]) -> Option<LayerPath>;
            let mut run = |ui: &mut Ui,
                           text: &'static str,
                           hint: &'static str,
                           label: &'static str,
                           op: Op,
                           quick: Option<bool>| {
                // Whether it can be done: `quick` if given (Merge down's
                // try would mix two whole layers every frame, decision
                // 0188), else tried on a copy.
                let enabled = match known.as_ref().and_then(|k| k.get(enabled_now.len())) {
                    Some(enabled) => *enabled,
                    None => quick.unwrap_or_else(|| op(&mut set.clone(), path).is_some()),
                };
                enabled_now.push(enabled);
                let response = ui
                    .add_enabled(enabled, egui::Button::new(tr(text)))
                    .on_hover_text(tr(hint))
                    .on_disabled_hover_text(tr(hint));
                if response.clicked() {
                    let mut set = set.clone();
                    let choose = op(&mut set, path);
                    // A new smart mask is made on the shape (decision 0195).
                    let remake = set
                        .layer(path)
                        .and_then(|l| l.mask.as_ref())
                        .is_some_and(|m| m.smart.is_some());
                    out.push(Change::Set {
                        set,
                        label,
                        key: None,
                        choose,
                        remake,
                    });
                }
            };
            run(
                ui,
                crate::text!("Up"),
                crate::text!("Move the chosen layer up one place, over the one above it"),
                crate::text!("Move a paint layer"),
                |s, p| s.move_layer(p, true),
                None,
            );
            run(
                ui,
                crate::text!("Down"),
                crate::text!("Move the chosen layer down one place"),
                crate::text!("Move a paint layer"),
                |s, p| s.move_layer(p, false),
                None,
            );
            run(
                ui,
                crate::text!("Duplicate"),
                crate::text!("A copy of the chosen layer just above it"),
                crate::text!("Duplicate a paint layer"),
                |s, p| s.duplicate_layer(p),
                None,
            );
            run(
                ui,
                crate::text!("Merge down"),
                crate::text!(
                    "Mix the chosen layer into the one under it, as you see them, into one \
                     paint layer"
                ),
                crate::text!("Merge paint layers"),
                |s, p| s.merge_down(p),
                Some(set.can_merge_down(path)),
            );
            if layer.children().is_some() {
                run(
                    ui,
                    crate::text!("Ungroup"),
                    crate::text!("Take the group's layers out of it and remove the group"),
                    crate::text!("Ungroup paint layers"),
                    |s, p| s.ungroup(p),
                    None,
                );
            } else {
                run(
                    ui,
                    crate::text!("Group"),
                    crate::text!("Put the chosen layer in a new group"),
                    crate::text!("Group a paint layer"),
                    |s, p| s.group_layer(p, tr("Group")),
                    None,
                );
            }
            if layer.mask.is_none() {
                run(
                    ui,
                    crate::text!("Add white mask"),
                    crate::text!(
                        "A mask that shows all of the layer: paint black on it to hide parts"
                    ),
                    crate::text!("Add a mask"),
                    |s, p| add_mask(s, p, true),
                    None,
                );
                run(
                    ui,
                    crate::text!("Add black mask"),
                    crate::text!(
                        "A mask that hides all of the layer: paint white on it, or fill \
                         polygons, to show parts"
                    ),
                    crate::text!("Add a mask"),
                    |s, p| add_mask(s, p, false),
                    None,
                );
                run(
                    ui,
                    crate::text!("Add smart mask"),
                    crate::text!(
                        "A mask made from the shape: the layer shows on its outer edges to \
                         start with. Choose edges, cavities, facing up or height in its \
                         settings, and break it up with noise"
                    ),
                    crate::text!("Add a smart mask"),
                    add_smart_mask,
                    None,
                );
            } else {
                run(
                    ui,
                    crate::text!("Remove mask"),
                    crate::text!("Remove the chosen layer's mask: all of the layer shows again"),
                    crate::text!("Remove a mask"),
                    |s, p| {
                        s.layer_mut(p)?.mask.take()?;
                        Some(p.to_vec())
                    },
                    None,
                );
            }
            run(
                ui,
                crate::text!("Delete"),
                crate::text!("Delete the chosen layer"),
                crate::text!("Delete a paint layer"),
                |s, p| {
                    s.remove_layer(p)?;
                    // The layer that was under it, or the new top.
                    let mut below = p.to_vec();
                    match below.last_mut() {
                        Some(i) if *i > 0 => *i -= 1,
                        _ => {
                            below.pop()?;
                        }
                    };
                    Some(if s.layer(&below).is_some() {
                        below
                    } else {
                        s.listed(false).first().map(|(p, _)| p.clone())?
                    })
                },
                None,
            );
        });
        self.buttons = Some((layout, path.to_vec(), enabled_now));
    }

    /// The chosen layer's name, opacity, blend mode, clipping and (for a
    /// fill layer) colour.
    fn settings(
        &mut self,
        ui: &mut Ui,
        set: &TextureSet,
        path: &[usize],
        layer: &Layer,
        out: &mut Vec<Change>,
    ) {
        let started = ui.ctx().input(|i| i.pointer.any_pressed());
        if started && !self.dragging {
            self.drags += 1;
        }
        let key = PANEL_KEYS | self.drags;
        let mut changed = layer.clone();
        let mut label = None;
        let mut gradient_changed = false;
        egui::Grid::new("paint_layer_settings")
            .num_columns(2)
            .spacing(vec2(8.0, 4.0))
            .show(ui, |ui| {
                ui.label(tr("Name"));
                let mut name = changed.name.clone();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(150.0)
                        .char_limit(paint::set::MAX_NAME),
                );
                if response.changed() && !name.trim().is_empty() {
                    changed.name = name;
                    label = Some((crate::text!("Rename a paint layer"), Some(key)));
                }
                ui.end_row();

                ui.label(tr("Opacity"));
                let mut percent = changed.opacity * 100.0;
                let response = ui.add(
                    egui::Slider::new(&mut percent, 0.0..=100.0)
                        .suffix("%")
                        .max_decimals(0),
                );
                if response.dragged() || response.has_focus() {
                    self.dragging = true;
                }
                if response.changed() {
                    changed.opacity = (percent / 100.0).clamp(0.0, 1.0);
                    label = Some((crate::text!("Paint layer opacity"), Some(key)));
                }
                ui.end_row();

                let blend_label = ui.label(tr("Blend"));
                egui::ComboBox::from_id_salt("paint_layer_blend")
                    .selected_text(tr(blend_name(changed.blend)))
                    .show_ui(ui, |ui| {
                        for blend in Blend::ALL {
                            if ui
                                .selectable_label(changed.blend == blend, tr(blend_name(blend)))
                                .on_hover_text(tr(blend_hint(blend)))
                                .clicked()
                                && changed.blend != blend
                            {
                                changed.blend = blend;
                                label = Some((crate::text!("Paint layer blend mode"), None));
                            }
                        }
                    })
                    .response
                    .labelled_by(blend_label.id);
                ui.end_row();

                ui.label("");
                let mut clipped = changed.clipped;
                if ui
                    .checkbox(&mut clipped, tr("Clip to the layer below"))
                    .on_hover_text(tr(
                        "Show this layer only where the layer under it is painted: shade or \
                         pattern a part without painting over its edges",
                    ))
                    .changed()
                {
                    changed.clipped = clipped;
                    label = Some((crate::text!("Clip a paint layer"), None));
                }
                ui.end_row();

                if let Some(mask) = &mut changed.mask {
                    ui.label("");
                    let mut inverted = mask.inverted;
                    if ui
                        .checkbox(&mut inverted, tr("Invert mask"))
                        .on_hover_text(tr(
                            "Swap what the mask shows and hides: white hides, black shows",
                        ))
                        .changed()
                    {
                        mask.inverted = inverted;
                        label = Some((crate::text!("Invert a mask"), None));
                    }
                    ui.end_row();
                }

                if let Some(smart) = changed.mask.as_mut().and_then(|m| m.smart.as_mut())
                    && smart_rows(ui, smart, &mut self.dragging)
                {
                    label = Some((crate::text!("Change a smart mask"), Some(key)));
                    gradient_changed = true;
                }

                if let Some(lighting) = &mut changed.lighting
                    && lighting_rows(ui, lighting, &mut self.dragging)
                {
                    label = Some((crate::text!("Change light and shade"), Some(key)));
                    gradient_changed = true;
                }

                if let Some(fill) = &mut changed.material
                    && material_rows(ui, fill, set.realism, &mut self.dragging)
                {
                    label = Some((crate::text!("Change a material layer"), Some(key)));
                    gradient_changed = true;
                }

                if let Some(gradient) = &mut changed.gradient
                    && gradient_rows(ui, gradient, &mut self.dragging)
                {
                    label = Some((crate::text!("Change a gradient"), Some(key)));
                    gradient_changed = true;
                }

                if let Content::Fill(color) = &mut changed.content {
                    ui.label(tr("Colour"));
                    let mut c =
                        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                    let response = ui.color_edit_button_srgba(&mut c);
                    if response.changed() {
                        *color = c.to_srgba_unmultiplied();
                        label = Some((crate::text!("Fill layer colour"), Some(key)));
                    }
                    ui.end_row();
                    // Its roughness, metal and glow (decision 0207).
                    if set.realism
                        && channel_rows::fill_rows(ui, &mut changed.channels, &mut self.dragging)
                    {
                        label = Some((crate::text!("Change a fill layer's channels"), Some(key)));
                    }
                }
            });
        if let Some((label, key)) = label {
            let mut set = set.clone();
            if let Some(slot) = set.layer_mut(path) {
                *slot = changed;
            }
            out.push(Change::Set {
                set,
                label,
                key,
                choose: None,
                remake: gradient_changed,
            });
        }
    }
}

/// The realism switch (decision 0207): whether the set uses roughness,
/// metal and glow.
fn realism_row(ui: &mut Ui, set: &TextureSet, out: &mut Vec<Change>) {
    let mut on = set.realism;
    if ui
        .checkbox(&mut on, tr(REALISM_LABEL))
        .on_hover_text(tr(
            "Paint how rough or shiny the surface is, where it is metal and where it glows, \
             besides its colour: for realistic materials. The brush and fill layers get \
             Roughness, Metal and Glow, and Display → Paint preview → Realistic shows them",
        ))
        .changed()
    {
        let mut changed = set.clone();
        changed.realism = on;
        out.push(Change::Set {
            set: changed,
            label: if on {
                crate::text!("Turn the realism channels on")
            } else {
                crate::text!("Turn the realism channels off")
            },
            key: None,
            choose: None,
            remake: false,
        });
    }
}

/// The realism switch's name (decision 0207).
pub(crate) const REALISM_LABEL: &str = crate::text!("Realism channels");

/// The materials of the selected objects with a shape, then those with
/// a texture set, each once.
fn materials(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in doc.selection() {
        for m in doc.scene().material_list(id) {
            if !out.contains(&m.name) {
                out.push(m.name.clone());
            }
        }
    }
    for (name, _) in doc.scene().paint().sets() {
        if !out.iter().any(|m| m == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// The material chooser, when there is more than one.
fn material_row(ui: &mut Ui, materials: &[String], shown: &str, out: &mut Vec<Change>) {
    ui.horizontal(|ui| {
        ui.label(tr("Material"));
        if materials.len() < 2 {
            ui.label(egui::RichText::new(shown).strong());
            return;
        }
        egui::ComboBox::from_id_salt("paint_layers_material")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for m in materials {
                    if ui.selectable_label(m == shown, m).clicked() {
                        out.push(Change::Material(m.clone()));
                    }
                }
            });
    });
}

/// The layers, top first, each with its eye, its kind and its name.
fn layer_list(
    ui: &mut Ui,
    set: &TextureSet,
    active: Option<&[usize]>,
    mask_chosen: bool,
    out: &mut Vec<Change>,
) {
    let listed = set.listed(false);
    let response = ui
        .vertical(|ui| {
            if listed.is_empty() {
                ui.label(
                    egui::RichText::new(tr(
                        "No layers yet: paint on the model, or add one with + Paint.",
                    ))
                    .color(theme::TEXT_WEAK),
                );
            }
            for (path, depth) in &listed {
                let Some(layer) = set.layer(path) else {
                    continue;
                };
                let chosen = (active == Some(path)).then_some(mask_chosen);
                layer_row(ui, set, path, *depth, layer, chosen, out);
            }
        })
        .response;
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(LAYER_LIST_LABEL))
    });
}

#[allow(clippy::too_many_arguments)]
fn layer_row(
    ui: &mut Ui,
    set: &TextureSet,
    path: &[usize],
    depth: usize,
    layer: &Layer,
    chosen: Option<bool>,
    out: &mut Vec<Change>,
) {
    let width = ui.available_width().max(120.0);
    let (rect, row) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::click());
    let painter = ui.painter_at(rect);
    let accent = theme::accent(ui.ctx()).colors();
    if chosen == Some(false) {
        painter.rect_filled(rect, 3.0, accent.fill.gamma_multiply(0.35));
    } else if row.hovered() {
        painter.rect_filled(rect, 3.0, theme::TEXT_WEAK.gamma_multiply(0.12));
    }
    // The eye.
    let eye = Rect::from_min_size(rect.min, vec2(ROW_HEIGHT, ROW_HEIGHT));
    let eye_response = ui.interact(eye, ui.id().with(("paint_eye", path)), Sense::click());
    let eye_label = if layer.visible {
        tr("Hide")
    } else {
        tr("Show")
    };
    eye_response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            crate::trf!("{} {}", eye_label, layer.name),
        )
    });
    icons::paint(
        &painter,
        Rect::from_center_size(eye.center(), vec2(15.0, 15.0)),
        if layer.visible {
            Icon::Shown
        } else {
            Icon::Hidden
        },
        if eye_response.hovered() {
            theme::TEXT_STRONG
        } else {
            theme::TEXT_WEAK
        },
    );
    if eye_response.clicked() {
        let mut changed = set.clone();
        if let Some(l) = changed.layer_mut(path) {
            l.visible = !l.visible;
        }
        out.push(Change::Set {
            set: changed,
            label: if layer.visible {
                crate::text!("Hide a paint layer")
            } else {
                crate::text!("Show a paint layer")
            },
            key: None,
            choose: None,
            remake: false,
        });
    }
    let mut x = eye.right() + 4.0 + depth as f32 * INDENT;
    let middle = rect.center().y;
    // A clipped layer's mark: an arrow down to the layer it clips to.
    if layer.clipped {
        painter.text(
            egui::pos2(x, middle),
            egui::Align2::LEFT_CENTER,
            "↳",
            egui::FontId::proportional(13.0),
            theme::TEXT_WEAK,
        );
        x += 14.0;
    }
    // What kind it is: an open or closed group's arrow, a fill's colour,
    // a paint layer's brush mark.
    let mark = Rect::from_min_size(egui::pos2(x, middle - 8.0), vec2(16.0, 16.0));
    match &layer.content {
        Content::Group(_) => {
            let toggle = ui.interact(mark, ui.id().with(("paint_group", path)), Sense::click());
            painter.text(
                mark.center(),
                egui::Align2::CENTER_CENTER,
                if layer.expanded { "▾" } else { "▸" },
                egui::FontId::proportional(14.0),
                theme::TEXT,
            );
            toggle.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    crate::trf!("{} {}", tr("Open or close"), layer.name),
                )
            });
            if toggle.clicked() {
                let mut changed = set.clone();
                if let Some(l) = changed.layer_mut(path) {
                    l.expanded = !l.expanded;
                }
                out.push(Change::Set {
                    set: changed,
                    label: crate::text!("Open or close a paint layer group"),
                    key: None,
                    choose: None,
                    remake: false,
                });
            }
        }
        Content::Fill(color) => {
            painter.rect_filled(
                mark.shrink(2.0),
                2.0,
                Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 255),
            );
            painter.rect_stroke(
                mark.shrink(2.0),
                2.0,
                egui::Stroke::new(1.0, theme::TEXT_WEAK),
                egui::StrokeKind::Inside,
            );
        }
        Content::Paint(_) => {
            icons::paint(&painter, mark, Icon::PaintMode, theme::TEXT_WEAK);
        }
    }
    x = mark.right() + 6.0;
    let name_color = if layer.visible {
        theme::TEXT
    } else {
        theme::TEXT_WEAK
    };
    let mut detail = Vec::new();
    if layer.blend != Blend::Normal {
        detail.push(tr(blend_name(layer.blend)).to_string());
    }
    if layer.opacity < 1.0 {
        detail.push(format!("{:.0}%", layer.opacity * 100.0));
    }
    let text = if detail.is_empty() {
        layer.name.clone()
    } else {
        format!("{}  ·  {}", layer.name, detail.join(" · "))
    };
    painter.text(
        egui::pos2(x, middle),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(13.0),
        name_color,
    );
    row.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            chosen == Some(false),
            crate::trf!("{} {}", tr("Layer"), layer.name),
        )
    });
    // The mask, at the row's end: white, black or grey as it starts,
    // outlined in the accent colour while chosen.
    let mut mask_clicked = false;
    if let Some(mask) = &layer.mask {
        let square = Rect::from_center_size(
            egui::pos2(rect.right() - ROW_HEIGHT / 2.0, rect.center().y),
            vec2(16.0, 16.0),
        );
        let response = ui.interact(square, ui.id().with(("paint_mask", path)), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                chosen == Some(true),
                crate::trf!("{} {}", tr("Mask of"), layer.name),
            )
        });
        let shows = mask.white != mask.inverted;
        let fill = if shows {
            Color32::WHITE
        } else {
            Color32::BLACK
        };
        painter.rect_filled(square, 2.0, fill);
        let outline = if chosen == Some(true) {
            egui::Stroke::new(2.0, accent.light)
        } else {
            egui::Stroke::new(1.0, theme::TEXT_WEAK)
        };
        painter.rect_stroke(square, 2.0, outline, egui::StrokeKind::Outside);
        if response.clicked() {
            mask_clicked = true;
            out.push(Change::Choose(path.to_vec(), true));
        }
        response.on_hover_text(tr("The layer's mask: click to paint on it"));
    }
    if row.clicked() && !eye_response.clicked() && !mask_clicked {
        out.push(Change::Choose(path.to_vec(), false));
    }
}

/// A blend mode's name, as listed.
pub(crate) fn blend_name(blend: Blend) -> &'static str {
    match blend {
        Blend::Normal => crate::text!("Normal"),
        Blend::Multiply => crate::text!("Multiply"),
        Blend::Screen => crate::text!("Screen"),
        Blend::Overlay => crate::text!("Overlay"),
        Blend::Add => crate::text!("Add"),
        Blend::Subtract => crate::text!("Subtract"),
        Blend::Colour => crate::text!("Colour"),
        Blend::Hue => crate::text!("Hue"),
    }
}

/// What a blend mode does, in a line.
fn blend_hint(blend: Blend) -> &'static str {
    match blend {
        Blend::Normal => crate::text!("Covers what's under it"),
        Blend::Multiply => crate::text!("Darkens: good for shadows and dirt (white does nothing)"),
        Blend::Screen => {
            crate::text!("Lightens: good for highlights and glow (black does nothing)")
        }
        Blend::Overlay => {
            crate::text!("Adds contrast: darkens the dark parts under it and lightens the light")
        }
        Blend::Add => crate::text!("Adds its colour: strong highlights and glow"),
        Blend::Subtract => crate::text!("Takes its colour away from what's under it"),
        Blend::Colour => {
            crate::text!("Tints: its colour with the lightness under it, keeping the shading")
        }
        Blend::Hue => crate::text!("Changes the hue only, keeping saturation and lightness"),
    }
}

/// Gives the layer at `path` a mask showing all of it (`white`) or none.
fn add_mask(set: &mut TextureSet, path: &[usize], white: bool) -> Option<LayerPath> {
    let size = set.size;
    let layer = set.layer_mut(path)?;
    if layer.mask.is_some() {
        return None;
    }
    layer.mask = Some(paint::Mask::new(size, white)?);
    Some(path.to_vec())
}

/// Gives the layer at `path` a smart mask picking out its edges.
fn add_smart_mask(set: &mut TextureSet, path: &[usize]) -> Option<LayerPath> {
    let size = set.size;
    let layer = set.layer_mut(path)?;
    if layer.mask.is_some() {
        return None;
    }
    // Black under it: hidden off the polygons, where it isn't made.
    let mut mask = paint::Mask::new(size, false)?;
    mask.smart = Some(paint::SmartMask::of(paint::SmartKind::Edges));
    layer.mask = Some(mask);
    Some(path.to_vec())
}

/// The rows of a smart mask's settings (decision 0195). Whether anything
/// changed.
fn smart_rows(ui: &mut Ui, smart: &mut paint::SmartMask, dragging: &mut bool) -> bool {
    let mut changed = false;
    let kind_name = |kind: paint::SmartKind| match kind {
        paint::SmartKind::Edges => tr("Edges"),
        paint::SmartKind::Cavities => tr("Cavities"),
        paint::SmartKind::FacingUp => tr("Facing up"),
        paint::SmartKind::Height => tr("Height"),
        paint::SmartKind::Wear => tr("Wear"),
        paint::SmartKind::Dirt => tr("Dirt"),
        paint::SmartKind::Rust => tr("Rust"),
    };
    let kind_hint = |kind: paint::SmartKind| match kind {
        paint::SmartKind::Edges => tr("Outer edges: where paint wears and chips"),
        paint::SmartKind::Cavities => tr("Inner edges and creases: where dirt gathers"),
        paint::SmartKind::FacingUp => tr("Surfaces facing up: where dust, snow and moss settle"),
        paint::SmartKind::Height => {
            tr("Above a height on the object: a tide line, or toward the top")
        }
        paint::SmartKind::Wear => tr(
            "Worn edges broken into chips and scratches, where paint wears off to the \
             material under it",
        ),
        paint::SmartKind::Dirt => tr(
            "Grime in the creases and over the lower part of the object, as if splashed up \
             from the ground",
        ),
        paint::SmartKind::Rust => tr(
            "Rust blotches spreading from edges and creases, with streaks running down the \
             sides",
        ),
    };
    let label = ui.label(tr("Smart mask"));
    egui::ComboBox::from_id_salt("paint_smart_kind")
        .selected_text(kind_name(smart.kind))
        .show_ui(ui, |ui| {
            for kind in paint::SmartKind::ALL {
                if ui
                    .selectable_label(smart.kind == kind, kind_name(kind))
                    .on_hover_text(kind_hint(kind))
                    .clicked()
                    && smart.kind != kind
                {
                    let seed = smart.seed;
                    *smart = paint::SmartMask {
                        seed,
                        ..paint::SmartMask::of(kind)
                    };
                    changed = true;
                }
            }
        })
        .response
        .labelled_by(label.id);
    ui.end_row();
    let amount_name = match smart.kind {
        paint::SmartKind::Edges | paint::SmartKind::Cavities | paint::SmartKind::Wear => {
            tr("Width")
        }
        paint::SmartKind::FacingUp | paint::SmartKind::Dirt => tr("Reach"),
        paint::SmartKind::Height => tr("Level"),
        paint::SmartKind::Rust => tr("Spread"),
    };
    for (name, hint, value) in [
        (
            amount_name,
            tr(
                "How much it takes in: how wide along edges, how far from straight up, or the \
                height it starts at",
            ),
            &mut smart.amount,
        ),
        (
            tr("Softness"),
            tr("How soft its border is: low is a crisp line, high a long fade"),
            &mut smart.softness,
        ),
        (
            tr("Noise"),
            tr("How much noise breaks its border up, so wear and dirt look natural"),
            &mut smart.noise,
        ),
        (
            tr("Noise size"),
            tr("How big the noise's blotches are"),
            &mut smart.noise_size,
        ),
    ] {
        ui.label(name).on_hover_text(hint);
        // Whole percents, so the slider's rounding isn't taken as a change.
        let mut percent = (*value * 100.0).round();
        let response = ui.add(
            egui::Slider::new(&mut percent, 0.0..=100.0)
                .suffix("%")
                .max_decimals(0),
        );
        if response.dragged() || response.has_focus() {
            *dragging = true;
        }
        if response.changed() {
            *value = (percent / 100.0).clamp(0.0, 1.0);
            changed = true;
        }
        ui.end_row();
    }
    ui.label("");
    if ui
        .button(tr("New pattern"))
        .on_hover_text(tr("Another noise pattern, with the same settings"))
        .clicked()
    {
        smart.seed = smart.seed.wrapping_add(1);
        changed = true;
    }
    ui.end_row();
    changed
}

/// The gradient rows of a gradient layer's settings (decision 0194):
/// its two colours, which way it runs and where it starts and ends.
/// Whether anything changed.
/// The rows of a light-and-shade layer's settings (decision 0196).
/// Whether anything changed.
fn lighting_rows(ui: &mut Ui, lighting: &mut paint::Lighting, dragging: &mut bool) -> bool {
    let mut changed = false;
    for (name, hint, value, most) in [
        (
            tr("Sun around"),
            tr(
                "Which way the sun comes from, around the object: 0° from the right, 90° from \
                the back, 180° from the left, 270° from the front",
            ),
            &mut lighting.around,
            360.0,
        ),
        (
            tr("Sun height"),
            tr("How high the sun is: 0° on the horizon, 90° straight overhead"),
            &mut lighting.height,
            90.0,
        ),
    ] {
        ui.label(name).on_hover_text(hint);
        // Whole degrees, so the slider's rounding isn't taken as a change.
        let mut degrees = value.round();
        let response = ui.add(
            egui::Slider::new(&mut degrees, 0.0..=most)
                .suffix("°")
                .max_decimals(0),
        );
        if response.dragged() || response.has_focus() {
            *dragging = true;
        }
        if response.changed() {
            *value = degrees.clamp(0.0, most);
            changed = true;
        }
        ui.end_row();
    }
    for (name, hint, colour) in [
        (
            tr("Light"),
            tr("The colour where the sun reaches: warm, like sunlight"),
            &mut lighting.light,
        ),
        (
            tr("Shade"),
            tr("The colour in the shade: cool, like the sky's light"),
            &mut lighting.shade,
        ),
    ] {
        ui.label(name).on_hover_text(hint);
        let mut c = Color32::from_rgba_unmultiplied(colour[0], colour[1], colour[2], colour[3]);
        if ui.color_edit_button_srgba(&mut c).changed() {
            *colour = c.to_srgba_unmultiplied();
            changed = true;
        }
        ui.end_row();
    }
    for (name, hint, value) in [
        (
            tr("Warm and cool"),
            tr(
                "How warm the light and how cool the shade: 0% for plain greys of the same \
                lightness, 100% for the colours as chosen",
            ),
            &mut lighting.warmth,
        ),
        (
            tr("Softness"),
            tr("How soft the turn from light to shade is: low for a hard, cartoon line"),
            &mut lighting.softness,
        ),
    ] {
        ui.label(name).on_hover_text(hint);
        let mut percent = (*value * 100.0).round();
        let response = ui.add(
            egui::Slider::new(&mut percent, 0.0..=100.0)
                .suffix("%")
                .max_decimals(0),
        );
        if response.dragged() || response.has_focus() {
            *dragging = true;
        }
        if response.changed() {
            *value = (percent / 100.0).clamp(0.0, 1.0);
            changed = true;
        }
        ui.end_row();
    }
    ui.label("");
    let mut shadows = lighting.shadows;
    if ui
        .checkbox(&mut shadows, tr("Cast shadows"))
        .on_hover_text(tr(
            "Parts of the object shade the parts behind them from the sun",
        ))
        .changed()
    {
        lighting.shadows = shadows;
        changed = true;
    }
    ui.end_row();
    changed
}

/// The rows of a material layer's settings (decision 0210): which
/// material, its colours, scale, how sharply the sides meet, a new
/// pattern, and (with `realism`) its roughness and metal. Whether
/// anything changed.
fn material_rows(
    ui: &mut Ui,
    fill: &mut paint::MaterialFill,
    realism: bool,
    dragging: &mut bool,
) -> bool {
    use paint::MaterialKind;
    let mut changed = false;
    let name = |kind: MaterialKind| match kind {
        MaterialKind::Metal => tr("Brushed metal"),
        MaterialKind::Wood => tr("Wood"),
        MaterialKind::Stone => tr("Stone"),
        MaterialKind::Paint => tr("Paint coat"),
    };
    let label = ui.label(tr("Material"));
    egui::ComboBox::from_id_salt("paint_material_kind")
        .selected_text(name(fill.kind))
        .show_ui(ui, |ui| {
            for kind in MaterialKind::ALL {
                if ui.selectable_label(fill.kind == kind, name(kind)).clicked() && fill.kind != kind
                {
                    // Its own colours and settings, the pattern kept.
                    let seed = fill.seed;
                    *fill = paint::MaterialFill {
                        seed,
                        ..paint::MaterialFill::of(kind)
                    };
                    changed = true;
                }
            }
        })
        .response
        .labelled_by(label.id);
    ui.end_row();
    for (text, hint, colour) in [
        (
            tr("Colour"),
            tr("The material's main colour"),
            &mut fill.colour,
        ),
        (
            tr("Second colour"),
            tr("The colour of its pattern: the streaks, the rings, the specks"),
            &mut fill.second,
        ),
    ] {
        ui.label(text).on_hover_text(hint);
        let mut c = Color32::from_rgb(colour[0], colour[1], colour[2]);
        if ui.color_edit_button_srgba(&mut c).changed() {
            let [r, g, b, _] = c.to_srgba_unmultiplied();
            *colour = [r, g, b, 255];
            changed = true;
        }
        ui.end_row();
    }
    ui.label(tr("Scale"))
        .on_hover_text(tr("How big the pattern is: centimetres across one repeat"));
    let response = ui.add(
        egui::Slider::new(&mut fill.scale, paint::material_fill::SCALE_RANGE)
            .logarithmic(true)
            .suffix(" cm")
            .max_decimals(0),
    );
    changed |= response.changed();
    if response.dragged() || response.has_focus() {
        *dragging = true;
    }
    ui.end_row();
    let mut rows = vec![(
        tr("Sharpness"),
        tr(
            "How sharply the pattern from each side of the object meets at corners: low \
             blends them, high keeps each side clean",
        ),
        &mut fill.sharpness,
    )];
    if realism {
        rows.push((
            tr("Roughness"),
            tr("How rough the material is (with the realism channels)"),
            &mut fill.roughness,
        ));
        rows.push((
            tr("Metal"),
            tr("Whether the material is metal (with the realism channels)"),
            &mut fill.metal,
        ));
    }
    for (text, hint, value) in rows {
        ui.label(text).on_hover_text(hint);
        let mut percent = *value * 100.0;
        let response = ui.add(
            egui::Slider::new(&mut percent, 0.0..=100.0)
                .suffix("%")
                .max_decimals(0),
        );
        if response.dragged() || response.has_focus() {
            *dragging = true;
        }
        let rounded = (percent.round() / 100.0).clamp(0.0, 1.0);
        if response.changed() && (rounded - *value).abs() > 1e-6 {
            *value = rounded;
            changed = true;
        }
        ui.end_row();
    }
    ui.label("");
    if ui
        .button(tr("New pattern"))
        .on_hover_text(tr("A different pattern with the same settings"))
        .clicked()
    {
        fill.seed = fill.seed.wrapping_add(1);
        changed = true;
    }
    ui.end_row();
    changed
}

fn gradient_rows(ui: &mut Ui, gradient: &mut paint::Gradient, dragging: &mut bool) -> bool {
    let mut changed = false;
    for (name, hint, colour) in [
        (
            tr("From"),
            tr("The colour at the low end: the bottom, left or front"),
            &mut gradient.from,
        ),
        (
            tr("To"),
            tr("The colour at the high end: the top, right or back"),
            &mut gradient.to,
        ),
    ] {
        ui.label(name).on_hover_text(hint);
        let mut c = Color32::from_rgba_unmultiplied(colour[0], colour[1], colour[2], colour[3]);
        if ui.color_edit_button_srgba(&mut c).changed() {
            *colour = c.to_srgba_unmultiplied();
            changed = true;
        }
        ui.end_row();
    }
    let along = ui.label(tr("Along"));
    let axis_name = |axis: paint::GradientAxis| match axis {
        paint::GradientAxis::X => tr("Left to right (X)"),
        paint::GradientAxis::Y => tr("Front to back (Y)"),
        paint::GradientAxis::Z => tr("Bottom to top (Z)"),
    };
    egui::ComboBox::from_id_salt("paint_gradient_axis")
        .selected_text(axis_name(gradient.axis))
        .show_ui(ui, |ui| {
            for axis in paint::GradientAxis::ALL {
                if ui
                    .selectable_label(gradient.axis == axis, axis_name(axis))
                    .clicked()
                    && gradient.axis != axis
                {
                    gradient.axis = axis;
                    changed = true;
                }
            }
        })
        .response
        .labelled_by(along.id);
    ui.end_row();
    // What it runs across (decision 0199).
    let across = ui.label(tr("Across"));
    let span_name = |span: paint::GradientSpan| match span {
        paint::GradientSpan::Object => tr("Each object"),
        paint::GradientSpan::Part => tr("Each part"),
        paint::GradientSpan::UvPiece => tr("Each UV piece"),
    };
    let span_hint = |span: paint::GradientSpan| match span {
        paint::GradientSpan::Object => tr("From each object's bottom to its top"),
        paint::GradientSpan::Part => tr(
            "From the bottom to the top of each separate part (pieces not joined to the \
             rest), such as each plank of a crate made as one object",
        ),
        paint::GradientSpan::UvPiece => tr("Across each UV piece on its own"),
    };
    egui::ComboBox::from_id_salt("paint_gradient_span")
        .selected_text(span_name(gradient.span))
        .show_ui(ui, |ui| {
            for span in paint::GradientSpan::ALL {
                if ui
                    .selectable_label(gradient.span == span, span_name(span))
                    .on_hover_text(span_hint(span))
                    .clicked()
                    && gradient.span != span
                {
                    gradient.span = span;
                    changed = true;
                }
            }
        })
        .response
        .labelled_by(across.id);
    ui.end_row();
    for (name, hint, value) in [
        (
            tr("Starts"),
            tr("Where the gradient starts along the object, from its low side (0%)"),
            &mut gradient.start,
        ),
        (
            tr("Ends"),
            tr("Where it ends, toward its high side (100%)"),
            &mut gradient.end,
        ),
    ] {
        ui.label(name).on_hover_text(hint);
        // Whole percents, so the slider's rounding isn't taken as a change.
        let mut percent = (*value * 100.0).round();
        let response = ui.add(
            egui::Slider::new(&mut percent, 0.0..=100.0)
                .suffix("%")
                .max_decimals(0),
        );
        if response.dragged() || response.has_focus() {
            *dragging = true;
        }
        if response.changed() {
            *value = (percent / 100.0).clamp(0.0, 1.0);
            changed = true;
        }
        ui.end_row();
    }
    changed
}
