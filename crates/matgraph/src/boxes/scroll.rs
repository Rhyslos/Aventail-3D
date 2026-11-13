//! Scroll: slides a height, a colour or a whole material across the
//! texture over time, a conveyor belt or a waterfall (milestone 15,
//! [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)).
//!
//! What slides off one edge comes back on the other (the input wraps),
//! so the texture tiles at every moment. With Loops on, the slide comes
//! back to where it started every Loop length seconds without a jump:
//! the way it moves is nudged so a whole number of repeats goes by.

use super::motion::{self, through};
use super::moving::{self, Source};
use super::{info, number, uv};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::Value;
use words::text;

/// The Scroll box.
#[derive(Debug)]
pub(crate) struct Scroll {
    info: BoxInfo,
}

impl Scroll {
    pub(crate) fn new() -> Self {
        Self {
            info: motion::animated(info(
                "scroll",
                text!("Scroll"),
                Group::Move,
                text!(
                    "Slides a height, a colour or a whole material across the texture as time \
                     goes by: a conveyor belt, a waterfall, clouds drifting. Plug in any of \
                     them (or all three, slid together) and take each from its output. What \
                     slides off one edge comes back on the other, so it always tiles. With \
                     Loops on, it is back where it started every Loop length seconds without a \
                     jump (the direction and speed are nudged so a whole number of repeats goes \
                     by, at least one). It moves in the preview while the player plays; \
                     exports bake the moment chosen there."
                ),
                moving::plugs(),
                vec![
                    motion::direction(),
                    motion::speed(20.0),
                    motion::switch_on(
                        "loops",
                        text!("Loops"),
                        text!(
                            "Comes back to the start every Loop length seconds without a jump, \
                             so a game can play it round and round."
                        ),
                    ),
                    number(
                        "loop",
                        text!("Loop length"),
                        text!("How many seconds one loop lasts, with Loops on."),
                        0.5,
                        600.0,
                        4.0,
                    ),
                ],
                &["material_from_height", "blend", "mix_materials"],
            )),
        }
    }
}

/// How far a Scroll has slid at its job's moment: across and down, in
/// whole textures (only the part past whole textures matters).
pub(super) fn slid(job: &Job<'_>) -> (f32, f32) {
    let (hx, hy) = motion::heading(job.number("angle"));
    // Textures a second.
    let speed = job.number("speed") / job.real_size_cm();
    let (vx, vy) = (hx * speed, hy * speed);
    let time = job.time();
    if !job.switch("loops") {
        return ((vx * time).fract(), (vy * time).fract());
    }
    let length = job.number("loop").max(0.01);
    let mut whole = ((vx * length).round(), (vy * length).round());
    if whole == (0.0, 0.0) && speed > 0.0 {
        // Too slow for one repeat a loop: at least one, the main way.
        whole = if vx.abs() >= vy.abs() {
            (vx.signum(), 0.0)
        } else {
            (0.0, vy.signum())
        };
    }
    let p = through(time, length);
    (whole.0 * p, whole.1 * p)
}

impl BoxType for Scroll {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let (du, dv) = slid(job);
        let n = job.size().get() as f32;
        let at = |x, y| {
            let (u, v) = uv(x, y, n);
            Source {
                u: u - du,
                v: v - dv,
                scale: 1.0,
            }
        };
        // Sliding doesn't turn anything: normals stay as they are.
        moving::moved_all(job, &at, &|n, _| n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{input, size, with};
    use crate::kind::ValueKind;
    use crate::seams::SeamCheck;
    use crate::settings::SettingValue;

    fn scrolled(seconds: f32, settings: &[(&str, SettingValue)], kind: usize) -> Value {
        let scroll = Scroll::new();
        let n = size(64);
        let mut inputs = vec![None, None, None];
        let kinds = [ValueKind::Height, ValueKind::Colour, ValueKind::Material];
        inputs[kind] = Some(input(kinds[kind], n));
        let job = Job::new(scroll.info(), n, 0, 2, &with(settings), inputs)
            .with_real_size(100.0)
            .with_time(seconds);
        scroll.run_outputs(&job).unwrap()[kind].clone().unwrap()
    }

    #[test]
    fn scroll_tiles_at_every_moment() {
        for kind in 0..3 {
            for i in 0..12 {
                let t = i as f32 * 0.37;
                let value = scrolled(t, &[("angle", SettingValue::Number(33.0))], kind);
                let check = SeamCheck::of(&value);
                assert!(
                    !check.across.shows() && !check.down.shows(),
                    "kind {kind} at {t} s: {}",
                    check.describe()
                );
            }
        }
    }

    #[test]
    fn scroll_moves_by_its_speed_in_centimetres() {
        // 25 cm a second to the right on a 100 cm texture, not looping:
        // after one second, a quarter of the texture (16 of 64 pixels).
        let settings = [
            ("speed", SettingValue::Number(25.0)),
            ("loops", SettingValue::Switch(false)),
        ];
        let still = scrolled(0.0, &settings, 0);
        let moved = scrolled(1.0, &settings, 0);
        let (a, b) = (still.grey().unwrap(), moved.grey().unwrap());
        for (x, y) in [(0, 0), (10, 20), (63, 5)] {
            assert!((b.at(x + 16, y) - a.at(x, y)).abs() < 1e-4, "{x},{y}");
        }
    }

    #[test]
    fn a_looping_scroll_comes_back_without_a_jump() {
        let settings = [
            ("speed", SettingValue::Number(30.0)),
            ("loop", SettingValue::Number(4.0)),
        ];
        let start = scrolled(0.0, &settings, 1);
        let end = scrolled(4.0, &settings, 1);
        assert_eq!(start, end, "a whole loop later it is the same");
        // Just before the end it is a hair from the start: no jump.
        let near = scrolled(3.99, &settings, 1);
        let (a, b) = (start.colour().unwrap(), near.colour().unwrap());
        let most = a
            .pixels()
            .iter()
            .zip(b.pixels())
            .map(|(p, q)| (0..3).map(|i| (p[i] - q[i]).abs()).fold(0.0, f32::max))
            .fold(0.0, f32::max);
        assert!(most < 0.1, "{most}");
    }

    #[test]
    fn a_slow_loop_still_moves_one_repeat() {
        let scroll = Scroll::new();
        let settings = with(&[
            ("speed", SettingValue::Number(1.0)),
            ("angle", SettingValue::Number(90.0)),
        ]);
        let job = Job::new(scroll.info(), size(16), 0, 1, &settings, Vec::new())
            .with_real_size(100.0)
            .with_time(2.0);
        let (du, dv) = slid(&job);
        assert_eq!(du, 0.0);
        assert!((dv + 0.5).abs() < 1e-6, "half way up after half a loop: {dv}");
        // Standing still at speed 0.
        let still = with(&[("speed", SettingValue::Number(0.0))]);
        let job = Job::new(scroll.info(), size(16), 0, 1, &still, Vec::new()).with_time(2.0);
        assert_eq!(slid(&job), (0.0, 0.0));
    }
}
