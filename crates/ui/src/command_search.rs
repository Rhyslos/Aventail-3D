//! Command search ([decision 0126](../../../docs/decisions/0126-rework-and-first-use.md)):
//! Space opens a box at the top of the window; typing a few letters of a
//! command's name (or a word from what it does) lists the matches, best
//! first. Enter or a click runs the highlighted one, the arrow keys move,
//! Esc closes. Every command the app has is in it, with its key, so it
//! also teaches the keys.

use crate::commands::CommandId;
use crate::palette::Mode;
use egui::{Context, Key, KeyboardShortcut, Modifiers};
use scene::PrimitiveKind;

/// The key that opens the search (the project lead's choice).
pub(crate) const SEARCH_KEY: Key = Key::Space;

/// How many matches are listed at most.
pub(crate) const MAX_SHOWN: usize = 12;

/// The search box's accessible name.
pub(crate) const SEARCH_LABEL: &str = "Command search box";

/// What a found entry does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchAction {
    /// A tool menu command.
    Command(CommandId),
    /// Switch to a mode.
    Mode(Mode),
    /// Make a ready-made shape.
    Create(PrimitiveKind),
    /// One of the menus' other entries.
    Menu(MenuAction),
}

/// Menu entries that aren't tool menu commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    New,
    Open,
    Save,
    SaveAs,
    Import,
    Export,
    CheckForBaking,
    Settings,
    Keys,
    Manual,
    Undo,
    Redo,
    SelectAll,
    SelectNothing,
    InvertSelection,
    SelectConnected,
    SelectSeams,
    SelectHardEdges,
    SelectFlipped,
    Hide,
    Isolate,
    ShowAll,
    UvEditor,
    ShowHardEdges,
    ShowSeams,
    ShowNormals,
    HighContrast,
    EditPivot,
    Group,
}

impl MenuAction {
    pub(crate) const ALL: [MenuAction; 29] = [
        Self::New,
        Self::Open,
        Self::Save,
        Self::SaveAs,
        Self::Import,
        Self::Export,
        Self::CheckForBaking,
        Self::Settings,
        Self::Keys,
        Self::Manual,
        Self::Undo,
        Self::Redo,
        Self::SelectAll,
        Self::SelectNothing,
        Self::InvertSelection,
        Self::SelectConnected,
        Self::SelectSeams,
        Self::SelectHardEdges,
        Self::SelectFlipped,
        Self::Hide,
        Self::Isolate,
        Self::ShowAll,
        Self::UvEditor,
        Self::ShowHardEdges,
        Self::ShowSeams,
        Self::ShowNormals,
        Self::HighContrast,
        Self::EditPivot,
        Self::Group,
    ];

    /// Its name, where it lives, and what it does.
    pub(crate) fn info(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::New => ("New project", "File", "Start a new, empty project"),
            Self::Open => ("Open…", "File", "Open a saved project"),
            Self::Save => ("Save", "File", "Save the project"),
            Self::SaveAs => (
                "Save As…",
                "File",
                "Save the project under a new name or in another folder",
            ),
            Self::Import => ("Import…", "File", "Bring in a model from an OBJ file"),
            Self::Export => (
                "Export…",
                "File",
                "Save the model for a game engine or a texturing program (glTF or OBJ)",
            ),
            Self::CheckForBaking => (
                "Check for baking…",
                "File",
                "List what would spoil a bake: overlapping pieces, missing UVs and more",
            ),
            Self::Settings => (
                "Settings…",
                "Edit",
                "Every preference in one place: look, units, navigation, undo",
            ),
            Self::Keys => ("Keys", "Menu bar", "See every key on a drawn keyboard"),
            Self::Manual => (
                "User manual",
                "Help",
                "Open the user manual in your web browser",
            ),
            Self::Undo => ("Undo", "Edit", "Take back the last step"),
            Self::Redo => ("Redo", "Edit", "Do again what was taken back"),
            Self::SelectAll => ("Select all", "Select", "Select everything"),
            Self::SelectNothing => ("Select nothing", "Select", "Clear the selection"),
            Self::InvertSelection => (
                "Invert the selection",
                "Select",
                "Select what isn't selected, and unselect what is",
            ),
            Self::SelectConnected => (
                "Select connected",
                "Select",
                "The whole part the selection is on",
            ),
            Self::SelectSeams => ("Select seams", "Select", "Every seam of the shapes"),
            Self::SelectHardEdges => (
                "Select hard edges",
                "Select",
                "Every hard edge of the shapes",
            ),
            Self::SelectFlipped => (
                "Select flipped",
                "Select",
                "The polygons facing the wrong way",
            ),
            Self::Hide => ("Hide the selection", "View", "Hide what is selected"),
            Self::Isolate => (
                "Isolate the selection",
                "View",
                "Hide everything except what is selected",
            ),
            Self::ShowAll => ("Show everything", "View", "Show everything hidden"),
            Self::UvEditor => (
                "UV Editor",
                "Window",
                "Show the UV Editor beside the viewport, or put it away",
            ),
            Self::ShowHardEdges => (
                "Show hard edges",
                "Display",
                "Mark hard edges while editing points, edges or polygons",
            ),
            Self::ShowSeams => (
                "Show seams",
                "Display",
                "Mark seams while editing points, edges or polygons",
            ),
            Self::ShowNormals => (
                "Show normals",
                "Display",
                "Show which way each polygon faces, as short lines",
            ),
            Self::HighContrast => (
                "High contrast",
                "Settings",
                "Draw the selection larger, with a dark rim",
            ),
            Self::EditPivot => (
                "Edit pivot",
                "Tools",
                "Move and turn the selected objects' pivots, not their shapes",
            ),
            Self::Group => (
                "Group",
                "Create",
                "An empty object to hold and organise other objects",
            ),
        }
    }

    /// Its key, if it has one.
    pub(crate) fn shortcut(self) -> Option<KeyboardShortcut> {
        use crate::workbench::showing;
        Some(match self {
            Self::New => crate::project::NEW_SHORTCUT,
            Self::Open => crate::project::OPEN_SHORTCUT,
            Self::Save => crate::project::SAVE_SHORTCUT,
            Self::SaveAs => crate::project::SAVE_AS_SHORTCUT,
            Self::Settings => crate::settings::SETTINGS_SHORTCUT,
            Self::Undo => crate::objects::UNDO_SHORTCUT,
            Self::Redo => crate::objects::REDO_SHORTCUT,
            Self::SelectAll => crate::selecting::SELECT_ALL_SHORTCUT,
            Self::SelectNothing => crate::objects::DESELECT_SHORTCUT,
            Self::InvertSelection => crate::selecting::INVERT_SHORTCUT,
            Self::SelectConnected => crate::selecting::CONNECTED_SHORTCUT,
            Self::Hide => showing::HIDE_KEYS,
            Self::Isolate => showing::ISOLATE_KEYS,
            Self::ShowAll => showing::SHOW_ALL_KEYS,
            _ => return None,
        })
    }
}

