//! The tool menu: press `E` to open the tools for what you're working on,
//! at the mouse ([decision 0037](../../../docs/decisions/0037-tool-menu-and-command-list.md)).
//!
//! - Tap `E` again to step to the next tool (`Shift + E`, back; the arrow
//!   keys work too), and `Enter` or `Space` to use it.
//! - Or point at a tool and click it. In the radial menu the whole slice
//!   towards a tool is its target, so a rough direction is enough.
//! - Or hold `E`, point towards a tool and let go.
//! - `Esc`, or a click away from the menu, closes it.
//!
//! Two looks, chosen in Settings → Interface: a ring around the mouse
//! (standard) or a list, like the shape list. Both hold the same tools in
//! the same order. The tools come from the command list (`commands`);
//! which can be used now is decided by the workbench.

use crate::commands::{CommandId, Slot};
use crate::icons;
use crate::palette::{Mode, icon_tint};
use crate::theme;
use egui::{
    Align2, Color32, Context, Event, FontId, Key, Pos2, Rect, Sense, Stroke, Ui, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

/// The key that opens the tool menu and steps through it.
pub const TOOL_MENU_KEY: Key = Key::E;

/// Accessible names of the menu's entries start with this.
pub(crate) const ENTRY_PREFIX: &str = "Tool menu: ";

/// How the tool menu looks: chosen in Settings → Interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToolMenuStyle {
    /// A ring of tools around the mouse.
    #[default]
    Radial,
    /// A list by the mouse, like the shape list.
    List,
}

impl ToolMenuStyle {
    /// Both looks, in Settings order.
    pub const ALL: [Self; 2] = [Self::Radial, Self::List];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Radial => "Radial",
            Self::List => "List",
        }
    }

    /// What it looks like, in plain words.
    pub fn description(self) -> &'static str {
        match self {
            Self::Radial => {
                "A ring of tools around the mouse: point towards one and click, or hold E and let go"
            }
            Self::List => "A list by the mouse, like the shape list (B)",
        }
    }
}

/// Whether a tool can be used now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ready {
    /// Yes.
    Yes,
    /// Not with this selection: why.
    NotNow(String),
    /// Not built yet: when it arrives.
    Later(&'static str),
}

/// One entry of the open menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The command and its place on the ring.
    pub(crate) slot: Slot,
    /// Whether it can be used now.
    pub(crate) ready: Ready,
}

/// The menu while it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Open {
    /// The ring's centre (radial) or the list's corner.
    at: Pos2,
    /// Which entry is highlighted (an index into the entries).
    highlight: usize,
    /// `E` is still held down from opening it.
    held: bool,
    /// While held, the mouse pointed at a tool: letting go uses it.
    pointed: bool,
}

/// What the keys did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyOutcome {
    /// A tool was chosen (Enter, Space, or letting go of `E`).
    Chose(CommandId),
    /// `E` was pressed in a mode with no tools.
    NoTools,
}

/// The tool menu's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ToolMenu {
    open: Option<Open>,
    /// How it looks.
    pub(crate) style: ToolMenuStyle,
}

/// A press of `E` (not repeated by holding it): `Some(true)` with Shift
/// (back), `Some(false)` without. Ctrl + E (Make editable) is left alone.
fn take_presses(ui: &Ui) -> (Vec<bool>, bool) {
    ui.input_mut(|i| {
        let mut presses = Vec::new();
        let mut released = false;
        i.events.retain(|event| {
            let Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            if *key != TOOL_MENU_KEY || modifiers.command || modifiers.ctrl || modifiers.alt {
                return true;
            }
            if !*pressed {
                released = true;
            } else if !*repeat {
                presses.push(modifiers.shift);
            }
            false
        });
        (presses, released)
    })
}

impl ToolMenu {
    /// Whether the menu is showing.
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The highlighted entry's command, while open.
    #[cfg(test)]
    pub(crate) fn highlighted(&self, entries: &[Entry]) -> Option<CommandId> {
        let open = self.open?;
        entries.get(open.highlight).map(|e| e.slot.command)
    }

