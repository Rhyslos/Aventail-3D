//! Remesh for sculpting ([decision 0312](../../../../docs/decisions/0312-remesh-for-sculpting.md)):
//! the voxel remesh baked into a sculpt's shape, made for shapes of
//! millions of polygons. Its limit is the polygons it would make (worked
//! out before it starts, from the shape's area), not the box round the
//! shape; holes are closed first when asked; the new surface can be laid
//! back on the old one, so detail the voxels can hold isn't smoothed away;
//! a label per polygon (the sculpt groups) is carried over; and it tells
//! how far it has got, and stops when asked.

use super::{Grid, RemeshOptions, RemeshProblem, Soup, holes, nets, sparse};
use crate::Mesh;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// The most polygons remeshing for sculpting may make: past this it is
/// refused before it starts, with the voxel size that fits.
pub const MAX_SCULPT_POLYGONS: u64 = 8_000_000;
/// Past this many polygons, sculpting gets heavy: the interface warns.
pub const HEAVY_POLYGONS: u64 = 3_000_000;

/// The settings of remeshing for sculpting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SculptRemeshOptions {
    /// How far apart the grid's points are, in the shape's own
    /// centimetres: the size of the new polygons, roughly.
    pub voxel_size: f32,
    /// Lay the new surface back on the old one after evening it out.
    pub keep_detail: bool,
    /// Close the holes of an open shape first (otherwise it is refused).
    pub fix_holes: bool,
    /// Relax passes evening out the new surface.
    pub smooth: u32,
}

impl Default for SculptRemeshOptions {
    fn default() -> Self {
        Self {
            voxel_size: RemeshOptions::STANDARD_VOXEL,
            keep_detail: true,
            fix_holes: true,
            smooth: RemeshOptions::STANDARD_SMOOTH,
        }
    }
}

/// How far a remesh has got, and a way to stop it, shared with another
/// thread.
#[derive(Debug, Default)]
pub struct Progress {
    /// How far, in thousandths.
    done: AtomicU32,
    /// Set to stop: the remesh then gives up as soon as it can.
    cancel: AtomicBool,
}

impl Progress {
    /// How far it is, 0 to 1.
    pub fn share(&self) -> f32 {
        self.done.load(Relaxed) as f32 / 1000.0
    }

    /// Asks it to stop.
    pub fn stop(&self) {
        self.cancel.store(true, Relaxed);
    }

