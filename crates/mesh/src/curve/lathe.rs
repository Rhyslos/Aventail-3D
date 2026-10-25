//! Lathes: a curve spun round an axis, making a vase, a bottle, a
//! column or a wheel.
//!
//! The curve is spun round one of its own axes (X, Y or Z, through its
//! centre, the object's pivot), all the way round or through part of a
//! turn. Points on the axis become one point (a pole), so the surface
//! closes there without a hole. With caps, the open ends are closed with
//! flat faces: the ends of an open curve that stop short of the axis get
//! a disc (or a pie slice), and a part turn gets a flat face at each side.
//!
//! The surface is one UV piece (U round the axis at the widest radius, V
//! along the curve, both at their real size), each cap its own, packed
//! into the texture square as every primitive's are.

use super::Curve;
use super::grid::{Grid, cap, join};
use super::path::{Detail, samples};
use crate::Mesh;
use crate::hardness::DEFAULT_ANGLE;
use crate::polygon::VertexId;
use crate::primitives::uvs::{Chart, set_charts};
use glam::{Quat, Vec3};

/// The fewest and most pieces round the axis.
pub const SEGMENTS: std::ops::RangeInclusive<u32> = 3..=256;

/// Which of the curve's own axes it spins round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LatheAxis {
    /// Its X.
    X,
    /// Its Y.
    Y,
    /// Its Z (up, the standard).
    #[default]
    Z,
}

impl LatheAxis {
    /// Every axis, in order.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// The axis as a direction.
    pub fn vector(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }
}

/// How a lathe spins its curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatheShape {
    /// The axis.
    pub axis: LatheAxis,
    /// How far round, in degrees: 1 to 360.
    pub angle: f32,
    /// How many pieces round ([`SEGMENTS`]).
    pub segments: u32,
    /// How finely it follows the curve.
    pub detail: Detail,
    /// Whether open ends are closed with flat faces.
    pub caps: bool,
}

impl Default for LatheShape {
    fn default() -> Self {
        Self {
            axis: LatheAxis::Z,
            angle: 360.0,
            segments: 24,
            detail: Detail::default(),
            caps: true,
        }
    }
}

impl LatheShape {
    /// The same settings within limits.
    pub fn clamped(self) -> Self {
        Self {
            angle: if self.angle.is_finite() {
                self.angle.clamp(1.0, 360.0)
            } else {
                360.0
            },
            segments: self.segments.clamp(*SEGMENTS.start(), *SEGMENTS.end()),
            detail: self.detail.clamped(),
            ..self
        }
    }
}

/// One place on the curve being spun: where it is, how high up the axis,
/// how far out from it, and whether the curve turns sharply there.
struct Place {
    position: Vec3,
    height: f32,
    radius: f32,
    sharp: bool,
}

/// The solid made by spinning `curve` round an axis, in the curve's own
/// space. An empty mesh for a curve with less than a line.
pub fn lathe(curve: &Curve, shape: &LatheShape) -> Mesh {
    let shape = shape.clamped();
    let axis = shape.axis.vector();
    let mut detail = shape.detail;
    let mut found = samples(curve, detail, 0.0);
    while found.samples.len() * shape.segments as usize > super::sweep::MAX_POLYGONS
        && detail.segments > 1
    {
        detail.segments = (detail.segments / 2).max(1);
        found = samples(curve, detail, 0.0);
    }
    let mut mesh = Mesh::new();
    let reach = curve
        .bounds()
        .map_or(1.0, |b| b.size().max_element())
        .max(1.0);
    let on_axis = 1e-5 * reach;
    let mut places: Vec<Place> = Vec::new();
    for s in &found.samples {
        if places
            .last()
            .is_some_and(|l| l.position.distance(s.position) < on_axis)
        {
            continue;
        }
        let height = s.position.dot(axis);
        places.push(Place {
            position: s.position,
            height,
            radius: (s.position - axis * height).length(),
            sharp: s
                .corner
                .is_some_and(|c| c.angle() > DEFAULT_ANGLE.to_radians()),
        });
    }
    let closed = found.closed && places.len() >= 3;
    if closed
        && places.len() > 1
        && places[0]
            .position
            .distance(places[places.len() - 1].position)
            < on_axis
    {
        places.pop();
    }
    if places.len() < 2 || places.iter().all(|p| p.radius < on_axis) {
        return mesh;
    }
    // An open curve whose ends stand off the axis at the same height (an
    // arch on the ground): capped, it closes straight across between its
    // ends, as going by the axis would fold flat on itself.
    let (first, last) = (&places[0], &places[places.len() - 1]);
    let closed = closed
        || (shape.caps
            && places.len() >= 3
            && first.radius >= on_axis
            && last.radius >= on_axis
            && (first.height - last.height).abs() < on_axis);
    let full = shape.angle >= 360.0 - 1e-3;
    let turn = shape.angle.to_radians();
    let columns = shape.segments as usize;
    let ring = if full { columns } else { columns + 1 };
    let rows: Vec<Vec<VertexId>> = places
        .iter()
        .map(|p| {
            if p.radius < on_axis {
                return vec![mesh.push_vertex(axis * p.height)];
            }
            (0..ring)
                .map(|k| {
                    let angle = turn * k as f32 / columns as f32;
                    mesh.push_vertex(Quat::from_axis_angle(axis, angle) * p.position)
                })
                .collect()
        })
        .collect();
    let widest = places.iter().map(|p| p.radius).fold(0.0, f32::max);
    let u: Vec<f32> = (0..=columns)
        .map(|k| turn * k as f32 / columns as f32 * widest)
        .collect();
    let mut v = vec![0.0];
    for j in 1..places.len() {
        v.push(v[j - 1] + places[j].position.distance(places[j - 1].position));
    }
    if closed {
        let last = v[v.len() - 1];
        v.push(
            last + places[0]
                .position
                .distance(places[places.len() - 1].position),
        );
    }
    let (surface, _) = join(
        &mut mesh,
        &Grid {
            rows: &rows,
            columns,
            wrap_rows: closed,
            u: &u,
            v: &v,
            flip: faces_inward(&places, closed),
        },
    );
    let mut charts: Vec<Chart> = vec![surface];
    for (row, place) in rows.iter().zip(&places) {
        if place.sharp && row.len() > 1 {
            for k in 0..columns {
                mesh.set_hard(row[k], row[(k + 1) % row.len()], true);
            }
        }
    }
    if shape.caps {
        caps(&mut mesh, &rows, &places, closed, axis, full, &mut charts);
    }
    set_charts(&mut mesh, &charts);
    mesh
}

