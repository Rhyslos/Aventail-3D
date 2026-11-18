//! Keeping, sharing and permissions (plan, milestone 5; [decision 0307](../../../docs/decisions/0307-keeping-sharing-and-permissions.md)):
//! the scripts kept with a project, which don't run until you turn them
//! on (trust); what a script asks to reach and your approval of it; and
//! a script being installed.
//!
//! **Trust** is kept by each script's fingerprint (its text's SHA-256,
//! `script::fingerprint`), on this computer only: turning a project's
//! scripts on trusts exactly that code, in any project, and a script
//! changed in any way is asked about again. Keeping one of your own
//! scripts with a project trusts it, so your own projects open with
//! their scripts on.
//!
//! **Approvals** name a script by [`ScriptId`]: a library script by its
//! file's name (following renames), a project's by its fingerprint, a
//! file run from elsewhere by its path. A script whose header asks for
//! more than was approved is asked about again before it runs.

use crate::script_commands::{CommandKey, LibraryScript, ScriptCommand, ScriptSettings};
use script::{Access, Ask, Described, Header, Limits, Names, ScriptFile};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

/// The most fingerprints of trusted scripts, and approvals, kept: the
/// oldest go first.
pub(crate) const MAX_TRUSTED: usize = 1_000;

/// A script, as trust and approvals name it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ScriptId {
    /// One in your library, by its file's name.
    Library(String),
    /// One kept with a project, by its fingerprint.
    Project(String),
    /// A file run with Run a script file…, by its full path.
    File(String),
}

/// What a script may ask for, as the settings file keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum SavedAsk {
    /// Read the project's folder.
    ReadProject,
    /// Read and write the project's folder.
    WriteProject,
    /// Read and write a folder the person chose.
    Chosen,
}

impl From<Ask> for SavedAsk {
    fn from(ask: Ask) -> Self {
        match ask {
            Ask::ReadProjectFolder => Self::ReadProject,
            Ask::WriteProjectFolder => Self::WriteProject,
            Ask::ChosenFolder => Self::Chosen,
        }
    }
}

/// What you allowed a script to reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Approval {
    pub(crate) script: ScriptId,
    pub(crate) asks: Vec<SavedAsk>,
    /// The folder you chose for it, when it asks for one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) folder: Option<PathBuf>,
}

impl ScriptSettings {
    /// What was approved for `id`, if anything.
    pub(crate) fn approval_of(&self, id: &ScriptId) -> Option<&Approval> {
        self.approvals.iter().find(|a| a.script == *id)
    }

    /// Whether `asks` need approving for `id` before it runs: one of them
    /// wasn't approved, or a chosen folder has none.
    pub(crate) fn needs_approval(&self, id: &ScriptId, asks: &[Ask]) -> bool {
        if asks.is_empty() {
            return false;
        }
        let Some(approval) = self.approval_of(id) else {
            return true;
        };
        asks.iter().any(|ask| {
            !approval.asks.contains(&SavedAsk::from(*ask))
                || (*ask == Ask::ChosenFolder && approval.folder.is_none())
        })
    }

    /// Approves `asks` for `id` (with the folder chosen, for a chosen
    /// folder), in place of what was approved before.
    pub(crate) fn approve(&mut self, id: &ScriptId, asks: &[Ask], folder: Option<PathBuf>) {
        self.approvals.retain(|a| a.script != *id);
        if self.approvals.len() >= MAX_TRUSTED {
            self.approvals.remove(0);
        }
        self.approvals.push(Approval {
            script: id.clone(),
            asks: asks.iter().map(|a| SavedAsk::from(*a)).collect(),
            folder: folder.filter(|_| asks.contains(&Ask::ChosenFolder)),
        });
    }

    /// Takes back what was approved for `id`.
    pub(crate) fn take_back(&mut self, id: &ScriptId) {
        self.approvals.retain(|a| a.script != *id);
    }

