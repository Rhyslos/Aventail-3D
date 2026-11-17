//! Binary STL files for 3D printing
//! ([decision 0298](../../../docs/decisions/0298-stl-export.md)).
//!
//! An STL file is only triangles: an 80-byte header, a count, and for
//! each triangle its normal and three corners (four times three 32-bit
//! numbers, little-endian) and two bytes nobody uses. It has no units,
//! materials, UVs or names, and every slicer reads its numbers as
//! millimetres, so the scene's centimetres are turned into the
//! [`PrintUnits`] chosen. The app is Z up, as STL files and slicers are,
//! so nothing is turned: a model stands on the print bed the way it
//! stands on the grid.
//!
//! The shapes come from [`crate::export::prepare_for`], as for OBJ and
//! glTF, so modifiers, live mirrors and booleans are applied and polygons
//! cut into triangles the viewport's way. With [`ExportOptions::merge`],
//! more than one shape is merged into one solid first with the app's own
//! boolean union (the `booleans` crate), on copies: the scene is never
//! changed. Each triangle's normal is worked out from its corners, in the
//! order they go round (counter-clockwise seen from outside), so it
//! points out wherever the shape's polygons face out; [`check`] says in
//! plain words where they don't, and what else would trouble a slicer.
//!
//! The app writes STL but never reads it (the project lead's choice): the
//! reader in the tests is only there to check what is written.

pub mod check;

use crate::export::{
    ExportError, ExportFiles, ExportMesh, ExportOptions, ExportSummary, ExportedObject, export_mesh,
};
use glam::{Mat4, Vec3};
use mesh::{Mesh, VertexId};
use serde::{Deserialize, Serialize};
use words::{text, tr};

/// The bytes before an STL file's triangle count.
pub const HEADER_SIZE: usize = 80;
/// The bytes of one triangle: a normal, three corners, two spare bytes.
pub const TRIANGLE_SIZE: usize = 50;

/// The units an STL file's numbers are in. STL can't say, and slicers
/// read millimetres, so that is the standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum PrintUnits {
    /// Millimetres: what every slicer reads.
    #[default]
    Millimetres,
    /// Centimetres.
    Centimetres,
    /// Metres.
    Metres,
    /// Inches.
    Inches,
}

impl PrintUnits {
    /// Every choice, in the order the Export window lists them.
    pub const ALL: [Self; 4] = [
        Self::Millimetres,
        Self::Centimetres,
        Self::Metres,
        Self::Inches,
    ];

    /// How many of these one centimetre (one unit of the scene) is.
    pub fn per_centimetre(self) -> f32 {
        match self {
            Self::Millimetres => 10.0,
            Self::Centimetres => 1.0,
            Self::Metres => 0.01,
            Self::Inches => 1.0 / 2.54,
        }
    }

    /// Its name, as listed.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Millimetres => text!("Millimetres"),
            Self::Centimetres => text!("Centimetres"),
            Self::Metres => text!("Metres"),
            Self::Inches => text!("Inches"),
        })
    }

    /// Its short sign after a number: "mm", "cm", "m" or "in".
    pub fn symbol(self) -> &'static str {
        tr(match self {
            Self::Millimetres => text!("mm"),
            Self::Centimetres => text!("cm"),
            Self::Metres => text!("m"),
            Self::Inches => text!("in"),
        })
    }
}

/// The STL file of `objects` (made with `options`, an STL's: see
/// [`crate::export::ExportFormat::options`]), merged into one solid first
/// if `options.merge` says so and there is more than one shape.
pub(crate) fn files(
    objects: &[ExportedObject],
    options: &ExportOptions,
    generator: &str,
    summary: ExportSummary,
) -> Result<ExportFiles, ExportError> {
    let solids = solids(objects, options.merge)?;
    let (main, triangles) = write_binary(
        solids.iter(),
        options.print_units.per_centimetre(),
        generator,
    )?;
    Ok(ExportFiles {
        main,
        beside: Vec::new(),
        summary: ExportSummary {
            triangles,
            materials: 0,
            ..summary
        },
        textures: Vec::new(),
    })
}

