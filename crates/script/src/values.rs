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
        self.shape_of(&value)
            .ok_or_else(|| self.wrong(at, words::tr("a shape"), &value))
    }

    /// The shape `value` names, if it's one the app handed over (or one
    /// LustScript made with an id).
    fn shape_of(&self, value: &LuaValue) -> Option<ObjectId> {
        let (id, kind): (Option<f64>, Option<String>) = self.unwrap.call(value.clone()).ok()?;
        let id = match (id, kind.as_deref(), value) {
            (Some(id), Some(r"Shape"), _) => Some(id),
            // A Shape LustScript made itself: its id is a field.
            (_, None, LuaValue::Table(t)) => t.raw_get::<Option<f64>>("id").ok().flatten(),
            _ => None,
        };
        match id {
            Some(id) if id >= 0.0 && id.fract() == 0.0 && id < 9.0e15 => {
                Some(ObjectId::from_raw(id as u64))
            }
            _ => None,
        }
    }

    /// Value `at` as a vec3: a Luau vector (LustScript's own), or a table
    /// with `x`, `y` and `z`.
    pub(crate) fn vec3(&self, at: usize) -> mlua::Result<glam::Vec3> {
        let value = self.at(at);
        vec3_of(value).ok_or_else(|| self.wrong(at, words::tr("a vec3"), value))
    }

    /// Value `at` as a list (a Lust array, or a Lua table counted from 1),
    /// of at most [`MAX_LIST`] items.
    pub(crate) fn list(&self, at: usize) -> mlua::Result<Vec<LuaValue>> {
        let value = self.at(at);
        let LuaValue::Table(table) = value else {
            return Err(self.wrong(at, words::tr("a list"), value));
        };
        let count = match table.raw_get::<LuaValue>("n")? {
            LuaValue::Integer(n) => n,
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => n as i64,
            _ => i64::try_from(table.raw_len()).unwrap_or(i64::MAX),
        };
        if count < 0 || count > MAX_LIST as i64 {
            return Err(mlua::Error::runtime(trf!(
                "{} takes at most {} items at a time in {}, and was given {}.",
                self.written(),
                MAX_LIST,
                self.value_name(at),
                count
            )));
        }
        (1..=count).map(|i| table.raw_get::<LuaValue>(i)).collect()
    }

    /// Value `at` as a list of whole numbers 0 or more (ids).
    pub(crate) fn ids(&self, at: usize) -> mlua::Result<Vec<u32>> {
        self.list(at)?
            .iter()
            .map(|item| {
                whole_of(item)
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| {
                        self.wrong_item(at, words::tr("an id (a whole number 0 or more)"), item)
                    })
            })
            .collect()
    }

    /// Value `at` as a list of Edges (tables with `a` and `b`).
    pub(crate) fn edges(&self, at: usize) -> mlua::Result<Vec<mesh::EdgeKey>> {
        self.list(at)?
            .iter()
            .map(|item| {
                let end = |key: &str| -> Option<u32> {
                    let LuaValue::Table(t) = item else {
                        return None;
                    };
                    whole_of(&t.raw_get::<LuaValue>(key).ok()?).and_then(|n| u32::try_from(n).ok())
                };
                match (end("a"), end("b")) {
                    (Some(a), Some(b)) => Ok(mesh::EdgeKey::new(
                        mesh::VertexId::from_raw(a),
                        mesh::VertexId::from_raw(b),
                    )),
                    _ => Err(self.wrong_item(at, words::tr("an Edge (with a and b)"), item)),
                }
            })
            .collect()
    }

    /// Value `at` as a list of vec3s.
    pub(crate) fn vec3s(&self, at: usize) -> mlua::Result<Vec<glam::Vec3>> {
        self.list(at)?
            .iter()
            .map(|item| vec3_of(item).ok_or_else(|| self.wrong_item(at, words::tr("a vec3"), item)))
            .collect()
    }

    /// Value `at` as a list of Polygons' corners (each a table whose
    /// `corners` is a list of whole numbers), with at most `most` corners
    /// in all.
    pub(crate) fn corner_lists(&self, at: usize, most: usize) -> mlua::Result<Vec<Vec<i64>>> {
        let mut total = 0usize;
        self.list(at)?
            .iter()
            .map(|item| {
                let corners = match item {
                    LuaValue::Table(t) => t.raw_get::<LuaValue>("corners")?,
                    _ => LuaValue::Nil,
                };
                let LuaValue::Table(inner) = corners else {
                    return Err(self.wrong_item(at, words::tr("a Polygon with its corners"), item));
                };
                let count = match inner.raw_get::<LuaValue>("n")? {
                    LuaValue::Integer(n) => usize::try_from(n).unwrap_or(usize::MAX),
                    _ => inner.raw_len(),
                };
                total = total.saturating_add(count);
                if total > most {
                    return Err(mlua::Error::runtime(trf!(
                        "{} takes at most {} corners in all, in {}.",
                        self.written(),
                        most,
                        self.value_name(at)
                    )));
                }
                (1..=count)
                    .map(|i| {
                        let corner: LuaValue = inner.raw_get(i)?;
                        whole_of(&corner).ok_or_else(|| {
                            self.wrong_item(at, words::tr("a Polygon with its corners"), item)
                        })
                    })
                    .collect()
            })
            .collect()
    }

    /// Value `at` as a list of shapes.
    pub(crate) fn shapes(&self, at: usize) -> mlua::Result<Vec<ObjectId>> {
        self.list(at)?
            .into_iter()
            .map(|item| {
                self.shape_of(&item)
                    .ok_or_else(|| self.wrong_item(at, words::tr("a shape"), &item))
            })
            .collect()
    }

    /// Value `at` as an option of `state` (its name as text).
    pub(crate) fn option(
        &self,
        at: usize,
        state: &crate::describe::State,
    ) -> mlua::Result<&'static str> {
        let value = self.at(at);
        if let LuaValue::String(text) = value {
            let text = text.to_string_lossy();
            // Lust hands an option over by its full name without the
            // state's; Lua scripts may write either.
            let name = text
                .strip_prefix(&format!("{}.", state.name))
                .unwrap_or(&text);
            if let Some(option) = state.options.iter().find(|o| **o == name) {
                return Ok(option);
            }
        }
        Err(mlua::Error::runtime(trf!(
            "{} wants one of {} for {}, such as {}.{}.",
            self.written(),
            state.name,
            self.value_name(at),
            state.name,
            state.options.first().copied().unwrap_or_default()
        )))
    }

    /// The name of value `at`, as the call's table writes it.
    fn value_name(&self, at: usize) -> String {
        if at == 0 {
            self.call.owner.to_lowercase()
        } else {
            self.call
                .values()
                .nth(at - 1)
                .map_or_else(|| trf!("value {}", at), |(_, name)| name.to_string())
        }
    }

    /// The mistake of an item in list `at` not being what the call wants.
    fn wrong_item(&self, at: usize, wanted: &str, given: &LuaValue) -> mlua::Error {
        mlua::Error::runtime(trf!(
            "{} wants every item in {} to be {}, and one is {}.",
            self.written(),
            self.value_name(at),
            wanted,
            kind_words(given)
        ))
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
        let name = self.value_name(at);
        let given_words = kind_words(given);
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

/// The most items a list given to one call may hold (choice 5's chunks).
pub const MAX_LIST: usize = 100_000;

/// What kind of value `given` is, in words.
fn kind_words(given: &LuaValue) -> &'static str {
    match given {
        LuaValue::Nil => words::tr("nothing (nil)"),
        LuaValue::Boolean(_) => words::tr("true or false"),
        LuaValue::Integer(_) | LuaValue::Number(_) => words::tr("a number"),
        LuaValue::String(_) => words::tr("text"),
        LuaValue::Vector(_) => words::tr("a vec3"),
        LuaValue::Table(_) => words::tr("a table"),
        LuaValue::Function(_) => words::tr("a function"),
        _ => words::tr("another kind of value"),
    }
}

/// `value` as a whole number, if it is one.
fn whole_of(value: &LuaValue) -> Option<i64> {
    match value {
        LuaValue::Integer(n) => Some(*n),
        LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Some(*n as i64),
        _ => None,
    }
}

/// `value` as a vec3: a Luau vector, or a table with `x`, `y` and `z`.
fn vec3_of(value: &LuaValue) -> Option<glam::Vec3> {
    let number = |v: LuaValue| -> Option<f32> {
        match v {
            LuaValue::Integer(n) => Some(n as f32),
            LuaValue::Number(n) => Some(n as f32),
            _ => None,
        }
    };
    match value {
        LuaValue::Vector(v) => Some(glam::Vec3::new(v.x(), v.y(), v.z())),
        LuaValue::Table(t) => Some(glam::Vec3::new(
            number(t.raw_get("x").ok()?)?,
            number(t.raw_get("y").ok()?)?,
            number(t.raw_get("z").ok()?)?,
        )),
        _ => None,
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
