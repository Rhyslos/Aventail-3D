//! Painted (Phase 12 milestone 20, [decision 0282](../../../../docs/decisions/0282-painted-box.md)):
//! a box you paint on with the painter's brushes, in the Material
//! editor's painting view, for things placed by hand: a crack just there,
//! moss in that corner.
//!
//! Its **Kind** says what it paints: a **Mask** (greys: white where the
//! paint is light), a **Height** (greys again, mid grey where nothing is
//! painted) or a **Colour**. Only the output of that kind gives anything;
//! wiring another says why and how to change it (plugs are fixed per kind
//! of box in the engine, [decision 0270](../../../../docs/decisions/0270-tiling-and-moving-colours.md)).
//! Its input of the same kind is what it paints over: the paint is laid
//! on it by **Blend** (Over, Add, Subtract, Multiply) and **Strength**;
//! with nothing plugged in, it paints over black (a mask), mid grey (a
//! height) or grey (a colour).
//!
//! The painting is kept with the box ([`crate::Painting`]) at its own
//! resolution and scaled to the size the graph is worked out at, wrapping
//! round its edges, so it tiles.

use super::{choice, info, number};
use crate::kind::ValueKind;
use crate::painting::lightness;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use std::sync::Arc;
use words::{text, tr, trf};

/// The box's id.
pub const PAINTED: &str = "painted";

/// What a Painted box paints (its Kind setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintedKind {
    /// Greys, black where nothing is painted: a shape.
    Mask,
    /// Greys, mid grey where nothing is painted.
    Height,
    /// Colours.
    Colour,
}

impl PaintedKind {
    /// Every kind, in the order of the setting's choices.
    pub const ALL: [Self; 3] = [Self::Mask, Self::Height, Self::Colour];

    /// The kind chosen by the box's settings.
    pub fn of(settings: &Settings) -> Self {
        match settings.get(KIND) {
            Some(SettingValue::Choice(1)) => Self::Height,
            Some(SettingValue::Choice(2)) => Self::Colour,
            _ => Self::Mask,
        }
    }

    /// Its index among the setting's choices.
    pub fn index(self) -> u32 {
        match self {
            Self::Mask => 0,
            Self::Height => 1,
            Self::Colour => 2,
        }
    }

    /// Its plain name.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Mask => text!("Mask"),
            Self::Height => text!("Height"),
            Self::Colour => text!("Colour"),
        })
    }

    /// Whether it paints greys (the brush's colour taken by its
    /// lightness).
    pub fn is_grey(self) -> bool {
        self != Self::Colour
    }

    /// The key of the input it paints over.
    pub fn input(self) -> &'static str {
        match self {
            Self::Mask => "mask",
            Self::Height => "height",
            Self::Colour => "colour",
        }
    }

    /// The key of the output it gives.
    pub fn output(self) -> &'static str {
        match self {
            Self::Mask => "mask_out",
            Self::Height => "height_out",
            Self::Colour => "colour_out",
        }
    }

    /// What shows where nothing is painted and nothing is plugged in.
    pub fn standard(self) -> [f32; 3] {
        match self {
            Self::Mask => [0.0; 3],
            Self::Height | Self::Colour => [0.5; 3],
        }
    }
}

/// The Kind setting's key.
pub const KIND: &str = "kind";

/// The Blend setting's choices.
const OVER: u32 = 0;
const ADD: u32 = 1;
const SUBTRACT: u32 = 2;
const MULTIPLY: u32 = 3;

/// The Painted box.
#[derive(Debug)]
pub(crate) struct Painted {
    info: BoxInfo,
}

