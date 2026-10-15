//! Getting a scene ready for an exported file
//! ([decision 0071](../../../docs/decisions/0071-obj-export-and-import.md)):
//! which objects go, their shapes as polygons (or triangles) with the
//! shading normals seen on screen, in the file's units and up axis.
//!
//! Every format writer (OBJ, and glTF later) starts from the
//! [`ExportedObject`]s made here, so they all agree on what is exported.
//!
//! **Space.** The scene is in centimetres with Z up. A file is in metres
//! or centimetres ([`FileUnits`]) with Y or Z up ([`UpAxis`]): Y up turns
//! the scene so its Z becomes the file's Y (a point at (x, y, z) goes to
//! (x, z, -y)), so models arrive standing upright in Godot, Unity and
//! Substance Painter ([decision 0012](../../../docs/decisions/0012-z-up-axes.md)).
//!
//! **Points stay shared.** Each object's points are listed once, and each
//! polygon corner names its point, its normal and its UV (if the shape
//! has UVs), so a smooth surface stays one connected piece in the file.

use glam::{Mat4, Vec2, Vec3};
use mesh::{AttributeElement, FaceId, Hardness, Mesh, Value};
use scene::{Document, ObjectId};
use std::collections::HashMap;

/// The units an exported file's lengths are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum FileUnits {
    /// Metres: what Godot, Unity, glTF and Substance Painter expect.
    #[default]
    Metres,
    /// Centimetres: what Unreal Engine expects in an OBJ.
    Centimetres,
}

impl FileUnits {
    /// How many file units one centimetre is.
    pub fn per_centimetre(self) -> f32 {
        match self {
            Self::Metres => 0.01,
            Self::Centimetres => 1.0,
        }
    }
}

/// Which way is up in an exported file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum UpAxis {
    /// Y up: Godot, Unity, glTF, Substance Painter and most OBJ readers.
    #[default]
    Y,
    /// Z up, as in the app: Blender's and 3ds Max's own axes.
    Z,
}

/// Which objects an export takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ExportWhat {
    /// Every shape that is shown.
    #[default]
    Everything,
    /// The selected objects and the objects inside them, if shown.
    Selected,
}

/// How a scene is exported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportOptions {
    /// Which objects.
    pub what: ExportWhat,
    /// The file's units.
    pub units: FileUnits,
    /// The file's up axis.
    pub up: UpAxis,
    /// Where the shading breaks: as the viewport shows it (decision 0071).
    pub hardness: Hardness,
    /// Whether polygons are cut into triangles, the same way the viewport
    /// cuts them, so a bake matches the game.
    pub triangulate: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            what: ExportWhat::Everything,
            units: FileUnits::Metres,
            up: UpAxis::Y,
            hardness: Hardness::Marked,
            triangulate: false,
        }
    }
}

impl ExportOptions {
    /// The change from the scene's space (centimetres, Z up) to the file's.
    pub fn to_file(&self) -> Mat4 {
        let scale = Mat4::from_scale(Vec3::splat(self.units.per_centimetre()));
        match self.up {
            UpAxis::Z => scale,
            // (x, y, z) → (x, z, -y): a turn of -90° around X.
            UpAxis::Y => Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2) * scale,
        }
    }
}

/// The kinds of file the app exports to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ExportFormat {
    /// Binary glTF (`.glb`): Godot's and Substance Painter's favourite,
    /// with each object at its pivot (decision 0072).
    #[default]
    Glb,
    /// OBJ (`.obj`): plain text, read by almost every 3D program.
    Obj,
}

impl ExportFormat {
    /// Every format, in the order the Export window lists them.
    pub const ALL: [Self; 2] = [Self::Glb, Self::Obj];

    /// Its name, as listed.
    pub fn name(self) -> &'static str {
        match self {
            Self::Glb => "glTF",
            Self::Obj => "OBJ",
        }
    }

    /// Its file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Glb => "glb",
            Self::Obj => "obj",
        }
    }

    /// Whether the format fixes the units (metres), up axis (Y) and
    /// triangles itself, whatever the Export window says: glTF does.
    pub fn fixes_space(self) -> bool {
        self == Self::Glb
    }

    /// `options` as this format writes them: for glTF, metres, Y up and
    /// triangles.
    pub fn options(self, options: &ExportOptions) -> ExportOptions {
        if self.fixes_space() {
            ExportOptions {
                units: FileUnits::Metres,
                up: UpAxis::Y,
                triangulate: true,
                ..*options
            }
        } else {
            *options
        }
    }
}

