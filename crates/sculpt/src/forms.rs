//! What one dab of the form brushes does to a point (decision 0306):
//! Clay strips and Clay, Inflate, Crease, Pinch, Flatten, Scrape and Fill,
//! Grab, Elastic grab and Snake hook. Plain functions of a point, its
//! distance from the brush's middle and what the live sculpt worked out
//! for the whole dab (its plane, its facing); the live sculpt
//! ([`crate::LiveSculpt`]) finds the points and applies the results.

use crate::brush::Dab;
use glam::Vec3;

/// How high Clay strips and Clay lay the clay above the plane under the
/// brush, as a share of the brush's radius: high enough that a round
/// surface under a big brush still takes clay in the middle.
pub const CLAY_RATE: f32 = 0.15;
/// How far towards the clay's level one dab at full strength moves a
/// point.
pub const CLAY_SHARE: f32 = 0.25;
/// How far one Inflate dab at full strength swells the middle of the
/// brush, as a share of its radius.
pub const INFLATE_RATE: f32 = 0.03;
/// How deep one Crease dab at full strength cuts, as a share of its
/// radius.
pub const CREASE_RATE: f32 = 0.04;
/// How far one Crease dab at full strength pinches its sides in.
pub const CREASE_PINCH: f32 = 0.1;
/// How far one Pinch dab at full strength pulls points towards the
/// brush's middle.
pub const PINCH_RATE: f32 = 0.15;
/// How far one Flatten, Scrape or Fill dab at full strength moves a point
/// towards the plane.
pub const FLATTEN_RATE: f32 = 0.5;
/// How springy Elastic grab's rubber is (Poisson's ratio): one half keeps
/// the volume (the material can't be squashed).
pub const POISSON: f32 = 0.5;
/// How far Elastic grab reaches, as a multiple of the brush's radius; it
/// fades to nothing over the outer half.
pub const ELASTIC_REACH: f32 = 4.0;

/// A plane under the brush: through a point, facing a way of unit length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    /// A point on it.
    pub point: Vec3,
    /// The way it faces.
    pub normal: Vec3,
}

impl Plane {
    /// How far `p` is above the plane (below: less than nothing).
    pub fn height(&self, p: Vec3) -> f32 {
        (p - self.point).dot(self.normal)
    }
}

/// Which side of the plane Flatten works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Both: Flatten.
    Both,
    /// Only what sticks out above: Scrape.
    Above,
    /// Only the hollows below: Fill.
    Below,
}

/// The dab's strength at `distance` from its middle, by its falloff.
pub fn weight(dab: &Dab, distance: f32) -> f32 {
    dab.falloff.at(distance / dab.radius.max(1e-12)) * dab.strength.clamp(0.0, 1.0)
}

/// How far `p` is from the middle of a square brush lying on `plane`,
/// turned so `along` (on the plane) runs up its middle, as a share of
/// `radius`: 1 on the square's edge. The square's corners are farther
/// from the middle than its radius, so the live sculpt gathers points
/// within `radius × √2`.
pub fn square_distance(plane: &Plane, along: Vec3, p: Vec3, radius: f32) -> f32 {
    let d = p - plane.point;
    let across = plane.normal.cross(along);
    d.dot(along).abs().max(d.dot(across).abs()) / radius.max(1e-12)
}

/// A way along `plane` for a square brush to point: `stroke` (the way
/// the brush moved) laid on the plane, or any way along it when the brush
/// hasn't moved.
pub fn along_plane(plane: &Plane, stroke: Vec3) -> Vec3 {
    let n = plane.normal;
    (stroke - n * stroke.dot(n))
        .try_normalize()
        .unwrap_or_else(|| n.any_orthonormal_vector())
}

/// Where Clay strips or Clay moves a point at `p` with strength `weight`
/// there: up towards a level above `plane` (by [`CLAY_RATE`] of the
/// radius), only if it is below it, so hollows fill first. Inverted, down
/// towards a level below. Points more than the radius behind the plane
/// (the far side of a thin part) stay.
pub fn clay(dab: &Dab, plane: &Plane, p: Vec3, weight: f32) -> Vec3 {
    let level = CLAY_RATE * dab.radius;
    let h = plane.height(p);
    let share = (weight * CLAY_SHARE).clamp(0.0, 1.0);
    if dab.invert {
        if h > -level && h < dab.radius {
            return p - plane.normal * ((h + level) * share);
        }
    } else if h < level && h > -dab.radius {
        return p + plane.normal * ((level - h) * share);
    }
    p
}

