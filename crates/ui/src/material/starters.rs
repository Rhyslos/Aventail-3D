//! The starter graphs ([decision 0264](../../../../docs/decisions/0264-starter-graphs.md)):
//! ready-made graphs to open instead of an empty canvas. The canvas bar's
//! **Starter graphs** button (and the empty canvas, and the shelf in the
//! Material mode) lists them as cards with their pictures and what each
//! teaches; choosing one copies it into the project, where it changes
//! freely, as one undo step.
//!
//! The starters themselves are graph files shipped with the app
//! (`fileio::starter_graphs`).

use super::MaterialEditor;
use crate::text::tr;
use crate::theme;
use egui::{Context, RichText, Sense, TextureHandle, TextureOptions, Ui, vec2};
use fileio::starter_graphs::{STARTERS, Starter};
use scene::{Command, Document};
use std::collections::{BTreeSet, HashMap};

/// The button (canvas bar) and window title.
pub(crate) const STARTERS_LABEL: &str = crate::text!("Starter graphs");
/// The empty canvas's button.
pub(crate) const START_FROM_LABEL: &str = crate::text!("Start from a starter graph");

/// The pictures on the starters' cards, as textures, by key.
#[derive(Default)]
pub(crate) struct StarterTextures(HashMap<&'static str, Option<TextureHandle>>);

impl std::fmt::Debug for StarterTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StarterTextures")
            .field(&self.0.len())
            .finish()
    }
}

impl MaterialEditor {
    /// Copies `starter` into the project as a new graph (named after it,
    /// with a number when the name is taken) with the own boxes it uses,
    /// shows it, and says so. One undo step.
    pub(crate) fn open_starter(
        &mut self,
        document: &mut Document,
        starter: &Starter,
        note: &mut Option<String>,
    ) {
        let file = match starter.load() {
            Ok(file) => file,
            Err(why) => {
                *note = Some(crate::trf!("The starter graph can't be read: {}", why));
                return;
            }
        };
        let store = document.scene().graphs();
        let Some(file) = file.fitted_to(|id| store.own(id).cloned()) else {
            *note = Some(tr("The starter graph's own boxes don't fit this project.").to_string());
            return;
        };
        let names: BTreeSet<&str> = store.iter().map(|(_, g)| g.name()).collect();
        let base = tr(starter.name).to_string();
        let name = std::iter::once(base.clone())
            .chain((2..).map(|n| format!("{base} {n}")))
            .find(|n| !names.contains(n.as_str()))
            .unwrap_or(base);
        let mut graph = file.graph;
        let registry = matgraph::own::registry_with(&file.own);
        if let Err(why) = graph.apply(&registry, &matgraph::GraphEdit::Rename(name.clone())) {
            *note = Some(why.reason);
            return;
        }
        let id = store.next_id();
        let commands = file
            .own
            .iter()
            .filter(|o| store.own(&o.id).is_none())
            .map(|o| Command::SetOwnBox {
                id: o.id.clone(),
                own: Some((**o).clone()),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(Command::AddGraph(graph)));
        let label = crate::trf!("Open the starter graph {}", name);
        let key = self.next_key();
        match document.execute_all_labelled(commands, &label, key) {
            Ok(()) => {
                self.show_graph(Some(id));
                self.canvas.read_along = true;
                self.starters_open = false;
                *note = Some(crate::trf!(
                    "Opened a copy of {}: it is yours to change. The notes in its frames say \
                     what each step does.",
                    name
                ));
            }
            Err(why) => *note = Some(why.to_string()),
        }
    }

    /// Starter `starter`'s card picture as a texture (`None` if it can't
    /// be read).
    fn starter_texture(&mut self, ctx: &Context, starter: &Starter) -> Option<egui::TextureId> {
        self.starter_textures
            .0
            .entry(starter.key)
            .or_insert_with(|| {
                let read = starter.card_picture().ok()?;
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [read.width as usize, read.height as usize],
                    &read.rgba,
                );
                Some(ctx.load_texture(
                    format!("starter {}", starter.key),
                    image,
                    TextureOptions::LINEAR,
                ))
            })
            .as_ref()
            .map(TextureHandle::id)
    }