/// Whether the surface, built with the turn's way round and the curve's
/// way along, would face in towards the axis: then its faces are turned.
fn faces_inward(places: &[Place], closed: bool) -> bool {
    let n = places.len();
    let pairs = if closed { n } else { n - 1 };
    let pair = |j: usize| (&places[j], &places[(j + 1) % n]);
    let rise: f32 = (0..pairs)
        .map(|j| {
            let (a, b) = pair(j);
            (b.height - a.height) * (a.radius + b.radius)
        })
        .sum();
    let scale: f32 = places
        .iter()
        .map(|p| p.radius.abs() + p.height.abs())
        .sum::<f32>();
    if rise.abs() > 1e-6 * scale * scale.max(1.0) / n as f32 {
        return rise < 0.0;
    }
    // Flat, like a disc spun from a line along the ground: faces up the
    // axis when the curve goes outward.
    let outward: f32 = (0..pairs)
        .map(|j| {
            let (a, b) = pair(j);
            b.radius - a.radius
        })
        .sum();
    outward > 0.0
}

/// Closes the lathe's open ends: a disc (or pie slice) where an open
/// curve's end stops short of the axis, and for a part turn a flat face
/// at each side.
fn caps(
    mesh: &mut Mesh,
    rows: &[Vec<VertexId>],
    places: &[Place],
    closed: bool,
    axis: Vec3,
    full: bool,
    charts: &mut Vec<Chart>,
) {
    let last = places.len() - 1;
    let middle = places.iter().map(|p| p.height).sum::<f32>() / places.len() as f32;
    // Where an open end meets the axis, shared by the faces that need it.
    let axis_point = |j: usize, mesh: &mut Mesh| -> Option<VertexId> {
        (rows[j].len() > 1).then(|| mesh.push_vertex(axis * places[j].height))
    };
    let (start_axis, end_axis) = if closed {
        (None, None)
    } else {
        (axis_point(0, mesh), axis_point(last, mesh))
    };
    if !closed {
        for (j, centre) in [(0, start_axis), (last, end_axis)] {
            let Some(centre) = centre else {
                continue;
            };
            let mut ring: Vec<VertexId> = Vec::new();
            if !full {
                ring.push(centre);
            }
            ring.extend(rows[j].iter().copied());
            let away = places[j].height - middle;
            let outward = if away.abs() > 1e-6 {
                axis * away.signum()
            } else if j == 0 {
                -axis
            } else {
                axis
            };
            if let Some((chart, _)) = cap(mesh, &ring, outward) {
                charts.push(chart);
            }
        }
    }
    if full {
        return;
    }
    // The flat sides of a part turn.
    let columns = rows.iter().map(Vec::len).max().unwrap_or(1) - 1;
    for (column, side) in [(0, -1.0), (columns, 1.0)] {
        let mut ring: Vec<VertexId> = Vec::new();
        ring.extend(end_axis.filter(|_| !closed));
        ring.extend(rows.iter().rev().map(|row| row[column.min(row.len() - 1)]));
        ring.extend(start_axis.filter(|_| !closed));
        let points: Vec<Vec3> = ring.iter().filter_map(|v| mesh.position(*v)).collect();
        let centre = points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32;
        let out = centre - axis * centre.dot(axis);
        // At the start of the turn the solid lies ahead (the turn's way);
        // at its end, behind.
        let way = axis.cross(out);
        if let Some((chart, _)) = cap(mesh, &ring, way * side) {
            charts.push(chart);
        }
    }
}
