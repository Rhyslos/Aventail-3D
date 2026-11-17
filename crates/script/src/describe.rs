//! Every call a script can make, listed once ([`CALLS`]), with the kinds
//! of things they hand over ([`CLASSES`], [`STATES`]). Three faces are
//! made from it, so they can't disagree (plan, "One table, three faces"):
//!
//! - the LustScript **description files** in the app's scripting folder
//!   (`aventail/scene.lsc` and the rest, [`descriptions`]), which Lust
//!   checks every call against before anything runs;
//! - Lua's names under the app's global (`aventail.Scene:selected()`),
//!   made by the sandbox from the same table;
//! - the manual's **reference page** ([`reference_page`]), which a test in
//!   the app compares with `docs/manual/script-reference.md`.
//!
//! The calls follow Solis's game API guidelines (choice 14): methods on
//! things, a few named objects for the whole app, fixed choices as states,
//! readers named as nouns.
//!
//! The table's texts are raw strings (`r"…"`): the help texts (`about`)
//! are the reference's own words, written into the description files and
//! the manual, and like the calls' names and Lust lines they stay in
//! English whatever language the app is in, so the language layer's check
//! passes over them.

use crate::Names;

/// The scripting version the calls make (choice 15): a script for a newer
/// one is refused, and new calls don't change it.
pub const API_VERSION: u32 = 1;

/// Who a call belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKind {
    /// One for the whole app, called by its name: `Scene:selected()`.
    Object,
    /// A thing scripts are handed, called on it: `shape:name()`.
    Thing,
}

/// A named object or a kind of thing that calls belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owner {
    /// Its name.
    pub name: &'static str,
    /// Which description file it's in (without its ending).
    pub file: &'static str,
    /// An object or a thing.
    pub kind: OwnerKind,
    /// What it is, in plain words.
    pub about: &'static str,
}

/// One call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Call {
    /// Whose call it is ([`OWNERS`]).
    pub owner: &'static str,
    /// Its name.
    pub name: &'static str,
    /// What it takes, as Lust writes it: `"int from, int count"`.
    pub takes: &'static str,
    /// What it gives back, as Lust writes it (`""` for nothing).
    pub gives: &'static str,
    /// What it does, in plain words.
    pub about: &'static str,
    /// The scripting version it arrived in.
    pub since: u32,
}

impl Call {
    /// Its body's key in `LUST_NATIVES`, as Lust's Lua calls it:
    /// `Scene_selected`, `Shape_name`.
    pub fn key(&self) -> String {
        format!("{}_{}", self.owner, self.name)
    }

    /// The values it takes, each as its kind and name.
    pub fn values(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.takes
            .split(',')
            .filter_map(|value| value.trim().rsplit_once(' '))
    }

    /// The names of the values it takes, joined: `"from, count"`.
    fn value_names(&self) -> String {
        self.values()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Its line in a description file.
    fn lust_line(&self) -> String {
        let gives = if self.gives.is_empty() {
            String::new()
        } else {
            format!("{} ", self.gives)
        };
        format!(r"method {gives}{}({}) native;", self.name, self.takes)
    }

    /// How LustScript writes a call to it: `Scene:selected()`,
    /// `shape:points(from, count)`.
    pub fn lust_call(&self) -> String {
        format!("{}:{}({})", self.on(), self.name, self.value_names())
    }

    /// How Lua writes a call to it, under the app's global `folder`:
    /// `aventail.Scene:selected()`, `shape:points(from, count)`.
    pub fn lua_call(&self, folder: &str) -> String {
        match owner(self.owner).map(|o| o.kind) {
            Some(OwnerKind::Thing) => self.lust_call(),
            _ => format!("{folder}.{}", self.lust_call()),
        }
    }

    /// What it's called on: an object's name, or a thing's kind in small
    /// letters (`shape`).
    fn on(&self) -> String {
        match owner(self.owner).map(|o| o.kind) {
            Some(OwnerKind::Thing) => self.owner.to_lowercase(),
            _ => self.owner.to_string(),
        }
    }
}

/// A kind of value calls hand back, declared as a Lust class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class {
    /// Its name.
    pub name: &'static str,
    /// Which description file it's in.
    pub file: &'static str,
    /// Its fields, one Lust line each: `"int id = 0;"`.
    pub fields: &'static [&'static str],
    /// What it is, in plain words.
    pub about: &'static str,
}

/// A fixed choice calls hand over, declared as a Lust state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    /// Its name.
    pub name: &'static str,
    /// Which description file it's in.
    pub file: &'static str,
    /// Its options, in order.
    pub options: &'static [&'static str],
    /// What it is, in plain words.
    pub about: &'static str,
}