/// Where Inflate moves a point at `p`, `distance` from the middle, facing
/// `normal` (its own facing, of unit length).
pub fn inflate(dab: &Dab, normal: Vec3, p: Vec3, distance: f32) -> Vec3 {
    let sign = if dab.invert { -1.0 } else { 1.0 };
    p + normal * (weight(dab, distance) * INFLATE_RATE * dab.radius * sign)
}

/// The part of `p`'s offset from `centre` that runs along the surface
/// (across `facing`): what Pinch pulls in.
fn sideways(centre: Vec3, facing: Vec3, p: Vec3) -> Vec3 {
    let d = p - centre;
    d - facing * d.dot(facing)
}

/// Where Pinch moves a point at `p`, `distance` from the middle: towards
/// the line through the brush's middle along `facing` (apart, inverted).
pub fn pinch(dab: &Dab, facing: Vec3, p: Vec3, distance: f32) -> Vec3 {
    let sign = if dab.invert { -1.0 } else { 1.0 };
    let share = (weight(dab, distance) * PINCH_RATE).clamp(0.0, 1.0) * sign;
    p - sideways(dab.centre, facing, p) * share
}

/// Where Crease moves a point at `p`, `distance` from the middle: in
/// along `facing` (out, inverted, for a ridge), its sides pinched
/// together either way.
pub fn crease(dab: &Dab, facing: Vec3, p: Vec3, distance: f32) -> Vec3 {
    let w = weight(dab, distance);
    let sign = if dab.invert { 1.0 } else { -1.0 };
    let cut = facing * (w * CREASE_RATE * dab.radius * sign);
    let pinched = sideways(dab.centre, facing, p) * (w * CREASE_PINCH).clamp(0.0, 1.0);
    p + cut - pinched
}

/// Where Flatten, Scrape or Fill moves a point at `p`, `distance` from
/// the middle, towards `plane` on `side` of it. Inverted, Flatten pushes
/// away from the plane, and Scrape and Fill swap.
pub fn flatten(dab: &Dab, plane: &Plane, side: Side, p: Vec3, distance: f32) -> Vec3 {
    let h = plane.height(p);
    let side = match (side, dab.invert) {
        (Side::Above, true) => Side::Below,
        (Side::Below, true) => Side::Above,
        (side, _) => side,
    };
    let works = match side {
        Side::Both => true,
        Side::Above => h > 0.0,
        Side::Below => h < 0.0,
    };
    if !works {
        return p;
    }
    let mut share = (weight(dab, distance) * FLATTEN_RATE).clamp(0.0, 1.0);
    if dab.invert && side == Side::Both {
        share = -share;
    }
    p - plane.normal * (h * share)
}

/// Where Grab puts a point that was at `from`, `distance` from where the
/// grab began, when the brush has moved `motion` since.
pub fn grab(dab: &Dab, from: Vec3, distance: f32, motion: Vec3) -> Vec3 {
    from + motion * weight(dab, distance)
}

/// How a regularised Kelvinlet (de Goes and James, 2017) moves the rubber
/// at `offset` from a grab moved `motion`, with a soft middle of size
/// `size`: exactly `motion` at the middle, falling off slowly, bending
/// the shape around it while keeping its volume ([`POISSON`] one half).
pub fn kelvinlet(offset: Vec3, motion: Vec3, size: f32) -> Vec3 {
    let e = size.max(1e-12);
    let a = 1.0;
    let b = a / (4.0 * (1.0 - POISSON));
    let r2 = offset.length_squared() + e * e;
    let re = r2.sqrt();
    let re3 = re * r2;
    let field =
        motion * ((a - b) / re + a * e * e / (2.0 * re3)) + offset * (b / re3 * offset.dot(motion));
    // At the middle the field is (3a/2 − b)/e times the motion: scaled so
    // the middle moves with the mouse.
    field * (e / (1.5 * a - b))
}

