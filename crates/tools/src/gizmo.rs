//! The move, rotate and scale gizmo: its handles on screen, which one is
//! under the mouse, and what dragging one does
//! ([decision 0030](../../../docs/decisions/0030-transform-gizmos.md)).
//!
//! The gizmo sits at a point with three axes (the world's, or an
//! object's). It is drawn a fixed size on screen, whatever the distance:
//!
//! - **Move:** an arrow per axis (move along it), a square between each
//!   pair (move in that plane), and the centre (move facing you).
//! - **Rotate:** a ring around each axis, and an outer ring that turns
//!   around the line of sight.
//! - **Scale:** a line with a box per axis, a square between each pair
//!   (scale those two axes, in the ratio the mouse moves along them), and
//!   the centre (all axes).
//! - **All:** arrows, plane squares, rings, scale boxes part way along the
//!   axes, and the centre (move).
//!
//! This module only does the arithmetic: the interface draws the shapes
//! it describes and turns a [`Change`] into commands. Nothing here knows
//! about the scene.

use crate::camera::{Camera, Ray};
use glam::{Vec2, Vec3};
use std::f32::consts::TAU;

/// How long the arrows are on screen, in points.
pub const GIZMO_SIZE: f32 = 90.0;
/// How close (in screen points) the mouse must be to grab a handle.
pub const GRAB_DISTANCE: f32 = 8.0;
/// How much slower a drag goes with the fine key held.
pub const FINE_FACTOR: f32 = 0.1;

/// Where a plane square starts and ends along its two axes, as parts of
/// the arrow length.
const PLANE_FROM: f32 = 0.22;
const PLANE_TO: f32 = 0.42;
/// The rotate rings' radius, as a part of the arrow length.
const RING: f32 = 0.85;
/// The outer, line-of-sight ring, as a part of the arrow length.
const VIEW_RING: f32 = 1.08;
/// Where the scale boxes sit along the axes, as a part of the arrow
/// length: at the end with the Scale tool, part way with All.
const SCALE_END: f32 = 0.85;
const SCALE_ALL_TOOLS: f32 = 0.55;
/// Half the size of a scale box and the centre, in screen points.
const BOX: f32 = 5.0;
const CENTRE: f32 = 7.0;
/// How many segments a ring is drawn with.
const RING_SEGMENTS: usize = 64;

/// Which gizmo is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoTool {
    /// Move (`W`).
    Move,
    /// Rotate (`R`).
    Rotate,
    /// Scale (`S`).
    Scale,
    /// All three together (`T`).
    All,
}

/// A part of the gizmo that can be grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Move along an axis (0 X, 1 Y, 2 Z).
    Axis(usize),
    /// Move in the plane across an axis (the plane square between the
    /// other two).
    Plane(usize),
    /// Move freely, facing the viewer.
    Centre,
    /// Turn around an axis.
    Ring(usize),
    /// Turn around the line of sight.
    ViewRing,
    /// Scale along an axis.
    ScaleAxis(usize),
    /// Scale along the two axes across an axis (the plane square between
    /// them, on the scale gizmo). Which way the mouse goes decides how
    /// much each of the two scales.
    ScalePlane(usize),
    /// Scale along every axis at once.
    ScaleAll,
}

impl Handle {
    /// The axis the handle belongs to, for its colour; `None` for the
    /// centre and the outer ring.
    pub fn axis(self) -> Option<usize> {
        match self {
            Self::Axis(i)
            | Self::Plane(i)
            | Self::Ring(i)
            | Self::ScaleAxis(i)
            | Self::ScalePlane(i) => Some(i),
            Self::Centre | Self::ViewRing | Self::ScaleAll => None,
        }
    }
}

/// Something to draw for a handle, in screen points.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A line, with an arrowhead at `b` if `arrow`.
    Line {
        /// Where it starts.
        a: Vec2,
        /// Where it ends.
        b: Vec2,
        /// Whether it ends in an arrowhead.
        arrow: bool,
    },
    /// A closed outline (a ring).
    Ring(Vec<Vec2>),
    /// A filled four-sided shape (a plane square or a box).
    Quad([Vec2; 4]),
    /// A filled dot.
    Dot {
        /// Its middle.
        centre: Vec2,
        /// Its radius.
        radius: f32,
    },
}