impl Painted {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                PAINTED,
                text!("Painted"),
                Group::Detail,
                text!(
                    "Paint on it with the painter's brushes: right-click it and choose Paint on \
                     it (or double-click it). Use it for things placed by hand, such as a crack \
                     just there or moss in one corner. Kind picks what it paints: a mask, a \
                     height or a colour; only that output gives anything. Plug something into \
                     the input of that kind to paint over it."
                ),
                (
                    vec![
                        PlugSpec::new("mask", text!("Mask"), ValueKind::Shape, false),
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                    ],
                    vec![
                        PlugSpec::new("mask_out", text!("Mask"), ValueKind::Shape, false),
                        PlugSpec::new("height_out", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("colour_out", text!("Colour"), ValueKind::Colour, false),
                    ],
                ),
                vec![
                    choice(
                        KIND,
                        text!("Kind"),
                        text!(
                            "What it paints: a Mask (greys, black where nothing is painted), a \
                             Height (greys, mid grey where nothing is painted) or a Colour. The \
                             brush's colour is taken by its lightness for a mask or a height."
                        ),
                        &[text!("Mask"), text!("Height"), text!("Colour")],
                        0,
                    ),
                    choice(
                        "blend",
                        text!("Blend"),
                        text!(
                            "How the paint is laid on what is plugged in: Over covers it, Add \
                             lightens it, Subtract darkens it, Multiply darkens it by the paint."
                        ),
                        &[
                            text!("Over"),
                            text!("Add"),
                            text!("Subtract"),
                            text!("Multiply"),
                        ],
                        OVER,
                    ),
                    number(
                        "strength",
                        text!("Strength"),
                        text!("How much of the paint shows: 0 none, 1 all of it."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                ],
                &[
                    "fill_gaps",
                    "mix_materials",
                    "blend",
                    "material_from_height",
                ],
            ),
        }
    }
}

impl BoxType for Painted {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn takes_painting(&self) -> bool {
        true
    }

    fn why_empty(&self, settings: &Settings, plug: &str) -> Option<String> {
        let kind = PaintedKind::of(settings);
        let wanted = PaintedKind::ALL.into_iter().find(|k| k.output() == plug)?;
        (wanted != kind).then(|| {
            trf!(
                "This Painted box paints a {}, so its {} output gives nothing. Change its Kind \
                 to {} in Attributes, or connect its {} output instead.",
                kind.name().to_lowercase(),
                wanted.name(),
                wanted.name(),
                kind.name()
            )
        })
    }

    /// Every output, each as if Kind chose it (the evaluator asks for
    /// [`Self::run_outputs`], which gives only the chosen one).
    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(PaintedKind::ALL.map(|k| made(job, k)).to_vec())
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let kind = match job.choice(KIND) {
            1 => PaintedKind::Height,
            2 => PaintedKind::Colour,
            _ => PaintedKind::Mask,
        };
        Ok(PaintedKind::ALL
            .into_iter()
            .map(|k| (k == kind).then(|| made(job, k)))
            .collect())
    }
}