/// One entry the search can find.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchEntry {
    /// Its name.
    pub(crate) name: String,
    /// Where it lives, such as "Mesh" or "File".
    pub(crate) place: String,
    /// What it does.
    pub(crate) help: String,
    /// Its key, as shown.
    pub(crate) keys: Option<String>,
    /// Why it can't be used now, if it can't.
    pub(crate) not_now: Option<String>,
    pub(crate) action: SearchAction,
}

/// How well `entry` matches `query` (lower is better), or `None` for no
/// match. Every word of the query must appear in the name or the help; a
/// name starting with the query beats a word of the name starting with
/// it, which beats a match only in the help.
pub(crate) fn score(entry: &SearchEntry, query: &str) -> Option<u32> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(10);
    }
    let name = entry.name.to_lowercase();
    let help = entry.help.to_lowercase();
    let place = entry.place.to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    let everywhere = |w: &str| name.contains(w) || help.contains(w) || place.contains(w);
    if !words.iter().all(|w| everywhere(w)) {
        return None;
    }
    let starts_word = |text: &str, w: &str| {
        text.split(|c: char| !c.is_alphanumeric())
            .any(|part| part.starts_with(w))
    };
    let base = if name.starts_with(&query) {
        0
    } else if words.iter().all(|w| starts_word(&name, w)) {
        1
    } else if words.iter().all(|w| name.contains(w)) {
        2
    } else if words
        .iter()
        .all(|w| starts_word(&help, w) || starts_word(&name, w))
    {
        3
    } else {
        4
    };
    // Things that can't be used now sort below the rest of their kind.
    Some(base * 2 + u32::from(entry.not_now.is_some()))
}

/// The entries matching `query`, best first (ties keep their order).
pub(crate) fn matches<'a>(entries: &'a [SearchEntry], query: &str) -> Vec<&'a SearchEntry> {
    let mut found: Vec<(u32, usize, &SearchEntry)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| score(e, query).map(|s| (s, i, e)))
        .collect();
    found.sort_by_key(|(s, i, _)| (*s, *i));
    found
        .into_iter()
        .map(|(_, _, e)| e)
        .take(MAX_SHOWN)
        .collect()
}

/// The search while open.
#[derive(Debug, Clone, Default)]
pub(crate) struct CommandSearch {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) highlight: usize,
    /// The text box takes the keyboard the first frame it shows.
    focus_pending: bool,
}

