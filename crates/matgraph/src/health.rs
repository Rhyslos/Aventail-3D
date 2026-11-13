//! The material health check (Phase 12, milestone 27,
//! [decision 0290](../../../docs/decisions/0290-material-health-check.md)):
//! what may look wrong in a game, said in plain words, each with a fix.
//!
//! A graph can work out without a single refusal and still make a
//! material that looks poor where it is used. The check looks at what
//! Material out was given and at the graph, and warns about:
//!
//! - **colours too dark or too bright to light well** (darker than
//!   charcoal, brighter than snow, metal darker than any real metal);
//! - **detail that turns to mush at a distance** (most of the light and
//!   dark is fine detail the engine averages away when the material is
//!   small on screen);
//! - **a repeat that shows** (one patch of a repeat stands out, so the eye
//!   finds it again every repeat);
//! - **joins that show** (the seam check, a trim sheet's top-bottom join
//!   left out: it is expected there);
//! - **pixel art that isn't** (Pixelate not last) and **pixel art at
//!   other sizes** (pixels a metre unlike the project's other pixel-art
//!   materials);
//! - **trim sheets** whose strips and the project's trim rows disagree,
//!   or whose cut strips cut through a row of their pattern (`trim`).
//!
//! Each [`Warning`] carries [`Remedy`]s: plain edits or a box to look at,
//! which the interface offers as buttons. Nothing changes until one is
//! picked.

mod looks;
mod measure;
mod trim;

use crate::baking::{join, names};
use crate::boxes::pixelate::{PIXELATE, pixel_grid};
use crate::eval::{Evaluation, MATERIAL_OUT, REAL_SIZE, real_size_cm};
use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
use crate::registry::Registry;
use crate::settings::SettingValue;
use crate::value::Value;
use glam::Vec2;
use std::collections::BTreeSet;
use words::{tr, trf};

/// How far apart two pixel-art materials' pixels a metre may be (a share)
/// before they look like different sizes side by side.
const SAME_PIXELS: f32 = 0.01;
/// What a warning is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Concern {
    /// Pixelate isn't the last box before Material out.
    PixelateNotLast,
    /// Pixel art with other pixels a metre than the project's other
    /// pixel-art materials.
    PixelSizes,
    /// A trim sheet on the project's trim rows, which has none.
    NoTrimRows,
    /// Strips with no trim row to lie on.
    PastTrimRows,
    /// A trim sheet laid by its own heights, unlike the project's rows.
    RowsDiffer,
    /// A cut strip cutting through a row of its pattern.
    CutThrough,
    /// Colour darker than charcoal.
    TooDark,
    /// Colour brighter than snow.
    TooBright,
    /// Metal darker than real metal.
    DarkMetal,
    /// Most light and dark is fine detail.
    Mush,
    /// A patch of the repeat stands out.
    Repeats,
    /// A join shows where the texture repeats.
    Joins,
}

/// Something that may look wrong in a game, in plain words, with ways to
/// fix it.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    /// What it is about.
    pub concern: Concern,
    /// What it found and why it matters, in plain words.
    pub words: String,
    /// Ways to fix it, best first.
    pub remedies: Vec<Remedy>,
    /// The boxes it is about (to mark or look at).
    pub boxes: Vec<BoxId>,
}

/// One way to fix a warning, offered as a button.
#[derive(Debug, Clone, PartialEq)]
pub struct Remedy {
    /// The button's words.
    pub words: String,
    /// What it does.
    pub action: Action,
}

/// What a remedy does.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Select and frame this box: its settings are where to change it.
    LookAt(BoxId),
    /// Set one setting of a box.
    SetSetting {
        /// The box.
        node: BoxId,
        /// The setting's key.
        key: String,
        /// Its new value.
        value: SettingValue,
    },
    /// Put a new box of `kind` on the wire from `from` into `to`: `from`
    /// into its `input`, its `output` into `to`, with `settings`, and
    /// optionally another new box feeding one of its inputs.
    InsertBox {
        /// The output the wire comes from.
        from: PlugRef,
        /// The input it goes into.
        to: PlugRef,
        /// The new box's kind.
        kind: String,
        /// Its input `from` goes into.
        input: String,
        /// Its output going into `to`.
        output: String,
        /// Its settings.
        settings: Vec<(String, SettingValue)>,
        /// A second new box feeding it.
        feed: Option<Feed>,
    },
    /// Make the project's trim rows match this Trim sheet box's strips.
    MakeTrimRows(BoxId),
}

