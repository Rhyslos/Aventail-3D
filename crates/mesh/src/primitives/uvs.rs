//! Ready-made UVs for the primitives ([decision 0095](../../../../docs/decisions/0095-uvs-before-advanced-modelling.md)):
//! every primitive comes laid out flat in the texture square, the way a
//! modeller would unwrap it by hand, with seams where it is cut open.
//!
//! - **Cube:** each side flat on its own, the six arranged side by side.
//! - **Plane and disc:** seen from above.
//! - **Cylinder, cone and tube:** the round side unrolled into a strip
//!   (cut open at the back, on the −X side), each end seen from its own
//!   side; a tube's inside is a strip of its own.
//! - **Sphere:** unrolled like a map of the world, cut open at the back.
//!
//! Every piece keeps its real size against the others (the same texture
//! detail everywhere), and they are packed into the square with a little
//! room between them. Seams are marked on the edges where the pieces were
//! cut apart.

use super::PrimitiveKind;
use crate::attributes::{AttributeValues, Domain, UV, Value};
use crate::uvs::seams_from_splits;
use crate::{FaceId, Mesh};
use glam::{Vec2, Vec3};
use std::f32::consts::PI;

/// A piece of the surface laid flat: which polygons, and each of their
/// corners' place on it (in the shape's own units).
#[derive(Debug, Default)]
struct Chart {
    corners: Vec<(usize, Vec2)>,
}

impl Chart {
    fn bounds(&self) -> (Vec2, Vec2) {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for (_, p) in &self.corners {
            min = min.min(*p);
            max = max.max(*p);
        }
        if min.is_finite() {
            (min, max)
        } else {
            (Vec2::ZERO, Vec2::ZERO)
        }
    }
}

/// How the surface is cut into flat pieces, by kind of shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    /// One piece per side of a box.
    Box,
    /// Seen from above.
    Flat,
    /// Ends seen from their side, the round side unrolled.
    Round,
    /// Unrolled like a map.
    Ball,
}

/// Gives `mesh`, a primitive of `kind`, its UVs and seams.
pub(super) fn add_uvs(mesh: &mut Mesh, kind: PrimitiveKind) {
    let style = match kind {
        PrimitiveKind::Cube => Style::Box,
        PrimitiveKind::Plane | PrimitiveKind::Disc => Style::Flat,
        PrimitiveKind::Cylinder | PrimitiveKind::Cone | PrimitiveKind::Tube => Style::Round,
        PrimitiveKind::Sphere => Style::Ball,
    };
    let charts = charts(mesh, style);
    let corner_count = mesh.face_layout().1.len();
    let mut uvs = vec![Vec2::ZERO; corner_count];
    for (index, uv) in layout(&charts) {
        if let Some(slot) = uvs.get_mut(index) {
            *slot = uv;
        }
    }
    if mesh
        .set_attribute_values(
            UV,
            Domain::Corner,
            Value::Vec2(Vec2::ZERO),
            AttributeValues::Vec2(uvs),
        )
        .is_ok()
    {
        seams_from_splits(mesh);
    }
}

/// The face's corners: their places in the corner list and positions.
fn corners_of(mesh: &Mesh, face: FaceId) -> Vec<(usize, Vec3)> {
    let (starts, corners) = mesh.face_layout();
    let (a, b) = (
        starts[face.get() as usize] as usize,
        starts[face.get() as usize + 1] as usize,
    );
    (a..b)
        .map(|i| (i, mesh.position(corners[i]).unwrap_or(Vec3::ZERO)))
        .collect()
}

/// `a`, a whole turn more or less if that brings it within half a turn of
/// `near`: so a polygon across the cut keeps its corners together.
fn unwrap(a: f32, near: f32) -> f32 {
    if a - near > PI {
        a - 2.0 * PI
    } else if near - a > PI {
        a + 2.0 * PI
    } else {
        a
    }
}

/// The angle of `p` around the up axis, measured from the back (−X), so
/// the round side's cut is at the back.
fn around(p: Vec3) -> f32 {
    // atan2 is ±π at −X: turning by π puts the cut there.
    let a = p.y.atan2(p.x) + PI;
    if a >= 2.0 * PI { a - 2.0 * PI } else { a }
}

