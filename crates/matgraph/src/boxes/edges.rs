//! Edge finder: white lines along a shape's outline, a real-world width
//! wide: grout lines, seams, worn edges.

use super::pattern::distance_inside;
use super::{choice, info, length};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Edge finder box.
#[derive(Debug)]
pub(crate) struct Edges {
    info: BoxInfo,
}

/// Which side of the outline the line is drawn on, in the Side setting's
/// order.
const BOTH: u32 = 0;
const INSIDE: u32 = 1;
const OUTSIDE: u32 = 2;

impl Edges {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "edges",
                text!("Edge finder"),
                Group::Shape,
                text!(
                    "Finds the outline of a shape and draws it as a soft white line: for worn \
                     edges, chipped corners, grout and seams. Width is how wide the line is, in \
                     centimetres on the finished material. Side says whether the line sits \
                     inside the shape, outside it, or across the outline."
                ),
                (
                    vec![PlugSpec::new(
                        "shape",
                        text!("Shape"),
                        ValueKind::Shape,
                        true,
                    )],
                    vec![PlugSpec::new(
                        "edges",
                        text!("Edges"),
                        ValueKind::Height,
                        false,
                    )],
                ),
                vec![
                    length(
                        "width",
                        text!("Width"),
                        text!("How wide the line is, in centimetres."),
                        0.01,
                        20.0,
                        0.5,
                    ),
                    choice(
                        "side",
                        text!("Side"),
                        text!("Where the line sits against the outline."),
                        &[text!("Both"), text!("Inside"), text!("Outside")],
                        INSIDE,
                    ),
                ],
                &["blend", "levels", "mix_materials"],
            ),
        }
    }
}

impl BoxType for Edges {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let shape = job.needed_grey("shape")?;
        let width = job.length_px("width").max(0.5);
        let side = job.choice("side");
        let threads = job.threads();
        let inside = distance_inside(shape, width + 1.0, threads);
        let outside = distance_inside(&shape.map(|v| 1.0 - v), width + 1.0, threads);
        let out = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let (din, dout) = (inside.at(x, y), outside.at(x, y));
            let line = |d: f32| (1.0 - d / width).clamp(0.0, 1.0);
            // A pixel is on one side: the other distance is 0 there.
            let is_inside = din > 0.0;
            match side {
                INSIDE if is_inside => line(din),
                OUTSIDE if !is_inside => line(dout),
                BOTH => line(din.max(dout)),
                _ => 0.0,
            }
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
