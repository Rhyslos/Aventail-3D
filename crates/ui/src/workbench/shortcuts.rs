//! The keyboard shortcut viewer: every key and mouse action in one
//! window, opened by **Keys** on the menu bar, next to Help
//! ([decision 0056](../../../../docs/decisions/0056-shortcut-viewer.md)).
//!
//! The keys are read from the same constants the app acts on (the command
//! list, the tool, mode and file keys), so the viewer can't drift from
//! what the keys do. Mouse actions, which have no constants, are written
//! out here.

use super::Workbench;
use super::{copying, modelling, showing};
use crate::commands::{self, CommandId};
use crate::palette::Mode;
use crate::{objects, project, selecting, settings, transforming, viewport};
use egui::{Context, KeyboardShortcut, Modifiers, Ui};

/// The window's title, and the menu bar button's name.
pub(crate) const VIEWER_TITLE: &str = "Keyboard shortcuts";
/// The menu bar button that opens it.
pub(crate) const KEYS_BUTTON: &str = "Keys";
/// The width of the actions' column, so every group's keys line up.
const ACTION_WIDTH: f32 = 300.0;

/// The search field's hint.
pub(crate) const FILTER_HINT: &str = "Find a key or action";

/// One line of the viewer: what it does, and its keys (as the system
/// writes them: Ctrl on Windows and Linux, ⌘ on a Mac).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) action: String,
    pub(crate) keys: String,
}

/// A heading and its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) title: &'static str,
    pub(crate) lines: Vec<Line>,
}

/// The viewer's own state.
#[derive(Debug, Default)]
pub(crate) struct ShortcutViewer {
    pub(crate) open: bool,
    filter: String,
}

fn line(action: impl Into<String>, keys: impl Into<String>) -> Line {
    Line {
        action: action.into(),
        keys: keys.into(),
    }
}

