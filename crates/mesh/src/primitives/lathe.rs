//! Shapes turned round the up axis or swept round it (decision 0129):
//! torus, capsule, dome and pipe elbow.
//!
//! Each is built as one or more *sheets*: a grid of points laid out in
//! rows, each row a ring of points (or a single point, a pole, where a
//! ring shrinks to nothing). Neighbouring rows are joined by quads, or by
//! triangles where one of them is a pole. Every sheet is also one UV piece,
//! unrolled flat at its real size: across the rows by the distance round,
//! along them by the distance along the profile. Flat ends are single
//! faces (n-gons) or, on a hollow pipe, a ring of quads, each its own UV
//! piece seen from its own side, with hard rims.
//!
//! Points round a circle start on +X and go counter-clockwise seen from
//! above, as on the cylinder and sphere. The torus, capsule and dome keep
//! their round axis on the pivot (like the cylinder, so with an odd number
//! of sides they lean a little towards their first point); the elbow, which
//! has no such axis of its own, has its bounding box centred on the pivot.
//! Every shape stands on the ground.

use super::uvs::{Chart, corners_of, set_charts};
use super::{CapsuleParams, DomeParams, ElbowParams, TorusParams};
use crate::polygon::VertexId;
use crate::{FaceId, Mesh};
use glam::{Vec2, Vec3};
use std::f32::consts::{FRAC_PI_2, TAU};

/// The point `i` of a row (a single-point row, a pole, answers every `i`;
/// a closed ring wraps round).
fn at(row: &[VertexId], i: usize) -> VertexId {
    row[i % row.len()]
}

/// How a grid of points is joined into faces and laid flat.
struct Sheet<'a> {
    /// The rows of points, in order along the profile. Each has either
    /// `columns` points (a closed ring), `columns + 1` (an open strip) or
    /// one (a pole).
    rows: &'a [Vec<VertexId>],
    /// How many faces across each row.
    columns: usize,
    /// Whether the last row joins back to the first (a closed profile,
    /// like the tube of a torus).
    wrap_rows: bool,
    /// Whether the faces look the other way (the inside wall of a pipe).
    inward: bool,
    /// The column where a closed ring is cut open for its UVs: its faces'
    /// UVs run on from this column round to it again.
    seam: usize,
}

/// Joins the sheet's rows with faces and returns them as one UV piece.
/// `uv(column, row)` places a grid point on the piece, where `column` runs
/// from 0 (at the seam) to `columns` and `row` from 0 to the number of rows
/// (one past the last for a closed profile, where it meets the first
/// again). Faces look the way the columns then the rows turn
/// (counter-clockwise), or the other way for `inward`.
fn sheet(mesh: &mut Mesh, sheet: &Sheet, uv: impl Fn(f32, usize) -> Vec2) -> Chart {
    let mut chart = Chart::default();
    let row_count = sheet.rows.len();
    let bands = if sheet.wrap_rows {
        row_count
    } else {
        row_count.saturating_sub(1)
    };
    for j in 0..bands {
        let (lower, upper) = (&sheet.rows[j], &sheet.rows[(j + 1) % row_count]);
        for i in 0..sheet.columns {
            // Where this face's left side lies on the UV piece.
            let c = ((i + sheet.columns - sheet.seam) % sheet.columns) as f32;
            let grid = [
                (at(lower, i), c, j),
                (at(lower, i + 1), c + 1.0, j),
                (at(upper, i + 1), c + 1.0, j + 1),
                (at(upper, i), c, j + 1),
            ];
            // A pole appears twice in a row: keep it once, placed on the UV
            // piece halfway between the two.
            let mut face: Vec<(VertexId, f32, usize)> = Vec::with_capacity(4);
            for corner in grid {
                match face.last_mut() {
                    Some(last) if last.0 == corner.0 => last.1 = (last.1 + corner.1) / 2.0,
                    _ => face.push(corner),
                }
            }
            if face.len() > 1 && face[0].0 == face[face.len() - 1].0 {
                let last = face.pop().map_or(0.0, |l| l.1);
                face[0].1 = (face[0].1 + last) / 2.0;
            }
            if face.len() < 3 {
                continue;
            }
            if sheet.inward {
                face.reverse();
            }
            let ids: Vec<VertexId> = face.iter().map(|c| c.0).collect();
            let Ok(id) = mesh.add_face(&ids) else {
                continue;
            };
            for ((corner, _), (_, column, row)) in corners_of(mesh, id).into_iter().zip(face) {
                chart.corners.push((corner, uv(column, row)));
            }
        }
    }
    chart
}

