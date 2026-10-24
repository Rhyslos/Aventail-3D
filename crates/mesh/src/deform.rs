//! Deformers ([decision 0144](../../../docs/decisions/0144-lattice-cage.md)):
//! modifiers that move a shape's points and change nothing else.
//!
//! A deformer works in **its own space**: a lattice in its cage's box, a
//! bend or twist along an axis through its centre. [`deform`] carries each
//! point of the shape into that space, asks the deformer where it goes
//! ([`Deformer::moved`]), and carries it back. Polygons, ids, UVs, seams,
//! creases, hard edges and materials all stay as they were, so a deformed
//! shape textures and selects exactly as before; only the shading follows
//! the new surface.
//!
//! Each deformer acts over a **range** of its space: a lattice over its
//! box ([`BoxRange`]), a bend, twist or taper between limits along its
//! axis ([`AxisRange`]). A range gives each place a fraction, 0 at its
//! start and 1 at its end, clamped beyond them, so a point past the range
//! is moved as its end is and the shape carries on smoothly, with no tear.
//!
//! Points the deformer would send somewhere that isn't a real number stay
//! where they were. Many points are worked out on several threads.

use crate::{Mesh, VertexId};
use glam::{Mat4, Vec3};
use words::tr;

/// Something that moves points in its own space: the work of a deformer
/// modifier (a lattice; later bend, twist and taper).
pub trait Deformer: Sync {
    /// Where the point at `p` (in the deformer's space) goes, in the same
    /// space.
    fn moved(&self, p: Vec3) -> Vec3;
}

/// A stretch along one axis of a deformer's space, from `from` to `to`
/// (never the wrong way round).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisRange {
    from: f32,
    to: f32,
}

impl AxisRange {
    /// The stretch between `a` and `b`, in either order; `None` if either
    /// isn't a real number. It may have no length (`a == b`).
    pub fn new(a: f32, b: f32) -> Option<Self> {
        (a.is_finite() && b.is_finite()).then(|| Self {
            from: a.min(b),
            to: a.max(b),
        })
    }

    /// Where it starts (the smaller end).
    pub fn from(self) -> f32 {
        self.from
    }

    /// Where it ends (the larger end).
    pub fn to(self) -> f32 {
        self.to
    }

    /// How long it is.
    pub fn length(self) -> f32 {
        self.to - self.from
    }

    /// `x` brought within the stretch.
    pub fn clamp(self, x: f32) -> f32 {
        if x.is_nan() {
            return self.from;
        }
        x.clamp(self.from, self.to)
    }

    /// How far along the stretch `x` is: 0 at its start, 1 at its end,
    /// clamped beyond them (0 everywhere for a stretch with no length).
    pub fn fraction(self, x: f32) -> f32 {
        let length = self.length();
        if length <= f32::EPSILON * self.to.abs().max(self.from.abs()).max(1.0) {
            return 0.0;
        }
        ((self.clamp(x) - self.from) / length).clamp(0.0, 1.0)
    }

    /// Whether `x` is within the stretch (its ends included).
    pub fn contains(self, x: f32) -> bool {
        x >= self.from && x <= self.to
    }
}

/// A box in a deformer's space, as three stretches: a lattice's cage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxRange {
    /// Along X, Y and Z.
    pub axes: [AxisRange; 3],
}

impl BoxRange {
    /// The box of side 1 centred on the space's middle.
    pub const UNIT: Self = Self {
        axes: [AxisRange {
            from: -0.5,
            to: 0.5,
        }; 3],
    };

    /// The box of `size` centred on the space's middle; `None` if a side
    /// isn't a real number.
    pub fn centred(size: Vec3) -> Option<Self> {
        let half = size.abs() * 0.5;
        Some(Self {
            axes: [
                AxisRange::new(-half.x, half.x)?,
                AxisRange::new(-half.y, half.y)?,
                AxisRange::new(-half.z, half.z)?,
            ],
        })
    }

