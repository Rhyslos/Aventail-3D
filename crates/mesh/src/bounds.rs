//! Bounding boxes: the smallest upright box around a set of points.
//!
//! Used to frame things in the viewport ("frame all", "frame selected"),
//! and later for picking and culling.

use glam::Vec3;

/// An axis-aligned box from `min` to `max`, in centimetres. Every point
/// of whatever it bounds lies inside it, edges included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The corner with the smallest X, Y and Z.
    pub min: Vec3,
    /// The corner with the largest X, Y and Z.
    pub max: Vec3,
}

impl Bounds {
    /// The box between two corners, given in any order. `None` if either
    /// corner is not a real position.
    pub fn new(a: Vec3, b: Vec3) -> Option<Self> {
        (a.is_finite() && b.is_finite()).then(|| Self {
            min: a.min(b),
            max: a.max(b),
        })
    }

    /// The smallest box around `points`, skipping any that are not real
    /// positions. `None` when there are no usable points.
    pub fn around(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        points
            .into_iter()
            .filter(|p| p.is_finite())
            .fold(None, |bounds: Option<Self>, p| {
                Some(match bounds {
                    Some(b) => Self {
                        min: b.min.min(p),
                        max: b.max.max(p),
                    },
                    None => Self { min: p, max: p },
                })
            })
    }

    /// The smallest box around both boxes.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// The middle of the box.
    pub fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// The box's width, depth and height.
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// The radius of the smallest ball around the box's middle that holds
    /// the whole box: half its diagonal. Zero for a single point.
    pub fn radius(&self) -> f32 {
        self.size().length() * 0.5
    }

    /// Whether `point` lies inside the box or on its surface.
    pub fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_can_come_in_any_order() {
        let b = Bounds::new(Vec3::new(5.0, -1.0, 2.0), Vec3::new(-3.0, 4.0, 0.0)).unwrap();
        assert_eq!(b.min, Vec3::new(-3.0, -1.0, 0.0));
        assert_eq!(b.max, Vec3::new(5.0, 4.0, 2.0));
    }

    #[test]
    fn a_box_around_points_holds_them_all() {
        let points = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-4.0, 0.5, 9.0),
            Vec3::new(2.0, -7.0, 0.0),
        ];
        let b = Bounds::around(points).unwrap();
        assert_eq!(b.min, Vec3::new(-4.0, -7.0, 0.0));
        assert_eq!(b.max, Vec3::new(2.0, 2.0, 9.0));
        assert!(points.iter().all(|p| b.contains(*p)));
    }

    #[test]
    fn nothing_usable_gives_no_box() {
        assert_eq!(Bounds::around([]), None);
        assert_eq!(Bounds::around([Vec3::NAN, Vec3::INFINITY]), None);
        assert_eq!(Bounds::new(Vec3::NAN, Vec3::ONE), None);
        let one = Bounds::around([Vec3::NAN, Vec3::ONE]).unwrap();
        assert_eq!(
            (one.min, one.max),
            (Vec3::ONE, Vec3::ONE),
            "bad points skipped"
        );
    }

    #[test]
    fn a_single_point_has_no_size() {
        let b = Bounds::around([Vec3::new(1.0, 1.0, 1.0)]).unwrap();
        assert_eq!(b.radius(), 0.0);
        assert_eq!(b.centre(), Vec3::ONE);
        assert!(b.contains(Vec3::ONE));
    }

    #[test]
    fn centre_size_and_radius_of_a_cube() {
        // The 2 m test cube standing on the grid.
        let b = Bounds::new(
            Vec3::new(-100.0, -100.0, 0.0),
            Vec3::new(100.0, 100.0, 200.0),
        )
        .unwrap();
        assert_eq!(b.centre(), Vec3::new(0.0, 0.0, 100.0));
        assert_eq!(b.size(), Vec3::splat(200.0));
        assert!((b.radius() - 100.0 * 3f32.sqrt()).abs() < 1e-3);
        assert!(!b.contains(Vec3::new(0.0, 0.0, 200.1)));
    }

    #[test]
    fn a_union_holds_both_boxes() {
        let a = Bounds::new(Vec3::ZERO, Vec3::ONE).unwrap();
        let b = Bounds::new(Vec3::splat(-2.0), Vec3::splat(-1.0)).unwrap();
        let u = a.union(b);
        assert_eq!((u.min, u.max), (Vec3::splat(-2.0), Vec3::ONE));
        assert_eq!(a.union(a), a);
    }
}