/// Lays flat faces on a UV piece of their own, seen from the side they
/// face: `right` and `up` are the directions across and up the piece.
fn flat_chart(mesh: &Mesh, faces: &[FaceId], right: Vec3, up: Vec3) -> Chart {
    let mut chart = Chart::default();
    for face in faces {
        for (corner, p) in corners_of(mesh, *face) {
            chart
                .corners
                .push((corner, Vec2::new(p.dot(right), p.dot(up))));
        }
    }
    chart
}

/// Marks the edges round a closed ring of points hard.
fn harden(mesh: &mut Mesh, ring: &[VertexId]) {
    if ring.len() < 2 {
        return;
    }
    for i in 0..ring.len() {
        mesh.set_hard(ring[i], at(ring, i + 1), true);
    }
}

/// Moves every point so the lowest stands on the ground and, if
/// `centre_across`, so the bounding box is centred on the pivot across the
/// ground too.
fn settle(mesh: &mut Mesh, centre_across: bool) {
    let Some(bounds) = mesh.bounds() else {
        return;
    };
    let centre = bounds.centre();
    let shift = if centre_across {
        Vec3::new(-centre.x, -centre.y, -bounds.min.z)
    } else {
        Vec3::new(0.0, 0.0, -bounds.min.z)
    };
    if shift == Vec3::ZERO {
        return;
    }
    let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    for (id, p) in points {
        mesh.set_position(id, p + shift);
    }
}

/// The column of a ring of `sides` points nearest the back (−X, half a
/// turn from the first point), where round surfaces are cut open.
fn back_column(sides: u32) -> usize {
    sides as usize / 2
}

// ---------------------------------------------------------------------------
// Turned shapes: a profile turned round the up axis.
// ---------------------------------------------------------------------------

/// A point on a profile turned round the up axis: its distance from the
/// axis (zero for a pole), its height, and how far along the profile it is
/// from the first point (for the UVs).
#[derive(Debug, Clone, Copy)]
struct ProfilePoint {
    radius: f32,
    height: f32,
    along: f32,
}

/// Turns `profile` (from bottom to top) round the up axis in `sides`
/// pieces, facing outward. Returns the rows of points and the turned
/// surface as one UV piece, cut open at the back.
fn turn(mesh: &mut Mesh, profile: &[ProfilePoint], sides: u32) -> (Vec<Vec<VertexId>>, Chart) {
    let rows: Vec<Vec<VertexId>> = profile
        .iter()
        .map(|p| {
            if p.radius <= 0.0 {
                return vec![mesh.push_vertex(Vec3::new(0.0, 0.0, p.height))];
            }
            (0..sides)
                .map(|i| {
                    let angle = TAU * i as f32 / sides as f32;
                    mesh.push_vertex(Vec3::new(
                        p.radius * angle.cos(),
                        p.radius * angle.sin(),
                        p.height,
                    ))
                })
                .collect()
        })
        .collect();
    // Round the strip at its widest, so it isn't squashed anywhere.
    let widest = profile.iter().map(|p| p.radius).fold(0.0, f32::max);
    let step = TAU / sides as f32 * widest;
    let chart = sheet(
        mesh,
        &Sheet {
            rows: &rows,
            columns: sides as usize,
            wrap_rows: false,
            inward: false,
            seam: back_column(sides),
        },
        |column, row| Vec2::new(column * step, profile[row].along),
    );
    (rows, chart)
}

/// A quarter of a circle of `radius` round a centre at `height`, in
/// `rings` pieces: from its side up to its top (`rising`), or from its
/// bottom up to its side. Distances along start at `along`. The first
/// point is left out when `skip_first` (it is the last of the piece
/// before).
fn quarter(
    radius: f32,
    height: f32,
    rings: u32,
    rising: bool,
    along: f32,
    skip_first: bool,
) -> Vec<ProfilePoint> {
    let first = u32::from(skip_first);
    (first..=rings)
        .map(|k| {
            let t = k as f32 / rings as f32;
            // Angle above the centre's level, from −90° to 0 or 0 to 90°.
            let angle = if rising {
                FRAC_PI_2 * t
            } else {
                FRAC_PI_2 * (t - 1.0)
            };
            // The ends exactly: a pole on the axis, a rim at full radius.
            let (radius_here, rise) = if (rising && k == rings) || (!rising && k == 0) {
                (0.0, if rising { radius } else { -radius })
            } else if (rising && k == 0) || (!rising && k == rings) {
                (radius, 0.0)
            } else {
                (radius * angle.cos(), radius * angle.sin())
            };
            ProfilePoint {
                radius: radius_here,
                height: height + rise,
                along: along + radius * FRAC_PI_2 * t,
            }
        })
        .collect()
}

