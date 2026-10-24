//! Edit layers ([decision 0091](../../../docs/decisions/0091-edit-layers.md)):
//! the modifiers palette beside the viewport, with its Edit layer button,
//! and the Edit layers panel, which lists the selected object's layers and
//! their edits.
//!
//! An object's edits are kept in layers, applied bottom first. New edits
//! go into the top layer that is switched on. A layer can be switched off
//! to see the shape without its edits, renamed, moved, merged into the
//! one below or deleted with its edits. Every change is an undoable
//! command ([`scene::Command::AddLayer`], [`scene::Command::ChangeLayer`]).

use crate::attributes::OpenedEdit;
use crate::icons::{self, Icon};
use crate::objects::DocRequest;
use crate::palette::{ButtonState, icon_button};
use crate::text::tr;
use crate::theme;
use egui::{Align2, FontId, Key, Rect, RichText, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};
use scene::{Command, Document, LayerChange, ObjectId, ObjectKind};

/// The modifiers palette's width, as the mode palette's.
pub(crate) const MODIFIERS_WIDTH: f32 = 44.0;

/// The Edit layer button's name, also used in the Mesh menu.
pub(crate) const ADD_LAYER_LABEL: &str = crate::text!("Add edit layer");

/// What the Edit layer button does.
pub(crate) const ADD_LAYER_HELP: &str = crate::text!(
    "Add an empty edit layer on top of each selected \
     object. The edits you make from now on go into it, so you can switch them all off to \
     compare, or delete them in one go. The Edit layers panel lists the layers."
);

/// The Edit layers panel's menu.
pub(crate) const LAYER_MENU: &str = crate::text!("Layer");

/// The mark on the layer new edits go into.
pub(crate) const NEW_EDITS_MARK: &str = crate::text!("new edits");

/// The mark on an edit that can't find some of its parts.
pub(crate) const MISSING_MARK: &str = crate::text!("parts missing");
/// The mark on an edit skipped as too big.
pub(crate) const SKIPPED_MARK: &str = crate::text!("skipped");

const ROW_HEIGHT: f32 = 22.0;
const EDIT_HEIGHT: f32 = 20.0;
/// Where an edit's eye and name sit, from the row's left.
const EDIT_EYE_X: f32 = 32.0;
const EDIT_TEXT_INDENT: f32 = 46.0;

/// What was picked on the modifiers palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModifierPick {
    /// Add an edit layer to the selected objects.
    AddLayer,
    /// A boolean on the selection: the first selected kept, the others
    /// cutting it (decision 0135).
    Boolean(scene::BooleanKind),
    /// A modifier added to the selected shapes (decision 0142), by its
    /// command.
    Modifier(crate::commands::CommandId),
}

/// The selected objects that can take an edit layer: those with a shape
/// of their own (not groups or instances).
pub(crate) fn layer_targets(doc: &Document) -> Vec<ObjectId> {
    doc.selection()
        .into_iter()
        .filter(|id| {
            doc.scene().object(*id).is_some_and(|o| {
                o.kind.has_shape()
                    && !matches!(o.kind, ObjectKind::Instance(_) | ObjectKind::Curve(_))
            })
        })
        .collect()
}

/// The modifiers palette: a thin column of buttons for what changes a
/// shape without destroying it. Edit layers is the first, then the
/// booleans (decision 0135), then the modifiers family by family
/// (decision 0142).
pub(crate) fn modifiers_palette(ui: &mut Ui, doc: &Document) -> Option<ModifierPick> {
    let mut pick = None;
    ui.vertical_centered(|ui| {
        ui.add_space(6.0);
        let any = !layer_targets(doc).is_empty();
        let (state, note) = if any {
            (ButtonState::Normal, tr(ADD_LAYER_HELP))
        } else {
            (
                ButtonState::Unavailable,
                tr("Select an object with a shape first (not a group or an instance)."),
            )
        };
        if icon_button(ui, Icon::EditLayer, tr(ADD_LAYER_LABEL), note, state).clicked() {
            pick = Some(ModifierPick::AddLayer);
        }
        ui.add_space(6.0);
        if let Some(kind) = crate::booleans::palette_buttons(ui, doc) {
            pick = Some(ModifierPick::Boolean(kind));
        }
        if let Some(command) = crate::modifiers::palette_buttons(ui, doc) {
            pick = Some(ModifierPick::Modifier(command));
        }
    });
    pick
}

