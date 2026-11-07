//! The shelf in the Material mode (decision 0263): your own boxes, the
//! project's and your library's, as cards with their pictures. A click
//! places one on the canvas; a right-click offers the rest (open it, keep
//! it in your library, export it as a box file, delete or remove it).
//! The project's boxes are saved with the project; your library is kept
//! with the app's settings, so it is there in every project.

use super::MaterialEditor;
use super::own_boxes::BoxFileAsk;
use crate::shelf_cards::card;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use fileio::own_box_file::BoxFile;
use scene::{Command, Document};

/// The button bringing a box file into your library.
pub(crate) const BRING_IN: &str = crate::text!("Bring in a box file…");

/// What a card asked for.
enum Asked {
    Place(BoxFile),
    Open(String),
    Keep(String),
    Export(BoxFile),
    Delete(String),
    Remove(usize),
}

impl MaterialEditor {
    /// The shelf's own boxes.
    pub(crate) fn shelf(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let mut asked = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Your own boxes")).color(theme::TEXT_STRONG));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(tr(BRING_IN))
                    .on_hover_text(tr("Add a box someone shared (a box file) to your library"))
                    .clicked()
                {
                    self.box_file_asked = Some(BoxFileAsk::Load);
                }
            });
        });
        let store = document.scene().graphs();
        let project: Vec<_> = store.own_boxes().cloned().collect();
        ui.label(
            RichText::new(tr("In this project"))
                .color(theme::TEXT_WEAK)
                .small(),
        );
        if project.is_empty() {
            crate::dialog::help(
                ui,
                tr("None yet: select boxes on the canvas and press Alt + G to make one."),
            );
        } else {
            egui::ScrollArea::horizontal()
                .id_salt("own boxes project")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for own in &project {
                            let texture = self.own_texture(ui.ctx(), own);
                            let label = crate::trf!("{} {}", tr("Own box"), own.name);
                            let response = card(ui, &own.name, label, false, |painter, rect| {
                                draw_picture(painter, rect, texture);
                            })
                            .on_hover_text(&own.about);
                            let file = || BoxFile::of(own, |i| store.own(i).cloned());
                            if response.clicked() {
                                asked = file().map(Asked::Place);
                            }
                            response.context_menu(|ui| {
                                if ui.button(tr("Place it")).clicked() {
                                    asked = file().map(Asked::Place);
                                }
                                if ui.button(tr(super::own_rows::OPEN_IT)).clicked() {
                                    asked = Some(Asked::Open(own.id.clone()));
                                }
                                if ui.button(tr(super::own_rows::SAVE_TO_LIBRARY)).clicked() {
                                    asked = Some(Asked::Keep(own.id.clone()));
                                }
                                if ui.button(tr(super::own_rows::EXPORT_BOX)).clicked() {
                                    asked = file().map(Asked::Export);
                                }
                                if ui
                                    .button(tr("Delete from the project"))
                                    .on_hover_text(tr(
                                        "Only when no graph uses it; undo brings it back",
                                    ))
                                    .clicked()
                                {
                                    asked = Some(Asked::Delete(own.id.clone()));
                                }
                            });
                        }
                    });
                });
        }
        ui.label(
            RichText::new(tr("Your library"))
                .color(theme::TEXT_WEAK)
                .small(),
        );
        if self.library.is_empty() {
            crate::dialog::help(
                ui,
                tr(
                    "Empty: right-click one of this project's boxes and keep it, or bring in a \
                    box file. Your library is there in every project.",
                ),
            );
        } else {
            let library = self.library.clone();
            egui::ScrollArea::horizontal()
                .id_salt("own boxes library")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, file) in library.iter().enumerate() {
                            let own = &file.main;
                            let texture = self.own_texture(ui.ctx(), own);
                            let label = crate::trf!("{} {}", tr("Library box"), own.name);
                            let response = card(ui, &own.name, label, false, |painter, rect| {
                                draw_picture(painter, rect, texture);
                            })
                            .on_hover_text(&own.about);
                            if response.clicked() {
                                asked = Some(Asked::Place(file.clone()));
                            }
                            response.context_menu(|ui| {
                                if ui.button(tr("Place it")).clicked() {
                                    asked = Some(Asked::Place(file.clone()));
                                }
                                if ui.button(tr(super::own_rows::EXPORT_BOX)).clicked() {
                                    asked = Some(Asked::Export(file.clone()));
                                }
                                if ui.button(tr("Remove from your library")).clicked() {
                                    asked = Some(Asked::Remove(i));
                                }
                            });
                        }
                    });
                });
        }
        match asked {
            Some(Asked::Place(file)) => self.place_own(document, &file, None, note),
            Some(Asked::Open(id)) => self.open_inside(id, None),
            Some(Asked::Keep(id)) => self.save_to_library(document, &id, note),
            Some(Asked::Export(file)) => self.box_file_asked = Some(BoxFileAsk::Save(file)),
            Some(Asked::Delete(id)) => {
                let command = Command::SetOwnBox { id, own: None };
                match document.execute(command) {
                    Ok(()) => {
                        *note = Some(
                            tr("Deleted it from the project. Undo brings it back.").to_string(),
                        )
                    }
                    Err(why) => *note = Some(why.to_string()),
                }
            }
            Some(Asked::Remove(i)) if i < self.library.len() => {
                let gone = self.library.remove(i);
                *note = Some(crate::trf!("Took {} out of your library.", gone.main.name));
            }
            Some(Asked::Remove(_)) | None => {}
        }
    }
}

/// An own box's picture on its card, or the graph mark when it has none.
fn draw_picture(painter: &egui::Painter, rect: egui::Rect, texture: Option<egui::TextureId>) {
    match texture {
        Some(texture) => {
            painter.image(
                texture,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => crate::shelf_cards::graph_mark(painter, rect),
    }
}