/// An upright cylinder with half-ball ends: a single point at each tip.
pub(super) fn capsule(p: &CapsuleParams) -> Mesh {
    let mut mesh = Mesh::new();
    let r = p.radius;
    let middle = (p.height - 2.0 * r).max(0.0);
    let quarter_arc = r * FRAC_PI_2;
    // From the bottom tip up to the bottom rim.
    let mut profile = quarter(r, r, p.cap_rings, false, 0.0, false);
    // The straight middle, unless it has no length (then the two rims are
    // one ring, so no face is flat to nothing).
    let straight = middle > r * 1e-5;
    if straight {
        for k in 1..=p.height_segments {
            let t = k as f32 / p.height_segments as f32;
            profile.push(ProfilePoint {
                radius: r,
                height: r + middle * t,
                along: quarter_arc + middle * t,
            });
        }
    }
    let middle = if straight { middle } else { 0.0 };
    // From the top rim up to the top tip.
    profile.extend(quarter(
        r,
        r + middle,
        p.cap_rings,
        true,
        quarter_arc + middle,
        true,
    ));
    let (_, chart) = turn(&mut mesh, &profile, p.sides);
    set_charts(&mut mesh, &[chart]);
    mesh
}

/// Half a ball on its flat side, with or without a flat bottom.
pub(super) fn dome(p: &DomeParams) -> Mesh {
    let mut mesh = Mesh::new();
    let profile = quarter(p.radius, 0.0, p.rings, true, 0.0, false);
    let (rows, curved) = turn(&mut mesh, &profile, p.sides);
    let mut charts = vec![curved];
    if p.base {
        let rim = &rows[0];
        // The rim backwards, so the bottom faces down.
        let bottom: Vec<VertexId> = rim.iter().rev().copied().collect();
        if let Ok(face) = mesh.add_face(&bottom) {
            // Seen from below: across is −X, up the piece is +Y.
            charts.push(flat_chart(&mesh, &[face], Vec3::NEG_X, Vec3::Y));
        }
        harden(&mut mesh, rim);
    }
    set_charts(&mut mesh, &charts);
    mesh
}

// ---------------------------------------------------------------------------
// Swept shapes: a circle swept round the up axis.
// ---------------------------------------------------------------------------

/// A circle (the pipe's cross-section) swept round the up axis.
#[derive(Debug, Clone, Copy)]
struct Sweep {
    /// From the up axis to the pipe's middle.
    bend_radius: f32,
    /// The height of the pipe's middle.
    centre_height: f32,
    /// How far round it goes, in radians (a whole turn joins up).
    angle: f32,
    /// Pieces along the sweep.
    segments: u32,
    /// Pieces round the pipe.
    sides: u32,
}

impl Sweep {
    /// Whether it goes all the way round, so the last cross-section is the
    /// first.
    fn whole_turn(&self) -> bool {
        self.angle >= TAU
    }
}

/// One wall of a swept pipe, at `radius` from the pipe's middle, facing
/// out from it (or in, for `inward`). Returns the rows (one per step round
/// the pipe, starting at the bottom, each running along the sweep) and the
/// wall as one UV piece: across it the distance along the sweep at the
/// wall's outermost edge, up it the distance round the pipe, cut open
/// along the bottom (where it rests on the ground) and at `seam`, a column
/// along the sweep (only used when it goes all the way round).
fn swept_wall(
    mesh: &mut Mesh,
    sweep: &Sweep,
    radius: f32,
    inward: bool,
    seam: usize,
) -> (Vec<Vec<VertexId>>, Chart) {
    let whole = sweep.whole_turn();
    let columns = sweep.segments as usize;
    let points_along = if whole { columns } else { columns + 1 };
    let rows: Vec<Vec<VertexId>> = (0..sweep.sides)
        .map(|j| {
            // Round the pipe from its bottom, outward first, then over.
            let around = TAU * j as f32 / sweep.sides as f32 - FRAC_PI_2;
            let (out, up) = if j == 0 {
                (0.0, -1.0)
            } else {
                (around.cos(), around.sin())
            };
            let from_axis = sweep.bend_radius + radius * out;
            let height = sweep.centre_height + radius * up;
            (0..points_along)
                .map(|i| {
                    let a = sweep.angle * i as f32 / columns as f32;
                    mesh.push_vertex(Vec3::new(from_axis * a.cos(), from_axis * a.sin(), height))
                })
                .collect()
        })
        .collect();
    let along = sweep.angle / columns as f32 * (sweep.bend_radius + radius);
    let round = TAU / sweep.sides as f32 * radius;
    // Seen from inside, the inner wall runs the other way.
    let flip = if inward { -1.0 } else { 1.0 };
    let chart = sheet(
        mesh,
        &Sheet {
            rows: &rows,
            columns,
            wrap_rows: true,
            inward,
            seam: if whole { seam } else { 0 },
        },
        |column, row| Vec2::new(flip * column * along, row as f32 * round),
    );
    (rows, chart)
}