    /// Whether it was asked to stop.
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Relaxed)
    }

    pub(super) fn set(&self, share: f32) {
        self.done
            .fetch_max((share.clamp(0.0, 1.0) * 1000.0) as u32, Relaxed);
    }

    pub(super) fn check(&self) -> Result<(), RemeshProblem> {
        if self.cancelled() {
            Err(RemeshProblem::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// A shape remeshed for sculpting.
#[derive(Debug, Clone)]
pub struct Remeshed {
    /// The new shape.
    pub mesh: Mesh,
    /// Each new polygon's label, from the labels given (empty when none
    /// were).
    pub labels: Vec<u32>,
}

/// The area `mesh`'s surface shows along X, Y and Z, added up (each
/// polygon's area times how much it faces each axis): a voxel remesh
/// makes about this divided by the voxel size squared polygons, since
/// each makes one quad across every grid edge the surface crosses. Quick
/// enough to work out once per shape.
pub fn projected_area(mesh: &Mesh) -> f64 {
    let mut total = 0.0f64;
    for (_, corners) in mesh.faces() {
        let points: Vec<_> = corners.iter().filter_map(|v| mesh.position(*v)).collect();
        if points.len() < 3 {
            continue;
        }
        // Twice the polygon's area along its facing (Newell's way).
        let mut normal = glam::Vec3::ZERO;
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            normal += a.cross(b);
        }
        let a = normal.abs() * 0.5;
        let sum = f64::from(a.x + a.y + a.z);
        if sum.is_finite() {
            total += sum;
        }
    }
    total
}

/// About how many polygons a remesh at `voxel_size` makes of a shape with
/// [`projected_area`] `area`.
pub fn polygons_from(area: f64, voxel_size: f32) -> u64 {
    let step = f64::from(voxel_size.max(RemeshOptions::MIN_VOXEL));
    (area / (step * step)).round().min(u64::MAX as f64) as u64
}

/// The smallest voxel size (rounded up to 0.01 cm) at which a shape with
/// [`projected_area`] `area` makes at most `polygons` polygons.
pub fn voxel_for(area: f64, polygons: u64) -> f32 {
    let size = (area / polygons.max(1) as f64).sqrt() as f32;
    let mut rounded = ((size * 100.0).ceil() / 100.0).max(RemeshOptions::MIN_VOXEL);
    while polygons_from(area, rounded) > polygons && rounded < RemeshOptions::MAX_VOXEL {
        rounded += 0.01;
    }
    rounded
}

/// `mesh` remeshed for sculpting with `options` (see the module notes):
/// an even, closed surface of quads, overlapping pieces merged into one.
/// `labels` gives a label per polygon of `mesh` (by id number), carried to
/// the new polygons from the nearest. Progress goes to `progress`, which
/// can stop it.
pub fn remesh_for_sculpting(
    mesh: &Mesh,
    options: &SculptRemeshOptions,
    labels: Option<&[u32]>,
    progress: &Progress,
) -> Result<Remeshed, RemeshProblem> {
    let step = RemeshOptions {
        voxel_size: options.voxel_size,
        smooth: options.smooth,
    }
    .clamped();
    let mut soup = Soup::of(mesh)?;
    let rims = holes::open_edges(mesh);
    if !rims.is_empty() {
        if !options.fix_holes {
            return Err(RemeshProblem::NotClosed);
        }
        if holes::close(&mut soup, &rims, mesh) > 0 {
            return Err(RemeshProblem::NotClosed);
        }
    }
    progress.check()?;
    let area: f64 = soup
        .triangles
        .iter()
        .map(|[a, b, c]| {
            let n = ((*b - *a).cross(*c - *a) * 0.5).abs();
            f64::from(n.x + n.y + n.z)
        })
        .filter(|a| a.is_finite())
        .sum();
    let polygons = polygons_from(area, step.voxel_size);
    if polygons > MAX_SCULPT_POLYGONS {
        return Err(RemeshProblem::TooManyPolygons {
            polygons,
            smallest: voxel_for(area, MAX_SCULPT_POLYGONS),
        });
    }
    let bounds = soup.bounds().ok_or(RemeshProblem::NotANumber)?;
    let counts = Grid::counts(bounds, step.voxel_size);
    if counts.iter().any(|c| !c.is_finite() || *c > 1e9)
        || sparse::block_dims(counts.map(|c| c as usize))
            .iter()
            .map(|d| *d as f64)
            .product::<f64>()
            > sparse::MAX_BLOCKS as f64
    {
        return Err(RemeshProblem::TooSpread);
    }
    let grid = Grid::spanning(bounds, step.voxel_size);
    let field = sparse::Sparse::new(grid, &soup, progress, (0.0, 0.7))?;
    let mut surface = nets::Surface::extract(&field, progress)?;
    if surface.quads.is_empty() {
        return Err(RemeshProblem::NothingLeft);
    }
    progress.set(0.8);
    surface.relax(step.smooth, progress)?;
    if options.keep_detail {
        surface.keep_detail(&field, &soup);
    }
    progress.check()?;
    progress.set(0.9);
    let new_labels = match labels {
        Some(labels) => {
            let per_triangle: Vec<u32> = soup
                .faces
                .iter()
                .map(|f| labels.get(*f as usize).copied().unwrap_or(0))
                .collect();
            surface.labels(&per_triangle)
        }
        None => Vec::new(),
    };
    drop(field);
    let mesh = surface.into_mesh(&soup, false)?;
    progress.set(1.0);
    Ok(Remeshed {
        mesh,
        labels: new_labels,
    })
}
