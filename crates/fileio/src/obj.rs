//! OBJ files: writing exported objects, and reading them back as plain
//! meshes ([decision 0071](../../../docs/decisions/0071-obj-export-and-import.md)).
//!
//! OBJ is a plain text format almost every 3D program reads: `v` lines
//! for points, `vt` for UVs, `vn` for normals, `f` for polygons (each
//! corner `point/uv/normal`, counted from 1 across the whole file), and
//! `o` to start a named object. `usemtl` names the material of the
//! polygons after it, and `mtllib` the `.mtl` file beside it that
//! describes each material ([decision 0074](../../../docs/decisions/0074-material-slots.md)).
//!
//! **Writing** keeps each object's points shared between its polygons and
//! gives every corner its shading normal, so hard and soft edges arrive as
//! they look on screen.
//!
//! **Reading** never trusts the file: lines it doesn't know are skipped
//! and counted, broken polygons are left out and counted, numbers must be
//! real, and the counts are held to the limits rulebook
//! ([`crate::limits`]). Each object (`o`, or `g` when there are no `o`
//! lines) becomes a plain mesh. Where the file gives two polygons
//! different normals at the points of an edge they share, that edge is
//! made hard, so a model shaded crisp in another program stays crisp.
//! Each `usemtl` name an object uses becomes one of its materials, its
//! polygons given that slot; colours come from the `.mtl` file when it is
//! there ([`read_mtl`], [`ObjImport::colour_materials`]).

use crate::export::{
    ExportOptions, ExportedObject, FileUnits, UpAxis, file_material, file_materials,
};
use glam::{Mat4, Vec2, Vec3};
use mesh::{AttributeElement, Domain, EdgeKey, FaceId, Mesh, Value, VertexId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use words::{tr, trf};

/// A number as OBJ writes it: the shortest form that reads back exactly,
/// with no "-0".
fn number(x: f32) -> String {
    if x == 0.0 {
        "0".to_string()
    } else {
        format!("{x}")
    }
}

/// A name as an OBJ line can hold it: on one line, without leading or
/// trailing space ("Object" when nothing is left).
fn clean_name(name: &str) -> String {
    let one_line: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = one_line.trim();
    if trimmed.is_empty() {
        crate::UNNAMED_IN_FILES.to_string()
    } else {
        trimmed.to_string()
    }
}

/// The text of an OBJ file holding `objects` (made with `options`), with
/// a first comment line naming `generator`. `mtl` names the `.mtl` file
/// of its materials, if one is written beside it ([`write_mtl`]).
pub fn write_obj(
    objects: &[ExportedObject],
    options: &ExportOptions,
    generator: &str,
    mtl: Option<&str>,
) -> String {
    let mut text = String::new();
    let units = match options.units {
        FileUnits::Metres => "metres",
        FileUnits::Centimetres => "centimetres",
    };
    let up = match options.up {
        UpAxis::Y => "Y",
        UpAxis::Z => "Z",
    };
    let _ = writeln!(text, "# {}", clean_name(generator));
    let _ = writeln!(text, "# Units: {units}, {up} up");
    if let Some(mtl) = mtl {
        let _ = writeln!(text, "mtllib {}", clean_name(mtl));
    }
    let materials = file_materials(objects);
    // Indices count from 1 across the whole file.
    let (mut points, mut uvs, mut normals) = (1usize, 1usize, 1usize);
    for object in objects {
        let shape = &object.world;
        let _ = writeln!(text, "o {}", clean_name(&object.name));
        for p in &shape.points {
            let _ = writeln!(text, "v {} {} {}", number(p.x), number(p.y), number(p.z));
        }
        for t in &shape.uvs {
            let _ = writeln!(text, "vt {} {}", number(t.x), number(t.y));
        }
        for n in &shape.normals {
            let _ = writeln!(text, "vn {} {} {}", number(n.x), number(n.y), number(n.z));
        }
        let mut current = None;
        for polygon in &shape.polygons {
            let material = file_material(&materials, object, polygon);
            if current != Some(material) {
                if let Some(m) = materials.get(material as usize) {
                    let _ = writeln!(text, "usemtl {}", material_name(&m.name));
                }
                current = Some(material);
            }
            text.push('f');
            for c in &polygon.corners {
                let v = points + c.point as usize;
                let n = normals + c.normal as usize;
                let _ = match c.uv {
                    Some(t) => write!(text, " {v}/{}/{n}", uvs + t as usize),
                    None => write!(text, " {v}//{n}"),
                };
            }
            text.push('\n');
        }
        points += shape.points.len();
        uvs += shape.uvs.len();
        normals += shape.normals.len();
    }
    text
}

/// A material's name as OBJ and MTL files hold it: one word (spaces
/// become underscores), as most programs read only the first word.
fn material_name(name: &str) -> String {
    clean_name(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

/// The text of the `.mtl` file describing the materials `objects` use:
/// for each, its name and colour (a plain, matte surface). A first
/// comment line names `generator`.
pub fn write_mtl(objects: &[ExportedObject], generator: &str) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "# {}", clean_name(generator));
    for material in file_materials(objects) {
        let [r, g, b] = material.color.map(number);
        let _ = writeln!(text);
        let _ = writeln!(text, "newmtl {}", material_name(&material.name));
        let _ = writeln!(text, "Ka 0 0 0");
        let _ = writeln!(text, "Kd {r} {g} {b}");
        let _ = writeln!(text, "Ks 0 0 0");
        let _ = writeln!(text, "d 1");
        let _ = writeln!(text, "illum 1");
    }
    text
}

/// One object read from an OBJ file.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedObject {
    /// Its name in the file (or "Object" when it had none).
    pub name: String,
    /// Its shape, in the scene's space (centimetres, Z up), placed as the
    /// file placed it.
    pub mesh: Mesh,
    /// Its materials, in the order it first uses them (empty when the
    /// file names none for it). Each is light grey until coloured from
    /// the `.mtl` file.
    pub materials: Vec<scene::Material>,
}