    /// How far across the box `p` is along each axis, each 0 to 1,
    /// clamped outside it.
    pub fn fraction(&self, p: Vec3) -> Vec3 {
        Vec3::new(
            self.axes[0].fraction(p.x),
            self.axes[1].fraction(p.y),
            self.axes[2].fraction(p.z),
        )
    }

    /// Whether `p` is inside the box (its sides included).
    pub fn contains(&self, p: Vec3) -> bool {
        self.axes[0].contains(p.x) && self.axes[1].contains(p.y) && self.axes[2].contains(p.z)
    }
}

/// Why a deformer couldn't move a shape's points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeformProblem {
    /// The deformer's space is flattened (scaled to nothing along an
    /// axis) or isn't made of real numbers.
    FlatSpace,
}

impl std::fmt::Display for DeformProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FlatSpace => f.write_str(tr(
                "its space is flattened (scaled to nothing along an axis), so points can't be \
                 placed in it",
            )),
        }
    }
}

impl std::error::Error for DeformProblem {}

/// `mesh` with each point moved by `deformer`. `into` carries the mesh's
/// space into the deformer's; the result is in the mesh's space again.
/// Everything but the points' places is kept. A point the deformer sends
/// to a place that isn't a real number stays where it was.
pub fn deform(mesh: &Mesh, into: Mat4, deformer: &impl Deformer) -> Result<Mesh, DeformProblem> {
    if !into.is_finite() || into.determinant().abs() < 1e-12 {
        return Err(DeformProblem::FlatSpace);
    }
    let back = into.inverse();
    if !back.is_finite() {
        return Err(DeformProblem::FlatSpace);
    }
    Ok(with_points_moved(mesh, |_, p| {
        back.transform_point3(deformer.moved(into.transform_point3(p)))
    }))
}

