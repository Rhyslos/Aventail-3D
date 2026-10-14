//! The Keys window: a drawn keyboard showing every key the app uses, and
//! the mouse actions under it, opened by **Keys** on the menu bar, next to
//! Help ([decision 0056](../../../../docs/decisions/0056-shortcut-viewer.md),
//! drawn keyboard: [decision 0061](../../../../docs/decisions/0061-drawn-keyboard.md)).
//!
//! The keys are read from the same constants the app acts on (the command
//! list, the tool, mode and file keys), so the window can't drift from
//! what the keys do. Mouse actions, and what a held Shift, Ctrl or Alt
//! does, have no constants and are written out here.
//!
//! Used keys are lit, with a coloured mark for each situation they work
//! in (anywhere, the Model mode, the element modes, while dragging, in
//! menus). Pointing at a key lists its jobs; clicking keeps it listed.
//! The search lights the keys whose jobs match.

use super::Workbench;
use super::{copying, modelling, showing};
use crate::commands::{self, CommandId};
use crate::keyboard::{self, Cap, KeyId, KeyboardLayout};
use crate::palette::Mode;
use crate::{objects, project, selecting, settings, theme, transforming, viewport};
use egui::{Color32, Context, Key, KeyboardShortcut, Modifiers, Ui};
use std::collections::BTreeSet;

/// The window's title, and the menu bar button's name.
pub(crate) const VIEWER_TITLE: &str = "Keyboard shortcuts";
/// The menu bar button that opens it.
pub(crate) const KEYS_BUTTON: &str = "Keys";
/// The search field's hint.
pub(crate) const FILTER_HINT: &str = "Find a key or action";
/// The keyboard picker's label.
pub(crate) const LAYOUT_LABEL: &str = "Keyboard";
/// The heading over the mouse actions.
pub(crate) const MOUSE_HEADING: &str = "Mouse";
/// What the side panel says while no key is pointed at or searched for.
const HINT: &str = "Point at a key to see what it does. Click a key to keep it shown.";
/// How wide the side panel beside the keyboard is, in key widths.
const DETAILS_KEYS: f32 = 7.5;
/// The key width at which the window's text is its normal size: bigger
/// keys scale the text with them.
const NORMAL_KEY: f32 = 38.0;
/// The smallest and largest key width, in points. Below the smallest, the
/// window scrolls.
const KEY_RANGE: (f32, f32) = (24.0, 110.0);
/// How tall everything is, in key widths, at the text scale that goes
/// with them: the search and chips (about 2), the keyboard (6.25), and
/// the mouse actions (about 7).
const HEIGHT_KEYS: f32 = 15.5;
/// How much of the app's window the Keys window takes when it opens.
const SCREEN_SHARE: f32 = 0.9;
/// The widest it opens, as a multiple of its height, so a very wide
/// monitor doesn't stretch it out of proportion.
const WIDEST: f32 = 2.1;

/// Where a key or mouse action works. Each has a colour, marked on the
/// keys and in the lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Situation {
    /// Whatever mode the app is in.
    Anywhere,
    /// The Model mode, with whole objects.
    Model,
    /// The Points, Edges and Polygons modes.
    Editing,
    /// While dragging a gizmo or a tool.
    Dragging,
    /// In the shape list, the tool menu and questions such as "Save
    /// changes?".
    Menus,
}

impl Situation {
    /// Every situation, in the order shown.
    pub(crate) const ALL: [Self; 5] = [
        Self::Anywhere,
        Self::Model,
        Self::Editing,
        Self::Dragging,
        Self::Menus,
    ];

    /// Its name, on its chip.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Anywhere => "Anywhere",
            Self::Model => "Model mode",
            Self::Editing => "Points, Edges, Polygons",
            Self::Dragging => "While dragging",
            Self::Menus => "Menus and questions",
        }
    }

    /// Its colour, marked on the keys.
    pub(crate) fn colour(self) -> Color32 {
        match self {
            Self::Anywhere => Color32::from_rgb(0x6c, 0x9d, 0xff),
            Self::Model => Color32::from_rgb(0xb1, 0x8c, 0xff),
            Self::Editing => Color32::from_rgb(0xfb, 0x9a, 0x4c),
            Self::Dragging => Color32::from_rgb(0x34, 0xc7, 0x9c),
            Self::Menus => Color32::from_rgb(0xe3, 0xb3, 0x41),
        }
    }
}

/// One job of one key: the key, the keys held with it, what it does and
/// where. For Shift, Ctrl and Alt on their own, what holding them does.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Binding {
    pub(crate) key: KeyId,
    pub(crate) modifiers: Modifiers,
    pub(crate) action: String,
    pub(crate) situation: Situation,
}

