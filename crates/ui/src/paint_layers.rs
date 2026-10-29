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
pub(crate) mod thumbs;

use crate::attributes::colour_picker::swatch;
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

/// The button under the list with everything else (decision 0227).
pub(crate) const MORE_LABEL: &str = crate::text!("···");

/// Merge keys of the panel's slider drags: apart from every other drag's.
const PANEL_KEYS: u64 = 0x504C_4159_0000_0000;

/// Height of one row of the list: room for the pictures, the name and
/// its kind under it (decision 0227).
const ROW_HEIGHT: f32 = 36.0;

/// The side of a row's pictures (the layer's and its mask's).
const PICTURE: f32 = 26.0;

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
    /// Whether "Making the paint layers…" was shown, so the layers asked
    /// for are made the next frame (decision 0196's layers are slow on a
    /// big texture: the note shows while the window waits).
    pub(crate) making_announced: bool,
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
    /// The rows' pictures of the layers and their masks.
    thumbs: thumbs::Thumbs,
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
    /// Whether one of a layer's settings is being dragged or typed now:
    /// made layers wait until it is let go.
    pub(crate) fn is_dragging(&self) -> bool {
        self.dragging
    }

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

    /// How many layer and mask pictures the list keeps (for tests).
    #[cfg(test)]
    pub(crate) fn pictures(&self) -> usize {
        self.thumbs.len()
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
        // The list first, the chosen layer's settings under it: choosing
        // a layer then doesn't push the list (and the row clicked) away
        // from under the mouse, as it did with the settings above.
        let mask_chosen = self.mask
            && active
                .as_ref()
                .and_then(|p| set.layer(p))
                .is_some_and(|l| l.mask.is_some());
        layer_list(
            ui,
            set,
            active.as_deref(),
            mask_chosen,
            &mut self.thumbs,
            &material,
            &mut changes,
        );
        ui.add_space(4.0);
        // Under the list, as in the mockup (decision 0227).
        if let Some(asked) = self.footer(ui, &shared, active.as_deref(), &mut changes) {
            self.preset_asked = Some(asked);
        }
        ui.add_space(6.0);
        if let Some(path) = &active
            && let Some(layer) = set.layer(path)
        {
            ui.separator();
            self.settings(ui, set, path, layer, &mut changes);
            ui.add_space(6.0);
        }
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
        self.apply(ui.ctx(), doc, &material, changes, note);
    }

    /// Carries out `changes` to `material`'s layers.
    fn apply(
        &mut self,
        ctx: &egui::Context,
        doc: &mut Document,
        material: &str,
        changes: Vec<Change>,
        note: &mut Option<String>,
    ) {
        for change in changes {
            match change {
                Change::Choose(path, mask) => self.choose(material, path, mask),
                Change::Material(m) => self.material = Some(m),
                Change::Set {
                    set,
                    label,
                    key,
                    choose,
                    remake,
                } => {
                    if remake && !self.gradients_to_make.iter().any(|m| m == material) {
                        self.gradients_to_make.push(material.to_string());
                        ctx.request_repaint();
                    }
                    if let Some(path) = choose {
                        // A new mask is chosen at once, to paint on.
                        let mask = label == crate::text!("Add a mask");
                        self.choose(material, path, mask);
                    }
                    let set = (!set.layers.is_empty() || set.realism).then_some(set);
                    let command = Command::SetTextureSet {
                        material: material.to_string(),
                        set,
                    };
                    if let Err(why) = doc.execute_labelled(command, tr(label), key) {
                        *note = Some(crate::trf!("Couldn't change the layers: {}.", why));
                    }
                }
            }
        }
    }

    /// Does what a card in the shelf's Materials or Smart masks tab asks
    /// (decision 0229), on the material shown here, as one undo step.
    pub(crate) fn from_shelf(
        &mut self,
        ctx: &egui::Context,
        doc: &mut Document,
        ask: ShelfAsk,
        note: &mut Option<String>,
    ) {
        let Some(material) = self.material(doc) else {
            *note = Some(tr("Select an object to paint first.").to_string());
            return;
        };
        let size = doc.scene().texture().size;
        let Some(mut set) = doc
            .scene()
            .paint()
            .set(&material)
            .map(|s| (**s).clone())
            .or_else(|| TextureSet::new(size))
        else {
            return;
        };
        let active = self
            .active(&material, &set)
            .or_else(|| set.listed(false).first().map(|(p, _)| p.clone()));
        let change = match ask {
            ShelfAsk::Material(kind) => {
                let Some(image) = paint::Image::new(set.size) else {
                    return;
                };
                let mut layer = Layer::paint(material_kind_name(kind), image);
                layer.material = Some(paint::MaterialFill::of(kind));
                set.insert_layer(layer, active.as_deref())
                    .map(|path| Change::Set {
                        set,
                        label: crate::text!("Add a material layer"),
                        key: None,
                        choose: Some(path),
                        remake: true,
                    })
            }
            ShelfAsk::SmartMask(kind) => {
                // Black under it: hidden off the polygons, where it isn't made.
                let Some(mut mask) = paint::Mask::new(set.size, false) else {
                    return;
                };
                mask.smart = Some(paint::SmartMask::of(kind));
                match active.clone().filter(|p| set.layer(p).is_some()) {
                    // The chosen layer gets it, in place of any mask it had.
                    Some(path) => {
                        if let Some(layer) = set.layer_mut(&path) {
                            layer.mask = Some(mask);
                        }
                        Some(Change::Set {
                            set,
                            label: crate::text!("Add a smart mask"),
                            key: None,
                            choose: Some(path),
                            remake: true,
                        })
                    }
                    // No layer yet: a grey fill shown through it.
                    None => {
                        let mut layer = Layer::fill(smart_kind_name(kind), [128, 128, 128, 255]);
                        layer.mask = Some(mask);
                        set.insert_layer(layer, None).map(|path| Change::Set {
                            set,
                            label: crate::text!("Add a smart mask"),
                            key: None,
                            choose: Some(path),
                            remake: true,
                        })
                    }
                }
            }
        };
        if let Some(change) = change {
            self.apply(ctx, doc, &material, vec![change], note);
        }
    }

    /// The buttons under the list (decision 0227): the paint, fill and
    /// group adds, and ··· with the rest (the other kinds of layer, what
    /// can be done to the chosen one, the realism channels and presets).
    fn footer(
        &mut self,
        ui: &mut Ui,
        shared: &Arc<TextureSet>,
        active: Option<&[usize]>,
        out: &mut Vec<Change>,
    ) -> Option<presets::PresetAsk> {
        let set: &TextureSet = shared;
        let mut asked = None;
        ui.horizontal_wrapped(|ui| {
            if let Some(image) = paint::Image::new(set.size) {
                add_button(
                    ui,
                    set,
                    active,
                    out,
                    crate::text!("+ Paint"),
                    crate::text!("A new paint layer above the chosen one, to paint on"),
                    crate::text!("Add a paint layer"),
                    Layer::paint(tr("Paint"), image),
                );
            }
            add_button(
                ui,
                set,
                active,
                out,
                crate::text!("+ Fill"),
                crate::text!(
                    "A new fill layer above the chosen one: one colour everywhere. Clip it to \
                     the layer below to colour only what that layer covers"
                ),
                crate::text!("Add a fill layer"),
                Layer::fill(tr("Fill"), [128, 128, 128, 255]),
            );
            add_button(
                ui,
                set,
                active,
                out,
                crate::text!("+ Group"),
                crate::text!("A new empty group above the chosen layer"),
                crate::text!("Add a paint layer group"),
                Layer::group(tr("Group"), Vec::new()),
            );
            ui.menu_button(tr(MORE_LABEL), |ui| {
                ui.set_min_width(220.0);
                ui.label(egui::RichText::new(tr("Add")).small().color(theme::TEXT_WEAK));
                    if let Some(image) = paint::Image::new(set.size) {
                        let mut layer = Layer::paint(tr("Gradient"), image);
                        layer.gradient = Some(paint::Gradient::default());
                        add_button(
                            ui,
                            set,
                            active,
                            out,
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
                        add_button(
                            ui,
                            set,
                            active,
                            out,
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
                        add_button(
                            ui,
                            set,
                            active,
                            out,
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
                        add_button(
                            ui,
                            set,
                            active,
                            out,
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
                                add_button(ui, set, active, out, text, hint, label, layer);
                            }
                        }
                    }
                if let Some(path) = active
                    && let Some(layer) = set.layer(path)
                {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(crate::trf!("{} {}", tr("Layer"), layer.name))
                            .small()
                            .color(theme::TEXT_WEAK),
                    );
                    self.layer_ops(ui, set, path, layer, out);
                }
                ui.separator();
                realism_row(ui, set, out);
                ui.separator();
                if let Some(ask) = presets::presets_row(ui, set, active.is_some()) {
                    asked = Some(ask);
                }
            })
            .response
            .on_hover_text(tr(
                "More: other kinds of layer (gradient, light and shade, edge highlight, \
                 material), moving, grouping, masking and deleting the chosen layer, the \
                 realism channels and presets",
            ));
        });
        asked
    }

    /// What can be done to the chosen layer at `path`: move it, copy it,
    /// merge it down, group it, give it a mask, delete it.
    fn layer_ops(
        &mut self,
        ui: &mut Ui,
        set: &TextureSet,
        path: &[usize],
        layer: &Layer,
        out: &mut Vec<Change>,
    ) {
        // The buttons' states from last frame, if the layers are laid out
        // the same and the same one is chosen (painting changes only
        // pixels).
        let layout = layout_of(set);
        let known = match &self.buttons {
            Some((was, at, enabled)) if *was == layout && at == path => Some(enabled.clone()),
            _ => None,
        };
        let mut enabled_now = Vec::new();
        ui.vertical(|ui| {
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
                    let id = ui.id().with("fill_colour");
                    if swatch(ui, id, color, tr("Fill layer colour")) {
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

/// A button adding `layer` above the chosen one (`active`), chosen.
#[allow(clippy::too_many_arguments)]
fn add_button(
    ui: &mut Ui,
    set: &TextureSet,
    active: Option<&[usize]>,
    out: &mut Vec<Change>,
    text: &'static str,
    hint: &'static str,
    label: &'static str,
    layer: Layer,
) {
    if ui.button(tr(text)).on_hover_text(tr(hint)).clicked() {
        let mut set = set.clone();
        // Made on the shape: a gradient, light and shade, or a smart mask.
        let remake = layer.is_made() || layer.mask.as_ref().is_some_and(|m| m.smart.is_some());
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

/// The layers, top first, each with its eye, its pictures, its name and
/// kind, and its blend mode and opacity (decision 0227).
#[allow(clippy::too_many_arguments)]
fn layer_list(
    ui: &mut Ui,
    set: &TextureSet,
    active: Option<&[usize]>,
    mask_chosen: bool,
    thumbs: &mut thumbs::Thumbs,
    material: &str,
    out: &mut Vec<Change>,
) {
    let listed = set.listed(false);
    let response = ui
        .vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
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
                let row = Row {
                    set,
                    path,
                    depth: *depth,
                    layer,
                    chosen,
                };
                layer_row(ui, &row, thumbs, material, out);
            }
        })
        .response;
    thumbs.end_frame();
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(LAYER_LIST_LABEL))
    });
}

/// One row of the list: the layer at `path` of `set`, `depth` groups
/// in; `chosen` is `Some(mask)` for the chosen layer (or its mask).
struct Row<'a> {
    set: &'a TextureSet,
    path: &'a [usize],
    depth: usize,
    layer: &'a Layer,
    chosen: Option<bool>,
}

