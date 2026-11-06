//! The baker's maps ([decision 0250](../../../docs/decisions/0250-the-baker.md)):
//! the maps a game asset is textured with, baked onto a low-detail shape's
//! UVs, from a high-detail shape when there is one.
//!
//! - **Normal** (tangent space, as Godot reads it): the high-detail shape's
//!   facing, in each texel's frame of the low shape's shading normal and
//!   its MikkTSpace tangent (the tangents Godot works out on import), green
//!   up (OpenGL) or down (DirectX).
//! - **Height**: how far above or below the low shape the high one lies,
//!   0.5 on it, within the cage.
//! - **Occlusion**, **Curvature**, **Thickness**, **Colour ID** and
//!   **Position**: from the low shape itself, as the painter's bakes are
//!   ([`crate::bake`]).
//!
//! A high-detail shape is found by a ray from just outside the low shape
//! (the *cage*, pushed out along its shading normal) back in through it,
//! the nearest hit within twice the push. Texels whose ray meets nothing
//! keep the flat normal and the middle height.

use crate::{BakeKind, BakeSettings, Progress, Rays, Source, colour, grey};
use glam::{Vec2, Vec3, Vec4};
use paint::{Image, PaintSurface};

/// A map the baker makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapKind {
    /// Tangent-space normals.
    Normal,
    /// Ambient occlusion.
    Occlusion,
    /// Outer edges light, inner ones dark.
    Curvature,
    /// Thin light, thick dark.
    Thickness,
    /// A colour per material.
    Id,
    /// Where on the shape, across its box.
    Position,
    /// The high shape's height over the low one.
    Height,
}

impl MapKind {
    /// Every map, in the order offered.
    pub const ALL: [Self; 7] = [
        Self::Normal,
        Self::Occlusion,
        Self::Curvature,
        Self::Thickness,
        Self::Id,
        Self::Position,
        Self::Height,
    ];

    /// The end of its file's name ("crate_normal.png").
    pub fn file_suffix(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Occlusion => "occlusion",
            Self::Curvature => "curvature",
            Self::Thickness => "thickness",
            Self::Id => "id",
            Self::Position => "position",
            Self::Height => "height",
        }
    }

    /// Whether it is baked from a high-detail shape when there is one.
    pub fn uses_high(self) -> bool {
        matches!(self, Self::Normal | Self::Height)
    }

    /// The painter's bake that makes it from the low shape, for the maps
    /// that come from the low shape alone.
    fn from_low(self) -> Option<BakeKind> {
        match self {
            Self::Occlusion => Some(BakeKind::Occlusion),
            Self::Curvature => Some(BakeKind::Curvature),
            Self::Thickness => Some(BakeKind::Thickness),
            Self::Id => Some(BakeKind::Id),
            Self::Position => Some(BakeKind::Position),
            Self::Normal | Self::Height => None,
        }
    }
}

/// How the maps are baked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapSettings {
    /// What the low-shape maps use (rays, reach, edge width).
    pub low: BakeSettings,
    /// How far the cage is pushed out from the low shape, as a share of
    /// its size (the length of its box's diagonal).
    pub cage: f32,
    /// Normal maps with green down (DirectX, as Unreal reads them) rather
    /// than up (OpenGL, as Godot and Blender read them).
    pub green_down: bool,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            low: BakeSettings::default(),
            cage: 0.02,
            green_down: false,
        }
    }
}

/// The limits of the cage's push.
pub const CAGE_RANGE: std::ops::RangeInclusive<f32> = 0.0005..=0.5;

/// A high-detail shape ready for rays: its triangles and their corners'
/// shading normals.
#[derive(Debug, Clone)]
pub struct High {
    rays: Rays,
    triangles: Vec<[Vec3; 3]>,
    normals: Vec<[Vec3; 3]>,
}

