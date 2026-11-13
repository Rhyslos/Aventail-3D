//! Comparing in the preview (decision 0291): the material shown beside
//! a picture **kept from before** (so a change's effect can be seen), or
//! beside **another graph's** material from the same project. Either as
//! one picture split by a line you drag (**Slider**), or as two smaller
//! pictures (**Side by side**); both turn together.
//!
//! The other graph is worked out on its own thread, as the canvas's
//! pictures are, but only when it changes: comparing costs nothing while
//! you work on the graph shown. A kept picture is shared, not copied.

use super::MaterialEditor;
use super::pictures::{PictureJob, Pictures};
use super::preview::{PreviewShape, selected_model, shown_box};
use crate::drawing::SceneSummary;
use crate::text::tr;
use matgraph::{GraphId, Value};
use scene::Document;

/// What the material is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompareWith {
    /// Nothing: the material alone (the standard).
    #[default]
    Nothing,
    /// The picture kept from before.
    Before,
    /// Another graph's material.
    Graph(GraphId),
}

/// How the two are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompareHow {
    /// One picture split by a line: before on the left, now on the right.
    #[default]
    Slider,
    /// Two smaller pictures next to each other.
    SideBySide,
}

/// One material to show, with how to lay it: the graph's now, or the
/// one compared with it.
#[derive(Debug, Clone)]
pub(crate) struct Source {
    /// The picture.
    pub(crate) value: Value,
    /// How many centimetres one repeat covers.
    pub(crate) real_size: f32,
    /// How many times more often the selected model lays it.
    pub(crate) tiling: f32,
    /// Pixel art: laid and drawn crisp (decision 0287).
    pub(crate) crisp: bool,
    /// A trim sheet: laid once (decision 0289).
    pub(crate) trim: bool,
    /// Its name on the picture.
    pub(crate) name: String,
}

/// The preview's comparing.
#[derive(Debug)]
pub(crate) struct Comparing {
    /// What it is compared with.
    pub(crate) with: CompareWith,
    /// How the two are shown.
    pub(crate) how: CompareHow,
    /// The picture kept from before.
    pub(crate) before: Option<Source>,
    /// Asked this frame: keep the picture shown now as before.
    pub(crate) keep_asked: bool,
    /// Where the slider's line is, from the left (0 to 1).
    pub(crate) split: f32,
    /// The other graph's pictures.
    pub(crate) pictures: Pictures,
}

impl Default for Comparing {
    fn default() -> Self {
        Self {
            with: CompareWith::Nothing,
            how: CompareHow::Slider,
            before: None,
            keep_asked: false,
            split: 0.5,
            pictures: Pictures::default(),
        }
    }
}

impl Comparing {
    /// Moves the slider's line to `share` of the way across, kept a
    /// little in from the edges so both sides stay in sight.
    pub(crate) fn set_split(&mut self, share: f32) {
        if share.is_finite() {
            self.split = share.clamp(0.02, 0.98);
        }
    }

    /// Whether the slider is in use.
    pub(crate) fn slider(&self) -> bool {
        self.with != CompareWith::Nothing && self.how == CompareHow::Slider
    }

    /// The tab is going to sleep (decision 0267): the other graph's
    /// pictures are let go of (the kept picture stays: it can't be made
    /// again). Returns about how many bytes they took.
    pub(crate) fn sleep(&mut self) -> u64 {
        self.pictures.sleep()
    }
}

impl MaterialEditor {
    /// Every frame, after the graph shown is sent: sends the graph
    /// compared with to be worked out when it changed, and forgets a
    /// comparison with a graph that is gone or now shown.
    pub(crate) fn update_compared(
        &mut self,
        ctx: &egui::Context,
        document: &Document,
        time: Option<f32>,
    ) {
        let store = document.scene().graphs();
        let job = match self.preview.comparing.with {
            CompareWith::Graph(id) => match store.get(id) {
                Some(graph) if !(Some(id) == self.graph && self.inside.is_empty()) => {
                    Some(PictureJob {
                        graph: graph.clone(),
                        registry: store.registry().clone(),
                        top: None,
                        time,
                    })
                }
                _ => {
                    self.preview.comparing.with = CompareWith::Nothing;
                    None
                }
            },
            _ => None,
        };
        // Small only while it plays: a button held for anything else
        // keeps the pictures there are.
        let playing = self.player.playing;
        self.preview.comparing.pictures.update(ctx, job, playing);
    }

    /// The material compared with, laid as `now` is laid, or why it
    /// can't be shown yet; `None` when nothing is compared.
    pub(crate) fn compared(
        &self,
        document: &Document,
        summary: &SceneSummary,
    ) -> Option<Result<Source, String>> {
        let comparing = &self.preview.comparing;
        match comparing.with {
            CompareWith::Nothing => None,
            CompareWith::Before => Some(
                comparing
                    .before
                    .clone()
                    .ok_or_else(|| tr("Nothing kept yet: choose Keep this as before.").to_string()),
            ),
            CompareWith::Graph(id) => {
                let graph = document.scene().graphs().get(id)?;
                let registry = self.registry();
                let Some(out) = shown_box(graph, None, &[]) else {
                    return Some(Err(crate::trf!("{} has no boxes yet.", graph.name())));
                };
                let Some(value) = comparing.pictures.value(out) else {
                    let why = comparing
                        .pictures
                        .shown()
                        .and_then(|d| d.evaluation.problems().find(|(id, _)| *id == out))
                        .map_or_else(
                            || tr("Working it out…").to_string(),
                            |(_, r)| r.reason.clone(),
                        );
                    return Some(Err(why));
                };
                let tiling = match self.preview.shape {
                    PreviewShape::Model => selected_model(summary).map_or(1.0, |item| {
                        super::giving::tiling_on(document.scene(), item.id, id)
                    }),
                    _ => 1.0,
                };
                Some(Ok(Source {
                    value: value.clone(),
                    real_size: matgraph::real_size_cm(registry, graph),
                    tiling,
                    crisp: matgraph::pixel_grid(registry, graph).is_some(),
                    trim: matgraph::is_trim_sheet(graph),
                    name: graph.name().to_string(),
                }))
            }
        }
    }

    /// Keeps `now` as the picture compared with (a click on Keep this as
    /// before, asked for this frame).
    pub(crate) fn keep_before(&mut self, now: &Source) {
        let comparing = &mut self.preview.comparing;
        if std::mem::take(&mut comparing.keep_asked) {
            comparing.before = Some(Source {
                name: tr(super::preview_options::COMPARE_BEFORE).to_string(),
                ..now.clone()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_split_stays_in_sight() {
        let mut comparing = Comparing::default();
        assert_eq!(comparing.split, 0.5);
        comparing.set_split(1.5);
        assert_eq!(comparing.split, 0.98);
        comparing.set_split(-1.0);
        assert_eq!(comparing.split, 0.02);
        comparing.set_split(f32::NAN);
        assert_eq!(comparing.split, 0.02);
        assert!(!comparing.slider(), "nothing compared");
        comparing.with = CompareWith::Before;
        assert!(comparing.slider());
        comparing.how = CompareHow::SideBySide;
        assert!(!comparing.slider());
    }
}