impl Action {
    /// The graph edits doing it in `graph`, in order, as one change (new
    /// boxes take the graph's next ids, placed between the boxes they join;
    /// the interface tidies after). Empty for what isn't a graph edit:
    /// looking at a box, or the project's trim rows (a scene change).
    pub fn edits(&self, graph: &Graph) -> Vec<GraphEdit> {
        match self {
            Self::LookAt(_) | Self::MakeTrimRows(_) => Vec::new(),
            Self::SetSetting { node, key, value } => vec![GraphEdit::SetSetting {
                node: *node,
                key: key.clone(),
                value: value.clone(),
            }],
            Self::InsertBox {
                from,
                to,
                kind,
                input,
                output,
                settings,
                feed,
            } => {
                let place = |id: BoxId| graph.get(id).map_or(Vec2::ZERO, |b| b.at);
                let at = (place(from.node) + place(to.node)) * 0.5;
                let new = BoxId(graph.next_id());
                let mut edits = vec![
                    GraphEdit::AddBox {
                        kind: kind.clone(),
                        at,
                    },
                    GraphEdit::Connect {
                        from: from.clone(),
                        to: PlugRef::new(new, input.as_str()),
                    },
                    GraphEdit::Connect {
                        from: PlugRef::new(new, output.as_str()),
                        to: to.clone(),
                    },
                ];
                edits.extend(settings.iter().map(|(key, value)| GraphEdit::SetSetting {
                    node: new,
                    key: key.clone(),
                    value: value.clone(),
                }));
                if let Some(feed) = feed {
                    let fed = BoxId(graph.next_id() + 1);
                    edits.push(GraphEdit::AddBox {
                        kind: feed.kind.clone(),
                        at: at - Vec2::new(0.0, FEED_BELOW),
                    });
                    edits.extend(
                        feed.settings
                            .iter()
                            .map(|(key, value)| GraphEdit::SetSetting {
                                node: fed,
                                key: key.clone(),
                                value: value.clone(),
                            }),
                    );
                    edits.push(GraphEdit::Connect {
                        from: PlugRef::new(fed, feed.output.as_str()),
                        to: PlugRef::new(new, feed.into.as_str()),
                    });
                }
                edits
            }
        }
    }
}

/// How far below an inserted box the box feeding it is first put (the
/// tidy layout moves it).
const FEED_BELOW: f32 = 160.0;

/// A new box feeding a box put in by [`Action::InsertBox`].
#[derive(Debug, Clone, PartialEq)]
pub struct Feed {
    /// Its kind.
    pub kind: String,
    /// Its output.
    pub output: String,
    /// The inserted box's input it goes into.
    pub into: String,
    /// Its settings.
    pub settings: Vec<(String, SettingValue)>,
}

/// Another graph of the project that is pixel art: its name and grid.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelArt {
    /// The graph's name.
    pub name: String,
    /// Its pixel count across.
    pub pixels: u32,
    /// How many centimetres one repeat covers.
    pub real_size_cm: f32,
}

impl PixelArt {
    /// `graph` as pixel art, if it is (Pixelate last before Material out).
    pub fn of(registry: &Registry, graph: &Graph) -> Option<Self> {
        Some(Self {
            name: graph.name().to_string(),
            pixels: pixel_grid(registry, graph)?,
            real_size_cm: real_size_cm(registry, graph),
        })
    }

    /// Its pixels a metre.
    pub fn per_metre(&self) -> f32 {
        self.pixels as f32 * 100.0 / self.real_size_cm.max(1e-3)
    }
}

/// What the check reads: the graph, its kinds of box, its worked-out
/// pictures and the project's other pixel-art materials.
struct Looking<'a> {
    registry: &'a Registry,
    graph: &'a Graph,
    evaluation: &'a Evaluation,
    out: Option<BoxId>,
}

impl Looking<'_> {
    fn name(&self, id: BoxId) -> String {
        names(self.registry, self.graph, &[id])
    }

    /// The value an output gives, if its box ran.
    fn output(&self, plug: &PlugRef) -> Option<&Value> {
        let b = self.graph.get(plug.node)?;
        let info = self.registry.get(&b.kind)?.info();
        let at = info.outputs.iter().position(|p| p.key == plug.plug)?;
        let result = self.evaluation.outcome(plug.node)?.as_ref().ok()?;
        result.outputs.get(at)?.as_ref()
    }

    /// Material out's input wire's output.
    fn feeding_out(&self) -> Option<PlugRef> {
        let out = self.out?;
        Some(
            self.graph
                .wire_into(&PlugRef::new(out, "material"))?
                .from
                .clone(),
        )
    }

    /// Every box feeding Material out, nearest first, bypassed ones left
    /// out.
    fn upstream(&self) -> Vec<BoxId> {
        let Some(out) = self.out else {
            return Vec::new();
        };
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        let mut waiting = std::collections::VecDeque::from([out]);
        while let Some(id) = waiting.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            if id != out && self.graph.get(id).is_some_and(|b| !b.bypassed) {
                order.push(id);
            }
            waiting.extend(self.graph.sources_of(id));
        }
        order
    }

    /// The nearest wire into a box's Colour input upstream of Material
    /// out (Material from height's colour, say): where the colour is made.
    fn colour_wire(&self) -> Option<(PlugRef, PlugRef)> {
        self.upstream().into_iter().find_map(|id| {
            let to = PlugRef::new(id, "colour");
            let b = self.graph.get(id)?;
            let spec = self.registry.get(&b.kind)?.info().input("colour")?.clone();
            if spec.kind != crate::ValueKind::Colour {
                return None;
            }
            Some((self.graph.wire_into(&to)?.from.clone(), to))
        })
    }

    /// A setting's value on box `id`, within range.
    fn setting(&self, id: BoxId, key: &str) -> Option<SettingValue> {
        let b = self.graph.get(id)?;
        let spec = self.registry.get(&b.kind)?.info().setting(key)?.clone();
        Some(
            b.settings
                .get(key)
                .map_or_else(|| spec.standard(), |v| spec.nearest(v)),
        )
    }
}