    /// Closes the menu.
    pub(crate) fn close(&mut self) {
        self.open = None;
    }

    /// Handles the menu's keys (not while typing).
    pub(crate) fn keys(&mut self, ui: &Ui, typing: bool, entries: &[Entry]) -> Option<KeyOutcome> {
        if typing {
            return None;
        }
        let (presses, released) = take_presses(ui);
        let count = entries.len();
        let pressed = |ui: &Ui, key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
        let Some(open) = self.open.as_mut() else {
            let &back = presses.first()?;
            if count == 0 {
                return Some(KeyOutcome::NoTools);
            }
            let mouse = ui
                .input(|i| i.pointer.latest_pos())
                .unwrap_or_else(|| ui.ctx().content_rect().center());
            let at = match self.style {
                ToolMenuStyle::Radial => {
                    fit_ring(mouse, count_places(entries), ui.ctx().content_rect())
                }
                ToolMenuStyle::List => mouse + vec2(12.0, 12.0),
            };
            self.open = Some(Open {
                at,
                highlight: if back { count - 1 } else { 0 },
                // A release in the same frame was a tap.
                held: !released,
                pointed: false,
            });
            return None;
        };
        if count == 0 {
            self.open = None;
            return None;
        }
        for back in presses {
            open.highlight = if back {
                (open.highlight + count - 1) % count
            } else {
                (open.highlight + 1) % count
            };
        }
        if pressed(ui, Key::ArrowDown) || pressed(ui, Key::ArrowRight) {
            open.highlight = (open.highlight + 1) % count;
        }
        if pressed(ui, Key::ArrowUp) || pressed(ui, Key::ArrowLeft) {
            open.highlight = (open.highlight + count - 1) % count;
        }
        if released && open.held {
            open.held = false;
            if open.pointed {
                let chosen = entries[open.highlight.min(count - 1)].slot.command;
                self.open = None;
                return Some(KeyOutcome::Chose(chosen));
            }
        }
        if pressed(ui, Key::Escape) {
            self.open = None;
            return None;
        }
        if pressed(ui, Key::Enter) || pressed(ui, Key::Space) {
            let chosen = entries[open.highlight.min(count - 1)].slot.command;
            self.open = None;
            return Some(KeyOutcome::Chose(chosen));
        }
        None
    }

    /// Draws the menu when open. Returns the tool clicked, if any.
    pub(crate) fn show(
        &mut self,
        ctx: &Context,
        mode: Mode,
        entries: &[Entry],
    ) -> Option<CommandId> {
        let open = self.open?;
        if entries.is_empty() {
            self.open = None;
            return None;
        }
        match self.style {
            ToolMenuStyle::Radial => self.show_ring(ctx, open, entries),
            ToolMenuStyle::List => self.show_list(ctx, mode, open, entries),
        }
    }