/// Every shortcut, grouped, with keys written for this system.
pub(crate) fn groups(ctx: &Context) -> Vec<Group> {
    let k = |s: &KeyboardShortcut| ctx.format_shortcut(s);
    let plain = |key: egui::Key| k(&KeyboardShortcut::new(Modifiers::NONE, key));
    let command = |id: CommandId| {
        let info = id.info();
        line(info.name, info.shortcut.map(|s| k(&s)).unwrap_or_default())
    };

    let mut file = vec![
        line("New project", k(&project::NEW_SHORTCUT)),
        line("Open a project", k(&project::OPEN_SHORTCUT)),
        line("Save", k(&project::SAVE_SHORTCUT)),
        line("Save as", k(&project::SAVE_AS_SHORTCUT)),
        line("Undo", k(&objects::UNDO_SHORTCUT)),
        line(
            "Redo",
            format!(
                "{} or {}",
                k(&objects::REDO_SHORTCUT),
                k(&objects::REDO_SHORTCUT_2)
            ),
        ),
        line("Settings", k(&settings::SETTINGS_SHORTCUT)),
    ];
    file.push(line(
        "Don't save (in the \"Save changes?\" question)",
        k(&super::DISCARD_SHORTCUT),
    ));

    let modes = Mode::ALL
        .into_iter()
        .filter_map(|mode| {
            let key = mode.key()?;
            Some(line(
                format!("{} mode (again: back to Model)", mode.name()),
                plain(key),
            ))
        })
        .collect();

    let select = vec![
        line("Select", "Click"),
        line("Add to the selection", "Ctrl + click (the standard keys)"),
        line(
            "Take out of the selection",
            "Shift + click (the standard keys)",
        ),
        line("Box, lasso or brush select", "Drag"),
        line(
            "Next select tool (Box, Lasso, Paint select)",
            plain(selecting::SELECT_TOOL_KEY),
        ),
        line("Select all", k(&selecting::SELECT_ALL_SHORTCUT)),
        line("Invert the selection", k(&selecting::INVERT_SHORTCUT)),
        line("Select nothing", k(&objects::DESELECT_SHORTCUT)),
        line("Loop / ring", "Double-click / Alt + double-click"),
        line(
            "Grow / shrink the selection",
            "Ctrl + wheel (with a counted tool: its count)",
        ),
    ];

    let mut transform: Vec<Line> = transforming::TOOL_KEYS
        .iter()
        .map(|(key, tool)| line(tool.name(), plain(*key)))
        .collect();
    transform.extend(
        transforming::LOCK_KEYS
            .iter()
            .map(|(keys, axis)| line(format!("Lock {}", ["X", "Y", "Z"][*axis]), k(keys))),
    );
    transform.extend([
        line("World or object axes", plain(transforming::AXES_KEY)),
        line(
            "Slide points along their edges",
            "Alt + drag the Move gizmo",
        ),
        line("Ten times finer, while dragging", "Hold Shift"),
        line("Snap (or not), while dragging", "Hold Ctrl"),
        line("Cancel a drag", "Esc or right-click"),
    ]);

    let mut modelling_lines: Vec<Line> = vec![line(
        "Tool menu: every tool for the mode, at the mouse",
        plain(crate::tool_menu::TOOL_MENU_KEY),
    )];
    modelling_lines.extend(
        commands::MODELLING_RING
            .iter()
            .filter(|c| c.info().shortcut.is_some())
            .map(|c| command(*c)),
    );
    modelling_lines.extend([
        line("Weld onto a point", k(&modelling::WELD_TARGET_KEYS)),
        line(
            "A tool's count (segments, cuts) while using it",
            "Ctrl + wheel",
        ),
        line(
            "Finish a slit",
            format!(
                "{} or {}",
                plain(egui::Key::Escape),
                plain(egui::Key::Enter)
            ),
        ),
    ]);

    let mut objects_lines: Vec<Line> = vec![
        command(CommandId::ShapeList),
        command(CommandId::MakeEditable),
        command(CommandId::Group),
        command(CommandId::Ungroup),
        line(
            "Delete the selected objects",
            format!(
                "{} or {}",
                k(&objects::DELETE_SHORTCUT),
                plain(egui::Key::Backspace)
            ),
        ),
        line("Copy objects", k(&copying::COPY_KEYS)),
        line("Paste copies", k(&copying::PASTE_KEYS)),
        line(
            "Copy objects (for instances)",
            k(&copying::COPY_INSTANCES_KEYS),
        ),
        line("Paste instances", k(&copying::PASTE_INSTANCES_KEYS)),
    ];
    objects_lines.extend([
        line("Hide the selection", k(&showing::HIDE_KEYS)),
        line("Isolate the selection", k(&showing::ISOLATE_KEYS)),
        line("Show everything", k(&showing::SHOW_ALL_KEYS)),
    ]);

    let mut view: Vec<Line> = vec![
        line(
            viewport::Frame::All.label(),
            plain(viewport::Frame::All.key()),
        ),
        line(
            viewport::Frame::Selected.label(),
            plain(viewport::Frame::Selected.key()),
        ),
    ];
    view.extend(
        viewport::VIEW_KEYS
            .iter()
            .map(|(key, kind)| line(format!("{} view", kind.name()), plain(*key))),
    );
    view.extend([
        line("All four views", plain(viewport::FOUR_VIEWS_KEY)),
        line(
            "Orbit, pan, zoom",
            "Your navigation controls (Options menu); as standard right drag, middle drag \
             and the wheel, and Alt + left, middle or right drag",
        ),
    ]);

    vec![
        Group {
            title: "File and editing",
            lines: file,
        },
        Group {
            title: "Modes",
            lines: modes,
        },
        Group {
            title: "Selecting",
            lines: select,
        },
        Group {
            title: "Move, rotate and scale",
            lines: transform,
        },
        Group {
            title: "Modelling (Points, Edges and Polygons modes)",
            lines: modelling_lines,
        },
        Group {
            title: "Objects",
            lines: objects_lines,
        },
        Group {
            title: "The view",
            lines: view,
        },
    ]
}

/// Whether `line` matches every word of `filter` (in its action or keys,
/// ignoring case).
fn matches(line: &Line, filter: &str) -> bool {
    let text = format!("{} {}", line.action, line.keys).to_lowercase();
    filter
        .to_lowercase()
        .split_whitespace()
        .all(|word| text.contains(word))
}

impl Workbench {
    /// The menu bar's Keys button: opens or closes the viewer.
    pub(super) fn keys_button(&mut self, ui: &mut Ui) {
        let response = ui
            .add(egui::Button::selectable(self.shortcuts.open, KEYS_BUTTON))
            .on_hover_text("Every keyboard shortcut and mouse action in one place");
        if response.clicked() {
            self.shortcuts.open = !self.shortcuts.open;
        }
    }

