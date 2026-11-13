//! Finding boxes in the graph (milestone 17, decision 0279): the search
//! field on the canvas bar, the list of matches under it, next and
//! previous, and jumping to a match (it is selected and the canvas frames
//! it). The matching itself is `matgraph`'s `Graph::find`: box names
//! (your own boxes' too), the choice a box is set to, the pieces drawn on
//! a Shape box, and comment frames' titles and notes.
//!
//! No key opens it (the project lead can choose one later): the field is
//! always on the bar, and "Find a box in the graph…" in the command
//! search puts the cursor in it. Enter in the field goes to the next
//! match, Esc empties it.

use super::MaterialEditor;
use super::look;
use crate::text::tr;
use crate::theme;
use egui::{Context, Id, Order, Rect, RichText, Ui};
use matgraph::{BoxId, Found, FoundBy, FoundThing, FrameId, Graph, Registry};
use scene::Document;
use std::collections::BTreeSet;
use std::sync::Arc;

/// The search field's words and accessible name.
pub(crate) const FIND_LABEL: &str = crate::text!("Find in the graph");
/// The button going to the previous match.
pub(crate) const PREVIOUS_LABEL: &str = crate::text!("Previous match");
/// The button going to the next match.
pub(crate) const NEXT_LABEL: &str = crate::text!("Next match");
/// The box menu's entry finding the boxes of the same kind.
pub(crate) const FIND_LIKE_LABEL: &str = crate::text!("Find others like it");
/// The list of matches' accessible name.
pub(crate) const MATCHES_LABEL: &str = crate::text!("Matches in the graph");

/// The field's width on the bar.
const FIELD_WIDTH: f32 = 170.0;
/// The list's width and most height.
const LIST_WIDTH: f32 = 300.0;
const LIST_HEIGHT: f32 = 280.0;

/// The search, kept while the graph is shown.
#[derive(Debug, Default)]
pub(crate) struct Finding {
    /// What is typed.
    pub(crate) query: String,
    /// The matches, in reading order.
    pub(crate) found: Vec<Found>,
    /// The match gone to last.
    pub(crate) at: Option<usize>,
    /// What the matches were found in: the words, the graph and the
    /// kinds of box (kept, so a change to any finds them again).
    of: Option<(String, Arc<Graph>, Arc<Registry>)>,
    /// Put the cursor in the field next frame.
    pub(crate) focus: bool,
    /// Whether the list of matches is shown under the field.
    pub(crate) list: bool,
    /// Where the field was drawn, for the list under it.
    field: Option<Rect>,
    /// Whether the field had the cursor last frame.
    focused: bool,
    /// Esc was pressed in the field: empty it.
    escaped: bool,
}

impl Finding {
    /// Takes Esc pressed in the field before the rest of the app sees it
    /// (egui lets go of the field first, so the app wouldn't know it was
    /// typing): it empties the field. True if it was taken.
    pub(crate) fn take_escape(&mut self, ctx: &Context) -> bool {
        if !self.focused {
            return false;
        }
        let taken = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if taken {
            self.escaped = true;
            self.focused = false;
        }
        taken
    }

    /// Finds the matches again if the words, the graph or the kinds of
    /// box changed; true when the words did (go to the first then).
    pub(crate) fn refresh(&mut self, graph: &Arc<Graph>, registry: &Arc<Registry>) -> bool {
        let same = self.of.as_ref().is_some_and(|(q, g, r)| {
            *q == self.query && Arc::ptr_eq(g, graph) && Arc::ptr_eq(r, registry)
        });
        if same {
            return false;
        }
        let words_changed = self.of.as_ref().is_none_or(|(q, ..)| *q != self.query);
        let was = self.current();
        self.found = graph.find(registry, &self.query);
        self.of = Some((self.query.clone(), graph.clone(), registry.clone()));
        // The same match stays chosen when the graph changes under it.
        self.at = if words_changed || self.found.is_empty() {
            None
        } else {
            was.and_then(|t| self.found.iter().position(|f| f.thing == t))
                .or(Some(0))
        };
        words_changed
    }

    /// The match gone to last.
    pub(crate) fn current(&self) -> Option<FoundThing> {
        self.found.get(self.at?).map(|f| f.thing)
    }

