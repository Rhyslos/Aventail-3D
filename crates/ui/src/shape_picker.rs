//! The shape list: press `B` to open a small list of primitives by the
//! mouse, press `B` again to move down it (`Shift + B` to move up, or the
//! arrow keys), and `Enter` or `Space` to create the highlighted shape.
//! `Esc` or a click elsewhere closes it; clicking a shape creates it.

use crate::icons;
use crate::objects::shape_icon;
use crate::theme::{self, tint};
use egui::{Context, Key, Modifiers, Pos2, Sense, Ui, WidgetInfo, WidgetType, vec2};
use scene::PrimitiveKind;

/// The key that opens the list and moves down it.
pub const SHAPE_LIST_KEY: Key = Key::B;

/// Accessible name of the list's entries start with this.
pub(crate) const ENTRY_PREFIX: &str = "Shape list: ";

/// The list while it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Open {
    /// Which shape is highlighted (an index into `PrimitiveKind::ALL`).
    highlight: usize,
    /// Where the list's top-left corner is.
    at: Pos2,
}

/// The shape list's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ShapePicker {
    open: Option<Open>,
}

impl ShapePicker {
    /// Whether the list is showing.
    #[cfg(test)]
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The highlighted shape, while the list is open.
    #[cfg(test)]
    pub(crate) fn highlighted(&self) -> Option<PrimitiveKind> {
        self.open.map(|o| PrimitiveKind::ALL[o.highlight])
    }

    /// Handles the list's keys (not while typing). Returns the shape to
    /// create, if `Enter` or `Space` chose one.
    pub(crate) fn keys(&mut self, ui: &Ui, typing: bool) -> Option<PrimitiveKind> {
        if typing {
            return None;
        }
        let count = PrimitiveKind::ALL.len();
        let pressed = |ui: &Ui, modifiers, key| ui.input_mut(|i| i.consume_key(modifiers, key));
        let back = pressed(ui, Modifiers::SHIFT, SHAPE_LIST_KEY);
        let forward = !back && pressed(ui, Modifiers::NONE, SHAPE_LIST_KEY);
        let Some(open) = self.open.as_mut() else {
            if forward || back {
                let at = ui
                    .input(|i| i.pointer.latest_pos())
                    .unwrap_or_else(|| ui.ctx().content_rect().center())
                    + vec2(12.0, 12.0);
                self.open = Some(Open {
                    highlight: if back { count - 1 } else { 0 },
                    at,
                });
            }
            return None;
        };
        if forward || pressed(ui, Modifiers::NONE, Key::ArrowDown) {
            open.highlight = (open.highlight + 1) % count;
        }
        if back || pressed(ui, Modifiers::NONE, Key::ArrowUp) {
            open.highlight = (open.highlight + count - 1) % count;
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.open = None;
            return None;
        }
        if pressed(ui, Modifiers::NONE, Key::Enter) || pressed(ui, Modifiers::NONE, Key::Space) {
            let chosen = PrimitiveKind::ALL[open.highlight];
            self.open = None;
            return Some(chosen);
        }
        None
    }

    /// Draws the list when open. Returns the shape clicked, if any.
    pub(crate) fn show(&mut self, ctx: &Context) -> Option<PrimitiveKind> {
        let open = self.open?;
        let mut chosen = None;
        let mut hovered = None;
        let area = egui::Area::new(egui::Id::new("shape_list"))
            .order(egui::Order::Foreground)
            .fixed_pos(open.at)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(170.0);
                    ui.label(egui::RichText::new("Create a shape").strong());
                    ui.add_space(2.0);
                    let accent = theme::accent(ui.ctx()).colors();
                    for (index, kind) in PrimitiveKind::ALL.into_iter().enumerate() {
                        let (rect, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                WidgetType::SelectableLabel,
                                true,
                                index == open.highlight,
                                format!("{ENTRY_PREFIX}{}", kind.name()),
                            )
                        });
                        let lit = index == open.highlight;
                        if lit {
                            ui.painter().rect_filled(rect, 4.0, accent.fill);
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                        }
                        let icon = egui::Rect::from_center_size(
                            rect.left_center() + vec2(14.0, 0.0),
                            vec2(16.0, 16.0),
                        );
                        icons::paint(ui.painter(), icon, shape_icon(kind), tint::SHAPE);
                        ui.painter().text(
                            rect.left_center() + vec2(30.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            kind.name(),
                            egui::FontId::proportional(13.0),
                            if lit { theme::TEXT_STRONG } else { theme::TEXT },
                        );
                        if response.hovered() {
                            hovered = Some(index);
                        }
                        if response.on_hover_text(kind.description()).clicked() {
                            chosen = Some(kind);
                        }
                    }
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("B: next · Shift + B: back · Enter or Space: create")
                            .small()
                            .color(theme::TEXT_WEAK),
                    );
                });
            });
        let moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
        if let (Some(index), Some(open), true) = (hovered, self.open.as_mut(), moved) {
            // Moving the mouse over the list moves the highlight too.
            open.highlight = index;
        }
        let clicked_elsewhere = ctx.input(|i| i.pointer.any_pressed())
            && !area.response.contains_pointer()
            && chosen.is_none();
        if chosen.is_some() || clicked_elsewhere {
            self.open = None;
        }
        chosen
    }
}