impl Shape {
    /// How far the screen point is from the shape (0 inside a filled
    /// shape).
    pub fn distance(&self, p: Vec2) -> f32 {
        match self {
            Self::Line { a, b, .. } => segment_distance(p, *a, *b),
            Self::Ring(points) => {
                let n = points.len();
                (0..n)
                    .map(|i| segment_distance(p, points[i], points[(i + 1) % n]))
                    .fold(f32::INFINITY, f32::min)
            }
            Self::Quad(corners) => {
                if inside_quad(corners, p) {
                    0.0
                } else {
                    (0..4)
                        .map(|i| segment_distance(p, corners[i], corners[(i + 1) % 4]))
                        .fold(f32::INFINITY, f32::min)
                }
            }
            Self::Dot { centre, radius } => (p.distance(*centre) - radius).max(0.0),
        }
    }
}

/// The gizmo: where it is, which way its axes point, and which tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gizmo {
    /// Where it sits, in the world.
    pub origin: Vec3,
    /// Its axes (X, Y, Z), unit length, at right angles to each other.
    pub axes: [Vec3; 3],
    /// Which tool it is for.
    pub tool: GizmoTool,
}

impl Gizmo {
    /// The arrow length in the world, for a view of `size` seen through
    /// `camera`.
    pub fn length(&self, camera: &Camera, size: Vec2) -> f32 {
        GIZMO_SIZE * camera.world_per_pixel(self.origin, size)
    }

    /// The handles this tool has, with what to draw for each. Handles of
    /// `locked` axes are left out (they can't be used), as are arrows that
    /// point almost straight at the viewer (too short to grab).
    pub fn shapes(&self, camera: &Camera, size: Vec2, locked: [bool; 3]) -> Vec<(Handle, Shape)> {
        let Some(centre) = camera.project(self.origin, size) else {
            return Vec::new();
        };
        let length = self.length(camera, size);
        let at = |v: Vec3| camera.project(v, size);
        let mut out = Vec::new();
        let tool = self.tool;
        let moves = matches!(tool, GizmoTool::Move | GizmoTool::All);
        let rotates = matches!(tool, GizmoTool::Rotate | GizmoTool::All);
        let scales = matches!(tool, GizmoTool::Scale | GizmoTool::All);
        let free = locked.iter().filter(|l| !**l).count();
        if rotates && free > 0 {
            let radius = GIZMO_SIZE * VIEW_RING;
            let ring = (0..RING_SEGMENTS)
                .map(|i| {
                    let a = i as f32 / RING_SEGMENTS as f32 * TAU;
                    centre + Vec2::new(a.cos(), a.sin()) * radius
                })
                .collect();
            out.push((Handle::ViewRing, Shape::Ring(ring)));
        }
        for (i, axis) in self.axes.into_iter().enumerate() {
            if locked[i] {
                continue;
            }
            let Some(tip) = at(self.origin + axis * length) else {
                continue;
            };
            let seen = tip.distance(centre) > GIZMO_SIZE * 0.15;
            if moves && seen {
                out.push((
                    Handle::Axis(i),
                    Shape::Line {
                        a: centre,
                        b: tip,
                        arrow: true,
                    },
                ));
            }
            if rotates {
                let (u, v) = (self.axes[(i + 1) % 3], self.axes[(i + 2) % 3]);
                let ring: Option<Vec<Vec2>> = (0..RING_SEGMENTS)
                    .map(|k| {
                        let a = k as f32 / RING_SEGMENTS as f32 * TAU;
                        at(self.origin + (u * a.cos() + v * a.sin()) * length * RING)
                    })
                    .collect();
                if let Some(ring) = ring {
                    out.push((Handle::Ring(i), Shape::Ring(ring)));
                }
            }
            if scales && seen {
                let along = if tool == GizmoTool::Scale {
                    SCALE_END
                } else {
                    SCALE_ALL_TOOLS
                };
                if let Some(end) = at(self.origin + axis * length * along) {
                    if tool == GizmoTool::Scale {
                        out.push((
                            Handle::ScaleAxis(i),
                            Shape::Line {
                                a: centre,
                                b: end,
                                arrow: false,
                            },
                        ));
                    }
                    out.push((Handle::ScaleAxis(i), Shape::Quad(square(end, BOX))));
                }
            }
        }
        if moves || tool == GizmoTool::Scale {
            let make = if moves {
                Handle::Plane
            } else {
                Handle::ScalePlane
            };
            for i in 0..3 {
                let (a, b) = ((i + 1) % 3, (i + 2) % 3);
                if locked[a] || locked[b] {
                    continue;
                }
                let corner = |s: f32, t: f32| {
                    at(self.origin + (self.axes[a] * s + self.axes[b] * t) * length)
                };
                let corners = [
                    corner(PLANE_FROM, PLANE_FROM),
                    corner(PLANE_TO, PLANE_FROM),
                    corner(PLANE_TO, PLANE_TO),
                    corner(PLANE_FROM, PLANE_TO),
                ];
                if let [Some(p), Some(q), Some(r), Some(s)] = corners
                    && quad_area(&[p, q, r, s]) > 30.0
                {
                    out.push((make(i), Shape::Quad([p, q, r, s])));
                }
            }
        }
        if free > 0 {
            let handle = if tool == GizmoTool::Scale {
                Handle::ScaleAll
            } else if moves {
                Handle::Centre
            } else {
                // The rotate tool has no centre handle.
                return out;
            };
            out.push((
                handle,
                Shape::Dot {
                    centre,
                    radius: CENTRE,
                },
            ));
        }
        out
    }