/// The objects and things calls belong to, in the reference's order.
pub const OWNERS: &[Owner] = &[
    Owner {
        name: r"Scene",
        file: r"scene",
        kind: OwnerKind::Object,
        about: r"The scene the script runs on: every shape in the Objects panel.",
    },
    Owner {
        name: r"Shape",
        file: r"scene",
        kind: OwnerKind::Thing,
        about: r"One object in the Objects panel (a shape, a group, a curve…). Its `id` is a whole number that names it for the whole run.",
    },
    Owner {
        name: r"Selection",
        file: r"selection",
        kind: OwnerKind::Object,
        about: r"What's selected, and the mode the app was in when the script started.",
    },
    Owner {
        name: r"Project",
        file: r"project",
        kind: OwnerKind::Object,
        about: r"The open project.",
    },
    Owner {
        name: r"Script",
        file: r"script",
        kind: OwnerKind::Object,
        about: r"The script itself: what it says in the output, and how far it has got.",
    },
];

/// The kinds of values calls hand back.
pub const CLASSES: &[Class] = &[
    Class {
        name: r"Point",
        file: r"scene",
        fields: &[r"int id = 0;", r"vec3 position = [0, 0, 0];"],
        about: r"A point of a shape: its id, and where it is in the shape's own space, in centimetres.",
    },
    Class {
        name: r"Polygon",
        file: r"scene",
        fields: &[
            r"int id = 0;",
            r"int array corners = [];",
            r#"string material = "";"#,
            r"vec3 facing = [0, 0, 1];",
        ],
        about: r"A polygon of a shape: its id, the ids of its corner points in order, its material's name, and which way it faces (one unit long, in the shape's own space).",
    },
    Class {
        name: r"Edge",
        file: r"scene",
        fields: &[r"int a = 0;", r"int b = 0;"],
        about: r"An edge of a shape, named by the ids of the two points at its ends.",
    },
];

/// The fixed choices calls hand over.
pub const STATES: &[State] = &[State {
    name: r"Mode",
    file: r"selection",
    options: &[
        r"Model",
        r"Object",
        r"Points",
        r"Edges",
        r"Polygons",
        r"Uv",
        r"Paint",
        r"Material",
    ],
    about: r"The app's modes, as the mode palette lists them.",
}];

/// Every call, in the reference's order.
pub const CALLS: &[Call] = &[
    Call {
        owner: r"Scene",
        name: r"shapes",
        takes: r"",
        gives: r"Shape array",
        about: r"Every object in the scene, in the Objects panel's order (top to bottom, the ones inside a group after it).",
        since: 1,
    },
    Call {
        owner: r"Scene",
        name: r"selected",
        takes: r"",
        gives: r"Shape array",
        about: r"The selected objects, in the Objects panel's order.",
        since: 1,
    },
    Call {
        owner: r"Scene",
        name: r"find",
        takes: r"string name",
        gives: r"Shape",
        about: r"The first object with this name, in the Objects panel's order, or nothing.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"name",
        takes: r"",
        gives: r"string",
        about: r"Its name, as the Objects panel shows it.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"position",
        takes: r"",
        gives: r"vec3",
        about: r"Where its pivot is, in centimetres, relative to the group it's in (as the Coordinates panel shows it).",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"rotation",
        takes: r"",
        gives: r"vec3",
        about: r"How it's turned, in degrees: heading (around Z) in x, pitch (around X) in y and bank (around Y) in z.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"scale",
        takes: r"",
        gives: r"vec3",
        about: r"Its size factor along each axis (1 is as made).",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"parent",
        takes: r"",
        gives: r"Shape",
        about: r"The group it's in, or nothing at the top of the scene.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"children",
        takes: r"",
        gives: r"Shape array",
        about: r"The objects directly inside it, in order.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"shown",
        takes: r"",
        gives: r"bool",
        about: r"Whether its own switch shows it (its groups may still hide it).",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"pointCount",
        takes: r"",
        gives: r"int",
        about: r"How many points its final shape has (0 for a group).",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"points",
        takes: r"int from, int count",
        gives: r"Point array",
        about: r"Up to count of its points (at most 100,000 at a time), starting at the one numbered from (from 0), in a fixed order: read a heavy shape a chunk at a time.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"polygonCount",
        takes: r"",
        gives: r"int",
        about: r"How many polygons its final shape has (0 for a group).",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"polygons",
        takes: r"int from, int count",
        gives: r"Polygon array",
        about: r"Up to count of its polygons (at most 100,000 at a time), starting at the one numbered from (from 0), in a fixed order.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"hardEdges",
        takes: r"",
        gives: r"Edge array",
        about: r"The edges marked hard (where shading breaks).",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"seams",
        takes: r"",
        gives: r"Edge array",
        about: r"The edges marked as UV seams.",
        since: 1,
    },
    Call {
        owner: r"Shape",
        name: r"materials",
        takes: r"",
        gives: r"string array",
        about: r"The names of its materials, in slot order.",
        since: 1,
    },
    Call {
        owner: r"Selection",
        name: r"mode",
        takes: r"",
        gives: r"Mode",
        about: r"The mode the app was in when the script started.",
        since: 1,
    },
    Call {
        owner: r"Selection",
        name: r"points",
        takes: r"Shape shape",
        gives: r"int array",
        about: r"The ids of the shape's selected points.",
        since: 1,
    },
    Call {
        owner: r"Selection",
        name: r"edges",
        takes: r"Shape shape",
        gives: r"Edge array",
        about: r"The shape's selected edges.",
        since: 1,
    },
    Call {
        owner: r"Selection",
        name: r"polygons",
        takes: r"Shape shape",
        gives: r"int array",
        about: r"The ids of the shape's selected polygons.",
        since: 1,
    },
    Call {
        owner: r"Project",
        name: r"name",
        takes: r"",
        gives: r"string",
        about: r"The project's name, as its tab shows it.",
        since: 1,
    },
    Call {
        owner: r"Project",
        name: r"folder",
        takes: r"",
        gives: r"string",
        about: r"The folder the project is saved in, or empty text if it hasn't been saved yet.",
        since: 1,
    },
    Call {
        owner: r"Script",
        name: r"api",
        takes: r"",
        gives: r"int",
        about: r"The scripting version this app has.",
        since: 1,
    },
    Call {
        owner: r"Script",
        name: r"say",
        takes: r"string text",
        gives: r"",
        about: r"Writes a line in the output. print does the same.",
        since: 1,
    },
    Call {
        owner: r"Script",
        name: r"warn",
        takes: r"string text",
        gives: r"",
        about: r"Writes a warning in the output, marked so it stands out.",
        since: 1,
    },
    Call {
        owner: r"Script",
        name: r"progress",
        takes: r"float done, string what",
        gives: r"",
        about: r"Says how far the script has got, from 0 (just started) to 1 (finished), and what it's doing: the status bar shows it.",
        since: 1,
    },
];

