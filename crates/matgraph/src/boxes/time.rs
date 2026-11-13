//! Time: how far through a loop the animation is, as flat greys that
//! other boxes can be driven by (milestone 15, [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)).

use super::motion::{self, Wave};
use super::{info, number};
use crate::image::Image;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Time box.
#[derive(Debug)]
pub(crate) struct Time {
    info: BoxInfo,
}

impl Time {
    pub(crate) fn new() -> Self {
        Self {
            info: motion::animated(info(
                "time",
                text!("Time"),
                Group::Move,
                text!(
                    "Gives the time as it plays, as a flat grey: Loop rises from black to white \
                     over Period seconds and starts again; Wave rises and falls smoothly over \
                     the same time. Plug either into Blend's mask or a Levels box to make \
                     anything fade, swap or change in time. In the Material mode it moves when \
                     the preview's player plays; exports bake the moment chosen there."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("loop", text!("Loop"), ValueKind::Height, false),
                        PlugSpec::new("wave", text!("Wave"), ValueKind::Height, false),
                    ],
                ),
                vec![number(
                    "period",
                    text!("Period"),
                    text!("How many seconds one loop lasts before it starts again."),
                    0.1,
                    600.0,
                    4.0,
                )],
                &["blend", "levels", "gradient_map"],
            )),
        }
    }
}

impl BoxType for Time {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let p = motion::through(job.time(), job.number("period"));
        let flat = |v: f32| Value::Height(Arc::new(Image::filled(job.size(), v.clamp(0.0, 1.0))));
        Ok(vec![flat(p), flat(Wave::Smooth.at(p))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{size, with};
    use crate::settings::{SettingValue, Settings};

    fn at(seconds: f32, period: f32) -> (f32, f32) {
        let time = Time::new();
        let settings = with(&[("period", SettingValue::Number(period))]);
        let job = Job::new(time.info(), size(16), 0, 1, &settings, Vec::new()).with_time(seconds);
        let out = time.run(&job).unwrap();
        let g = |v: &Value| v.grey().unwrap().at(3, 5);
        (g(&out[0]), g(&out[1]))
    }

    #[test]
    fn time_loops_over_its_period() {
        assert_eq!(at(0.0, 4.0), (0.0, 0.0));
        let (l, w) = at(2.0, 4.0);
        assert!((l - 0.5).abs() < 1e-6 && (w - 1.0).abs() < 1e-6);
        assert_eq!(at(1.0, 4.0), at(5.0, 4.0));
        assert_eq!(at(1.0, 4.0), at(9.0, 4.0));
    }

    #[test]
    fn the_same_moment_gives_the_same_picture() {
        for s in [0.0, 0.3, 17.25, 3599.9] {
            assert_eq!(at(s, 3.0), at(s, 3.0));
            let (l, w) = at(s, 3.0);
            assert!((0.0..=1.0).contains(&l) && (0.0..=1.0).contains(&w));
        }
        // A moment that isn't a real number is the start.
        let time = Time::new();
        let job =
            Job::new(time.info(), size(16), 0, 1, &Settings::new(), Vec::new()).with_time(f32::NAN);
        assert_eq!(job.time(), 0.0);
    }
}
