//! The calls that change the scene (version 1's changing calls; decision
//! 0302). Each one is a [`scene::Command`], or an [`Operation`] recorded
//! by one, or a change to the selection: exactly what the app's own tools
//! make, so every edit can be undone and shows in Edit layers.
//!
//! Each change is carried out at once on the run's own copy of the
//! document, so the script reads its own results as it goes, and kept in
//! order. When the run ends well, the app carries out the same changes on
//! the real document as one undo step ([`crate::Landing`]).
//!
//! A change the scene refuses stops the script with the scene's reason,
//! and **spoils** the run: even if the script catches the mistake with
//! `pcall`, every later change is refused and nothing it did lands, so
//! the copy and the real document can never part ways.

use crate::describe::{Call, STATES, State};
use crate::natives::Lent;
use crate::values::Args;
use glam::{Mat4, Vec3};
use mesh::{Anchors, EdgeKey, ElementKind, FaceId, Mesh, PrimitiveKind, VertexId};
use mlua::{MultiValue, Value as LuaValue};
use scene::{Change, Command, Object, ObjectId, ObjectKind, Operation, Place};
use std::sync::Arc;
use words::trf;

/// The most changes one run may make (choice 5).
pub const MAX_CHANGES: usize = 100_000;
/// The most points, or polygons, one `Scene:build` may make.
pub const MAX_BUILD: usize = 100_000;
/// The most corners one `Scene:build` may be given, all its polygons'
/// together.
pub const MAX_CORNERS: usize = 1_000_000;
/// The most points and polygons one run may build, all its shapes
/// together: what it builds is kept in the app's memory, which the
/// script's own memory limit doesn't count.
pub const MAX_BUILT: usize = 5_000_000;

/// Whether `call` changes the scene.
pub(crate) fn is_changing(call: &Call) -> bool {
    match call.owner {
        r"Scene" => matches!(call.name, "add" | "build" | "group"),
        r"Shape" => matches!(
            call.name,
            "setName"
                | "setPosition"
                | "setRotation"
                | "setScale"
                | "show"
                | "hide"
                | "delete"
                | "bevel"
                | "extrude"
                | "inset"
                | "markHard"
                | "markSeams"
                | "unwrap"
                | "setMaterial"
                | "makeEditable"
        ),
        r"Selection" => matches!(
            call.name,
            "set" | "setPoints" | "setEdges" | "setPolygons" | "clear"
        ),
        _ => false,
    }
}

/// Carries out `call` if it's a changing call, giving back what it gives
/// back; `None` for a reading call.
pub(crate) fn call(
    call: &'static Call,
    args: &Args<'_>,
    lent: &Lent,
    shape: &dyn Fn(ObjectId) -> mlua::Result<LuaValue>,
) -> mlua::Result<Option<MultiValue>> {
    if !is_changing(call) {
        return Ok(None);
    }
    let value = match (call.owner, call.name) {
        (r"Scene", "add") => {
            let option = args.option(1, primitive_state())?;
            let kind = PrimitiveKind::ALL
                .into_iter()
                .find(|k| format!("{k:?}") == option)
                .unwrap_or(PrimitiveKind::Cube);
            let create = Command::Create(ObjectKind::Primitive(kind.standard()));
            apply(lent, args, Change::Command(create))?;
            newest(lent, shape)?
        }
        (r"Scene", "build") => {
            let object = built(lent, args)?;
            let place = Place {
                parent: None,
                index: 0,
            };
            apply(
                lent,
                args,
                Change::Command(Command::Insert { object, place }),
            )?;
            newest(lent, shape)?
        }
        (r"Scene", "group") => {
            let ids = present(lent, args, args.shapes(1)?)?;
            if ids.is_empty() {
                return Err(mlua::Error::runtime(trf!(
                    "{} wants at least one shape to put in the group.",
                    args.written()
                )));
            }
            apply(lent, args, Change::Command(Command::Group(ids)))?;
            newest(lent, shape)?
        }
        (r"Shape", _) => match shape_change(call, args, lent)? {
            Some(()) => LuaValue::Nil,
            None => return Ok(None),
        },
        (r"Selection", "set") => {
            let ids = present(lent, args, args.shapes(1)?)?;
            apply(lent, args, Change::SelectObjects(ids))?;
            LuaValue::Nil
        }
        (r"Selection", "clear") => {
            apply(lent, args, Change::SelectObjects(Vec::new()))?;
            LuaValue::Nil
        }
        (r"Selection", "setPoints" | "setEdges" | "setPolygons") => {
            select_elements(call, args, lent)?;
            LuaValue::Nil
        }
        _ => return Ok(None),
    };
    Ok(Some(MultiValue::from_vec(vec![value])))
}

