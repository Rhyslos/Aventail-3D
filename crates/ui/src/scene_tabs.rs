//! Scene tabs ([decision 0266](../../../docs/decisions/0266-scene-tabs.md)):
//! several projects open in one window, one tab each.
//!
//! Each tab is a whole [`Workbench`]: its project and undo history, its
//! selection, view, mode and panel layouts, and its own background work
//! (the scene drawing's caches, bakes and graph fills, its backups), so
//! switching tabs leaves everything in the tab behind exactly as it was.
//! What the person sets up for the app as a whole is shared: settings,
//! the libraries (brushes, own boxes, recipes, colours) and the recent
//! projects are handed from the tab in use to the next when the tab in
//! use changes ([`Workbench::take_shared_from`]). The app keeps what
//! belongs to each tab outside the interface (its GPU renderer, backup
//! session and emergency copy) by the tab's [`TabId`].
//!
//! Only the tab in use is shown. The others are kept whole, so a tab can
//! later be put to sleep (its costly caches let go and made again when it
//! is used, step 3 of `docs/plans/scene-tabs.md`) without changing this.

use crate::text::tr;
use crate::viewport::ViewportRenderer;
use crate::workbench::{Workbench, WorkbenchAction};
use scene::Document;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod quitting;
mod sleep;
#[cfg(test)]
mod sleep_tests;
pub(crate) mod strip;
#[cfg(test)]
mod tests;

pub use sleep::SleepReason;
pub(crate) use strip::TabStrip;
use strip::{TabEvent, TabLabel};

/// A tab's lasting number: it never changes while the tab is open, and is
/// never given to another tab in the same run.
pub type TabId = u64;

/// The height of the tabs' own row, when Settings puts them there.
pub(crate) const OWN_ROW_HEIGHT: f32 = 32.0;

/// One open project.
struct Tab {
    id: TabId,
    workbench: Workbench,
    /// When it was last left for another tab (`None` while in use, or
    /// not yet looked at since).
    left_at: Option<Instant>,
    /// While it sleeps (decision 0267): why, and what was freed.
    asleep: Option<sleep::Asleep>,
}

impl Tab {
    fn new(id: TabId, workbench: Workbench) -> Self {
        Self {
            id,
            workbench,
            left_at: None,
            asleep: None,
        }
    }
}

/// Where quitting has got to, while it asks about unsaved changes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Quitting {
    /// The window listing every tab with unsaved changes is open.
    Listing,
    /// Asking about each tab in turn: `current` is being asked about, the
    /// rest come after.
    Reviewing { current: TabId, rest: Vec<TabId> },
}

/// Every open project, one tab each, and which is in use.
pub struct SceneTabs {
    /// Never empty: closing the last tab leaves a new, empty project.
    tabs: Vec<Tab>,
    /// The tab in use (its place in `tabs`).
    active: usize,
    next_id: TabId,
    /// Tabs closed since the app last asked ([`Self::take_closed`]).
    closed: Vec<TabId>,
    /// Actions from tabs that were left (their backup copies), handed to
    /// the app with the next frame's.
    pending: Vec<(TabId, WorkbenchAction)>,
    quitting: Option<Quitting>,
}

impl SceneTabs {
    /// The tabs, with `first` as the only one.
    pub fn new(mut first: Workbench) -> Self {
        first.become_tab(None);
        Self {
            tabs: vec![Tab::new(1, first)],
            active: 0,
            next_id: 2,
            closed: Vec::new(),
            pending: Vec::new(),
            quitting: None,
        }
    }

    /// How many tabs are open.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Never: there is always at least one tab.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// The tabs' numbers, in the row's order.
    pub fn ids(&self) -> Vec<TabId> {
        self.tabs.iter().map(|t| t.id).collect()
    }

    /// The tab in use.
    pub fn active_id(&self) -> TabId {
        self.tabs[self.active].id
    }

    /// The tab in use's workbench.
    pub fn active(&self) -> &Workbench {
        &self.tabs[self.active].workbench
    }

