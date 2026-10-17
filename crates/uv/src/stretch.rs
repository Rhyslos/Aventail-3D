//! How evenly UVs lie, polygon by polygon, and what to do about it
//! ([decision 0102](../../../docs/decisions/0102-checker-and-stretch.md)).
//!
//! The map from a polygon's surface to the texture stretches it by two
//! amounts (its largest and smallest stretch, at right angles). Their
//! ratio says how **unevenly** the texture lies (a square becoming a long
//! rectangle), and their product how much **room** the polygon gets,
//! compared here with the whole shape's texel density. The viewport's
//! Stretch view colours the same measures.

use glam::{Vec2, Vec3};
use mesh::uvs::Uvs;
use mesh::{FaceId, Mesh};

/// From this far off (1.5 times) a polygon counts as stretched.
pub const SOME: f32 = 1.5;
/// From this far off (twice) a polygon is far off.
pub const FAR: f32 = 2.0;

/// How one polygon lies on the texture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    /// How many times more it is stretched one way than the other (1:
    /// evenly).
    pub uneven: f32,
    /// How much room it gets against the shape's texel density (1: the
    /// same; 2: twice as much texture per length).
    pub size: f32,
    /// Whether it lies mirrored on the texture.
    pub flipped: bool,
    /// Whether it has no room at all (squashed flat, or no UVs).
    pub empty: bool,
}

impl Stretch {
    /// How far off it is, as a power of two: 0 even, about 0.58 at 1.5
    /// times, 1 at twice.
    pub fn off(&self) -> f32 {
        if self.empty {
            return f32::INFINITY;
        }
        self.uneven
            .max(1.0)
            .log2()
            .max(self.size.max(1e-6).log2().abs())
    }

    /// What is wrong, if anything, in plain words, and what to do.
    pub fn explain(&self) -> Option<String> {
        if self.empty {
            return Some(
                "has no room on the texture: its UVs are squashed flat (an extrude's new walls \
                 start like this). Fix: select it and Unwrap."
                    .into(),
            );
        }
        if self.flipped {
            return Some(
                "lies mirrored on the texture, so a texture shows back to front on it. Fix: \
                 Unwrap its piece, or flip it back."
                    .into(),
            );
        }
        if self.uneven >= SOME && self.uneven.log2() >= self.size.log2().abs() {
            return Some(format!(
                "is stretched {:.1} times more one way than the other, so a texture's squares \
                 become long rectangles. Fix: Relax it; if its piece wraps round the shape, \
                 add a seam across it and Unwrap.",
                self.uneven
            ));
        }
        if self.size >= SOME {
            return Some(format!(
                "gets {:.1} times more texture for its size than the rest of the shape. Fix: \
                 Unwrap its piece (it keeps the shape's density), or Set texel density.",
                self.size
            ));
        }
        if self.size <= 1.0 / SOME {
            return Some(format!(
                "gets {:.1} times less texture for its size than the rest of the shape, so it \
                 looks blurrier. Fix: Unwrap its piece, or Set texel density.",
                1.0 / self.size
            ));
        }
        None
    }
}

/// The two stretches of the map taking a triangle's corners `p` (on the
/// surface) to `uv`, and whether it mirrors; `None` for a triangle with no
/// area on the surface.
pub(crate) fn stretches(p: [Vec3; 3], uv: [Vec2; 3]) -> Option<(f32, f32, bool)> {
    let e1 = p[1] - p[0];
    let e2 = p[2] - p[0];
    let across = e1.cross(e2);
    if across.length() < 1e-12 || e1.length() < 1e-9 {
        return None;
    }
    let x = e1 / e1.length();
    let y = across.normalize().cross(x);
    let a = Vec2::new(e1.length(), 0.0);
    let b = Vec2::new(e2.dot(x), e2.dot(y));
    let det = a.x * b.y - b.x * a.y;
    if det.abs() < 1e-12 {
        return None;
    }
    // J = U · P⁻¹, with P's columns a, b and U's the UV edges.
    let (u1, u2) = (uv[1] - uv[0], uv[2] - uv[0]);
    let inv = [Vec2::new(b.y, -a.y) / det, Vec2::new(-b.x, a.x) / det];
    let j0 = u1 * inv[0].x + u2 * inv[0].y;
    let j1 = u1 * inv[1].x + u2 * inv[1].y;
    let (aa, ab, bb) = (j0.dot(j0), j0.dot(j1), j1.dot(j1));
    let trace = aa + bb;
    let d = aa * bb - ab * ab;
    let spread = (trace * trace / 4.0 - d).max(0.0).sqrt();
    let most = (trace / 2.0 + spread).max(0.0).sqrt();
    let least = (trace / 2.0 - spread).max(0.0).sqrt();
    let mirrored = j0.perp_dot(j1) < 0.0;
    Some((most, least, mirrored))
}