impl Binding {
    /// Its keys, as the system writes them (Ctrl on Windows and Linux, ⌘
    /// on a Mac).
    pub(crate) fn keys(&self, ctx: &Context) -> String {
        match self.key {
            KeyId::Key(key) => ctx.format_shortcut(&KeyboardShortcut::new(self.modifiers, key)),
            held => format!("Hold {}", modifier_name(ctx, held)),
        }
    }

    /// Whether `modifier` (Shift, Ctrl or Alt) is held with its key.
    fn held_with(&self, modifier: KeyId) -> bool {
        let m = self.modifiers;
        matches!(self.key, KeyId::Key(_))
            && match modifier {
                KeyId::Shift => m.shift,
                KeyId::Ctrl => m.command || m.ctrl || m.mac_cmd,
                KeyId::Alt => m.alt,
                KeyId::Key(_) => false,
            }
    }
}

/// A mouse action: what it does, how, and where.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MouseLine {
    pub(crate) action: &'static str,
    pub(crate) how: &'static str,
    pub(crate) situation: Situation,
}

/// The window's own state.
#[derive(Debug, Default)]
pub(crate) struct ShortcutViewer {
    pub(crate) open: bool,
    pub(crate) filter: String,
    /// The key clicked, whose jobs stay listed.
    pub(crate) pinned: Option<KeyId>,
    /// The situations switched off with their chips.
    pub(crate) hidden: BTreeSet<Situation>,
}

fn is_mac(ctx: &Context) -> bool {
    ctx.os() == egui::os::OperatingSystem::Mac
}

/// Shift, Ctrl or Alt as this system calls it.
fn modifier_name(ctx: &Context, key: KeyId) -> &'static str {
    match (key, is_mac(ctx)) {
        (KeyId::Shift, _) => "Shift",
        (KeyId::Ctrl, false) => "Ctrl",
        (KeyId::Ctrl, true) => "Cmd",
        (KeyId::Alt, false) => "Alt",
        (KeyId::Alt, true) => "Option",
        (KeyId::Key(_), _) => "",
    }
}

fn bind(keys: KeyboardShortcut, action: impl Into<String>, situation: Situation) -> Binding {
    Binding {
        key: KeyId::Key(keys.logical_key),
        modifiers: keys.modifiers,
        action: action.into(),
        situation,
    }
}

fn plain(key: Key, action: impl Into<String>, situation: Situation) -> Binding {
    bind(
        KeyboardShortcut::new(Modifiers::NONE, key),
        action,
        situation,
    )
}

fn held(key: KeyId, action: impl Into<String>, situation: Situation) -> Binding {
    Binding {
        key,
        modifiers: Modifiers::NONE,
        action: action.into(),
        situation,
    }
}

/// A command's key, under its name.
fn command(id: CommandId, situation: Situation) -> Option<Binding> {
    let info = id.info();
    Some(bind(info.shortcut?, info.name, situation))
}

