//! Your own boxes on the canvas ([decision 0263](../../../../docs/decisions/0263-your-own-boxes.md)):
//! making one from the selected boxes (Alt + G, with a window to name it
//! and choose its plugs and sliders), opening one back up (Shift + G),
//! going inside one (double-click, or Open it in Attributes) and coming
//! back out (the bar's way back, or Tab), and placing one from the
//! shelf.
//!
//! Inside an own box the canvas shows the boxes in it: every change
//! there is a `scene::Command::EditOwnBox`, so it changes every box
//! placed from it (they are linked), and undo takes it back. Its
//! pictures follow what is plugged into the box it was opened through.

use super::MaterialEditor;
use super::pictures::PictureJob;
use super::view::View;
use crate::text::tr;
use crate::theme;
use egui::{Context, RichText};
use fileio::own_box_file::BoxFile;
use glam::Vec2;
use matgraph::own::{Proposal, open_up, picture_of};
use matgraph::{BoxId, Evaluator, Graph, GraphEdit, GraphId, OwnBox, Size};
use scene::{Command, Document};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The window's title, and its accessible name.
pub(crate) const MAKING_TITLE: &str = crate::text!("Make your own box");
/// The button that makes it.
pub(crate) const MAKE_IT: &str = crate::text!("Make it");
/// The button that goes back out of an own box.
pub(crate) const BACK_OUT: &str = crate::text!("Back out");

/// One own box opened on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Inside {
    /// The own box's id.
    pub(crate) own: String,
    /// The placed box it was opened through, in the graph outside; `None`
    /// when opened from the shelf (nothing is plugged into it then).
    pub(crate) through: Option<BoxId>,
    /// The canvas's view and selection outside, to go back to.
    view: View,
    selected: BTreeSet<BoxId>,
}

/// Making your own box: the choices in the window.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Making {
    /// What it will be.
    pub(crate) proposal: Proposal,
    /// Why it couldn't be made last time "Make it" was pressed.
    pub(crate) why: Option<String>,
    /// Whether the name field still has to take the keyboard.
    first: bool,
}

/// A box file to write or read (the app asks where).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BoxFileAsk {
    /// Write this box file.
    Save(BoxFile),
    /// Read one into your library.
    Load,
}

/// Where the canvas's changes go: a project's graph, or the inside of
/// one of its own boxes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Target {
    Graph(GraphId),
    Own(String),
}

impl Target {
    /// The command making `edit` there.
    pub(crate) fn command(&self, edit: GraphEdit) -> Command {
        match self {
            Self::Graph(id) => Command::EditGraph { id: *id, edit },
            Self::Own(id) => Command::EditOwnBox {
                id: id.clone(),
                edit,
            },
        }
    }
}

/// `want`, or `want 2`, `want 3`… whichever isn't in `names`.
pub(crate) fn unused_name(want: &str, names: &BTreeSet<String>) -> String {
    if !names.contains(want) {
        return want.to_string();
    }
    (2..)
        .map(|n| format!("{want} {n}"))
        .find(|n| !names.contains(n))
        .unwrap_or_default()
}

impl MaterialEditor {
    /// Where the canvas's changes go now.
    pub(crate) fn target(&self) -> Option<Target> {
        match self.inside.last() {
            Some(inside) => Some(Target::Own(inside.own.clone())),
            None => self.graph.map(Target::Graph),
        }
    }

    /// The project's registry as `document` has it now.
    pub(crate) fn registry_of(&self, document: &Document) -> Arc<matgraph::Registry> {
        document.scene().graphs().registry().clone()
    }