    /// The handle under the mouse, if any: the nearest within
    /// [`GRAB_DISTANCE`], the centre first.
    pub fn handle_at(
        &self,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
        locked: [bool; 3],
    ) -> Option<Handle> {
        let shapes = self.shapes(camera, size, locked);
        if let Some((handle, _)) = shapes.iter().find(|(h, s)| {
            matches!(h, Handle::Centre | Handle::ScaleAll) && s.distance(mouse) <= 0.0
        }) {
            return Some(*handle);
        }
        shapes
            .iter()
            .map(|(h, s)| (*h, s.distance(mouse)))
            .filter(|(_, d)| *d <= GRAB_DISTANCE)
            .min_by(|a, b| {
                // Filled squares win over lines they overlap.
                let rank = |h: &Handle, d: f32| {
                    if matches!(
                        h,
                        Handle::Plane(_) | Handle::ScaleAxis(_) | Handle::ScalePlane(_)
                    ) && d <= 0.0
                    {
                        -1.0
                    } else {
                        d
                    }
                };
                rank(&a.0, a.1).total_cmp(&rank(&b.0, b.1))
            })
            .map(|(h, _)| h)
    }
}

/// What a drag does, in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    /// Move by this much.
    Translate(Vec3),
    /// Turn by `angle` (radians) around `axis` through the gizmo's origin.
    Rotate {
        /// The axis, unit length.
        axis: Vec3,
        /// The angle, right-handed around the axis.
        angle: f32,
    },
    /// Scale by these factors along the gizmo's own axes, around its
    /// origin.
    Scale(Vec3),
}

/// A drag of a handle in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct Drag {
    /// The handle being dragged.
    pub handle: Handle,
    /// The gizmo as it was when the drag started.
    pub gizmo: Gizmo,
    camera: Camera,
    size: Vec2,
    /// The arrow length in the world when the drag started.
    length: f32,
    /// Where the mouse was last frame.
    last_mouse: Vec2,
    /// The handle's own measure last frame (distance along the axis, point
    /// in the plane), to work out each frame's step.
    last_raw: Option<Vec3>,
    /// The change so far, with fine steps counted smaller.
    total: Total,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Total {
    Translate(Vec3),
    Angle(f32),
    Factors(Vec3),
}

