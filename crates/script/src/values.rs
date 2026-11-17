//! A call's values, read one at a time, so a value of the wrong kind is a
//! mistake in plain words that names the call and the value:
//! "aventail.Scene:find(name) wants text for name, and was given nothing
//! (nil)." Lust checks its calls before running, so mostly Lua meets
//! these. Adapted from Solis's `native_values.rs`.

use crate::describe::Call;
use crate::natives::Lent;
use mlua::{Function, MultiValue, Value as LuaValue};
use scene::ObjectId;
use words::trf;

/// The values a call was given: the object or thing it's called on
/// first, then what it takes.
pub(crate) struct Args<'a> {
    call: &'static Call,
    values: Vec<LuaValue>,
    lent: &'a Lent,
    unwrap: &'a Function,
}

impl<'a> Args<'a> {
    /// The values given to `call`.
    pub(crate) fn new(
        call: &'static Call,
        given: MultiValue,
        lent: &'a Lent,
        unwrap: &'a Function,
    ) -> Self {
        Self {
            call,
            values: given.into_vec(),
            lent,
            unwrap,
        }
    }

    /// How the script writes the call: `aventail.Scene:find(name)` in Lua,
    /// `Scene:find(name)` in LustScript.
    pub(crate) fn written(&self) -> String {
        if self.lent.lua {
            self.call.lua_call(&self.lent.folder)
        } else {
            self.call.lust_call()
        }
    }

    fn at(&self, at: usize) -> &LuaValue {
        self.values.get(at).unwrap_or(&LuaValue::Nil)
    }

    /// Value `at` (0 is what it's called on) as text.
    pub(crate) fn text(&self, at: usize) -> mlua::Result<String> {
        match self.at(at) {
            LuaValue::String(s) => Ok(s.to_string_lossy()),
            LuaValue::Integer(n) => Ok(n.to_string()),
            LuaValue::Number(n) => Ok(n.to_string()),
            other => Err(self.wrong(at, words::tr("text"), other)),
        }
    }

    /// Value `at` as a number.
    pub(crate) fn number(&self, at: usize) -> mlua::Result<f64> {
        match self.at(at) {
            LuaValue::Integer(n) => Ok(*n as f64),
            LuaValue::Number(n) => Ok(*n),
            other => Err(self.wrong(at, words::tr("a number"), other)),
        }
    }

    /// Value `at` as a whole number.
    pub(crate) fn whole(&self, at: usize) -> mlua::Result<i64> {
        match self.at(at) {
            LuaValue::Integer(n) => Ok(*n),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Ok(*n as i64),
            other => Err(self.wrong(at, words::tr("a whole number"), other)),
        }
    }

    /// Value `at` as a shape the app handed over (or one LustScript made
    /// with an id).
    pub(crate) fn shape(&self, at: usize) -> mlua::Result<ObjectId> {
        let value = self.at(at).clone();
        let (id, kind): (Option<f64>, Option<String>) = self.unwrap.call(value.clone())?;
        let id = match (id, kind.as_deref(), &value) {
            (Some(id), Some(r"Shape"), _) => Some(id),
            // A Shape LustScript made itself: its id is a field.
            (_, None, LuaValue::Table(t)) => t.raw_get::<Option<f64>>("id").ok().flatten(),
            _ => None,
        };
        match id {
            Some(id) if id >= 0.0 && id.fract() == 0.0 && id < 9.0e15 => {
                Ok(ObjectId::from_raw(id as u64))
            }
            _ => Err(self.wrong(at, words::tr("a shape"), &value)),
        }
    }

    /// The mistake of a shape that isn't in the scene.
    pub(crate) fn gone(&self) -> mlua::Error {
        mlua::Error::runtime(trf!(
            "{} was given a shape that isn't in the scene.",
            self.written()
        ))
    }

    /// The mistake of value `at` not being what the call wants.
    fn wrong(&self, at: usize, wanted: &str, given: &LuaValue) -> mlua::Error {
        let name = if at == 0 {
            self.call.owner.to_lowercase()
        } else {
            self.call
                .values()
                .nth(at - 1)
                .map_or_else(|| trf!("value {}", at), |(_, name)| name.to_string())
        };
        let given_words = match given {
            LuaValue::Nil => words::tr("nothing (nil)").to_string(),
            LuaValue::Boolean(_) => words::tr("true or false").to_string(),
            LuaValue::Integer(_) | LuaValue::Number(_) => words::tr("a number").to_string(),
            LuaValue::String(_) => words::tr("text").to_string(),
            LuaValue::Vector(_) => words::tr("a vec3").to_string(),
            LuaValue::Table(_) => words::tr("a table").to_string(),
            LuaValue::Function(_) => words::tr("a function").to_string(),
            _ => words::tr("another kind of value").to_string(),
        };
        // A misspelt name is nil in Lua, and so is every value when the
        // object isn't given first: aventail.Scene.find("x") for
        // aventail.Scene:find("x").
        let hint = if matches!(given, LuaValue::Nil) && self.lent.lua {
            words::tr(
                " Is a value missing, its name misspelt, or the colon before the call's name written as a dot?",
            )
        } else {
            ""
        };
        mlua::Error::runtime(trf!(
            "{} wants {} for {}, and was given {}.{}",
            self.written(),
            wanted,
            name,
            given_words,
            hint
        ))
    }
}

/// The mistake of a call with no body: a bug in the app (a test checks
/// every call has one).
pub(crate) fn wrong_call(call: &Call) -> mlua::Error {
    mlua::Error::runtime(trf!(
        "{} has no body in this app. This is a bug; please report it.",
        call.key()
    ))
}
