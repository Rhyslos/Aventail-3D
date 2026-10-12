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
//! The ring's buttons hold only their icons. The middle names the tool
//! you point at; a click there (the cancel sign) closes the menu. Every
//! mode's ring has the same 14 places at the same size, so a button never
//! moves. Four ring styles and a list, chosen in Settings → Interface →
//! Radial menu, all with the same tools in the same order; keys shown
//! around the ring, or in the middle with the name.
//!
//! "Tap E to open" (on as standard) lets a tap open the menu and keep it
//! open, for stepping with `E` and choosing with `Enter`: easier for anyone
//! who finds holding a key hard. Off, the menu shows only while `E` is
//! held: letting go over a tool uses it, anywhere else closes it. The tools
//! come from the command list (`commands`); which can be used now is
//! decided by the workbench.

use crate::commands::{CommandId, Slot};
use crate::icons::{self, Icon};
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

/// How the tool menu looks: chosen in Settings → Interface → Radial menu
/// (the project lead's picks from the icon mockup, 2026-10-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToolMenuStyle {
    /// One solid band split by thin lines, like a dial; the highlighted
    /// slice lights up with an accent edge on its outside.
    #[default]
    ContinuousRing,
    /// Dark tiles whose inner edge faces the middle; the highlighted one
    /// gets an accent bar along that edge.
    InnerEdge,
    /// Dark tiles with a crisp edge; the highlighted one glows in the
    /// accent colour.
    Outlined,
    /// Plain grey tiles; the highlighted one fills with the accent colour.
    SolidTiles,
    /// A list by the mouse, like the shape list.
    List,
}

impl ToolMenuStyle {
    /// Every look, in Settings order.
    pub const ALL: [Self; 5] = [
        Self::ContinuousRing,
        Self::InnerEdge,
        Self::Outlined,
        Self::SolidTiles,
        Self::List,
    ];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ContinuousRing => "Continuous ring",
            Self::InnerEdge => "Inner edge",
            Self::Outlined => "Outlined",
            Self::SolidTiles => "Solid tiles",
            Self::List => "List",
        }
    }

    /// What it looks like, in plain words.
    pub fn description(self) -> &'static str {
        match self {
            Self::ContinuousRing => {
                "One solid band split by thin lines, like a dial; the highlighted slice lights up"
            }
            Self::InnerEdge => {
                "Dark tiles; the highlighted one gets an accent bar along the edge facing the middle"
            }
            Self::Outlined => "Dark tiles with a crisp edge; the highlighted one glows",
            Self::SolidTiles => "Grey tiles; the highlighted one fills with the accent colour",
            Self::List => "A list by the mouse, like the shape list (B)",
        }
    }

    /// Whether it is a ring (pointing in a direction picks a tool).
    fn is_ring(self) -> bool {
        self != Self::List
    }
}

/// How the tool menu looks and opens: chosen in Settings → Interface →
/// Radial menu, and remembered by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolMenuSettings {
    /// Its look.
    pub style: ToolMenuStyle,
    /// Each tool's key just outside its button (on as standard). Off: the
    /// key shows in the middle, with the name of the tool pointed at.
    pub show_keys: bool,
    /// A tap of `E` opens the menu and keeps it open (on as standard).
    /// Off: the menu shows only while `E` is held.
    pub tap_to_open: bool,
}

