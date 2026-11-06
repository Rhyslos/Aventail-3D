//! Levels: moves a height's darks and lights, to make it crisper, softer
//! or turned the other way up.

use super::{next, number, switch};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Levels box.
#[derive(Debug)]
pub(crate) struct Levels {
    info: BoxInfo,
}

impl Levels {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed("levels"),
                name: Label::Key(text!("Levels")),
                group: Group::Shape,
                about: Label::Key(text!(
                    "Changes how dark and light a height is. Greys below Darkest become black and \
                     above Lightest white, so moving them together makes edges crisper; Middle \
                     moves the greys between. Turn upside down swaps high and low."
                )),
                inputs: vec![PlugSpec::new("in", text!("In"), ValueKind::Height, true)],
                outputs: vec![PlugSpec::new("out", text!("Out"), ValueKind::Height, false)],
                settings: vec![
                    number(
                        "low",
                        text!("Darkest"),
                        text!("Greys this dark or darker become black."),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    number(
                        "high",
                        text!("Lightest"),
                        text!("Greys this light or lighter become white."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    number(
                        "middle",
                        text!("Middle"),
                        text!("Above 1 lightens the greys between; below 1 darkens them."),
                        0.1,
                        10.0,
                        1.0,
                    ),
                    switch(
                        "invert",
                        text!("Turn upside down"),
                        text!("Swaps high and low: black becomes white."),
                    ),
                ],
                comes_next: next(&["blend", "material_from_height"]),
                one_per_graph: false,
            },
        }
    }
}

impl BoxType for Levels {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_grey("in")?;
        let (low, high) = (job.number("low"), job.number("high"));
        let power = 1.0 / job.number("middle").max(0.1);
        let invert = job.switch("invert");
        let out = job.map_grey(input, |v| {
            let t = if high - low > 1e-4 {
                ((v - low) / (high - low)).clamp(0.0, 1.0)
            } else if v >= low {
                1.0
            } else {
                0.0
            };
            let t = t.powf(power);
            if invert { 1.0 - t } else { t }
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