/// Cuts the surface into flat pieces and lays each corner on its piece.
fn charts(mesh: &Mesh, style: Style) -> Vec<Chart> {
    let mut charts: Vec<Chart> = Vec::new();
    let chart = |id: usize, charts: &mut Vec<Chart>| -> usize {
        if charts.len() <= id {
            charts.resize_with(id + 1, Chart::default);
        }
        id
    };
    // The round side's largest radius, inside and out, so each strip is as
    // long as its widest ring.
    let (mut outer, mut inner) = (0.0f32, f32::INFINITY);
    let ball_centre = mesh.bounds().map_or(Vec3::ZERO, |b| b.centre());
    let cone = if style == Style::Round {
        Cone::of(mesh)
    } else {
        None
    };
    for (face, _) in mesh.faces() {
        let corners = corners_of(mesh, face);
        let n = mesh.face_normal(face);
        let centre = corners.iter().map(|(_, p)| *p).sum::<Vec3>() / corners.len().max(1) as f32;
        match style {
            Style::Box => {
                let a = n.abs();
                let (id, flat): (usize, fn(Vec3) -> Vec2) = if a.x >= a.y && a.x >= a.z {
                    if n.x > 0.0 {
                        (0, |p| Vec2::new(p.y, p.z))
                    } else {
                        (1, |p| Vec2::new(-p.y, p.z))
                    }
                } else if a.y >= a.z {
                    if n.y > 0.0 {
                        (2, |p| Vec2::new(-p.x, p.z))
                    } else {
                        (3, |p| Vec2::new(p.x, p.z))
                    }
                } else if n.z > 0.0 {
                    (4, |p| Vec2::new(p.x, p.y))
                } else {
                    (5, |p| Vec2::new(-p.x, p.y))
                };
                let id = chart(id, &mut charts);
                for (i, p) in corners {
                    charts[id].corners.push((i, flat(p)));
                }
            }
            Style::Flat => {
                let id = chart(0, &mut charts);
                for (i, p) in corners {
                    charts[id].corners.push((i, Vec2::new(p.x, p.y)));
                }
            }
            Style::Round if n.z.abs() > 0.999 => {
                let (id, flat): (usize, fn(Vec3) -> Vec2) = if n.z > 0.0 {
                    (0, |p| Vec2::new(p.x, p.y))
                } else {
                    (1, |p| Vec2::new(-p.x, p.y))
                };
                let id = chart(id, &mut charts);
                for (i, p) in corners {
                    charts[id].corners.push((i, flat(p)));
                }
            }
            Style::Round => {
                let radial = Vec3::new(centre.x, centre.y, 0.0);
                let outside = n.dot(radial) >= 0.0;
                for (_, p) in &corners {
                    let r = p.truncate().length();
                    if outside {
                        outer = outer.max(r);
                    } else {
                        inner = inner.min(r);
                    }
                }
                // Along a slanted side (a cone), its real length.
                let slant = 1.0 / (1.0 - n.z * n.z).max(1e-6).sqrt();
                let middle = around(centre);
                let id = chart(if outside { 2 } else { 3 }, &mut charts);
                if outside && let Some(cone) = cone {
                    for (i, p) in corners {
                        let a = if p.truncate().length() < 1e-6 {
                            middle
                        } else {
                            unwrap(around(p), middle)
                        };
                        charts[id].corners.push((i, cone.unrolled(p, a)));
                    }
                    continue;
                }
                for (i, p) in corners {
                    let a = if p.truncate().length() < 1e-6 {
                        middle
                    } else {
                        unwrap(around(p), middle)
                    };
                    // Seen from inside, the inner wall runs the other way.
                    let u = if outside { a } else { -a };
                    charts[id].corners.push((i, Vec2::new(u, p.z * slant)));
                }
            }
            Style::Ball => {
                let middle = around(centre - ball_centre);
                let id = chart(0, &mut charts);
                for (i, p) in corners {
                    let d = p - ball_centre;
                    let r = d.truncate().length();
                    let a = if r < 1e-6 {
                        middle
                    } else {
                        unwrap(around(d), middle)
                    };
                    let latitude = d.z.atan2(r);
                    let radius = d.length();
                    outer = outer.max(radius);
                    charts[id].corners.push((i, Vec2::new(a, latitude)));
                }
            }
        }
    }
    // Angles to lengths: each strip as long as its widest ring.
    let scale_u = |chart: &mut Chart, by: f32, v_by: f32| {
        for (_, p) in &mut chart.corners {
            p.x *= by;
            p.y *= v_by;
        }
    };
    match style {
        Style::Round => {
            // A cone's side is unrolled at its real size already.
            if cone.is_none()
                && let Some(c) = charts.get_mut(2)
            {
                scale_u(c, outer, 1.0);
            }
            if let Some(c) = charts.get_mut(3) {
                scale_u(c, if inner.is_finite() { inner } else { outer }, 1.0);
            }
        }
        Style::Ball => {
            if let Some(c) = charts.get_mut(0) {
                scale_u(c, outer, outer);
            }
        }
        _ => {}
    }
    charts.retain(|c| !c.corners.is_empty());
    charts
}

