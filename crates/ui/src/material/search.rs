//! The box search (the project lead's choice, decision 0256): **Space**
//! over the canvas, or a right-click on empty canvas, opens it at the
//! pointer; dropping a new wire on empty canvas opens it with only the
//! boxes that take (or give) what the wire carries. Boxes are listed in
//! the experts' working order (Make, Shape, Detail, Colour, Finish), with
//! the ones that usually come next first. Typing narrows the list, the
//! arrow keys move, Enter or a click adds the box, Esc closes.

use super::MaterialEditor;
use super::drags::WireEnd;
use crate::text::tr;
use crate::theme;
use egui::{Context, Id, Key, Modifiers, Order, Rect, RichText};
use glam::Vec2;
use matgraph::{BoxInfo, Graph, Group, Registry, ValueKind};
use scene::Document;

/// How tall the list may grow before it scrolls.
const LIST_HEIGHT: f32 = 380.0;
/// The search's width.
const WIDTH: f32 = 280.0;

/// The box search, while open.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Search {
    /// Where the new box goes, on the canvas.
    pub(crate) at: Vec2,
    /// What is typed.
    pub(crate) text: String,
    /// The wire dropped on empty canvas, if that opened it: the new box
    /// is connected to it.
    pub(crate) wire: Option<WireEnd>,
    /// The highlighted entry.
    pub(crate) chosen: usize,
    /// The kind whose "usually comes next" boxes are listed first.
    pub(crate) after: Option<String>,
    /// Whether it opened this frame (the field takes the keyboard).
    pub(crate) first: bool,
}

/// One box kind in the list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) about: String,
    /// What it makes, in a word ("height").
    pub(crate) makes: String,
}

/// Whether a wire carrying `from` can go into a plug taking `to` (as it
/// is, or converted).
fn fits(from: ValueKind, to: ValueKind) -> bool {
    matgraph::kind::fit(from, to).is_ok()
}

/// The list's groups, each a heading and its boxes, in order.
pub(crate) fn entries(search: &Search, registry: &Registry) -> Vec<(String, Vec<Entry>)> {
    let words: Vec<String> = search
        .text
        .to_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let wanted = |info: &BoxInfo| {
        let fits = match &search.wire {
            None => true,
            Some(WireEnd::Output(_, kind)) => info.inputs.iter().any(|i| fits(*kind, i.kind)),
            Some(WireEnd::Input(_, kind)) => info.outputs.iter().any(|o| fits(o.kind, *kind)),
        };
        let text = format!("{} {}", info.name.show(), info.about.show()).to_lowercase();
        fits && words.iter().all(|w| text.contains(w.as_str()))
    };
    let entry = |info: &BoxInfo| Entry {
        kind: info.id.to_string(),
        name: info.name.show(),
        about: info.about.show(),
        makes: info
            .outputs
            .first()
            .map_or_else(String::new, |o| o.kind.name().to_string()),
    };
    let mut out = Vec::new();
    let next: Vec<Entry> = search
        .after
        .as_deref()
        .and_then(|kind| registry.get(kind))
        .map(|t| {
            t.info()
                .comes_next
                .iter()
                .filter_map(|k| registry.get(k))
                .map(|t| t.info())
                .filter(|i| wanted(i))
                .map(entry)
                .collect()
        })
        .unwrap_or_default();
    if !next.is_empty() {
        out.push((tr(super::COMES_NEXT).to_string(), next));
    }
    for group in Group::ALL {
        let boxes: Vec<Entry> = registry
            .iter()
            .map(|t| t.info())
            .filter(|i| i.group == group && wanted(i))
            .map(entry)
            .collect();
        if !boxes.is_empty() {
            out.push((group.name().to_string(), boxes));
        }
    }
    out
}

impl MaterialEditor {
    /// Opens the box search for a box at `at` on the canvas, for the wire
    /// `wire` if one was dropped.
    pub(crate) fn open_search(&mut self, at: Vec2, wire: Option<WireEnd>, document: &Document) {
        let graph = self.current(document);
        let kind_of = |id| graph.and_then(|g| g.get(id)).map(|b| b.kind.clone());
        let after = match &wire {
            Some(WireEnd::Output(plug, _)) => kind_of(plug.node),
            Some(WireEnd::Input(..)) => None,
            None if self.canvas.selected.len() == 1 => self
                .canvas
                .selected
                .iter()
                .next()
                .and_then(|id| kind_of(*id)),
            None => None,
        };
        self.canvas.note = None;
        self.canvas.search = Some(Search {
            at,
            text: String::new(),
            wire,
            chosen: 0,
            after,
            first: true,
        });
    }