/// Every key's every job.
pub(crate) fn bindings() -> Vec<Binding> {
    use Situation::*;
    let mut all = vec![
        bind(project::NEW_SHORTCUT, "New project", Anywhere),
        bind(project::OPEN_SHORTCUT, "Open a project", Anywhere),
        bind(project::SAVE_SHORTCUT, "Save", Anywhere),
        bind(project::SAVE_AS_SHORTCUT, "Save as", Anywhere),
        bind(objects::UNDO_SHORTCUT, "Undo", Anywhere),
        bind(objects::REDO_SHORTCUT, "Redo", Anywhere),
        bind(objects::REDO_SHORTCUT_2, "Redo", Anywhere),
        bind(settings::SETTINGS_SHORTCUT, "Settings", Anywhere),
    ];

    // Modes.
    all.extend(Mode::ALL.into_iter().filter_map(|mode| {
        Some(plain(
            mode.key()?,
            format!(
                "{} mode (again: back to Model; with a tool in use: selecting in it)",
                mode.name()
            ),
            Anywhere,
        ))
    }));
    all.push(plain(
        super::BACK_KEY,
        "Step back to the Model mode (finishes a slit, puts a tool away)",
        Anywhere,
    ));

    // Selecting.
    all.extend([
        plain(
            selecting::SELECT_TOOL_KEY,
            "Next select tool (Box, Lasso, Paint select)",
            Anywhere,
        ),
        bind(selecting::SELECT_ALL_SHORTCUT, "Select all", Anywhere),
        bind(selecting::INVERT_SHORTCUT, "Invert the selection", Anywhere),
        bind(objects::DESELECT_SHORTCUT, "Select nothing", Anywhere),
        held(
            KeyId::Ctrl,
            "Add to the selection, with a click or drag (the standard keys)",
            Anywhere,
        ),
        held(
            KeyId::Shift,
            "Take out of the selection, with a click or drag (the standard keys)",
            Anywhere,
        ),
    ]);

    // Move, rotate and scale.
    all.extend(
        transforming::TOOL_KEYS
            .iter()
            .map(|(key, tool)| plain(*key, tool.name(), Anywhere)),
    );
    all.extend(
        transforming::LOCK_KEYS
            .iter()
            .map(|(keys, axis)| bind(*keys, format!("Lock {}", ["X", "Y", "Z"][*axis]), Anywhere)),
    );
    all.extend([
        plain(transforming::AXES_KEY, "World or object axes", Anywhere),
        held(KeyId::Shift, "Ten times finer", Dragging),
        held(KeyId::Ctrl, "Snap (or not, when snapping is on)", Dragging),
        plain(Key::Escape, "Cancel the drag", Dragging),
    ]);

    // Modelling.
    all.push(plain(
        crate::tool_menu::TOOL_MENU_KEY,
        "Tool menu: every tool for the mode, at the mouse",
        Anywhere,
    ));
    all.extend(
        commands::MODELLING_RING
            .iter()
            .filter_map(|c| command(*c, Editing)),
    );
    all.extend(
        [CommandId::DeleteElements, CommandId::Dissolve]
            .into_iter()
            .filter_map(|c| command(c, Editing)),
    );
    all.extend([
        bind(modelling::WELD_TARGET_KEYS, "Weld onto a point", Editing),
        plain(Key::Escape, "Finish a slit", Editing),
        plain(Key::Enter, "Finish a slit", Editing),
        held(
            KeyId::Ctrl,
            "With the wheel: a tool's count (segments, cuts) while using it; \
             otherwise grow or shrink the selection",
            Editing,
        ),
        held(
            KeyId::Alt,
            "Slide points along their edges, dragging the Move gizmo",
            Editing,
        ),
        held(
            KeyId::Alt,
            "A ring instead of a loop, with a double-click",
            Editing,
        ),
    ]);

    // Objects.
    all.extend(
        [
            CommandId::MakeEditable,
            CommandId::Group,
            CommandId::Ungroup,
        ]
        .into_iter()
        .filter_map(|c| command(c, Model)),
    );
    all.extend(command(CommandId::ShapeList, Anywhere));
    all.extend([
        bind(
            KeyboardShortcut::new(Modifiers::SHIFT, crate::shape_picker::SHAPE_LIST_KEY),
            "Shape list, from the last shape",
            Anywhere,
        ),
        bind(
            objects::DELETE_SHORTCUT,
            "Delete the selected objects",
            Model,
        ),
        plain(Key::Backspace, "Delete the selected objects", Model),
        bind(copying::COPY_KEYS, "Copy objects", Model),
        bind(copying::PASTE_KEYS, "Paste copies", Model),
        bind(
            copying::COPY_INSTANCES_KEYS,
            "Copy objects (for instances)",
            Model,
        ),
        bind(copying::PASTE_INSTANCES_KEYS, "Paste instances", Model),
        bind(showing::HIDE_KEYS, "Hide the selection", Anywhere),
        bind(showing::ISOLATE_KEYS, "Isolate the selection", Anywhere),
        bind(showing::SHOW_ALL_KEYS, "Show everything", Anywhere),
    ]);

    // The view.
    for frame in [viewport::Frame::All, viewport::Frame::Selected] {
        all.push(plain(frame.key(), frame.label(), Anywhere));
    }
    all.extend(
        viewport::VIEW_KEYS
            .iter()
            .map(|(key, kind)| plain(*key, format!("{} view", kind.name()), Anywhere)),
    );
    all.extend([
        plain(viewport::FOUR_VIEWS_KEY, "All four views", Anywhere),
        held(
            KeyId::Alt,
            "Orbit, pan and zoom with a left, middle or right drag (standard navigation)",
            Anywhere,
        ),
    ]);

    // Menus and questions.
    all.extend([
        plain(
            Key::Escape,
            "Close the shape list, tool menu or question",
            Menus,
        ),
        plain(
            Key::Enter,
            "Choose (shape list, tool menu); Save (\"Save changes?\")",
            Menus,
        ),
        plain(Key::Space, "Choose (shape list, tool menu)", Menus),
        plain(Key::ArrowDown, "Next in the shape list or tool menu", Menus),
        plain(Key::ArrowUp, "Back in the shape list or tool menu", Menus),
        plain(Key::ArrowRight, "Next in the tool menu", Menus),
        plain(Key::ArrowLeft, "Back in the tool menu", Menus),
        bind(
            super::DISCARD_SHORTCUT,
            "Don't save (in the \"Save changes?\" question)",
            Menus,
        ),
    ]);
    all
}