/// Where one step of Elastic grab puts a point now at `from`, `distance`
/// from where the grab is (`centre`), when the brush moves `motion`: the
/// Kelvinlet, by the strength, fading to nothing over the outer half of
/// its reach so nothing past it jumps. The live sculpt takes it step by
/// step, dab by dab, so each small bend keeps the volume.
pub fn elastic(dab: &Dab, centre: Vec3, from: Vec3, distance: f32, motion: Vec3) -> Vec3 {
    let reach = dab.radius * ELASTIC_REACH;
    let x = distance / reach.max(1e-12);
    let fade = if x <= 0.5 {
        1.0
    } else if x >= 1.0 {
        0.0
    } else {
        let t = (x - 0.5) / 0.5;
        1.0 - t * t * (3.0 - 2.0 * t)
    };
    let strength = dab.strength.clamp(0.0, 1.0);
    from + kelvinlet(from - centre, motion, dab.radius) * (fade * strength)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::{BrushKind, Falloff};

    fn dab(kind: BrushKind) -> Dab {
        Dab {
            kind,
            centre: Vec3::ZERO,
            radius: 10.0,
            strength: 1.0,
            falloff: Falloff::Smooth,
            invert: false,
        }
    }

    const FLAT: Plane = Plane {
        point: Vec3::ZERO,
        normal: Vec3::Z,
    };

    #[test]
    fn clay_fills_up_to_its_level_and_never_past_it() {
        let d = dab(BrushKind::Clay);
        let level = CLAY_RATE * 10.0;
        let low = clay(&d, &FLAT, Vec3::new(0.0, 0.0, -0.2), 1.0);
        assert!((low.z - (-0.2 + (level + 0.2) * CLAY_SHARE)).abs() < 1e-6);
        // Above the clay's level: left alone.
        let high = Vec3::new(0.0, 0.0, level + 0.1);
        assert_eq!(clay(&d, &FLAT, high, 1.0), high);
        // Far behind the plane (the other side of a thin part): left.
        let behind = Vec3::new(0.0, 0.0, -11.0);
        assert_eq!(clay(&d, &FLAT, behind, 1.0), behind);
        // Inverted: carved down to a level below.
        let carved = clay(&Dab { invert: true, ..d }, &FLAT, Vec3::ZERO, 1.0);
        assert!((carved.z + level * CLAY_SHARE).abs() < 1e-6);
        assert_eq!(clay(&d, &FLAT, Vec3::ZERO, 0.0), Vec3::ZERO);
    }

    #[test]
    fn a_square_brush_measures_to_its_edges_and_turns_with_the_stroke() {
        let along = along_plane(&FLAT, Vec3::new(1.0, 0.0, 5.0));
        assert!((along - Vec3::X).length() < 1e-6);
        // The corner of the square is on its edge.
        assert!(
            (square_distance(&FLAT, along, Vec3::new(10.0, 10.0, 0.0), 10.0) - 1.0).abs() < 1e-6
        );
        assert!((square_distance(&FLAT, along, Vec3::new(5.0, 0.0, 3.0), 10.0) - 0.5).abs() < 1e-6);
        // Turned 45 degrees, the same point is past the edge.
        let turned = along_plane(&FLAT, Vec3::new(1.0, 1.0, 0.0));
        assert!(square_distance(&FLAT, turned, Vec3::new(10.0, 10.0, 0.0), 10.0) > 1.4);
        // No stroke yet: still some way along the plane.
        let any = along_plane(&FLAT, Vec3::ZERO);
        assert!(any.dot(Vec3::Z).abs() < 1e-6 && (any.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn inflate_swells_along_each_points_own_facing() {
        let d = dab(BrushKind::Inflate);
        let p = inflate(&d, Vec3::X, Vec3::ZERO, 0.0);
        assert!((p - Vec3::X * INFLATE_RATE * 10.0).length() < 1e-6);
        let q = inflate(&Dab { invert: true, ..d }, Vec3::X, Vec3::ZERO, 0.0);
        assert!((q + Vec3::X * INFLATE_RATE * 10.0).length() < 1e-6);
        assert_eq!(inflate(&d, Vec3::X, Vec3::ONE, 10.0), Vec3::ONE);
    }

    #[test]
    fn pinch_pulls_sideways_only_and_ctrl_pushes_apart() {
        let d = dab(BrushKind::Pinch);
        let p = Vec3::new(4.0, 0.0, 1.0);
        let pulled = pinch(&d, Vec3::Z, p, p.length());
        assert!(pulled.x < 4.0 && pulled.x > 0.0);
        assert_eq!(pulled.z, 1.0);
        let pushed = pinch(&Dab { invert: true, ..d }, Vec3::Z, p, p.length());
        assert!(pushed.x > 4.0);
    }

    #[test]
    fn crease_cuts_in_and_pinches_and_ctrl_makes_a_ridge() {
        let d = dab(BrushKind::Crease);
        let p = Vec3::new(2.0, 0.0, 0.0);
        let cut = crease(&d, Vec3::Z, p, 2.0);
        assert!(cut.z < 0.0 && cut.x < 2.0);
        let ridge = crease(&Dab { invert: true, ..d }, Vec3::Z, p, 2.0);
        assert!(ridge.z > 0.0 && ridge.x < 2.0);
    }

    #[test]
    fn flatten_scrape_and_fill_work_on_their_sides() {
        let d = dab(BrushKind::Flatten);
        let (up, down) = (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(flatten(&d, &FLAT, Side::Both, up, 0.0).z < 1.0);
        assert!(flatten(&d, &FLAT, Side::Both, down, 0.0).z > -1.0);
        assert!(flatten(&d, &FLAT, Side::Above, up, 0.0).z < 1.0);
        assert_eq!(flatten(&d, &FLAT, Side::Above, down, 0.0), down);
        assert_eq!(flatten(&d, &FLAT, Side::Below, up, 0.0), up);
        assert!(flatten(&d, &FLAT, Side::Below, down, 0.0).z > -1.0);
        let inv = Dab { invert: true, ..d };
        // Inverted Flatten pushes away; Scrape and Fill swap.
        assert!(flatten(&inv, &FLAT, Side::Both, up, 0.0).z > 1.0);
        assert_eq!(flatten(&inv, &FLAT, Side::Above, up, 0.0), up);
        assert!(flatten(&inv, &FLAT, Side::Above, down, 0.0).z > -1.0);
    }

    #[test]
    fn a_kelvinlet_moves_its_middle_with_the_mouse_and_fades_slowly() {
        let motion = Vec3::new(0.0, 0.0, 2.0);
        let middle = kelvinlet(Vec3::ZERO, motion, 3.0);
        assert!((middle - motion).length() < 1e-5, "{middle}");
        let near = kelvinlet(Vec3::new(3.0, 0.0, 0.0), motion, 3.0).length();
        let far = kelvinlet(Vec3::new(30.0, 0.0, 0.0), motion, 3.0).length();
        assert!(near < 2.0 && far < near && far > 0.0);
        // Volume kept: the field's divergence is nothing, by numbers.
        let at = Vec3::new(1.3, -0.7, 2.1);
        let h = 1e-2;
        let div: f32 = [Vec3::X, Vec3::Y, Vec3::Z]
            .iter()
            .map(|axis| {
                let up = kelvinlet(at + *axis * h, motion, 3.0);
                let down = kelvinlet(at - *axis * h, motion, 3.0);
                (up - down).dot(*axis) / (2.0 * h)
            })
            .sum();
        assert!(div.abs() < 1e-3, "{div}");
        // Elastic grab fades out past its reach.
        let d = dab(BrushKind::ElasticGrab);
        let gone = Vec3::new(ELASTIC_REACH * 10.0 + 1.0, 0.0, 0.0);
        assert_eq!(elastic(&d, Vec3::ZERO, gone, gone.length(), motion), gone);
    }

    #[test]
    fn grab_moves_by_the_falloff_from_where_it_began() {
        let d = dab(BrushKind::Grab);
        let m = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(grab(&d, Vec3::ZERO, 0.0, m), m);
        assert_eq!(grab(&d, Vec3::ONE, 10.0, m), Vec3::ONE);
    }
}