/// How each polygon of `mesh` lies on the texture, in face order, if the
/// mesh has UVs.
pub fn stretch_of(mesh: &Mesh) -> Option<Vec<(FaceId, Stretch)>> {
    let uvs = Uvs::of(mesh)?;
    let average = mesh::uvs::average_density(mesh).unwrap_or(0.0);
    let (starts, _) = mesh.face_layout();
    let mut out = Vec::new();
    for (f, list) in mesh.faces() {
        let start = starts[f.get() as usize] as usize;
        let mut weight = 0.0;
        let mut uneven = 0.0;
        let mut room = 0.0;
        let mut mirrored = 0.0;
        mesh::triangulate::face_triangles(mesh, list, |[a, b, c]| {
            let p = |i: usize| mesh.position(list[i]).unwrap_or(Vec3::ZERO);
            let corners = [p(a), p(b), p(c)];
            let area = (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .length();
            if let Some((most, least, flipped)) = stretches(
                corners,
                [uvs.at(start + a), uvs.at(start + b), uvs.at(start + c)],
            ) {
                weight += area;
                uneven += area * (most / least.max(most * 1e-6)).log2();
                room += area * (most * least).sqrt();
                if flipped {
                    mirrored += area;
                }
            }
        });
        let stretch = if weight <= 0.0 {
            Stretch {
                uneven: 1.0,
                size: 1.0,
                flipped: false,
                empty: false,
            }
        } else {
            let room = room / weight;
            Stretch {
                uneven: (uneven / weight).exp2(),
                size: if average > 0.0 { room / average } else { 1.0 },
                flipped: mirrored > weight / 2.0,
                empty: room <= average * 1e-3 || average <= 0.0,
            }
        };
        out.push((f, stretch));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::{CubeParams, PlaneParams, Primitive};

    #[test]
    fn a_ready_made_cube_lies_evenly() {
        let cube = Primitive::Cube(CubeParams::default()).mesh();
        for (_, s) in stretch_of(&cube).unwrap() {
            assert!(s.off() < 0.01, "{s:?}");
            assert!(s.explain().is_none());
        }
    }

    #[test]
    fn stretched_squashed_and_mirrored_polygons_are_told_apart_and_explained() {
        let mut plane = Primitive::Plane(PlaneParams {
            segments: [2, 1],
            ..Default::default()
        })
        .mesh();
        let faces: Vec<FaceId> = plane.faces().map(|(f, _)| f).collect();
        // One polygon squashed to a third one way: uneven.
        let squash = glam::Affine2::from_scale(Vec2::new(1.0 / 3.0, 1.0));
        let mut values = Uvs::of(&plane).unwrap().into_values(8);
        let (starts, _) = plane.face_layout();
        let start = starts[faces[0].get() as usize] as usize;
        let middle = (values[start] + values[start + 2]) / 2.0;
        for v in &mut values[start..start + 4] {
            *v = middle + squash.transform_vector2(*v - middle);
        }
        mesh::uvs::set_uvs(&mut plane, values.clone());
        let found = stretch_of(&plane).unwrap();
        let first = found.iter().find(|(f, _)| *f == faces[0]).unwrap().1;
        assert!(first.uneven > 2.5, "{first:?}");
        assert!(first.explain().unwrap().contains("stretched"));
        // Mirrored.
        for v in &mut values[start..start + 4] {
            v.x = 2.0 * middle.x - v.x;
        }
        mesh::uvs::set_uvs(&mut plane, values.clone());
        let first = stretch_of(&plane)
            .unwrap()
            .into_iter()
            .find(|(f, _)| *f == faces[0])
            .unwrap()
            .1;
        assert!(first.flipped);
        assert!(first.explain().unwrap().contains("mirrored"));
        // Squashed flat: no room.
        for v in &mut values[start..start + 4] {
            *v = middle;
        }
        mesh::uvs::set_uvs(&mut plane, values);
        let first = stretch_of(&plane)
            .unwrap()
            .into_iter()
            .find(|(f, _)| *f == faces[0])
            .unwrap()
            .1;
        assert!(first.empty && first.off().is_infinite());
        assert!(first.explain().unwrap().contains("no room"));
    }

    #[test]
    fn too_much_or_too_little_room_is_explained() {
        let s = Stretch {
            uneven: 1.0,
            size: 2.4,
            flipped: false,
            empty: false,
        };
        assert!(s.explain().unwrap().contains("more texture"));
        let s = Stretch { size: 0.4, ..s };
        assert!(s.explain().unwrap().contains("less texture"));
        assert!((s.off() - 0.4f32.log2().abs()).abs() < 1e-5);
    }

    #[test]
    fn a_shape_without_uvs_has_no_stretch() {
        let mut cube = Primitive::Cube(CubeParams::default()).mesh();
        cube.remove_attribute(mesh::attributes::UV);
        assert!(stretch_of(&cube).is_none());
    }
}
