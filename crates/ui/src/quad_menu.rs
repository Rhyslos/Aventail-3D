//! The quad menu ([decision 0094](../../../docs/decisions/0094-quad-menu.md)):
//! four short lists of commands around the mouse, opened by a right-click
//! in the viewport when switched on in Settings (off as standard). The
//! layout is 3ds Max's, the contents our own: what to show and hide on the
//! left, the modes and the current mode's tools on the right.
//!
//! This module only draws the menu and says what was picked; the
//! workbench fills it and carries the pick out.

use crate::commands::CommandId;
use crate::palette::Mode;
use crate::theme;
use egui::{Align2, Id, Pos2, Rect, RichText, Ui, vec2};

/// Something the quad menu can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuadAction {
    /// A command from the command list.
    Command(CommandId),
    /// Switch to a mode.
    Mode(Mode),
    /// Undo or redo.
    Undo,
    Redo,
    /// Hide the selection, or everything else.
    HideSelection,
    HideOthers,
    /// Show everything hidden.
    ShowAll,
    /// Add an edit layer to the selection (decision 0091).
    AddEditLayer,
    /// Snap to, or Align (decision 0093).
    SnapTo,
    Align,
}

/// One entry.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuadItem {
    pub(crate) label: String,
    /// Its key, shown on the right.
    pub(crate) shortcut: Option<String>,
    /// Why it can't be used now, if it can't.
    pub(crate) unavailable: Option<String>,
    /// Whether it is the current mode.
    pub(crate) current: bool,
    pub(crate) action: QuadAction,
}

impl QuadItem {
    /// An entry that can be used now.
    pub(crate) fn new(label: impl Into<String>, action: QuadAction) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            unavailable: None,
            current: false,
            action,
        }
    }
}

/// One of the four lists, with its heading.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Quadrant {
    pub(crate) title: &'static str,
    pub(crate) items: Vec<QuadItem>,
}

/// The four lists' places: top left, top right, bottom left, bottom right.
const CORNERS: [(Align2, [f32; 2]); 4] = [
    (Align2::RIGHT_BOTTOM, [-GAP, -GAP]),
    (Align2::LEFT_BOTTOM, [GAP, -GAP]),
    (Align2::RIGHT_TOP, [-GAP, GAP]),
    (Align2::LEFT_TOP, [GAP, GAP]),
];
const GAP: f32 = 5.0;
const WIDTH: f32 = 180.0;

/// The quad menu: where it is open, if it is.
#[derive(Debug, Clone, Default)]
pub(crate) struct QuadMenu {
    at: Option<Pos2>,
    /// Where its lists were drawn last frame, to tell a click outside.
    rects: Vec<Rect>,
}

impl QuadMenu {
    /// Opens it around `at`.
    pub(crate) fn open(&mut self, at: Pos2) {
        self.at = Some(at);
        self.rects.clear();
    }

    pub(crate) fn close(&mut self) {
        self.at = None;
        self.rects.clear();
    }

    pub(crate) fn is_open(&self) -> bool {
        self.at.is_some()
    }

    /// Draws it, if open, with `quadrants` (top left, top right, bottom
    /// left, bottom right). Returns what was picked. Esc or a click
    /// outside it closes it.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        quadrants: &[Quadrant; 4],
    ) -> Option<QuadAction> {
        let at = self.at?;
        let pressed_outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer.interact_pos().is_some_and(|p| {
                    !self.rects.is_empty() && !self.rects.iter().any(|r| r.contains(p))
                })
        });
        if pressed_outside
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.close();
            return None;
        }
        let mut picked = None;
        let mut rects = Vec::new();
        for (index, (quadrant, (pivot, offset))) in quadrants.iter().zip(CORNERS).enumerate() {
            if quadrant.items.is_empty() {
                continue;
            }
            let area = egui::Area::new(Id::new(("quad menu", index)))
                .order(egui::Order::Foreground)
                .pivot(pivot)
                .fixed_pos(at + vec2(offset[0], offset[1]))
                .constrain(true)
                .show(ctx, |ui| {
                    egui::Frame::menu(ui.style())
                        .show(ui, |ui| {
                            ui.set_width(WIDTH);
                            // Full-width rows, so keys line up on the right.
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| list(ui, quadrant),
                            )
                            .inner
                        })
                        .inner
                });
            rects.push(area.response.rect);
            if area.inner.is_some() {
                picked = area.inner;
            }
        }
        // A dot where it was opened.
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::new("quad dot"),
        ))
        .circle_filled(at, 3.0, theme::accent(ctx).colors().light);
        self.rects = rects;
        if picked.is_some() {
            self.close();
        }
        picked
    }
}

/// One list: its heading, then its entries. Returns the one clicked.
fn list(ui: &mut Ui, quadrant: &Quadrant) -> Option<QuadAction> {
    crate::dialog::menu_style(ui.style_mut());
    ui.label(
        RichText::new(quadrant.title)
            .small()
            .color(theme::TEXT_WEAK),
    );
    let mut picked = None;
    for item in &quadrant.items {
        let mut button = egui::Button::selectable(item.current, &item.label);
        if let Some(keys) = &item.shortcut {
            button = button.shortcut_text(keys.as_str());
        }
        let response = ui.add_enabled(item.unavailable.is_none(), button);
        let response = match &item.unavailable {
            Some(why) => response.on_disabled_hover_text(why),
            None => response,
        };
        if response.clicked() {
            picked = Some(item.action);
        }
    }
    picked
}