/// A layer's name being typed.
#[derive(Debug, Clone, PartialEq)]
struct Renaming {
    id: ObjectId,
    index: usize,
    text: String,
    /// The text box still needs to take the keyboard and select its text.
    just_started: bool,
}

/// The Edit layers panel's own state: the layer clicked last (which its
/// Layer menu works on) and a name being typed.
#[derive(Debug, Clone, Default)]
pub(crate) struct LayersPanel {
    /// The object and layer (from the bottom) clicked last.
    chosen: Option<(ObjectId, usize)>,
    renaming: Option<Renaming>,
}

/// Which object the panel shows, or why it shows none.
fn shown_object(doc: &Document) -> Result<ObjectId, String> {
    let selection = doc.selection();
    let [id] = selection.as_slice() else {
        return Err(if selection.is_empty() {
            tr("Select an object to see its edit layers.").into()
        } else {
            crate::trf!(
                "{} objects are selected: select one to see its edit layers. (Add edit layer \
                 still adds one to each.)",
                selection.len()
            )
        });
    };
    let object = doc
        .scene()
        .object(*id)
        .ok_or_else(|| tr("Select an object to see its edit layers.").to_string())?;
    match object.kind {
        ObjectKind::Group => Err(tr("A group has no shape of its own, so no edit layers.").into()),
        ObjectKind::Lattice(_) => Err(tr(
            "A lattice cage has no shape of its own, so no edit layers: press 1 to move its \
             points.",
        )
        .into()),
        ObjectKind::Instance(_) => Err(tr(
            "An instance shows its original's shape and edits: select the original to see its \
             layers.",
        )
        .into()),
        ObjectKind::Curve(_) => Err(tr(
            "A curve's shape comes from its points and settings, so it has no edit layers: Bake \
             into shape (Ctrl + E) to edit it as a mesh.",
        )
        .into()),
        _ => Ok(*id),
    }
}

impl LayersPanel {
    /// The layer the Layer menu works on, if it is still there.
    fn chosen_layer(&self, doc: &Document) -> Option<(ObjectId, usize)> {
        let id = shown_object(doc).ok()?;
        let (chosen, index) = self.chosen?;
        let count = doc.scene().object(id)?.stack.layers().len();
        (chosen == id && index < count).then_some((id, index))
    }

    /// The panel's menu line: the Layer menu.
    pub(crate) fn menus(&mut self, ui: &mut Ui, doc: &Document, out: &mut Vec<DocRequest>) {
        crate::dialog::menu(ui, tr(LAYER_MENU), |ui| {
            let targets = layer_targets(doc);
            if ui
                .add_enabled(!targets.is_empty(), egui::Button::new(tr(ADD_LAYER_LABEL)))
                .on_hover_text(tr(ADD_LAYER_HELP))
                .clicked()
            {
                out.push(DocRequest::Run(Command::AddLayer(targets)));
                ui.close();
            }
            ui.separator();
            match self.chosen_layer(doc) {
                Some((id, index)) => self.layer_entries(ui, doc, id, index, out),
                None => {
                    ui.label(
                        RichText::new(tr("Click a layer to change it")).color(theme::TEXT_WEAK),
                    );
                }
            }
        });
    }

