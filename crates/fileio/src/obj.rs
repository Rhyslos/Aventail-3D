//! OBJ files: writing exported objects, and reading them back as plain
//! meshes ([decision 0071](../../../docs/decisions/0071-obj-export-and-import.md)).
//!
//! OBJ is a plain text format almost every 3D program reads: `v` lines
//! for points, `vt` for UVs, `vn` for normals, `f` for polygons (each
//! corner `point/uv/normal`, counted from 1 across the whole file), and
//! `o` to start a named object.
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

use crate::export::{ExportOptions, ExportedObject, FileUnits, UpAxis};
use glam::{Mat4, Vec2, Vec3};
use mesh::{AttributeElement, Domain, EdgeKey, FaceId, Mesh, Value, VertexId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

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
        "Object".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The text of an OBJ file holding `objects` (made with `options`), with
/// a first comment line naming `generator`.
pub fn write_obj(objects: &[ExportedObject], options: &ExportOptions, generator: &str) -> String {
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
        for polygon in &shape.polygons {
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

/// One object read from an OBJ file.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedObject {
    /// Its name in the file (or "Object" when it had none).
    pub name: String,
    /// Its shape, in the scene's space (centimetres, Z up), placed as the
    /// file placed it.
    pub mesh: Mesh,
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
            Self::Empty => f.write_str("it holds no polygons"),
            Self::BadNumber { line } => write!(f, "line {line} holds a number that isn't real"),
            Self::TooLarge => f.write_str(crate::limits::TOO_MUCH),
        }
    }
}

impl std::error::Error for ObjError {}

/// A polygon as the file gives it: per corner, indices into the file's
/// points, UVs and normals (from 0).
struct FilePolygon {
    corners: Vec<(usize, Option<usize>, Option<usize>)>,
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
            "o" | "g" | "s" | "mtllib" | "usemtl" => {}
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
                    object.polygons.push(FilePolygon { corners });
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
            "Object".to_string()
        } else {
            object.name
        };
        let (mesh, skipped) = build(&object.polygons, &points, &uvs, &normals);
        out.skipped_polygons += skipped;
        if mesh.face_count() > 0 {
            out.objects.push(ImportedObject { name, mesh });
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
/// normals break. Returns it and how many polygons were left out.
fn build(
    polygons: &[FilePolygon],
    points: &[Vec3],
    uvs: &[Vec2],
    normals: &[Vec3],
) -> (Mesh, usize) {
    let mut mesh = Mesh::new();
    let mut ids: BTreeMap<usize, VertexId> = BTreeMap::new();
    let mut skipped = 0;
    // Each kept polygon's id, with its corners' UVs and normals.
    let mut kept: Vec<(FaceId, Vec<Corner>)> = Vec::new();
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
            Ok(face) => kept.push((face, corners)),
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
    (mesh, skipped)
}

#[cfg(test)]
mod tests;