    /// The starters as small cards in a row, for the shelf: a click opens
    /// a copy.
    pub(crate) fn starter_shelf(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let mut chosen = None;
        egui::ScrollArea::horizontal()
            .id_salt("starter graphs shelf")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for starter in &STARTERS {
                        let texture = self.starter_texture(ui.ctx(), starter);
                        let name = tr(starter.name);
                        let label = crate::trf!("{} {}", tr("Starter graph"), name);
                        let response =
                            crate::shelf_cards::card(ui, name, label, false, |painter, rect| {
                                draw_card_picture(painter, rect, texture);
                            })
                            .on_hover_text(tr(starter.teaches));
                        if response.clicked() {
                            chosen = Some(starter);
                        }
                    }
                });
            });
        if let Some(starter) = chosen {
            self.open_starter(document, starter, note);
        }
    }

    /// The Starter graphs window, while it is open: a card for each with
    /// its picture, name and what it teaches.
    pub(crate) fn starters_window(
        &mut self,
        ctx: &Context,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        if !self.starters_open {
            return;
        }
        let mut open = true;
        let mut chosen = None;
        let rect = self.canvas.rect.or(self.canvas.last_rect);
        let mut window = egui::Window::new(tr(STARTERS_LABEL))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .max_height(rect.map_or(640.0, |r| r.height() - 20.0));
        if let Some(rect) = rect {
            window = window.default_pos(rect.center() - vec2(280.0, 260.0));
        }
        window.show(ctx, |ui| {
            crate::dialog::help(
                ui,
                tr(
                    "Choose one to open a copy of it in this project, to change as you like. \
                     Each is laid out step by step in frames, with a note in each frame \
                     saying what the step does and why.",
                ),
            );
            ui.add_space(6.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("starter graphs")
                    .num_columns(2)
                    .spacing(vec2(10.0, 10.0))
                    .show(ui, |ui| {
                        for (i, starter) in STARTERS.iter().enumerate() {
                            let texture = self.starter_texture(ui.ctx(), starter);
                            if big_card(ui, starter, texture).clicked() {
                                chosen = Some(starter);
                            }
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });
        if !open {
            self.starters_open = false;
        }
        if let Some(starter) = chosen {
            self.open_starter(document, starter, note);
        }
    }
}

/// A starter's card in the window: its picture, name and what it
/// teaches, all one button.
fn big_card(ui: &mut Ui, starter: &Starter, texture: Option<egui::TextureId>) -> egui::Response {
    const SIDE: f32 = 84.0;
    let size = vec2(260.0, SIDE + 8.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let name = tr(starter.name);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            crate::trf!("Open a copy of {}", name),
        )
    });
    let painter = ui.painter_at(rect);
    if response.hovered() {
        painter.rect_filled(rect, 6.0, theme::TEXT_WEAK.gamma_multiply(0.12));
    }
    let picture = egui::Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(SIDE, SIDE));
    draw_card_picture(&painter, picture, texture);
    painter.rect_stroke(
        picture,
        4.0,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Outside,
    );
    let text = egui::Rect::from_min_max(
        egui::pos2(picture.right() + 10.0, rect.top() + 4.0),
        rect.max - vec2(4.0, 4.0),
    );
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    inner.label(RichText::new(name).color(theme::TEXT_STRONG).strong());
    inner.add(
        egui::Label::new(
            RichText::new(tr(starter.teaches))
                .font(crate::dialog::help_font())
                .color(theme::TEXT_WEAK),
        )
        .wrap()
        .selectable(false),
    );
    response
}

/// A card's picture, or the graph mark when it can't be read.
fn draw_card_picture(painter: &egui::Painter, rect: egui::Rect, texture: Option<egui::TextureId>) {
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