    /// The tab in use's workbench, to change.
    pub fn active_mut(&mut self) -> &mut Workbench {
        &mut self.tabs[self.active].workbench
    }

    /// The workbench of the tab `id`, if it is still open.
    pub fn get(&self, id: TabId) -> Option<&Workbench> {
        self.tabs.iter().find(|t| t.id == id).map(|t| &t.workbench)
    }

    /// The workbench of the tab `id`, if it is still open, to change.
    pub fn get_mut(&mut self, id: TabId) -> Option<&mut Workbench> {
        self.tabs
            .iter_mut()
            .find(|t| t.id == id)
            .map(|t| &mut t.workbench)
    }

    /// The workbench of the tab `id` while it is open, or else the tab in
    /// use's (for reporting work back to the tab that asked for it).
    pub fn get_or_active_mut(&mut self, id: Option<TabId>) -> &mut Workbench {
        let place = id.and_then(|id| self.place_of(id)).unwrap_or(self.active);
        &mut self.tabs[place].workbench
    }

    /// Every tab's workbench, in the row's order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (TabId, &mut Workbench)> {
        self.tabs.iter_mut().map(|t| (t.id, &mut t.workbench))
    }

    /// The tab that has the project at `path` open, if any.
    pub fn tab_with_project(&self, path: &Path) -> Option<TabId> {
        self.tabs
            .iter()
            .find(|t| t.workbench.project_path() == Some(path))
            .map(|t| t.id)
    }

    /// The window's title: the tab in use's.
    pub fn title(&self) -> String {
        self.active().title()
    }

    /// Whether any tab has unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs.iter().any(|t| t.workbench.has_unsaved_changes())
    }

    /// The names of the tabs with unsaved changes, in the row's order.
    pub fn unsaved_names(&self) -> Vec<String> {
        self.tabs
            .iter()
            .filter(|t| t.workbench.has_unsaved_changes())
            .map(|t| t.workbench.project_name())
            .collect()
    }

    /// Tabs closed since last asked: the app lets go of what it kept for
    /// them (renderer, backups, emergency copy).
    pub fn take_closed(&mut self) -> Vec<TabId> {
        std::mem::take(&mut self.closed)
    }

    /// Opens a new tab with a new, empty project (settings and layouts as
    /// in the tab in use) and makes it the tab in use. Returns its number.
    pub fn add_tab(&mut self) -> TabId {
        let mut workbench = self.active().new_tab_like();
        workbench.become_tab(self.free_untitled_name());
        self.insert_after_active(workbench)
    }

    /// `document`, opened from `path` (with what was left out, `notes`),
    /// in a new tab that becomes the tab in use; the tab in use instead
    /// while it is empty and untouched. Returns the tab's number.
    pub fn opened_in_new_tab(
        &mut self,
        document: Document,
        path: PathBuf,
        notes: &[String],
    ) -> TabId {
        if !self.active().is_untouched() {
            let workbench = self.active().new_tab_like();
            self.insert_after_active(workbench);
        }
        self.active_mut().opened_noting(document, path, notes);
        self.active_id()
    }

    /// Makes `id` the tab in use. Returns `false` if there is no such tab.
    pub fn switch_to(&mut self, id: TabId) -> bool {
        let Some(place) = self.place_of(id) else {
            return false;
        };
        if place == self.active {
            return true;
        }
        let left = self.active_id();
        if let Some(backup) = self.tabs[self.active].workbench.leave_tab() {
            self.pending.push((left, backup));
        }
        let (from, to) = (self.active, place);
        self.active = to;
        let (old, new) = two_mut(&mut self.tabs, from, to);
        new.workbench.take_shared_from(&old.workbench);
        old.left_at = Some(Instant::now());
        new.left_at = None;
        // A sleeping tab wakes: its next frame works out again what
        // sleeping let go of (decision 0267).
        if new.asleep.take().is_some() {
            new.workbench.wake();
        }
        true
    }

    /// Moves the tab `id` to place `to` in the row (the last place, if
    /// `to` is past the end). The tab in use stays in use.
    pub fn move_tab(&mut self, id: TabId, to: usize) {
        let Some(from) = self.place_of(id) else {
            return;
        };
        let active = self.active_id();
        let tab = self.tabs.remove(from);
        let to = to.min(self.tabs.len());
        self.tabs.insert(to, tab);
        self.active = self.place_of(active).unwrap_or(0);
    }

    /// Closes the tab `id`: at once if it has no unsaved changes, else
    /// after asking (in that tab, which becomes the tab in use). A tab
    /// whose save or export is under way waits for it.
    pub fn close_tab(&mut self, id: TabId) {
        let Some(place) = self.place_of(id) else {
            return;
        };
        if let Some(work) = self.tabs[place].workbench.file_work() {
            let note = crate::trf!("{} Close its tab once it's done.", work.doing());
            self.active_mut().set_note(note);
            return;
        }
        if self.tabs[place].workbench.ask_to_close_tab(false) {
            self.remove(id);
        } else {
            self.switch_to(id);
        }
    }

    /// The window's close button was pressed. Returns `true` if it may
    /// close now; with unsaved changes in any tab it asks first and
    /// returns `false` (a [`WorkbenchAction::Quit`] follows if the person
    /// agrees).
    pub fn request_close(&mut self) -> bool {
        if self.quitting.is_some() {
            return false;
        }
        self.start_quitting()
    }

    /// Draws the window around the tab in use, with the tabs, and returns
    /// what was asked for, each with the tab it is for. `renderer` draws
    /// the tab in use's views.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<(TabId, WorkbenchAction)> {
        let mut out = std::mem::take(&mut self.pending);
        let mut quit = false;
        // The list of tabs with changes, first, so its keys come first.
        match self.quitting_dialog(ui.ctx()) {
            Some(quitting::Answer::GoThrough) => {
                let unsaved: Vec<TabId> = self
                    .tabs
                    .iter()
                    .filter(|t| t.workbench.has_unsaved_changes())
                    .map(|t| t.id)
                    .collect();
                quit |= self.review(unsaved);
            }
            Some(quitting::Answer::CloseAll) => {
                self.quitting = None;
                quit = true;
            }
            Some(quitting::Answer::Cancel) => self.quitting = None,
            None => {}
        }
        let id = self.active_id();
        let strip = self.strip();
        let workbench = self.active_mut();
        workbench.set_tab_strip(strip);
        let actions = workbench.show(ui, renderer);
        let events = workbench
            .take_tab_strip()
            .map(|s| s.events)
            .unwrap_or_default();
        for action in actions {
            match action {
                WorkbenchAction::NewTab => {
                    self.add_tab();
                }
                WorkbenchAction::CloseTab => self.remove(id),
                WorkbenchAction::Quit => quit |= self.start_quitting(),
                other => out.push((id, other)),
            }
        }
        // While quitting asks about each tab, the tabs stay as they are.
        if self.quitting.is_none() {
            for event in events {
                self.carry_out(event);
            }
        }
        quit |= self.keep_reviewing();
        if quit {
            self.quitting = None;
            out.push((self.active_id(), WorkbenchAction::Quit));
        }
        out
    }

    // -------------------------------------------------------- inside ---

    /// The project's place in the row.
    fn place_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Puts `workbench` in a new tab just right of the tab in use, and
    /// makes it the tab in use.
    fn insert_after_active(&mut self, workbench: Workbench) -> TabId {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.insert(self.active + 1, Tab::new(id, workbench));
        self.tabs[self.active].left_at = Some(Instant::now());
        // Switching hands the backup of the tab left and the settings on.
        let new = self.active + 1;
        let left = self.active_id();
        if let Some(backup) = self.tabs[self.active].workbench.leave_tab() {
            self.pending.push((left, backup));
        }
        self.active = new;
        id
    }

    /// "Untitled" for the first untitled project, then "Untitled 2" and
    /// so on: the lowest number no open untitled tab has.
    fn free_untitled_name(&self) -> Option<String> {
        let taken: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| t.workbench.project_path().is_none())
            .map(|t| t.workbench.project_name())
            .collect();
        let plain = tr("Untitled").to_string();
        if !taken.contains(&plain) {
            return None;
        }
        (2..)
            .map(|n| crate::trf!("Untitled {}", n))
            .find(|name| !taken.contains(name))
    }

    /// What the strip draws this frame.
    fn strip(&self) -> TabStrip {
        TabStrip {
            tabs: self
                .tabs
                .iter()
                .map(|t| TabLabel {
                    id: t.id,
                    name: t.workbench.project_name(),
                    unsaved: t.workbench.has_unsaved_changes(),
                    path: t.workbench.project_path().map(Path::to_path_buf),
                    asleep: t.asleep,
                })
                .collect(),
            active: self.active,
            placement: self.active().view_options().tab_placement,
            events: Vec::new(),
        }
    }

    /// Carries out what was done with the tabs.
    fn carry_out(&mut self, event: TabEvent) {
        match event {
            TabEvent::Switch(id) => {
                self.switch_to(id);
            }
            TabEvent::Close(id) => self.close_tab(id),
            TabEvent::New => {
                self.add_tab();
            }
            TabEvent::Move { id, to } => self.move_tab(id, to),
        }
    }

    /// Takes the tab `id` out of the row. The tab beside it (to the
    /// right, or the left at the end) becomes the tab in use if it was;
    /// the last tab closed leaves a new, empty project.
    fn remove(&mut self, id: TabId) {
        let Some(place) = self.place_of(id) else {
            return;
        };
        if self.tabs.len() == 1 {
            let fresh = self.tabs[0].workbench.new_tab_like();
            self.tabs.push(Tab::new(self.next_id, fresh));
            self.next_id += 1;
        }
        let tab = self.tabs.remove(place);
        self.closed.push(id);
        if place < self.active {
            self.active -= 1;
        } else if place == self.active {
            self.active = place.min(self.tabs.len() - 1);
            // The closed tab was in use: its settings are the newest.
            let next = &mut self.tabs[self.active];
            next.workbench.take_shared_from(&tab.workbench);
            next.left_at = None;
            if next.asleep.take().is_some() {
                next.workbench.wake();
            }
        }
    }

    /// Starts quitting: returns `true` if the app may close at once (no
    /// tab has unsaved changes); otherwise asks, about the one tab with
    /// changes in that tab, or about several in a list first.
    fn start_quitting(&mut self) -> bool {
        let unsaved: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|t| t.workbench.has_unsaved_changes())
            .map(|t| t.id)
            .collect();
        match unsaved.len() {
            0 => true,
            1 => self.review(unsaved),
            _ => {
                self.quitting = Some(Quitting::Listing);
                false
            }
        }
    }

    /// Asks about each tab in `tabs` in turn, as closing it would.
    /// Returns `true` if none is left to ask about.
    fn review(&mut self, mut tabs: Vec<TabId>) -> bool {
        tabs.reverse();
        while let Some(id) = tabs.pop() {
            let Some(workbench) = self.get_mut(id) else {
                continue;
            };
            if workbench.ask_to_close_tab(true) {
                // Saved meanwhile: nothing to ask.
                continue;
            }
            self.switch_to(id);
            tabs.reverse();
            self.quitting = Some(Quitting::Reviewing {
                current: id,
                rest: tabs,
            });
            return false;
        }
        self.quitting = None;
        true
    }

    /// While asking about each tab: goes on to the next once the tab
    /// asked about has closed, and stops if its question was cancelled.
    /// Returns `true` once every tab has been dealt with.
    fn keep_reviewing(&mut self) -> bool {
        let Some(Quitting::Reviewing { current, rest }) = self.quitting.clone() else {
            return false;
        };
        match self.get(current) {
            // Closed (saved or not): on to the next.
            None => self.review(rest),
            Some(workbench) if workbench.closing_tab() => false,
            // Cancelled: the app stays open.
            Some(_) => {
                self.quitting = None;
                false
            }
        }
    }
}

/// Two different elements of `items`, both to change: the one at `a`
/// and the one at `b`.
fn two_mut<T>(items: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    if a < b {
        let (left, right) = items.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = items.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}