impl High {
    /// `surface`'s triangles (those made of numbers), shaded as the
    /// viewport shades it.
    pub fn of(surface: &PaintSurface) -> Self {
        let mut triangles = Vec::new();
        let mut normals = Vec::new();
        for (i, t) in surface.triangles().iter().enumerate() {
            if !t.points.iter().all(|p| p.is_finite()) {
                continue;
            }
            triangles.push(t.points);
            let shading = surface.shading(i);
            let n = if shading.iter().all(|n| n.is_finite() && n.length() > 0.5) {
                shading
            } else {
                [t.normal; 3]
            };
            normals.push(n);
        }
        Self {
            rays: Rays::of_triangles(triangles.clone()),
            triangles,
            normals,
        }
    }

    /// Where the ray from `origin` along `direction` meets it within
    /// `reach`: how far, and the surface's facing there.
    fn hit(&self, origin: Vec3, direction: Vec3, reach: f32) -> Option<(f32, Vec3)> {
        let (d, i) = self.rays.nearest(origin, direction, reach)?;
        let p = origin + direction * d;
        let w = barycentric(&self.triangles[i], p);
        let n = self.normals[i];
        let normal = (n[0] * w[0] + n[1] * w[1] + n[2] * w[2]).normalize_or_zero();
        Some((d, if normal == Vec3::ZERO { n[0] } else { normal }))
    }
}

/// `p`'s weights over triangle `t`'s corners (summing to 1).
fn barycentric(t: &[Vec3; 3], p: Vec3) -> [f32; 3] {
    let (e0, e1, e2) = (t[1] - t[0], t[2] - t[0], p - t[0]);
    let (d00, d01, d11) = (e0.dot(e0), e0.dot(e1), e1.dot(e1));
    let (d20, d21) = (e2.dot(e0), e2.dot(e1));
    let det = d00 * d11 - d01 * d01;
    if det.abs() < 1e-20 {
        return [1.0, 0.0, 0.0];
    }
    let v = (d11 * d20 - d01 * d21) / det;
    let w = (d00 * d21 - d01 * d20) / det;
    [1.0 - v - w, v, w]
}

/// Each of `surface`'s triangles' corners' MikkTSpace tangents (xyz the
/// tangent, w the bitangent's sign), on its UVs with V up.
pub fn tangents(surface: &PaintSurface) -> Vec<[Vec4; 3]> {
    struct Mikk<'a> {
        surface: &'a PaintSurface,
        out: Vec<[Vec4; 3]>,
    }
    impl bevy_mikktspace::Geometry for Mikk<'_> {
        fn num_faces(&self) -> usize {
            self.surface.triangles().len()
        }
        fn num_vertices_of_face(&self, _face: usize) -> usize {
            3
        }
        fn position(&self, face: usize, vert: usize) -> [f32; 3] {
            self.surface.triangles()[face].points[vert].to_array()
        }
        fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
            let n = self.surface.shading(face)[vert];
            if n.is_finite() && n.length() > 0.5 {
                n.to_array()
            } else {
                self.surface.triangles()[face].normal.to_array()
            }
        }
        fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
            self.surface.triangles()[face].uvs[vert].to_array()
        }
        fn set_tangent(
            &mut self,
            tangent: Option<bevy_mikktspace::TangentSpace>,
            face: usize,
            vert: usize,
        ) {
            let t = Vec4::from_array(tangent.unwrap_or_default().tangent_encoded());
            self.out[face][vert] = if t.truncate().is_finite() && t.truncate().length() > 0.5 {
                t
            } else {
                // No UV area anywhere round the corner: any tangent square
                // to the normal, as nothing shows there.
                Vec3::from_array(self.normal(face, vert))
                    .normalize_or_zero()
                    .any_orthonormal_vector()
                    .extend(1.0)
            };
        }
    }
    let mut mikk = Mikk {
        surface,
        out: vec![[Vec4::new(1.0, 0.0, 0.0, 1.0); 3]; surface.triangles().len()],
    };
    let _ = bevy_mikktspace::generate_tangents(&mut mikk);
    mikk.out
}

/// The map `kind` of `source` (the low shape), from `high` where it uses
/// one, saying how far it is in `progress`; `None` when cancelled or for a
/// size out of range.
pub fn bake_map(
    source: &Source<'_>,
    high: Option<&High>,
    kind: MapKind,
    settings: &MapSettings,
    progress: &Progress,
) -> Option<Image> {
    bake_map_caged(source, high, kind, settings, None, progress)
}

