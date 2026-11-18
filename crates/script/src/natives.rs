//! The bodies of the calls in [`crate::describe::CALLS`]: Rust functions
//! in the run's `LUST_NATIVES`, which Lust's description files and Lua's
//! `aventail.` both reach. The reading calls read the run's own copy of
//! the document; the changing ones are in [`crate::changes`].

use crate::describe::{CALLS, Call};
use crate::output::Output;
use crate::run::Snapshot;
use crate::sandbox::Sandbox;
use crate::values::{Args, wrong_call};
use glam::Vec3;
use mesh::{EdgeKey, Mesh};
use mlua::{Function, IntoLua, Lua, MultiValue, Value as LuaValue};
use scene::{Change, Document, ObjectId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use words::trf;

/// The most points or polygons one call reads (choice 5's chunks).
pub const MAX_CHUNK: i64 = 100_000;

/// What the natives lend from: the copy of the document, what the run
/// has changed, and the output.
pub(crate) struct Lent {
    /// The rest of what the run was given (its scene is in `document`).
    pub(crate) snapshot: Snapshot,
    /// The run's own copy of the document, which the changing calls
    /// change as they go.
    pub(crate) document: RefCell<Document>,
    /// What the run has changed, in order, to land on the real document.
    pub(crate) changes: RefCell<Vec<Change>>,
    /// Points and polygons built so far ([`crate::changes::MAX_BUILT`]).
    pub(crate) built: Cell<usize>,
    /// Set once the scene refused a change: the run lands nothing.
    pub(crate) spoilt: Cell<bool>,
    pub(crate) output: Rc<Output>,
    /// Whether the script is Lua (for how its calls are written in
    /// messages).
    pub(crate) lua: bool,
    /// The app's global for Lua.
    pub(crate) folder: String,
    /// Who made what the run exports (the app's name and version).
    pub(crate) generator: String,
    /// What the run has written, waiting for it to end well.
    pub(crate) writes: RefCell<crate::access::Writes>,
    /// Why a script stopped the export starting, once one did
    /// (`Export:cancel`).
    pub(crate) cancelled: RefCell<Option<String>>,
}

/// Fills the sandbox's natives with every call's body.
pub(crate) fn lend(sandbox: &Sandbox, lent: Rc<Lent>) -> mlua::Result<()> {
    for call in CALLS {
        let lent = lent.clone();
        let wrap = sandbox.wrap.clone();
        let unwrap = sandbox.unwrap.clone();
        let function = sandbox.lua.create_function(move |lua, given: MultiValue| {
            let args = Args::new(call, given, &lent, &unwrap);
            body(lua, call, &args, &lent, &wrap)
        })?;
        sandbox.natives.raw_set(call.key(), function)?;
    }
    sandbox.seal();
    Ok(())
}

/// One call's work.
fn body(
    lua: &Lua,
    call: &'static Call,
    args: &Args<'_>,
    lent: &Lent,
    wrap: &mlua::Function,
) -> mlua::Result<MultiValue> {
    let shape = |id: ObjectId| -> mlua::Result<LuaValue> { wrap.call((r"Shape", id.get())) };
    if let Some(done) = crate::changes::call(call, args, lent, &shape)? {
        return Ok(done);
    }
    if crate::files::is_file_call(call) {
        return crate::files::call(lua, call, args, lent);
    }
    if crate::moments::is_moment_call(call) {
        return crate::moments::call(lua, call, args, lent, &shape);
    }
    let document = lent.document.borrow();
    let scene = document.scene();
    let shapes = |ids: &mut dyn Iterator<Item = ObjectId>| -> mlua::Result<LuaValue> {
        let list: Vec<LuaValue> = ids.map(shape).collect::<mlua::Result<_>>()?;
        array(lua, list)
    };
    let value: LuaValue = match (call.owner, call.name) {
        (r"Scene", "shapes") => shapes(&mut scene.walk().into_iter().map(|(id, _)| id))?,
        (r"Scene", "selected") => {
            let chosen = &document.selection();
            shapes(
                &mut scene
                    .walk()
                    .into_iter()
                    .map(|(id, _)| id)
                    .filter(|id| chosen.contains(id)),
            )?
        }
        (r"Scene", "find") => {
            let name = args.text(1)?;
            let found = scene
                .walk()
                .into_iter()
                .map(|(id, _)| id)
                .find(|id| scene.object(*id).is_some_and(|o| o.name == name));
            match found {
                Some(id) => shape(id)?,
                None => LuaValue::Nil,
            }
        }
        (r"Shape", _) => return shape_call(lua, call, args, lent, &shape),
        (r"Selection", _) => return selection_call(lua, call, args, lent, &document),
        (r"Project", "name") => lent.snapshot.project_name.clone().into_lua(lua)?,
        (r"Project", "folder") => lent.snapshot.project_folder.clone().into_lua(lua)?,
        (r"Script", "api") => crate::describe::API_VERSION.into_lua(lua)?,
        (r"Script", "say") => {
            lent.output.say(&args.text(1)?);
            LuaValue::Nil
        }
        (r"Script", "warn") => {
            lent.output.warn(&args.text(1)?);
            LuaValue::Nil
        }
        (r"Script", "progress") => {
            let done = args.number(1)?;
            let what = args.text(2)?;
            lent.output.progress(done, &what);
            LuaValue::Nil
        }
        _ => return Err(wrong_call(call)),
    };
    Ok(MultiValue::from_vec(vec![value]))
}

/// A shape's own calls.
fn shape_call(
    lua: &Lua,
    call: &'static Call,
    args: &Args<'_>,
    lent: &Lent,
    shape: &dyn Fn(ObjectId) -> mlua::Result<LuaValue>,
) -> mlua::Result<MultiValue> {
    let document = lent.document.borrow();
    let scene = document.scene();
    let id = args.shape(0)?;
    let object = scene.object(id).ok_or_else(|| args.gone())?;
    let final_shape = || scene.evaluated(id).map(|e| e.mesh);
    let value: LuaValue = match call.name {
        "name" => object.name.clone().into_lua(lua)?,
        "position" => vector(object.transform.position),
        "rotation" => {
            let r = object.transform.rotation;
            vector(Vec3::new(
                r.x.to_degrees(),
                r.y.to_degrees(),
                r.z.to_degrees(),
            ))
        }
        "scale" => vector(object.transform.scale),
        "parent" => match scene.parent(id) {
            Some(parent) => shape(parent)?,
            None => LuaValue::Nil,
        },
        "children" => {
            let list: Vec<LuaValue> = scene
                .children(Some(id))
                .iter()
                .map(|child| shape(*child))
                .collect::<mlua::Result<_>>()?;
            array(lua, list)?
        }
        "shown" => object.visible.into_lua(lua)?,
        "pointCount" => final_shape()
            .map_or(0, |m| m.vertex_count())
            .into_lua(lua)?,
        "polygonCount" => final_shape().map_or(0, |m| m.face_count()).into_lua(lua)?,
        "points" => {
            let (from, count) = chunk(args)?;
            match final_shape() {
                Some(mesh) => points(lua, &mesh, from, count)?,
                None => array(lua, Vec::new())?,
            }
        }
        "polygons" => {
            let (from, count) = chunk(args)?;
            let materials = scene.material_list(id);
            match final_shape() {
                Some(mesh) => polygons(lua, &mesh, from, count, &materials)?,
                None => array(lua, Vec::new())?,
            }
        }
        "hardEdges" => {
            let edges: Vec<EdgeKey> = final_shape().map_or_else(Vec::new, |m| {
                let mut found: Vec<EdgeKey> = m.hard_edges().collect();
                found.sort_unstable();
                found
            });
            edge_array(lua, &edges)?
        }
        "seams" => {
            let edges = final_shape().map_or_else(Vec::new, |m| mesh::uvs::seams(&m));
            edge_array(lua, &edges)?
        }
        "materials" => {
            let names: Vec<LuaValue> = scene
                .material_list(id)
                .into_iter()
                .map(|m| m.name.into_lua(lua))
                .collect::<mlua::Result<_>>()?;
            array(lua, names)?
        }
        _ => return Err(wrong_call(call)),
    };
    Ok(MultiValue::from_vec(vec![value]))
}

/// The selection's calls.
fn selection_call(
    lua: &Lua,
    call: &'static Call,
    args: &Args<'_>,
    lent: &Lent,
    document: &Document,
) -> mlua::Result<MultiValue> {
    let value: LuaValue = if call.name == "mode" {
        lent.snapshot.mode.name().into_lua(lua)?
    } else {
        let id = args.shape(1)?;
        if document.scene().object(id).is_none() {
            return Err(args.gone());
        }
        let elements = document.elements(id);
        match call.name {
            "points" => {
                let ids: Vec<LuaValue> = elements
                    .into_iter()
                    .flat_map(|e| e.points())
                    .map(|p| LuaValue::Integer(i64::from(p.get())))
                    .collect();
                array(lua, ids)?
            }
            "polygons" => {
                let ids: Vec<LuaValue> = elements
                    .into_iter()
                    .flat_map(|e| e.polygons())
                    .map(|p| LuaValue::Integer(i64::from(p.get())))
                    .collect();
                array(lua, ids)?
            }
            "edges" => {
                let edges: Vec<EdgeKey> = elements.into_iter().flat_map(|e| e.edges()).collect();
                edge_array(lua, &edges)?
            }
            _ => return Err(wrong_call(call)),
        }
    };
    Ok(MultiValue::from_vec(vec![value]))
}

/// A chunk's start and length, from a call's `from` and `count`.
fn chunk(args: &Args<'_>) -> mlua::Result<(usize, usize)> {
    let from = args.whole(1)?;
    let count = args.whole(2)?;
    if from < 0 || count < 0 {
        return Err(mlua::Error::runtime(trf!(
            "{} wants from and count of 0 or more, and was given {} and {}.",
            args.written(),
            from,
            count
        )));
    }
    if count > MAX_CHUNK {
        return Err(mlua::Error::runtime(trf!(
            "{} reads at most {} at a time, and was asked for {}: read a heavy shape a chunk at a time.",
            args.written(),
            MAX_CHUNK,
            count
        )));
    }
    Ok((
        usize::try_from(from).unwrap_or(usize::MAX),
        usize::try_from(count).unwrap_or(0),
    ))
}

/// A list for Lust and Lua alike: in order from 1, with its count in `n`.
pub(crate) fn array(lua: &Lua, items: Vec<LuaValue>) -> mlua::Result<LuaValue> {
    let n = items.len();
    let table = lua.create_sequence_from(items)?;
    table.raw_set("n", n)?;
    Ok(LuaValue::Table(table))
}

/// A vec3, as Luau's own vector (Lust's vec3 in Luau).
fn vector(v: Vec3) -> LuaValue {
    LuaValue::Vector(mlua::Vector::new(v.x, v.y, v.z))
}

/// Up to `count` Points from the `from`th. Packed into bytes here and
/// made into tables by the prelude's `points_from`: one call into Lua for
/// the chunk instead of several for every point.
fn points(lua: &Lua, mesh: &Arc<Mesh>, from: usize, count: usize) -> mlua::Result<LuaValue> {
    let mut data = Vec::with_capacity(count.min(mesh.vertex_count()) * 16);
    let mut made = 0usize;
    for (v, at) in mesh.vertices().skip(from).take(count) {
        data.extend_from_slice(&v.get().to_le_bytes());
        for f in [at.x, at.y, at.z] {
            data.extend_from_slice(&f.to_le_bytes());
        }
        made += 1;
    }
    let build: Function = lua.named_registry_value(crate::sandbox::POINTS_FROM)?;
    build.call((lua.create_buffer(data)?, made))
}

/// Up to `count` Polygons from the `from`th, packed for the prelude's
/// `polygons_from` (as [`points`] is).
fn polygons(
    lua: &Lua,
    mesh: &Arc<Mesh>,
    from: usize,
    count: usize,
    materials: &[scene::Material],
) -> mlua::Result<LuaValue> {
    let standard = scene::Material::standard().name;
    // A slot past the list is the standard material, the last name.
    let mut names: Vec<&str> = materials.iter().map(|m| m.name.as_str()).collect();
    names.push(standard.as_str());
    let mut data = Vec::with_capacity(count.min(mesh.face_count()) * 40);
    let mut made = 0usize;
    for (face, corners) in mesh.faces().skip(from).take(count) {
        data.extend_from_slice(&face.get().to_le_bytes());
        data.extend_from_slice(&(corners.len() as u32).to_le_bytes());
        for corner in corners {
            data.extend_from_slice(&corner.get().to_le_bytes());
        }
        let facing = mesh.face_normal(face);
        for f in [facing.x, facing.y, facing.z] {
            data.extend_from_slice(&f.to_le_bytes());
        }
        let slot = (mesh::material::slot_of(mesh, face) as usize).min(materials.len());
        data.extend_from_slice(&(slot as u32).to_le_bytes());
        made += 1;
    }
    let build: Function = lua.named_registry_value(crate::sandbox::POLYGONS_FROM)?;
    build.call((lua.create_buffer(data)?, made, names))
}

/// A list of Edges.
fn edge_array(lua: &Lua, edges: &[EdgeKey]) -> mlua::Result<LuaValue> {
    let list: Vec<LuaValue> = edges
        .iter()
        .map(|edge| {
            let (a, b) = edge.ends();
            let table = lua.create_table_with_capacity(0, 2)?;
            table.raw_set("a", a.get())?;
            table.raw_set("b", b.get())?;
            Ok(LuaValue::Table(table))
        })
        .collect::<mlua::Result<_>>()?;
    array(lua, list)
}

#[cfg(test)]
#[path = "natives_tests.rs"]
mod tests;
