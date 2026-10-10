//! The round primitives: cylinders and cones (both built as a
//! frustum), the sphere, the disc and the tube.
//!
//! Points around a circle start on the +X axis and go counter-clockwise
//! seen from above. Round sides are soft (they shade as a smooth curve);
//! the rims where a side meets a flat end are hard. Flat ends are, as
//! standard, single faces with many corners (n-gons), which keeps them
//! clean for modelling; they can also be cut into rings, and meet in a
//! centre point instead.

use super::{DiscParams, SphereParams, TubeParams};
use crate::Mesh;
use crate::polygon::VertexId;
use glam::Vec3;
use std::f32::consts::{PI, TAU};

/// A ring of `sides` points at `radius` and `height`, or a single point
/// for a zero radius.
fn ring(mesh: &mut Mesh, radius: f32, height: f32, sides: u32) -> Vec<VertexId> {
    if radius <= 0.0 {
        return vec![mesh.push_vertex(Vec3::new(0.0, 0.0, height))];
    }
    (0..sides)
        .map(|i| {
            let angle = TAU * i as f32 / sides as f32;
            mesh.push_vertex(Vec3::new(
                radius * angle.cos(),
                radius * angle.sin(),
                height,
            ))
        })
        .collect()
}

/// The point `i` of a ring (a single-point ring answers every `i`).
fn at(ring: &[VertexId], i: usize) -> VertexId {
    ring[i % ring.len()]
}

/// Faces joining `lower` to `upper`, facing out (or in, for `inward`).
/// Where a ring is a single point, the faces are triangles.
fn band(mesh: &mut Mesh, lower: &[VertexId], upper: &[VertexId], sides: u32, inward: bool) {
    for i in 0..sides as usize {
        let mut face = vec![
            at(lower, i),
            at(lower, i + 1),
            at(upper, i + 1),
            at(upper, i),
        ];
        face.dedup();
        if face.first() == face.last() && face.len() > 1 {
            face.pop();
        }
        if inward {
            face.reverse();
        }
        mesh.push_face(&face);
    }
}

/// Marks a ring's edges hard.
fn harden(mesh: &mut Mesh, ring: &[VertexId]) {
    if ring.len() < 2 {
        return;
    }
    for i in 0..ring.len() {
        mesh.set_hard(ring[i], at(ring, i + 1), true);
    }
}

/// A flat round end across `rim` (a ring at `height` with `radius`),
/// facing up (or down): cut into `segments` rings from the rim inwards,
/// with either a point in the middle (triangles around it) or a flat
/// n-gon as the innermost piece.
fn round_end(
    mesh: &mut Mesh,
    rim: &[VertexId],
    shape: (f32, f32, u32),
    segments: u32,
    centre_point: bool,
    down: bool,
) {
    let (radius, height, sides) = shape;
    let mut outer = rim.to_vec();
    // Rings between the rim and the middle: none for one segment and an
    // n-gon; with a centre point, the last "ring" is the point.
    let inner_rings = if centre_point { segments } else { segments - 1 };
    for k in 1..=inner_rings {
        let r = radius * (segments - k) as f32 / segments as f32;
        let inner = ring(mesh, r, height, sides);
        flat_ring(mesh, &inner, &outer, sides, down);
        outer = inner;
    }
    if !centre_point {
        let mut face = outer;
        if down {
            face.reverse();
        }
        mesh.push_face(&face);
    }
}

/// Settings shared by the cylinder and the cone.
pub(super) struct Frustum {
    pub(super) bottom_radius: f32,
    pub(super) top_radius: f32,
    pub(super) height: f32,
    pub(super) sides: u32,
    pub(super) height_segments: u32,
    pub(super) caps: bool,
    pub(super) cap_segments: u32,
    pub(super) cap_centre: bool,
}