/// What reading an OBJ file gave.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjImport {
    /// The objects, in the file's order.
    pub objects: Vec<ImportedObject>,
    /// Lines of kinds the app doesn't read (curves, groups of settings).
    pub skipped_lines: usize,
    /// Polygons left out: fewer than three different points, or a point,
    /// UV or normal that isn't in the file.
    pub skipped_polygons: usize,
    /// The `.mtl` files the file names (`mtllib`), as written.
    pub material_files: Vec<String>,
}

impl ObjImport {
    /// Gives the objects' materials the colours `colours` (from
    /// [`read_mtl`]) has for their names.
    pub fn colour_materials(&mut self, colours: &HashMap<String, [f32; 3]>) {
        for object in &mut self.objects {
            for material in &mut object.materials {
                if let Some(c) = colours.get(&material.name) {
                    material.color = *c;
                }
            }
        }
    }
}

impl ObjImport {
    /// Colours the materials from the `.mtl` files the OBJ names, looked
    /// for in `folder` (the OBJ's own folder) by file name only, so a
    /// file can't point anywhere else. Files that are missing, unreadable
    /// or too large are passed over: the materials stay grey.
    pub fn colour_from_files(&mut self, folder: &std::path::Path) {
        let mut colours = HashMap::new();
        for name in &self.material_files {
            let Some(file) = std::path::Path::new(name.trim()).file_name() else {
                continue;
            };
            let path = folder.join(file);
            let small = std::fs::metadata(&path)
                .is_ok_and(|m| m.is_file() && m.len() <= crate::limits::MAX_UNPACKED);
            if !small {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                for (name, colour) in read_mtl(&String::from_utf8_lossy(&bytes)) {
                    colours.entry(name).or_insert(colour);
                }
            }
        }
        self.colour_materials(&colours);
    }
}

/// The colours (`Kd`) a `.mtl` file gives its materials, by name. Lines
/// it doesn't know, and colours that aren't real numbers, are skipped;
/// colours are held to 0 to 1.
pub fn read_mtl(text: &str) -> HashMap<String, [f32; 3]> {
    let mut colours = HashMap::new();
    let mut current: Option<String> = None;
    for raw in text.lines().take(crate::limits::MAX_POLYGONS) {
        let content = raw.split('#').next().unwrap_or_default();
        let tokens: Vec<&str> = content.split_whitespace().collect();
        match tokens.as_slice() {
            ["newmtl", name, ..] => current = Some(clean_name(name)),
            [key, r, g, b, ..] if *key == "Kd" => {
                let parsed: Option<Vec<f32>> = [r, g, b]
                    .iter()
                    .map(|t| t.parse::<f32>().ok().filter(|v| v.is_finite()))
                    .collect();
                if let (Some(name), Some(c)) = (&current, parsed) {
                    colours.insert(
                        name.clone(),
                        [
                            c[0].clamp(0.0, 1.0),
                            c[1].clamp(0.0, 1.0),
                            c[2].clamp(0.0, 1.0),
                        ],
                    );
                }
            }
            _ => {}
        }
        if colours.len() > crate::limits::MAX_OBJECTS {
            break;
        }
    }
    colours
}