/// Each of `surface`'s triangles' corners' extra cage push from the shape
/// they were made of (decision 0251): `mesh::cage::push_of` at each
/// corner's point.
pub fn cage_pushes(surface: &PaintSurface, mesh: &mesh::Mesh) -> Vec<[f32; 3]> {
    surface
        .triangles()
        .iter()
        .map(|t| t.corners.map(|c| mesh::cage::push_of(mesh, mesh::VertexId::from_raw(c))))
        .collect()
}

/// [`bake_map`], the cage pushed further out by `extra` (per triangle
/// corner of the source, in the shape's units, from [`cage_pushes`]).
pub fn bake_map_caged(
    source: &Source<'_>,
    high: Option<&High>,
    kind: MapKind,
    settings: &MapSettings,
    extra: Option<&[[f32; 3]]>,
    progress: &Progress,
) -> Option<Image> {
    if let Some(low) = kind.from_low() {
        return crate::bake_watched(std::slice::from_ref(source), low, &settings.low, progress);
    }
    use std::sync::atomic::Ordering::Relaxed;
    let size = source.size;
    let mut image = Image::new(size)?;
    let texels = crate::texels::covered(source.surface, source.slots, size);
    progress.done.store(0, Relaxed);
    progress.total.store(texels.len() as u64, Relaxed);
    let (lo, hi) = crate::bounds(source.surface, source.slots);
    let push = (hi - lo).length().max(1e-6)
        * settings
            .cage
            .clamp(*CAGE_RANGE.start(), *CAGE_RANGE.end());
    let tangents = (kind == MapKind::Normal).then(|| tangents(source.surface));
    let triangles = source.surface.triangles();
    let parts = mesh::parallel::in_parallel(texels.len(), |range| {
        let mut out = Vec::with_capacity(range.len());
        for t in &texels[range] {
            if progress.cancelled() {
                break;
            }
            let n = t.shading.normalize_or_zero();
            let n = if n == Vec3::ZERO { t.normal } else { n };
            let tri = &triangles[t.triangle as usize];
            let w = barycentric(&tri.points, t.at);
            // The cage painted further out (or in) here.
            let push = extra
                .and_then(|e| e.get(t.triangle as usize))
                .map_or(push, |e| (push + e[0] * w[0] + e[1] * w[1] + e[2] * w[2]).max(push * 0.05));
            // From the cage back in through the low shape.
            let hit = high.and_then(|h| h.hit(t.at + n * push, -n, push * 2.0));
            let value = match kind {
                MapKind::Height => {
                    let height = hit.map_or(0.0, |(d, _)| push - d);
                    grey(0.5 + height / (2.0 * push))
                }
                _ => {
                    let facing = hit.map_or(n, |(_, normal)| normal);
                    let corner = tangents.as_ref().map_or([Vec4::X; 3], |all| all[t.triangle as usize]);
                    let tangent4 = corner[0] * w[0] + corner[1] * w[1] + corner[2] * w[2];
                    // Unnormalised and per texel, as MikkTSpace asks: the
                    // bitangent from the normal and tangent here.
                    let tangent = (tangent4.truncate() - n * n.dot(tangent4.truncate())).normalize_or_zero();
                    let sign = if corner[0].w < 0.0 { -1.0 } else { 1.0 };
                    let bitangent = n.cross(tangent) * sign;
                    let mut local = Vec3::new(facing.dot(tangent), facing.dot(bitangent), facing.dot(n));
                    if settings.green_down {
                        local.y = -local.y;
                    }
                    colour(local.normalize_or_zero() * 0.5 + Vec3::splat(0.5))
                }
            };
            out.push((t.x, t.y, value));
        }
        progress.done.fetch_add(out.len() as u64, Relaxed);
        out
    });
    if progress.cancelled() {
        return None;
    }
    for part in parts {
        for (x, y, value) in part {
            image.set_pixel(x, y, value);
        }
    }
    Some(image)
}