    /// The viewer's window, while open.
    pub(super) fn shortcut_window(&mut self, ctx: &Context) {
        if !self.shortcuts.open {
            return;
        }
        let groups = groups(ctx);
        let mut open = true;
        let viewer = &mut self.shortcuts;
        egui::Window::new(VIEWER_TITLE)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(600.0)
            .max_width(640.0)
            .default_height(520.0)
            // In the middle, clear of the menu bar.
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut viewer.filter)
                        .hint_text(FILTER_HINT)
                        .desired_width(ui.available_width()),
                );
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut shown = 0;
                        for group in &groups {
                            let lines: Vec<&Line> = group
                                .lines
                                .iter()
                                .filter(|l| matches(l, &viewer.filter))
                                .collect();
                            if lines.is_empty() {
                                continue;
                            }
                            shown += lines.len();
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(group.title).strong());
                            ui.separator();
                            egui::Grid::new(group.title)
                                .num_columns(2)
                                .min_col_width(ACTION_WIDTH)
                                .spacing(egui::vec2(16.0, 4.0))
                                .striped(true)
                                .show(ui, |ui| {
                                    for l in lines {
                                        ui.label(&l.action);
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&l.keys).monospace(),
                                            )
                                            .wrap(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                        if shown == 0 {
                            ui.label("Nothing matches.");
                        }
                    });
            });
        self.shortcuts.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context that has drawn one frame: on a Mac, writing out a
    /// shortcut needs the fonts, which load on the first frame.
    fn ready_context() -> Context {
        let ctx = Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx
    }

    fn all_text(ctx: &Context) -> String {
        groups(ctx)
            .iter()
            .flat_map(|g| g.lines.iter())
            .map(|l| format!("{} = {}\n", l.action, l.keys))
            .collect()
    }

    #[test]
    fn every_command_with_a_key_is_listed_with_it() {
        let ctx = ready_context();
        let text = all_text(&ctx);
        for command in commands::ALL {
            let Some(keys) = command.info().shortcut else {
                continue;
            };
            // Delete and Backspace in the element modes, and Delete for
            // objects, are written out in their own lines.
            if matches!(
                command,
                CommandId::DeleteElements | CommandId::Dissolve | CommandId::Delete
            ) {
                continue;
            }
            let wanted = format!("= {}", ctx.format_shortcut(&keys));
            assert!(
                text.lines()
                    .any(|l| l.starts_with(command.name()) && l.ends_with(&wanted[2..])),
                "{command:?} ({wanted}) isn't listed"
            );
        }
    }

    #[test]
    fn every_line_has_an_action_and_keys_and_groups_have_lines() {
        let ctx = ready_context();
        for group in groups(&ctx) {
            assert!(!group.lines.is_empty(), "{}", group.title);
            for l in &group.lines {
                assert!(!l.action.is_empty() && !l.keys.is_empty(), "{l:?}");
            }
        }
    }

    #[test]
    fn the_filter_matches_every_word_in_either_column() {
        let l = line("Hide the selection", "Shift+H");
        assert!(matches(&l, ""));
        assert!(matches(&l, "hide"));
        assert!(matches(&l, "SHIFT selection"));
        assert!(!matches(&l, "show"));
    }

    #[test]
    fn the_keys_button_opens_the_viewer_and_the_filter_narrows_it() {
        use egui_kittest::kittest::Queryable;
        let mut harness = crate::workbench::tests::fresh();
        let button = crate::workbench::tests::main_menu(&harness, KEYS_BUTTON);
        button.click();
        harness.run();
        assert!(harness.state().workbench.shortcuts.open);
        assert!(harness.query_by_label(VIEWER_TITLE).is_some());
        assert!(harness.query_by_label("Hide the selection").is_some());
        assert!(harness.query_by_label("Slit").is_some());
        // Typing narrows the list.
        harness.state_mut().workbench.shortcuts.filter = "weld".into();
        harness.run();
        assert!(harness.query_by_label("Weld onto a point").is_some());
        assert!(harness.query_by_label("Hide the selection").is_none());
        // The button closes it again.
        crate::workbench::tests::main_menu(&harness, KEYS_BUTTON).click();
        harness.run();
        assert!(!harness.state().workbench.shortcuts.open);
    }
}