/// What an export wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExportSummary {
    /// How many objects.
    pub objects: usize,
    /// How many triangles they make in a game engine.
    pub triangles: usize,
}

/// Why an export didn't happen, in plain words.
#[derive(Debug)]
pub enum ExportError {
    /// Nothing to export: no shown shapes (or none selected).
    Nothing,
    /// The file couldn't be written.
    Io(std::io::Error),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nothing => {
                f.write_str("there is nothing to export: no shapes are shown or selected")
            }
            Self::Io(e) => write!(f, "the file couldn't be written ({e})"),
        }
    }
}

impl std::error::Error for ExportError {}

/// The file `document` exports to in `format` with `options`, and what it
/// holds. `generator` names the app in the file.
pub fn export_bytes(
    document: &Document,
    format: ExportFormat,
    options: &ExportOptions,
    generator: &str,
) -> Result<(Vec<u8>, ExportSummary), ExportError> {
    let options = &format.options(options);
    let objects = prepare(document, options);
    if objects.is_empty() {
        return Err(ExportError::Nothing);
    }
    let summary = ExportSummary {
        objects: objects.len(),
        triangles: objects.iter().map(|o| o.world.triangle_count()).sum(),
    };
    let bytes = match format {
        ExportFormat::Glb => crate::gltf::write_glb(&objects, generator),
        ExportFormat::Obj => crate::obj::write_obj(&objects, options, generator).into_bytes(),
    };
    Ok((bytes, summary))
}

/// Exports `document` to the file at `path` (written safely, never half
/// done; see [`crate::project::safe_write`]).
pub fn export_to(
    path: &std::path::Path,
    document: &Document,
    format: ExportFormat,
    options: &ExportOptions,
    generator: &str,
) -> Result<ExportSummary, ExportError> {
    let (bytes, summary) = export_bytes(document, format, options, generator)?;
    crate::project::safe_write(path, &bytes).map_err(ExportError::Io)?;
    Ok(summary)
}

/// One corner of an exported polygon: indices into its object's lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExportCorner {
    /// Its point, in [`ExportMesh::points`].
    pub point: u32,
    /// Its shading normal, in [`ExportMesh::normals`].
    pub normal: u32,
    /// Its UV, in [`ExportMesh::uvs`], when the shape has UVs.
    pub uv: Option<u32>,
}

/// One exported polygon (or triangle).
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPolygon {
    /// Its corners, counter-clockwise seen from outside.
    pub corners: Vec<ExportCorner>,
    /// Its material slot (0 until material slots arrive).
    pub material: u32,
}

/// An object's shape, ready to write.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportMesh {
    /// Each point once, in the file's space.
    pub points: Vec<Vec3>,
    /// Each distinct shading normal once, unit length, in the file's space.
    pub normals: Vec<Vec3>,
    /// Each distinct UV once (empty without UVs).
    pub uvs: Vec<Vec2>,
    /// The polygons.
    pub polygons: Vec<ExportPolygon>,
}

impl ExportMesh {
    /// How many triangles it makes in a game engine.
    pub fn triangle_count(&self) -> usize {
        self.polygons
            .iter()
            .map(|p| p.corners.len().saturating_sub(2))
            .sum()
    }
}

/// One object, ready to write.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportedObject {
    /// Its id in the scene.
    pub id: ObjectId,
    /// Its name.
    pub name: String,
    /// Where its pivot is, in the file's space.
    pub pivot: Mat4,
    /// Its shape, in the world of the file (pivot and all applied).
    pub world: ExportMesh,
    /// Its shape around its pivot, in the file's units and axes (for
    /// formats that keep each object's place apart: glTF).
    pub local: ExportMesh,
}

