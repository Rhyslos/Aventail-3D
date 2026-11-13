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
use crate::settings::{ColourGradient, SettingKind, SettingSpec, SettingValue, Settings};
use crate::value::{MaterialMaps, Value};
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
    /// Boxes that bring in the model's own maps in the painter
    /// (curvature, occlusion): listed as "From the model" (milestone 5).
    Model,
    /// Boxes that change a shape or height (levels, blur, blend).
    Shape,
    /// Boxes that add small detail (cracks, scratches).
    Detail,
    /// Boxes that make or change colour.
    Colour,
    /// Boxes that move over time (Time, Scroll, Pulse, Flow; milestone
    /// 15): listed as "Moving".
    Move,
    /// Boxes that make and finish a whole material (listed as "Finish",
    /// the last step of the working order).
    Material,
    /// Your own boxes (milestone 7, [`crate::own`]): listed last.
    Own,
}

impl Group {
    /// Every group, in order.
    pub const ALL: [Self; 8] = [
        Self::Make,
        Self::Model,
        Self::Shape,
        Self::Detail,
        Self::Colour,
        Self::Move,
        Self::Material,
        Self::Own,
    ];

    /// Its plain name.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Make => text!("Make"),
            Self::Model => text!("From the model"),
            Self::Shape => text!("Shape"),
            Self::Detail => text!("Detail"),
            Self::Colour => text!("Colour"),
            Self::Move => text!("Moving"),
            Self::Material => text!("Finish"),
            Self::Own => text!("Your own boxes"),
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
    /// Whether it changes over time (Time, Scroll, Pulse and Flow, Phase
    /// 12's milestone 15, [decision 0277](../../../docs/decisions/0277-animated-boxes.md);
    /// an own box holding one). Its result depends on the job's time
    /// ([`Job::time`]), which is part of its cache key; an export bakes it
    /// at the graph's moment ([`crate::Graph::moment`]) and says so
    /// ([`crate::baking`]).
    pub animated: bool,
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

    /// Whether it has a length setting (in centimetres), so the graph's
    /// real-world size changes it.
    pub fn uses_real_size(&self) -> bool {
        self.settings
            .iter()
            .any(|s| matches!(s.kind, SettingKind::Length { .. }))
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

    /// Changes whenever the kind's working changes while the app runs
    /// (your own boxes, edited inside): part of each result's key, so an
    /// old result is never used for a changed box. 0 for the standard
    /// boxes, which never change.
    fn version(&self) -> u64 {
        0
    }

    /// The definition, for one of your own boxes ([`crate::own`]).
    fn own(&self) -> Option<&Arc<crate::own::OwnBox>> {
        None
    }

    /// Makes its pictures as [`Self::run`] does, except that an output
    /// may be left empty (`None`) when what it is made from isn't plugged
    /// in: Transform and Repeat move a height, a colour and a material,
    /// each only when plugged in ([decision 0270](../../../docs/decisions/0270-tiling-and-moving-colours.md)).
    /// The evaluator calls this; a box wired to an empty output says why
    /// it can't run. Every output is made unless a box says otherwise.
    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        Ok(self.run(job)?.into_iter().map(Some).collect())
    }

    /// Whether it can't run with nothing plugged in at all, though none of
    /// its inputs is needed on its own (Transform and Repeat: a height, a
    /// colour or a material). Such a box is never offered as a maker.
    fn needs_an_input(&self) -> bool {
        false
    }

    /// Whether it is drawn on the shape canvas (the Shape box, milestone
    /// 11): only such a box keeps a drawing ([`crate::GraphEdit::SetDrawing`]),
    /// which its job is given ([`Job::drawing`]).
    fn takes_drawing(&self) -> bool {
        false
    }

    /// Whether it is painted on in the Material editor (the Painted box,
    /// milestone 20): only such a box keeps a painting
    /// ([`crate::GraphEdit::SetPainting`]), which its job is given
    /// ([`Job::painting`]).
    fn takes_painting(&self) -> bool {
        false
    }

    /// Whether it is made from a photo or painting brought in (the Image
    /// box, milestone 21): only such a box keeps one
    /// ([`crate::GraphEdit::SetPhoto`]), which its job is given
    /// ([`Job::photo`]).
    fn takes_photo(&self) -> bool {
        false
    }

    /// Whether it stamps small pictures brought in (the Stamps box,
    /// milestone 22): only such a box keeps them
    /// ([`crate::GraphEdit::SetStamps`]), which its job is given
    /// ([`Job::stamps`]).
    fn takes_stamps(&self) -> bool {
        false
    }

