//! Sleeping tabs ([decision 0267](../../../../docs/decisions/0267-sleeping-tabs.md)):
//! a tab left for another goes to sleep after the time chosen in Settings
//! (two minutes as standard), or at once when the computer runs low on
//! memory. Asleep, its workbench lets go of what it worked out and the
//! app lets go of its renderer's meshes, textures and view images; its
//! project, undo history, selection, view, mode, layouts and unsaved
//! changes are all kept, and its backups carry on. Using it again wakes
//! it as it was left.
//!
//! Why two minutes rather than straight away: waking means turning every
//! shape into GPU geometry and uploading it again, which the measurements
//! in the decision show takes a noticeable moment for a big project
//! (about a tenth of a second for a scene of 200,000 polygons in light
//! shapes, and seconds for one shape of three million). Waiting a little
//! keeps switching back and forth instant, and still frees the memory of
//! tabs left for a while.

use super::{SceneTabs, TabId};
use crate::viewport::{MemoryFreed, TabSleep};
use std::time::{Duration, Instant};

/// How soon to look again at a tab that can't sleep yet (a save, bake or
/// export under way).
const BUSY_LOOK_AGAIN: Duration = Duration::from_secs(2);

/// Why a tab is asleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepReason {
    /// It was left for another tab, and has rested as long as the
    /// setting asks.
    Left(TabSleep),
    /// The computer was running low on memory.
    LowMemory,
}

/// A sleeping tab's note: why it sleeps, since when and what was freed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Asleep {
    pub(crate) reason: SleepReason,
    pub(crate) since: Instant,
    pub(crate) freed: MemoryFreed,
}

impl Asleep {
    /// What the moon's tooltip says: why it sleeps, how much was freed,
    /// and that nothing is lost.
    pub(crate) fn explanation(&self, unsaved: bool) -> String {
        let why = match self.reason {
            SleepReason::Left(TabSleep::StraightAway) => {
                crate::text::tr("You left it for another tab").to_string()
            }
            SleepReason::Left(setting) => crate::trf!(
                "You left it more than {} ago",
                minutes(setting.delay().unwrap_or_default())
            ),
            SleepReason::LowMemory => {
                crate::text::tr("The computer was running low on memory").to_string()
            }
        };
        let freed = crate::format_bytes(usize::try_from(self.freed.total()).unwrap_or(usize::MAX));
        let kept = if unsaved {
            crate::text::tr(
                "Nothing is lost: its unsaved changes are kept safe (and backed up as usual), \
                 with its undo history, selection, view and mode.",
            )
        } else {
            crate::text::tr(
                "Nothing is lost: the project, its undo history, selection, view and mode are \
                 all kept.",
            )
        };
        crate::trf!(
            "Asleep. {}, so its shapes and textures were put away to free about {} of memory.\n{}\nClick the tab to wake it as you left it.",
            why,
            freed,
            kept
        )
    }
}

/// "2 minutes", "30 minutes".
fn minutes(delay: Duration) -> String {
    crate::trf!("{} minutes", delay.as_secs() / 60)
}

impl SceneTabs {
    /// Puts to sleep the tabs that are due (decision 0267): left long
    /// enough ago for Settings' choice, or at once while `low_memory`;
    /// never the tab in use, one already asleep, one whose work can't stop
    /// (a save, bake or export) or any with the choice Never.
    /// `sleep_renderer` lets go of a tab's GPU side and says what it
    /// freed. Returns when to look again, if a tab will be due.
    pub fn sleep_tabs(
        &mut self,
        now: Instant,
        low_memory: bool,
        mut sleep_renderer: impl FnMut(TabId) -> MemoryFreed,
    ) -> Option<Duration> {
        let setting = self.active().view_options().tab_sleep;
        let delay = setting.delay()?;
        let reason = if low_memory {
            SleepReason::LowMemory
        } else {
            SleepReason::Left(setting)
        };
        let active = self.active;
        let mut next: Option<Duration> = None;
        let mut sooner = |wait: Duration| next = Some(next.map_or(wait, |n| n.min(wait)));
        for (place, tab) in self.tabs.iter_mut().enumerate() {
            if place == active || tab.asleep.is_some() {
                continue;
            }
            let left = *tab.left_at.get_or_insert(now);
            if tab.workbench.stays_awake().is_some() {
                sooner(BUSY_LOOK_AGAIN);
                continue;
            }
            let due = left + delay;
            if !low_memory && now < due {
                sooner(due - now);
                continue;
            }
            let freed = tab.workbench.sleep() + sleep_renderer(tab.id);
            tab.asleep = Some(Asleep {
                reason,
                since: now,
                freed,
            });
        }
        next
    }

    /// Whether the tab `id` is asleep.
    pub fn is_asleep(&self, id: TabId) -> bool {
        self.tabs
            .iter()
            .any(|t| t.id == id && t.asleep.is_some() && t.workbench.is_asleep())
    }

    /// What sleeping the tab `id` freed, while it sleeps.
    pub fn freed_by_sleep(&self, id: TabId) -> Option<MemoryFreed> {
        self.tabs
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.asleep)
            .map(|a| a.freed)
    }
}
