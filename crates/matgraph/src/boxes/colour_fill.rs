//! Colour fill: one colour everywhere.

use super::{colour, next};
use crate::image::Image;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Colour fill box.
#[derive(Debug)]
pub(crate) struct ColourFill {
    info: BoxInfo,
}

impl ColourFill {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed("colour_fill"),
                name: Label::Key(text!("Colour fill")),
                group: Group::Colour,
                about: Label::Key(text!(
                    "One colour everywhere: plug it into Material from height's Colour to colour \
                     a whole material."
                )),
                inputs: Vec::new(),
                outputs: vec![PlugSpec::new(
                    "colour",
                    text!("Colour"),
                    ValueKind::Colour,
                    false,
                )],
                settings: vec![colour(
                    "colour",
                    text!("Colour"),
                    text!("The colour."),
                    [0.5, 0.5, 0.5],
                )],
                comes_next: next(&["material_from_height", "mix_materials"]),
                one_per_graph: false,
            },
        }
    }
}

impl BoxType for ColourFill {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let image = Image::filled(job.size(), job.colour("colour"));
        Ok(vec![Value::Colour(Arc::new(image))])
    }
}