    /// The match `by` steps on from the last (wrapping round), or the
    /// first (last, going back) when none was gone to yet.
    pub(crate) fn stepped(&self, by: isize) -> Option<usize> {
        let n = self.found.len() as isize;
        if n == 0 {
            return None;
        }
        let next = match self.at {
            Some(at) => (at as isize + by).rem_euclid(n),
            None if by < 0 => n - 1,
            None => 0,
        };
        Some(next as usize)
    }

    /// The boxes and frames matching, to mark on the canvas and the
    /// minimap; none while nothing is typed.
    pub(crate) fn marked(&self) -> (BTreeSet<BoxId>, BTreeSet<FrameId>) {
        let mut boxes = BTreeSet::new();
        let mut frames = BTreeSet::new();
        if self.query.trim().is_empty() {
            return (boxes, frames);
        }
        for f in &self.found {
            match f.thing {
                FoundThing::Box(id) => {
                    boxes.insert(id);
                }
                FoundThing::Frame(id) => {
                    frames.insert(id);
                }
            }
        }
        (boxes, frames)
    }

    /// "2 of 5", "5 found" or "No match", for the bar.
    fn count(&self) -> String {
        match (self.found.len(), self.at) {
            (0, _) => tr("No match").to_string(),
            (n, Some(at)) => crate::trf!("{} of {}", at + 1, n),
            (n, None) => crate::trf!("{} found", n),
        }
    }
}

/// A button's accessible name, when it shows only a sign.
trait Labelled {
    fn labelled(self, name: &str) -> Self;
}

impl Labelled for egui::Response {
    fn labelled(self, name: &str) -> Self {
        let enabled = self.enabled();
        self.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
        self
    }
}

/// What a match is, for its line in the list.
fn kind_words(found: &Found) -> String {
    let short = found.detail.chars().count() <= 26;
    match found.by {
        FoundBy::Choice | FoundBy::Piece if short => found.detail.clone(),
        FoundBy::Choice | FoundBy::Piece => tr("Box").to_string(),
        FoundBy::Name => tr("Box").to_string(),
        FoundBy::OwnBox => tr("Your box").to_string(),
        FoundBy::Title => tr("Frame").to_string(),
        FoundBy::Note => tr("Frame's note").to_string(),
    }
}

