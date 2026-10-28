//! How the brush feels ([decision 0184](../../../docs/decisions/0184-brush-feel.md)):
//! the pen's pressure and tilt, the curves that turn pressure into size
//! and flow, and smoothing (a "lazy mouse": the brush follows the pointer
//! on a string, so strokes come out steadier).
//!
//! Pressure comes from a tablet through Wintab (read by the app, which
//! hands it over with [`crate::Workbench::set_tablet_pen`]) or, for other
//! pens, from the system as touch with force (read here from egui's
//! events). A mouse has no pressure: the brush then paints at full size
//! and flow.

/// A pen's state while it is near the tablet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenState {
    /// How hard it presses, 0 to 1.
    pub pressure: f32,
    /// Which way and how far it leans, across the tablet (x right, y
    /// down; length 0 upright to 1 flat), if the tablet tells.
    pub tilt: Option<[f32; 2]>,
    /// Whether its eraser end is down (the brush then erases).
    pub eraser: bool,
}

/// The shape of a pressure curve: how fast size or flow grows as the pen
/// presses harder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Curve {
    /// Grows quickly at first: light touches already paint well.
    Soft,
    /// In step with the pressure.
    #[default]
    Even,
    /// Grows slowly at first: a firm press is needed.
    Firm,
}

impl Curve {
    /// Every curve, in the order Attributes lists them.
    pub(crate) const ALL: [Curve; 3] = [Self::Soft, Self::Even, Self::Firm];

    fn power(self) -> f32 {
        match self {
            Self::Soft => 0.6,
            Self::Even => 1.0,
            Self::Firm => 1.7,
        }
    }
}

/// How pressure changes a brush setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PressureCurve {
    /// Whether pressure changes it at all.
    pub(crate) on: bool,
    /// The share of the setting at the lightest touch, 0 to 1.
    pub(crate) least: f32,
    /// How it grows in between.
    pub(crate) curve: Curve,
}

impl PressureCurve {
    /// The share of the setting at `pressure` (none: a mouse, full).
    pub(crate) fn share(&self, pressure: Option<f32>) -> f32 {
        let Some(p) = pressure.filter(|_| self.on) else {
            return 1.0;
        };
        let p = if p.is_finite() {
            p.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let least = self.least.clamp(0.0, 1.0);
        least + (1.0 - least) * p.powf(self.curve.power())
    }
}

/// The longest string of the lazy mouse, at full smoothing, in points.
pub(crate) const SMOOTHING_REACH: f32 = 60.0;

/// Moves `brush` toward `pointer` on a string `reach` long: it stays
/// put while the pointer is within reach, and is pulled along behind it
/// beyond. Returns whether it moved.
pub(crate) fn follow(brush: &mut glam::Vec2, pointer: glam::Vec2, reach: f32) -> bool {
    let gap = pointer - *brush;
    let length = gap.length();
    if !length.is_finite() || length <= reach {
        return false;
    }
    *brush += gap * (1.0 - reach / length);
    true
}

/// The pen's pressure in this frame's touch events (a pen through the
/// system), if any: `Some(None)` when the pen lifted.
pub(crate) fn touch_pressure(ctx: &egui::Context) -> Option<Option<f32>> {
    ctx.input(|i| {
        let mut out = None;
        for event in &i.events {
            if let egui::Event::Touch { phase, force, .. } = event {
                out = match phase {
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => Some(None),
                    _ => force.map(|f| Some(f.clamp(0.0, 1.0))).or(out),
                };
            }
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn curves_turn_pressure_into_a_share() {
        let even = PressureCurve {
            on: true,
            least: 0.2,
            curve: Curve::Even,
        };
        assert_eq!(even.share(None), 1.0, "a mouse paints fully");
        assert!((even.share(Some(0.0)) - 0.2).abs() < 1e-6);
        assert!((even.share(Some(1.0)) - 1.0).abs() < 1e-6);
        assert!((even.share(Some(0.5)) - 0.6).abs() < 1e-6);
        let soft = PressureCurve {
            curve: Curve::Soft,
            ..even
        };
        let firm = PressureCurve {
            curve: Curve::Firm,
            ..even
        };
        assert!(soft.share(Some(0.3)) > even.share(Some(0.3)));
        assert!(firm.share(Some(0.3)) < even.share(Some(0.3)));
        let off = PressureCurve { on: false, ..even };
        assert_eq!(off.share(Some(0.1)), 1.0);
        assert_eq!(even.share(Some(f32::NAN)), 1.0);
        assert_eq!(even.share(Some(7.0)), 1.0);
    }

    #[test]
    fn the_lazy_brush_follows_on_a_string() {
        let mut brush = Vec2::ZERO;
        assert!(!follow(&mut brush, Vec2::new(10.0, 0.0), 20.0));
        assert_eq!(brush, Vec2::ZERO, "within reach, it stays");
        assert!(follow(&mut brush, Vec2::new(50.0, 0.0), 20.0));
        assert!((brush - Vec2::new(30.0, 0.0)).length() < 1e-4);
        assert!(!follow(&mut brush, Vec2::splat(f32::NAN), 20.0));
        // No string: it is the pointer.
        assert!(follow(&mut brush, Vec2::new(31.0, 0.0), 0.0));
        assert_eq!(brush, Vec2::new(31.0, 0.0));
    }
}
