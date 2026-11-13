//! The preview's player (Phase 12, milestone 15, [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)):
//! plays a graph's animated boxes (Time, Scroll, Pulse, Flow) in the
//! app's own preview, with Play / Pause, Back to the start and a time
//! slider, and chooses the moment exports and the painter bake.
//!
//! The player's time is the preview's own: it is not saved and not an
//! undo step. While it plays, the graph is worked out at the small size
//! on the pictures' thread at the player's time (`pictures`); boxes that
//! don't move come from the cache, so only the moving part is worked out
//! again. Each frame sends the newest moment, and older ones still
//! waiting are skipped, so a slow graph plays at fewer frames rather
//! than holding up the interface. **Bake this moment** sets the graph's
//! moment (`GraphEdit::SetMoment`, one undo step), which every export,
//! the painter and the export check use.
//!
//! No keys or mouse actions: buttons and a slider only (Space already
//! opens the box search over the canvas).

use super::MaterialEditor;
use crate::text::tr;
use egui::{Context, Ui};
use matgraph::{Graph, GraphEdit};
use scene::Document;
use std::time::Duration;

/// How long the time slider runs, in seconds; playing starts again from
/// 0 after it.
pub(crate) const LENGTH: f32 = 60.0;
/// How often the preview is drawn again while it plays: about 30 frames
/// a second.
const FRAME: Duration = Duration::from_millis(33);
/// The longest step the time takes in one frame, in seconds, so a frame
/// held up (a file dialog, a slow computer) doesn't jump the animation.
const LONGEST_STEP: f64 = 0.25;

/// The player's accessible names.
pub(crate) const PLAY: &str = crate::text!("Play");
pub(crate) const PAUSE: &str = crate::text!("Pause");
pub(crate) const TO_START: &str = crate::text!("Back to the start");
pub(crate) const TIME: &str = crate::text!("Time");
pub(crate) const BAKE_MOMENT: &str = crate::text!("Bake this moment");

/// The player's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Player {
    /// Whether it is playing.
    pub(crate) playing: bool,
    /// Seconds into the animation shown, from 0 to [`LENGTH`].
    pub(crate) time: f32,
    /// The interface's clock at the last frame played, in seconds.
    last: Option<f64>,
}

impl Player {
    /// A player paused at `moment` (a graph's, when it is shown).
    pub(crate) fn at(moment: f32) -> Self {
        Self {
            playing: false,
            time: moment.clamp(0.0, LENGTH),
            last: None,
        }
    }

    /// Moves the time on by the interface clock `now` (seconds) while
    /// playing, wrapping at [`LENGTH`]. Whether it moved.
    pub(crate) fn step(&mut self, now: f64) -> bool {
        if !self.playing {
            self.last = None;
            return false;
        }
        let gone = self
            .last
            .map_or(0.0, |last| (now - last).clamp(0.0, LONGEST_STEP));
        self.last = Some(now);
        if gone <= 0.0 {
            return false;
        }
        self.time = (self.time + gone as f32) % LENGTH;
        true
    }

    /// Plays from where it is, or pauses there.
    pub(crate) fn toggle(&mut self) {
        self.playing = !self.playing;
        self.last = None;
    }

    /// Goes to `seconds` (kept from 0 to [`LENGTH`]).
    pub(crate) fn seek(&mut self, seconds: f32) {
        self.time = if seconds.is_finite() {
            seconds.clamp(0.0, LENGTH)
        } else {
            0.0
        };
        self.last = None;
    }
}

impl MaterialEditor {
    /// Every frame: whether the graph shown moves over time (then the
    /// player's time is worked out, and it plays on while playing).
    pub(crate) fn tick_player(&mut self, ctx: &Context, document: &Document) -> Option<f32> {
        // A graph newly shown: paused at its moment.
        if self.player_for != self.graph {
            let moment = self.project_graph(document).map_or(0.0, Graph::moment);
            self.player = Player::at(moment);
            self.player_for = self.graph;
        }
        let animated = self.animated(document);
        if !animated {
            self.player.playing = false;
            return None;
        }
        let now = ctx.input(|i| i.time);
        self.player.step(now);
        if self.player.playing {
            ctx.request_repaint_after(FRAME);
        }
        Some(self.player.time)
    }

    /// Whether the graph shown, or the project's graph it is inside, has
    /// a box that moves over time.
    pub(crate) fn animated(&self, document: &Document) -> bool {
        let graphs = document.scene().graphs();
        let registry = graphs.registry();
        let top = self.graph.and_then(|id| graphs.get(id));
        self.current(document)
            .into_iter()
            .chain(top)
            .any(|g| g.is_animated(registry))
    }

