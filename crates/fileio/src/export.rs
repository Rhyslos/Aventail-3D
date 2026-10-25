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
//!
//! **Materials** ([decision 0074](../../../docs/decisions/0074-material-slots.md)).
//! Each polygon carries its object's material slot; each object carries
//! its list of materials (an instance its original's). In a file,
//! materials of the same name are one material ([`file_materials`]).

pub(crate) use collision::part_objects;
use glam::{Mat4, Vec2, Vec3};
pub use lods::{ExportLevel, LevelMark};
pub(crate) use lods::{level_objects, marks as level_marks};
use mesh::{AttributeElement, FaceId, Hardness, Mesh, Value};
use scene::{Document, Material, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use words::{text, tr, trf};

/// The units an exported file's lengths are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum UpAxis {
    /// Y up: Godot, Unity, glTF, Substance Painter and most OBJ readers.
    #[default]
    Y,
    /// Z up, as in the app: Blender's and 3ds Max's own axes.
    Z,
}

/// Which objects an export takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
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
    /// Where the shading breaks for every object, or (`None`, the
    /// standard) each object's own smoothing, as the viewport shows it
    /// (decisions 0071 and 0126).
    pub hardness: Option<Hardness>,
    /// Whether polygons are cut into triangles, the same way the viewport
    /// cuts them, so a bake matches the game.
    pub triangulate: bool,
    /// Whether objects' names carry Godot's collision hints (`-col`,
    /// `-convcol`, and a `-convcolonly` box; decision 0078). glTF only.
    pub godot_names: bool,
    /// Whether objects with levels of detail get each level beside them
    /// as an object of its own, `name_LOD1` and on (decision 0155).
    pub lod_levels: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            what: ExportWhat::Everything,
            units: FileUnits::Metres,
            up: UpAxis::Y,
            hardness: None,
            triangulate: false,
            godot_names: false,
            lod_levels: false,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
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

/// Ready-made export settings for where a model is going
/// ([decision 0075](../../../docs/decisions/0075-export-presets.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportPreset {
    /// Godot: glTF, each object at its pivot (metres, Y up, triangles).
    Godot,
    /// Unity: glTF too, read through Unity's glTFast package.
    Unity,
    /// Unreal Engine: glTF too (Unreal turns metres into its centimetres).
    UnrealEngine,
    /// Solis, the game engine made beside this app: glTF, the way Godot
    /// gets it but without Godot's collision names (decision 0122).
    Solis,
    /// Substance Painter: OBJ in metres, Y up, cut into triangles the
    /// viewport's way, its materials becoming texture sets.
    SubstancePainter,
}

/// The settings a preset chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetSettings {
    /// The kind of file.
    pub format: ExportFormat,
    /// The file's units.
    pub units: FileUnits,
    /// The file's up axis.
    pub up: UpAxis,
    /// Whether polygons are cut into triangles.
    pub triangulate: bool,
    /// Whether names carry Godot's collision hints.
    pub godot_names: bool,
}

impl ExportPreset {
    /// Every preset, in the order the Export window lists them.
    pub const ALL: [Self; 5] = [
        Self::Godot,
        Self::Unity,
        Self::UnrealEngine,
        Self::Solis,
        Self::SubstancePainter,
    ];