/// Every mouse action.
pub(crate) fn mouse() -> Vec<MouseLine> {
    use Situation::*;
    let line = |action, how, situation| MouseLine {
        action,
        how,
        situation,
    };
    vec![
        line("Select", "Click", Anywhere),
        line(
            "Add to / take out of the selection",
            "Ctrl + click / Shift + click (the standard keys)",
            Anywhere,
        ),
        line("Box, lasso or brush select", "Drag", Anywhere),
        line(
            "Orbit, pan, zoom",
            "Your navigation controls (Options menu); as standard right drag, middle drag \
             and the wheel, and Alt + left, middle or right drag",
            Anywhere,
        ),
        line(
            "Select a loop / ring",
            "Double-click / Alt + double-click",
            Editing,
        ),
        line("Grow / shrink the selection", "Ctrl + wheel", Editing),
        line(
            "A tool's count (segments, cuts)",
            "Ctrl + wheel, while using the tool",
            Editing,
        ),
        line(
            "Slide points along their edges",
            "Alt + drag the Move gizmo",
            Editing,
        ),
        line("Cancel a drag", "Right-click (or Esc)", Dragging),
    ]
}

/// Whether `text` holds every word of `filter`, ignoring case.
fn matches(text: &str, filter: &str) -> bool {
    let text = text.to_lowercase();
    filter
        .to_lowercase()
        .split_whitespace()
        .all(|word| text.contains(word))
}

/// What the search looks through for a binding: its job and its keys.
fn search_text(binding: &Binding, ctx: &Context) -> String {
    format!("{} {}", binding.action, binding.keys(ctx))
}

/// What a drawn key says on this system (a Mac's Cmd and Option where a PC
/// has Ctrl and Alt).
fn cap_label(cap: &Cap, ctx: &Context) -> String {
    match cap.id {
        Some(id @ (KeyId::Ctrl | KeyId::Alt)) => modifier_name(ctx, id).to_string(),
        _ => cap.label.clone(),
    }
}

/// How a key is drawn this frame.
#[derive(Default)]
struct CapState {
    /// Its jobs in the situations shown.
    situations: BTreeSet<Situation>,
    /// Whether a job of it matches the search.
    found: bool,
}

impl Workbench {
    /// The menu bar's Keys button: opens or closes the window.
    pub(super) fn keys_button(&mut self, ui: &mut Ui) {
        let response = ui
            .add(egui::Button::selectable(self.shortcuts.open, KEYS_BUTTON))
            .on_hover_text("Every keyboard shortcut and mouse action, on a drawn keyboard");
        if response.clicked() {
            self.shortcuts.open = !self.shortcuts.open;
        }
    }

    /// The keyboard the window draws: the one picked there, or the one
    /// guessed from the system language.
    pub(crate) fn keyboard_layout(&self) -> KeyboardLayout {
        self.viewport
            .options()
            .keyboard
            .unwrap_or(self.system_keyboard)
    }

    /// Guesses the keyboard from the system language (such as `nb-NO`),
    /// for the Keys window until one is picked there.
    pub fn set_system_language(&mut self, language: &str) {
        self.system_keyboard = KeyboardLayout::guess(language);
    }

    /// The window, while open.
    pub(super) fn shortcut_window(&mut self, ctx: &Context) {
        if !self.shortcuts.open {
            return;
        }
        let mut layout = self.keyboard_layout();
        let picked = layout;
        let bindings = bindings();
        let mut open = true;
        let viewer = &mut self.shortcuts;
        // It opens as big as fits comfortably: most of the app's window,
        // but no wider than about twice its height.
        let screen = ctx.content_rect();
        let height = screen.height() * SCREEN_SHARE;
        let width = (screen.width() * SCREEN_SHARE).min(height * WIDEST);
        egui::Window::new(VIEWER_TITLE)
            // Its own id, so a size remembered from before it grew is
            // forgotten.
            .id(egui::Id::new("keys window, drawn keyboard"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([width, height])
            // In the middle, clear of the menu bar.
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .show(ctx, |ui| {
                let unit = key_width(ui.available_size());
                scale_text(ui, unit / NORMAL_KEY);
                header(ui, viewer, &mut layout);
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let caps = keyboard::caps(layout);
                        ui.horizontal_top(|ui| {
                            let hovered = draw_keyboard(ui, viewer, &caps, &bindings, unit);
                            // A line between the keyboard and the jobs, as tall as
                            // the keyboard.
                            let (line, _) = ui.allocate_exact_size(
                                egui::vec2((unit * 0.6).round(), keyboard::SIZE.1 * unit),
                                egui::Sense::hover(),
                            );
                            ui.painter().vline(
                                line.center().x.round(),
                                line.y_range(),
                                ui.visuals().widgets.noninteractive.bg_stroke,
                            );
                            ui.vertical(|ui| {
                                ui.set_width(unit * DETAILS_KEYS);
                                details(ui, viewer, &caps, &bindings, hovered.or(viewer.pinned));
                            });
                        });
                        ui.add_space((unit * 0.3).round());
                        mouse_section(ui, viewer);
                    });
            });
        self.shortcuts.open = open;
        if layout != picked {
            self.viewport.options_mut().keyboard = Some(layout);
        }
    }
}

