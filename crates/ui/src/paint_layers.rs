//! The Paint layers panel ([decision 0181](../../../docs/decisions/0181-paint-layers.md)):
//! the layers of one material's texture set, top first as Substance
//! Painter and other paint programs list them, and what can be done with
//! them: add paint and fill layers and groups, show and hide, rename,
//! reorder, set opacity and blend mode, clip to the layer below, merge
//! down, group and ungroup, duplicate and delete.
//!
//! Every change gives the material a new texture set through
//! `Command::SetTextureSet`, one undo step each (a slider's drag is one).
//! The chosen layer is where the brush paints.

use crate::icons::{self, Icon};
use crate::text::tr;
use crate::theme;
use egui::{Color32, Rect, Sense, Ui, vec2};
use paint::{Blend, Content, Layer, LayerPath, TextureSet};
use scene::{Command, Document};
use std::collections::HashMap;

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
    },
    /// Choose a layer.
    Choose(LayerPath),
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

    /// Chooses the layer at `path` of `material`'s set.
    pub(crate) fn choose(&mut self, material: &str, path: LayerPath) {
        self.active.insert(material.to_string(), path);
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
        let set = doc
            .scene()
            .paint()
            .set(&material)
            .map(|s| (**s).clone())
            .or_else(|| TextureSet::new(size));
        let Some(set) = set else { return };
        let active = self
            .active(&material, &set)
            .or_else(|| set.listed(false).first().map(|(p, _)| p.clone()));
        let mut changes = Vec::new();
        material_row(ui, &materials, &material, &mut changes);
        ui.add_space(4.0);
        self.toolbar(ui, &set, active.as_deref(), &mut changes);
        ui.add_space(4.0);
        if let Some(path) = &active
            && let Some(layer) = set.layer(path)
        {
            self.settings(ui, &set, path, layer, &mut changes);
            ui.add_space(4.0);
        }
        layer_list(ui, &set, active.as_deref(), &mut changes);
        if !ui.ctx().input(|i| i.pointer.any_down()) {
            self.dragging = false;
        }
        for change in changes {
            match change {
                Change::Choose(path) => self.choose(&material, path),
                Change::Material(m) => self.material = Some(m),
                Change::Set {
                    set,
                    label,
                    key,
                    choose,
                } => {
                    if let Some(path) = choose {
                        self.choose(&material, path);
                    }
                    let set = (!set.layers.is_empty()).then_some(set);
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
        set: &TextureSet,
        active: Option<&[usize]>,
        out: &mut Vec<Change>,
    ) {
        ui.horizontal_wrapped(|ui| {
            let mut add = |ui: &mut Ui,
                           text: &'static str,
                           hint: &'static str,
                           label: &'static str,
                           layer: Layer| {
                if ui.button(tr(text)).on_hover_text(tr(hint)).clicked() {
                    let mut set = set.clone();
                    if let Some(path) = set.insert_layer(layer, active) {
                        out.push(Change::Set {
                            set,
                            label,
                            key: None,
                            choose: Some(path),
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
            add(
                ui,
                crate::text!("+ Group"),
                crate::text!("A new empty group above the chosen layer"),
                crate::text!("Add a paint layer group"),
                Layer::group(tr("Group"), Vec::new()),
            );
        });
        let Some(path) = active else { return };
        let Some(layer) = set.layer(path) else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            type Op = fn(&mut TextureSet, &[usize]) -> Option<LayerPath>;
            let mut run = |ui: &mut Ui,
                           text: &'static str,
                           hint: &'static str,
                           label: &'static str,
                           op: Op| {
                let mut changed = set.clone();
                let enabled = op(&mut changed, path).is_some();
                let response = ui
                    .add_enabled(enabled, egui::Button::new(tr(text)))
                    .on_hover_text(tr(hint))
                    .on_disabled_hover_text(tr(hint));
                if response.clicked() {
                    let mut set = set.clone();
                    let choose = op(&mut set, path);
                    out.push(Change::Set {
                        set,
                        label,
                        key: None,
                        choose,
                    });
                }
            };
            run(
                ui,
                crate::text!("Up"),
                crate::text!("Move the chosen layer up one place, over the one above it"),
                crate::text!("Move a paint layer"),
                |s, p| s.move_layer(p, true),
            );
            run(
                ui,
                crate::text!("Down"),
                crate::text!("Move the chosen layer down one place"),
                crate::text!("Move a paint layer"),
                |s, p| s.move_layer(p, false),
            );
            run(
                ui,
                crate::text!("Duplicate"),
                crate::text!("A copy of the chosen layer just above it"),
                crate::text!("Duplicate a paint layer"),
                |s, p| s.duplicate_layer(p),
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
            );
            if layer.children().is_some() {
                run(
                    ui,
                    crate::text!("Ungroup"),
                    crate::text!("Take the group's layers out of it and remove the group"),
                    crate::text!("Ungroup paint layers"),
                    |s, p| s.ungroup(p),
                );
            } else {
                run(
                    ui,
                    crate::text!("Group"),
                    crate::text!("Put the chosen layer in a new group"),
                    crate::text!("Group a paint layer"),
                    |s, p| s.group_layer(p, tr("Group")),
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
            );
        });
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
            });
        }
    }
}

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
fn layer_list(ui: &mut Ui, set: &TextureSet, active: Option<&[usize]>, out: &mut Vec<Change>) {
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
                layer_row(ui, set, path, *depth, layer, active == Some(path), out);
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
    chosen: bool,
    out: &mut Vec<Change>,
) {
    let width = ui.available_width().max(120.0);
    let (rect, row) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::click());
    let painter = ui.painter_at(rect);
    let accent = theme::accent(ui.ctx()).colors();
    if chosen {
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
            chosen,
            crate::trf!("{} {}", tr("Layer"), layer.name),
        )
    });
    if row.clicked() && !eye_response.clicked() {
        out.push(Change::Choose(path.to_vec()));
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