    /// Its name, as listed.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Godot => text!("Godot"),
            Self::Unity => text!("Unity"),
            Self::UnrealEngine => text!("Unreal Engine"),
            Self::Solis => text!("Solis"),
            Self::SubstancePainter => text!("Substance Painter"),
        })
    }

    /// What it is for and what it chooses, in a sentence.
    pub fn description(self) -> &'static str {
        tr(match self {
            Self::Godot => text!(
                "glTF, each object at its pivot, in metres with Y up: drop it into the Godot \
                 project and it imports by itself"
            ),
            Self::Unity => text!(
                "glTF, each object at its pivot, in metres with Y up: Unity reads it once its \
                 glTFast package is installed"
            ),
            Self::UnrealEngine => text!(
                "glTF, each object at its pivot, in metres with Y up: Unreal Engine 5 reads it \
                 and turns it into its centimetres and Z up"
            ),
            Self::Solis => text!(
                "glTF, each object at its pivot, in metres with Y up: what the Solis game \
                 engine imports"
            ),
            Self::SubstancePainter => text!(
                "OBJ in metres with Y up, cut into the viewport's triangles so the bake \
                 matches the game; each material becomes a texture set"
            ),
        })
    }

    /// Whether it is for a game engine (not for a texturing program).
    pub fn is_engine(self) -> bool {
        self != Self::SubstancePainter
    }

    /// The settings it chooses.
    pub fn settings(self) -> PresetSettings {
        match self {
            Self::Godot | Self::Unity | Self::UnrealEngine | Self::Solis => PresetSettings {
                format: ExportFormat::Glb,
                units: FileUnits::Metres,
                up: UpAxis::Y,
                triangulate: true,
                godot_names: self == Self::Godot,
            },
            Self::SubstancePainter => PresetSettings {
                format: ExportFormat::Obj,
                units: FileUnits::Metres,
                up: UpAxis::Y,
                triangulate: true,
                godot_names: false,
            },
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
    /// How many materials (by name) they use.
    pub materials: usize,
}

/// The files an export makes: the one asked for, and any that go beside
/// it (an OBJ's `.mtl` file of materials).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportFiles {
    /// The file asked for.
    pub main: Vec<u8>,
    /// Files beside it: each one's name and bytes.
    pub beside: Vec<(String, Vec<u8>)>,
    /// What they hold.
    pub summary: ExportSummary,
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
            Self::Nothing => f.write_str(tr(
                "there is nothing to export: no shapes are shown or selected",
            )),
            Self::Io(e) => f.write_str(&trf!("the file couldn't be written ({})", e)),
        }
    }
}

impl std::error::Error for ExportError {}

/// The files `document` exports to in `format` with `options`, and what
/// they hold. `file_name` is the name of the file asked for (an OBJ names
/// its `.mtl` file after it); `generator` names the app in the file.
pub fn export_bytes(
    document: &Document,
    format: ExportFormat,
    options: &ExportOptions,
    generator: &str,
    file_name: &str,
) -> Result<ExportFiles, ExportError> {
    let options = &format.options(options);
    let objects = prepare(document, options);
    write_files(&objects, format, options, generator, file_name)
}

/// The files `objects` (made with `options`) export to in `format`: the
/// one asked for, called `file_name`, and any beside it.
pub fn write_files(
    objects: &[ExportedObject],
    format: ExportFormat,
    options: &ExportOptions,
    generator: &str,
    file_name: &str,
) -> Result<ExportFiles, ExportError> {
    if objects.is_empty() {
        return Err(ExportError::Nothing);
    }
    let summary = ExportSummary {
        objects: objects.len(),
        triangles: objects.iter().map(|o| o.world.triangle_count()).sum(),
        materials: file_materials(objects).len(),
    };
    let files = match format {
        ExportFormat::Glb => ExportFiles {
            main: crate::gltf::write_glb(objects, generator, options.godot_names),
            beside: Vec::new(),
            summary,
        },
        ExportFormat::Obj => {
            let stem = std::path::Path::new(file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("materials");
            let mtl = format!("{stem}.mtl");
            // Levels of detail as objects of their own (decision 0155).
            let levels = level_objects(objects);
            let with_levels: Vec<ExportedObject>;
            let objects = if levels.is_empty() {
                objects
            } else {
                with_levels = objects
                    .iter()
                    .cloned()
                    .chain(levels.into_iter().map(|(o, _)| o))
                    .collect();
                &with_levels
            };
            ExportFiles {
                main: crate::obj::write_obj(objects, options, generator, Some(&mtl)).into_bytes(),
                beside: vec![(mtl, crate::obj::write_mtl(objects, generator).into_bytes())],
                summary,
            }
        }
    };
    Ok(files)
}

/// Exports `document` to the file at `path`, and any files that go
/// beside it (each written safely, never half done; see
/// [`crate::project::safe_write`]).
pub fn export_to(
    path: &std::path::Path,
    document: &Document,
    format: ExportFormat,
    options: &ExportOptions,
    generator: &str,
) -> Result<ExportSummary, ExportError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let files = export_bytes(document, format, options, generator, name)?;
    for (name, bytes) in &files.beside {
        crate::project::safe_write(&path.with_file_name(name), bytes).map_err(ExportError::Io)?;
    }
    crate::project::safe_write(path, &files.main).map_err(ExportError::Io)?;
    Ok(files.summary)
}