    fn show_ring(&mut self, ctx: &Context, open: Open, entries: &[Entry]) -> Option<CommandId> {
        let places = count_places(entries);
        let ring = Ring::new(open.at, places);
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        let moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
        // Pointing: the slice the mouse is in lights up.
        let aimed = pointer.and_then(|p| ring.aimed(p, entries));
        if let (Some(index), true) = (aimed, moved)
            && let Some(state) = self.open.as_mut()
        {
            state.highlight = index;
            if state.held {
                state.pointed = true;
            }
        }
        let highlight = self.open.map_or(open.highlight, |o| o.highlight);
        let accent = theme::accent(ctx).colors();
        let mut chosen = None;
        let bounds = ring.bounds();
        egui::Area::new(egui::Id::new("tool_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(bounds.min)
            .show(ctx, |ui| {
                let (_, _) = ui.allocate_exact_size(bounds.size(), Sense::hover());
                let painter = ui.painter();
                // The ring's band, behind the tools.
                painter.circle(
                    ring.centre,
                    ring.radius + CHIP_RADIUS + 8.0,
                    theme::PANEL.gamma_multiply(0.92),
                    Stroke::new(1.0, theme::BORDER),
                );
                painter.circle_stroke(ring.centre, DEADZONE, Stroke::new(1.0, theme::BORDER));
                if let Some(p) = pointer
                    && p.distance(ring.centre) > DEADZONE
                {
                    let towards = (p - ring.centre).normalized();
                    painter.circle_filled(ring.centre + towards * DEADZONE, 3.0, accent.fill);
                }
                for (index, entry) in entries.iter().enumerate() {
                    let info = entry.slot.command.info();
                    let centre = ring.place(entry.slot.place);
                    let chip = Rect::from_center_size(centre, Vec2::splat(CHIP_RADIUS * 2.0));
                    let response = ui.interact(
                        chip,
                        ui.id().with(("tool", entry.slot.command)),
                        Sense::click(),
                    );
                    let lit = index == highlight;
                    response.widget_info(|| {
                        WidgetInfo::selected(
                            WidgetType::Button,
                            true,
                            lit,
                            format!("{ENTRY_PREFIX}{}", info.name),
                        )
                    });
                    let usable = entry.ready == Ready::Yes;
                    let (fill, icon_color) = if lit {
                        (accent.fill, Color32::WHITE)
                    } else {
                        (theme::HOVER, icon_tint(info.icon))
                    };
                    let icon_color = if usable || lit {
                        icon_color
                    } else {
                        faint(icon_color)
                    };
                    painter.circle_filled(
                        centre,
                        CHIP_RADIUS,
                        if usable || lit { fill } else { faint(fill) },
                    );
                    icons::paint(
                        painter,
                        Rect::from_center_size(centre, Vec2::splat(20.0)),
                        info.icon,
                        icon_color,
                    );
                    label(ui, &ring, entry, lit);
                    if response.clicked() {
                        chosen = Some(entry.slot.command);
                    }
                }
                // In the middle: what the highlighted tool is.
                if let Some(entry) = entries.get(highlight) {
                    caption(ui, ring.centre, ring.radius - CHIP_RADIUS - 12.0, entry);
                    // The same in words for screen readers (and tests).
                    let middle = Rect::from_center_size(ring.centre, Vec2::splat(2.0 * DEADZONE));
                    let text = caption_text(ui.ctx(), entry);
                    ui.interact(middle, ui.id().with("caption"), Sense::hover())
                        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
                }
            });
        // A click anywhere in a tool's slice uses it; in the middle or far
        // away, it closes the menu.
        let pressed = ctx.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked());
        if chosen.is_none() && pressed {
            let reach = ring.radius + CHIP_RADIUS + 60.0;
            match pointer {
                Some(p)
                    if p.distance(ring.centre) <= reach
                        && ctx.input(|i| i.pointer.primary_clicked()) =>
                {
                    chosen = ring.aimed(p, entries).map(|i| entries[i].slot.command);
                    if chosen.is_none() {
                        self.open = None;
                    }
                }
                _ => self.open = None,
            }
        }
        if chosen.is_some() {
            self.open = None;
        }
        chosen
    }

    fn show_list(
        &mut self,
        ctx: &Context,
        mode: Mode,
        open: Open,
        entries: &[Entry],
    ) -> Option<CommandId> {
        let mut chosen = None;
        let mut hovered = None;
        let accent = theme::accent(ctx).colors();
        let area = egui::Area::new(egui::Id::new("tool_menu_list"))
            .order(egui::Order::Foreground)
            .fixed_pos(open.at)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(230.0);
                    ui.label(egui::RichText::new(title(mode)).strong());
                    ui.add_space(2.0);
                    for (index, entry) in entries.iter().enumerate() {
                        let info = entry.slot.command.info();
                        let (rect, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                        let lit = index == open.highlight;
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                WidgetType::SelectableLabel,
                                true,
                                lit,
                                format!("{ENTRY_PREFIX}{}", info.name),
                            )
                        });
                        let usable = entry.ready == Ready::Yes;
                        if lit {
                            ui.painter().rect_filled(rect, 4.0, accent.fill);
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                        }
                        let text = match (lit, usable) {
                            (true, _) => theme::TEXT_STRONG,
                            (false, true) => theme::TEXT,
                            (false, false) => theme::TEXT_WEAK,
                        };
                        let icon_color = if lit {
                            Color32::WHITE
                        } else if usable {
                            icon_tint(info.icon)
                        } else {
                            faint(icon_tint(info.icon))
                        };
                        let icon = Rect::from_center_size(
                            rect.left_center() + vec2(14.0, 0.0),
                            vec2(16.0, 16.0),
                        );
                        icons::paint(ui.painter(), icon, info.icon, icon_color);
                        ui.painter().text(
                            rect.left_center() + vec2(30.0, 0.0),
                            Align2::LEFT_CENTER,
                            info.name,
                            FontId::proportional(13.0),
                            text,
                        );
                        if let Some(keys) = info.shortcut {
                            ui.painter().text(
                                rect.right_center() - vec2(8.0, 0.0),
                                Align2::RIGHT_CENTER,
                                ui.ctx().format_shortcut(&keys),
                                FontId::proportional(12.0),
                                if lit {
                                    theme::TEXT_STRONG
                                } else {
                                    theme::TEXT_WEAK
                                },
                            );
                        }
                        if response.hovered() {
                            hovered = Some(index);
                        }
                        if response.on_hover_text(hover_text(entry)).clicked() {
                            chosen = Some(entry.slot.command);
                        }
                    }
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(
                            "E: next · Shift + E: back · Enter or Space: use · Esc: close",
                        )
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                });
            });
        let moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
        if let (Some(index), Some(state), true) = (hovered, self.open.as_mut(), moved) {
            // Moving the mouse over the list moves the highlight too.
            state.highlight = index;
            if state.held {
                state.pointed = true;
            }
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

/// The radius of a tool's round button on the ring.
const CHIP_RADIUS: f32 = 19.0;
/// The middle of the ring, where the mouse points at nothing.
const DEADZONE: f32 = 22.0;
/// Room around the ring for the tools' names.
const LABEL_ROOM: f32 = 110.0;

/// How many places the entries' ring has.
fn count_places(entries: &[Entry]) -> usize {
    entries.first().map_or(1, |e| e.slot.places.max(1))
}

/// The ring's radius for `places` tools: bigger rings for more tools, so
/// each keeps room to aim at.
pub(crate) fn ring_radius(places: usize) -> f32 {
    (places as f32 * 13.0).clamp(120.0, 175.0)
}

/// Where the ring's centre goes for a mouse at `mouse`, kept inside
/// `screen` so every tool and name can be seen.
fn fit_ring(mouse: Pos2, places: usize, screen: Rect) -> Pos2 {
    let half = ring_radius(places) + CHIP_RADIUS + LABEL_ROOM.min(60.0);
    let inner = screen.shrink2(vec2(
        half.min(screen.width() / 2.0),
        half.min(screen.height() / 2.0),
    ));
    pos2(
        mouse.x.clamp(inner.left(), inner.right().max(inner.left())),
        mouse.y.clamp(inner.top(), inner.bottom().max(inner.top())),
    )
}

/// The ring's shape on screen.
pub(crate) struct Ring {
    pub(crate) centre: Pos2,
    pub(crate) radius: f32,
    places: usize,
}

impl Ring {
    pub(crate) fn new(centre: Pos2, places: usize) -> Self {
        Self {
            centre,
            radius: ring_radius(places),
            places: places.max(1),
        }
    }

    /// The direction of `place`, clockwise from the top, in radians (0
    /// pointing right, growing clockwise on screen).
    fn angle(&self, place: usize) -> f32 {
        -PI / 2.0 + place as f32 * TAU / self.places as f32
    }

    /// Where the tool at `place` sits.
    pub(crate) fn place(&self, place: usize) -> Pos2 {
        let a = self.angle(place);
        self.centre + vec2(a.cos(), a.sin()) * self.radius
    }

    /// The entry the mouse at `pointer` points towards: the one whose
    /// direction is nearest, once the mouse is out of the middle.
    pub(crate) fn aimed(&self, pointer: Pos2, entries: &[Entry]) -> Option<usize> {
        let offset = pointer - self.centre;
        if offset.length() <= DEADZONE {
            return None;
        }
        let pointing = offset.y.atan2(offset.x);
        let gap = |a: f32| {
            let d = (a - pointing).rem_euclid(TAU);
            d.min(TAU - d)
        };
        entries
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                gap(self.angle(a.slot.place)).total_cmp(&gap(self.angle(b.slot.place)))
            })
            .map(|(i, _)| i)
    }

    /// The rectangle the ring and its names need.
    fn bounds(&self) -> Rect {
        Rect::from_center_size(
            self.centre,
            Vec2::splat(2.0 * (self.radius + CHIP_RADIUS + LABEL_ROOM)),
        )
    }
}