    /// Why output `plug` of a box of this kind, set as `settings` say,
    /// gives nothing, when the box chose to leave it empty for a reason of
    /// its own (not for want of an input): the Painted box gives only the
    /// output of the kind it paints. `None` uses the evaluator's own
    /// words.
    fn why_empty(&self, _settings: &Settings, _plug: &str) -> Option<String> {
        None
    }

    /// Whether it reads the project's palettes ([`Job::palettes`]): a box
    /// with a palette setting, or your own box holding one. Their key is
    /// then part of its result's key.
    fn uses_palettes(&self) -> bool {
        self.info()
            .settings
            .iter()
            .any(|s| matches!(s.kind, SettingKind::Palette { .. }))
    }
}

/// The real-world size a graph's texture covers until its Material out
/// says otherwise, in centimetres (Material out's standard).
pub const STANDARD_REAL_SIZE_CM: f32 = 100.0;

/// The smallest real-world size, in centimetres (Material out's smallest).
pub const MIN_REAL_SIZE_CM: f32 = 1.0;

/// What a box runs with: its size, seed, settings and inputs.
#[derive(Debug)]
pub struct Job<'a> {
    size: Size,
    real_size_cm: f32,
    seed: u64,
    graph_seed: u32,
    threads: usize,
    info: &'a BoxInfo,
    settings: Vec<SettingValue>,
    inputs: Vec<Option<Value>>,
    model: Option<Arc<crate::model::ModelMaps>>,
    drawing: Option<Arc<shapes::Drawing>>,
    painting: Option<Arc<crate::painting::Painting>>,
    photo: Option<Arc<crate::photo::Photo>>,
    stamps: Option<Arc<crate::stamp::Stamps>>,
    time: f32,
    palettes: crate::palettes::Palettes,
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
            real_size_cm: STANDARD_REAL_SIZE_CM,
            seed,
            graph_seed: 0,
            threads: threads.max(1),
            info,
            settings: settings.resolved(&info.settings),
            inputs,
            model: None,
            drawing: None,
            painting: None,
            photo: None,
            stamps: None,
            time: 0.0,
            palettes: crate::palettes::Palettes::default(),
        }
    }

    /// The same job with the project's palettes `palettes` (milestone 19).
    pub fn with_palettes(mut self, palettes: crate::palettes::Palettes) -> Self {
        self.palettes = palettes;
        self
    }

    /// The project's palettes (none unless the evaluator was given a
    /// project's registry).
    pub fn palettes(&self) -> &crate::palettes::Palettes {
        &self.palettes
    }

    /// The same job at `seconds` into the animation (animated boxes,
    /// milestone 15). Not a real number, or below 0, is 0.
    pub fn with_time(mut self, seconds: f32) -> Self {
        self.time = crate::graph::usable_moment(seconds);
        self
    }

    /// How many seconds into the animation it is worked out at: the
    /// preview's player while it plays, else the graph's moment.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// The same job with the box's drawing (the Shape box's).
    pub fn with_drawing(mut self, drawing: Option<Arc<shapes::Drawing>>) -> Self {
        self.drawing = drawing;
        self
    }

    /// The same job with the box's photo (the Image box's).
    pub fn with_photo(mut self, photo: Option<Arc<crate::photo::Photo>>) -> Self {
        self.photo = photo;
        self
    }

    /// The photo brought in on the box, if any (the Image box's).
    pub fn photo(&self) -> Option<&Arc<crate::photo::Photo>> {
        self.photo.as_ref()
    }

    /// The same job with the box's stamps (the Stamps box's).
    pub fn with_stamps(mut self, stamps: Option<Arc<crate::stamp::Stamps>>) -> Self {
        self.stamps = stamps;
        self
    }

    /// The stamps brought in on the box, if any (the Stamps box's).
    pub fn stamps(&self) -> Option<&Arc<crate::stamp::Stamps>> {
        self.stamps.as_ref()
    }

    /// The same job with the box's painting (the Painted box's).
    pub fn with_painting(mut self, painting: Option<Arc<crate::painting::Painting>>) -> Self {
        self.painting = painting;
        self
    }

    /// What is painted on the box, if anything.
    pub fn painting(&self) -> Option<&Arc<crate::painting::Painting>> {
        self.painting.as_ref()
    }

    /// The box's drawing on the shape canvas, if it has one.
    pub fn drawing(&self) -> Option<&Arc<shapes::Drawing>> {
        self.drawing.as_ref()
    }

    /// The same job with the model's maps (in the painter), for the From
    /// the model boxes. Ignored unless they are this job's size.
    pub fn with_model(mut self, maps: Option<Arc<crate::model::ModelMaps>>) -> Self {
        self.model = maps.filter(|m| m.size() == self.size);
        self
    }

    /// The same job in a graph with seed `seed` (your own boxes run the
    /// boxes inside with it, so making and opening one keeps its look).
    pub fn with_graph_seed(mut self, seed: u32) -> Self {
        self.graph_seed = seed;
        self
    }

    /// The seed of the graph the box is in.
    pub fn graph_seed(&self) -> u32 {
        self.graph_seed
    }

    /// Every setting's value, in the kind's settings' order, made allowed.
    pub fn resolved_settings(&self) -> &[SettingValue] {
        &self.settings
    }

    /// The model's map `map`, in the painter; `None` in the Material mode
    /// (the From the model boxes then show a stand-in).
    pub fn model_map(&self, map: crate::model::ModelMap) -> Option<&Arc<Grey>> {
        self.model.as_ref()?.get(map)
    }

    /// The same job for a graph whose texture covers `cm` centimetres
    /// (Material out's real-world size; [`STANDARD_REAL_SIZE_CM`] until
    /// set). Kept from [`MIN_REAL_SIZE_CM`] up; not a real number is the
    /// standard.
    pub fn with_real_size(mut self, cm: f32) -> Self {
        self.real_size_cm = if cm.is_finite() {
            cm.max(MIN_REAL_SIZE_CM)
        } else {
            STANDARD_REAL_SIZE_CM
        };
        self
    }

    /// Pixels across.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The plain name of the box it runs.
    pub fn box_name(&self) -> String {
        self.info.name.show()
    }

    /// How many centimetres the texture covers.
    pub fn real_size_cm(&self) -> f32 {
        self.real_size_cm
    }

    /// The length setting `key` (in centimetres) in pixels at this job's
    /// size, never more than the texture is across.
    pub fn length_px(&self, key: &str) -> f32 {
        let n = self.size.get() as f32;
        (self.number(key) / self.real_size_cm * n).clamp(0.0, n)
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
        self.settings.get(at).cloned()
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

    /// The gradient setting `key` (black to white if it has none).
    pub fn gradient(&self, key: &str) -> ColourGradient {
        match self.setting(key) {
            Some(SettingValue::Gradient(g)) => g,
            _ => ColourGradient::two([0.0; 3], [1.0; 3]),
        }
    }

    /// The palette setting `key`: a palette's name, or empty for the
    /// project's first.
    pub fn palette(&self, key: &str) -> String {
        match self.setting(key) {
            Some(SettingValue::Palette(name)) => name,
            _ => String::new(),
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
        self.grey(key).ok_or_else(|| self.missing(key))
    }

    /// The colours plugged into needed input `key`, or a refusal.
    pub fn needed_colour(&self, key: &str) -> Result<&Arc<Rgb>, Refusal> {
        self.input(key)
            .and_then(Value::colour)
            .ok_or_else(|| self.missing(key))
    }

    /// The material plugged into needed input `key`, or a refusal.
    pub fn needed_material(&self, key: &str) -> Result<&Arc<MaterialMaps>, Refusal> {
        self.input(key)
            .and_then(Value::material)
            .ok_or_else(|| self.missing(key))
    }

    fn missing(&self, key: &str) -> Refusal {
        Refusal::new(trf!(
            "{} has nothing plugged into {}.",
            self.info.name.show(),
            key
        ))
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

    /// `image` with `f` applied to every colour pixel, shared over the
    /// cores.
    pub fn map_rgb(&self, image: &Rgb, f: impl Fn([f32; 3]) -> [f32; 3] + Sync) -> Rgb {
        let n = i64::from(image.size());
        parallel::fill(self.size, self.threads, |x, y| {
            f(image.at(i64::from(x) % n, i64::from(y) % n))
        })
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
    /// The project's colour palettes (milestone 19): none in
    /// [`Self::standard`]; a project's registry carries its own.
    palettes: crate::palettes::Palettes,
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

    /// The project's colour palettes, which boxes with a palette setting
    /// colour with (Palette map, milestone 19).
    pub fn palettes(&self) -> &crate::palettes::Palettes {
        &self.palettes
    }

    /// The same kinds with the project's palettes `palettes`.
    pub fn with_palettes(mut self, palettes: crate::palettes::Palettes) -> Self {
        self.palettes = palettes;
        self
    }
}
