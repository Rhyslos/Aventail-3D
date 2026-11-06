//! Kinds of box: what each is ([`BoxInfo`]), how it works ([`BoxType`]),
//! and the list of every kind the app knows ([`Registry`]).
//!
//! A new kind of box is a type implementing [`BoxType`], added to
//! [`Registry::with_standard_boxes`]. Its [`BoxInfo`] says everything the
//! interface shows (plain name, group, "What does this do?", plugs,
//! settings, what usually comes next); its [`BoxType::run`] makes its
//! pictures from a [`Job`]. Graphs name kinds by their id, which is saved
//! in files, so an id never changes once released.

use crate::image::{Grey, Image, Rgb, Size};
use crate::kind::ValueKind;
use crate::parallel;
use crate::refusal::Refusal;
use crate::settings::{SettingKind, SettingSpec, SettingValue, Settings};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};
use words::{text, tr, trf};

/// Text a person reads: an English key that goes through the language
/// layer, or a person's own words (a box they made and named), shown as
/// they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// English text made with `text!`, translated when shown.
    Key(&'static str),
    /// A person's own words.
    Own(String),
}

impl Label {
    /// The text to show, in the language in use.
    pub fn show(&self) -> String {
        match self {
            Self::Key(key) => tr(key).to_string(),
            Self::Own(own) => own.clone(),
        }
    }
}

/// The groups boxes are listed in, in the order experts work: big shapes
/// first, then shaping, detail, colour, and the material last (named
/// "Finish" in the box search).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    /// Boxes that make a pattern from nothing (noise, tiles).
    Make,
    /// Boxes that change a shape or height (levels, blur, blend).
    Shape,
    /// Boxes that add small detail (cracks, scratches).
    Detail,
    /// Boxes that make or change colour.
    Colour,
    /// Boxes that make and finish a whole material (listed as "Finish",
    /// the last step of the working order).
    Material,
}

impl Group {
    /// Every group, in order.
    pub const ALL: [Self; 5] = [
        Self::Make,
        Self::Shape,
        Self::Detail,
        Self::Colour,
        Self::Material,
    ];

    /// Its plain name.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Make => text!("Make"),
            Self::Shape => text!("Shape"),
            Self::Detail => text!("Detail"),
            Self::Colour => text!("Colour"),
            Self::Material => text!("Finish"),
        })
    }
}

/// One plug on a box: an input or an output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlugSpec {
    /// Its key, saved in files with the wires: never changed once
    /// released.
    pub key: Cow<'static, str>,
    /// Its plain name.
    pub name: Label,
    /// What it carries.
    pub kind: ValueKind,
    /// For an input: whether the box can't run with nothing plugged in.
    /// An input that isn't needed has a plain standard (the box says
    /// which in its "What does this do?").
    pub needed: bool,
}

impl PlugSpec {
    /// A plug from an English key and name.
    pub fn new(key: &'static str, name: &'static str, kind: ValueKind, needed: bool) -> Self {
        Self {
            key: Cow::Borrowed(key),
            name: Label::Key(name),
            kind,
            needed,
        }
    }
}

/// Everything about a kind of box the interface shows.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxInfo {
    /// Its id, saved in files: never changed once released.
    pub id: Cow<'static, str>,
    /// Its plain name ("Material from height").
    pub name: Label,
    /// The group it is listed in.
    pub group: Group,
    /// "What does this do?": a few plain sentences.
    pub about: Label,
    /// Its inputs, in the order drawn.
    pub inputs: Vec<PlugSpec>,
    /// Its outputs, in the order drawn.
    pub outputs: Vec<PlugSpec>,
    /// Its settings, in the order shown.
    pub settings: Vec<SettingSpec>,
    /// The ids of the boxes that usually come next, best first.
    pub comes_next: Vec<Cow<'static, str>>,
    /// Whether a graph may hold only one (Material out: a graph makes one
    /// material).
    pub one_per_graph: bool,
}

impl BoxInfo {
    /// Its input with key `key`.
    pub fn input(&self, key: &str) -> Option<&PlugSpec> {
        self.inputs.iter().find(|p| p.key == key)
    }

    /// Its output with key `key`.
    pub fn output(&self, key: &str) -> Option<&PlugSpec> {
        self.outputs.iter().find(|p| p.key == key)
    }

    /// Its setting with key `key`.
    pub fn setting(&self, key: &str) -> Option<&SettingSpec> {
        self.settings.iter().find(|s| s.key == key)
    }

    /// Whether it has a seed setting (so the graph's seed changes it).
    pub fn uses_seed(&self) -> bool {
        self.settings
            .iter()
            .any(|s| matches!(s.kind, SettingKind::Seed))
    }
}

/// A kind of box: what it is and how it works.
pub trait BoxType: Send + Sync + std::fmt::Debug {
    /// What it is.
    fn info(&self) -> &BoxInfo;

    /// Makes its pictures: one value per output, in order, each of the
    /// output's kind and the job's size. A box with no outputs (Material
    /// out) gives back what its picture shows. A reason in plain words
    /// when it can't (the evaluator has already checked that every needed
    /// input is plugged in).
    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal>;
}

/// What a box runs with: its size, seed, settings and inputs.
#[derive(Debug)]
pub struct Job<'a> {
    size: Size,
    seed: u64,
    threads: usize,
    info: &'a BoxInfo,
    settings: Vec<SettingValue>,
    inputs: Vec<Option<Value>>,
}

