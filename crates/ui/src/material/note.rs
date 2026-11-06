//! "Why it won't work" (the project lead's wish after the mockups,
//! decision 0258): when the engine refuses something on the canvas, a
//! note by the pointer says why in plain words and offers the ways to
//! make it work as buttons. Nothing changes until one is picked; the
//! status bar repeats the reason. While a wire is dragged over a plug that
//! won't take it, the plug turns red and the note follows the pointer;
//! letting go cancels the wire and leaves the note, so a fix can still be
//! picked.
//!
//! The canvas's other overlays live here too: the box a frame's title is
//! typed in, and the note while a wire is dragged.

use super::MaterialEditor;
use super::canvas::Places;
use super::drags::{Drag, Target};
use super::look;
use crate::text::tr;
use crate::theme;
use egui::{Context, Id, Order, Pos2, Rect, RichText};
use glam::Vec2;
use matgraph::{BoxId, Fit, Fix, Graph, GraphEdit, PlugRef, Refusal, Registry};
use scene::Document;

/// The note's width.
const WIDTH: f32 = 300.0;
/// The note's button that closes it without a fix.
pub(crate) const NEVER_MIND: &str = crate::text!("Never mind");

/// Where a refusal happened, for its fixes: the wire's two plugs, or the
/// empty input of a box that can't run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FixContext {
    pub(crate) from: Option<PlugRef>,
    pub(crate) to: Option<PlugRef>,
}

impl FixContext {
    /// A wire from `from` to `to`.
    pub(crate) fn wire(from: PlugRef, to: PlugRef) -> Self {
        Self {
            from: Some(from),
            to: Some(to),
        }
    }

    /// Box `id`, which can't run: its first needed input with nothing in
    /// it.
    pub(crate) fn problem(graph: &Graph, registry: &Registry, id: BoxId) -> Self {
        let to = graph
            .get(id)
            .and_then(|b| registry.get(&b.kind))
            .and_then(|t| {
                t.info()
                    .inputs
                    .iter()
                    .map(|p| PlugRef::new(id, p.key.to_string()))
                    .find(|p| graph.wire_into(p).is_none())
            });
        Self { from: None, to }
    }
}

/// A refusal shown on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Note {
    pub(crate) refusal: Refusal,
    /// Where it points, on the canvas.
    pub(crate) at: Vec2,
    pub(crate) context: FixContext,
}

/// What was picked on a note.
enum Answer {
    Fix(Fix),
    Close,
}

/// Draws a note saying `why` at `at`; with `pick`, its fixes are buttons.
/// Gives back where it was drawn and what was picked.
fn draw_note(
    ctx: &Context,
    id: Id,
    at: Pos2,
    why: &Refusal,
    graph: &Graph,
    pick: bool,
) -> (Rect, Option<Answer>) {
    let mut answer = None;
    let shown = egui::Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(at + egui::vec2(14.0, 10.0))
        .interactable(pick)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .stroke(egui::Stroke::new(1.0, look::REFUSED))
                .show(ui, |ui| {
                    ui.set_max_width(WIDTH);
                    ui.horizontal_top(|ui| {
                        let (dot, _) =
                            ui.allocate_exact_size(egui::vec2(10.0, 16.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 4.0, look::REFUSED);
                        ui.add(
                            egui::Label::new(RichText::new(&why.reason).color(theme::TEXT_STRONG))
                                .wrap(),
                        );
                    });
                    if why.fixes.is_empty() && !pick {
                        return;
                    }
                    ui.add_space(4.0);
                    if !pick {
                        crate::dialog::help(ui, tr("Let go, then pick one of these:"));
                    }
                    let registry = Registry::standard();
                    for fix in &why.fixes {
                        let words = fix.describe(registry, graph);
                        if pick {
                            if ui.button(words).clicked() {
                                answer = Some(Answer::Fix(fix.clone()));
                            }
                        } else {
                            crate::dialog::help(ui, format!("• {words}"));
                        }
                    }
                    if pick {
                        ui.add_space(2.0);
                        if ui
                            .add(egui::Button::new(
                                RichText::new(tr(NEVER_MIND)).color(theme::TEXT_WEAK),
                            ))
                            .clicked()
                        {
                            answer = Some(Answer::Close);
                        }
                    }
                });
        });
    (shown.response.rect, answer)
}

