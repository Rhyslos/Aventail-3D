//! Flow: drags a height, a colour or a whole material along over time,
//! each part its own way (a flow map, or downhill along a height), for
//! lava, rivers and slime (milestone 15, [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)).
//!
//! Dragging a picture along for ever would stretch it into streaks, so it
//! is dragged for Stretch time seconds and started again. Two copies take
//! turns half a stretch apart, and the picture fades from one to the
//! other: each copy starts again only while it is fully faded out, so the
//! resets never show (the "two-phase" flow of games' water shaders).

use super::motion::{self, through};
use super::moving::{self, Source};
use super::warp::{slope, steepest};
use super::{info, number, uv};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use words::text;

/// The Flow box.
#[derive(Debug)]
pub(crate) struct Flow {
    info: BoxInfo,
}

impl Flow {
    pub(crate) fn new() -> Self {
        let (mut inputs, outputs) = moving::plugs();
        inputs.push(PlugSpec::new(
            "flow",
            text!("Flow map"),
            ValueKind::Colour,
            false,
        ));
        inputs.push(PlugSpec::new(
            "downhill",
            text!("Downhill on"),
            ValueKind::Height,
            false,
        ));
        Self {
            info: motion::animated(info(
                "flow",
                text!("Flow"),
                Group::Move,
                text!(
                    "Drags a height, a colour or a whole material along as time goes by, each \
                     part its own way: lava creeping, a river, slime. Plug what should move \
                     into Height, Colour or Material. Without anything else it all flows in \
                     Direction; plug a height into Downhill on to make it run down that height's \
                     slopes, or a flow map (a colour: red is right, green is up) into Flow map \
                     to steer it yourself. To keep it from smearing into streaks, it starts \
                     again every Stretch time seconds, fading between two copies so the \
                     restart never shows. It moves in the preview while the player plays; \
                     exports bake the moment chosen there."
                ),
                (inputs, outputs),
                vec![
                    motion::direction(),
                    motion::speed(10.0),
                    number(
                        "stretch",
                        text!("Stretch time"),
                        text!(
                            "How many seconds it is dragged before fading into a fresh copy: \
                             longer stretches it further, shorter keeps it crisp."
                        ),
                        0.25,
                        30.0,
                        2.0,
                    ),
                ],
                &["material_from_height", "blend", "mix_materials"],
            )),
        }
    }
}

/// Which way each pixel flows: across and down, at most about a unit
/// long (a full-strength flow map, the steepest slope, or Direction).
enum Way<'a> {
    Map(&'a Rgb),
    Downhill(&'a Grey, f32),
    Even(f32, f32),
}

impl Way<'_> {
    fn at(&self, x: u32, y: u32) -> (f32, f32) {
        match self {
            Self::Map(map) => {
                let n = i64::from(map.size());
                let [r, g, _] = map.at(i64::from(x) % n, i64::from(y) % n);
                // Red right, green up the texture (as normal maps).
                (r * 2.0 - 1.0, -(g * 2.0 - 1.0))
            }
            Self::Downhill(height, steepest) => {
                let n = height.size();
                let (dx, dy) = slope(height, x % n, y % n);
                (-dx / steepest, -dy / steepest)
            }
            Self::Even(dx, dy) => (*dx, *dy),
        }
    }
}

/// Where the two copies are at a moment: each one's share through its
/// stretch, and how much of the second copy shows (0 to 1).
pub(super) fn phases(time: f32, stretch: f32) -> (f32, f32, f32) {
    let first = through(time, stretch);
    let second = through(time / stretch + 0.5, 1.0);
    (first, second, (1.0 - 2.0 * first).abs())
}