/// A colour for a tool that can't be used now.
fn faint(color: Color32) -> Color32 {
    theme::PALETTE.lerp_to_gamma(color, theme::UNAVAILABLE_STRENGTH)
}

/// The menu's heading.
fn title(mode: Mode) -> String {
    match mode {
        Mode::Model | Mode::Object => "Tools for objects".to_string(),
        _ => format!("Tools for {}", mode.name().to_lowercase()),
    }
}

/// The tooltip for an entry: what it does, and why it can't be used.
fn hover_text(entry: &Entry) -> String {
    let info = entry.slot.command.info();
    match &entry.ready {
        Ready::Yes => info.help.to_string(),
        Ready::NotNow(why) => format!("{}\n{why}", info.help),
        Ready::Later(when) => format!("{}\n{when}", info.help),
    }
}

/// A tool's name (and key) outside its button, on a dark pill so it reads
/// over the scene.
fn label(ui: &Ui, ring: &Ring, entry: &Entry, lit: bool) {
    let info = entry.slot.command.info();
    let place = ring.place(entry.slot.place);
    let direction = (place - ring.centre).normalized();
    let anchor = place + direction * (CHIP_RADIUS + 8.0);
    let align = if direction.x > 0.15 {
        Align2::LEFT_CENTER
    } else if direction.x < -0.15 {
        Align2::RIGHT_CENTER
    } else if direction.y < 0.0 {
        Align2::CENTER_BOTTOM
    } else {
        Align2::CENTER_TOP
    };
    let usable = entry.ready == Ready::Yes;
    let color = match (lit, usable) {
        (true, _) => theme::TEXT_STRONG,
        (false, true) => theme::TEXT,
        (false, false) => theme::TEXT_WEAK,
    };
    let mut text = info.name.to_string();
    if let Some(keys) = info.shortcut {
        // (Delete's key is called Delete too: no need to say it twice.)
        let keys = ui.ctx().format_shortcut(&keys);
        if keys != info.name {
            text.push_str("  ");
            text.push_str(&keys);
        }
    }
    let galley = ui
        .painter()
        .layout_no_wrap(text, FontId::proportional(12.5), color);
    let rect = align.anchor_size(anchor, galley.size());
    ui.painter().rect_filled(
        rect.expand2(vec2(5.0, 2.0)),
        4.0,
        theme::PANEL.gamma_multiply(0.92),
    );
    ui.painter().galley(rect.min, galley, color);
}