    /// What to work out for the pictures of `graph`, the graph shown:
    /// inside an own box opened through a placed box, the graph it is
    /// placed in and the way down to it.
    pub(crate) fn picture_job(&self, document: &Document, graph: Arc<Graph>) -> PictureJob {
        let store = document.scene().graphs();
        let registry = store.registry().clone();
        // From the last box opened from the shelf (nothing plugged in),
        // or the project's graph.
        let start = self.inside.iter().rposition(|i| i.through.is_none());
        let top = match start {
            Some(at) => store.own(&self.inside[at].own).map(|o| o.inner.clone()),
            None => self.graph.and_then(|id| store.get(id).cloned()),
        };
        let from = start.map_or(0, |at| at + 1);
        let path: Vec<BoxId> = self.inside[from..]
            .iter()
            .filter_map(|i| i.through)
            .collect();
        let top = top.filter(|_| !path.is_empty()).map(|t| (t, path));
        PictureJob {
            graph,
            registry,
            top,
            time: None,
        }
    }

    /// Opens own box `own` on the canvas (through placed box `through`,
    /// if any).
    pub(crate) fn open_inside(&mut self, own: String, through: Option<BoxId>) {
        self.inside.push(Inside {
            own,
            through,
            view: self.canvas.view,
            selected: std::mem::take(&mut self.canvas.selected),
        });
        self.canvas.view = View::default();
        self.canvas.finding = super::finding::Finding::default();
        self.canvas.framed = None;
        self.canvas.frame = None;
        self.canvas.looking_at = None;
        self.canvas.search = None;
        self.canvas.note = None;
        self.making = None;
        self.shape = None;
    }

    /// Comes `levels` own boxes back out.
    pub(crate) fn leave_inside(&mut self, levels: usize) {
        for _ in 0..levels {
            let Some(inside) = self.inside.pop() else {
                break;
            };
            self.canvas.view = inside.view;
            self.canvas.finding = super::finding::Finding::default();
            self.canvas.selected = inside.selected;
            self.canvas.frame = None;
            self.canvas.looking_at = None;
            self.canvas.search = None;
            self.canvas.note = None;
        }
        // Keep the view as it was outside rather than framing again.
        self.canvas.framed = self.graph;
        self.making = None;
        self.shape = None;
    }

    /// Opens box `id` of the graph shown (double-click): the shape canvas
    /// of a Shape box, or the inside of one of your own; says why not
    /// for an own box the project no longer holds.
    pub(crate) fn open_box(&mut self, document: &Document, id: BoxId, note: &mut Option<String>) {
        let Some(kind) = self
            .current(document)
            .and_then(|g| g.get(id))
            .map(|b| b.kind.clone())
        else {
            return;
        };
        if self.is_drawn(document, id) {
            self.open_shape_canvas(id);
        } else if document.scene().graphs().own(&kind).is_some() {
            self.open_inside(kind, Some(id));
        } else if matgraph::own::is_own(&kind) {
            *note = Some(
                tr("This box of your own isn't in the project any more, so it can't be opened.")
                    .to_string(),
            );
        }
    }

    /// Alt + G: opens the window making the selected boxes into your own
    /// box, or says why they can't be.
    pub(crate) fn start_making(&mut self, document: &Document, note: &mut Option<String>) {
        let Some(graph) = self.current(document) else {
            return;
        };
        let registry = self.registry_of(document);
        match Proposal::of(graph, &registry, &self.canvas.selected) {
            Ok(mut proposal) => {
                let names: BTreeSet<String> = document
                    .scene()
                    .graphs()
                    .own_boxes()
                    .map(|o| o.name.clone())
                    .collect();
                proposal.name = unused_name(tr("My box"), &names);
                self.making = Some(Making {
                    proposal,
                    why: None,
                    first: true,
                });
            }
            Err(why) => {
                let rect = self.canvas.rect.or(self.canvas.last_rect);
                match rect {
                    Some(rect) => {
                        let context = super::note::FixContext::default();
                        self.show_refusal(why, rect.center(), rect, context, note);
                    }
                    None => *note = Some(why.reason),
                }
            }
        }
    }