/// The cross-section ring at column `i` of a swept wall's rows, in order
/// round the pipe.
fn cross_section(rows: &[Vec<VertexId>], i: usize) -> Vec<VertexId> {
    rows.iter().map(|row| row[i]).collect()
}

/// A ring (doughnut) lying flat on the ground.
pub(super) fn torus(p: &TorusParams) -> Mesh {
    let mut mesh = Mesh::new();
    let sweep = Sweep {
        bend_radius: p.radius,
        centre_height: p.tube_radius,
        angle: TAU,
        segments: p.sides,
        sides: p.tube_sides,
    };
    let (_, chart) = swept_wall(
        &mut mesh,
        &sweep,
        p.tube_radius,
        false,
        back_column(p.sides),
    );
    set_charts(&mut mesh, &[chart]);
    settle(&mut mesh, false);
    mesh
}

/// A piece of pipe bent round the up axis, solid or hollow, lying on the
/// ground.
pub(super) fn elbow(p: &ElbowParams) -> Mesh {
    let mut mesh = Mesh::new();
    let whole = p.angle >= 360.0;
    let sweep = Sweep {
        bend_radius: p.bend_radius,
        centre_height: p.pipe_radius,
        // A whole turn exactly, so the ends are known to meet.
        angle: if whole { TAU } else { p.angle.to_radians() },
        // Joined up, it needs at least three pieces to go round (fewer
        // would fold flat onto itself).
        segments: if whole { p.segments.max(3) } else { p.segments },
        sides: p.sides,
    };
    let hollow = p.inner_radius > 0.0;
    let (outer, outer_chart) = swept_wall(&mut mesh, &sweep, p.pipe_radius, false, 0);
    let mut charts = vec![outer_chart];
    let inner = if hollow {
        let (inner, inner_chart) = swept_wall(&mut mesh, &sweep, p.inner_radius, true, 0);
        charts.push(inner_chart);
        Some(inner)
    } else {
        None
    };
    if !sweep.whole_turn() {
        let last = p.segments as usize;
        // The start looks back along −Y; the end looks on along the bend.
        let end_radial = Vec3::new(sweep.angle.cos(), sweep.angle.sin(), 0.0);
        let ends = [(0, false, Vec3::X), (last, true, -end_radial)];
        for (i, at_end, right) in ends {
            let outer_ring = cross_section(&outer, i);
            let faces = match &inner {
                None => {
                    // One n-gon; its ring runs counter-clockwise seen from
                    // behind the start, so the end's is turned round.
                    let mut face = outer_ring.clone();
                    if at_end {
                        face.reverse();
                    }
                    mesh.add_face(&face).into_iter().collect::<Vec<_>>()
                }
                Some(inner) => {
                    let inner_ring = cross_section(inner, i);
                    let n = outer_ring.len();
                    let mut faces = Vec::with_capacity(n);
                    for k in 0..n {
                        let mut face = vec![
                            outer_ring[k],
                            outer_ring[(k + 1) % n],
                            inner_ring[(k + 1) % n],
                            inner_ring[k],
                        ];
                        if at_end {
                            face.reverse();
                        }
                        faces.extend(mesh.add_face(&face));
                    }
                    harden(&mut mesh, &inner_ring);
                    faces
                }
            };
            harden(&mut mesh, &outer_ring);
            charts.push(flat_chart(&mesh, &faces, right, Vec3::Z));
        }
    }
    set_charts(&mut mesh, &charts);
    settle(&mut mesh, true);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{AttributeValues, UV};
    use crate::polygon::EdgeKey;
    use crate::primitives::Primitive;
    use crate::shading::shaded_triangles;
    use std::collections::BTreeMap;
    use std::f32::consts::PI;

    /// How many faces use each edge.
    fn edge_use(mesh: &Mesh) -> BTreeMap<EdgeKey, usize> {
        let mut uses = BTreeMap::new();
        for (_, corners) in mesh.faces() {
            for i in 0..corners.len() {
                let key = EdgeKey::new(corners[i], corners[(i + 1) % corners.len()]);
                *uses.entry(key).or_insert(0) += 1;
            }
        }
        uses
    }

    fn is_closed(mesh: &Mesh) -> bool {
        edge_use(mesh).values().all(|n| *n == 2)
    }

    /// The volume inside a closed mesh (positive when faces look out).
    fn volume(mesh: &Mesh) -> f32 {
        shaded_triangles(mesh)
            .iter()
            .map(|t| t.positions[0].dot(t.positions[1].cross(t.positions[2])) / 6.0)
            .sum()
    }

    /// Checks every shape must pass: finite points, no face visiting a
    /// point twice, UVs inside the texture square, standing on the ground.
    fn assert_sound(mesh: &Mesh, what: &str) {
        assert!(mesh.face_count() > 0, "{what}: no faces");
        for (_, p) in mesh.vertices() {
            assert!(p.is_finite(), "{what}: point {p}");
        }
        for (_, corners) in mesh.faces() {
            let mut sorted = corners.to_vec();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), corners.len(), "{what}: repeated corner");
            assert!(corners.len() >= 3);
        }
        let Some((_, _, AttributeValues::Vec2(uvs))) = mesh.attribute_values(UV) else {
            panic!("{what}: no UVs");
        };
        assert_eq!(
            uvs.len(),
            mesh.face_layout().1.len(),
            "{what}: a UV per corner"
        );
        for uv in uvs {
            assert!(
                uv.is_finite() && uv.min_element() >= 0.0 && uv.max_element() <= 1.0,
                "{what}: UV {uv}"
            );
        }
        let bounds = mesh.bounds().expect("not empty");
        assert!(bounds.min.z.abs() < 1e-3, "{what}: on the ground");
    }

    fn assert_closed_solid(mesh: &Mesh, what: &str) {
        assert_sound(mesh, what);
        assert!(is_closed(mesh), "{what}: not closed");
        assert!(volume(mesh) > 0.0, "{what}: faces inward");
    }

    /// The relative difference between the mesh's volume and `exact`.
    fn volume_error(mesh: &Mesh, exact: f32) -> f32 {
        (volume(mesh) / exact - 1.0).abs()
    }

    /// Every face looks away from the point `inside` it would surround
    /// (for shapes that are star-shaped round it).
    fn faces_look_away_from(mesh: &Mesh, inside: impl Fn(Vec3) -> Vec3) {
        for (face, corners) in mesh.faces() {
            let centre = corners
                .iter()
                .filter_map(|v| mesh.position(*v))
                .sum::<Vec3>()
                / corners.len() as f32;
            let n = mesh.face_normal(face);
            assert!(n.dot(centre - inside(centre)) > 0.0, "face {face:?}");
        }
    }

    // Torus ------------------------------------------------------------

    fn torus_mesh(p: TorusParams) -> Mesh {
        Primitive::Torus(p).mesh()
    }

    #[test]
    fn a_torus_is_a_closed_ring_of_quads() {
        let mesh = torus_mesh(TorusParams::default());
        assert_closed_solid(&mesh, "torus");
        assert_eq!(mesh.face_count(), 32 * 12);
        assert_eq!(mesh.vertex_count(), 32 * 12);
        assert!(mesh.faces().all(|(_, c)| c.len() == 4));
        assert_eq!(mesh.hard_edge_count(), 0, "all soft");
        let b = mesh.bounds().unwrap();
        assert!((b.max.z - 20.0).abs() < 1e-3, "as tall as the tube");
        assert!((b.max.x - 50.0).abs() < 1e-3);
        // Each face looks away from the middle of the tube near it.
        faces_look_away_from(&mesh, |c| {
            Vec3::new(c.x, c.y, 0.0).normalize() * 40.0 + Vec3::Z * 10.0
        });
    }

    #[test]
    fn a_torus_holds_the_volume_of_a_true_ring_more_closely_with_more_sides() {
        let exact = 2.0 * PI * PI * 40.0 * 10.0 * 10.0;
        let coarse = volume_error(&torus_mesh(TorusParams::default()), exact);
        let fine = volume_error(
            &torus_mesh(TorusParams {
                sides: 128,
                tube_sides: 64,
                ..TorusParams::default()
            }),
            exact,
        );
        // 12 round the tube keep about 95% of a round tube's area.
        assert!(coarse < 0.07, "{coarse}");
        assert!(fine < 0.003, "{fine}");
        assert!(fine < coarse);
    }

    #[test]
    fn a_torus_survives_thin_and_fat_settings() {
        for p in [
            TorusParams {
                sides: 3,
                tube_sides: 3,
                ..TorusParams::default()
            },
            // As fat as it can be: the tube meets itself in the middle.
            TorusParams {
                radius: 30.0,
                tube_radius: 30.0,
                sides: 7,
                tube_sides: 4,
            },
            TorusParams {
                tube_radius: 500.0,
                ..TorusParams::default()
            },
        ] {
            let mesh = torus_mesh(p);
            assert_closed_solid(&mesh, &format!("{p:?}"));
            assert_eq!(mesh.face_count(), (p.sides * p.tube_sides) as usize);
        }
    }

    // Capsule ----------------------------------------------------------

    fn capsule_mesh(p: CapsuleParams) -> Mesh {
        Primitive::Capsule(p).mesh()
    }

    fn capsule_volume(r: f32, h: f32) -> f32 {
        PI * r * r * (h - 2.0 * r) + 4.0 / 3.0 * PI * r * r * r
    }

    #[test]
    fn a_capsule_is_closed_with_a_point_at_each_tip() {
        let p = CapsuleParams::default();
        let mesh = capsule_mesh(p);
        assert_closed_solid(&mesh, "capsule");
        // 16 round; 4 rings per end and one piece in the middle.
        assert_eq!(mesh.face_count(), 16 * (2 * 4 + 1));
        assert_eq!(mesh.vertex_count(), 2 + 16 * (2 * 4 + 1 - 1));
        let triangles = mesh.faces().filter(|(_, c)| c.len() == 3).count();
        assert_eq!(triangles, 32, "a fan at each tip");
        let tips: Vec<Vec3> = mesh
            .vertices()
            .map(|(_, p)| p)
            .filter(|p| p.truncate().length() < 1e-4)
            .collect();
        assert_eq!(tips.len(), 2, "each tip a single point");
        let b = mesh.bounds().unwrap();
        assert!((b.max.z - 100.0).abs() < 1e-3 && b.min.z.abs() < 1e-6);
        assert_eq!(mesh.hard_edge_count(), 0, "all soft");
        faces_look_away_from(&mesh, |c| Vec3::new(0.0, 0.0, c.z.clamp(25.0, 75.0)));
    }

    #[test]
    fn a_capsule_holds_its_true_volume_more_closely_with_more_sides() {
        let p = CapsuleParams::default();
        let exact = capsule_volume(p.radius, p.height);
        let coarse = volume_error(&capsule_mesh(p), exact);
        let fine = volume_error(
            &capsule_mesh(CapsuleParams {
                sides: 128,
                cap_rings: 32,
                ..p
            }),
            exact,
        );
        assert!(coarse < 0.06, "{coarse}");
        assert!(fine < 0.003, "{fine}");
        assert!(fine < coarse);
    }

    #[test]
    fn a_capsule_with_no_straight_middle_is_a_ball_without_flat_faces() {
        let p = CapsuleParams {
            radius: 25.0,
            height: 50.0,
            height_segments: 3,
            ..CapsuleParams::default()
        };
        let mesh = capsule_mesh(p);
        assert_closed_solid(&mesh, "round capsule");
        // No middle band: the two rims are one ring.
        assert_eq!(mesh.face_count(), 16 * 2 * 4);
        assert_eq!(mesh.vertex_count(), 2 + 16 * (2 * 4 - 1));
        // No face flat to nothing.
        for t in shaded_triangles(&mesh) {
            let [a, b, c] = t.positions;
            assert!((b - a).cross(c - a).length() > 1e-3);
        }
        assert!(volume_error(&mesh, capsule_volume(25.0, 50.0)) < 0.08);
    }

    #[test]
    fn a_capsule_survives_the_fewest_pieces() {
        let p = CapsuleParams {
            sides: 3,
            cap_rings: 1,
            height_segments: 1,
            ..CapsuleParams::default()
        };
        let mesh = capsule_mesh(p);
        assert_closed_solid(&mesh, "fewest");
        // Two fans of three triangles and one band of quads.
        assert_eq!(mesh.face_count(), 3 + 3 + 3);
        assert_eq!(mesh.vertex_count(), 2 + 6);
        let tiny = capsule_mesh(CapsuleParams {
            sides: 3,
            cap_rings: 1,
            height: 0.0,
            ..p
        });
        assert_closed_solid(&tiny, "fewest, no middle");
        assert_eq!(tiny.face_count(), 6);
    }

    // Dome -------------------------------------------------------------

    fn dome_mesh(p: DomeParams) -> Mesh {
        Primitive::Dome(p).mesh()
    }

    #[test]
    fn a_dome_with_a_base_is_closed_with_a_hard_rim() {
        let mesh = dome_mesh(DomeParams::default());
        assert_closed_solid(&mesh, "dome");
        assert_eq!(mesh.face_count(), 24 * 6 + 1);
        assert_eq!(mesh.vertex_count(), 24 * 5 + 24 + 1);
        let bases: Vec<_> = mesh.faces().filter(|(_, c)| c.len() == 24).collect();
        assert_eq!(bases.len(), 1, "the base is one n-gon");
        assert!((mesh.face_normal(bases[0].0) + Vec3::Z).length() < 1e-5);
        assert_eq!(mesh.hard_edge_count(), 24, "the rim");
        let tops = mesh.vertices().filter(|(_, p)| p.z > 49.999).count();
        assert_eq!(tops, 1, "the top is a single point");
        faces_look_away_from(&mesh, |_| Vec3::new(0.0, 0.0, 10.0));
    }

    #[test]
    fn a_dome_holds_half_a_balls_volume_more_closely_with_more_sides() {
        let exact = 2.0 / 3.0 * PI * 50.0f32.powi(3);
        let coarse = volume_error(&dome_mesh(DomeParams::default()), exact);
        let fine = volume_error(
            &dome_mesh(DomeParams {
                sides: 128,
                rings: 32,
                ..DomeParams::default()
            }),
            exact,
        );
        assert!(coarse < 0.05, "{coarse}");
        assert!(fine < 0.003, "{fine}");
        assert!(fine < coarse);
    }

    #[test]
    fn a_dome_without_a_base_is_open_at_the_rim() {
        let mesh = dome_mesh(DomeParams {
            base: false,
            ..DomeParams::default()
        });
        assert_sound(&mesh, "open dome");
        assert_eq!(mesh.face_count(), 24 * 6);
        assert_eq!(mesh.hard_edge_count(), 0);
        let uses = edge_use(&mesh);
        assert!(uses.values().all(|n| *n <= 2));
        let border: Vec<_> = uses.iter().filter(|(_, n)| **n == 1).collect();
        assert_eq!(border.len(), 24, "the rim is the only border");
        for (edge, _) in border {
            let (a, b) = edge.ends();
            assert_eq!(mesh.position(a).unwrap().z, 0.0);
            assert_eq!(mesh.position(b).unwrap().z, 0.0);
        }
    }

    #[test]
    fn a_dome_survives_the_fewest_pieces() {
        let mesh = dome_mesh(DomeParams {
            sides: 3,
            rings: 1,
            ..DomeParams::default()
        });
        // A three-sided pyramid: three triangles and the base.
        assert_closed_solid(&mesh, "fewest");
        assert_eq!(mesh.face_count(), 4);
        assert_eq!(mesh.vertex_count(), 4);
    }

    // Elbow ------------------------------------------------------------

    fn elbow_mesh(p: ElbowParams) -> Mesh {
        Primitive::Elbow(p).mesh()
    }

    #[test]
    fn a_solid_elbow_is_closed_with_flat_hard_ends() {
        let p = ElbowParams::default();
        let mesh = elbow_mesh(p);
        assert_closed_solid(&mesh, "elbow");
        assert_eq!(mesh.face_count(), 16 * 8 + 2);
        assert_eq!(mesh.vertex_count(), 16 * 9);
        let ends: Vec<_> = mesh.faces().filter(|(_, c)| c.len() == 16).collect();
        assert_eq!(ends.len(), 2, "each end one n-gon");
        assert_eq!(mesh.hard_edge_count(), 32, "the two rims");
        // Centred across the ground; the ends look back along the bend.
        let b = mesh.bounds().unwrap();
        assert!(b.centre().x.abs() < 1e-3 && b.centre().y.abs() < 1e-3);
        assert!((b.max.z - 30.0).abs() < 1e-3);
        let normals: Vec<Vec3> = ends.iter().map(|(f, _)| mesh.face_normal(*f)).collect();
        assert!((normals[0] + Vec3::Y).length() < 1e-4, "{}", normals[0]);
        assert!((normals[1] + Vec3::X).length() < 1e-4, "{}", normals[1]);
    }

    #[test]
    fn an_elbow_holds_its_true_volume_more_closely_with_more_sides() {
        let p = ElbowParams::default();
        let exact = PI * 15.0 * 15.0 * 50.0 * FRAC_PI_2;
        let coarse = volume_error(&elbow_mesh(p), exact);
        let fine = volume_error(
            &elbow_mesh(ElbowParams {
                sides: 128,
                segments: 64,
                ..p
            }),
            exact,
        );
        assert!(coarse < 0.05, "{coarse}");
        assert!(fine < 0.003, "{fine}");
        assert!(fine < coarse);
        // Hollow: the hole's volume taken away.
        let hollow = elbow_mesh(ElbowParams {
            inner_radius: 10.0,
            sides: 128,
            segments: 64,
            ..p
        });
        let exact = PI * (15.0f32.powi(2) - 10.0f32.powi(2)) * 50.0 * FRAC_PI_2;
        assert!(volume_error(&hollow, exact) < 0.005);
    }

    #[test]
    fn a_hollow_elbow_has_an_inner_wall_and_ring_shaped_ends() {
        let p = ElbowParams {
            inner_radius: 10.0,
            ..ElbowParams::default()
        };
        let mesh = elbow_mesh(p);
        assert_closed_solid(&mesh, "hollow elbow");
        assert_eq!(mesh.face_count(), 2 * 16 * 8 + 2 * 16);
        assert_eq!(mesh.vertex_count(), 2 * 16 * 9);
        assert!(mesh.faces().all(|(_, c)| c.len() == 4));
        assert_eq!(mesh.hard_edge_count(), 4 * 16, "outer and inner rims");
    }

    #[test]
    fn a_whole_turn_elbow_joins_up_without_ends() {
        let solid = elbow_mesh(ElbowParams {
            angle: 360.0,
            segments: 32,
            ..ElbowParams::default()
        });
        assert_closed_solid(&solid, "ring");
        assert_eq!(solid.face_count(), 16 * 32);
        assert_eq!(solid.vertex_count(), 16 * 32, "the last ring is the first");
        assert_eq!(solid.hard_edge_count(), 0);
        let exact = 2.0 * PI * PI * 50.0 * 15.0 * 15.0;
        assert!(volume_error(&solid, exact) < 0.04);
        let hollow = elbow_mesh(ElbowParams {
            angle: 360.0,
            inner_radius: 10.0,
            ..ElbowParams::default()
        });
        assert_closed_solid(&hollow, "hollow ring");
        assert_eq!(hollow.face_count(), 2 * 16 * 8);
        // Too few pieces to go round: three are used.
        let fewest = elbow_mesh(ElbowParams {
            angle: 360.0,
            segments: 1,
            sides: 3,
            ..ElbowParams::default()
        });
        assert_closed_solid(&fewest, "fewest ring");
        assert_eq!(fewest.face_count(), 3 * 3);
    }

    #[test]
    fn an_elbow_survives_extreme_settings() {
        let d = ElbowParams::default();
        let cases = [
            ElbowParams { angle: 0.01, ..d },
            ElbowParams {
                angle: 0.01,
                inner_radius: 10.0,
                ..d
            },
            ElbowParams {
                sides: 3,
                segments: 1,
                ..d
            },
            ElbowParams {
                sides: 3,
                segments: 1,
                inner_radius: 5.0,
                angle: 360.0,
                ..d
            },
            // The hole only just smaller than the pipe.
            ElbowParams {
                inner_radius: 14.95,
                ..d
            },
            // The pipe as fat as its bend.
            ElbowParams {
                pipe_radius: 50.0,
                angle: 270.0,
                ..d
            },
            ElbowParams {
                inner_radius: 1e9,
                angle: f32::NAN,
                ..d
            },
        ];
        for p in cases {
            let mesh = elbow_mesh(p);
            assert_closed_solid(&mesh, &format!("{p:?}"));
        }
        let sliver = elbow_mesh(cases[0]);
        let exact = PI * 15.0 * 15.0 * 50.0 * 0.01f32.to_radians();
        assert!(volume_error(&sliver, exact) < 0.05);
        let fewest = elbow_mesh(cases[2]);
        assert_eq!(fewest.face_count(), 3 + 2);
    }

    // All four ---------------------------------------------------------

    fn all_four() -> Vec<Primitive> {
        vec![
            Primitive::Torus(TorusParams::default()),
            Primitive::Capsule(CapsuleParams::default()),
            Primitive::Dome(DomeParams::default()),
            Primitive::Dome(DomeParams {
                base: false,
                ..DomeParams::default()
            }),
            Primitive::Elbow(ElbowParams::default()),
            Primitive::Elbow(ElbowParams {
                inner_radius: 8.0,
                angle: 135.0,
                sides: 7,
                ..ElbowParams::default()
            }),
        ]
    }

    #[test]
    fn the_same_settings_build_the_identical_mesh() {
        for shape in all_four() {
            let (a, b) = (shape.mesh(), shape.mesh());
            assert_eq!(a, b, "{shape:?}");
            let (Some(ua), Some(ub)) = (a.attribute_values(UV), b.attribute_values(UV)) else {
                panic!("{shape:?}: no UVs");
            };
            assert_eq!(ua, ub, "{shape:?}: UVs");
        }
    }

    #[test]
    fn no_uv_piece_is_mirrored() {
        for shape in all_four() {
            let mesh = shape.mesh();
            let Some((_, _, AttributeValues::Vec2(uvs))) = mesh.attribute_values(UV) else {
                panic!("{shape:?}: no UVs");
            };
            // Each UV face winds the same way as the face itself (no piece
            // is mirrored).
            let (starts, _) = mesh.face_layout();
            for f in 0..starts.len() - 1 {
                let (a, b) = (starts[f] as usize, starts[f + 1] as usize);
                let mut area = 0.0;
                for i in a..b {
                    let (p, q) = (uvs[i], uvs[if i + 1 == b { a } else { i + 1 }]);
                    area += p.x * q.y - q.x * p.y;
                }
                assert!(area > 0.0, "{shape:?}: face {f} mirrored or flat in UV");
            }
        }
    }

    #[test]
    fn the_estimate_covers_the_real_count() {
        for shape in all_four() {
            assert!(shape.polygon_estimate() >= shape.mesh().face_count());
        }
    }
}
