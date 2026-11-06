//! Blend: mixes two heights, through a shape if one is given.

use super::{next, number};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::settings::{SettingKind, SettingSpec};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// How Blend mixes its two heights (its Mode setting, by index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// The top height in place of the bottom one.
    Mix,
    /// Both added.
    Add,
    /// The top taken away from the bottom.
    Subtract,
    /// Both multiplied: darkens.
    Multiply,
    /// The lighter of the two at each place.
    Lighter,
    /// The darker of the two at each place.
    Darker,
}

impl BlendMode {
    /// Every mode, in the Mode setting's order.
    pub const ALL: [Self; 6] = [
        Self::Mix,
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Lighter,
        Self::Darker,
    ];

    /// The mode's plain name, as an English key.
    fn key(self) -> &'static str {
        match self {
            Self::Mix => text!("Mix"),
            Self::Add => text!("Add"),
            Self::Subtract => text!("Subtract"),
            Self::Multiply => text!("Multiply"),
            Self::Lighter => text!("Lighter"),
            Self::Darker => text!("Darker"),
        }
    }

    /// `bottom` and `top` mixed this way, before Amount.
    pub fn apply(self, bottom: f32, top: f32) -> f32 {
        match self {
            Self::Mix => top,
            Self::Add => (bottom + top).min(1.0),
            Self::Subtract => (bottom - top).max(0.0),
            Self::Multiply => bottom * top,
            Self::Lighter => bottom.max(top),
            Self::Darker => bottom.min(top),
        }
    }
}

/// The Blend box.
#[derive(Debug)]
pub(crate) struct Blend {
    info: BoxInfo,
}

impl Blend {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed("blend"),
                name: Label::Key(text!("Blend")),
                group: Group::Shape,
                about: Label::Key(text!(
                    "Mixes two heights: Top over Bottom, the way Mode says. With a shape in Where, \
                     Top shows only where the shape is white. Amount sets how much of Top shows."
                )),
                inputs: vec![
                    PlugSpec::new("bottom", text!("Bottom"), ValueKind::Height, true),
                    PlugSpec::new("top", text!("Top"), ValueKind::Height, true),
                    PlugSpec::new("where", text!("Where"), ValueKind::Shape, false),
                ],
                outputs: vec![PlugSpec::new("out", text!("Out"), ValueKind::Height, false)],
                settings: vec![
                    SettingSpec {
                        key: Cow::Borrowed("mode"),
                        name: Label::Key(text!("Mode")),
                        about: Label::Key(text!("How Top and Bottom are mixed.")),
                        kind: SettingKind::Choice {
                            options: BlendMode::ALL.map(|m| Label::Key(m.key())).to_vec(),
                            standard: 0,
                        },
                    },
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How much of Top shows."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                ],
                comes_next: next(&["levels", "material_from_height"]),
                one_per_graph: false,
            },
        }
    }
}

impl BoxType for Blend {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let bottom = job.needed_grey("bottom")?;
        let top = job.needed_grey("top")?;
        let mask = job.grey("where");
        let mode = BlendMode::ALL
            .get(job.choice("mode") as usize)
            .copied()
            .unwrap_or(BlendMode::Mix);
        let amount = job.number("amount");
        let out = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let (b, t) = (bottom.at(x, y), top.at(x, y));
            let share = amount * mask.map_or(1.0, |m| m.at(x, y));
            b + (mode.apply(b, t) - b) * share
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