/// The health check of `graph` as `evaluation` worked it out (every box
/// it holds, at any size), with the project's other pixel-art materials
/// `others`. Warnings come in reading order: the graph's set-up first,
/// then how the material looks.
pub fn check(
    registry: &Registry,
    graph: &Graph,
    evaluation: &Evaluation,
    others: &[PixelArt],
) -> Vec<Warning> {
    let looking = Looking {
        registry,
        graph,
        evaluation,
        out: graph.first_of(MATERIAL_OUT),
    };
    let mut out = Vec::new();
    pixel_art(&looking, others, &mut out);
    trim::rows(&looking, &mut out);
    trim::cut_through(&looking, &mut out);
    let material = looking
        .out
        .and_then(|id| evaluation.picture(id))
        .and_then(Value::material);
    if let Some(m) = material {
        looks::colours(&looking, m, &mut out);
        let look = measure::look(m);
        looks::mush(&looking, m, &look, &mut out);
        let trim = crate::is_trim_sheet(graph);
        if !trim && !m.is_cut_out() {
            looks::repeats(&looking, m, &look, &mut out);
        }
        looks::joins(&looking, m, trim, &mut out);
    }
    out
}

/// A length in centimetres or metres, for people: whole centimetres (a
/// tenth under 10), or metres past 1000 cm.
pub fn length_words(cm: f32) -> String {
    if cm >= 1000.0 {
        trf!("{} m", (cm / 10.0).round() / 10.0)
    } else if cm >= 10.0 {
        trf!("{} cm", cm.round())
    } else {
        trf!("{} cm", (cm * 10.0).round() / 10.0)
    }
}

fn pixel_art(looking: &Looking<'_>, others: &[PixelArt], out: &mut Vec<Warning>) {
    let (registry, graph) = (looking.registry, looking.graph);
    let Some(material_out) = looking.out else {
        return;
    };
    if let Some(art) = PixelArt::of(registry, graph) {
        let ppm = art.per_metre();
        let unlike: Vec<&PixelArt> = others
            .iter()
            .filter(|o| (o.per_metre() / ppm - 1.0).abs() > SAME_PIXELS)
            .collect();
        let Some(first) = unlike.first() else {
            return;
        };
        let target = first.per_metre();
        let alike: Vec<String> = unlike
            .iter()
            .filter(|o| (o.per_metre() / target - 1.0).abs() <= SAME_PIXELS)
            .map(|o| o.name.clone())
            .collect();
        let size = art.pixels as f32 * 100.0 / target;
        let mut remedies = Vec::new();
        if (1.0..=10_000.0).contains(&size) {
            remedies.push(Remedy {
                words: trf!(
                    "Make one repeat cover {} ({} pixels a metre)",
                    length_words(size),
                    target.round()
                ),
                action: Action::SetSetting {
                    node: material_out,
                    key: REAL_SIZE.to_string(),
                    value: SettingValue::Number(size),
                },
            });
        }
        out.push(Warning {
            concern: Concern::PixelSizes,
            words: trf!(
                "This pixel-art material has {} pixels a metre, but {} {} {}: side by side in \
                 the game their pixels are different sizes, which pixel art usually avoids.",
                ppm.round(),
                join(&alike),
                if alike.len() == 1 {
                    tr("has")
                } else {
                    tr("have")
                },
                target.round()
            ),
            remedies,
            boxes: vec![material_out],
        });
        return;
    }
    let late = looking.upstream().into_iter().find(|id| {
        graph
            .get(*id)
            .is_some_and(|b| b.kind == PIXELATE && !b.bypassed)
    });
    if let Some(pixelate) = late {
        let last = looking
            .feeding_out()
            .map_or_else(|| tr("another box").to_string(), |p| looking.name(p.node));
        out.push(Warning {
            concern: Concern::PixelateNotLast,
            words: trf!(
                "Pixelate is in the graph, but {} comes between it and Material out, so the \
                 material isn't treated as pixel art: it is exported at the texture size and \
                 shown smooth, and boxes after Pixelate can blur or shift its squares. Plug \
                 Pixelate straight into Material out, with everything else before it.",
                last
            ),
            remedies: vec![Remedy {
                words: trf!("Look at {}", looking.name(pixelate)),
                action: Action::LookAt(pixelate),
            }],
            boxes: vec![pixelate],
        });
    }
}

#[cfg(test)]
mod tests;
