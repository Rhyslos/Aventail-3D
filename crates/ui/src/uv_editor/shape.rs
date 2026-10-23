//! One object's UVs as the editor draws and picks them, and the flat
//! geometry it needs (a polygon's area, whether a point is inside it, the
//! distance to an edge).

use egui::Pos2;
use glam::Vec2;
use mesh::uvs::Uvs;
use scene::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// One object's UVs, as drawn and picked.
#[derive(Debug, Clone)]
pub(super) struct UvShape {
    /// The shape they were worked out from, to tell when it changes.
    pub(super) mesh: Arc<Mesh>,
    /// Each polygon's corners: its point and UV.
    pub(super) polygons: Vec<(FaceId, Vec<(VertexId, Vec2)>)>,
    /// Edges as drawn once each: the two UVs, the mesh edge, and whether it
    /// borders an island (a seam or the shape's open edge).
    pub(super) edges: Vec<([Vec2; 2], EdgeKey, bool)>,
    /// The island of each polygon, by slot.
    pub(super) island_of: Vec<usize>,
    /// The polygons of each island.
    pub(super) islands: Vec<Vec<FaceId>>,
    /// The edges marked as seams, drawn orange (decision 0098).
    pub(super) seams: BTreeSet<EdgeKey>,
    /// Each polygon's UV area and real area, in the order of `polygons`,
    /// for its texel density (decision 0100).
    pub(super) areas: Vec<(f32, f32)>,
    /// How evenly each polygon lies, in the order of `polygons`
    /// (decision 0102).
    pub(super) stretch: Vec<uv::stretch::Stretch>,
    /// The spots of pinned UVs, each once (decision 0109).
    pub(super) pins: Vec<Vec2>,
    /// The edges with a seam preference (decision 0111).
    pub(super) preferences: HashMap<EdgeKey, scene::SeamPreference>,
    /// Whether each polygon, in the order of `polygons`, is locked
    /// (decision 0112).
    pub(super) locked: Vec<bool>,
}

impl UvShape {
    pub(super) fn of(mesh: Arc<Mesh>) -> Option<Self> {
        let uvs = Uvs::of(&mesh)?;
        let (starts, corners) = mesh.face_layout();
        let mut polygons = Vec::new();
        // Each side of each polygon, by its edge: the UVs from both sides.
        let mut sides: HashMap<EdgeKey, Vec<[Vec2; 2]>> = HashMap::new();
        for (face, list) in mesh.faces() {
            let start = starts[face.get() as usize] as usize;
            let points: Vec<(VertexId, Vec2)> = (0..list.len())
                .map(|k| (corners[start + k], uvs.at(start + k)))
                .collect();
            for k in 0..points.len() {
                let (a, ua) = points[k];
                let (b, ub) = points[(k + 1) % points.len()];
                let key = EdgeKey::new(a, b);
                let pair = if a < b { [ua, ub] } else { [ub, ua] };
                sides.entry(key).or_default().push(pair);
            }
            polygons.push((face, points));
        }
        let mut edges = Vec::new();
        for (key, uses) in sides {
            let joined = |p: &[Vec2; 2], q: &[Vec2; 2]| {
                p[0].distance(q[0]) <= mesh::uvs::SAME_UV
                    && p[1].distance(q[1]) <= mesh::uvs::SAME_UV
            };
            match uses.as_slice() {
                [only] => edges.push((*only, key, true)),
                [p, q] if joined(p, q) => edges.push((*p, key, false)),
                many => {
                    for p in many {
                        edges.push((*p, key, true));
                    }
                }
            }
        }
        edges.sort_by_key(|(_, key, _)| *key);
        let (island_of, count) = mesh::uvs::island_of_each(&mesh);
        let mut islands = vec![Vec::new(); count];
        for (face, _) in &polygons {
            if let Some(i) = island_of.get(face.get() as usize).copied()
                && i < count
            {
                islands[i].push(*face);
            }
        }
        let seams = mesh::uvs::seams(&mesh).into_iter().collect();
        let stretch = uv::stretch::stretch_of(&mesh)
            .map(|list| list.into_iter().map(|(_, s)| s).collect())
            .unwrap_or_default();
        let areas = polygons
            .iter()
            .map(|(face, _)| uv::density::areas(&mesh, &[*face], &uvs))
            .collect();
        let mut pins: Vec<Vec2> = Vec::new();
        if let Some(pinned) = mesh::uvs::pins(&mesh) {
            for (k, pinned) in pinned.iter().enumerate() {
                let spot = uvs.at(k);
                if *pinned && !pins.iter().any(|p| p.distance(spot) <= mesh::uvs::SAME_UV) {
                    pins.push(spot);
                }
            }
        }
        let preferences = mesh::uvs::seam_preferences(&mesh).into_iter().collect();
        let lock = mesh::uvs::locked(&mesh);
        let locked = polygons
            .iter()
            .map(|(f, _)| {
                lock.as_ref()
                    .is_some_and(|l| l.get(f.get() as usize).copied().unwrap_or(false))
            })
            .collect();
        Some(Self {
            mesh,
            polygons,
            edges,
            island_of,
            islands,
            seams,
            areas,
            stretch,
            pins,
            preferences,
            locked,
        })
    }
}

/// Twice the signed area of a polygon (counter-clockwise positive).
pub(super) fn polygon_area(corners: &[Vec2]) -> f32 {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .map(|(a, b)| a.perp_dot(*b))
        .sum::<f32>()
        / 2.0
}

/// Whether `p` is inside the polygon (by the winding of its edges around
/// it, so it works for any shape).
pub(super) fn inside(corners: &[Vec2], p: Vec2) -> bool {
    let mut winding = 0i32;
    for (a, b) in corners.iter().zip(corners.iter().cycle().skip(1)) {
        if a.y <= p.y {
            if b.y > p.y && (*b - *a).perp_dot(p - *a) > 0.0 {
                winding += 1;
            }
        } else if b.y <= p.y && (*b - *a).perp_dot(p - *a) < 0.0 {
            winding -= 1;
        }
    }
    winding != 0
}

/// How far `p` is from the segment `a`–`b`.
pub(super) fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0.0 {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}