/// Carries out `change` on the run's copy and keeps it, or stops the
/// script with the reason it can't be.
fn apply(lent: &Lent, args: &Args<'_>, change: Change) -> mlua::Result<()> {
    if lent.snapshot.moment == Some(crate::moments::Moment::ExportStarting) {
        lent.spoilt.set(true);
        return Err(mlua::Error::runtime(trf!(
            "{} can't change the scene while an export is starting: the export is made from the scene as it is. Warn instead (Script:warn), or stop the export with a reason (Export:cancel).",
            args.written()
        )));
    }
    if lent.spoilt.get() {
        return Err(mlua::Error::runtime(trf!(
            "{} can't change the scene: an earlier change couldn't be made, so nothing this run does will be kept.",
            args.written()
        )));
    }
    if lent.changes.borrow().len() >= MAX_CHANGES {
        lent.spoilt.set(true);
        return Err(mlua::Error::runtime(trf!(
            "{} would be more than {} changes, the most a script may make in one run, so it was stopped and nothing it did was kept.",
            args.written(),
            MAX_CHANGES
        )));
    }
    let refused = lent
        .document
        .borrow_mut()
        .execute_batch([change.clone()], "")
        .err();
    if let Some(refused) = refused {
        lent.spoilt.set(true);
        return Err(mlua::Error::runtime(trf!(
            "{} couldn't be done: {}.",
            args.written(),
            refused.why
        )));
    }
    lent.changes.borrow_mut().push(change);
    Ok(())
}

/// The shape just made, which the command selected.
fn newest(
    lent: &Lent,
    shape: &dyn Fn(ObjectId) -> mlua::Result<LuaValue>,
) -> mlua::Result<LuaValue> {
    let id = lent.document.borrow().selection_order().last().copied();
    match id {
        Some(id) => shape(id),
        None => Ok(LuaValue::Nil),
    }
}

/// `ids`, each checked to be in the scene.
fn present(lent: &Lent, args: &Args<'_>, ids: Vec<ObjectId>) -> mlua::Result<Vec<ObjectId>> {
    let document = lent.document.borrow();
    if ids.iter().any(|id| !document.scene().contains(*id)) {
        return Err(args.gone());
    }
    Ok(ids)
}

/// The `Primitive` state.
fn primitive_state() -> &'static State {
    STATES
        .iter()
        .find(|s| s.name == r"Primitive")
        .unwrap_or(&STATES[0])
}

/// A shape's own changing calls; `None` for its reading ones.
fn shape_change(call: &'static Call, args: &Args<'_>, lent: &Lent) -> mlua::Result<Option<()>> {
    let id = args.shape(0)?;
    let object = {
        let document = lent.document.borrow();
        document
            .scene()
            .object(id)
            .cloned()
            .ok_or_else(|| args.gone())?
    };
    let mut transform = object.transform;
    let command = match call.name {
        "setName" => Command::Rename {
            id,
            name: args.text(1)?,
        },
        "setPosition" => {
            transform.position = args.vec3(1)?;
            Command::SetTransforms(vec![(id, transform)])
        }
        "setRotation" => {
            let degrees = args.vec3(1)?;
            transform.rotation = Vec3::new(
                degrees.x.to_radians(),
                degrees.y.to_radians(),
                degrees.z.to_radians(),
            );
            Command::SetTransforms(vec![(id, transform)])
        }
        "setScale" => {
            transform.scale = args.vec3(1)?;
            Command::SetTransforms(vec![(id, transform)])
        }
        "show" | "hide" => Command::SetVisible {
            ids: vec![id],
            visible: call.name == "show",
        },
        "delete" => Command::Delete(vec![id]),
        "makeEditable" => Command::MakeEditable(vec![id]),
        "bevel" | "extrude" | "inset" | "markHard" | "markSeams" | "unwrap" | "setMaterial" => {
            for change in edit(call, args, lent, id)? {
                apply(lent, args, change)?;
            }
            return Ok(Some(()));
        }
        _ => return Ok(None),
    };
    apply(lent, args, Change::Command(command))?;
    Ok(Some(()))
}