    /// What a run of `id`, whose header asks for `asks`, may reach: only
    /// what it asks for and was approved. A chosen folder wins over the
    /// project's (`project_folder`, when the project is saved).
    pub(crate) fn access_for(
        &self,
        id: Option<&ScriptId>,
        asks: &[Ask],
        project_folder: Option<&Path>,
    ) -> Access {
        let Some(approval) = id.and_then(|id| self.approval_of(id)) else {
            return Access::default();
        };
        let granted = |ask: Ask| asks.contains(&ask) && approval.asks.contains(&ask.into());
        if granted(Ask::ChosenFolder)
            && let Some(folder) = &approval.folder
        {
            return Access {
                folder: Some(folder.clone()),
                read: true,
                write: true,
                chosen: true,
            };
        }
        let write = granted(Ask::WriteProjectFolder);
        let read = write || granted(Ask::ReadProjectFolder);
        Access {
            folder: project_folder.filter(|_| read).map(Path::to_path_buf),
            read,
            write,
            chosen: false,
        }
    }

    /// Trusts the script with `fingerprint` on this computer.
    pub(crate) fn trust(&mut self, fingerprint: &str) {
        if self.is_trusted(fingerprint) {
            return;
        }
        if self.trusted.len() >= MAX_TRUSTED {
            self.trusted.remove(0);
        }
        self.trusted.push(fingerprint.to_string());
    }

    /// Whether the script with `fingerprint` is trusted.
    pub(crate) fn is_trusted(&self, fingerprint: &str) -> bool {
        self.trusted.iter().any(|t| t == fingerprint)
    }
}

/// One script kept with the project, with what reading it found once
/// it's trusted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectEntry {
    /// Its file's name.
    pub(crate) name: String,
    pub(crate) fingerprint: String,
    /// Its header, read without running anything.
    pub(crate) header: Header,
    /// Its commands, read once it's trusted (`None` until then).
    pub(crate) described: Option<Described>,
}

impl ProjectEntry {
    /// The script as a library script, for reusing what reads commands.
    fn as_library(&self) -> LibraryScript {
        LibraryScript::reading(PathBuf::new(), self.name.clone(), self.described.clone())
    }
}

/// The scripts kept with the project in a tab, and their commands once
/// trusted (read on a thread of their own, never before).
#[derive(Default)]
pub(crate) struct ProjectScripts {
    /// The list last seen in the scene.
    seen: Option<Arc<[scene::ProjectScript]>>,
    pub(crate) entries: Vec<ProjectEntry>,
    reading: Option<Receiver<(String, Described)>>,
    /// Whether the trust bar was put away with Not now.
    pub(crate) bar_hidden: bool,
}

impl ProjectScripts {
    /// Takes in the scene's scripts, when they changed, and starts
    /// reading the commands of those trusted and not yet read. Gives
    /// back whether anything changed.
    pub(crate) fn refresh(
        &mut self,
        kept: Arc<[scene::ProjectScript]>,
        settings: &ScriptSettings,
        names: &Names,
        limits: Limits,
    ) -> bool {
        let mut changed = self.take_in();
        if !self.seen.as_ref().is_some_and(|s| Arc::ptr_eq(s, &kept)) {
            let before = std::mem::take(&mut self.entries);
            self.entries = kept
                .iter()
                .map(|s| {
                    let fingerprint = script::fingerprint(&s.text);
                    let described = before
                        .iter()
                        .find(|e| e.fingerprint == fingerprint && e.name == s.name)
                        .and_then(|e| e.described.clone());
                    ProjectEntry {
                        name: s.name.clone(),
                        header: script::read_header(&s.text),
                        fingerprint,
                        described,
                    }
                })
                .collect();
            // Not now lasts until the scripts change (another project
            // opened, one kept or taken out).
            self.bar_hidden = false;
            self.seen = Some(kept.clone());
            changed = true;
        }
        if self.reading.is_none() {
            let to_read: Vec<ScriptFile> = kept
                .iter()
                .zip(&self.entries)
                .filter(|(_, e)| e.described.is_none() && settings.is_trusted(&e.fingerprint))
                .map(|(s, _)| ScriptFile {
                    name: s.name.clone(),
                    text: s.text.clone(),
                })
                .collect();
            if !to_read.is_empty() {
                self.read(to_read, names.clone(), limits);
            }
        }
        changed
    }

