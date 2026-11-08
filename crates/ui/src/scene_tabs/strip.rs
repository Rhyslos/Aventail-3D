//! Drawing the scene tabs (decision 0266): one tab for each project open,
//! with its name, a dot in the accent colour while it has unsaved changes
//! and × to close it, then + for a new tab. Mouse only: a click switches,
//! × or a middle click closes, a drag moves a tab along the row. When the
//! tabs don't fit they shrink, down to a width that still shows a few
//! letters of the name, and then the row scrolls.
//!
//! The strip only draws and notes what was done ([`TabEvent`]); the
//! [`super::SceneTabs`] carry it out after the frame.

use crate::text::tr;
use crate::theme;
use crate::viewport::TabPlacement;
use egui::{
    Color32, CornerRadius, FontId, PointerButton, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};
use std::path::PathBuf;

/// The widest a tab grows, in points.
pub(crate) const TAB_MAX: f32 = 190.0;
/// The narrowest a tab shrinks before the row scrolls.
pub(crate) const TAB_MIN: f32 = 84.0;
/// The space between two tabs.
const GAP: f32 = 2.0;
/// The space on each side of a tab's contents.
const PADDING: f32 = 8.0;
/// The unsaved dot's radius, and the room it takes.
const DOT: f32 = 3.5;
const DOT_ROOM: f32 = 12.0;
/// The × button's size.
const CLOSE: f32 = 16.0;
/// The + button's size.
const PLUS: f32 = 22.0;
/// The name's size.
const TEXT_SIZE: f32 = 13.0;
/// The tallest a tab is.
const TAB_HEIGHT: f32 = 24.0;
/// How far the mouse must move before a press on a tab is a drag.
const DRAG_START: f32 = 4.0;

/// One tab as drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TabLabel {
    /// The tab's lasting number ([`super::TabId`]).
    pub(crate) id: u64,
    /// The project's name: its file name, or "Untitled".
    pub(crate) name: String,
    /// Whether it has unsaved changes (the dot).
    pub(crate) unsaved: bool,
    /// Where it is saved, for its tooltip.
    pub(crate) path: Option<PathBuf>,
}

/// What was done with the tabs this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabEvent {
    /// Use this tab.
    Switch(u64),
    /// Close this tab (asking first if it has unsaved changes).
    Close(u64),
    /// Start a new project in a new tab.
    New,
    /// Move this tab to this place in the row (counting from 0).
    Move {
        /// Which tab.
        id: u64,
        /// Its new place.
        to: usize,
    },
}

/// The tabs to draw, and what was done with them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TabStrip {
    /// Every tab, in the row's order.
    pub(crate) tabs: Vec<TabLabel>,
    /// Which one is in use (its place in `tabs`).
    pub(crate) active: usize,
    /// Where the tabs sit.
    pub(crate) placement: TabPlacement,
    /// What was clicked or dragged, in order.
    pub(crate) events: Vec<TabEvent>,
}

/// The accessible name (and kittest label) of the + button.
pub(crate) const NEW_TAB: &str = crate::text!("New tab");

/// The accessible name of a tab's × button.
pub(crate) fn close_label(name: &str) -> String {
    crate::trf!("Close {}", name)
}

/// Each tab's width: as wide as its name needs (within limits), all
/// shrunk alike when they don't fit `room`, never below [`TAB_MIN`].
pub(crate) fn widths(wanted: &[f32], room: f32) -> Vec<f32> {
    let clamped: Vec<f32> = wanted.iter().map(|w| w.clamp(TAB_MIN, TAB_MAX)).collect();
    let gaps = GAP * clamped.len().saturating_sub(1) as f32;
    if clamped.iter().sum::<f32>() + gaps <= room || clamped.is_empty() {
        return clamped;
    }
    let share = ((room - gaps) / clamped.len() as f32).max(TAB_MIN);
    clamped.iter().map(|w| w.min(share)).collect()
}

/// Where a tab dragged to `x` belongs among the other tabs' `centres`
/// (left to right, the dragged one left out): the number of them left of
/// the mouse.
pub(crate) fn place_for(x: f32, centres: &[f32]) -> usize {
    centres.iter().filter(|c| **c < x).count()
}