    /// The player under the preview's picture, for a graph that moves:
    /// Play / Pause, Back to the start, the time, and the moment exports
    /// bake with Bake this moment.
    pub(crate) fn player_rows(&mut self, ui: &mut Ui, document: &mut Document) {
        if !self.animated(document) {
            return;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let (words, hint) = if self.player.playing {
                (tr(PAUSE), tr("Stop the animation where it is"))
            } else {
                (tr(PLAY), tr("Play the moving boxes in the preview"))
            };
            if ui.button(words).on_hover_text(hint).clicked() {
                self.player.toggle();
            }
            if ui
                .button(tr(TO_START))
                .on_hover_text(tr("Go back to 0 seconds"))
                .clicked()
            {
                self.player.seek(0.0);
            }
            let mut time = self.player.time;
            let slider = ui
                .add(
                    egui::Slider::new(&mut time, 0.0..=LENGTH)
                        .suffix(" s")
                        .fixed_decimals(1),
                )
                .on_hover_text(tr(
                    "Drag to see any moment of the animation; it plays on from there",
                ));
            slider.widget_info(|| {
                egui::WidgetInfo::slider(true, f64::from(time), tr(TIME))
            });
            if slider.changed() {
                self.player.seek(time);
            }
        });
        let Some(graph) = self.project_graph(document) else {
            return;
        };
        let moment = graph.moment();
        let shown = matgraph::baking::seconds(moment);
        if (moment - self.player.time).abs() < 0.05 {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "Exports and the painter bake this moment ({} s): textures can't move, so \
                     the game shows it standing still.",
                    shown
                ),
            );
            return;
        }
        crate::dialog::help(
            ui,
            crate::trf!(
                "Exports and the painter bake the moment at {} s: textures can't move, so the \
                 game shows it standing still.",
                shown
            ),
        );
        let bake = ui.button(tr(BAKE_MOMENT)).on_hover_text(tr(
            "Exports, the painter and paint layers using this graph show the moment the player is at now",
        ));
        if bake.clicked() {
            let seconds = (self.player.time * 10.0).round() / 10.0;
            let edit = GraphEdit::SetMoment(seconds.min(matgraph::MAX_MOMENT));
            let label = edit.describe();
            let _ = self.run_top(document, edit, &label);
        }
    }

    /// The project's graph shown (outside any own box opened).
    pub(crate) fn project_graph<'d>(&self, document: &'d Document) -> Option<&'d Graph> {
        document
            .scene()
            .graphs()
            .get(self.graph?)
            .map(std::convert::AsRef::as_ref)
    }

    /// Makes `edit` to the project's graph shown (even from inside an own
    /// box), as one undo step.
    fn run_top(
        &mut self,
        document: &mut Document,
        edit: GraphEdit,
        label: &str,
    ) -> Result<(), matgraph::Refusal> {
        let Some(id) = self.graph else {
            return Ok(());
        };
        let key = self.next_key();
        let command = scene::Command::EditGraph { id, edit };
        document
            .execute_all_labelled([command], label, key)
            .map_err(|e| match e {
                scene::SceneError::Graph(why) => why,
                other => matgraph::Refusal::new(other.to_string()),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_player_moves_on_only_while_playing() {
        let mut p = Player::at(1.0);
        assert!(!p.step(10.0));
        assert_eq!(p.time, 1.0);
        p.toggle();
        // The first frame only starts the clock.
        assert!(!p.step(10.0));
        assert!(p.step(10.5));
        assert!((p.time - 1.5).abs() < 1e-6);
        // A held-up frame takes a short step, not a jump.
        assert!(p.step(30.0));
        assert!((p.time - 1.75).abs() < 1e-6);
        // The clock going backwards (a test, a new context) doesn't.
        assert!(!p.step(5.0));
        p.toggle();
        assert!(!p.step(40.0));
        assert!((p.time - 1.75).abs() < 1e-6);
    }

    #[test]
    fn playing_wraps_round_and_seeking_stays_in_range() {
        let mut p = Player::at(LENGTH - 0.1);
        p.toggle();
        p.step(0.0);
        p.step(0.2);
        assert!((p.time - 0.1).abs() < 1e-4, "{}", p.time);
        p.seek(-4.0);
        assert_eq!(p.time, 0.0);
        p.seek(f32::NAN);
        assert_eq!(p.time, 0.0);
        p.seek(1.0e6);
        assert_eq!(p.time, LENGTH);
        assert_eq!(Player::at(4000.0).time, LENGTH);
    }
}