/// Why an OBJ file couldn't be read, in plain words.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjError {
    /// It holds nothing to make a shape from.
    Empty,
    /// A number on a line isn't a real number.
    BadNumber {
        /// The line, counted from 1.
        line: usize,
    },
    /// It holds more than the app can take.
    TooLarge,
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str(tr("it holds no polygons")),
            Self::BadNumber { line } => {
                f.write_str(&trf!("line {} holds a number that isn't real", line))
            }
            Self::TooLarge => f.write_str(tr(crate::limits::TOO_MUCH)),
        }
    }
}

impl std::error::Error for ObjError {}

/// A polygon as the file gives it: per corner, indices into the file's
/// points, UVs and normals (from 0).
struct FilePolygon {
    corners: Vec<(usize, Option<usize>, Option<usize>)>,
    /// Its material's name (`usemtl`), if the file gave one.
    material: Option<usize>,
}

/// An object being read.
struct FileObject {
    name: String,
    polygons: Vec<FilePolygon>,
}

/// An index on an `f` line: from 1, or from the end when negative.
fn index(token: &str, count: usize) -> Option<usize> {
    let i: i64 = token.parse().ok()?;
    let count = i64::try_from(count).ok()?;
    let at = match i {
        i if i > 0 => i - 1,
        i if i < 0 => count + i,
        _ => return None,
    };
    (0..count).contains(&at).then_some(at as usize)
}

/// The units and up axis a file says it uses, in the comment this app
/// writes near its top ("# Units: metres, Y up").
pub fn noted_units(text: &str) -> Option<(FileUnits, UpAxis)> {
    let line = text
        .lines()
        .take(10)
        .find_map(|l| l.trim().strip_prefix("# Units:"))?;
    let words: Vec<String> = line
        .split([',', ' '])
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let units = match words.first()?.as_str() {
        "metres" => FileUnits::Metres,
        "centimetres" => FileUnits::Centimetres,
        _ => return None,
    };
    let up = match words.get(1)?.as_str() {
        "y" => UpAxis::Y,
        "z" => UpAxis::Z,
        _ => return None,
    };
    Some((units, up))
}