/// What the middle of the ring says, as one line of words.
fn caption_text(ctx: &Context, entry: &Entry) -> String {
    let info = entry.slot.command.info();
    let mut text = info.name.to_string();
    if let Some(keys) = info.shortcut {
        text.push_str(&format!(". Key: {}", ctx.format_shortcut(&keys)));
    }
    let status = match &entry.ready {
        Ready::Yes => info.help.to_string(),
        Ready::NotNow(why) => why.clone(),
        Ready::Later(when) => (*when).to_string(),
    };
    format!("{text}. {status}")
}

/// The middle of the ring: the highlighted tool's name above the pointer
/// ring, and below it its key and what it does, or why it can't be used
/// now.
fn caption(ui: &Ui, centre: Pos2, room: f32, entry: &Entry) {
    let info = entry.slot.command.info();
    let painter = ui.painter();
    let width = (room * 1.3).max(120.0);
    let name = painter.layout(
        info.name.to_string(),
        FontId::proportional(14.0),
        theme::TEXT_STRONG,
        width,
    );
    let above = pos2(
        centre.x - name.size().x / 2.0,
        centre.y - DEADZONE - 6.0 - name.size().y,
    );
    painter.galley(above, name, theme::TEXT_STRONG);
    let mut lines = Vec::new();
    if let Some(keys) = info.shortcut {
        lines.push((
            format!("Key: {}", ui.ctx().format_shortcut(&keys)),
            theme::TEXT_WEAK,
        ));
    }
    lines.push(match &entry.ready {
        Ready::Yes => (info.help.to_string(), theme::TEXT),
        Ready::NotNow(why) => (why.clone(), theme::WARNING),
        Ready::Later(when) => (when.to_string(), theme::TEXT_WEAK),
    });
    let mut y = centre.y + DEADZONE + 6.0;
    for (text, color) in lines {
        let galley = painter.layout(text, FontId::proportional(11.5), color, width);
        let at = pos2(centre.x - galley.size().x / 2.0, y);
        y += galley.size().y + 2.0;
        painter.galley(at, galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::menu_for;

    fn entries(mode: Mode) -> Vec<Entry> {
        menu_for(mode)
            .into_iter()
            .map(|slot| Entry {
                slot,
                ready: Ready::Yes,
            })
            .collect()
    }

    #[test]
    fn the_first_tool_sits_at_the_top_and_the_rest_go_clockwise() {
        let ring = Ring::new(pos2(500.0, 500.0), 4);
        let top = ring.place(0);
        assert!((top.x - 500.0).abs() < 0.01 && top.y < 500.0);
        let right = ring.place(1);
        assert!(right.x > 500.0 && (right.y - 500.0).abs() < 0.01);
    }

    #[test]
    fn pointing_picks_the_nearest_tool_and_the_middle_picks_none() {
        let list = entries(Mode::Model);
        let ring = Ring::new(pos2(500.0, 500.0), list[0].slot.places);
        assert_eq!(ring.aimed(pos2(502.0, 498.0), &list), None, "the middle");
        let up = ring.aimed(pos2(500.0, 300.0), &list).unwrap();
        assert_eq!(list[up].slot.command, CommandId::Move);
        // A rough direction is enough: just off a tool's own direction.
        let near = ring.place(list[2].slot.place) + vec2(8.0, 8.0);
        assert_eq!(ring.aimed(near, &list), Some(2));
    }

    #[test]
    fn gaps_are_skipped_when_pointing() {
        // Points mode leaves Extrude's place (the top) empty.
        let list = entries(Mode::Points);
        assert!(list.iter().all(|e| e.slot.command != CommandId::Extrude));
        let ring = Ring::new(pos2(500.0, 500.0), list[0].slot.places);
        let aimed = ring.aimed(pos2(500.0, 300.0), &list).unwrap();
        assert_ne!(list[aimed].slot.place, 0);
    }

    #[test]
    fn rings_grow_with_their_tools_and_stay_on_screen() {
        assert!(ring_radius(14) > ring_radius(8));
        assert!(ring_radius(40) <= 175.0);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1440.0, 900.0));
        let at = fit_ring(pos2(5.0, 5.0), 14, screen);
        let ring = Ring::new(at, 14);
        assert!(screen.contains(ring.place(0) - vec2(0.0, CHIP_RADIUS)));
        assert!(screen.contains(ring.place(10) - vec2(CHIP_RADIUS, 0.0)));
    }

    #[test]
    fn both_looks_are_named_and_radial_is_standard() {
        assert_eq!(ToolMenuStyle::default(), ToolMenuStyle::Radial);
        for style in ToolMenuStyle::ALL {
            assert!(!style.name().is_empty() && !style.description().is_empty());
        }
        let read: ToolMenuStyle = ron::from_str("List").unwrap();
        assert_eq!(read, ToolMenuStyle::List);
    }
}