    /// The entries that change one layer, for the Layer menu and a
    /// layer's right-click menu.
    fn layer_entries(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        index: usize,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(object) = doc.scene().object(id) else {
            return;
        };
        let layers = object.stack.layers();
        let Some(layer) = layers.get(index) else {
            return;
        };
        let change =
            |change: LayerChange| DocRequest::Run(Command::ChangeLayer { id, index, change });
        let entry = |ui: &mut Ui, name: &str, enabled: bool, help: &str| {
            ui.add_enabled(enabled, egui::Button::new(name))
                .on_hover_text(help)
                .on_disabled_hover_text(help)
                .clicked()
        };
        if entry(ui, tr("Rename"), true, tr("Type a new name for the layer")) {
            self.start_rename(id, index, &layer.name);
            ui.close();
        }
        let (word, help) = if layer.enabled {
            (
                tr("Switch off"),
                tr(
                    "See the shape without this layer's edits. New edits go into the top layer \
                 that is on.",
                ),
            )
        } else {
            (tr("Switch on"), tr("Bring this layer's edits back"))
        };
        if entry(ui, word, true, help) {
            out.push(change(LayerChange::SetEnabled(!layer.enabled)));
            ui.close();
        }
        ui.separator();
        let top = index + 1 == layers.len();
        if entry(
            ui,
            tr("Move up"),
            !top,
            tr("Apply this layer's edits after the layer above it"),
        ) {
            out.push(change(LayerChange::Move { up: true }));
            self.chosen = Some((id, index + 1));
            ui.close();
        }
        if entry(
            ui,
            tr("Move down"),
            index > 0,
            tr("Apply this layer's edits before the layer below it"),
        ) {
            out.push(change(LayerChange::Move { up: false }));
            self.chosen = Some((id, index - 1));
            ui.close();
        }
        let both_on = index > 0 && layer.enabled && layers[index - 1].enabled;
        let merge_help = if index == 0 {
            tr("There is no layer below to merge into")
        } else if both_on {
            tr("Put this layer's edits at the end of the layer below, making them one layer")
        } else {
            tr("Switch both layers on first")
        };
        if entry(ui, tr("Merge down"), both_on, merge_help) {
            out.push(change(LayerChange::MergeDown));
            self.chosen = Some((id, index - 1));
            ui.close();
        }
        ui.separator();
        if entry(
            ui,
            tr("Delete layer"),
            true,
            tr("Delete the layer and every edit in it (Ctrl + Z brings them back)"),
        ) {
            out.push(change(LayerChange::Delete));
            self.chosen = None;
            ui.close();
        }
    }

    fn start_rename(&mut self, id: ObjectId, index: usize, name: &str) {
        self.renaming = Some(Renaming {
            id,
            index,
            text: name.to_string(),
            just_started: true,
        });
    }

    /// Whether a layer's name is being typed (so keys go to the box).
    pub(crate) fn is_renaming(&self) -> bool {
        self.renaming.is_some()
    }

