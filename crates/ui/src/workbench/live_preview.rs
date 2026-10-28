//! Live engine preview ([decision 0212](../../../../docs/decisions/0212-live-engine-preview.md)):
//! while it is on, the project is exported into its game project folder
//! a moment after the painting changes, so the engine (Godot reimports
//! changed files by itself) shows the new paint without saving or
//! exporting by hand.

use super::{Workbench, WorkbenchAction};
use std::time::{Duration, Instant};

/// How long the painting must rest before it goes out: long enough not
/// to export in the middle of quick strokes.
pub(crate) const LIVE_DELAY: Duration = Duration::from_millis(800);

/// The live preview's state.
#[derive(Debug, Default)]
pub(crate) struct LivePreview {
    /// The painting last sent out (each texture set's place in memory:
    /// unchanged sets keep theirs).
    sent: Option<Vec<usize>>,
    /// The painting last seen, and when it was first seen so.
    seen: Option<(Vec<usize>, Instant)>,
    /// How long the painting must rest first (shorter in tests).
    pub(crate) delay: Option<Duration>,
}

impl Workbench {
    /// Exports into the game project folder once the painting has
    /// changed and rested, while the live preview is on.
    pub(super) fn update_live_preview(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        let live = self
            .document
            .scene()
            .export_target()
            .is_some_and(|t| t.live);
        if !live {
            self.live = LivePreview {
                delay: self.live.delay,
                ..LivePreview::default()
            };
            return;
        }
        let now: Vec<usize> = self
            .document
            .scene()
            .paint()
            .sets()
            .map(|(_, set)| std::sync::Arc::as_ptr(set) as usize)
            .collect();
        if self.live.sent.as_ref() == Some(&now) {
            self.live.seen = None;
            return;
        }
        let at = Instant::now();
        let since = match &self.live.seen {
            Some((seen, since)) if *seen == now => *since,
            _ => {
                self.live.seen = Some((now.clone(), at));
                at
            }
        };
        // Not in the middle of a stroke, nor while files are being
        // written.
        let delay = self.live.delay.unwrap_or(LIVE_DELAY);
        let rested = at.duration_since(since);
        if self.brushing.stroking() || self.file_work().is_some() || rested < delay {
            ctx.request_repaint_after(delay.saturating_sub(rested).max(Duration::from_millis(50)));
            return;
        }
        if let Some(request) = self.target_request(None) {
            actions.push(WorkbenchAction::ExportToTarget(Box::new(request)));
            self.live.sent = Some(now);
            self.live.seen = None;
        }
    }
}
