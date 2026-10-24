//! The helper drawn over a selected shape with a *Bend, twist or taper*
//! modifier ([decision 0158](../../../../docs/decisions/0158-bend-twist-taper.md)):
//! its axis through the shape as it bends, twists or tapers, a cross at
//! each limit, and a small cross at its centre, as thin overlay lines in
//! every mode, as collision outlines are (decision 0151).

use super::OverlayLine;
use crate::drawing::{OverlayStyle, SceneDrawing};
use glam::Vec3;

/// The helpers' lines, in the world, for every selected shape shown.
pub(super) fn helper_lines(drawing: &SceneDrawing, scene: &scene::Scene) -> Vec<OverlayLine> {
    let mut out = Vec::new();
    for item in &drawing.items {
        if !item.selected || item.outline || !item.transform.is_finite() {
            continue;
        }
        let world = |p: Vec3| item.transform.transform_point3(p);
        for helper in scene.deform_helpers(item.id) {
            let mut lines: Vec<[Vec3; 2]> = helper.axis.windows(2).map(|w| [w[0], w[1]]).collect();
            lines.extend(helper.limits.iter().copied());
            // The centre: a small cross along the shape's own axes, half
            // a limit cross's size.
            let size = helper
                .limits
                .first()
                .map_or(1.0, |[a, b]| a.distance(*b) * 0.25)
                .max(0.25);
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let c = helper.centre;
                lines.push([c - axis * size, c + axis * size]);
            }
            out.extend(
                lines
                    .into_iter()
                    .map(|[a, b]| ([world(a), world(b)], OverlayStyle::Helper)),
            );
        }
    }
    out
}