/// A tapering round side (a cone or a cut-off cone): unrolled flat, it is
/// a slice of a ring around the point where the side would meet the axis
/// (its apex), so it lies flat without stretching.
#[derive(Debug, Clone, Copy)]
struct Cone {
    /// The height of the apex.
    apex: f32,
    /// How much of a full circle the unrolled side covers per turn: the
    /// sine of the side's angle from the axis.
    spread: f32,
}

impl Cone {
    /// The cone of the mesh's round outer side, if it tapers.
    fn of(mesh: &Mesh) -> Option<Self> {
        let (mut low, mut high) = ((f32::INFINITY, 0.0f32), (f32::NEG_INFINITY, 0.0f32));
        for (face, corners) in mesh.faces() {
            if mesh.face_normal(face).z.abs() > 0.999 {
                continue;
            }
            for v in corners {
                let p = mesh.position(*v)?;
                let r = p.truncate().length();
                if p.z < low.0 - 1e-4 || ((p.z - low.0).abs() <= 1e-4 && r > low.1) {
                    low = (p.z, r);
                }
                if p.z > high.0 + 1e-4 || ((p.z - high.0).abs() <= 1e-4 && r > high.1) {
                    high = (p.z, r);
                }
            }
        }
        let (dz, dr) = (high.0 - low.0, low.1 - high.1);
        if !dz.is_finite() || dz < 1e-6 || dr.abs() < 1e-3 * low.1.max(high.1).max(1e-6) {
            return None;
        }
        Some(Self {
            apex: low.0 + low.1 * dz / dr,
            spread: dr.abs() / (dr * dr + dz * dz).sqrt(),
        })
    }

    /// Where point `p`, `angle` around from the back, lies on the
    /// unrolled side: the apex at the middle, the side hanging away from
    /// it.
    fn unrolled(self, p: Vec3, angle: f32) -> Vec2 {
        let r = p.truncate().length();
        let distance = (r * r + (self.apex - p.z).powi(2)).sqrt();
        let turn = (angle - PI) * self.spread;
        // Apex above (a cone standing on its base): the side hangs down.
        let down = if self.apex > p.z { -1.0 } else { 1.0 };
        Vec2::new(distance * turn.sin(), down * distance * turn.cos())
    }
}

/// Packs the pieces into the texture square at one scale (so texture
/// detail is the same everywhere), tallest first in rows, with room
/// between them. Returns each corner's UV.
fn layout(charts: &[Chart]) -> Vec<(usize, Vec2)> {
    let sized: Vec<(usize, Vec2, Vec2)> = charts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (min, max) = c.bounds();
            (i, min, (max - min).max(Vec2::splat(1e-6)))
        })
        .collect();
    let area: f32 = sized.iter().map(|(_, _, s)| s.x * s.y).sum();
    let pad = 0.04 * area.sqrt().max(1e-6);
    let widest = sized.iter().map(|(_, _, s)| s.x).fold(0.0, f32::max);
    let row_limit = (area.sqrt() * 1.25).max(widest) + pad;
    let mut order: Vec<usize> = (0..sized.len()).collect();
    order.sort_by(|a, b| sized[*b].2.y.total_cmp(&sized[*a].2.y).then(a.cmp(b)));
    // Each piece's bottom-left corner, before scaling.
    let mut placed = vec![Vec2::ZERO; sized.len()];
    let (mut x, mut y, mut row_height, mut width) = (pad, pad, 0.0f32, 0.0f32);
    for i in order {
        let size = sized[i].2;
        if x > pad && x + size.x + pad > row_limit {
            x = pad;
            y += row_height + pad;
            row_height = 0.0;
        }
        placed[i] = Vec2::new(x, y);
        x += size.x + pad;
        width = width.max(x);
        row_height = row_height.max(size.y);
    }
    let height = y + row_height + pad;
    let scale = 1.0 / width.max(height).max(1e-6);
    // Centred in the square.
    let offset = (Vec2::ONE - Vec2::new(width, height) * scale) / 2.0;
    let mut out = Vec::new();
    for (i, chart) in charts.iter().enumerate() {
        let (min, _) = (sized[i].1, sized[i].2);
        for (corner, p) in &chart.corners {
            out.push((*corner, (*p - min + placed[i]) * scale + offset));
        }
    }
    out
}
