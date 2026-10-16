//! The view dice's sides ([decision 0084](../../../docs/decisions/0084-view-dice.md)):
//! the 26 directions a view can look from, like 3ds Max's view cube. Six
//! faces (straight on), twelve edges (from 45° between two faces) and
//! eight corners (a three-quarter view).
//!
//! A side is a direction from the middle of the scene out to where the
//! camera stands, written with each of X, Y and Z as -1, 0 or 1: `(0, 0,
//! 1)` is the top, `(1, -1, 0)` the edge between the front and the right,
//! `(1, -1, 1)` the top front right corner. The front is -Y, as the Front
//! view looks along +Y from there.
//!
//! Plain maths, tested without a window; the viewport draws the dice and
//! asks the [`crate::Navigator`] to look from a side.

use glam::{IVec3, Vec3};

/// Every side of the dice: the 6 faces first, then the 12 edges, then the
/// 8 corners.
pub fn sides() -> Vec<IVec3> {
    let mut all: Vec<IVec3> = Vec::with_capacity(26);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if (x, y, z) != (0, 0, 0) {
                    all.push(IVec3::new(x, y, z));
                }
            }
        }
    }
    // Faces, edges, corners: by how many of X, Y and Z they use.
    all.sort_by_key(|s| s.abs().element_sum());
    all
}

/// Which way the camera looks when it looks from `side`: towards the
/// middle, so the opposite of the side's direction (unit length). Zero
/// for the zero vector, which is no side.
pub fn looking_from(side: IVec3) -> Vec3 {
    (-side.as_vec3()).normalize_or_zero()
}

/// The side nearest to looking along `forward` (the camera's direction):
/// the one the camera most nearly looks from. The top for a direction
/// that isn't a number or is zero.
pub fn nearest(forward: Vec3) -> IVec3 {
    let from = -forward.normalize_or_zero();
    if from == Vec3::ZERO || !from.is_finite() {
        return IVec3::Z;
    }
    sides()
        .into_iter()
        .max_by(|a, b| {
            let da = a.as_vec3().normalize().dot(from);
            let db = b.as_vec3().normalize().dot(from);
            da.total_cmp(&db)
        })
        .unwrap_or(IVec3::Z)
}

/// The side's name, as the dice's tooltip and its accessible label say
/// it: "top", "front right", "top front right". Empty for no side.
pub fn name(side: IVec3) -> String {
    let words = [
        match side.z.signum() {
            1 => Some("top"),
            -1 => Some("bottom"),
            _ => None,
        },
        match side.y.signum() {
            -1 => Some("front"),
            1 => Some("back"),
            _ => None,
        },
        match side.x.signum() {
            -1 => Some("left"),
            1 => Some("right"),
            _ => None,
        },
    ];
    words.into_iter().flatten().collect::<Vec<_>>().join(" ")
}

/// A face's label, written on the dice: "Top", "Front" and so on; `None`
/// for an edge or a corner.
pub fn face_label(side: IVec3) -> Option<&'static str> {
    match (side.x, side.y, side.z) {
        (0, 0, 1) => Some("Top"),
        (0, 0, -1) => Some("Bottom"),
        (0, -1, 0) => Some("Front"),
        (0, 1, 0) => Some("Back"),
        (1, 0, 0) => Some("Right"),
        (-1, 0, 0) => Some("Left"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_six_faces_twelve_edges_and_eight_corners() {
        let all = sides();
        assert_eq!(all.len(), 26);
        let count = |n: i32| all.iter().filter(|s| s.abs().element_sum() == n).count();
        assert_eq!((count(1), count(2), count(3)), (6, 12, 8));
        // Faces first, corners last.
        assert_eq!(all[0].abs().element_sum(), 1);
        assert_eq!(all[25].abs().element_sum(), 3);
        // Each once.
        let unique: std::collections::BTreeSet<[i32; 3]> =
            all.iter().map(|s| s.to_array()).collect();
        assert_eq!(unique.len(), 26);
    }

    #[test]
    fn looking_from_a_side_looks_towards_the_middle() {
        assert_eq!(looking_from(IVec3::Z), -Vec3::Z);
        assert_eq!(looking_from(IVec3::NEG_Y), Vec3::Y);
        let corner = looking_from(IVec3::new(1, -1, 1));
        assert!((corner.length() - 1.0).abs() < 1e-6);
        assert!(corner.x < 0.0 && corner.y > 0.0 && corner.z < 0.0);
        assert_eq!(looking_from(IVec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn every_side_is_nearest_to_itself_and_slightly_off_still_snaps() {
        for side in sides() {
            let forward = looking_from(side);
            assert_eq!(nearest(forward), side);
            // A little off in any direction still snaps to it.
            let nudged = (forward + Vec3::new(0.05, -0.04, 0.03)).normalize();
            assert_eq!(nearest(nudged), side, "{side}");
        }
        // Halfway between the top and the top front edge goes to one of
        // them, never somewhere else.
        let between = (looking_from(IVec3::Z) + looking_from(IVec3::new(0, -1, 1))).normalize();
        assert!([IVec3::Z, IVec3::new(0, -1, 1)].contains(&nearest(between)));
        assert_eq!(nearest(Vec3::ZERO), IVec3::Z);
        assert_eq!(nearest(Vec3::NAN), IVec3::Z);
    }

    #[test]
    fn sides_have_names_and_faces_have_labels() {
        assert_eq!(name(IVec3::Z), "top");
        assert_eq!(name(IVec3::new(1, -1, 0)), "front right");
        assert_eq!(name(IVec3::new(-1, 1, -1)), "bottom back left");
        assert_eq!(name(IVec3::ZERO), "");
        assert_eq!(face_label(IVec3::NEG_Y), Some("Front"));
        assert_eq!(face_label(IVec3::X), Some("Right"));
        assert_eq!(face_label(IVec3::new(1, 0, 1)), None);
        let labels: Vec<_> = sides().into_iter().filter_map(face_label).collect();
        assert_eq!(labels.len(), 6);
    }
}