/// Reads the OBJ file `text`, written in `units` with `up` up (most OBJ
/// files are metres, Y up), unless the file says otherwise
/// ([`noted_units`]).
pub fn read_obj(text: &str, units: FileUnits, up: UpAxis) -> Result<ObjImport, ObjError> {
    let (units, up) = noted_units(text).unwrap_or((units, up));
    let options = ExportOptions {
        units,
        up,
        ..ExportOptions::default()
    };
    let from_file: Mat4 = options.to_file().inverse();
    let mut points: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<Vec2> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut objects: Vec<FileObject> = Vec::new();
    let mut out = ObjImport::default();
    let mut corners_total = 0usize;
    // The material names `usemtl` lines give, and the one in use.
    let mut material_names: Vec<String> = Vec::new();
    let mut material: Option<usize> = None;
    // Objects start at `o` lines; in a file without any, at `g` lines.
    let splits = if text.lines().any(|l| l.trim_start().starts_with("o ")) {
        "o"
    } else {
        "g"
    };
    let numbers = |rest: &[&str], n: usize, line: usize| -> Result<Vec<f32>, ObjError> {
        let values: Vec<f32> = rest
            .iter()
            .take(n)
            .map(|t| t.parse::<f32>())
            .collect::<Result<_, _>>()
            .map_err(|_| ObjError::BadNumber { line })?;
        if values.len() < n.min(2) || values.iter().any(|v| !v.is_finite()) {
            return Err(ObjError::BadNumber { line });
        }
        Ok(values)
    };
    for (number, raw) in text.lines().enumerate() {
        let line = number + 1;
        let content = raw.split('#').next().unwrap_or_default();
        let tokens: Vec<&str> = content.split_whitespace().collect();
        let Some((&kind, rest)) = tokens.split_first() else {
            continue;
        };
        match kind {
            "v" => {
                let v = numbers(rest, 3, line)?;
                let get = |i: usize| v.get(i).copied().unwrap_or(0.0);
                points.push(from_file.transform_point3(Vec3::new(get(0), get(1), get(2))));
            }
            "vt" => {
                let t = numbers(rest, 2, line)?;
                uvs.push(Vec2::new(t[0], t.get(1).copied().unwrap_or(0.0)));
            }
            "vn" => {
                let n = numbers(rest, 3, line)?;
                let get = |i: usize| n.get(i).copied().unwrap_or(0.0);
                normals.push(
                    from_file
                        .transform_vector3(Vec3::new(get(0), get(1), get(2)))
                        .normalize_or_zero(),
                );
            }
            "o" | "g" if kind == splits => {
                let name = clean_name(&rest.join(" "));
                match objects.last_mut() {
                    // Nothing in the last one yet: it is this one.
                    Some(last) if last.polygons.is_empty() => last.name = name,
                    _ => objects.push(FileObject {
                        name,
                        polygons: Vec::new(),
                    }),
                }
            }
            // A `g` inside `o` objects names a part of one, nothing more.
            "usemtl" => {
                let name = clean_name(&rest.join(" "));
                material = Some(match material_names.iter().position(|n| *n == name) {
                    Some(at) => at,
                    None => {
                        material_names.push(name);
                        material_names.len() - 1
                    }
                });
            }
            "mtllib" => {
                let name = rest.join(" ");
                if !name.is_empty() && out.material_files.len() < 16 {
                    out.material_files.push(name);
                }
            }
            "o" | "g" | "s" => {}
            "f" => {
                let mut corners = Vec::with_capacity(rest.len());
                let mut broken = rest.len() < 3;
                for token in rest {
                    let mut parts = token.split('/');
                    let v = parts.next().and_then(|t| index(t, points.len()));
                    let t = match parts.next() {
                        Some("") | None => Some(None),
                        Some(t) => index(t, uvs.len()).map(Some),
                    };
                    let n = match parts.next() {
                        Some("") | None => Some(None),
                        Some(n) => index(n, normals.len()).map(Some),
                    };
                    match (v, t, n) {
                        (Some(v), Some(t), Some(n)) => corners.push((v, t, n)),
                        _ => broken = true,
                    }
                }
                if broken {
                    out.skipped_polygons += 1;
                    continue;
                }
                corners_total += corners.len();
                if corners_total > crate::limits::MAX_POLYGONS * 4 {
                    return Err(ObjError::TooLarge);
                }
                if objects.is_empty() {
                    objects.push(FileObject {
                        name: String::new(),
                        polygons: Vec::new(),
                    });
                }
                if let Some(object) = objects.last_mut() {
                    object.polygons.push(FilePolygon { corners, material });
                }
            }
            _ => out.skipped_lines += 1,
        }
        if points.len() > crate::limits::MAX_POLYGONS * 4
            || objects.len() > crate::limits::MAX_OBJECTS
        {
            return Err(ObjError::TooLarge);
        }
    }
    for object in objects {
        if object.polygons.is_empty() {
            continue;
        }
        let name = if object.name.is_empty() {
            tr("Object").to_string()
        } else {
            object.name
        };
        let (mut mesh, skipped, faces) = build(&object.polygons, &points, &uvs, &normals);
        out.skipped_polygons += skipped;
        // Its materials, in the order it first uses them; past the most an
        // object can have, the rest share the last.
        let mut used: Vec<usize> = Vec::new();
        for (face, m) in &faces {
            let Some(m) = m else { continue };
            let slot = match used.iter().position(|u| u == m) {
                Some(at) => at,
                None if used.len() < mesh::material::MAX_SLOTS as usize => {
                    used.push(*m);
                    used.len() - 1
                }
                None => used.len() - 1,
            };
            if slot > 0 {
                mesh::material::set_material(&mut mesh, &[*face], slot as u32);
            }
        }
        let materials = used
            .iter()
            .map(|m| scene::Material {
                name: material_names[*m].clone(),
                ..scene::Material::standard()
            })
            .collect();
        if mesh.face_count() > 0 {
            out.objects.push(ImportedObject {
                name,
                mesh,
                materials,
            });
        }
    }
    if out.objects.is_empty() {
        return Err(ObjError::Empty);
    }
    Ok(out)
}