/// The objects of `document` to export with `options`, in the Objects
/// panel's order: each shown object with a shape (primitives, meshes and
/// instances), with its shape as it is now (every edit applied).
pub fn prepare(document: &Document, options: &ExportOptions) -> Vec<ExportedObject> {
    let scene = document.scene();
    let selection = document.selection();
    let to_file = options.to_file();
    let from_file = to_file.inverse();
    scene
        .walk()
        .into_iter()
        .filter(|(id, _)| scene.is_shown(*id))
        .filter(|(id, _)| match options.what {
            ExportWhat::Everything => true,
            ExportWhat::Selected => selection
                .iter()
                .any(|s| *s == *id || scene.is_inside(*id, *s)),
        })
        .filter_map(|(id, _)| {
            let object = scene.object(id)?;
            let shape = scene.evaluated(id)?.mesh;
            let pivot = to_file * scene.world_matrix(id) * from_file;
            let world = to_file * scene.shape_matrix(id);
            let local = to_file * object.offset.matrix();
            Some(ExportedObject {
                id,
                name: object.name.clone(),
                pivot,
                world: export_mesh(&shape, world, options),
                local: export_mesh(&shape, local, options),
            })
        })
        .collect()
}

/// Bits of a number, so equal normals and UVs are listed once.
fn key3(v: Vec3) -> [u32; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

/// `mesh` placed by `matrix`, as polygons (or triangles) with shading
/// normals (worked out with `options.hardness`) and UVs.
pub fn export_mesh(mesh: &Mesh, matrix: Mat4, options: &ExportOptions) -> ExportMesh {
    // A mirrored placement turns polygons inside out: run them backwards.
    let flipped = matrix.determinant() < 0.0;
    let normal_matrix = matrix.inverse().transpose();
    let mut out = ExportMesh::default();
    let mut point_index: HashMap<u32, u32> = HashMap::new();
    for (v, p) in mesh.vertices() {
        point_index.insert(v.get(), out.points.len() as u32);
        out.points.push(matrix.transform_point3(p));
    }
    let (_, corner_normals) = mesh::corner_shading_with(mesh, options.hardness);
    let (starts, corners) = mesh.face_layout();
    let has_uv = mesh.attributes().any(|(name, domain, kind)| {
        name == mesh::attributes::UV
            && domain == mesh::Domain::Corner
            && kind == mesh::AttributeKind::Vec2
    });
    let mut normal_index: HashMap<[u32; 3], u32> = HashMap::new();
    let mut uv_index: HashMap<[u32; 2], u32> = HashMap::new();
    for (f, pair) in starts.windows(2).enumerate() {
        let (a, b) = (pair[0] as usize, pair[1] as usize);
        if b <= a + 2 {
            continue;
        }
        let face = FaceId::from_raw(f as u32);
        if mesh.face(face).is_none() {
            continue;
        }
        let mut all = Vec::with_capacity(b - a);
        for (k, v) in corners.iter().enumerate().take(b).skip(a) {
            let Some(point) = point_index.get(&v.get()).copied() else {
                break;
            };
            let n = normal_matrix
                .transform_vector3(corner_normals.get(k).copied().unwrap_or(Vec3::Z))
                .normalize_or(Vec3::Y);
            let next = out.normals.len() as u32;
            let normal = *normal_index.entry(key3(n)).or_insert(next);
            if normal == next {
                out.normals.push(n);
            }
            let uv = has_uv.then(|| {
                let uv = match mesh
                    .attribute(mesh::attributes::UV, AttributeElement::Corner(face, k - a))
                {
                    Some(Value::Vec2(uv)) => uv,
                    _ => Vec2::ZERO,
                };
                let next = out.uvs.len() as u32;
                let index = *uv_index
                    .entry([uv.x.to_bits(), uv.y.to_bits()])
                    .or_insert(next);
                if index == next {
                    out.uvs.push(uv);
                }
                index
            });
            all.push(ExportCorner { point, normal, uv });
        }
        if all.len() != b - a {
            continue;
        }
        let mut push = |mut corners: Vec<ExportCorner>| {
            if flipped {
                corners.reverse();
            }
            out.polygons.push(ExportPolygon {
                corners,
                material: 0,
            });
        };
        if options.triangulate && all.len() > 3 {
            mesh::face_triangles(mesh, &corners[a..b], |[i, j, k]| {
                push(vec![all[i], all[j], all[k]]);
            });
        } else {
            push(all);
        }
    }
    out
}

#[cfg(test)]
mod tests;
