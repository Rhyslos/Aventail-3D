//! Blur: softens a height, over a real-world distance.

use super::pattern::soft_blur;
use super::{info, length};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Blur box.
#[derive(Debug)]
pub(crate) struct Blur {
    info: BoxInfo,
}

impl Blur {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "blur",
                text!("Blur"),
                Group::Shape,
                text!(
                    "Softens a height, smoothing sharp edges and small bumps away. Radius is how \
                     far the softening reaches, in centimetres on the finished material (the \
                     graph's real-world size, set on Material out, says how big the texture is). \
                     It wraps round the edges, so the texture still tiles."
                ),
                (
                    vec![PlugSpec::new("in", text!("In"), ValueKind::Height, true)],
                    vec![PlugSpec::new("out", text!("Out"), ValueKind::Height, false)],
                ),
                vec![length(
                    "radius",
                    text!("Radius"),
                    text!("How far the softening reaches, in centimetres."),
                    0.0,
                    50.0,
                    1.0,
                )],
                &["levels", "blend", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Blur {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_grey("in")?;
        let out = soft_blur(input, job.length_px("radius"), job.threads());
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