/// Draws the tabs and the + button in what is left of `ui`'s row.
pub(crate) fn show(ui: &mut Ui, strip: &mut TabStrip) {
    let height = ui.spacing().interact_size.y.clamp(18.0, TAB_HEIGHT);
    let font = FontId::new(TEXT_SIZE, egui::FontFamily::Proportional);
    let wanted: Vec<f32> = strip
        .tabs
        .iter()
        .map(|tab| {
            let text = ui
                .painter()
                .layout_no_wrap(tab.name.clone(), font.clone(), theme::TEXT)
                .size()
                .x;
            PADDING + DOT_ROOM + text + 4.0 + CLOSE + PADDING
        })
        .collect();
    let room = (ui.available_width() - PLUS - 3.0 * GAP).max(TAB_MIN);
    let sizes = widths(&wanted, room);
    let row = sizes.iter().sum::<f32>() + GAP * sizes.len().saturating_sub(1) as f32;
    ui.spacing_mut().item_spacing.x = GAP;
    egui::ScrollArea::horizontal()
        .id_salt("scene_tabs_row")
        .max_width(row.min(room))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                tabs(ui, strip, (&sizes, &wanted), height, &font);
            });
        });
    plus_button(ui, strip, height);
}

/// The tabs, left to right.
fn tabs(
    ui: &mut Ui,
    strip: &mut TabStrip,
    (sizes, wanted): (&[f32], &[f32]),
    height: f32,
    font: &FontId,
) {
    // Every tab's place first, so a dragged tab can be placed among them.
    let rects: Vec<Rect> = sizes
        .iter()
        .map(|w| ui.allocate_exact_size(vec2(*w, height), Sense::hover()).0)
        .collect();
    // Vertical wheel scrolls the row sideways, as a mouse has no other.
    let over_row = rects
        .first()
        .zip(rects.last())
        .map(|(a, b)| a.union(*b))
        .is_some_and(|r| ui.rect_contains_pointer(r));
    if over_row {
        let delta = ui.input(|i| i.smooth_scroll_delta);
        if delta.y != 0.0 && delta.x == 0.0 {
            ui.scroll_with_delta(vec2(delta.y, 0.0));
        }
    }
    let labels = strip.tabs.clone();
    for (place, (tab, rect)) in labels.iter().zip(&rects).enumerate() {
        let active = place == strip.active;
        let shrunk = rect.width() + 0.5 < wanted[place].clamp(TAB_MIN, TAB_MAX);
        let events = one_tab(ui, tab, *rect, (active, shrunk), font);
        for event in events {
            match event {
                Drag::Event(event) => strip.events.push(event),
                Drag::To(x) => {
                    let centres: Vec<f32> = rects
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != place)
                        .map(|(_, r)| r.center().x)
                        .collect();
                    let to = place_for(x, &centres);
                    if to != place {
                        strip.events.push(TabEvent::Move { id: tab.id, to });
                    }
                }
            }
        }
        // Keep the tab in use in sight once, when it becomes the one in
        // use (so the row can still be scrolled away from it).
        if active {
            let scroll_to = ui.ctx().memory(|m| m.data.get_temp::<u64>(scroll_id()));
            if scroll_to != Some(tab.id) {
                ui.scroll_to_rect(*rect, None);
                ui.ctx()
                    .memory_mut(|m| m.data.insert_temp(scroll_id(), tab.id));
            }
        }
    }
}

/// Where the strip remembers which tab it last scrolled into sight.
fn scroll_id() -> egui::Id {
    egui::Id::new("scene_tabs_scrolled_to")
}

/// What one tab did.
enum Drag {
    Event(TabEvent),
    /// Dragged, the mouse now at this x.
    To(f32),
}