/// The changes an edit on the shape's final mesh makes: none when
/// there's nothing to change.
fn edit(call: &Call, args: &Args<'_>, lent: &Lent, id: ObjectId) -> mlua::Result<Vec<Change>> {
    let document = lent.document.borrow();
    let scene = document.scene();
    let Some(mesh) = scene.evaluated(id).map(|e| e.mesh) else {
        return Err(mlua::Error::runtime(trf!(
            "{} couldn't be done: {}.",
            args.written(),
            scene::SceneError::NoShape
        )));
    };
    let matrix = scene.shape_matrix(id);
    let scale = average_scale(matrix);
    let stretch = scene::stretch_of(matrix);
    let record = |operation: Operation| Change::Command(Command::Record { id, operation });
    let operation = match call.name {
        "bevel" => {
            let edges = known_edges(args, &mesh, 1)?;
            let width = args.number(2)? as f32;
            let segments = args.whole(3)?;
            let most = i64::from(mesh::bevel::MAX_SEGMENTS);
            if !(1..=most).contains(&segments) {
                return Err(mlua::Error::runtime(trf!(
                    "{} wants segments from 1 to {}, and was given {}.",
                    args.written(),
                    most,
                    segments
                )));
            }
            if edges.is_empty() {
                return Ok(Vec::new());
            }
            let anchors = Anchors::record(&mesh, ElementKind::Edges, &edge_places(&edges));
            Operation::BevelEdges {
                edges,
                shape: scene::BevelShape {
                    width: width / scale,
                    segments: u32::try_from(segments).unwrap_or(1),
                    ..scene::BevelShape::default()
                },
                stretch,
                anchors,
            }
        }
        "extrude" | "inset" => {
            let faces = known_faces(args, &mesh, 1)?;
            let distance = args.number(2)? as f32 / scale;
            if faces.is_empty() {
                return Ok(Vec::new());
            }
            let anchors = face_anchors(&mesh, &faces);
            if call.name == "inset" {
                Operation::InsetPolygons {
                    faces,
                    distance,
                    grouping: scene::Grouping::Together,
                    stretch,
                    anchors,
                }
            } else {
                Operation::ExtrudePolygons {
                    faces,
                    distance,
                    grouping: scene::Grouping::Together,
                    shape: scene::ExtrudeShape::default(),
                    stretch,
                    anchors,
                }
            }
        }
        "markHard" => {
            let edges: Vec<EdgeKey> = known_edges(args, &mesh, 1)?
                .into_iter()
                .filter(|e| {
                    let (a, b) = e.ends();
                    !mesh.is_hard(a, b)
                })
                .collect();
            if edges.is_empty() {
                return Ok(Vec::new());
            }
            let anchors = Anchors::record(&mesh, ElementKind::Edges, &edge_places(&edges));
            Operation::SetHardEdges {
                edges,
                hard: true,
                anchors,
            }
        }
        "markSeams" => {
            let topology = mesh.topology();
            let edges: Vec<EdgeKey> = known_edges(args, &mesh, 1)?
                .into_iter()
                .filter(|e| topology.edge_uses(*e).len() == 2 && !mesh::uvs::is_seam(&mesh, *e))
                .collect();
            if edges.is_empty() {
                return Ok(Vec::new());
            }
            let anchors = Anchors::record(&mesh, ElementKind::Edges, &edge_places(&edges));
            Operation::SetSeams {
                edges,
                seam: true,
                anchors,
            }
        }
        "unwrap" => {
            let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
            if faces.is_empty() {
                return Ok(Vec::new());
            }
            Operation::Unwrap {
                anchors: face_anchors(&mesh, &faces),
                faces,
                options: lent.snapshot.unwrap,
            }
        }
        _ => return material(args, lent, id, &mesh),
    };
    Ok(vec![record(operation)])
}