/// What can go wrong at a texel when baking from a high shape (decision
/// 0252).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Problem {
    /// The ray from the cage met nothing: the high shape lies further out
    /// or in than the cage reaches.
    Missed,
    /// The ray met the high shape near the end of its reach: probably
    /// another part of it, behind the one meant (the cage reaches too
    /// far there).
    WrongPart,
    /// The high surface found faces far from the low one's way (more than
    /// about 70° off): the detail comes out skewed or smeared.
    Skewed,
}

impl Problem {
    /// Every problem, in the order listed.
    pub const ALL: [Self; 3] = [Self::Missed, Self::WrongPart, Self::Skewed];

    /// Its colour in the problem map.
    pub fn colour(self) -> [u8; 4] {
        match self {
            Self::Missed => [235, 40, 40, 255],
            Self::WrongPart => [220, 60, 220, 255],
            Self::Skewed => [245, 200, 30, 255],
        }
    }
}

/// How many of a bake's texels have each problem, of how many baked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Problems {
    /// Texels looked at.
    pub texels: usize,
    /// Texels whose ray met nothing.
    pub missed: usize,
    /// Texels that probably met another part.
    pub wrong_part: usize,
    /// Texels whose detail comes out skewed.
    pub skewed: usize,
}

impl Problems {
    /// How many have `problem`.
    pub fn count(&self, problem: Problem) -> usize {
        match problem {
            Problem::Missed => self.missed,
            Problem::WrongPart => self.wrong_part,
            Problem::Skewed => self.skewed,
        }
    }

    /// The share of the texels with `problem`, 0 to 1.
    pub fn share(&self, problem: Problem) -> f32 {
        if self.texels == 0 {
            0.0
        } else {
            self.count(problem) as f32 / self.texels as f32
        }
    }
}

/// How far into the ray's reach (as a share) a hit counts as probably
/// another part.
const WRONG_PART_FROM: f32 = 0.9;
/// The facing (cosine) under which detail counts as skewed.
const SKEWED_UNDER: f32 = 0.34;

/// The problems baking `source` from `high` meets, as a map (each
/// problem's colour, grey where all is well) and their counts; `None`
/// when cancelled or for a size out of range.
pub fn problem_map(
    source: &Source<'_>,
    high: &High,
    settings: &MapSettings,
    extra: Option<&[[f32; 3]]>,
    progress: &Progress,
) -> Option<(Image, Problems)> {
    use std::sync::atomic::Ordering::Relaxed;
    let size = source.size;
    let mut image = Image::new(size)?;
    let texels = crate::texels::covered(source.surface, source.slots, size);
    progress.done.store(0, Relaxed);
    progress.total.store(texels.len() as u64, Relaxed);
    let (lo, hi) = crate::bounds(source.surface, source.slots);
    let base = (hi - lo).length().max(1e-6) * settings.cage.clamp(*CAGE_RANGE.start(), *CAGE_RANGE.end());
    let triangles = source.surface.triangles();
    let parts = mesh::parallel::in_parallel(texels.len(), |range| {
        let mut out = Vec::with_capacity(range.len());
        for t in &texels[range] {
            if progress.cancelled() {
                break;
            }
            let n = t.shading.normalize_or_zero();
            let n = if n == Vec3::ZERO { t.normal } else { n };
            let w = barycentric(&triangles[t.triangle as usize].points, t.at);
            let push = extra
                .and_then(|e| e.get(t.triangle as usize))
                .map_or(base, |e| (base + e[0] * w[0] + e[1] * w[1] + e[2] * w[2]).max(base * 0.05));
            let problem = match high.hit(t.at + n * push, -n, push * 2.0) {
                None => Some(Problem::Missed),
                Some((d, _)) if d > push * 2.0 * WRONG_PART_FROM => Some(Problem::WrongPart),
                Some((_, facing)) if facing.dot(n) < SKEWED_UNDER => Some(Problem::Skewed),
                Some(_) => None,
            };
            out.push((t.x, t.y, problem));
        }
        progress.done.fetch_add(out.len() as u64, Relaxed);
        out
    });
    if progress.cancelled() {
        return None;
    }
    let mut counts = Problems::default();
    for part in parts {
        for (x, y, problem) in part {
            counts.texels += 1;
            let colour = match problem {
                Some(p) => {
                    match p {
                        Problem::Missed => counts.missed += 1,
                        Problem::WrongPart => counts.wrong_part += 1,
                        Problem::Skewed => counts.skewed += 1,
                    }
                    p.colour()
                }
                None => [150, 150, 150, 255],
            };
            image.set_pixel(x, y, colour);
        }
    }
    Some((image, counts))
}