    /// Makes the box the window describes: one undo step adding it and
    /// putting one of it in place of the selected boxes. `false` (with
    /// why in the window) when it can't be made.
    pub(crate) fn make_own_box(
        &mut self,
        document: &mut Document,
        note: &mut Option<String>,
    ) -> bool {
        let Some(making) = self.making.clone() else {
            return false;
        };
        let Some(graph) = self.current(document).cloned() else {
            return false;
        };
        let registry = self.registry_of(document);
        let store = document.scene().graphs();
        let seed = format!(
            "{} {} {:?}",
            making.proposal.name,
            graph.name(),
            making.proposal.boxes
        );
        let id = matgraph::own::fresh_id(&seed, |id| store.own(id).is_some());
        // Its picture: what its first output gives now.
        let picture = making
            .proposal
            .outputs
            .iter()
            .find(|o| o.keep)
            .and_then(|o| {
                let shown = self.pictures.value(o.from.node).cloned();
                let at = graph
                    .get(o.from.node)
                    .and_then(|b| registry.get(&b.kind))
                    .and_then(|k| k.info().outputs.iter().position(|p| p.key == o.from.plug));
                let first = at == Some(0);
                shown.filter(|_| first).or_else(|| {
                    let evaluation = Evaluator::with_threads(2).run(
                        &registry,
                        &graph,
                        Size::nearest(64),
                        &[o.from.node],
                    );
                    let result = evaluation.outcome(o.from.node)?.as_ref().ok()?;
                    result.outputs.get(at?).cloned().flatten()
                })
            })
            .map(|value| picture_of(&value));
        let made = making.proposal.make(&graph, &registry, id, picture);
        let (own, edits) = match made {
            Ok(made) => made,
            Err(why) => {
                if let Some(m) = &mut self.making {
                    m.why = Some(why.reason);
                }
                return false;
            }
        };
        let placed = BoxId(graph.next_id());
        let name = own.name.clone();
        let count = making.proposal.boxes.len();
        match self.run_with(document, vec![own], edits, tr(MAKING_TITLE), None, true) {
            Ok(()) => {
                self.making = None;
                self.canvas.selected = [placed].into();
                *note = Some(crate::trf!(
                    "Made {} boxes into your own box {}. Double-click it to see inside; \
                     Shift + G opens it back up; it is on the shelf to place again.",
                    count,
                    name
                ));
                true
            }
            Err(why) => {
                if let Some(m) = &mut self.making {
                    m.why = Some(why.reason);
                }
                false
            }
        }
    }