impl CommandSearch {
    pub(crate) fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.highlight = 0;
        self.focus_pending = true;
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.query.clear();
    }

    /// Draws the search, if open, and returns the action chosen.
    pub(crate) fn show(&mut self, ctx: &Context, entries: &[SearchEntry]) -> Option<SearchAction> {
        if !self.open {
            return None;
        }
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if escape {
            self.close();
            return None;
        }
        let found = matches(entries, &self.query);
        if down && self.highlight + 1 < found.len() {
            self.highlight += 1;
        }
        if up {
            self.highlight = self.highlight.saturating_sub(1);
        }
        self.highlight = self.highlight.min(found.len().saturating_sub(1));
        let mut chosen = None;
        if enter {
            chosen = found.get(self.highlight).map(|e| e.action);
        }
        let screen = ctx.content_rect();
        let width = 460.0_f32.min(screen.width() - 32.0);
        egui::Area::new(egui::Id::new("command search"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(
                screen.center().x - width / 2.0,
                screen.top() + 72.0,
            ))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(width);
                    let edit = egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Type a command, such as extrude or export")
                        .desired_width(f32::INFINITY);
                    let response = ui.add(edit).labelled_by(
                        ui.label(egui::RichText::new(SEARCH_LABEL).small().weak())
                            .id,
                    );
                    if self.focus_pending {
                        response.request_focus();
                        self.focus_pending = false;
                    }
                    if response.changed() {
                        self.highlight = 0;
                    }
                    ui.add_space(4.0);
                    if found.is_empty() {
                        ui.label(
                            egui::RichText::new("Nothing found. Try another word.")
                                .color(crate::theme::TEXT_WEAK),
                        );
                    }
                    for (i, entry) in found.iter().enumerate() {
                        let text = match &entry.keys {
                            Some(keys) => format!("{}    {keys}", entry.name),
                            None => entry.name.clone(),
                        };
                        let hint = match &entry.not_now {
                            Some(why) => format!(
                                "{} › {}\n{}\nNot now: {why}",
                                entry.place, entry.name, entry.help
                            ),
                            None => format!("{} › {}\n{}", entry.place, entry.name, entry.help),
                        };
                        let mut label = egui::RichText::new(text);
                        if entry.not_now.is_some() {
                            label = label.color(crate::theme::TEXT_WEAK);
                        }
                        let row = ui
                            .add(egui::Button::selectable(i == self.highlight, label))
                            .on_hover_text(hint);
                        if row.clicked() {
                            chosen = Some(entry.action);
                        }
                        ui.label(
                            egui::RichText::new(format!("{} · {}", entry.place, entry.help))
                                .small()
                                .color(crate::theme::TEXT_WEAK),
                        );
                    }
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("↑ ↓ choose · Enter run · Esc close")
                            .small()
                            .color(crate::theme::TEXT_WEAK),
                    );
                });
            });
        if chosen.is_some() {
            self.close();
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, help: &str) -> SearchEntry {
        SearchEntry {
            name: name.into(),
            place: "Mesh".into(),
            help: help.into(),
            keys: None,
            not_now: None,
            action: SearchAction::Menu(MenuAction::Save),
        }
    }

    #[test]
    fn names_starting_with_the_query_come_first() {
        let entries = vec![
            entry("Inset", "Shrink the polygons inward, making a border"),
            entry("Extrude", "Pull the selected polygons out"),
            entry("Export…", "Save the model for a game engine"),
        ];
        let found: Vec<&str> = matches(&entries, "ex")
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(found, ["Extrude", "Export…"]);
        // A word in the help finds it too, after the names.
        let found: Vec<&str> = matches(&entries, "border")
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(found, ["Inset"]);
        // Case and spaces don't matter; every word must be there.
        assert_eq!(matches(&entries, "  EXTRUDE ").len(), 1);
        assert_eq!(matches(&entries, "pull polygons").len(), 1);
        assert!(matches(&entries, "pull nothing").is_empty());
    }

    #[test]
    fn what_cant_be_used_now_comes_after_what_can() {
        let mut busy = entry("Bevel", "Cut the corner off edges");
        busy.not_now = Some("Select edges first".into());
        let entries = vec![busy, entry("Bevel copy", "Cut the corner off")];
        let found = matches(&entries, "bevel");
        assert_eq!(found[0].name, "Bevel copy");
    }

    #[test]
    fn an_empty_query_lists_the_first_ones_and_never_too_many() {
        let entries: Vec<SearchEntry> = (0..40).map(|i| entry(&format!("C{i}"), "x")).collect();
        assert_eq!(matches(&entries, "").len(), MAX_SHOWN);
        assert_eq!(matches(&entries, "")[0].name, "C0");
    }

    #[test]
    fn every_menu_action_reads_well() {
        for action in MenuAction::ALL {
            let (name, place, help) = action.info();
            assert!(!name.is_empty() && !place.is_empty() && !help.is_empty());
        }
    }

    #[test]
    fn odd_queries_never_panic() {
        let entries = vec![entry("Extrude", "Pull")];
        for q in ["", " ", "→", "ÆØÅ", "\u{0}", &"x".repeat(10_000), "( [ *"] {
            let _ = matches(&entries, q);
        }
    }
}