/// The materials `objects` use, each name once, in the order they are
/// first used. Two materials of the same name are one material in a file
/// (the first one's colour wins).
pub fn file_materials(objects: &[ExportedObject]) -> Vec<Material> {
    let mut out: Vec<Material> = Vec::new();
    for object in objects {
        let mut used = vec![false; object.materials.len()];
        for polygon in &object.world.polygons {
            if let Some(u) = used.get_mut(polygon.material as usize) {
                *u = true;
            }
        }
        for (material, used) in object.materials.iter().zip(used) {
            if used && !out.iter().any(|m| m.name == material.name) {
                out.push(material.clone());
            }
        }
    }
    out
}

/// The index in [`file_materials`] of `polygon`'s material, of `object`.
pub fn file_material(files: &[Material], object: &ExportedObject, polygon: &ExportPolygon) -> u32 {
    object
        .materials
        .get(polygon.material as usize)
        .and_then(|m| files.iter().position(|f| f.name == m.name))
        .unwrap_or(0) as u32
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
    /// Its material slot: an index into its object's
    /// [`ExportedObject::materials`].
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
    /// Its materials, which its polygons' slots point into (never
    /// empty: the standard material when it has none of its own).
    pub materials: Vec<Material>,
    /// The collision shape a game engine makes for it (decision 0078).
    pub collision: scene::Collision,
    /// The hulls of its collision in parts (decision 0151), around its
    /// pivot in the file's units and axes as `local`: only for an object
    /// wrapped in parts, in an export with Godot's names.
    pub collision_parts: Vec<ExportMesh>,
    /// Its levels of detail (decision 0155), simplest last: only for an
    /// object with them, in an export that asks for them.
    pub levels: Vec<ExportLevel>,
}

/// The objects of `document` to export with `options`, in the Objects
/// panel's order: each shown object with a shape (primitives, meshes and
/// instances), with its shape as it is now (every edit applied).
pub fn prepare(document: &Document, options: &ExportOptions) -> Vec<ExportedObject> {
    let scene = document.scene();
    let selection = document.selection();
    // Cutters go out as part of what they cut (decision 0135).
    let cutters = scene.cutters();
    scene
        .walk()
        .into_iter()
        .filter(|(id, _)| scene.is_shown(*id) && !cutters.contains(id))
        .filter(|(id, _)| match options.what {
            ExportWhat::Everything => true,
            ExportWhat::Selected => selection
                .iter()
                .any(|s| *s == *id || scene.is_inside(*id, *s)),
        })
        .filter_map(|(id, _)| exported(scene, id, Mat4::IDENTITY, options))
        .collect()
}

/// What an export of `document` with `options` would hold, worked out
/// without making the file: the Export window shows it before exporting.
/// The same numbers as [`ExportFiles::summary`] for the same export.
pub fn summary(document: &Document, options: &ExportOptions) -> ExportSummary {
    let scene = document.scene();
    let selection = document.selection();
    let mut objects = 0;
    let mut triangles = 0;
    let mut materials: Vec<String> = Vec::new();
    let cutters = scene.cutters();
    for (id, _) in scene.walk() {
        if !scene.is_shown(id) || cutters.contains(&id) {
            continue;
        }
        if options.what == ExportWhat::Selected
            && !selection
                .iter()
                .any(|s| *s == id || scene.is_inside(id, *s))
        {
            continue;
        }
        let Some(shape) = scene.exported_shape(id) else {
            continue;
        };
        objects += 1;
        let list = scene.material_list(id);
        let (starts, _) = shape.face_layout();
        let mut used = vec![false; list.len()];
        for (f, pair) in starts.windows(2).enumerate() {
            let corners = (pair[1] - pair[0]) as usize;
            let face = FaceId::from_raw(f as u32);
            if corners < 3 || shape.face(face).is_none() {
                continue;
            }
            triangles += corners - 2;
            let slot = mesh::material::slot_of(&shape, face) as usize;
            // A slot with no material in the list uses the first.
            let slot = if slot < list.len() { slot } else { 0 };
            if let Some(u) = used.get_mut(slot) {
                *u = true;
            }
        }
        for (material, used) in list.iter().zip(used) {
            if used && !materials.contains(&material.name) {
                materials.push(material.name.clone());
            }
        }
    }
    ExportSummary {
        objects,
        triangles,
        materials: materials.len(),
    }
}