impl MaterialEditor {
    /// The search field, its count and next / previous, on the canvas
    /// bar.
    pub(super) fn find_bar(&mut self, ui: &mut Ui, document: &Document) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = self
            .registry
            .clone()
            .unwrap_or_else(|| Arc::new(Registry::standard().clone()));
        let mut step = None;
        // Right to left: next, previous, the count, then the field.
        let typed = !self.canvas.finding.query.trim().is_empty();
        if typed {
            if ui
                .add_enabled(
                    !self.canvas.finding.found.is_empty(),
                    egui::Button::new("›").min_size(egui::vec2(22.0, 0.0)),
                )
                .on_hover_text(tr("The next match (Enter in the field)"))
                .labelled(tr(NEXT_LABEL))
                .clicked()
            {
                step = Some(1);
            }
            if ui
                .add_enabled(
                    !self.canvas.finding.found.is_empty(),
                    egui::Button::new("‹").min_size(egui::vec2(22.0, 0.0)),
                )
                .on_hover_text(tr("The previous match"))
                .labelled(tr(PREVIOUS_LABEL))
                .clicked()
            {
                step = Some(-1);
            }
            ui.label(
                RichText::new(self.canvas.finding.count())
                    .color(theme::TEXT_WEAK)
                    .small(),
            );
        }
        let finding = &mut self.canvas.finding;
        // Narrower when the bar is crowded, so it never covers the graph
        // buttons on its left.
        let width = FIELD_WIDTH.min(ui.available_width() - 16.0).max(48.0);
        let field = ui
            .add(
                egui::TextEdit::singleline(&mut finding.query)
                    .desired_width(width)
                    .hint_text(tr(FIND_LABEL)),
            )
            .on_hover_text(tr(
                "Find a box by its name, your own box's name, the choice it is set to, or a \
                 comment frame's title or note",
            ));
        field.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, tr(FIND_LABEL))
        });
        finding.field = Some(field.rect);
        if std::mem::take(&mut finding.focus) {
            field.request_focus();
            finding.list = true;
        }
        if field.gained_focus() || field.changed() {
            finding.list = true;
        }
        let (enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        finding.focused = field.has_focus();
        if std::mem::take(&mut finding.escaped) || (field.lost_focus() && escape) {
            finding.query.clear();
            finding.list = false;
        } else if field.lost_focus() && enter {
            // Enter goes on to the next match and keeps typing.
            field.request_focus();
            step = Some(1);
        }
        let words_changed = finding.refresh(&graph, &registry);
        if words_changed && !finding.found.is_empty() {
            self.jump_to(0, &graph);
        } else if let Some(by) = step
            && let Some(next) = self.canvas.finding.stepped(by)
        {
            self.jump_to(next, &graph);
        }
    }

    /// The list of matches under the field, while it is open: a click
    /// goes to one. A press anywhere else closes it.
    pub(super) fn find_list(&mut self, ctx: &Context, graph: &Arc<Graph>) {
        let finding = &self.canvas.finding;
        let Some(field) = finding.field else {
            return;
        };
        if !finding.list || finding.query.trim().is_empty() {
            return;
        }
        let rows: Vec<(String, String, String)> = finding
            .found
            .iter()
            .map(|f| (f.name.clone(), kind_words(f), f.detail.clone()))
            .collect();
        let at = finding.at;
        let mut picked = None;
        let left = (field.right() - LIST_WIDTH).max(0.0);
        let shown = egui::Area::new(Id::new("material find list"))
            .order(Order::Foreground)
            .fixed_pos(egui::pos2(left, field.bottom() + 4.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(LIST_WIDTH);
                    let list = ui.label(
                        RichText::new(tr(MATCHES_LABEL))
                            .small()
                            .strong()
                            .color(theme::TEXT_WEAK),
                    );
                    list.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, tr(MATCHES_LABEL))
                    });
                    if rows.is_empty() {
                        crate::dialog::help(
                            ui,
                            tr(
                                "Nothing in this graph has those words. Box names, your own \
                                boxes' names, the choices boxes are set to and frames' titles \
                                and notes are searched.",
                            ),
                        );
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .max_height(LIST_HEIGHT)
                        .show(ui, |ui| {
                            for (i, (name, kind, detail)) in rows.iter().enumerate() {
                                let mut row =
                                    crate::dialog::drop_down_entry(ui, at == Some(i), name, kind);
                                if !detail.is_empty() {
                                    row = row.on_hover_text(detail);
                                }
                                if row.clicked() {
                                    picked = Some(i);
                                }
                            }
                            if rows.len() == matgraph::MAX_FOUND {
                                crate::dialog::help(
                                    ui,
                                    tr("Only the first matches are listed: type more words \
                                        to find fewer."),
                                );
                            }
                        });
                });
            });
        self.canvas.overlays.push(shown.response.rect);
        if let Some(i) = picked {
            self.jump_to(i, graph);
            self.canvas.finding.list = false;
            return;
        }
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !shown.response.rect.contains(p) && !field.contains(p))
        });
        if outside {
            self.canvas.finding.list = false;
        }
    }

    /// Goes to match `index`: selects it and frames it on the canvas.
    pub(crate) fn jump_to(&mut self, index: usize, graph: &Graph) {
        let Some(found) = self.canvas.finding.found.get(index).cloned() else {
            return;
        };
        self.canvas.finding.at = Some(index);
        self.canvas.search = None;
        self.canvas.note = None;
        self.canvas.menu = None;
        self.canvas.dot = None;
        let corners = match found.thing {
            FoundThing::Box(id) => {
                self.canvas.selected = [id].into();
                self.canvas.frame = None;
                look::box_rect(graph, self.registry(), id)
            }
            FoundThing::Frame(id) => {
                self.canvas.selected.clear();
                self.canvas.frame = Some(id);
                graph.frame(id).map(|f| (f.min, f.max))
            }
        };
        if let (Some((lo, hi)), Some(rect)) = (corners, self.canvas.rect.or(self.canvas.last_rect))
        {
            self.canvas.view.frame(rect, lo, hi);
        }
    }

    /// Puts the cursor in the search field (the command search's "Find a
    /// box in the graph…").
    pub(crate) fn start_finding(&mut self) {
        self.shape = None;
        self.canvas.finding.focus = true;
        self.canvas.finding.list = true;
    }

    /// Finds the boxes of the same kind as box `id` (its right-click
    /// menu), going to the first.
    pub(crate) fn find_like(&mut self, id: BoxId, document: &Document) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let Some(b) = graph.get(id) else {
            return;
        };
        let name = self
            .registry()
            .get(&b.kind)
            .map_or_else(|| b.kind.clone(), |t| t.info().name.show());
        let registry = self
            .registry
            .clone()
            .unwrap_or_else(|| Arc::new(Registry::standard().clone()));
        self.canvas.finding.query = name;
        self.canvas.finding.refresh(&graph, &registry);
        self.canvas.finding.list = true;
        let index = self
            .canvas
            .finding
            .found
            .iter()
            .position(|f| f.thing == FoundThing::Box(id))
            .unwrap_or(0);
        self.jump_to(index, &graph);
    }
}