    /// Draws the search while open, and adds the box picked.
    pub(super) fn search_overlay(
        &mut self,
        ctx: &Context,
        rect: Rect,
        graph: &Graph,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(mut search) = self.canvas.search.take() else {
            return;
        };
        let groups = entries(&search, self.registry());
        let flat: Vec<&Entry> = groups.iter().flat_map(|(_, e)| e.iter()).collect();
        let (down, up, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if down {
            search.chosen = (search.chosen + 1).min(flat.len().saturating_sub(1));
        }
        if up {
            search.chosen = search.chosen.saturating_sub(1);
        }
        let mut picked = enter
            .then(|| flat.get(search.chosen).map(|e| e.kind.clone()))
            .flatten();
        // Your own boxes show their pictures (decision 0263).
        let registry = self.registry.clone();
        let pictures: std::collections::HashMap<String, egui::TextureId> = flat
            .iter()
            .filter_map(|e| {
                let own = registry.as_ref()?.get(&e.kind)?.own()?.clone();
                Some((e.kind.clone(), self.own_texture(ctx, &own)?))
            })
            .collect();
        let at = super::note::kept_inside(rect, self.canvas.view.on_screen(rect, search.at));
        let shown = egui::Area::new(Id::new("material box search"))
            .order(Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(WIDTH);
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut search.text)
                            .desired_width(WIDTH)
                            .hint_text(tr(super::BOX_SEARCH_LABEL)),
                    );
                    field.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::TextEdit,
                            true,
                            tr(super::BOX_SEARCH_LABEL),
                        )
                    });
                    if search.first {
                        field.request_focus();
                        search.first = false;
                    }
                    if field.changed() {
                        search.chosen = 0;
                    }
                    if let Some(WireEnd::Output(_, kind) | WireEnd::Input(_, kind)) = &search.wire {
                        crate::dialog::help(
                            ui,
                            crate::trf!("Boxes that fit a {} wire", kind.name()),
                        );
                    }
                    egui::ScrollArea::vertical()
                        .max_height(LIST_HEIGHT)
                        .show(ui, |ui| {
                            let mut index = 0;
                            for (heading, boxes) in &groups {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(heading)
                                        .small()
                                        .strong()
                                        .color(theme::TEXT_WEAK),
                                );
                                for entry in boxes {
                                    let chosen = index == search.chosen;
                                    let line = |ui: &mut egui::Ui| {
                                        crate::dialog::drop_down_entry(
                                            ui,
                                            chosen,
                                            &entry.name,
                                            &entry.makes,
                                        )
                                    };
                                    let row = match pictures.get(&entry.kind) {
                                        Some(picture) => {
                                            ui.horizontal(|ui| {
                                                ui.image((*picture, egui::vec2(18.0, 18.0)));
                                                line(ui)
                                            })
                                            .inner
                                        }
                                        None => line(ui),
                                    }
                                    .on_hover_text(&entry.about);
                                    if chosen && (down || up) {
                                        row.scroll_to_me(None);
                                    }
                                    if row.clicked() {
                                        picked = Some(entry.kind.clone());
                                    }
                                    index += 1;
                                }
                            }
                            if flat.is_empty() {
                                crate::dialog::help(ui, tr("No box matches."));
                            }
                        });
                });
            });
        self.canvas.overlays.push(shown.response.rect);
        // A press outside closes it.
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !shown.response.rect.contains(p))
        });
        if let Some(kind) = picked {
            self.add_from_search(document, &kind, &search, rect, note);
        } else if !escape && !outside {
            self.canvas.search = Some(search);
        }
        let _ = graph;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::{BoxId, PlugRef};

    fn search(text: &str, wire: Option<WireEnd>, after: Option<&str>) -> Search {
        Search {
            at: Vec2::ZERO,
            text: text.into(),
            wire,
            chosen: 0,
            after: after.map(str::to_string),
            first: false,
        }
    }

    fn kinds(groups: &[(String, Vec<Entry>)]) -> Vec<String> {
        groups
            .iter()
            .flat_map(|(_, e)| e.iter().map(|e| e.kind.clone()))
            .collect()
    }

    #[test]
    fn boxes_are_listed_in_the_working_order() {
        let groups = entries(&search("", None, None), Registry::standard());
        let headings: Vec<&str> = groups.iter().map(|(h, _)| h.as_str()).collect();
        assert_eq!(
            headings,
            ["Make", "From the model", "Shape", "Colour", "Finish"]
        );
        assert_eq!(kinds(&groups).len(), Registry::standard().iter().count());
    }

    #[test]
    fn what_usually_comes_next_is_first() {
        let groups = entries(&search("", None, Some("noise")), Registry::standard());
        assert_eq!(groups[0].0, "Usually comes next");
        assert_eq!(groups[0].1[0].kind, "levels");
    }

    #[test]
    fn typing_narrows_the_list() {
        let groups = entries(&search("mater", None, None), Registry::standard());
        let found = kinds(&groups);
        assert!(found.contains(&"material_out".to_string()));
        assert!(!found.contains(&"noise".to_string()));
        assert!(entries(&search("zzzz", None, None), Registry::standard()).is_empty());
    }

    #[test]
    fn a_dropped_wire_lists_only_boxes_that_take_it() {
        let wire = WireEnd::Output(PlugRef::new(BoxId(1), "colour"), ValueKind::Colour);
        let found = kinds(&entries(
            &search("", Some(wire), None),
            Registry::standard(),
        ));
        // Material from height takes a colour; Noise takes nothing.
        assert!(found.contains(&"material_from_height".to_string()));
        assert!(!found.contains(&"noise".to_string()));
        let wanted = WireEnd::Input(PlugRef::new(BoxId(1), "in"), ValueKind::Material);
        let found = kinds(&entries(
            &search("", Some(wanted), None),
            Registry::standard(),
        ));
        // Transform and Repeat give a material too (decision 0270).
        assert_eq!(
            found,
            [
                "transform",
                "repeat",
                "material_from_height",
                "mix_materials"
            ]
        );
    }
}