impl<'a> Job<'a> {
    /// A job for a box of kind `info` at `size`, with `settings` (made
    /// allowed) and `inputs` in its inputs' order, each converted to its
    /// plug's kind (`None`: nothing plugged in). `threads` is how many
    /// cores to share the pixels over.
    pub fn new(
        info: &'a BoxInfo,
        size: Size,
        seed: u64,
        threads: usize,
        settings: &Settings,
        inputs: Vec<Option<Value>>,
    ) -> Self {
        Self {
            size,
            seed,
            threads: threads.max(1),
            info,
            settings: settings.resolved(&info.settings),
            inputs,
        }
    }

    /// Pixels across.
    pub fn size(&self) -> Size {
        self.size
    }

    /// How many cores to share the work over.
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// The box's own seed mixed with the graph's.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn setting(&self, key: &str) -> Option<SettingValue> {
        let at = self.info.settings.iter().position(|s| s.key == key)?;
        self.settings.get(at).copied()
    }

    /// The number setting `key` (0 if it has none).
    pub fn number(&self, key: &str) -> f32 {
        match self.setting(key) {
            Some(SettingValue::Number(n)) => n,
            _ => 0.0,
        }
    }

    /// The whole-number setting `key` (0 if it has none).
    pub fn whole(&self, key: &str) -> i32 {
        match self.setting(key) {
            Some(SettingValue::Whole(n)) => n,
            _ => 0,
        }
    }

    /// The colour setting `key` (black if it has none).
    pub fn colour(&self, key: &str) -> [f32; 3] {
        match self.setting(key) {
            Some(SettingValue::Colour(c)) => c,
            _ => [0.0; 3],
        }
    }

    /// The choice setting `key`, by index (0 if it has none).
    pub fn choice(&self, key: &str) -> u32 {
        match self.setting(key) {
            Some(SettingValue::Choice(c)) => c,
            _ => 0,
        }
    }

    /// The switch setting `key` (off if it has none).
    pub fn switch(&self, key: &str) -> bool {
        matches!(self.setting(key), Some(SettingValue::Switch(true)))
    }

    /// What is plugged into input `key`, as that input's kind.
    pub fn input(&self, key: &str) -> Option<&Value> {
        let at = self.info.inputs.iter().position(|p| p.key == key)?;
        self.inputs.get(at)?.as_ref()
    }

    /// The greys plugged into input `key` (a shape or height input).
    pub fn grey(&self, key: &str) -> Option<&Arc<Grey>> {
        self.input(key).and_then(Value::grey)
    }

    /// The greys plugged into needed input `key`, or a refusal saying it
    /// is missing (the evaluator checks first, so this is a safeguard).
    pub fn needed_grey(&self, key: &str) -> Result<&Arc<Grey>, Refusal> {
        self.grey(key).ok_or_else(|| {
            Refusal::new(trf!(
                "{} has nothing plugged into {}.",
                self.info.name.show(),
                key
            ))
        })
    }

    /// A grey picture made by `f(x, y)` for every pixel, shared in tiles
    /// over the cores.
    pub fn fill_grey(&self, f: impl Fn(u32, u32) -> f32 + Sync) -> Grey {
        parallel::fill(self.size, self.threads, f)
    }

    /// A colour picture made by `f(x, y)` for every pixel, the same way.
    pub fn fill_rgb(&self, f: impl Fn(u32, u32) -> [f32; 3] + Sync) -> Rgb {
        parallel::fill(self.size, self.threads, f)
    }

    /// `image` with `f` applied to every pixel, shared over the cores.
    pub fn map_grey(&self, image: &Grey, f: impl Fn(f32) -> f32 + Sync) -> Grey {
        let n = i64::from(image.size());
        let out: Image<f32> = parallel::fill(self.size, self.threads, |x, y| {
            f(image.at(i64::from(x) % n, i64::from(y) % n))
        });
        out
    }
}

/// Every kind of box the app knows.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    types: Vec<Arc<dyn BoxType>>,
}

impl Registry {
    /// No kinds at all.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The app's own boxes ([`crate::boxes`]).
    pub fn with_standard_boxes() -> Self {
        let mut registry = Self::empty();
        for kind in crate::boxes::standard() {
            // The standard ids are all different (a test checks).
            let _ = registry.add(kind);
        }
        registry
    }

    /// The app's own boxes, made once and shared.
    pub fn standard() -> &'static Registry {
        static STANDARD: OnceLock<Registry> = OnceLock::new();
        STANDARD.get_or_init(Self::with_standard_boxes)
    }

    /// Adds a kind; refused when its id is taken.
    pub fn add(&mut self, kind: Arc<dyn BoxType>) -> Result<(), Refusal> {
        if self.get(&kind.info().id).is_some() {
            return Err(Refusal::new(trf!(
                "There is already a box called {}.",
                kind.info().name.show()
            )));
        }
        self.types.push(kind);
        Ok(())
    }

    /// The kind with id `id`.
    pub fn get(&self, id: &str) -> Option<&Arc<dyn BoxType>> {
        self.types.iter().find(|t| t.info().id == id)
    }

    /// Every kind, by group, then in the order added.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn BoxType>> {
        Group::ALL
            .into_iter()
            .flat_map(|g| self.types.iter().filter(move |t| t.info().group == g))
    }
}