/// Marks the matching boxes and frames on the canvas: a ring round each.
pub(super) fn draw_marks(
    painter: &egui::Painter,
    places: &super::canvas::Places,
    boxes: &BTreeSet<BoxId>,
    frames: &BTreeSet<FrameId>,
) {
    let stroke = egui::Stroke::new(2.0, look::FOUND);
    for b in places.boxes.iter().filter(|b| boxes.contains(&b.id)) {
        painter.rect_stroke(b.rect.expand(4.0), 7.0, stroke, egui::StrokeKind::Outside);
    }
    for f in places.frames.iter().filter(|f| frames.contains(&f.id)) {
        painter.rect_stroke(f.rect.expand(3.0), 8.0, stroke, egui::StrokeKind::Outside);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use matgraph::GraphEdit;

    fn graph() -> Arc<Graph> {
        let r = Registry::standard();
        let mut g = Graph::new("Stone");
        for (kind, x) in [("noise", 0.0), ("bricks", 200.0), ("noise", 400.0)] {
            g.apply(
                r,
                &GraphEdit::AddBox {
                    kind: kind.into(),
                    at: Vec2::new(x, 0.0),
                },
            )
            .unwrap();
        }
        Arc::new(g)
    }

    fn registry() -> Arc<Registry> {
        Arc::new(Registry::standard().clone())
    }

    #[test]
    fn next_and_previous_wrap_round_the_matches() {
        let (g, r) = (graph(), registry());
        let mut f = Finding {
            query: "noise".into(),
            ..Finding::default()
        };
        assert!(f.refresh(&g, &r));
        assert_eq!(f.found.len(), 2);
        assert_eq!(f.stepped(1), Some(0));
        assert_eq!(f.stepped(-1), Some(1));
        f.at = Some(1);
        assert_eq!(f.stepped(1), Some(0));
        f.at = Some(0);
        assert_eq!(f.stepped(-1), Some(1));
        assert_eq!(f.count(), "1 of 2");
        // Nothing found: nowhere to go.
        f.query = "zebra".into();
        assert!(f.refresh(&g, &r));
        assert_eq!(f.stepped(1), None);
        assert_eq!(f.count(), "No match");
    }

    #[test]
    fn the_same_match_stays_chosen_when_the_graph_changes() {
        let (g, r) = (graph(), registry());
        let mut f = Finding {
            query: "noise".into(),
            ..Finding::default()
        };
        f.refresh(&g, &r);
        f.at = Some(1);
        let second = f.current();
        // Nothing changed: nothing found again.
        assert!(!f.refresh(&g, &r));
        // A box added in front of it: still the same box.
        let mut changed = (*g).clone();
        changed
            .apply(
                Registry::standard(),
                &GraphEdit::AddBox {
                    kind: "noise".into(),
                    at: Vec2::new(-200.0, 0.0),
                },
            )
            .unwrap();
        assert!(!f.refresh(&Arc::new(changed), &r));
        assert_eq!(f.found.len(), 3);
        assert_eq!(f.current(), second);
        assert_eq!(f.at, Some(2));
    }

    #[test]
    fn only_typed_words_mark_boxes() {
        let (g, r) = (graph(), registry());
        let mut f = Finding::default();
        f.refresh(&g, &r);
        assert!(f.marked().0.is_empty());
        f.query = "bricks".into();
        f.refresh(&g, &r);
        assert_eq!(f.marked().0, [BoxId(2)].into());
    }
}