impl BoxType for Flow {
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
        let even = motion::heading(job.number("angle"));
        let downhill = job.grey("downhill");
        let steep = downhill.map_or(0.0, |h| steepest(h));
        let way = match (job.input("flow").and_then(Value::colour), downhill) {
            (Some(map), _) => Way::Map(map),
            (None, Some(h)) if steep > 0.0 => Way::Downhill(h, steep),
            _ => Way::Even(even.0, even.1),
        };
        let stretch = job.number("stretch").max(0.01);
        // How far a full-strength flow goes in one stretch, in textures.
        let reach = job.number("speed") / job.real_size_cm() * stretch;
        let (first, second, w) = phases(job.time(), stretch);
        let n = job.size().get() as f32;
        let copy = |share: f32| {
            let way = &way;
            move |x, y| {
                let (u, v) = uv(x, y, n);
                let (dx, dy) = way.at(x, y);
                Source {
                    u: u - dx * reach * share,
                    v: v - dy * reach * share,
                    scale: 1.0,
                }
            }
        };
        let a = moving::moved_all(job, &copy(first), &|n, _| n)?;
        let b = moving::moved_all(job, &copy(second), &|n, _| n)?;
        Ok(a.into_iter()
            .zip(b)
            .map(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => Some(motion::mixed(job, &a, &b, w)),
                _ => None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{input, size, with};
    use crate::seams::SeamCheck;
    use crate::settings::SettingValue;
    use std::sync::Arc;

    fn flowed(seconds: f32, extra: &[Option<Value>], kind: usize) -> Value {
        let flow = Flow::new();
        let n = size(64);
        let kinds = [ValueKind::Height, ValueKind::Colour, ValueKind::Material];
        let mut inputs = vec![None, None, None];
        inputs[kind] = Some(input(kinds[kind], n));
        inputs.extend(extra.iter().cloned());
        let settings = with(&[
            ("speed", SettingValue::Number(40.0)),
            ("angle", SettingValue::Number(20.0)),
        ]);
        let job = Job::new(flow.info(), n, 0, 2, &settings, inputs)
            .with_real_size(100.0)
            .with_time(seconds);
        flow.run_outputs(&job).unwrap()[kind].clone().unwrap()
    }

    /// The largest change between two pictures, channel by channel.
    fn change(a: &Value, b: &Value) -> f32 {
        match (a, b) {
            (Value::Height(a), Value::Height(b)) => a
                .pixels()
                .iter()
                .zip(b.pixels())
                .map(|(p, q)| (p - q).abs())
                .fold(0.0, f32::max),
            (Value::Colour(a), Value::Colour(b)) => a
                .pixels()
                .iter()
                .zip(b.pixels())
                .map(|(p, q)| (0..3).map(|i| (p[i] - q[i]).abs()).fold(0.0, f32::max))
                .fold(0.0, f32::max),
            (Value::Material(a), Value::Material(b)) => change(
                &Value::Colour(a.colour.clone()),
                &Value::Colour(b.colour.clone()),
            )
            .max(change(
                &Value::Height(a.height.clone()),
                &Value::Height(b.height.clone()),
            )),
            _ => f32::MAX,
        }
    }

    #[test]
    fn the_copies_take_turns_without_showing_a_reset() {
        for i in 0..200 {
            let t = i as f32 * 0.031;
            let (first, second, w) = phases(t, 2.0);
            // A copy starts again only while it doesn't show.
            if first < 0.02 {
                assert!(w > 0.95, "{t}: {w}");
            }
            if second < 0.02 {
                assert!(w < 0.05, "{t}: {w}");
            }
            assert!((0.0..=1.0).contains(&w));
        }
    }

    #[test]
    fn flow_changes_smoothly_through_its_restarts() {
        // Small steps across the moments each copy starts again (0, 1 and
        // 2 seconds with a 2 second stretch): never a jump.
        for kind in 0..3 {
            for at in [0.0f32, 1.0, 2.0, 0.6] {
                let before = flowed((at - 0.01).max(0.0), &[], kind);
                let after = flowed(at + 0.01, &[], kind);
                let most = change(&before, &after);
                assert!(most < 0.08, "kind {kind} at {at} s: {most}");
            }
        }
    }

    #[test]
    fn flow_tiles_at_every_moment_and_with_every_way() {
        let n = size(64);
        let map = Value::Colour(Arc::new(
            input(ValueKind::Height, n)
                .grey()
                .unwrap()
                .map(|v| [v, 1.0 - v, 0.5]),
        ));
        let ways: [&[Option<Value>]; 3] = [
            &[],
            &[Some(map), None],
            &[None, Some(input(ValueKind::Height, n))],
        ];
        for extra in ways {
            for kind in 0..3 {
                for t in [0.0, 0.7, 1.9, 5.3] {
                    let value = flowed(t, extra, kind);
                    let check = SeamCheck::of(&value);
                    assert!(
                        !check.across.shows() && !check.down.shows(),
                        "kind {kind} at {t}: {}",
                        check.describe()
                    );
                }
            }
        }
    }

    #[test]
    fn the_same_moment_gives_the_same_flow() {
        assert_eq!(flowed(1.234, &[], 2), flowed(1.234, &[], 2));
        assert_ne!(flowed(0.5, &[], 0), flowed(0.9, &[], 0));
    }
}