/// What a layer is, in a few words, as its row says under its name.
pub(crate) fn kind_text(layer: &Layer) -> String {
    let kind = match &layer.content {
        Content::Group(_) => tr("Group"),
        Content::Fill(_) => tr("Fill"),
        Content::Paint(_) if layer.gradient.is_some() => tr("Gradient"),
        Content::Paint(_) if layer.lighting.is_some() => tr("Light and shade"),
        Content::Paint(_) if layer.material.is_some() => tr("Material"),
        Content::Paint(_) => tr("Paint"),
    };
    let mut text = kind.to_string();
    if let Some(mask) = &layer.mask {
        let what = if mask.smart.is_some() {
            tr("smart mask")
        } else {
            tr("mask")
        };
        text = format!("{text} · {what}");
    }
    if layer.clipped {
        text = format!("{text} · {}", tr("clipped"));
    }
    text
}

fn layer_row(
    ui: &mut Ui,
    row_of: &Row<'_>,
    thumbs: &mut thumbs::Thumbs,
    material: &str,
    out: &mut Vec<Change>,
) {
    let Row {
        set,
        path,
        depth,
        layer,
        chosen,
    } = *row_of;
    let width = ui.available_width().max(160.0);
    let (rect, row) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::click());
    let painter = ui.painter_at(rect);
    let accent = theme::accent(ui.ctx()).colors();
    if chosen == Some(false) {
        painter.rect_filled(rect, 0.0, accent.fill.gamma_multiply(0.35));
    } else if row.hovered() {
        painter.rect_filled(rect, 0.0, theme::TEXT_WEAK.gamma_multiply(0.12));
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, theme::BORDER),
    );
    let middle = rect.center().y;
    // The eye.
    let eye = Rect::from_min_size(rect.min, vec2(24.0, ROW_HEIGHT));
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
    let mut x = eye.right() + 2.0 + depth as f32 * INDENT;
    // A group's arrow, to open and close it.
    let mut toggled = false;
    if layer.children().is_some() {
        let arrow = Rect::from_min_size(egui::pos2(x, middle - 8.0), vec2(14.0, 16.0));
        let toggle = ui.interact(arrow, ui.id().with(("paint_group", path)), Sense::click());
        crate::attributes::folds::arrow(&painter, arrow.center(), layer.expanded, theme::TEXT);
        toggle.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                crate::trf!("{} {}", tr("Open or close"), layer.name),
            )
        });
        if toggle.clicked() {
            toggled = true;
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
        x = arrow.right() + 2.0;
    } else if layer.clipped {
        // A clipped layer's mark: an arrow down to the layer it clips to.
        painter.text(
            egui::pos2(x, middle),
            egui::Align2::LEFT_CENTER,
            "↳",
            egui::FontId::proportional(13.0),
            theme::TEXT_WEAK,
        );
        x += 14.0;
    }
    // The layer's picture: its pixels, its colour, or stacked squares
    // for a group.
    let picture = Rect::from_min_size(
        egui::pos2(x, middle - PICTURE / 2.0),
        vec2(PICTURE, PICTURE),
    );
    match &layer.content {
        Content::Group(_) => {
            for step in [4.0, 0.0] {
                let r = Rect::from_min_size(
                    picture.min + vec2(step + 3.0, 7.0 - step),
                    vec2(PICTURE - 10.0, PICTURE - 10.0),
                );
                painter.rect_filled(r, 2.0, theme::PANEL);
                painter.rect_stroke(
                    r,
                    2.0,
                    egui::Stroke::new(1.0, theme::TEXT_WEAK),
                    egui::StrokeKind::Inside,
                );
            }
        }
        Content::Fill(color) => {
            painter.rect_filled(
                picture,
                3.0,
                Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 255),
            );
        }
        Content::Paint(image) => {
            let id = thumbs.picture(ui.ctx(), material, path, image, None);
            painter.image(
                id,
                picture,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
    if layer.children().is_none() {
        painter.rect_stroke(
            picture,
            3.0,
            egui::Stroke::new(1.0, theme::BORDER),
            egui::StrokeKind::Inside,
        );
    }
    x = picture.right() + 4.0;
    // The mask's picture beside it: click it to paint on the mask.
    let mut mask_clicked = false;
    if let Some(mask) = &layer.mask {
        let square = Rect::from_min_size(
            egui::pos2(x, middle - PICTURE / 2.0),
            vec2(PICTURE, PICTURE),
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
        let id = thumbs.picture(
            ui.ctx(),
            material,
            path,
            &mask.image,
            Some((mask.white, mask.inverted)),
        );
        painter.image(
            id,
            square,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let outline = if chosen == Some(true) {
            egui::Stroke::new(2.0, accent.light)
        } else {
            egui::Stroke::new(1.0, theme::TEXT_WEAK)
        };
        painter.rect_stroke(square, 3.0, outline, egui::StrokeKind::Outside);
        if response.clicked() {
            mask_clicked = true;
            out.push(Change::Choose(path.to_vec(), true));
        }
        response.on_hover_text(tr("The layer's mask: click to paint on it"));
        x = square.right() + 4.0;
    }
    x += 4.0;
    // The blend mode over the opacity, on the right.
    let right = rect.right() - 8.0;
    let small = egui::FontId::proportional(11.0);
    let blend = painter.text(
        egui::pos2(right, middle - 1.0),
        egui::Align2::RIGHT_BOTTOM,
        tr(blend_name(layer.blend)),
        small.clone(),
        theme::TEXT_WEAK,
    );
    let opacity = painter.text(
        egui::pos2(right, middle + 1.0),
        egui::Align2::RIGHT_TOP,
        format!("{:.0}%", layer.opacity * 100.0),
        small.clone(),
        theme::TEXT_WEAK,
    );
    // The name, and what kind of layer it is under it.
    let text_end = blend.left().min(opacity.left()) - 6.0;
    let words = painter.with_clip_rect(Rect::from_x_y_ranges(x..=text_end.max(x), rect.y_range()));
    let name_color = if layer.visible {
        theme::TEXT
    } else {
        theme::TEXT_WEAK
    };
    words.text(
        egui::pos2(x, middle + 1.0),
        egui::Align2::LEFT_BOTTOM,
        &layer.name,
        egui::FontId::proportional(13.0),
        name_color,
    );
    words.text(
        egui::pos2(x, middle + 2.0),
        egui::Align2::LEFT_TOP,
        kind_text(layer),
        small,
        theme::TEXT_WEAK,
    );
    row.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            chosen == Some(false),
            crate::trf!("{} {}", tr("Layer"), layer.name),
        )
    });
    if row.clicked() && !eye_response.clicked() && !mask_clicked && !toggled {
        out.push(Change::Choose(path.to_vec(), false));
    }
}