/// Whether `kind` can be baked into vertex colours (decision 0253): the
/// maps worked out from the shape itself.
pub fn bakes_to_points(kind: MapKind) -> bool {
    kind.from_low().is_some()
}

/// Map `kind` sampled at each polygon corner of `surface` (the shape
/// `mesh`, shaded as `surface` is) instead of at texels: vertex colours,
/// for a game that colours a shape by its corners (decision 0253). Every
/// corner gets its point's value, so it shades smoothly across. `None`
/// for a map that needs texels (normal, height).
pub fn bake_corners(
    mesh: &mesh::Mesh,
    surface: &PaintSurface,
    kind: MapKind,
    settings: &BakeSettings,
) -> Option<Vec<(mesh::FaceId, mesh::VertexId, [u8; 4])>> {
    let low = kind.from_low()?;
    let all_slots: Vec<u32> = {
        let mut s: Vec<u32> = surface.triangles().iter().map(|t| t.slot).collect();
        s.sort_unstable();
        s.dedup();
        s
    };
    let (lo, hi) = crate::bounds(surface, &all_slots);
    let diagonal = (hi - lo).length().max(1e-6);
    let rays = matches!(low, BakeKind::Occlusion | BakeKind::Thickness).then(|| Rays::of(surface));
    let edges = (low == BakeKind::Curvature)
        .then(|| crate::curvature::Edges::of(surface, &all_slots, diagonal * settings.edge_width));
    // Each point's shading normal and slot, from the triangles.
    let mut normals: std::collections::BTreeMap<u32, (Vec3, u32)> = std::collections::BTreeMap::new();
    for (i, t) in surface.triangles().iter().enumerate() {
        let shading = surface.shading(i);
        for k in 0..3 {
            let entry = normals.entry(t.corners[k]).or_insert((Vec3::ZERO, t.slot));
            let n = shading[k];
            entry.0 += if n.is_finite() { n } else { t.normal };
        }
    }
    let value = |v: mesh::VertexId| {
        let p = mesh.position(v)?;
        let (n, slot) = normals.get(&v.get()).copied().unwrap_or((Vec3::Z, 0));
        let n = n.normalize_or_zero();
        let n = if n == Vec3::ZERO { Vec3::Z } else { n };
        let seed = v.get().wrapping_mul(2_654_435_761);
        Some(match low {
            BakeKind::Position => colour((p - lo) / (hi - lo).max(Vec3::splat(1e-6))),
            BakeKind::Normal => colour(n * 0.5 + Vec3::splat(0.5)),
            BakeKind::Occlusion => grey(rays.as_ref().map_or(1.0, |r| {
                r.openness(p, n, diagonal * settings.reach, settings.rays, seed)
            })),
            BakeKind::Thickness => grey(rays.as_ref().map_or(0.0, |r| {
                r.thinness(p, n, diagonal * settings.reach, settings.rays, seed)
            })),
            BakeKind::Curvature => grey(edges.as_ref().map_or(0.5, |e| e.at(p))),
            BakeKind::Id => crate::id_colour(slot),
        })
    };
    let mut cache: std::collections::BTreeMap<mesh::VertexId, [u8; 4]> = std::collections::BTreeMap::new();
    let mut out = Vec::new();
    for (f, corners) in mesh.faces() {
        for v in corners {
            let c = match cache.get(v) {
                Some(c) => *c,
                None => {
                    let c = value(*v)?;
                    cache.insert(*v, c);
                    c
                }
            };
            out.push((f, *v, c));
        }
    }
    Some(out)
}

/// The UV of the texel at `x`, `y` of a `size` texture (for tests).
#[cfg(test)]
fn texel_uv(x: u32, y: u32, size: u32) -> Vec2 {
    Vec2::new((x as f32 + 0.5) / size as f32, 1.0 - (y as f32 + 0.5) / size as f32)
}

#[cfg(test)]
mod tests;