/// The owner called `name`, if there is one.
pub fn owner(name: &str) -> Option<&'static Owner> {
    OWNERS.iter().find(|o| o.name == name)
}

/// What the first line of each description file says after who made it:
/// how a file describing the app is told apart, without the app's name in
/// the code.
pub const DESCRIPTION_FILE: &str = r": what scripts can ask of the app.";

/// The description files, as (path, text) with `/` between folders, in
/// the app's scripting folder (`aventail/scene.lsc` and the rest).
pub fn descriptions(names: &Names) -> Vec<(String, String)> {
    let mut files: Vec<&str> = Vec::new();
    for file in OWNERS.iter().map(|o| o.file) {
        if !files.contains(&file) {
            files.push(file);
        }
    }
    files
        .into_iter()
        .map(|file| {
            (
                format!("{}/{file}.lsc", names.folder),
                description(names, file),
            )
        })
        .collect()
}

/// One description file's text.
fn description(names: &Names, file: &str) -> String {
    let mut text = format!(
        r"-- Made by {}{DESCRIPTION_FILE} Scripting version {API_VERSION}.
-- The app writes it again, so changes made here don't last.

",
        names.made_by
    );
    let fetched = fetched_by(file);
    for (other, names_there) in &fetched {
        text.push_str(&format!(
            r#"fetch "{}/{other}":[{}];
"#,
            names.folder,
            names_there.join(", ")
        ));
    }
    if !fetched.is_empty() {
        text.push('\n');
    }
    let mut passed: Vec<&str> = Vec::new();
    for state in STATES.iter().filter(|s| s.file == file) {
        text.push_str(&format!(
            r"-- {}
state {} = [{}];

",
            state.about,
            state.name,
            state.options.join(", ")
        ));
        passed.push(state.name);
    }
    for class in CLASSES.iter().filter(|c| c.file == file) {
        text.push_str(&format!(
            r"-- {}
class {} {{
    has {{
",
            class.about, class.name
        ));
        for field in class.fields {
            text.push_str(&format!(
                r"        {field}
"
            ));
        }
        text.push_str("    }\n}\n\n");
        passed.push(class.name);
    }
    for owner in OWNERS.iter().filter(|o| o.file == file) {
        text.push_str(&format!("-- {}\n", owner.about));
        match owner.kind {
            OwnerKind::Thing => {
                text.push_str(&format!(
                    r"class {} {{
    has {{
        int id = 0;
    }}

",
                    owner.name
                ));
            }
            OwnerKind::Object => text.push_str(&format!(
                r"object {} {{
",
                owner.name
            )),
        }
        for call in CALLS.iter().filter(|c| c.owner == owner.name) {
            text.push_str(&format!(
                "    -- {}\n    {}\n\n",
                call.about,
                call.lust_line()
            ));
        }
        text.truncate(text.trim_end().len());
        text.push_str("\n}\n\n");
        passed.push(owner.name);
    }
    text.push_str(&format!(
        r"pass {};
end
",
        passed.join(", ")
    ));
    text
}

/// The names a description file uses from the others, by file: what its
/// calls take and give that's declared elsewhere.
fn fetched_by(file: &str) -> Vec<(&'static str, Vec<&'static str>)> {
    let declared = || {
        STATES
            .iter()
            .map(|s| (s.name, s.file))
            .chain(CLASSES.iter().map(|c| (c.name, c.file)))
            .chain(OWNERS.iter().map(|o| (o.name, o.file)))
    };
    let mut wanted: Vec<(&str, Vec<&str>)> = Vec::new();
    for call in CALLS
        .iter()
        .filter(|c| owner(c.owner).is_some_and(|o| o.file == file))
    {
        let kinds = call
            .values()
            .map(|(kind, _)| kind)
            .chain(std::iter::once(call.gives));
        for kind in kinds {
            let kind = kind.trim_end_matches(r" array");
            let Some((name, there)) = declared().find(|(name, _)| *name == kind) else {
                continue;
            };
            if there == file {
                continue;
            }
            match wanted.iter_mut().find(|(f, _)| *f == there) {
                Some((_, list)) if !list.contains(&name) => list.push(name),
                Some(_) => {}
                None => wanted.push((there, vec![name])),
            }
        }
    }
    wanted
}

/// The manual's reference page: every call, what it takes and gives, in
/// LustScript and Lua side by side, with the version it arrived in.
pub fn reference_page(names: &Names) -> String {
    let folder = &names.folder;
    let mut page = String::from(
        r"# Script reference

> Every call a script can make, what it takes and gives back, in LustScript and Lua.

<!-- Made from the scripting crate's table (crates/script/src/describe.rs): a test fails when it's out of date. Write it again with UPDATE_REFERENCE=1 cargo test -p app --test script_reference. -->

",
    );
    page.push_str(&format!(
        r#"Scripts are explained in [Scripting](scripts.md); words like *array* and *state* are in [Concepts](concepts.md#scripting). This page lists **scripting version {API_VERSION}**: a script says the version it was written for with an `-- api: {API_VERSION}` line at its top.

In LustScript, fetch what you use from the app's own files first, such as `fetch "{folder}/scene":[Scene, Shape];`. In Lua everything is under `{folder}.`, such as `{folder}.Scene:selected()`; a shape's own calls are written the same in both (`shape:name()`).

Version {API_VERSION} only reads: no call changes the scene yet.

"#
    ));
    for owner in OWNERS {
        page.push_str(&format!(
            r#"## {}

{}

LustScript: `fetch "{folder}/{}":{};`

"#,
            owner.name, owner.about, owner.file, owner.name
        ));
        page.push_str(
            r"| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
",
        );
        for call in CALLS.iter().filter(|c| c.owner == owner.name) {
            let gives = if call.gives.is_empty() {
                "nothing"
            } else {
                call.gives
            };
            page.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                call.lust_call(),
                call.lua_call(folder),
                gives,
                call.about,
                call.since
            ));
        }
        page.push('\n');
    }
    page.push_str(
        r"## What calls hand back

",
    );
    for class in CLASSES {
        let fields: Vec<String> = class
            .fields
            .iter()
            .map(|f| format!("`{}`", f.split(" = ").next().unwrap_or(f)))
            .collect();
        page.push_str(&format!(
            r#"- **{}** (`fetch "{folder}/{}":{};`): {} Fields: {}.
"#,
            class.name,
            class.file,
            class.name,
            class.about,
            fields.join(", ")
        ));
    }
    for state in STATES {
        let options: Vec<String> = state
            .options
            .iter()
            .map(|o| format!("`{}.{o}`", state.name))
            .collect();
        page.push_str(&format!(
            r#"- **{}** (`fetch "{folder}/{}":{};`): {} One of {}. In Lua it's the option's name as text (`"{}"`), also kept as `{folder}.{}.{}`.
"#,
            state.name,
            state.file,
            state.name,
            state.about,
            options.join(", "),
            state.options[0],
            state.name,
            state.options[0]
        ));
    }
    page.push_str(
        r"
## Related

- [Scripting](scripts.md): running a script, the output, mistakes and limits.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, sandbox and the other words.
",
    );
    page
}

#[cfg(test)]
#[path = "describe_tests.rs"]
mod tests;