/// A smart mask kind's name, as listed.
pub(crate) fn smart_kind_name(kind: paint::SmartKind) -> &'static str {
    match kind {
        paint::SmartKind::Edges => tr("Edges"),
        paint::SmartKind::Cavities => tr("Cavities"),
        paint::SmartKind::FacingUp => tr("Facing up"),
        paint::SmartKind::Height => tr("Height"),
        paint::SmartKind::Wear => tr("Wear"),
        paint::SmartKind::Dirt => tr("Dirt"),
        paint::SmartKind::Rust => tr("Rust"),
    }
}

/// What a smart mask kind shows.
pub(crate) fn smart_kind_hint(kind: paint::SmartKind) -> &'static str {
    match kind {
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
    }
}

/// A material kind's name, as listed.
pub(crate) fn material_kind_name(kind: paint::MaterialKind) -> &'static str {
    match kind {
        paint::MaterialKind::Metal => tr("Brushed metal"),
        paint::MaterialKind::Wood => tr("Wood"),
        paint::MaterialKind::Stone => tr("Stone"),
        paint::MaterialKind::Paint => tr("Paint coat"),
    }
}

/// What a card in the shelf's Materials or Smart masks tab asks for
/// (decision 0229).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShelfAsk {
    /// A material layer of this kind above the chosen layer.
    Material(paint::MaterialKind),
    /// A smart mask of this kind on the chosen layer.
    SmartMask(paint::SmartKind),
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
    let kind_name = smart_kind_name;
    let kind_hint = smart_kind_hint;
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
        if swatch(ui, ui.id().with(name), colour, name) {
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
    let name = material_kind_name;
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
        if swatch(ui, ui.id().with(text), colour, text) {
            colour[3] = 255;
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
        if swatch(ui, ui.id().with(name), colour, name) {
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
