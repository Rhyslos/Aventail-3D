//! The calls that reach files (decision 0307): `Project:readText`,
//! `writeText` and `exportShapes`, each checked against what the run may
//! reach ([`crate::access`]). Writing waits for the run to end well.

use crate::access::{check_write, read_text, write};
use crate::describe::{Call, STATES, State};
use crate::natives::Lent;
use crate::values::Args;
use fileio::export::{ExportOptions, ExportPreset, ExportWhat};
use mlua::{IntoLua, Lua, MultiValue};
use scene::Document;
use words::trf;

/// Whether `call` reaches files.
pub(crate) fn is_file_call(call: &Call) -> bool {
    call.owner == r"Project" && matches!(call.name, "readText" | "writeText" | "exportShapes")
}

/// Carries out a call that reaches files.
pub(crate) fn call(
    lua: &Lua,
    call: &'static Call,
    args: &Args<'_>,
    lent: &Lent,
) -> mlua::Result<MultiValue> {
    let access = &lent.snapshot.access;
    let written = args.written();
    let value = match call.name {
        "readText" => {
            let path = args.text(1)?;
            let writes = lent.writes.borrow();
            read_text(access, &writes, &written, &path)
                .map_err(mlua::Error::runtime)?
                .into_lua(lua)?
        }
        "writeText" => {
            let path = args.text(1)?;
            let text = args.text(2)?;
            let mut writes = lent.writes.borrow_mut();
            write(access, &mut writes, &written, &path, text.into_bytes())
                .map_err(mlua::Error::runtime)?;
            mlua::Value::Nil
        }
        "exportShapes" => {
            export(args, lent, &written)?;
            mlua::Value::Nil
        }
        _ => return Err(crate::values::wrong_call(call)),
    };
    Ok(MultiValue::from_vec(vec![value]))
}

/// The `Preset` state.
pub(crate) fn preset_state() -> &'static State {
    STATES
        .iter()
        .find(|s| s.name == r"Preset")
        .unwrap_or(&STATES[0])
}

/// `Project:exportShapes(shapes, path, preset)`: the shapes as they are
/// in the run's copy, exported as the Export window would with that
/// preset, held to write at the end.
fn export(args: &Args<'_>, lent: &Lent, written: &str) -> mlua::Result<()> {
    let ids = args.shapes(1)?;
    let path = args.text(2)?;
    let option = args.option(3, preset_state())?;
    let preset = preset_state()
        .options
        .iter()
        .position(|o| *o == option)
        .and_then(|at| ExportPreset::ALL.get(at).copied())
        .unwrap_or(ExportPreset::Godot);
    let settings = preset.settings();
    let ending = settings.format.extension();
    if !path
        .rsplit_once('.')
        .is_some_and(|(_, e)| e.eq_ignore_ascii_case(ending))
    {
        return Err(mlua::Error::runtime(trf!(
            "{} with Preset.{} writes a .{} file: end the path in .{} (such as models/chair.{}).",
            written,
            option,
            ending,
            ending,
            ending
        )));
    }
    check_write(&lent.snapshot.access, written, &path).map_err(mlua::Error::runtime)?;
    let scene = lent.document.borrow().scene().copy_with_results();
    for id in &ids {
        if !scene.contains(*id) {
            return Err(args.gone());
        }
    }
    let document = Document::from_scene(scene, ids.iter().copied());
    let options = ExportOptions {
        what: ExportWhat::Selected,
        units: settings.units,
        up: settings.up,
        triangulate: settings.triangulate,
        godot_names: settings.godot_names,
        packing: settings.packing,
        green_down: settings.green_down,
        ..ExportOptions::default()
    };
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string();
    let files = fileio::export::export_bytes(
        &document,
        settings.format,
        &options,
        &lent.generator,
        &file_name,
    )
    .map_err(|why| mlua::Error::runtime(trf!("{} couldn't export: {}.", written, why)))?;
    let folder = &path[..path.len() - file_name.len()];
    let access = &lent.snapshot.access;
    let mut writes = lent.writes.borrow_mut();
    for (name, bytes) in files.beside {
        write(
            access,
            &mut writes,
            written,
            &format!("{folder}{name}"),
            bytes,
        )
        .map_err(mlua::Error::runtime)?;
    }
    write(access, &mut writes, written, &path, files.main).map_err(mlua::Error::runtime)?;
    Ok(())
}
