//! What the boxes that move over time share (Time, Scroll, Pulse and
//! Flow, Phase 12's milestone 15, [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)):
//! marking a box as animated, waves from 0 to 1, and blending two
//! pictures of any kind.
//!
//! Every animated box reads the moment from [`Job::time`]: the preview's
//! player while it plays, else the graph's moment (what exports and the
//! painter bake). Nothing here reads a clock, so a moment always gives
//! the same pictures.

use crate::image::Image;
use crate::registry::{BoxInfo, Job};
use crate::settings::{SettingKind, SettingSpec};
use crate::value::{MaterialMaps, Value};
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// `info` marked as changing over time ([`BoxInfo::animated`]).
pub(super) fn animated(mut info: BoxInfo) -> BoxInfo {
    info.animated = true;
    info
}

/// A switch setting, on to start with.
pub(super) fn switch_on(key: &'static str, name: &'static str, about: &'static str) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: crate::registry::Label::Key(name),
        about: crate::registry::Label::Key(about),
        kind: SettingKind::Switch { standard: true },
    }
}

/// The Direction setting of Scroll and Flow: an angle in degrees, 0 to the
/// right and 90 up the texture.
pub(super) fn direction() -> SettingSpec {
    super::number(
        "angle",
        text!("Direction"),
        text!("Which way it moves, in degrees: 0 is to the right, 90 up, 180 left, -90 down."),
        -180.0,
        180.0,
        0.0,
    )
}

/// The Speed setting of Scroll and Flow, in centimetres a second (so it
/// keeps its real speed whatever the graph's real-world size).
pub(super) fn speed(standard: f32) -> SettingSpec {
    super::number(
        "speed",
        text!("Speed"),
        text!(
            "How fast it moves, in centimetres a second, at the graph's real-world size (Material \
             out's)."
        ),
        0.0,
        1000.0,
        standard,
    )
}

/// The way a Direction setting of `degrees` points on the texture: across
/// and down, a unit long (90 degrees is up the texture, so down is minus).
pub(super) fn heading(degrees: f32) -> (f32, f32) {
    let a = degrees.to_radians();
    (a.cos(), -a.sin())
}

/// How far through a loop of `period` seconds the moment `time` is, from
/// 0 up to (not including) 1. A period of 0 or less stands still at 0.
pub(super) fn through(time: f32, period: f32) -> f32 {
    if period <= 0.0 || !period.is_finite() || !time.is_finite() {
        return 0.0;
    }
    let p = (time / period).fract();
    if p < 0.0 { p + 1.0 } else { p }
}

/// The shapes a pulse can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Wave {
    /// Rises and falls smoothly (a cosine): breathing, glowing embers.
    Smooth,
    /// Jumps up at once and fades: a heartbeat, a flash.
    Sharp,
    /// Holds at even levels, stepping between them: a blinking light.
    Steps(u32),
}

impl Wave {
    /// How high the wave is a share `p` (0 to 1) through one pulse: from
    /// 0 to 1, starting and ending at the same height so it loops
    /// without a jump (Sharp and Steps jump on purpose).
    pub(super) fn at(self, p: f32) -> f32 {
        let p = p.clamp(0.0, 1.0);
        match self {
            Self::Smooth => 0.5 - 0.5 * (p * std::f32::consts::TAU).cos(),
            Self::Sharp => (1.0 - p).powi(3),
            Self::Steps(n) => {
                let n = n.max(2);
                let step = ((p * n as f32).floor() as u32).min(n - 1);
                step as f32 / (n - 1) as f32
            }
        }
    }
}