    /// The panel: the selected object's layers, top first, each with its
    /// edits.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        opened: &mut Option<OpenedEdit>,
        out: &mut Vec<DocRequest>,
    ) {
        let id = match shown_object(doc) {
            Ok(id) => id,
            Err(why) => {
                self.renaming = None;
                ui.label(RichText::new(why).color(theme::TEXT_WEAK));
                return;
            }
        };
        let Some(object) = doc.scene().object(id) else {
            return;
        };
        let layers = object.stack.layers();
        let recording = object.stack.recording_layer();
        ui.horizontal(|ui| {
            ui.label(RichText::new(&object.name).color(theme::TEXT_STRONG));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(tr("Add layer"))
                    .on_hover_text(tr(ADD_LAYER_HELP))
                    .clicked()
                {
                    out.push(DocRequest::Run(Command::AddLayer(vec![id])));
                    self.chosen = Some((id, layers.len()));
                }
            });
        });
        ui.add_space(4.0);
        if layers.is_empty() {
            ui.label(
                RichText::new(tr(
                    "No edit layers yet. The first edit you make goes into a layer called \
                     Edits. Add a layer to keep a set of edits apart: switch it off to \
                     compare, or delete it to take them all back.",
                ))
                .color(theme::TEXT_WEAK),
            );
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for index in (0..layers.len()).rev() {
                    self.layer_row(ui, doc, id, index, recording == Some(index), out);
                    self.edit_rows(ui, doc, id, index, &layers[index], opened, out);
                    ui.add_space(4.0);
                }
                if recording.is_none() {
                    ui.label(
                        RichText::new(
                            tr("Every layer is off: the next edit makes a new layer on top."),
                        )
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                }
                ui.label(
                    RichText::new(tr("Click an edit to open it in the Attributes panel. Right-click a layer or an edit for more."))
                        .small()
                        .color(theme::TEXT_WEAK),
                );
            });
    }

    /// One layer's row: eye, name, edit count and, on the layer new edits
    /// go into, a "new edits" mark.
    fn layer_row(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        index: usize,
        recording: bool,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(layer) = doc
            .scene()
            .object(id)
            .and_then(|o| o.stack.layers().get(index))
            .cloned()
        else {
            return;
        };
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
        // A fixed id, so a double-click is told apart however the rows
        // around it change.
        let response = ui.interact(rect, ui.id().with(("layer row", id, index)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::SelectableLabel,
                true,
                self.chosen == Some((id, index)),
                &layer.name,
            )
        });
        let painter = ui.painter_at(rect);
        let chosen = self.chosen == Some((id, index));
        if chosen {
            painter.rect_filled(rect, 3.0, theme::accent(ui.ctx()).colors().fill);
        } else if response.hovered() {
            painter.rect_filled(rect, 3.0, theme::HOVER.gamma_multiply(0.6));
        }

        // The eye, on the left: is the layer on?
        let eye =
            Rect::from_center_size(pos2(rect.left() + 12.0, rect.center().y), vec2(20.0, 18.0));
        let eye_response = ui.interact(eye, ui.id().with(("layer eye", id, index)), Sense::click());
        let eye_label = if layer.enabled {
            tr("Switch off")
        } else {
            tr("Switch on")
        };
        eye_response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Button,
                true,
                crate::trf!("{} {}", eye_label, layer.name),
            )
        });
        let eye_color = if eye_response.hovered() {
            theme::TEXT_STRONG
        } else if layer.enabled {
            theme::TEXT_WEAK
        } else {
            theme::TEXT_WEAK.gamma_multiply(0.6)
        };
        icons::paint(
            &painter,
            Rect::from_center_size(eye.center(), vec2(15.0, 15.0)),
            if layer.enabled {
                Icon::Shown
            } else {
                Icon::Hidden
            },
            eye_color,
        );
        if eye_response
            .on_hover_text(crate::trf!(
                "{} {}: see the shape {} its edits",
                eye_label,
                layer.name,
                if layer.enabled {
                    tr("without")
                } else {
                    tr("with")
                }
            ))
            .clicked()
        {
            out.push(DocRequest::Run(Command::ChangeLayer {
                id,
                index,
                change: LayerChange::SetEnabled(!layer.enabled),
            }));
        }

        // On the right: how many edits, and the "new edits" mark.
        let count = match layer.entries.len() {
            1 => tr("1 edit").to_string(),
            n => crate::trf!("{} edits", n),
        };
        let mut right = rect.right() - 6.0;
        let count_rect = painter.text(
            pos2(right, rect.center().y),
            Align2::RIGHT_CENTER,
            count,
            FontId::proportional(11.5),
            theme::TEXT_WEAK,
        );
        right = count_rect.left() - 8.0;
        if recording {
            let mark = painter.text(
                pos2(right, rect.center().y),
                Align2::RIGHT_CENTER,
                tr(NEW_EDITS_MARK),
                FontId::proportional(11.0),
                theme::accent(ui.ctx()).colors().light,
            );
            let mark_response =
                ui.interact(mark, ui.id().with(("new edits", id, index)), Sense::hover());
            mark_response
                .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, tr(NEW_EDITS_MARK)));
            mark_response.on_hover_text(tr(
                "The edits you make now go into this layer: the top layer that is switched on",
            ));
            right = mark.left() - 8.0;
        }

        // The name, or a box to type a new one in.
        let name_rect = Rect::from_min_max(
            pos2(eye.right() + 4.0, rect.top() + 1.0),
            pos2(right.max(eye.right() + 24.0), rect.bottom() - 1.0),
        );
        let renaming_here = self
            .renaming
            .as_ref()
            .is_some_and(|r| r.id == id && r.index == index);
        if renaming_here {
            self.rename_box(ui, name_rect, out);
        } else {
            let color = match (chosen, layer.enabled) {
                (true, _) => theme::TEXT_STRONG,
                (false, true) => theme::TEXT,
                (false, false) => theme::TEXT_WEAK.gamma_multiply(0.7),
            };
            ui.painter_at(name_rect).text(
                pos2(name_rect.left(), rect.center().y),
                Align2::LEFT_CENTER,
                &layer.name,
                FontId::proportional(13.0),
                color,
            );
        }

        if !renaming_here {
            if response.double_clicked() {
                self.chosen = Some((id, index));
                self.start_rename(id, index, &layer.name);
            } else if response.clicked() || response.secondary_clicked() {
                self.chosen = Some((id, index));
            }
        }
        response.context_menu(|ui| {
            ui.set_min_width(crate::dialog::MENU_WIDTH);
            self.layer_entries(ui, doc, id, index, out);
        });
    }

    /// A layer's edits, newest first, as the Edit menu names them (the
    /// edit list, decision 0092). Click one to open it in the Attributes
    /// panel; its eye switches it off; right-click for the rest.
    #[allow(clippy::too_many_arguments)]
    fn edit_rows(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        index: usize,
        layer: &scene::Layer,
        opened: &mut Option<OpenedEdit>,
        out: &mut Vec<DocRequest>,
    ) {
        if layer.entries.is_empty() {
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), EDIT_HEIGHT), Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, tr("No edits")));
            ui.painter_at(rect).text(
                pos2(rect.left() + EDIT_TEXT_INDENT, rect.center().y),
                Align2::LEFT_CENTER,
                tr("No edits"),
                FontId::proportional(12.0),
                theme::TEXT_WEAK.gamma_multiply(0.7),
            );
            return;
        }
        let warnings = doc
            .scene()
            .evaluated(id)
            .map(|e| e.warnings)
            .unwrap_or_default();
        for (place, entry) in layer.entries.iter().enumerate().rev() {
            let text = entry.operation.describe();
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), EDIT_HEIGHT), Sense::hover());
            let response = ui.interact(
                rect,
                ui.id().with(("edit row", id, entry.id)),
                Sense::click(),
            );
            let is_open = opened.is_some_and(|o| o.is(doc, id, entry.id));
            response.widget_info(|| {
                WidgetInfo::selected(WidgetType::SelectableLabel, true, is_open, &text)
            });
            let painter = ui.painter_at(rect);
            let accent = theme::accent(ui.ctx()).colors();
            if is_open {
                painter.rect_filled(rect, 3.0, accent.fill.gamma_multiply(0.55));
            } else if response.hovered() {
                painter.rect_filled(rect, 3.0, theme::HOVER.gamma_multiply(0.5));
            }

            // Its eye.
            let eye = Rect::from_center_size(
                pos2(rect.left() + EDIT_EYE_X, rect.center().y),
                vec2(16.0, 16.0),
            );
            let eye_response = ui.interact(
                eye,
                ui.id().with(("edit eye", id, entry.id)),
                Sense::click(),
            );
            let eye_label = if entry.enabled {
                tr("Switch off")
            } else {
                tr("Switch on")
            };
            eye_response.widget_info(|| {
                WidgetInfo::labeled(
                    WidgetType::Button,
                    true,
                    crate::trf!("{} {}", eye_label, text),
                )
            });
            let eye_color = if eye_response.hovered() {
                theme::TEXT_STRONG
            } else if entry.enabled && layer.enabled {
                theme::TEXT_WEAK.gamma_multiply(0.85)
            } else {
                theme::TEXT_WEAK.gamma_multiply(0.45)
            };
            icons::paint(
                &painter,
                Rect::from_center_size(eye.center(), vec2(12.0, 12.0)),
                if entry.enabled {
                    Icon::Shown
                } else {
                    Icon::Hidden
                },
                eye_color,
            );
            if eye_response
                .on_hover_text(crate::trf!(
                    "{} this edit: see the shape {} it",
                    eye_label,
                    if entry.enabled {
                        tr("without")
                    } else {
                        tr("with")
                    }
                ))
                .clicked()
            {
                out.push(DocRequest::Run(Command::SetEditEnabled {
                    id,
                    edit: entry.id,
                    enabled: !entry.enabled,
                }));
            }

            // Marks on the right: kept editable, and a real problem.
            let mut right = rect.right() - 6.0;
            let mut mark = |word: &str, color: egui::Color32, help: String, key: &str| {
                let at = painter.text(
                    pos2(right, rect.center().y),
                    Align2::RIGHT_CENTER,
                    word,
                    FontId::proportional(11.0),
                    color,
                );
                let hover = ui.interact(at, ui.id().with((key, id, entry.id)), Sense::hover());
                hover.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, word));
                hover.on_hover_text(help);
                right = at.left() - 8.0;
            };
            if let Some(problem) = crate::attributes::edit_warning(&warnings, index, place) {
                let skipped = warnings
                    .iter()
                    .any(|w| w.layer == index && w.operation == place && w.too_big);
                let word = if skipped {
                    tr(SKIPPED_MARK)
                } else {
                    tr(MISSING_MARK)
                };
                mark(word, theme::WARNING, problem, "edit_problem");
            }

            let mut color = if is_open {
                theme::TEXT_STRONG
            } else {
                theme::TEXT
            };
            if !entry.enabled || !layer.enabled {
                color = theme::TEXT_WEAK.gamma_multiply(0.6);
            }
            let shown = if entry.enabled {
                text.clone()
            } else {
                crate::trf!("{} (off)", text)
            };
            let text_rect = Rect::from_min_max(
                pos2(rect.left() + EDIT_TEXT_INDENT, rect.top()),
                pos2(
                    right.max(rect.left() + EDIT_TEXT_INDENT + 20.0),
                    rect.bottom(),
                ),
            );
            ui.painter_at(text_rect).text(
                pos2(text_rect.left(), rect.center().y),
                Align2::LEFT_CENTER,
                shown,
                FontId::proportional(12.0),
                color,
            );

            if response.clicked() || response.secondary_clicked() {
                *opened = Some(OpenedEdit::new(doc, id, entry.id));
                self.chosen = Some((id, index));
            }
            response.context_menu(|ui| {
                ui.set_min_width(crate::dialog::MENU_WIDTH);
                edit_entries(ui, doc, id, entry, opened, out);
            });
        }
    }

    /// The text box for a layer's new name. Enter (or clicking elsewhere)
    /// keeps the name; Esc goes back to the old one.
    fn rename_box(&mut self, ui: &mut Ui, rect: Rect, out: &mut Vec<DocRequest>) {
        let Some(renaming) = self.renaming.as_mut() else {
            return;
        };
        let box_id = ui.id().with(("rename layer", renaming.id, renaming.index));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let output = egui::TextEdit::singleline(&mut renaming.text)
            .id(box_id)
            .desired_width(rect.width())
            .margin(vec2(4.0, 1.0))
            .show(&mut child);
        let response = output.response;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, tr("Layer name")));
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.renaming = None;
            ui.memory_mut(|m| m.surrender_focus(box_id));
            return;
        }
        if renaming.just_started {
            renaming.just_started = false;
            response.request_focus();
            let mut state = output.state;
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(renaming.text.chars().count()),
            );
            state.cursor.set_char_range(Some(all));
            state.store(ui.ctx(), box_id);
        } else if response.lost_focus() {
            out.push(DocRequest::Run(Command::ChangeLayer {
                id: renaming.id,
                index: renaming.index,
                change: LayerChange::Rename(renaming.text.clone()),
            }));
            self.renaming = None;
        }
    }
}