/// The width of a drawn key that fills `size` (the window's inside):
/// the keyboard and the side panel across, everything down.
fn key_width(size: egui::Vec2) -> f32 {
    let across = size.x / (keyboard::SIZE.0 + DETAILS_KEYS + 1.0);
    let down = size.y / HEIGHT_KEYS;
    across.min(down).clamp(KEY_RANGE.0, KEY_RANGE.1).floor()
}

/// Makes the window's text and spacing `scale` times their normal size,
/// to go with the drawn keys.
fn scale_text(ui: &mut Ui, scale: f32) {
    let style = ui.style_mut();
    for font in style.text_styles.values_mut() {
        font.size *= scale;
    }
    // Spacing stays on whole points, so widgets line up with the pixels.
    let spacing = &mut style.spacing;
    spacing.item_spacing = (spacing.item_spacing * scale).round();
    spacing.button_padding = (spacing.button_padding * scale).round();
    spacing.interact_size = (spacing.interact_size * scale).round();
    spacing.icon_width = (spacing.icon_width * scale).round();
    spacing.icon_width_inner = (spacing.icon_width_inner * scale).round();
    spacing.icon_spacing = (spacing.icon_spacing * scale).round();
    spacing.combo_width = (spacing.combo_width * scale).round();
    spacing.text_edit_width = (spacing.text_edit_width * scale).round();
}

/// The search, the keyboard picker and the situation chips.
fn header(ui: &mut Ui, viewer: &mut ShortcutViewer, layout: &mut KeyboardLayout) {
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut viewer.filter)
                .hint_text(FILTER_HINT)
                .desired_width((ui.spacing().text_edit_width * 0.8).round()),
        );
        egui::ComboBox::from_label(LAYOUT_LABEL)
            .selected_text(layout.name())
            .show_ui(ui, |ui| {
                for choice in KeyboardLayout::ALL {
                    ui.selectable_value(layout, choice, choice.name());
                }
            })
            .response
            .on_hover_text(
                "Which keyboard to draw. Letters follow what is printed on your keys; \
                 the number row is always 1 to 0.",
            );
    });
    // The situations, each a chip with its colour: click one to hide or
    // show its keys.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = (ui.spacing().item_spacing.x * 0.5).round();
        for situation in Situation::ALL {
            let shown = !viewer.hidden.contains(&situation);
            let colour = situation.colour();
            ui.label(egui::RichText::new("●").color(if shown {
                colour
            } else {
                theme::TEXT_WEAK.gamma_multiply(0.5)
            }));
            let text = egui::RichText::new(situation.name()).color(if shown {
                theme::TEXT_STRONG
            } else {
                theme::TEXT_WEAK
            });
            let chip = if shown {
                egui::Button::new(text)
                    .fill(colour.gamma_multiply(0.18))
                    .stroke(egui::Stroke::new(1.0, colour.gamma_multiply(0.6)))
            } else {
                egui::Button::new(text)
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(1.0, theme::HOVER))
            };
            let chip = ui
                .add(chip.corner_radius(ui.spacing().interact_size.y * 0.5))
                .on_hover_text("Click to hide or show the keys for this situation");
            if chip.clicked() {
                if shown {
                    viewer.hidden.insert(situation);
                } else {
                    viewer.hidden.remove(&situation);
                }
            }
            ui.add_space(ui.spacing().item_spacing.x * 2.0);
        }
    });
}

/// The bindings shown: those whose situation is not switched off.
fn visible<'b>(
    bindings: &'b [Binding],
    viewer: &ShortcutViewer,
) -> impl Iterator<Item = &'b Binding> {
    let hidden = viewer.hidden.clone();
    bindings
        .iter()
        .filter(move |b| !hidden.contains(&b.situation))
}