/// An upright, possibly tapering, round shape: a cylinder when both radii
/// match, a cone when one is zero.
pub(super) fn frustum(p: &Frustum) -> Mesh {
    let mut mesh = Mesh::new();
    let rings: Vec<Vec<VertexId>> = (0..=p.height_segments)
        .map(|k| {
            let t = k as f32 / p.height_segments as f32;
            let radius = p.bottom_radius + (p.top_radius - p.bottom_radius) * t;
            ring(&mut mesh, radius, p.height * t, p.sides)
        })
        .collect();
    for pair in rings.windows(2) {
        band(&mut mesh, &pair[0], &pair[1], p.sides, false);
    }
    if p.caps {
        let ends = [
            (rings.first(), p.bottom_radius, 0.0, true),
            (rings.last(), p.top_radius, p.height, false),
        ];
        for (ring, radius, height, down) in ends {
            if let Some(ring) = ring.filter(|r| r.len() > 1) {
                let shape = (radius, height, p.sides);
                round_end(&mut mesh, ring, shape, p.cap_segments, p.cap_centre, down);
                harden(&mut mesh, ring);
            }
        }
    }
    mesh
}

/// A ball resting on the ground: a point at each pole and rings between.
pub(super) fn sphere(p: &SphereParams) -> Mesh {
    let mut mesh = Mesh::new();
    // From the top pole (ring 0) to the bottom pole (ring `rings`).
    let rings: Vec<Vec<VertexId>> = (0..=p.rings)
        .map(|k| {
            let polar = PI * k as f32 / p.rings as f32;
            let radius = if k == 0 || k == p.rings {
                0.0
            } else {
                p.radius * polar.sin()
            };
            ring(
                &mut mesh,
                radius,
                p.radius + p.radius * polar.cos(),
                p.sides,
            )
        })
        .collect();
    for pair in rings.windows(2) {
        // Built from the lower ring up, so faces look outward.
        band(&mut mesh, &pair[1], &pair[0], p.sides, false);
    }
    mesh
}

/// A flat circle (or ring, with a hole) on the ground, facing up.
pub(super) fn disc(p: &DiscParams) -> Mesh {
    let mut mesh = Mesh::new();
    let outer = ring(&mut mesh, p.radius, 0.0, p.sides);
    if p.inner_radius <= 0.0 {
        let shape = (p.radius, 0.0, p.sides);
        round_end(
            &mut mesh,
            &outer,
            shape,
            p.ring_segments,
            p.centre_point,
            false,
        );
        return mesh;
    }
    // A ring-shaped disc: rings from the rim in to the hole.
    let mut previous = outer;
    for k in 1..=p.ring_segments {
        let t = k as f32 / p.ring_segments as f32;
        let r = p.radius + (p.inner_radius - p.radius) * t;
        let inner = ring(&mut mesh, r, 0.0, p.sides);
        flat_ring(&mut mesh, &inner, &previous, p.sides, false);
        previous = inner;
    }
    mesh
}

/// Faces between an inner and an outer ring at the same height, facing up
/// (or down). An inner ring of one point gives triangles.
fn flat_ring(mesh: &mut Mesh, inner: &[VertexId], outer: &[VertexId], sides: u32, down: bool) {
    for i in 0..sides as usize {
        let mut face = vec![
            at(inner, i),
            at(outer, i),
            at(outer, i + 1),
            at(inner, i + 1),
        ];
        // A single-point inner ring (a centre point) makes triangles.
        if face[0] == face[3] {
            face.pop();
        }
        if down {
            face.reverse();
        }
        mesh.push_face(&face);
    }
}

/// An upright pipe with flat ring-shaped ends.
pub(super) fn tube(p: &TubeParams) -> Mesh {
    let mut mesh = Mesh::new();
    let levels = p.height_segments;
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    for k in 0..=levels {
        let z = p.height * k as f32 / levels as f32;
        outer.push(ring(&mut mesh, p.outer_radius, z, p.sides));
        inner.push(ring(&mut mesh, p.inner_radius, z, p.sides));
    }
    for k in 0..levels as usize {
        band(&mut mesh, &outer[k], &outer[k + 1], p.sides, false);
        band(&mut mesh, &inner[k], &inner[k + 1], p.sides, true);
    }
    let (bottom, top) = (0, levels as usize);
    flat_ring(&mut mesh, &inner[bottom], &outer[bottom], p.sides, true);
    flat_ring(&mut mesh, &inner[top], &outer[top], p.sides, false);
    for ring in [&outer[bottom], &outer[top], &inner[bottom], &inner[top]] {
        harden(&mut mesh, ring);
    }
    mesh
}