/// The objects of one kit piece ([decision 0078](../../../docs/decisions/0078-kit-export.md)):
/// `root` and the shown objects inside it, moved so `root`'s pivot is at
/// the file's origin (turned and sized as in the scene).
pub fn prepare_piece(
    document: &Document,
    root: ObjectId,
    options: &ExportOptions,
) -> Vec<ExportedObject> {
    let scene = document.scene();
    let at = scene.world_matrix(root).w_axis.truncate();
    let shift = Mat4::from_translation(-at);
    let cutters = scene.cutters();
    scene
        .walk()
        .into_iter()
        .filter(|(id, _)| *id == root || scene.is_inside(*id, root))
        .filter(|(id, _)| scene.is_shown(*id) && !cutters.contains(id))
        .filter_map(|(id, _)| exported(scene, id, shift, options))
        .collect()
}

/// Object `id` ready to write, its place in the world moved by `shift`
/// first; `None` without a shape.
fn exported(
    scene: &scene::Scene,
    id: ObjectId,
    shift: Mat4,
    options: &ExportOptions,
) -> Option<ExportedObject> {
    let to_file = options.to_file();
    let from_file = to_file.inverse();
    let object = scene.object(id)?;
    // With its live mirror's copies (decision 0088) and its modifiers
    // (decision 0142): one mesh.
    let shape = scene.exported_shape(id)?;
    let pivot = to_file * shift * scene.world_matrix(id) * from_file;
    let world = to_file * shift * scene.shape_matrix(id);
    let local = to_file * object.offset.matrix();
    // An instance shows its original's materials.
    let materials = scene.material_list(id);
    // Shaded by the object's own smoothing (an instance's original's),
    // unless the options set one for every object.
    let own = match &object.kind {
        scene::ObjectKind::Instance(source) => *source,
        _ => id,
    };
    let hardness = options
        .hardness
        .or_else(|| scene.object(own).map(|o| o.hardness))
        .unwrap_or(Hardness::Marked);
    let shading = ExportOptions {
        hardness: Some(hardness),
        ..*options
    };
    let collision_parts = if options.godot_names {
        collision::part_hulls(scene, id, &shape, local)
    } else {
        Vec::new()
    };
    let mut levels = lods::levels(scene, id, &shape, (world, local), &shading);
    let mut world = export_mesh(&shape, world, &shading);
    let mut local = export_mesh(&shape, local, &shading);
    // A slot with no material in the list uses the first.
    let meshes = levels
        .iter_mut()
        .flat_map(|l| [&mut l.world, &mut l.local])
        .chain([&mut world, &mut local]);
    for p in meshes.flat_map(|m| m.polygons.iter_mut()) {
        if p.material as usize >= materials.len() {
            p.material = 0;
        }
    }
    Some(ExportedObject {
        id,
        name: object.name.clone(),
        pivot,
        world,
        local,
        materials,
        collision: object.collision,
        collision_parts,
        levels,
    })
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
    let (_, corner_normals) =
        mesh::corner_shading_with(mesh, options.hardness.unwrap_or(Hardness::Marked));
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
        let material = mesh::material::slot_of(mesh, face);
        let mut push = |mut corners: Vec<ExportCorner>| {
            if flipped {
                corners.reverse();
            }
            out.polygons.push(ExportPolygon { corners, material });
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
mod axis_deform_tests;
mod collision;
#[cfg(test)]
mod displace_tests;
#[cfg(test)]
mod game_bevel_tests;
mod lods;
#[cfg(test)]
mod lods_tests;
#[cfg(test)]
mod normals_tests;
#[cfg(test)]
mod struts_tests;
#[cfg(test)]
mod tests;