    fn read(&mut self, files: Vec<ScriptFile>, names: Names, limits: Limits) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let started = std::thread::Builder::new()
            .name(r"project script commands".into())
            .stack_size(16 << 20)
            .spawn(move || {
                for file in files {
                    let fingerprint = script::fingerprint(&file.text);
                    let described = script::commands_of(&file, &names, limits);
                    if sender.send((fingerprint, described)).is_err() {
                        return;
                    }
                }
            });
        if started.is_ok() {
            self.reading = Some(receiver);
        }
    }

    fn take_in(&mut self) -> bool {
        let Some(reading) = &self.reading else {
            return false;
        };
        let mut any = false;
        loop {
            match reading.try_recv() {
                Ok((fingerprint, described)) => {
                    any = true;
                    for entry in self
                        .entries
                        .iter_mut()
                        .filter(|e| e.fingerprint == fingerprint)
                    {
                        entry.described = Some(described.clone());
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.reading = None;
                    break;
                }
            }
        }
        any
    }

    /// Waits for the reading under way (for tests).
    #[cfg(test)]
    pub(crate) fn wait(&mut self) {
        while self.reading.is_some() {
            self.take_in();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// How many of the project's scripts aren't trusted.
    pub(crate) fn untrusted(&self, settings: &ScriptSettings) -> usize {
        self.entries
            .iter()
            .filter(|e| !settings.is_trusted(&e.fingerprint))
            .count()
    }

    /// The commands of the project's trusted scripts.
    pub(crate) fn commands(&self, settings: &ScriptSettings) -> Vec<ScriptCommand> {
        self.entries
            .iter()
            .filter(|e| settings.is_trusted(&e.fingerprint))
            .flat_map(|e| crate::script_commands::commands_of(&e.as_library()))
            .map(|mut c| {
                c.key.project = true;
                c
            })
            .collect()
    }

    /// The text of the project's script called `name`.
    pub(crate) fn text_of(&self, name: &str) -> Option<String> {
        self.seen
            .as_ref()?
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.text.clone())
    }

    /// The entry called `name`.
    pub(crate) fn entry(&self, name: &str) -> Option<&ProjectEntry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

/// The script a run comes from, for its permissions.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Source {
    /// One of your library's scripts, by its file's name.
    Library(String),
    /// A script kept with the project, by its file's name.
    Project(String),
    /// A file run from elsewhere.
    File(PathBuf),
    /// The console line: it reaches nothing.
    Console,
}

/// What runs once a script's asks are allowed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Then {
    /// Nothing: allowed from the script manager.
    Nothing,
    /// One of the scripts' commands.
    Command(CommandKey),
    /// A file, called `name`.
    Run {
        file: ScriptFile,
        name: String,
        source: Source,
    },
}

/// A script asking to reach something, in its window.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AccessAsk {
    pub(crate) script: ScriptId,
    /// The script's name in words.
    pub(crate) name: String,
    pub(crate) asks: Vec<Ask>,
    /// The folder chosen, for a script asking for one.
    pub(crate) folder: Option<PathBuf>,
    pub(crate) then: Then,
}

/// A script file being installed, shown first.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Installing {
    pub(crate) file: ScriptFile,
    pub(crate) header: Header,
    /// Why installing failed, if it did.
    pub(crate) why: Option<String>,
}

#[cfg(test)]
#[path = "script_sharing_tests.rs"]
mod tests;