/// Draws the keyboard, filling the width left beside the side panel.
/// Returns the key pointed at, if any.
fn draw_keyboard(
    ui: &mut Ui,
    viewer: &mut ShortcutViewer,
    caps: &[Cap],
    bindings: &[Binding],
    unit: f32,
) -> Option<KeyId> {
    let ctx = ui.ctx().clone();
    let scale = unit / NORMAL_KEY;
    let gap = unit * 0.1;
    let corner = unit * 0.1;
    let (area, _) = ui.allocate_exact_size(
        egui::vec2(keyboard::SIZE.0 * unit, keyboard::SIZE.1 * unit),
        egui::Sense::hover(),
    );
    let searching = !viewer.filter.trim().is_empty();
    let accent = theme::accent(&ctx).colors();
    let mut hovered = None;
    for (index, cap) in caps.iter().enumerate() {
        let rect = egui::Rect::from_min_size(
            area.min + egui::vec2(cap.x * unit, cap.y * unit),
            egui::vec2(cap.w * unit - gap, cap.h * unit - gap),
        );
        let mut state = CapState::default();
        if let Some(id) = cap.id {
            for b in visible(bindings, viewer).filter(|b| b.key == id) {
                state.situations.insert(b.situation);
                state.found |= searching && matches(&search_text(b, &ctx), &viewer.filter);
            }
        }
        let label = cap_label(cap, &ctx);
        let used = !state.situations.is_empty();
        let mut is_hovered = false;
        // Every printed key can be pointed at (and found by screen
        // readers and tests); keys the app reads can be clicked.
        if !label.is_empty() {
            let sense = if cap.id.is_some() {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let response = ui.interact(rect, ui.id().with(("key", index)), sense);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Key {label}"))
            });
            if response.hovered() {
                hovered = cap.id;
                is_hovered = cap.id.is_some();
            }
            if let Some(id) = cap.id.filter(|_| response.clicked()) {
                viewer.pinned = if viewer.pinned == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
        }
        let pinned = cap.id.is_some() && viewer.pinned == cap.id;
        let lit = used && (!searching || state.found);
        let fill = if pinned {
            accent.fill
        } else if is_hovered {
            Color32::from_rgb(0x4a, 0x4a, 0x4a)
        } else if lit {
            Color32::from_rgb(0x3a, 0x3a, 0x3a)
        } else {
            theme::FIELD
        };
        let painter = ui.painter();
        painter.rect_filled(rect, corner, fill);
        let outline = if state.found {
            egui::Stroke::new(2.0 * scale, accent.light)
        } else {
            egui::Stroke::new(1.0, theme::BORDER)
        };
        painter.rect_stroke(rect, corner, outline, egui::StrokeKind::Inside);
        let text_colour = if pinned || lit {
            theme::TEXT_STRONG
        } else if used {
            theme::TEXT_WEAK
        } else {
            Color32::from_rgb(0x5a, 0x5a, 0x5a)
        };
        let size = if label.chars().count() > 3 {
            unit * 0.26
        } else {
            unit * 0.36
        };
        painter.text(
            rect.center() - egui::vec2(0.0, unit * 0.06),
            egui::Align2::CENTER_CENTER,
            &label,
            egui::FontId::proportional(size),
            text_colour,
        );
        // A small coloured mark for each situation it works in.
        let mark = egui::vec2(unit * 0.16, 3.0 * scale);
        for (i, situation) in state.situations.iter().enumerate() {
            let min = rect.left_bottom()
                + egui::vec2(
                    5.0 * scale + i as f32 * (mark.x + 2.0 * scale),
                    -7.0 * scale,
                );
            let colour = if lit || pinned {
                situation.colour()
            } else {
                situation.colour().gamma_multiply(0.35)
            };
            painter.rect_filled(egui::Rect::from_min_size(min, mark), 1.0, colour);
        }
    }
    hovered
}

/// The name of a key, as drawn.
fn key_name(caps: &[Cap], id: KeyId, ctx: &Context) -> String {
    caps.iter()
        .find(|c| c.id == Some(id))
        .map(|c| cap_label(c, ctx))
        .unwrap_or_default()
}

/// One job, in the side panel or the search's matches.
fn job_row(ui: &mut Ui, binding: &Binding) {
    ui.horizontal_top(|ui| {
        ui.label(egui::RichText::new("●").color(binding.situation.colour()))
            .on_hover_text(binding.situation.name());
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(binding.keys(ui.ctx()))
                    .monospace()
                    .color(theme::TEXT_WEAK),
            );
            ui.add(egui::Label::new(&binding.action).wrap());
        });
    });
}

