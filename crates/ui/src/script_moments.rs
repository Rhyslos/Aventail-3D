//! Scripts that run by themselves (plan, milestone 6, choice 13;
//! [decision 0309](../../../docs/decisions/0309-event-scripts-and-examples.md)):
//! the moments waiting for their scripts, and the scripts left to run for
//! the moment under way.
//!
//! Saving and exporting wait while their moment's scripts run, one after
//! another in name order, each for at most `script::MOMENT_LIMIT`: the
//! save or export is held back as a [`Job`] and released when they've
//! all ended. A script that fails, is stopped or runs out of time is said
//! in the output, and the save or export goes on; only `Export:cancel`
//! stops an export.

use crate::WorkbenchAction;
use crate::script_sharing::ScriptId;
use script::{Access, Choice, ExportInfo, Moment, ScriptFile};
use std::collections::VecDeque;

/// What a script stopping the export means for what waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnCancel {
    /// Nothing can be stopped (opening, saving).
    Nothing,
    /// The export waiting doesn't happen.
    DropAction,
    /// The save goes on without the export into the game project that
    /// follows it (decision 0076).
    SkipSaveExport,
}

/// A moment whose scripts run before what waits for it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Job {
    pub(crate) moment: Moment,
    /// What goes into the export starting, for `Export`'s calls.
    pub(crate) export: Option<ExportInfo>,
    /// What waits for the scripts: handed to the app when they've ended.
    pub(crate) then: Option<WorkbenchAction>,
    pub(crate) on_cancel: OnCancel,
}

/// One script to run at a moment, ready to start.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MomentScript {
    pub(crate) file: ScriptFile,
    /// Its name in words.
    pub(crate) name: String,
    pub(crate) id: ScriptId,
    /// Its commands' settings as last used.
    pub(crate) choices: Vec<Choice>,
    /// What it may reach: what it asks for and you allowed.
    pub(crate) access: Access,
}

/// The moment under way: its scripts left, and what the ended ones did.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Current {
    pub(crate) job: Job,
    pub(crate) scripts: VecDeque<MomentScript>,
    /// The names of the scripts that ran to their end.
    pub(crate) ran: Vec<String>,
    /// The names of those that didn't (a mistake, stopped, out of time).
    pub(crate) failed: Vec<String>,
    /// The script that stopped the export, and its reason.
    pub(crate) cancelled: Option<(String, String)>,
}

/// The moments waiting and under way in a tab.
#[derive(Debug, Default)]
pub(crate) struct Moments {
    pub(crate) jobs: VecDeque<Job>,
    pub(crate) current: Option<Current>,
    /// Whether the script running now is one of a moment's.
    pub(crate) running: bool,
    /// What waited, released once its scripts ended: handed to the app
    /// with the frame's actions (from a tab not in use too).
    pub(crate) released: Vec<WorkbenchAction>,
    /// Why the export into the game project after the next save won't
    /// happen: a script stopped it.
    pub(crate) skip_save_export: Option<String>,
    /// What the status bar adds once the save or export waiting is done:
    /// the scripts that ran first.
    pub(crate) said: Option<String>,
}

impl Moments {
    /// Whether anything is waiting for scripts or under way.
    pub(crate) fn busy(&self) -> bool {
        self.current.is_some() || !self.jobs.is_empty()
    }

    /// Forgets every moment waiting and under way (the project was
    /// replaced): nothing held back is handed on.
    pub(crate) fn forget(&mut self) {
        self.jobs.clear();
        self.current = None;
        self.skip_save_export = None;
        self.said = None;
    }
}

/// When a moment happens, in words for the status bar: "as the project
/// was saved".
pub(crate) fn moment_words(moment: Moment) -> &'static str {
    match moment {
        Moment::ProjectOpened => crate::text!("as the project opened"),
        Moment::ProjectSaved => crate::text!("as the project was saved"),
        Moment::ExportStarting => crate::text!("as the export started"),
    }
}

/// A moment as the script manager's switch names it: "a project opens".
pub(crate) fn switch_words(moment: Moment) -> &'static str {
    match moment {
        Moment::ProjectOpened => crate::text!("a project opens"),
        Moment::ProjectSaved => crate::text!("a project is saved"),
        Moment::ExportStarting => crate::text!("an export starts"),
    }
}