/// The shapes to write: each object's (in the world, in centimetres),
/// or with `merge` and more than one, the one solid they make together.
pub fn solids(objects: &[ExportedObject], merge: bool) -> Result<Vec<ExportMesh>, ExportError> {
    let shapes: Vec<&ExportedObject> = objects
        .iter()
        .filter(|o| !o.built().polygons.is_empty())
        .collect();
    if !merge || shapes.len() < 2 {
        return Ok(shapes.into_iter().map(|o| o.built().clone()).collect());
    }
    let meshes: Vec<Mesh> = shapes.iter().map(|o| to_mesh(o.built())).collect();
    let (kept, rest) = meshes.split_first().ok_or(ExportError::Nothing)?;
    let cutters: Vec<booleans::Cutter> = rest
        .iter()
        .map(|mesh| booleans::Cutter {
            kind: booleans::Kind::Union,
            mesh,
            matrix: Mat4::IDENTITY,
        })
        .collect();
    let merged = booleans::apply(kept, &cutters, &booleans::Options::default()).map_err(|why| {
        let shape = match &why {
            booleans::Problem::KeptNotClosed => Some(0),
            booleans::Problem::CutterNotClosed(i) | booleans::Problem::CutterFlat(i) => Some(i + 1),
            _ => None,
        };
        ExportError::Merge {
            shape: shape.and_then(|i| shapes.get(i)).map(|o| o.name.clone()),
            why,
        }
    })?;
    let triangles = ExportOptions {
        triangulate: true,
        ..ExportOptions::default()
    };
    Ok(vec![export_mesh(&merged, Mat4::IDENTITY, &triangles)])
}

/// `shape` as a mesh (points shared as in the file, so a closed shape
/// stays closed), for the boolean. Polygons a mesh can't hold (a point
/// twice) are left out.
pub fn to_mesh(shape: &ExportMesh) -> Mesh {
    let mut mesh = Mesh::new();
    let ids: Vec<Option<VertexId>> = shape
        .points
        .iter()
        .map(|p| mesh.add_vertex(*p).ok())
        .collect();
    for polygon in &shape.polygons {
        let corners: Option<Vec<VertexId>> = polygon
            .corners
            .iter()
            .map(|c| ids.get(c.point as usize).copied().flatten())
            .collect();
        if let Some(corners) = corners {
            let _ = mesh.add_face(&corners);
        }
    }
    mesh
}

/// The triangles of `shapes`, each as its three corners: polygons of more
/// than three corners as a fan (an STL export's are triangles already),
/// and triangles naming a point twice, or a point that isn't there, left
/// out.
fn triangles<'a>(shape: &'a ExportMesh) -> impl Iterator<Item = [Vec3; 3]> + 'a {
    shape.polygons.iter().flat_map(move |polygon| {
        let corners = &polygon.corners;
        (1..corners.len().saturating_sub(1)).filter_map(move |i| {
            let index = [corners[0].point, corners[i].point, corners[i + 1].point];
            if index[0] == index[1] || index[1] == index[2] || index[0] == index[2] {
                return None;
            }
            let point = |k: u32| shape.points.get(k as usize).copied();
            Some([point(index[0])?, point(index[1])?, point(index[2])?])
        })
    })
}

/// The normal of the triangle `[a, b, c]`: by the way its corners go
/// round, unit length, or zero for a triangle with no area.
pub fn facet_normal([a, b, c]: [Vec3; 3]) -> Vec3 {
    let n = (b - a).cross(c - a).normalize_or_zero();
    if n.is_finite() { n } else { Vec3::ZERO }
}

/// The header naming `generator`: its bytes, cut to fit, then zeros. A
/// header starting "solid" makes some readers take the file for text, so
/// such a name is put after "binary ".
pub fn header(generator: &str) -> [u8; HEADER_SIZE] {
    let mut out = [0u8; HEADER_SIZE];
    let text = if generator
        .get(..5)
        .is_some_and(|s| s.eq_ignore_ascii_case("solid"))
    {
        format!("binary {generator}")
    } else {
        generator.to_string()
    };
    let bytes = text.as_bytes();
    let n = bytes.len().min(HEADER_SIZE);
    out[..n].copy_from_slice(&bytes[..n]);
    out
}

/// A binary STL file of `shapes`, their points multiplied by `scale`
/// (from centimetres into the print units), naming `generator` in its
/// header; and how many triangles it holds.
pub fn write_binary<'a>(
    shapes: impl Iterator<Item = &'a ExportMesh> + Clone,
    scale: f32,
    generator: &str,
) -> Result<(Vec<u8>, usize), ExportError> {
    let count: usize = shapes.clone().map(|s| triangles(s).count()).sum();
    let count32 = u32::try_from(count).map_err(|_| ExportError::TooBig)?;
    let size = count
        .checked_mul(TRIANGLE_SIZE)
        .and_then(|n| n.checked_add(HEADER_SIZE + 4))
        .ok_or(ExportError::TooBig)?;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(&header(generator));
    out.extend_from_slice(&count32.to_le_bytes());
    let put = |v: Vec3, out: &mut Vec<u8>| {
        for x in v.to_array() {
            out.extend_from_slice(&x.to_le_bytes());
        }
    };
    for shape in shapes {
        for corners in triangles(shape) {
            let corners = corners.map(|p| p * scale);
            put(facet_normal(corners), &mut out);
            for p in corners {
                put(p, &mut out);
            }
            out.extend_from_slice(&0u16.to_le_bytes());
        }
    }
    Ok((out, count))
}

#[cfg(test)]
mod tests;
