//! Material out: the graph's result.

use super::number;
use crate::eval::MATERIAL_OUT;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::value::Value;
use std::borrow::Cow;
use words::{text, tr};

/// The Material out box.
#[derive(Debug)]
pub(crate) struct MaterialOut {
    info: BoxInfo,
}

impl MaterialOut {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed(MATERIAL_OUT),
                name: Label::Key(text!("Material out")),
                group: Group::Material,
                about: Label::Key(text!(
                    "The graph's result: the material plugged in here is what the graph makes. \
                     Real-world size says how many centimetres one repeat of the texture covers, \
                     so the material is laid on at the right size."
                )),
                inputs: vec![PlugSpec::new(
                    "material",
                    text!("Material"),
                    ValueKind::Material,
                    true,
                )],
                outputs: Vec::new(),
                settings: vec![number(
                    crate::eval::REAL_SIZE,
                    text!("Real-world size"),
                    text!("How many centimetres one repeat of the texture covers."),
                    1.0,
                    10_000.0,
                    100.0,
                )],
                comes_next: Vec::new(),
                one_per_graph: true,
                animated: false,
            },
        }
    }
}

impl BoxType for MaterialOut {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let material = job
            .input("material")
            .ok_or_else(|| Refusal::new(tr("Material out has nothing plugged in.")))?;
        Ok(vec![material.clone()])
    }
}