impl MaterialEditor {
    /// The canvas's overlays: the note, the wire's note while dragging,
    /// the box search and a frame's title being typed.
    pub(super) fn overlays(
        &mut self,
        ctx: &Context,
        rect: Rect,
        graph: &Graph,
        places: &Places,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let _ = places;
        if let Some(Drag::Wire {
            target: Some(Target { fits, .. }),
            pointer,
            ..
        }) = &self.canvas.drag
        {
            match fits {
                Err(why) => {
                    let id = Id::new("material wire note");
                    let _ = draw_note(ctx, id, kept_inside(rect, *pointer), why, graph, false);
                }
                Ok(Fit::Converts(how)) => {
                    egui::Area::new(Id::new("material wire converts"))
                        .order(Order::Foreground)
                        .fixed_pos(*pointer + egui::vec2(14.0, 10.0))
                        .interactable(false)
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.set_max_width(WIDTH);
                                ui.add(
                                    egui::Label::new(RichText::new(how).color(look::FITS)).wrap(),
                                );
                            });
                        });
                }
                Ok(Fit::Fits) => {}
            }
        }
        if let Some(shown) = self.canvas.note.clone() {
            let at = kept_inside(rect, self.canvas.view.on_screen(rect, shown.at));
            let id = Id::new("material note");
            let (drawn, answer) = draw_note(ctx, id, at, &shown.refusal, graph, true);
            self.canvas.overlays.push(drawn);
            match answer {
                Some(Answer::Fix(fix)) => {
                    self.canvas.note = None;
                    self.carry_out_fix(document, &fix, &shown.context, rect, note);
                }
                Some(Answer::Close) => self.canvas.note = None,
                None => {}
            }
        }
        self.search_overlay(ctx, rect, graph, document, note);
        self.title_box(ctx, rect, graph, document, note);
    }

    /// The box a frame's title is typed in, over its title (double-click
    /// the title). Enter or clicking away keeps it; Esc doesn't.
    fn title_box(
        &mut self,
        ctx: &Context,
        rect: Rect,
        graph: &Graph,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some((id, mut text, first)) = self.canvas.renaming.take() else {
            return;
        };
        let Some(frame) = graph.frame(id) else {
            return;
        };
        let at = self.canvas.view.on_screen(rect, frame.min);
        let width = (frame.max.x - frame.min.x) * self.canvas.view.zoom;
        let mut done = None;
        let shown = egui::Area::new(Id::new("material frame title"))
            .order(Order::Foreground)
            .fixed_pos(at + egui::vec2(4.0, 2.0))
            .show(ctx, |ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .desired_width(width.clamp(120.0, 400.0))
                        .hint_text(tr("Frame title")),
                );
                if first {
                    field.request_focus();
                }
                let (enter, escape) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::Enter),
                        i.key_pressed(egui::Key::Escape),
                    )
                });
                if escape {
                    done = Some(false);
                } else if field.lost_focus() || (enter && field.has_focus()) {
                    done = Some(true);
                }
            });
        self.canvas.overlays.push(shown.response.rect);
        match done {
            Some(true) => {
                let title = text.trim().to_string();
                if !title.is_empty() && title != frame.title {
                    let mut changed = frame.clone();
                    changed.title = title;
                    let edit = GraphEdit::ChangeFrame { id, frame: changed };
                    let label = tr("Rename a comment frame");
                    if let Err(why) = self.run(document, vec![edit], label, None, false) {
                        let context = FixContext::default();
                        self.show_refusal(why, at, rect, context, note);
                    }
                }
            }
            Some(false) => {}
            None => self.canvas.renaming = Some((id, text, false)),
        }
    }
}

/// A note's anchor kept on screen: a little in from the canvas's edges.
pub(crate) fn kept_inside(rect: Rect, at: Pos2) -> Pos2 {
    let right = (rect.right() - WIDTH - 20.0).max(rect.left());
    Pos2::new(
        at.x.clamp(rect.left(), right),
        at.y.clamp(rect.top(), rect.bottom() - 40.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::GraphEdit;

    #[test]
    fn a_box_that_cant_run_points_at_its_empty_input() {
        let registry = Registry::standard();
        let mut g = Graph::new("Stone");
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: "levels".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        let context = FixContext::problem(&g, registry, BoxId(1));
        assert_eq!(context.to, Some(PlugRef::new(BoxId(1), "in")));
        assert_eq!(
            FixContext::problem(&g, registry, BoxId(7)),
            FixContext::default()
        );
    }

    #[test]
    fn notes_stay_on_the_canvas() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let at = kept_inside(rect, Pos2::new(790.0, 700.0));
        assert!(at.x + WIDTH <= rect.right());
        assert!(at.y < rect.bottom());
    }
}