/// What a Painted box of kind `kind` gives for `job`.
fn made(job: &Job<'_>, kind: PaintedKind) -> Value {
    let blend = job.choice("blend");
    let strength = job.number("strength").clamp(0.0, 1.0);
    let n = job.size().get();
    let paint: Vec<[f32; 4]> = match job.painting() {
        Some(p) => p.sampled(n),
        None => vec![[0.0; 4]; (n * n) as usize],
    };
    let laid = move |base: f32, value: f32, amount: f32| -> f32 {
        let out = match blend {
            ADD => base + value * amount,
            SUBTRACT => base - value * amount,
            MULTIPLY => base * (1.0 - amount + value * amount),
            _ => base + (value - base) * amount,
        };
        out.clamp(0.0, 1.0)
    };
    // The paint at a pixel: its straight colour and how much of it shows
    // there.
    let at = |x: u32, y: u32| -> ([f32; 3], f32) {
        let [r, g, b, a] = paint[(y * n + x) as usize];
        if a <= 1e-6 {
            return ([0.0; 3], 0.0);
        }
        ([r / a, g / a, b / a], a * strength)
    };
    let standard = kind.standard();
    match kind {
        PaintedKind::Mask | PaintedKind::Height => {
            let under = job.grey(kind.input()).cloned();
            let grey = Arc::new(job.fill_grey(|x, y| {
                let base = under
                    .as_ref()
                    .map_or(standard[0], |u| u.at(i64::from(x), i64::from(y)));
                let (colour, amount) = at(x, y);
                if amount <= 0.0 {
                    return base;
                }
                laid(base, lightness(colour), amount)
            }));
            if kind == PaintedKind::Mask {
                Value::Shape(grey)
            } else {
                Value::Height(grey)
            }
        }
        PaintedKind::Colour => {
            let under = job.input(kind.input()).and_then(Value::colour).cloned();
            Value::Colour(Arc::new(job.fill_rgb(|x, y| {
                let base = under
                    .as_ref()
                    .map_or(standard, |u| u.at(i64::from(x), i64::from(y)));
                let (colour, amount) = at(x, y);
                if amount <= 0.0 {
                    return base;
                }
                [0, 1, 2].map(|c| laid(base[c], colour[c], amount))
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
    use crate::image::Size;
    use crate::painting::Painting;
    use crate::{Evaluator, Registry};
    use glam::Vec2;

    /// A painting with a white square from (8, 8) to (24, 24) on a
    /// 256-pixel canvas, half see-through at its edge column.
    fn square() -> Painting {
        let mut rgba = vec![0u8; 256 * 256 * 4];
        for y in 8..24 {
            for x in 8..24 {
                let at = (y * 256 + x) * 4;
                rgba[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        Painting::from_rgba(256, &rgba).unwrap()
    }

    fn graph_with(kind: u32, painting: Option<Painting>) -> (Graph, &'static Registry) {
        let registry = Registry::standard();
        let mut g = Graph::new("Painted");
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: PAINTED.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        g.apply(
            registry,
            &GraphEdit::SetSetting {
                node: BoxId(1),
                key: KIND.into(),
                value: SettingValue::Choice(kind),
            },
        )
        .unwrap();
        g.apply(
            registry,
            &GraphEdit::SetPainting {
                node: BoxId(1),
                painting: painting.map(Arc::new),
            },
        )
        .unwrap();
        (g, registry)
    }

    fn output(g: &Graph, registry: &Registry, size: u32, plug: &str) -> Option<Value> {
        let out =
            Evaluator::with_threads(2).run(registry, g, Size::new(size).unwrap(), &[BoxId(1)]);
        let result = out.outcome(BoxId(1)).unwrap().as_ref().ok()?.clone();
        let index = Painted::new()
            .info
            .outputs
            .iter()
            .position(|o| o.key == plug)?;
        result.outputs[index].clone()
    }

    #[test]
    fn a_mask_is_white_where_painted_and_black_elsewhere() {
        let (g, r) = graph_with(0, Some(square()));
        let Some(Value::Shape(mask)) = output(&g, r, 256, "mask_out") else {
            panic!("a mask");
        };
        assert_eq!(mask.at(10, 10), 1.0);
        assert_eq!(mask.at(100, 100), 0.0);
        // Scaled down to the graph's size: the same square, smaller.
        let Some(Value::Shape(small)) = output(&g, r, 64, "mask_out") else {
            panic!("a mask");
        };
        assert!(small.at(3, 3) > 0.9);
        assert_eq!(small.at(40, 40), 0.0);
        // Only the kind's output gives anything.
        assert!(output(&g, r, 64, "colour_out").is_none());
        assert!(output(&g, r, 64, "height_out").is_none());
    }

    #[test]
    fn a_height_rests_at_mid_grey_and_a_colour_at_grey() {
        let (g, r) = graph_with(1, Some(square()));
        let Some(Value::Height(h)) = output(&g, r, 256, "height_out") else {
            panic!("a height");
        };
        assert_eq!(h.at(100, 100), 0.5);
        assert_eq!(h.at(10, 10), 1.0);
        let (g, r) = graph_with(2, None);
        let Some(Value::Colour(c)) = output(&g, r, 64, "colour_out") else {
            panic!("a colour");
        };
        assert_eq!(c.at(5, 5), [0.5; 3]);
    }

    #[test]
    fn it_paints_over_what_is_plugged_in() {
        let (mut g, r) = graph_with(2, Some(square()));
        g.apply(
            r,
            &GraphEdit::AddBox {
                kind: "colour_fill".into(),
                at: Vec2::new(-300.0, 0.0),
            },
        )
        .unwrap();
        g.apply(
            r,
            &GraphEdit::SetSetting {
                node: BoxId(2),
                key: "colour".into(),
                value: SettingValue::Colour([1.0, 0.0, 0.0]),
            },
        )
        .unwrap();
        g.apply(
            r,
            &GraphEdit::Connect {
                from: PlugRef::new(BoxId(2), "colour"),
                to: PlugRef::new(BoxId(1), "colour"),
            },
        )
        .unwrap();
        let Some(Value::Colour(c)) = output(&g, r, 256, "colour_out") else {
            panic!("a colour");
        };
        assert_eq!(c.at(100, 100), [1.0, 0.0, 0.0]);
        assert_eq!(c.at(10, 10), [1.0, 1.0, 1.0]);
        // Multiply by white keeps what is under it; half strength over
        // goes half way.
        for (blend, strength, want) in [
            (MULTIPLY, 1.0, [1.0, 0.0, 0.0]),
            (OVER, 0.5, [1.0, 0.5, 0.5]),
        ] {
            let mut h = g.clone();
            for (key, value) in [
                ("blend", SettingValue::Choice(blend)),
                ("strength", SettingValue::Number(strength)),
            ] {
                h.apply(
                    r,
                    &GraphEdit::SetSetting {
                        node: BoxId(1),
                        key: key.into(),
                        value,
                    },
                )
                .unwrap();
            }
            let Some(Value::Colour(c)) = output(&h, r, 256, "colour_out") else {
                panic!("a colour");
            };
            let got = c.at(10, 10);
            for k in 0..3 {
                assert!((got[k] - want[k]).abs() < 0.01, "{blend}: {got:?}");
            }
        }
    }

    #[test]
    fn wiring_the_wrong_kind_says_why() {
        let (mut g, r) = graph_with(0, Some(square()));
        g.apply(
            r,
            &GraphEdit::AddBox {
                kind: "hue_saturation".into(),
                at: Vec2::new(300.0, 0.0),
            },
        )
        .unwrap();
        g.apply(
            r,
            &GraphEdit::Connect {
                from: PlugRef::new(BoxId(1), "colour_out"),
                to: PlugRef::new(BoxId(2), "in"),
            },
        )
        .unwrap();
        let out = Evaluator::with_threads(1).run(r, &g, Size::new(32).unwrap(), &[BoxId(2)]);
        let why = out.outcome(BoxId(2)).unwrap().clone().unwrap_err();
        assert!(why.reason.contains("paints a mask"), "{}", why.reason);
        assert!(why.reason.contains("Kind"), "{}", why.reason);
    }

    #[test]
    fn it_tiles_and_follows_the_painting() {
        // Paint across the right edge: it comes back on the left.
        let mut rgba = vec![0u8; 256 * 256 * 4];
        for y in 0..256 {
            for x in 250..256 {
                let at = (y * 256 + x) * 4;
                rgba[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let (g, r) = graph_with(0, Painting::from_rgba(256, &rgba));
        let Some(Value::Shape(big)) = output(&g, r, 1024, "mask_out") else {
            panic!("a mask");
        };
        assert!(big.at(0, 10) > 0.0, "the right edge blends into the left");
        // A stroke across the edge (as the painting view paints one) tiles.
        for y in 0..256 {
            for x in 0..6 {
                let at = (y * 256 + x) * 4;
                rgba[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let (g, r) = graph_with(0, Painting::from_rgba(256, &rgba));
        let Some(Value::Shape(big)) = output(&g, r, 1024, "mask_out") else {
            panic!("a mask");
        };
        assert!(big.at(0, 10) > 0.0, "the right edge blends into the left");
        let check = crate::SeamCheck::of(&Value::Shape(big));
        assert!(!check.shows(), "{}", check.describe());
        // Another painting, another result; the same painting again, the
        // same cached one.
        let mut ev = Evaluator::with_threads(1);
        let size = Size::new(64).unwrap();
        ev.run(r, &g, size, &[BoxId(1)]);
        let again = ev.run(r, &g, size, &[BoxId(1)]);
        assert_eq!(again.made(), 0);
        let (h, _) = graph_with(0, Some(square()));
        let other = ev.run(r, &h, size, &[BoxId(1)]);
        assert_eq!(other.made(), 1);
    }

    #[test]
    fn only_a_painted_box_takes_a_painting() {
        let r = Registry::standard();
        let mut g = Graph::new("Noise");
        g.apply(
            r,
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        let why = g
            .apply(
                r,
                &GraphEdit::SetPainting {
                    node: BoxId(1),
                    painting: Some(Arc::new(square())),
                },
            )
            .unwrap_err();
        assert!(why.reason.contains("Painted box"), "{}", why.reason);
        // Kept through parts; dropped from a box of another kind.
        let (g, r) = graph_with(1, Some(square()));
        let back = Graph::from_parts(g.to_parts(), r).unwrap();
        assert_eq!(back, g);
        let mut parts = g.to_parts();
        parts.boxes[0].1.kind = "noise".into();
        let back = Graph::from_parts(parts, r).unwrap();
        assert!(back.get(BoxId(1)).unwrap().painting.is_none());
        // Clearing leaves no painting; copies carry it.
        let mut cleared = g.clone();
        cleared
            .apply(
                r,
                &GraphEdit::SetPainting {
                    node: BoxId(1),
                    painting: None,
                },
            )
            .unwrap();
        assert!(cleared.get(BoxId(1)).unwrap().painting.is_none());
        let edits = g.get(BoxId(1)).unwrap().copy_edits(BoxId(9));
        assert!(
            edits
                .iter()
                .any(|e| matches!(e, GraphEdit::SetPainting { .. }))
        );
    }
}