/// `shape:setMaterial(polygons, name)`: the material added to the shape
/// first when it hasn't got it.
fn material(args: &Args<'_>, lent: &Lent, id: ObjectId, mesh: &Mesh) -> mlua::Result<Vec<Change>> {
    let faces = known_faces(args, mesh, 1)?;
    let name = args.text(2)?.trim().to_string();
    if name.is_empty() {
        return Err(mlua::Error::runtime(trf!(
            "{} wants a material's name, and was given empty text.",
            args.written()
        )));
    }
    if faces.is_empty() {
        return Ok(Vec::new());
    }
    let document = lent.document.borrow();
    let scene = document.scene();
    let mut materials = scene.material_list(id);
    let mut changes = Vec::new();
    let slot = match materials.iter().position(|m| m.name == name) {
        Some(slot) => slot,
        None => {
            let color = scene
                .walk()
                .into_iter()
                .filter_map(|(other, _)| scene.object(other))
                .flat_map(|o| o.materials.iter())
                .find(|m| m.name == name)
                .map_or(scene::Material::standard().color, |m| m.color);
            materials.push(scene::Material { name, color });
            changes.push(Change::Command(Command::SetMaterials {
                id,
                materials: materials.clone(),
            }));
            materials.len() - 1
        }
    };
    changes.push(Change::Command(Command::Record {
        id,
        operation: Operation::SetMaterial {
            anchors: face_anchors(mesh, &faces),
            faces,
            slot: u32::try_from(slot).unwrap_or(u32::MAX),
        },
    }));
    Ok(changes)
}

/// `Selection:setPoints`, `setEdges` and `setPolygons`: the shape's
/// elements of that kind replaced, the others kept.
fn select_elements(call: &Call, args: &Args<'_>, lent: &Lent) -> mlua::Result<()> {
    let id = args.shape(1)?;
    let (mesh, mut selection) = {
        let document = lent.document.borrow();
        let scene = document.scene();
        if !scene.contains(id) {
            return Err(args.gone());
        }
        let mesh = scene.evaluated(id).map(|e| e.mesh).unwrap_or_default();
        (mesh, document.elements(id).cloned().unwrap_or_default())
    };
    match call.name {
        "setPoints" => {
            selection.clear_kind(ElementKind::Points);
            for point in known_points(args, &mesh, 2)? {
                selection.insert(mesh::Element::Point(point));
            }
        }
        "setEdges" => {
            selection.clear_kind(ElementKind::Edges);
            for edge in known_edges(args, &mesh, 2)? {
                selection.insert(mesh::Element::Edge(edge));
            }
        }
        _ => {
            selection.clear_kind(ElementKind::Polygons);
            for face in known_faces(args, &mesh, 2)? {
                selection.insert(mesh::Element::Polygon(face));
            }
        }
    }
    apply(lent, args, Change::SelectElements(id, selection))
}

/// The shape a `Scene:build` makes, checked.
fn built(lent: &Lent, args: &Args<'_>) -> mlua::Result<Object> {
    let name = args.text(1)?.trim().to_string();
    if name.is_empty() {
        return Err(mlua::Error::runtime(trf!(
            "{} wants a name for the shape, and was given empty text.",
            args.written()
        )));
    }
    let points = args.vec3s(2)?;
    let polygons = args.corner_lists(3, MAX_CORNERS)?;
    if points.len() > MAX_BUILD || polygons.len() > MAX_BUILD {
        return Err(mlua::Error::runtime(trf!(
            "{} makes at most {} points and {} polygons, and was given {} and {}: build a big shape from smaller ones.",
            args.written(),
            MAX_BUILD,
            MAX_BUILD,
            points.len(),
            polygons.len()
        )));
    }
    let built = lent
        .built
        .get()
        .saturating_add(points.len() + polygons.len());
    if built > MAX_BUILT {
        return Err(mlua::Error::runtime(trf!(
            "{} would build more than {} points and polygons in one run, the most a script may.",
            args.written(),
            MAX_BUILT
        )));
    }
    let mut mesh = Mesh::new();
    for (n, at) in points.iter().enumerate() {
        if !at.is_finite() {
            return Err(mlua::Error::runtime(trf!(
                "{} was given point {}, which isn't a real number.",
                args.written(),
                n
            )));
        }
        mesh.add_vertex(*at).map_err(|why| refused(args, &why))?;
    }
    for (n, corners) in polygons.iter().enumerate() {
        let ids: Vec<VertexId> = corners
            .iter()
            .map(|c| match u32::try_from(*c) {
                Ok(c) if (c as usize) < points.len() => Ok(VertexId::from_raw(c)),
                _ => Err(mlua::Error::runtime(trf!(
                    "{} was given polygon {} with a corner at point {}, but there are only {} points, counted from 0.",
                    args.written(),
                    n,
                    c,
                    points.len()
                ))),
            })
            .collect::<mlua::Result<_>>()?;
        mesh.add_face(&ids).map_err(|why| {
            mlua::Error::runtime(trf!(
                "{} couldn't make polygon {}: {}.",
                args.written(),
                n,
                why
            ))
        })?;
    }
    lent.built.set(built);
    let mut object = Object::new(ObjectKind::Mesh(Arc::new(mesh)));
    object.name = name;
    Ok(object)
}