    /// The Make your own box window, while open.
    pub(crate) fn making_window(
        &mut self,
        ctx: &Context,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(mut making) = self.making.take() else {
            return;
        };
        let mut make = false;
        let mut cancel = false;
        let rect = self.canvas.rect.or(self.canvas.last_rect);
        let mut window = egui::Window::new(tr(MAKING_TITLE))
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .max_height(rect.map_or(600.0, |r| r.height() - 20.0));
        if let Some(rect) = rect {
            window = window.default_pos(rect.center() - egui::vec2(180.0, 200.0));
        }
        let shown = window.show(ctx, |ui| {
            crate::dialog::help(
                ui,
                tr(
                    "The selected boxes become one box. Wires coming in become its inputs, \
                     wires going out its outputs; tick the settings to show on it as sliders.",
                ),
            );
            ui.add_space(4.0);
            let p = &mut making.proposal;
            let field =
                crate::dialog::row(ui, tr("Name"), |ui| ui.text_edit_singleline(&mut p.name));
            if std::mem::take(&mut making.first) {
                field.request_focus();
            }
            ui.label(RichText::new(tr(super::WHAT_IT_DOES)).color(theme::TEXT_WEAK));
            ui.add(
                egui::TextEdit::multiline(&mut p.about)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text(tr(
                        "In your own words, for later and for whoever you share it with",
                    )),
            );
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    crate::attributes::section(ui, tr("Inputs"));
                    if p.inputs.is_empty() {
                        crate::dialog::help(ui, tr("No wires come into the selection: no inputs."));
                    }
                    for (i, input) in p.inputs.iter_mut().enumerate() {
                        ui.push_id(("input", i), |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut input.keep, "").on_hover_text(tr(
                                    "Keep this input (left out, nothing comes in there)",
                                ));
                                ui.add_enabled(
                                    input.keep,
                                    egui::TextEdit::singleline(&mut input.name)
                                        .desired_width(140.0),
                                );
                                ui.label(
                                    RichText::new(input.kind.name())
                                        .color(super::look::kind_colour(input.kind)),
                                );
                            });
                        });
                    }
                    crate::attributes::section(ui, tr("Outputs"));
                    for (i, output) in p.outputs.iter_mut().enumerate() {
                        ui.push_id(("output", i), |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut output.keep, "").on_hover_text(tr(
                                    "Keep this output (left out, the wires from it go)",
                                ));
                                ui.add_enabled(
                                    output.keep,
                                    egui::TextEdit::singleline(&mut output.name)
                                        .desired_width(140.0),
                                );
                                ui.label(
                                    RichText::new(output.kind.name())
                                        .color(super::look::kind_colour(output.kind)),
                                );
                            });
                        });
                    }
                    crate::attributes::section(ui, tr("Sliders"));
                    if p.sliders.is_empty() {
                        crate::dialog::help(ui, tr("The selected boxes have no settings."));
                    }
                    let mut last_box = None;
                    for (i, slider) in p.sliders.iter_mut().enumerate() {
                        if last_box != Some(slider.node) {
                            last_box = Some(slider.node);
                            ui.label(
                                RichText::new(&slider.box_name)
                                    .color(theme::TEXT_WEAK)
                                    .small(),
                            );
                        }
                        ui.push_id(("slider", i), |ui| {
                            ui.horizontal(|ui| {
                                let tick = ui.checkbox(&mut slider.chosen, "");
                                tick.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Checkbox,
                                        true,
                                        slider.chosen,
                                        crate::trf!("{} {}", tr(SLIDER_SWITCH), slider.name),
                                    )
                                });
                                if slider.chosen {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut slider.name)
                                            .desired_width(160.0),
                                    );
                                } else {
                                    ui.label(&slider.name);
                                }
                            });
                        });
                    }
                });
            if let Some(why) = &making.why {
                ui.add_space(4.0);
                ui.add(egui::Label::new(RichText::new(why).color(super::look::REFUSED)).wrap());
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                make = crate::dialog::main_button(ui, tr(MAKE_IT)).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
        if let Some(shown) = shown {
            self.canvas.overlays.push(shown.response.rect);
        }
        if cancel {
            return;
        }
        self.making = Some(making);
        if make {
            self.make_own_box(document, note);
        }
    }

    /// Shift + G: opens the selected own boxes back up into the boxes
    /// inside them, one undo step; says why when none is one of yours.
    pub(crate) fn open_up_selected(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = self.registry_of(document);
        let chosen: Vec<BoxId> = self.canvas.selected.iter().copied().collect();
        if chosen.is_empty() {
            *note = Some(
                tr("Select one of your own boxes first, then press Shift + G to open it back up.")
                    .to_string(),
            );
            return;
        }
        let mut scratch = (*graph).clone();
        let mut edits = Vec::new();
        let mut boxes = Vec::new();
        for id in chosen {
            let opened = match open_up(&scratch, &registry, id) {
                Ok(opened) => opened,
                Err(why) => {
                    *note = Some(why.reason);
                    return;
                }
            };
            for edit in &opened.edits {
                if let Err(why) = scratch.apply(&registry, edit) {
                    *note = Some(why.reason);
                    return;
                }
            }
            edits.extend(opened.edits);
            boxes.extend(opened.boxes);
        }
        match self.run(document, edits, tr("Open your own box back up"), None, true) {
            Ok(()) => {
                self.canvas.selected = boxes.into_iter().collect();
                *note = Some(
                    tr("Opened it back up: its boxes are back in the graph. It stays on the shelf.")
                        .to_string(),
                );
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// Places own box `file.main` at `at` on the canvas (or its middle),
    /// adding it and the own boxes inside it to the project first if it
    /// hasn't them (from your library). One undo step.
    pub(crate) fn place_own(
        &mut self,
        document: &mut Document,
        file: &BoxFile,
        at: Option<Vec2>,
        note: &mut Option<String>,
    ) {
        if self.current(document).is_none() {
            *note = Some(tr("Make a material graph first.").to_string());
            return;
        }
        let store = document.scene().graphs();
        let missing: Vec<OwnBox> = file
            .boxes
            .iter()
            .filter(|o| store.own(&o.id).is_none())
            .map(|o| (**o).clone())
            .collect();
        let kept_own = store.own(&file.main.id).is_some_and(|o| **o != *file.main);
        let at = at.unwrap_or_else(|| self.canvas_middle())
            - Vec2::new(super::look::BOX_WIDTH / 2.0, super::look::HEADER);
        let new = self.current(document).map(|g| BoxId(g.next_id()));
        let edits = vec![GraphEdit::AddBox {
            kind: file.main.id.clone(),
            at,
        }];
        match self.run_with(
            document,
            missing,
            edits,
            tr("Place your own box"),
            None,
            true,
        ) {
            Ok(()) => {
                if let Some(new) = new {
                    self.canvas.selected = [new].into();
                }
                *note = Some(if kept_own {
                    crate::trf!(
                        "Placed {}. This project already has its own version of it (changed \
                         here), so that is the one placed.",
                        file.main.name
                    )
                } else {
                    crate::trf!("Placed {}.", file.main.name)
                });
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// The project's own box `id` with the own boxes inside it, as a box
    /// file holds it.
    pub(crate) fn box_file_of(&self, document: &Document, id: &str) -> Option<BoxFile> {
        let store = document.scene().graphs();
        let own = store.own(id)?;
        BoxFile::of(own, |i| store.own(i).cloned())
    }

    /// Keeps the project's own box `id` in your library (replacing the
    /// one there with the same id).
    pub(crate) fn save_to_library(
        &mut self,
        document: &Document,
        id: &str,
        note: &mut Option<String>,
    ) {
        let Some(file) = self.box_file_of(document, id) else {
            return;
        };
        let name = file.main.name.clone();
        match self.library.iter().position(|f| f.main.id == file.main.id) {
            Some(at) => self.library[at] = file,
            None if self.library.len() >= matgraph::own::MAX_OWN_BOXES => {
                *note = Some(tr("Your library is full; remove a box from it first.").to_string());
                return;
            }
            None => self.library.push(file),
        }
        *note = Some(crate::trf!(
            "Kept {} in your library: it is on the shelf in every project.",
            name
        ));
    }

    /// Takes the picture of the own box opened again: what its first
    /// output gives now.
    pub(crate) fn take_picture(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(inside) = self.inside.last() else {
            return;
        };
        let Some(own) = document.scene().graphs().own(&inside.own).cloned() else {
            return;
        };
        let value = own
            .outputs
            .iter()
            .filter_map(|o| o.from.as_ref())
            .find_map(|from| {
                let registry = self.registry();
                let at = own
                    .inner
                    .get(from.node)
                    .and_then(|b| registry.get(&b.kind))
                    .and_then(|k| k.info().outputs.iter().position(|p| p.key == from.plug))?;
                let result = self
                    .pictures
                    .shown()?
                    .evaluation
                    .outcome(from.node)?
                    .as_ref()
                    .ok()?
                    .clone();
                result.outputs.get(at).cloned().flatten()
            });
        let Some(value) = value else {
            *note = Some(
                tr("Its output has no picture yet: wait for the pictures, or fix what can't run.")
                    .to_string(),
            );
            return;
        };
        let mut changed = (*own).clone();
        changed.picture = Some(picture_of(&value));
        let command = Command::SetOwnBox {
            id: own.id.clone(),
            own: Some(changed),
        };
        match document.execute_labelled(command, tr("Take a new picture of your own box"), None) {
            Ok(()) => *note = Some(tr("Took a new picture of it.").to_string()),
            Err(why) => *note = Some(why.to_string()),
        }
    }
}

/// The start of each slider switch's accessible name ("Slider: Darkest").
pub(crate) const SLIDER_SWITCH: &str = crate::text!("Slider:");