impl Drag {
    /// Starts dragging `handle` of `gizmo` from `mouse`.
    pub fn new(gizmo: Gizmo, handle: Handle, camera: &Camera, size: Vec2, mouse: Vec2) -> Self {
        let length = gizmo.length(camera, size);
        let total = match handle {
            Handle::Axis(_) | Handle::Plane(_) | Handle::Centre => Total::Translate(Vec3::ZERO),
            Handle::Ring(_) | Handle::ViewRing => Total::Angle(0.0),
            Handle::ScaleAxis(_) | Handle::ScalePlane(_) | Handle::ScaleAll => {
                Total::Factors(Vec3::ONE)
            }
        };
        let mut drag = Self {
            handle,
            gizmo,
            camera: *camera,
            size,
            length,
            last_mouse: mouse,
            last_raw: None,
            total,
        };
        drag.last_raw = drag.raw(mouse);
        drag
    }

    /// The change for the mouse now at `mouse`. With `fine`, this frame's
    /// step counts [`FINE_FACTOR`] as much. Locked axes don't change.
    pub fn update(&mut self, mouse: Vec2, fine: bool, locked: [bool; 3]) -> Change {
        let factor = if fine { FINE_FACTOR } else { 1.0 };
        let raw = self.raw(mouse);
        let turn = self.screen_turn(self.last_mouse, mouse);
        match (&mut self.total, raw, self.last_raw) {
            (Total::Translate(total), Some(now), Some(before)) => {
                *total += (now - before) * factor;
            }
            (Total::Factors(total), Some(now), Some(before)) => {
                *total += (now - before) * factor;
            }
            (Total::Angle(total), _, _) => {
                *total += turn * factor;
            }
            _ => {}
        }
        if raw.is_some() {
            self.last_raw = raw;
        }
        self.last_mouse = mouse;
        self.change(locked)
    }

    /// The change so far, without moving the mouse (after the locks or
    /// snapping changed).
    pub fn change(&self, locked: [bool; 3]) -> Change {
        let axes = self.gizmo.axes;
        match self.total {
            Total::Translate(v) => {
                let mut v = v;
                for (axis, locked) in axes.iter().zip(locked) {
                    if locked {
                        v -= *axis * v.dot(*axis);
                    }
                }
                Change::Translate(v)
            }
            Total::Angle(angle) => Change::Rotate {
                axis: self.turn_axis(),
                angle,
            },
            Total::Factors(f) => {
                let mut f = f;
                for (i, locked) in locked.into_iter().enumerate() {
                    if locked {
                        f[i] = 1.0;
                    }
                }
                Change::Scale(f)
            }
        }
    }

    /// The axis a ring turns around.
    fn turn_axis(&self) -> Vec3 {
        match self.handle {
            Handle::Ring(i) => self.gizmo.axes[i],
            _ => towards_viewer(&self.camera, self.gizmo.origin),
        }
    }

    /// The handle's own measure for the mouse at `mouse`: the point on
    /// the axis or in the plane under the mouse (as an offset from the
    /// origin), or the scale factors. `None` when the mouse ray runs along
    /// the plane, so nothing sensible can be worked out this frame.
    fn raw(&self, mouse: Vec2) -> Option<Vec3> {
        let ray = self.camera.ray_through(mouse, self.size);
        let origin = self.gizmo.origin;
        let axes = self.gizmo.axes;
        match self.handle {
            Handle::Axis(i) => Some(axes[i] * along_axis(&ray, origin, axes[i])?),
            Handle::Plane(i) => Some(on_plane(&ray, origin, axes[i])? - origin),
            Handle::Centre => {
                Some(on_plane(&ray, origin, towards_viewer(&self.camera, origin))? - origin)
            }
            Handle::ScaleAxis(i) => {
                let t = along_axis(&ray, origin, axes[i])? / self.length.max(1e-6);
                let mut f = Vec3::ZERO;
                f[i] = t;
                Some(f)
            }
            Handle::ScalePlane(i) => {
                // The spot in the plane under the mouse, measured along
                // each of the plane's two axes: dragging along one axis
                // scales mostly that one, diagonally both.
                let spot = on_plane(&ray, origin, axes[i])? - origin;
                let length = self.length.max(1e-6);
                let mut f = Vec3::ZERO;
                for j in [(i + 1) % 3, (i + 2) % 3] {
                    f[j] = spot.dot(axes[j]) / length;
                }
                Some(f)
            }
            Handle::ScaleAll => {
                // Right or up grows; left or down shrinks.
                let px = (mouse.x - mouse.y) / GIZMO_SIZE;
                Some(Vec3::splat(px))
            }
            Handle::Ring(_) | Handle::ViewRing => None,
        }
    }

