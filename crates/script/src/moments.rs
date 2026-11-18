//! Scripts that run by themselves at a moment (plan, milestone 6, choice
//! 13; [decision 0309](../../../docs/decisions/0309-event-scripts-and-examples.md)):
//! a project opened, a project being saved, an export starting.
//!
//! A moment is a Lust event the app's description files declare
//! (`aventail/moments.lsc`). In LustScript a component reacts to it with
//! `on ProjectSaved() { … }`; in Lua a command has a function named `on_`
//! and the moment's name (`function Tidy:on_ProjectSaved() … end`), as
//! Solis's Lua components react to events. [`crate::Run::start_moment`]
//! runs each one reacting, for at most [`MOMENT_LIMIT`].
//!
//! While an export starts, a script reads what goes in with the `Export`
//! calls, can warn, and can stop the export with `Export:cancel(reason)`,
//! but can't change the scene: the export is made from the scene as it is.

use crate::describe::{Call, STATES, State};
use crate::natives::Lent;
use crate::values::Args;
use mlua::{IntoLua, Lua, MultiValue, Table, Value as LuaValue};
use scene::ObjectId;
use std::time::Duration;
use words::trf;

/// The longest a script running by itself may take (choice 13): it holds
/// up opening, saving or exporting.
pub const MOMENT_LIMIT: Duration = Duration::from_secs(2);

/// A moment a script can run by itself at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Moment {
    /// A project was opened.
    ProjectOpened,
    /// A project is about to be saved.
    ProjectSaved,
    /// An export is starting.
    ExportStarting,
}

impl Moment {
    /// Every moment, in the order the app lists them.
    pub const ALL: [Self; 3] = [
        Self::ProjectOpened,
        Self::ProjectSaved,
        Self::ExportStarting,
    ];

    /// Its event's name, as scripts write it: `ProjectSaved`.
    pub fn event(self) -> &'static str {
        match self {
            Self::ProjectOpened => r"ProjectOpened",
            Self::ProjectSaved => r"ProjectSaved",
            Self::ExportStarting => r"ExportStarting",
        }
    }

    /// The name of a Lua command's function reacting to it:
    /// `on_ProjectSaved`.
    pub fn lua_handler(self) -> String {
        format!(r"on_{}", self.event())
    }

    /// The name Lust gives a component's `on` for it, in its Lua.
    fn lust_handler(self) -> String {
        format!(r"on__{}", self.event())
    }
}

/// The export that's starting, as the `Export` calls read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportInfo {
    /// The objects it writes, in the Objects panel's order.
    pub shapes: Vec<ObjectId>,
    /// The kind of file.
    pub format: fileio::export::ExportFormat,
}

/// Whether a LustScript component's class reacts to `moment`.
pub(crate) fn lust_handles(class: &Table, moment: Moment) -> mlua::Result<bool> {
    Ok(class
        .raw_get::<Option<mlua::Function>>(moment.lust_handler())?
        .is_some())
}

/// Whether a Lua command's class reacts to any moment.
pub(crate) fn lua_handles_any(class: &Table) -> mlua::Result<bool> {
    for moment in Moment::ALL {
        if class
            .raw_get::<Option<mlua::Function>>(moment.lua_handler())?
            .is_some()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The moments a LustScript component's class reacts to.
pub(crate) fn lust_moments(class: &Table) -> mlua::Result<Vec<Moment>> {
    let mut found = Vec::new();
    for moment in Moment::ALL {
        if lust_handles(class, moment)? {
            found.push(moment);
        }
    }
    Ok(found)
}

/// The moments a Lua command's class reacts to.
pub(crate) fn lua_moments(class: &Table) -> mlua::Result<Vec<Moment>> {
    let mut found = Vec::new();
    for moment in Moment::ALL {
        if class
            .raw_get::<Option<mlua::Function>>(moment.lua_handler())?
            .is_some()
        {
            found.push(moment);
        }
    }
    Ok(found)
}

/// Whether `call` is one of the `Export` calls.
pub(crate) fn is_moment_call(call: &Call) -> bool {
    call.owner == r"Export"
}

/// The `Format` state.
fn format_state() -> &'static State {
    STATES
        .iter()
        .find(|s| s.name == r"Format")
        .unwrap_or(&STATES[0])
}

/// Carries out one of the `Export` calls.
pub(crate) fn call(
    lua: &Lua,
    call: &'static Call,
    args: &Args<'_>,
    lent: &Lent,
    shape: &dyn Fn(ObjectId) -> mlua::Result<LuaValue>,
) -> mlua::Result<MultiValue> {
    let Some(export) = lent
        .snapshot
        .export
        .as_ref()
        .filter(|_| lent.snapshot.moment == Some(Moment::ExportStarting))
    else {
        return Err(mlua::Error::runtime(trf!(
            "{} only works while an export is starting: in a component's on ExportStarting() (in Lua, a command's on_ExportStarting function), run by itself when you export.",
            args.written()
        )));
    };
    let value = match call.name {
        "shapes" => {
            let document = lent.document.borrow();
            let list: Vec<LuaValue> = export
                .shapes
                .iter()
                .copied()
                .filter(|id| document.scene().object(*id).is_some())
                .map(shape)
                .collect::<mlua::Result<_>>()?;
            crate::natives::array(lua, list)?
        }
        "format" => {
            let at = match export.format {
                fileio::export::ExportFormat::Glb => 0,
                fileio::export::ExportFormat::Obj => 1,
                fileio::export::ExportFormat::Stl => 2,
            };
            format_state().options[at].into_lua(lua)?
        }
        "cancel" => {
            let reason = args.text(1)?;
            let reason = reason.trim();
            let reason = if reason.is_empty() {
                words::tr("it gave no reason").to_string()
            } else {
                crate::header::capped(reason, 500)
            };
            let mut cancelled = lent.cancelled.borrow_mut();
            if cancelled.is_none() {
                lent.output
                    .send(crate::output::Event::Cancelled(reason.clone()));
                *cancelled = Some(reason);
            }
            LuaValue::Nil
        }
        _ => return Err(crate::values::wrong_call(call)),
    };
    Ok(MultiValue::from_vec(vec![value]))
}

#[cfg(test)]
#[path = "moments_tests.rs"]
mod tests;