impl Default for ToolMenuSettings {
    fn default() -> Self {
        Self {
            style: ToolMenuStyle::ContinuousRing,
            show_keys: true,
            tap_to_open: true,
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
    /// How it looks and opens.
    pub(crate) settings: ToolMenuSettings,
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
            if released && !self.settings.tap_to_open {
                // A tap does nothing while the menu opens only when held.
                return None;
            }
            let at = if self.settings.style.is_ring() {
                fit_ring(mouse, ui.ctx().content_rect())
            } else {
                mouse + vec2(12.0, 12.0)
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
            if !self.settings.tap_to_open {
                // Let go without pointing at a tool: nothing, and close.
                self.open = None;
                return None;
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
        match self.settings.style {
            ToolMenuStyle::List => self.show_list(ctx, mode, open, entries),
            style => self.show_ring(ctx, open, entries, style),
        }
    }

    /// The ring, in one of its four styles.
    fn show_ring(
        &mut self,
        ctx: &Context,
        open: Open,
        entries: &[Entry],
        style: ToolMenuStyle,
    ) -> Option<CommandId> {
        let ring = Ring::new(open.at, count_places(entries));
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
        let accent = theme::accent(ctx).colors().fill;
        let show_keys = self.settings.show_keys;
        let mut chosen = None;
        let bounds = ring.bounds();
        egui::Area::new(egui::Id::new("tool_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(bounds.min)
            .show(ctx, |ui| {
                let (_, _) = ui.allocate_exact_size(bounds.size(), Sense::hover());
                let painter = ui.painter();
                if style == ToolMenuStyle::ContinuousRing {
                    ring.band(painter);
                }
                // The middle, where a click closes the menu: the cancel sign.
                painter.circle(
                    ring.centre,
                    MIDDLE,
                    MIDDLE_FILL,
                    Stroke::new(1.0, theme::TEXT_WEAK.gamma_multiply(0.45)),
                );
                icons::paint(
                    painter,
                    Rect::from_center_size(ring.centre, Vec2::splat(MIDDLE * 0.95)),
                    Icon::Cancel,
                    theme::TEXT_WEAK,
                );
                if let Some(p) = pointer
                    && p.distance(ring.centre) > MIDDLE
                {
                    let towards = (p - ring.centre).normalized();
                    painter.circle_filled(ring.centre + towards * MIDDLE, 3.0, accent);
                }
                for (index, entry) in entries.iter().enumerate() {
                    let info = entry.slot.command.info();
                    let centre = ring.place(entry.slot.place);
                    let button = Rect::from_center_size(centre, Vec2::splat(TILE_HALF * 2.0));
                    let response = ui.interact(
                        button,
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
                    ring.button(painter, entry.slot.place, style, lit, accent);
                    let usable = entry.ready == Ready::Yes;
                    let icon_color = if lit {
                        Color32::WHITE
                    } else if usable {
                        icon_tint(info.icon)
                    } else {
                        faint(icon_tint(info.icon))
                    };
                    icons::paint(
                        painter,
                        Rect::from_center_size(centre, Vec2::splat(ICON_SIZE)),
                        info.icon,
                        icon_color,
                    );
                    if show_keys {
                        key_label(ui, &ring, entry, lit);
                    }
                    if response.clicked() {
                        chosen = Some(entry.slot.command);
                    }
                }
                // In the middle: what the highlighted tool is.
                if let Some(entry) = entries.get(highlight) {
                    caption(ui, ring.centre, entry, !show_keys);
                    // The same in words for screen readers (and tests).
                    let middle = Rect::from_center_size(ring.centre, Vec2::splat(2.0 * MIDDLE));
                    let text = caption_text(ui.ctx(), entry);
                    ui.interact(middle, ui.id().with("caption"), Sense::hover())
                        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
                }
            });
        // A click anywhere in a tool's slice uses it; in the middle or far
        // away, it closes the menu.
        let pressed = ctx.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked());
        if chosen.is_none() && pressed {
            let reach = ring.radius + TILE_HALF + 60.0;
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

/// The ring's radius (to the middle of its buttons), the same in every
/// mode: the size of the icon mockup the project lead approved.
pub(crate) const RING_RADIUS: f32 = 140.0;
/// Half a button's depth, from the ring's middle line in and out.
const TILE_HALF: f32 = 37.0;
/// The space between buttons, in screen points.
const TILE_GAP: f32 = 5.0;
/// The icon's size in a button.
const ICON_SIZE: f32 = 28.0;
/// The middle of the ring: the mouse points at nothing there, and a click
/// closes the menu.
const MIDDLE: f32 = 38.0;
/// Room around the ring for the keys.
const KEY_ROOM: f32 = 60.0;
/// The middle circle's fill.
const MIDDLE_FILL: Color32 = Color32::from_rgb(0x26, 0x26, 0x28);
/// The ring's greys, from the mockup: buttons, the continuous band, the
/// lit slice of the band, lines between slices, and outlines.
const TILE: Color32 = Color32::from_rgb(0x38, 0x38, 0x3c);
const BAND: Color32 = Color32::from_rgb(0x2c, 0x2c, 0x30);
const BAND_LIT: Color32 = Color32::from_rgb(0x4a, 0x4a, 0x50);
const DIVIDER: Color32 = Color32::from_rgb(0x46, 0x46, 0x4c);
const OUTLINE: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x60);
const INNER_LIT: Color32 = Color32::from_rgb(0x3e, 0x3e, 0x44);
const INNER_BAR: Color32 = Color32::from_rgb(0x55, 0x55, 0x5b);

/// How many places the entries' ring has.
fn count_places(entries: &[Entry]) -> usize {
    entries
        .first()
        .map_or(crate::commands::RING_PLACES, |e| e.slot.places.max(1))
}

/// Where the ring's centre goes for a mouse at `mouse`, kept inside
/// `screen` so every tool and key can be seen.
fn fit_ring(mouse: Pos2, screen: Rect) -> Pos2 {
    let half = RING_RADIUS + TILE_HALF + KEY_ROOM * 0.6;
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
            radius: RING_RADIUS,
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
        self.at(self.radius, a)
    }

    fn at(&self, radius: f32, angle: f32) -> Pos2 {
        self.centre + vec2(angle.cos(), angle.sin()) * radius
    }

    /// The entry the mouse at `pointer` points towards: the one whose
    /// direction is nearest, once the mouse is out of the middle.
    pub(crate) fn aimed(&self, pointer: Pos2, entries: &[Entry]) -> Option<usize> {
        let offset = pointer - self.centre;
        if offset.length() <= MIDDLE {
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

    /// The rectangle the ring and its keys need.
    fn bounds(&self) -> Rect {
        Rect::from_center_size(
            self.centre,
            Vec2::splat(2.0 * (self.radius + TILE_HALF + KEY_ROOM)),
        )
    }

    /// A slice of the band around the ring: from `inner` to `outer`
    /// radius, `half_span` either side of `middle` (radians).
    fn slice(&self, middle: f32, half_span: f32, inner: f32, outer: f32) -> Vec<(Pos2, Pos2)> {
        // Enough points that even the whole band reads as a circle.
        let steps = ((half_span * 2.0 / TAU) * 120.0).ceil().max(8.0) as usize;
        (0..=steps)
            .map(|i| {
                let a = middle - half_span + 2.0 * half_span * i as f32 / steps as f32;
                (self.at(inner, a), self.at(outer, a))
            })
            .collect()
    }

    /// The slice of a button at `place`, with the gap to its neighbours
    /// left out (`gap`) or not.
    fn button_slice(&self, place: usize, inner: f32, outer: f32, gap: bool) -> Vec<(Pos2, Pos2)> {
        let half = PI / self.places as f32
            - if gap {
                TILE_GAP / 2.0 / self.radius
            } else {
                0.0
            };
        self.slice(self.angle(place), half, inner, outer)
    }

    /// The Continuous ring's whole band, with thin lines between its
    /// places (empty places included, so the band is always whole).
    fn band(&self, painter: &egui::Painter) {
        let (inner, outer) = (self.radius - TILE_HALF, self.radius + TILE_HALF);
        painter.add(fill_slice(&self.slice(0.0, PI, inner, outer), BAND));
        for place in 0..self.places {
            let a = self.angle(place) + PI / self.places as f32;
            painter.line_segment(
                [self.at(inner, a), self.at(outer, a)],
                Stroke::new(1.0, DIVIDER),
            );
        }
    }

    /// One button, in `style`, lit or not.
    fn button(
        &self,
        painter: &egui::Painter,
        place: usize,
        style: ToolMenuStyle,
        lit: bool,
        accent: Color32,
    ) {
        let (inner, outer) = (self.radius - TILE_HALF, self.radius + TILE_HALF);
        let tile = self.button_slice(place, inner, outer, true);
        match style {
            ToolMenuStyle::ContinuousRing => {
                if lit {
                    let whole = self.button_slice(place, inner, outer, false);
                    painter.add(fill_slice(&whole, BAND_LIT));
                    let rim = self.button_slice(
                        place,
                        outer - 0.13 * TILE_HALF * 2.0 / 2.1,
                        outer,
                        false,
                    );
                    painter.add(fill_slice(&rim, accent));
                }
            }
            ToolMenuStyle::InnerEdge => {
                painter.add(fill_slice(&tile, if lit { INNER_LIT } else { BAND }));
                outline_slice(painter, &tile, Stroke::new(1.0, DIVIDER));
                let depth = if lit { 0.19 } else { 0.07 } * TILE_HALF * 2.0 / 2.1;
                let bar = self.button_slice(place, inner, inner + depth, true);
                painter.add(fill_slice(&bar, if lit { accent } else { INNER_BAR }));
            }
            ToolMenuStyle::Outlined => {
                let fill = if lit {
                    accent.gamma_multiply(0.28)
                } else {
                    MIDDLE_FILL
                };
                painter.add(fill_slice(&tile, fill));
                let edge = if lit {
                    Stroke::new(2.0, accent)
                } else {
                    Stroke::new(1.2, OUTLINE)
                };
                outline_slice(painter, &tile, edge);
            }
            ToolMenuStyle::SolidTiles => {
                painter.add(fill_slice(&tile, if lit { accent } else { TILE }));
            }
            ToolMenuStyle::List => {}
        }
    }
}

/// A slice of the ring (pairs of inner and outer points) as a filled mesh.
fn fill_slice(pairs: &[(Pos2, Pos2)], color: Color32) -> egui::Shape {
    let mut mesh = egui::Mesh::default();
    for (i, (inner, outer)) in pairs.iter().enumerate() {
        mesh.colored_vertex(*inner, color);
        mesh.colored_vertex(*outer, color);
        if i > 0 {
            let n = 2 * i as u32;
            mesh.add_triangle(n - 2, n - 1, n);
            mesh.add_triangle(n - 1, n + 1, n);
        }
    }
    egui::Shape::mesh(mesh)
}

/// A slice of the ring's outline.
fn outline_slice(painter: &egui::Painter, pairs: &[(Pos2, Pos2)], stroke: Stroke) {
    let mut points: Vec<Pos2> = pairs.iter().map(|(_, outer)| *outer).collect();
    points.extend(pairs.iter().rev().map(|(inner, _)| *inner));
    painter.add(egui::Shape::closed_line(points, stroke));
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

/// A tool's key just outside its button, on a dark label so it reads over
/// the scene.
fn key_label(ui: &Ui, ring: &Ring, entry: &Entry, lit: bool) {
    let info = entry.slot.command.info();
    let Some(keys) = info.shortcut else {
        return;
    };
    let place = ring.place(entry.slot.place);
    let direction = (place - ring.centre).normalized();
    let anchor = ring.centre + direction * (ring.radius + TILE_HALF + 7.0);
    let align = if direction.x > 0.15 {
        Align2::LEFT_CENTER
    } else if direction.x < -0.15 {
        Align2::RIGHT_CENTER
    } else if direction.y < 0.0 {
        Align2::CENTER_BOTTOM
    } else {
        Align2::CENTER_TOP
    };
    let color = if lit { theme::TEXT_STRONG } else { theme::TEXT };
    let galley = ui.painter().layout_no_wrap(
        ui.ctx().format_shortcut(&keys),
        FontId::monospace(11.0),
        color,
    );
    let rect = align.anchor_size(anchor, galley.size());
    ui.painter().rect_filled(
        rect.expand2(vec2(5.0, 2.0)),
        4.0,
        theme::PANEL.gamma_multiply(0.92),
    );
    ui.painter().galley(rect.min, galley, color);
}

/// A dark label behind text in the middle, so it reads over a light
/// shape: the middle of the ring is open to the scene.
fn backing(painter: &egui::Painter, text: Rect) {
    painter.rect_filled(
        text.expand2(vec2(6.0, 2.0)),
        4.0,
        theme::PANEL.gamma_multiply(0.92),
    );
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

/// The middle of the ring: the highlighted tool's name above the cancel
/// sign, and below it (`with_key`) its key, then why it can't be used now,
/// or when it arrives.
fn caption(ui: &Ui, centre: Pos2, entry: &Entry, with_key: bool) {
    let info = entry.slot.command.info();
    let painter = ui.painter();
    let width = (RING_RADIUS - TILE_HALF) * 1.6;
    let name = painter.layout(
        info.name.to_string(),
        FontId::proportional(15.0),
        theme::TEXT_STRONG,
        width,
    );
    let above = pos2(
        centre.x - name.size().x / 2.0,
        centre.y - MIDDLE - 8.0 - name.size().y,
    );
    backing(painter, Rect::from_min_size(above, name.size()));
    painter.galley(above, name, theme::TEXT_STRONG);
    let mut lines = Vec::new();
    if with_key && let Some(keys) = info.shortcut {
        lines.push((
            format!("Key {}", ui.ctx().format_shortcut(&keys)),
            FontId::monospace(12.0),
            theme::TEXT,
        ));
    }
    match &entry.ready {
        Ready::Yes => {}
        Ready::NotNow(why) => lines.push((why.clone(), FontId::proportional(11.5), theme::WARNING)),
        Ready::Later(when) => lines.push((
            when.to_string(),
            FontId::proportional(11.5),
            theme::TEXT_WEAK,
        )),
    }
    let mut y = centre.y + MIDDLE + 8.0;
    for (text, font, color) in lines {
        let galley = painter.layout(text, font, color, width);
        let at = pos2(centre.x - galley.size().x / 2.0, y);
        y += galley.size().y + 4.0;
        backing(painter, Rect::from_min_size(at, galley.size()));
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
    fn every_ring_has_one_size_and_stays_on_screen() {
        let places = crate::commands::RING_PLACES;
        assert_eq!(Ring::new(Pos2::ZERO, places).radius, RING_RADIUS);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1440.0, 900.0));
        let at = fit_ring(pos2(5.0, 5.0), screen);
        let ring = Ring::new(at, places);
        assert!(screen.contains(ring.place(0) - vec2(0.0, TILE_HALF)));
        assert!(screen.contains(ring.place(places * 3 / 4) - vec2(TILE_HALF, 0.0)));
        // The middle reaches no button.
        const { assert!(MIDDLE < RING_RADIUS - TILE_HALF) };
    }

    #[test]
    fn every_look_is_named_and_the_continuous_ring_is_standard() {
        assert_eq!(ToolMenuStyle::default(), ToolMenuStyle::ContinuousRing);
        let standard = ToolMenuSettings::default();
        assert!(standard.tap_to_open && standard.show_keys);
        // Settings saved without the newer switches read them as on.
        let read: ToolMenuSettings = ron::from_str("(style: InnerEdge)").unwrap();
        assert_eq!(read.style, ToolMenuStyle::InnerEdge);
        assert!(read.tap_to_open && read.show_keys);
        for style in ToolMenuStyle::ALL {
            assert!(!style.name().is_empty() && !style.description().is_empty());
        }
        let read: ToolMenuStyle = ron::from_str("List").unwrap();
        assert_eq!(read, ToolMenuStyle::List);
    }
}