    /// The turn (radians, right-handed around the turn axis) from the
    /// mouse at `from` to `to`, measured around the gizmo's centre on
    /// screen.
    fn screen_turn(&self, from: Vec2, to: Vec2) -> f32 {
        let Some(centre) = self.camera.project(self.gizmo.origin, self.size) else {
            return 0.0;
        };
        let (a, b) = (from - centre, to - centre);
        if a.length() < 1.0 || b.length() < 1.0 {
            return 0.0;
        }
        // Screen y points down, so a turn that looks anticlockwise has a
        // negative cross product here.
        let screen = a.perp_dot(b).atan2(a.dot(b));
        let facing = self
            .turn_axis()
            .dot(towards_viewer(&self.camera, self.gizmo.origin));
        let sign = if facing >= 0.0 { -1.0 } else { 1.0 };
        screen * sign
    }
}

/// The unit direction from `at` towards the viewer.
fn towards_viewer(camera: &Camera, at: Vec3) -> Vec3 {
    if camera.is_flat() {
        -camera.forward()
    } else {
        (camera.position - at).normalize_or(-camera.forward())
    }
}

/// Where along the line through `origin` in direction `axis` the ray
/// passes closest, as a distance from `origin`. `None` when the ray runs
/// along the line.
fn along_axis(ray: &Ray, origin: Vec3, axis: Vec3) -> Option<f32> {
    let w = origin - ray.origin;
    let b = axis.dot(ray.direction);
    let denominator = 1.0 - b * b;
    if denominator.abs() < 1e-6 {
        return None;
    }
    let t = (b * w.dot(ray.direction) - w.dot(axis)) / denominator;
    t.is_finite().then_some(t)
}

/// Where the ray meets the plane through `origin` across `normal`. `None`
/// when it runs along the plane.
fn on_plane(ray: &Ray, origin: Vec3, normal: Vec3) -> Option<Vec3> {
    let facing = ray.direction.dot(normal);
    if facing.abs() < 1e-4 {
        return None;
    }
    let distance = (origin - ray.origin).dot(normal) / facing;
    let hit = ray.at(distance);
    hit.is_finite().then_some(hit)
}

/// `value` rounded to the nearest multiple of `step` (unchanged for a step
/// that isn't positive).
pub fn snap(value: f32, step: f32) -> f32 {
    if step > 0.0 && step.is_finite() {
        (value / step).round() * step
    } else {
        value
    }
}

/// A point on the line through `origin` along `axis` nearest `target`.
pub fn nearest_on_axis(origin: Vec3, axis: Vec3, target: Vec3) -> Vec3 {
    origin + axis * (target - origin).dot(axis)
}

/// `target` brought into the plane through `origin` across `normal`.
pub fn into_plane(origin: Vec3, normal: Vec3, target: Vec3) -> Vec3 {
    target - normal * (target - origin).dot(normal)
}

fn square(centre: Vec2, half: f32) -> [Vec2; 4] {
    [
        centre + Vec2::new(-half, -half),
        centre + Vec2::new(half, -half),
        centre + Vec2::new(half, half),
        centre + Vec2::new(-half, half),
    ]
}

fn quad_area(q: &[Vec2; 4]) -> f32 {
    let mut twice = 0.0;
    for i in 0..4 {
        twice += q[i].perp_dot(q[(i + 1) % 4]);
    }
    (twice * 0.5).abs()
}

fn inside_quad(q: &[Vec2; 4], p: Vec2) -> bool {
    // Inside a convex four-sided shape: on the same side of every edge.
    let sides: Vec<f32> = (0..4)
        .map(|i| (q[(i + 1) % 4] - q[i]).perp_dot(p - q[i]))
        .collect();
    sides.iter().all(|s| *s >= 0.0) || sides.iter().all(|s| *s <= 0.0)
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    let t = if length_squared > 1e-12 {
        ((p - a).dot(along) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + along * t).distance(p)
}

#[cfg(test)]
mod tests;