/// The side panel: the jobs of the key pointed at or clicked, or the
/// search's matches, or a hint.
fn details(
    ui: &mut Ui,
    viewer: &ShortcutViewer,
    caps: &[Cap],
    bindings: &[Binding],
    focus: Option<KeyId>,
) {
    let ctx = ui.ctx().clone();
    if let Some(id) = focus {
        ui.label(egui::RichText::new(focus_title(caps, id, &ctx)).strong());
        ui.add_space(4.0);
        let own: Vec<&Binding> = visible(bindings, viewer).filter(|b| b.key == id).collect();
        let with: Vec<&Binding> = visible(bindings, viewer)
            .filter(|b| b.held_with(id))
            .collect();
        if own.is_empty() && with.is_empty() {
            ui.label(egui::RichText::new("Not used.").color(theme::TEXT_WEAK));
        }
        for b in &own {
            job_row(ui, b);
        }
        if !with.is_empty() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Held with").strong());
            for b in &with {
                job_row(ui, b);
            }
        }
        return;
    }
    if !viewer.filter.trim().is_empty() {
        let found: Vec<&Binding> = visible(bindings, viewer)
            .filter(|b| matches(&search_text(b, &ctx), &viewer.filter))
            .collect();
        let n = found.len();
        ui.label(
            egui::RichText::new(match n {
                0 => "No keys match.".to_string(),
                1 => "1 match".to_string(),
                n => format!("{n} matches"),
            })
            .strong(),
        );
        ui.add_space(4.0);
        for b in found {
            job_row(ui, b);
        }
        return;
    }
    ui.label(egui::RichText::new(HINT).color(theme::TEXT_WEAK));
}

/// The side panel's heading for a key: its name, and "(clicked)" when it
/// stays shown.
fn focus_title(caps: &[Cap], id: KeyId, ctx: &Context) -> String {
    let name = key_name(caps, id, ctx);
    match id {
        KeyId::Shift | KeyId::Ctrl | KeyId::Alt => format!("{name} (held)"),
        KeyId::Key(_) => name,
    }
}