/// `a` blended toward `b` by `w` (0 all `a`, 1 all `b`), both the same
/// kind and size; a material's normals are made a unit long again.
pub(super) fn mixed(job: &Job<'_>, a: &Value, b: &Value, w: f32) -> Value {
    let w = w.clamp(0.0, 1.0);
    let grey = |x: &Image<f32>, y: &Image<f32>| {
        let n = i64::from(x.size());
        Arc::new(job.fill_grey(|px, py| {
            let (px, py) = (i64::from(px) % n, i64::from(py) % n);
            x.at(px, py) * (1.0 - w) + y.at(px, py) * w
        }))
    };
    let rgb = |x: &Image<[f32; 3]>, y: &Image<[f32; 3]>| {
        let n = i64::from(x.size());
        Arc::new(job.fill_rgb(|px, py| {
            let (px, py) = (i64::from(px) % n, i64::from(py) % n);
            let (p, q) = (x.at(px, py), y.at(px, py));
            [0, 1, 2].map(|i| p[i] * (1.0 - w) + q[i] * w)
        }))
    };
    match (a, b) {
        (Value::Shape(x), Value::Shape(y)) => Value::Shape(grey(x, y)),
        (Value::Height(x), Value::Height(y)) => Value::Height(grey(x, y)),
        (Value::Colour(x), Value::Colour(y)) => Value::Colour(rgb(x, y)),
        (Value::Material(x), Value::Material(y)) => {
            let normal = rgb(&x.normal, &y.normal);
            let normal = Arc::new(normal.map(|n| {
                glam::Vec3::from_array(n)
                    .normalize_or(glam::Vec3::Z)
                    .to_array()
            }));
            let either = |p: &Option<Arc<Image<f32>>>, q: &Option<Arc<Image<f32>>>| match (p, q) {
                (Some(p), Some(q)) => Some(grey(p, q)),
                _ => p.clone().or_else(|| q.clone()),
            };
            Value::Material(Arc::new(MaterialMaps {
                colour: rgb(&x.colour, &y.colour),
                height: grey(&x.height, &y.height),
                roughness: grey(&x.roughness, &y.roughness),
                metal: grey(&x.metal, &y.metal),
                normal,
                occlusion: grey(&x.occlusion, &y.occlusion),
                curvature: grey(&x.curvature, &y.curvature),
                opacity: either(&x.opacity, &y.opacity),
                translucency: either(&x.translucency, &y.translucency),
            }))
        }
        // Never asked for: two kinds can't be blended, so the first stays.
        _ => a.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn through_a_loop_stays_from_0_to_1() {
        assert_eq!(through(0.0, 4.0), 0.0);
        assert!((through(1.0, 4.0) - 0.25).abs() < 1e-6);
        assert!((through(5.0, 4.0) - 0.25).abs() < 1e-6);
        assert_eq!(through(3.0, 0.0), 0.0);
        assert_eq!(through(f32::NAN, 1.0), 0.0);
        for i in 0..2000 {
            let p = through(i as f32 * 0.137, 1.3);
            assert!((0.0..1.0).contains(&p), "{p}");
        }
    }

    #[test]
    fn waves_stay_from_0_to_1_and_smooth_loops() {
        for wave in [Wave::Smooth, Wave::Sharp, Wave::Steps(2), Wave::Steps(5)] {
            for i in 0..=100 {
                let v = wave.at(i as f32 / 100.0);
                assert!((0.0..=1.0).contains(&v), "{wave:?} {v}");
            }
        }
        assert!(Wave::Smooth.at(0.0) < 1e-6);
        assert!((Wave::Smooth.at(0.5) - 1.0).abs() < 1e-6);
        assert!((Wave::Smooth.at(1.0) - Wave::Smooth.at(0.0)).abs() < 1e-5);
        assert_eq!(Wave::Sharp.at(0.0), 1.0);
        assert_eq!(Wave::Steps(2).at(0.25), 0.0);
        assert_eq!(Wave::Steps(2).at(0.75), 1.0);
        assert_eq!(Wave::Steps(3).at(0.5), 0.5);
    }

    #[test]
    fn headings_point_the_way_named() {
        let close = |(a, b): (f32, f32), (c, d): (f32, f32)| {
            assert!((a - c).abs() < 1e-6 && (b - d).abs() < 1e-6, "{a},{b}");
        };
        close(heading(0.0), (1.0, 0.0));
        close(heading(90.0), (0.0, -1.0));
        close(heading(180.0), (-1.0, 0.0));
        close(heading(-90.0), (0.0, 1.0));
    }
}