/// A mesh's refusal, in words.
fn refused(args: &Args<'_>, why: &mesh::MeshError) -> mlua::Error {
    mlua::Error::runtime(trf!("{} couldn't be done: {}.", args.written(), why))
}

/// The point ids in value `at`, each checked to be in `mesh`.
fn known_points(args: &Args<'_>, mesh: &Mesh, at: usize) -> mlua::Result<Vec<VertexId>> {
    args.ids(at)?
        .into_iter()
        .map(VertexId::from_raw)
        .map(|v| {
            if mesh.position(v).is_some() {
                Ok(v)
            } else {
                Err(missing(args, &trf!("point {}", v.get())))
            }
        })
        .collect()
}

/// The polygon ids in value `at`, each checked to be in `mesh`.
fn known_faces(args: &Args<'_>, mesh: &Mesh, at: usize) -> mlua::Result<Vec<FaceId>> {
    args.ids(at)?
        .into_iter()
        .map(FaceId::from_raw)
        .map(|f| {
            if mesh.face(f).is_some() {
                Ok(f)
            } else {
                Err(missing(args, &trf!("polygon {}", f.get())))
            }
        })
        .collect()
}

/// The edges in value `at`, each checked to be in `mesh`.
fn known_edges(args: &Args<'_>, mesh: &Mesh, at: usize) -> mlua::Result<Vec<EdgeKey>> {
    let topology = mesh.topology();
    args.edges(at)?
        .into_iter()
        .map(|e| {
            if topology.edge_uses(e).is_empty() {
                let (a, b) = e.ends();
                Err(missing(
                    args,
                    &trf!("the edge from point {} to {}", a.get(), b.get()),
                ))
            } else {
                Ok(e)
            }
        })
        .collect()
}

/// The mistake of an element the shape hasn't got.
fn missing(args: &Args<'_>, what: &str) -> mlua::Error {
    mlua::Error::runtime(trf!(
        "{} was given {}, which the shape hasn't got (it may have been changed since).",
        args.written(),
        what
    ))
}

/// Each edge as its two ends, for its anchors.
fn edge_places(edges: &[EdgeKey]) -> Vec<Vec<VertexId>> {
    edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            vec![a, b]
        })
        .collect()
}

/// The anchors of `faces` in `mesh`.
fn face_anchors(mesh: &Mesh, faces: &[FaceId]) -> Option<Anchors> {
    let places: Vec<Vec<VertexId>> = faces
        .iter()
        .filter_map(|f| mesh.face(*f).map(<[VertexId]>::to_vec))
        .collect();
    Anchors::record(mesh, ElementKind::Polygons, &places)
}

/// How much a shape placed by `matrix` is scaled, on average: a size
/// given as seen in the scene is divided by it, as the tools do.
fn average_scale(matrix: Mat4) -> f32 {
    let scale = (matrix.x_axis.truncate().length()
        + matrix.y_axis.truncate().length()
        + matrix.z_axis.truncate().length())
        / 3.0;
    if scale.is_finite() && scale > 1e-6 {
        scale
    } else {
        1.0
    }
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod tests;