/// The mouse actions under the keyboard, narrowed by the search and the
/// chips.
fn mouse_section(ui: &mut Ui, viewer: &ShortcutViewer) {
    ui.label(egui::RichText::new(MOUSE_HEADING).strong());
    ui.separator();
    let lines: Vec<MouseLine> = mouse()
        .into_iter()
        .filter(|l| !viewer.hidden.contains(&l.situation))
        .filter(|l| matches(&format!("{} {}", l.action, l.how), &viewer.filter))
        .collect();
    if lines.is_empty() {
        ui.label(egui::RichText::new("Nothing matches.").color(theme::TEXT_WEAK));
        return;
    }
    egui::Grid::new("mouse actions")
        .num_columns(3)
        .spacing(egui::vec2(12.0, 4.0))
        .striped(true)
        .show(ui, |ui| {
            for l in lines {
                ui.label(egui::RichText::new("●").color(l.situation.colour()))
                    .on_hover_text(l.situation.name());
                ui.label(l.action);
                ui.add(egui::Label::new(egui::RichText::new(l.how).color(theme::TEXT_WEAK)).wrap());
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    /// A context that has drawn one frame: on a Mac, writing out a
    /// shortcut needs the fonts, which load on the first frame.
    fn ready_context() -> Context {
        let ctx = Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx
    }

    #[test]
    fn every_command_with_a_key_is_listed_with_it() {
        let all = bindings();
        for command in commands::ALL {
            let Some(keys) = command.info().shortcut else {
                continue;
            };
            assert!(
                all.iter().any(|b| b.key == KeyId::Key(keys.logical_key)
                    && b.modifiers == keys.modifiers),
                "{command:?} isn't listed"
            );
        }
    }

    #[test]
    fn every_key_used_is_on_every_keyboard() {
        for layout in KeyboardLayout::ALL {
            let caps = keyboard::caps(layout);
            for b in bindings() {
                assert!(
                    caps.iter().any(|c| c.id == Some(b.key)),
                    "{layout:?} has no key for {:?} ({})",
                    b.key,
                    b.action
                );
            }
        }
    }

    #[test]
    fn every_binding_reads_well() {
        let ctx = ready_context();
        for b in bindings() {
            assert!(!b.action.is_empty(), "{b:?}");
            assert!(!b.keys(&ctx).is_empty(), "{b:?}");
        }
        let hold = held(KeyId::Shift, "x", Situation::Dragging);
        assert_eq!(hold.keys(&ctx), "Hold Shift");
        for l in mouse() {
            assert!(!l.action.is_empty() && !l.how.is_empty());
        }
    }

    #[test]
    fn keys_fill_the_window_within_limits() {
        // A small window: the smallest keys, and the window scrolls.
        assert_eq!(key_width(egui::vec2(400.0, 300.0)), KEY_RANGE.0);
        // A wide, short window: its height decides.
        let wide = key_width(egui::vec2(3000.0, 1000.0));
        assert_eq!(wide, (1000.0 / HEIGHT_KEYS).floor());
        // A narrow, tall one: its width decides.
        let tall = key_width(egui::vec2(1000.0, 3000.0));
        assert_eq!(
            tall,
            (1000.0 / (keyboard::SIZE.0 + DETAILS_KEYS + 1.0)).floor()
        );
        // Never bigger than the largest.
        assert_eq!(key_width(egui::vec2(9000.0, 9000.0)), KEY_RANGE.1);
    }

    #[test]
    fn the_filter_matches_every_word() {
        assert!(matches("Hide the selection Shift+H", ""));
        assert!(matches("Hide the selection Shift+H", "hide"));
        assert!(matches("Hide the selection Shift+H", "SHIFT selection"));
        assert!(!matches("Hide the selection Shift+H", "show"));
    }

    fn open_window() -> egui_kittest::Harness<'static, crate::workbench::tests::State> {
        let mut harness = crate::workbench::tests::fresh();
        crate::workbench::tests::main_menu(&harness, KEYS_BUTTON).click();
        harness.run();
        harness
    }

    fn key<'h>(
        harness: &'h egui_kittest::Harness<'static, crate::workbench::tests::State>,
        label: &str,
    ) -> egui_kittest::Node<'h> {
        harness
            .get_all_by_label(Box::leak(format!("Key {label}").into_boxed_str()))
            .next()
            .unwrap_or_else(|| panic!("no key {label}"))
    }

    #[test]
    fn the_keys_button_opens_a_drawn_keyboard() {
        let mut harness = open_window();
        assert!(harness.state().workbench.shortcuts.open);
        assert!(harness.query_by_label(VIEWER_TITLE).is_some());
        key(&harness, "H");
        key(&harness, "Backspace");
        assert!(harness.query_by_label(MOUSE_HEADING).is_some());
        assert!(harness.query_by_label(HINT).is_some());
        // The button closes it again.
        crate::workbench::tests::main_menu(&harness, KEYS_BUTTON).click();
        harness.run();
        assert!(!harness.state().workbench.shortcuts.open);
    }

    #[test]
    fn clicking_a_key_keeps_its_jobs_shown() {
        let mut harness = open_window();
        key(&harness, "H").click();
        harness.run();
        assert_eq!(
            harness.state().workbench.shortcuts.pinned,
            Some(KeyId::Key(Key::H))
        );
        assert!(harness.query_by_label("Hide the selection").is_some());
        assert!(harness.query_by_label("Show everything").is_some());
        assert!(
            harness
                .query_by_label(viewport::Frame::All.label())
                .is_some()
        );
        // Clicking it again lets go.
        key(&harness, "H").click();
        harness.run();
        assert_eq!(harness.state().workbench.shortcuts.pinned, None);
    }

    #[test]
    fn a_modifier_shows_what_holding_it_does_and_what_it_is_held_with() {
        let mut harness = open_window();
        key(&harness, "Shift").click();
        harness.run();
        assert!(harness.query_by_label("Ten times finer").is_some());
        assert!(harness.query_by_label("Held with").is_some());
        assert!(harness.query_by_label("Hide the selection").is_some());
    }

    #[test]
    fn the_search_and_the_chips_narrow_the_jobs() {
        let mut harness = open_window();
        harness.state_mut().workbench.shortcuts.filter = "weld".into();
        harness.run();
        assert!(harness.query_by_label("Weld onto a point").is_some());
        assert!(harness.query_by_label("Hide the selection").is_none());
        // Switching off the element modes' chip hides their keys.
        harness.get_by_label(Situation::Editing.name()).click();
        harness.run();
        assert!(
            harness
                .state()
                .workbench
                .shortcuts
                .hidden
                .contains(&Situation::Editing)
        );
        assert!(harness.query_by_label("Weld onto a point").is_none());
        assert!(harness.query_by_label("No keys match.").is_some());
    }

    #[test]
    fn the_keyboard_is_guessed_from_the_language_and_a_pick_is_remembered() {
        let mut harness = crate::workbench::tests::fresh();
        harness.state_mut().workbench.set_system_language("nb-NO");
        crate::workbench::tests::main_menu(&harness, KEYS_BUTTON).click();
        harness.run();
        assert_eq!(
            harness.state().workbench.keyboard_layout(),
            KeyboardLayout::Norwegian
        );
        key(&harness, "Ø");
        assert_eq!(harness.state().workbench.viewport.options().keyboard, None);
        // Picking another keyboard draws it and remembers it.
        harness.get_by_label(LAYOUT_LABEL).click();
        harness.run();
        harness.get_by_label(KeyboardLayout::German.name()).click();
        harness.run();
        key(&harness, "Ü");
        assert_eq!(
            harness.state().workbench.viewport.options().keyboard,
            Some(KeyboardLayout::German)
        );
    }
}