/// The entries for one edit, in its right-click menu (decision 0092).
fn edit_entries(
    ui: &mut Ui,
    doc: &Document,
    id: ObjectId,
    entry: &scene::Entry,
    opened: &mut Option<OpenedEdit>,
    out: &mut Vec<DocRequest>,
) {
    let edit = entry.id;
    if ui
        .button(tr("Open in Attributes"))
        .on_hover_text(tr(
            "Show this edit's settings in the Attributes panel, to change them",
        ))
        .clicked()
    {
        *opened = Some(OpenedEdit::new(doc, id, edit));
        ui.close();
    }
    ui.separator();
    let word = if entry.enabled {
        tr("Switch off")
    } else {
        tr("Switch on")
    };
    if ui
        .button(word)
        .on_hover_text(tr("Off: the shape is made as if this edit wasn't there"))
        .clicked()
    {
        out.push(DocRequest::Run(Command::SetEditEnabled {
            id,
            edit,
            enabled: !entry.enabled,
        }));
        ui.close();
    }
    ui.separator();
    let problem = doc
        .scene()
        .object(id)
        .and_then(|o| o.stack.freeze_problem(edit));
    let help = problem.map_or_else(
        || {
            tr(
                "Bake the shape's settings and every edit up to this one into the shape: the \
             shape stays the same, big models stay quick, and those edits can't be changed \
             any more",
            )
            .to_string()
        },
        |why| crate::trf!("Can't bake into the shape: {}", why),
    );
    if ui
        .add_enabled(
            problem.is_none(),
            egui::Button::new(tr(crate::attributes::FREEZE_LABEL)),
        )
        .on_hover_text(&help)
        .on_disabled_hover_text(&help)
        .clicked()
    {
        out.push(DocRequest::Run(Command::Freeze { id, edit }));
        *opened = None;
        ui.close();
    }
    if ui
        .button(tr(crate::attributes::DELETE_EDIT_LABEL))
        .on_hover_text(tr(
            "Take this edit out; the edits after it are made again without it",
        ))
        .clicked()
    {
        out.push(DocRequest::Run(Command::DeleteEdit { id, edit }));
        *opened = None;
        ui.close();
    }
}