/// `mesh` with each point moved to `moved(id, place)`, in the mesh's own
/// space, and everything else kept: the shared work of every deformer,
/// for those that need to know which point they move (Displace reads a
/// point's normal and whether it was chosen, decision 0157). A point sent
/// to a place that isn't a real number stays where it was. Many points
/// are worked out on several threads.
pub fn with_points_moved(mesh: &Mesh, moved: impl Fn(VertexId, Vec3) -> Vec3 + Sync) -> Mesh {
    let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    let parts = crate::parallel::in_parallel(points.len(), |range| {
        points[range]
            .iter()
            .map(|&(id, p)| {
                let to = moved(id, p);
                (id, if to.is_finite() { to } else { p })
            })
            .collect::<Vec<_>>()
    });
    let mut out = mesh.clone();
    for (id, p) in parts.into_iter().flatten() {
        out.set_position(id, p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrimitiveKind;

    /// Moves every point up by its X.
    struct Shear;
    impl Deformer for Shear {
        fn moved(&self, p: Vec3) -> Vec3 {
            p + Vec3::Z * p.x
        }
    }

    /// Sends every point nowhere.
    struct Broken;
    impl Deformer for Broken {
        fn moved(&self, _: Vec3) -> Vec3 {
            Vec3::NAN
        }
    }

    fn cube() -> Mesh {
        PrimitiveKind::Cube.standard().mesh()
    }

    #[test]
    fn ranges_give_fractions_clamped_beyond_their_ends() {
        let r = AxisRange::new(10.0, -10.0).unwrap();
        assert_eq!((r.from(), r.to(), r.length()), (-10.0, 10.0, 20.0));
        assert_eq!(r.fraction(-10.0), 0.0);
        assert_eq!(r.fraction(0.0), 0.5);
        assert_eq!(r.fraction(10.0), 1.0);
        assert_eq!(r.fraction(50.0), 1.0);
        assert_eq!(r.fraction(-50.0), 0.0);
        assert_eq!(r.fraction(f32::NAN), 0.0);
        assert!(r.contains(3.0) && !r.contains(11.0));
        assert_eq!(AxisRange::new(f32::NAN, 1.0), None);
        assert_eq!(AxisRange::new(1.0, f32::INFINITY), None);
        let flat = AxisRange::new(2.0, 2.0).unwrap();
        assert_eq!(flat.fraction(2.0), 0.0);
        assert_eq!(flat.fraction(9.0), 0.0);
    }

    #[test]
    fn a_box_is_centred_and_clamps() {
        let b = BoxRange::centred(Vec3::new(2.0, -4.0, 0.0)).unwrap();
        assert_eq!(b.fraction(Vec3::ZERO), Vec3::new(0.5, 0.5, 0.0));
        assert_eq!(
            b.fraction(Vec3::new(5.0, -5.0, 1.0)),
            Vec3::new(1.0, 0.0, 0.0)
        );
        assert!(b.contains(Vec3::new(1.0, 2.0, 0.0)));
        assert!(!b.contains(Vec3::new(1.1, 0.0, 0.0)));
        assert_eq!(BoxRange::centred(Vec3::NAN), None);
    }

    #[test]
    fn deforming_moves_points_and_keeps_everything_else() {
        let mesh = cube();
        let out = deform(&mesh, Mat4::IDENTITY, &Shear).unwrap();
        assert!(out.same_structure(&mesh));
        assert_eq!(out.hard_edge_count(), mesh.hard_edge_count());
        for (id, p) in mesh.vertices() {
            assert_eq!(out.position(id), Some(p + Vec3::Z * p.x));
        }
        let names: Vec<_> = mesh.attributes().collect();
        assert_eq!(out.attributes().collect::<Vec<_>>(), names);
    }

    #[test]
    fn the_deformer_works_in_its_own_space() {
        // The deformer's space is the mesh's moved 100 along X: a point at
        // X = 0 is at -100 there, so it rises by -100.
        let mesh = cube();
        let into = Mat4::from_translation(Vec3::new(-100.0, 0.0, 0.0));
        let out = deform(&mesh, into, &Shear).unwrap();
        for (id, p) in mesh.vertices() {
            let want = p + Vec3::Z * (p.x - 100.0);
            assert!(out.position(id).unwrap().distance(want) < 1e-3);
        }
    }

    #[test]
    fn a_flat_space_is_refused_and_lost_points_stay() {
        let mesh = cube();
        let flat = Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0));
        assert_eq!(deform(&mesh, flat, &Shear), Err(DeformProblem::FlatSpace));
        let nan = Mat4::from_translation(Vec3::NAN);
        assert_eq!(deform(&mesh, nan, &Shear), Err(DeformProblem::FlatSpace));
        assert!(!DeformProblem::FlatSpace.to_string().is_empty());
        assert_eq!(deform(&mesh, Mat4::IDENTITY, &Broken).unwrap(), mesh);
    }

    #[test]
    fn empty_and_loose_shapes_are_fine() {
        let empty = Mesh::default();
        assert_eq!(deform(&empty, Mat4::IDENTITY, &Shear).unwrap(), empty);
        let loose =
            Mesh::from_slots(vec![Some(Vec3::X), None, Some(Vec3::Y)], Vec::new(), []).unwrap();
        let out = deform(&loose, Mat4::IDENTITY, &Shear).unwrap();
        assert_eq!(
            out.position(VertexId::from_raw(0)),
            Some(Vec3::new(1.0, 0.0, 1.0))
        );
        assert_eq!(out.position(VertexId::from_raw(2)), Some(Vec3::Y));
        assert_eq!(out.vertex_count(), 2);
    }

    #[test]
    fn many_points_on_several_threads_give_the_same_result() {
        let points: Vec<Option<Vec3>> = (0..250_000)
            .map(|i| Some(Vec3::new(i as f32 * 0.01, 0.0, 0.0)))
            .collect();
        let mesh = Mesh::from_slots(points, Vec::new(), []).unwrap();
        let out = deform(&mesh, Mat4::IDENTITY, &Shear).unwrap();
        for (id, p) in mesh.vertices().step_by(997) {
            assert_eq!(out.position(id), Some(p + Vec3::Z * p.x));
        }
    }
}
