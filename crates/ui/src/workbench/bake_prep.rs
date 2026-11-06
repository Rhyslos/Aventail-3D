//! Getting an object ready for File → Bake… (decisions 0250 and 0253):
//! its shape in the world with its painted cage scaled along, the
//! stacked and mirrored copies to leave out, and a file name of its own.

use mesh::uvs::Uvs;
use scene::{FaceId, Mesh, VertexId};
use std::collections::BTreeMap;

/// The grid the UV square (or the UVs' box, past it) is cut into to
/// compare where pieces lie, along its longer side.
const GRID: f32 = 256.0;

/// The UV corners of each polygon of `piece` (polygons without UVs
/// left out).
fn piece_polygons(mesh: &Mesh, uvs: &Uvs, piece: &[FaceId]) -> Vec<Vec<glam::Vec2>> {
    piece
        .iter()
        .filter_map(|f| {
            let n = mesh.face(*f).map_or(0, <[_]>::len);
            (0..n).map(|k| uvs.corner(mesh, *f, k)).collect()
        })
        .collect()
}

/// Whether `p` lies inside the polygon `corners` (even-odd, so concave
/// polygons and either winding work).
fn inside(corners: &[glam::Vec2], p: glam::Vec2) -> bool {
    let mut odd = false;
    let mut j = corners.len().saturating_sub(1);
    for (i, a) in corners.iter().enumerate() {
        let b = corners[j];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// The grid cells a piece's polygons cover (by each cell's middle, and
/// the cell under each polygon's first corner, so even a tiny piece
/// covers one), sorted.
fn cells(polygons: &[Vec<glam::Vec2>], origin: glam::Vec2, cell: f32, width: u32) -> Vec<u32> {
    let to_cell = |p: glam::Vec2| ((p - origin) / cell).floor().max(glam::Vec2::ZERO);
    let mut out = Vec::new();
    for corners in polygons {
        let Some(first) = corners.first() else {
            continue;
        };
        let (mut lo, mut hi) = (*first, *first);
        for p in corners {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let (a, b) = (to_cell(lo), to_cell(hi));
        let f = to_cell(*first);
        out.push(f.y as u32 * width + f.x as u32);
        for y in a.y as u32..=(b.y as u32) {
            for x in a.x as u32..=(b.x as u32) {
                let middle = origin + (glam::Vec2::new(x as f32, y as f32) + 0.5) * cell;
                if inside(corners, middle) {
                    out.push(y * width + x);
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// How many of two sorted lists' entries they share.
fn shared(a: &[u32], b: &[u32]) -> usize {
    let (mut i, mut j, mut n) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                n += 1;
                i += 1;
                j += 1;
            }
        }
    }
    n
}

/// The pieces of `mesh` to leave out of a bake (decision 0253): each
/// piece whose UVs lie mostly over an earlier piece's (stacked copies,
/// and mirrored halves laid onto their other half), so only one of each
/// writes its texels. Where the pieces' polygons lie is compared, not
/// their boxes, so a piece packed in an L-shaped piece's empty corner (or
/// a ring's hole) stays in.
pub(crate) fn stacked_pieces(mesh: &Mesh) -> Vec<FaceId> {
    let Some(uvs) = Uvs::of(mesh) else {
        return Vec::new();
    };
    let pieces: Vec<(Vec<FaceId>, Vec<Vec<glam::Vec2>>)> = mesh::uvs::islands(mesh)
        .into_iter()
        .map(|piece| {
            let polygons = piece_polygons(mesh, &uvs, &piece);
            (piece, polygons)
        })
        .collect();
    let (mut lo, mut hi) = (glam::Vec2::splat(f32::MAX), glam::Vec2::splat(f32::MIN));
    for p in pieces
        .iter()
        .flat_map(|(_, polygons)| polygons.iter().flatten())
    {
        if p.is_finite() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let extent = (hi - lo).max_element();
    if !(extent > 0.0 && extent.is_finite()) {
        return Vec::new();
    }
    let cell = extent / GRID;
    let width = ((hi.x - lo.x) / cell).floor() as u32 + 1;
    let covered: Vec<(&[FaceId], Vec<u32>)> = pieces
        .iter()
        .filter(|(_, polygons)| polygons.iter().flatten().all(|p| p.is_finite()))
        .map(|(piece, polygons)| (piece.as_slice(), cells(polygons, lo, cell, width)))
        .filter(|(_, cells)| cells.len() > 1)
        .collect();
    let mut skipped = Vec::new();
    for (i, (piece, mine)) in covered.iter().enumerate() {
        let lies_over = covered[..i].iter().any(|(_, theirs)| {
            shared(mine, theirs) as f32 > 0.9 * mine.len().min(theirs.len()) as f32
        });
        if lies_over {
            skipped.extend(piece.iter().copied());
        }
    }
    skipped
}

/// A copy of `mesh` whose painted cage pushes (decision 0251) are scaled
/// as `matrix` scales the shape along each point's facing, so the cage
/// baked reaches as far as the one drawn on the object. `None` when
/// nothing is painted (the shape can be used as it is).
pub(crate) fn cage_in_the_world(mesh: &Mesh, matrix: glam::Mat4) -> Option<Mesh> {
    let pushes = mesh::cage::pushes(mesh);
    if pushes.is_empty() {
        return None;
    }
    // Each point's facing, as the cage is drawn (cage::cage_lines).
    let mut normals: BTreeMap<VertexId, glam::Vec3> = BTreeMap::new();
    for (f, corners) in mesh.faces() {
        let n = mesh.face_normal(f);
        for v in corners {
            *normals.entry(*v).or_default() += n;
        }
    }
    let linear = glam::Mat3::from_mat4(matrix);
    let even = linear.determinant().abs().cbrt();
    let scaled: Vec<(VertexId, f32)> = pushes
        .into_iter()
        .map(|(v, push)| {
            let n = normals
                .get(&v)
                .copied()
                .unwrap_or_default()
                .normalize_or_zero();
            let factor = if n == glam::Vec3::ZERO {
                even
            } else {
                (linear * n).length()
            };
            (
                v,
                if factor.is_finite() {
                    push * factor
                } else {
                    push
                },
            )
        })
        .collect();
    let mut copy = mesh.clone();
    mesh::cage::set_push(&mut copy, &scaled);
    Some(copy)
}

/// The first material slot `mesh`'s polygons don't use, for polygons
/// kept in the shape but not baked; `None` when every slot is in use.
pub(crate) fn spare_slot(mesh: &Mesh) -> Option<u32> {
    let next = mesh
        .faces()
        .map(|(f, _)| mesh::material::slot_of(mesh, f) + 1)
        .max()
        .unwrap_or(0);
    (next < mesh::material::MAX_SLOTS).then_some(next)
}

/// `name` made safe for a file name.
pub(crate) fn file_safe(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.trim_matches('_').is_empty() {
        "bake".to_string()
    } else {
        safe
    }
}

/// `names` made different from each other, as file names on any system
/// (ignoring case): a second "crate_low" becomes "crate_low_2", and so
/// on, so no bake writes over another's files.
pub(crate) fn unique_names(names: &mut [String]) {
    let mut used = std::collections::BTreeSet::new();
    for name in names.iter_mut() {
        if !used.insert(name.to_lowercase()) {
            let mut n = 2;
            while !used.insert(format!("{name}_{n}").to_lowercase()) {
                n += 1;
            }
            *name = format!("{name}_{n}");
        }
    }
}