/// A kept polygon's corner: its point, and the file's UV and normal for it.
type Corner = (VertexId, Option<usize>, Option<usize>);

/// A polygon's use of an edge: its two points as the polygon runs, with
/// the file's normal at each.
type EdgeUse = (VertexId, Option<Vec3>, VertexId, Option<Vec3>);

/// A mesh from an object's polygons: only the points it uses, its UVs in
/// the [`mesh::attributes::UV`] layer, and hard edges where the file's
/// normals break. Returns it, how many polygons were left out, and each
/// kept polygon with its material in the file.
fn build(
    polygons: &[FilePolygon],
    points: &[Vec3],
    uvs: &[Vec2],
    normals: &[Vec3],
) -> (Mesh, usize, Vec<(FaceId, Option<usize>)>) {
    let mut mesh = Mesh::new();
    let mut ids: BTreeMap<usize, VertexId> = BTreeMap::new();
    let mut skipped = 0;
    // Each kept polygon's id, with its corners' UVs and normals.
    let mut kept: Vec<(FaceId, Vec<Corner>)> = Vec::new();
    let mut materials: Vec<(FaceId, Option<usize>)> = Vec::new();
    for polygon in polygons {
        let mut corners = Vec::with_capacity(polygon.corners.len());
        for (v, t, n) in &polygon.corners {
            let id = match ids.get(v) {
                Some(id) => *id,
                None => {
                    let Ok(id) = mesh.add_vertex(points[*v]) else {
                        continue;
                    };
                    ids.insert(*v, id);
                    id
                }
            };
            corners.push((id, *t, *n));
        }
        let only: Vec<VertexId> = corners.iter().map(|c| c.0).collect();
        match mesh.add_face(&only) {
            Ok(face) => {
                kept.push((face, corners));
                materials.push((face, polygon.material));
            }
            Err(_) => skipped += 1,
        }
    }
    // UVs, where the file gives them.
    if kept
        .iter()
        .any(|(_, c)| c.iter().any(|(_, t, _)| t.is_some()))
        && mesh
            .add_attribute(
                mesh::attributes::UV,
                Domain::Corner,
                Value::Vec2(Vec2::ZERO),
            )
            .is_ok()
    {
        for (face, corners) in &kept {
            for (i, (_, t, _)) in corners.iter().enumerate() {
                if let Some(uv) = t.and_then(|t| uvs.get(t)) {
                    let _ = mesh.set_attribute(
                        mesh::attributes::UV,
                        AttributeElement::Corner(*face, i),
                        Value::Vec2(*uv),
                    );
                }
            }
        }
    }
    // Hard edges: two polygons meeting at an edge with different normals
    // at either of its points.
    let mut seen: HashMap<EdgeKey, Vec<EdgeUse>> = HashMap::new();
    for (_, corners) in &kept {
        let n = corners.len();
        for i in 0..n {
            let (a, _, na) = corners[i];
            let (b, _, nb) = corners[(i + 1) % n];
            let normal = |k: Option<usize>| k.and_then(|k| normals.get(k).copied());
            seen.entry(EdgeKey::new(a, b))
                .or_default()
                .push((a, normal(na), b, normal(nb)));
        }
    }
    let differs = |x: Option<Vec3>, y: Option<Vec3>| match (x, y) {
        (Some(x), Some(y)) => x.dot(y) < 0.9999,
        _ => false,
    };
    let mut hard: Vec<EdgeKey> = seen
        .into_iter()
        .filter(|(_, uses)| {
            uses.len() == 2 && {
                let (a0, na0, b0, nb0) = uses[0];
                let (_, na1, _, nb1) = uses[1];
                // The second use may run the other way round.
                let (na1, nb1) = if uses[1].0 == a0 {
                    (na1, nb1)
                } else {
                    (nb1, na1)
                };
                let _ = b0;
                differs(na0, na1) || differs(nb0, nb1)
            }
        })
        .map(|(e, _)| e)
        .collect();
    hard.sort_unstable();
    for edge in hard {
        let (a, b) = edge.ends();
        mesh.set_hard(a, b, true);
    }
    (mesh, skipped, materials)
}

#[cfg(test)]
mod tests;