/// Draws one tab in `rect` and returns what was done with it: `active`
/// if it is the tab in use, `shrunk` if it is narrower than its name
/// needs.
fn one_tab(
    ui: &mut Ui,
    tab: &TabLabel,
    rect: Rect,
    (active, shrunk): (bool, bool),
    font: &FontId,
) -> Vec<Drag> {
    let id = egui::Id::new("scene_tab").with(tab.id);
    let response = ui.interact(rect, id, Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, active, &tab.name));
    let accent = theme::accent(ui.ctx()).colors();
    let hovered = response.hovered() || response.dragged();
    let fill = if active {
        theme::HOVER
    } else if hovered {
        theme::PALETTE
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter_at(rect.expand(1.0));
    painter.rect_filled(rect, CornerRadius::same(4), fill);
    if active {
        painter.hline(
            rect.x_range().shrink(4.0),
            rect.bottom() - 1.0,
            Stroke::new(2.0, accent.light),
        );
    }
    let mut left = rect.left() + PADDING;
    if tab.unsaved {
        let centre = pos2(left + DOT, rect.center().y);
        painter.circle_filled(centre, DOT, accent.fill);
        let dot = Rect::from_center_size(centre, vec2(DOT_ROOM, DOT_ROOM));
        ui.interact(dot, id.with("unsaved"), Sense::hover())
            .on_hover_text(tr("Unsaved changes"))
            .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, tr("Unsaved changes")));
    }
    left += DOT_ROOM;
    // × shows on every tab with room for its whole name; on shrunk tabs,
    // only on the tab in use and the one under the mouse, leaving the
    // name the room.
    let show_close = active || hovered || !shrunk;
    let close_rect = Rect::from_center_size(
        pos2(rect.right() - PADDING - CLOSE / 2.0, rect.center().y),
        vec2(CLOSE, CLOSE),
    );
    let text_right = if show_close {
        close_rect.left() - 4.0
    } else {
        rect.right() - PADDING
    };
    let colour = if active {
        theme::TEXT_STRONG
    } else {
        theme::TEXT
    };
    let mut job = egui::text::LayoutJob::simple_singleline(tab.name.clone(), font.clone(), colour);
    job.wrap = egui::text::TextWrapping::truncate_at_width((text_right - left).max(8.0));
    let galley = ui.painter().layout_job(job);
    painter.galley(
        pos2(left, rect.center().y - galley.size().y / 2.0),
        galley,
        colour,
    );
    let mut done = Vec::new();
    let mut closed = false;
    if show_close {
        let close = ui.interact(close_rect, id.with("close"), Sense::click());
        close.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, close_label(&tab.name)));
        if close.hovered() {
            painter.rect_filled(close_rect, CornerRadius::same(3), theme::FIELD_FILL);
        }
        let c = close_rect.center();
        let arm = 3.5;
        let stroke = Stroke::new(
            1.3,
            if close.hovered() {
                theme::TEXT_STRONG
            } else {
                theme::TEXT_WEAK
            },
        );
        painter.line_segment([c + vec2(-arm, -arm), c + vec2(arm, arm)], stroke);
        painter.line_segment([c + vec2(-arm, arm), c + vec2(arm, -arm)], stroke);
        if close.on_hover_text(tr("Close this tab")).clicked() {
            done.push(Drag::Event(TabEvent::Close(tab.id)));
            closed = true;
        }
    }
    if closed {
        return done;
    }
    if response.clicked_by(PointerButton::Middle) {
        done.push(Drag::Event(TabEvent::Close(tab.id)));
        return done;
    }
    if response.drag_started_by(PointerButton::Primary) && !active {
        done.push(Drag::Event(TabEvent::Switch(tab.id)));
    }
    if response.dragged_by(PointerButton::Primary)
        && response.drag_delta() != egui::Vec2::ZERO
        && let Some(at) = response.interact_pointer_pos()
        && ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|o| (at.x - o.x).abs() >= DRAG_START)
    {
        done.push(Drag::To(at.x));
    }
    if response.clicked() && !active {
        done.push(Drag::Event(TabEvent::Switch(tab.id)));
    }
    let where_saved = match &tab.path {
        Some(path) => path.display().to_string(),
        None => tr("Not saved yet").to_string(),
    };
    response.on_hover_text(crate::trf!(
        "{}\n{}\nMiddle-click or × closes it; drag it to move it along the row.",
        tab.name,
        where_saved
    ));
    done
}

/// The + button after the tabs.
fn plus_button(ui: &mut Ui, strip: &mut TabStrip, height: f32) {
    let size = vec2(PLUS, height.min(PLUS));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr(NEW_TAB)));
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(4), theme::HOVER);
    }
    let colour = if response.hovered() {
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    let c = rect.center();
    let arm = 5.0;
    let stroke = Stroke::new(1.5, colour);
    painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], stroke);
    painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], stroke);
    if response
        .on_hover_text(tr("Start a new project in a new tab"))
        .clicked()
    {
        strip.events.push(TabEvent::New);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_take_what_their_names_need_within_limits() {
        assert_eq!(
            widths(&[120.0, 30.0, 500.0], 2000.0),
            [120.0, TAB_MIN, TAB_MAX]
        );
        assert!(widths(&[], 100.0).is_empty());
    }

    #[test]
    fn tabs_shrink_alike_and_then_stop_shrinking() {
        let wanted = [150.0; 4];
        // Room for 4 × 120 and the gaps: each shrinks to its share.
        let room = 4.0 * 120.0 + 3.0 * GAP;
        let got = widths(&wanted, room);
        assert!(got.iter().all(|w| (w - 120.0).abs() < 0.01), "{got:?}");
        // A narrow tab keeps its own width.
        let got = widths(&[150.0, 90.0, 150.0], 2.0 * 100.0 + 90.0 + 2.0 * GAP);
        assert_eq!(got[1], 90.0);
        // Far too many: never below the least; the row scrolls instead.
        let got = widths(&[150.0; 40], 600.0);
        assert!(got.iter().all(|w| *w == TAB_MIN));
    }

    #[test]
    fn a_dragged_tab_goes_where_the_mouse_is() {
        let centres = [50.0, 150.0, 250.0];
        assert_eq!(place_for(10.0, &centres), 0);
        assert_eq!(place_for(100.0, &centres), 1);
        assert_eq!(place_for(260.0, &centres), 3);
        assert_eq!(place_for(f32::NAN, &centres), 0);
    }
}
