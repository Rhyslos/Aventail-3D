//! Trim sheet (Phase 12, milestone 26,
//! [decision 0289](../../../../docs/decisions/0289-trim-sheet-box.md)): strips
//! of different materials stacked on one texture, each repeating left to
//! right, for walls, edges and pipes.
//!
//! Each Strip plug with a material in it becomes a strip. The strips lie
//! on the project's trim rows (made in the UV Editor's Trim menu, decision
//! 0106), so the strips of a model laid on a row with Put on trim row get
//! that row's material; with no rows, or by choice, they share the height
//! by their own Height settings. A strip either **cuts** a band from its
//! material at the material's own size (the strips keep one real-world
//! scale) or **shrinks** the whole material to fit its height (a pipe's
//! round profile). Between the strips a thin gap (a colour, or a material
//! plugged into Between strips) and bevelled edges sinking into it make
//! each strip read as a piece of moulding; the normals, occlusion and
//! curvature follow, so weathering after it gathers in the grooves.
//!
//! The sheet repeats left to right because every material does; top to
//! bottom it repeats too (its last strip meets its first in the middle of
//! a gap), so it tiles like any other texture.

use super::material_from_height::{curvature, normal, occlusion};
use super::{choice, colour, info, length, number, whole};
use crate::image::{Grey, Image, Rgb, Size};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec, Registry};
use crate::settings::{SettingValue, Settings};
use crate::trim::TrimRows;
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::{text, tr, trf};

/// The Trim sheet box's id.
pub const TRIM_SHEET: &str = "trim_sheet";

/// How many strips a sheet can have (its Strip plugs).
pub const STRIPS: usize = 8;

/// The Strip plugs' keys, top to bottom.
pub const STRIP_KEYS: [&str; STRIPS] = [
    "strip1", "strip2", "strip3", "strip4", "strip5", "strip6", "strip7", "strip8",
];

const STRIP_NAMES: [&str; STRIPS] = [
    text!("Strip 1"),
    text!("Strip 2"),
    text!("Strip 3"),
    text!("Strip 4"),
    text!("Strip 5"),
    text!("Strip 6"),
    text!("Strip 7"),
    text!("Strip 8"),
];

const HEIGHT_KEYS: [&str; STRIPS] = [
    "height1", "height2", "height3", "height4", "height5", "height6", "height7", "height8",
];

const HEIGHT_NAMES: [&str; STRIPS] = [
    text!("Strip 1 height"),
    text!("Strip 2 height"),
    text!("Strip 3 height"),
    text!("Strip 4 height"),
    text!("Strip 5 height"),
    text!("Strip 6 height"),
    text!("Strip 7 height"),
    text!("Strip 8 height"),
];

const FIT_KEYS: [&str; STRIPS] = [
    "fit1", "fit2", "fit3", "fit4", "fit5", "fit6", "fit7", "fit8",
];

const FIT_NAMES: [&str; STRIPS] = [
    text!("Strip 1 fit"),
    text!("Strip 2 fit"),
    text!("Strip 3 fit"),
    text!("Strip 4 fit"),
    text!("Strip 5 fit"),
    text!("Strip 6 fit"),
    text!("Strip 7 fit"),
    text!("Strip 8 fit"),
];

/// Strips on (the `lay` setting): the project's trim rows.
pub const ON_ROWS: u32 = 0;
/// Strips on: heights of their own.
pub const OWN_HEIGHTS: u32 = 1;

/// A strip's Fit: a band cut from the material at its own size.
pub const CUT: u32 = 0;
/// A strip's Fit: the whole material shrunk into the strip.
pub const SHRINK: u32 = 1;

/// The Edges setting's choices.
const SQUARE: u32 = 0;
const CHAMFERED: u32 = 1;
const ROUNDED: u32 = 2;

/// How deep a sink of 1 bends the normals, in hundredths of the texture's
/// width (as the weathering boxes raise theirs).
const BEND: f32 = 2.0;

/// The most times a shrunk material repeats across its strip.
const MOST_ACROSS: f32 = 256.0;

/// The plain gap's height and roughness, with nothing in Between strips.
const GAP_HEIGHT: f32 = 0.5;
const GAP_ROUGHNESS: f32 = 0.9;

/// Where the strips come from, as laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Laid {
    /// On the project's trim rows, strip 1 on the top row.
    OnRows,
    /// Sharing the height by their own Height settings, as chosen.
    OwnHeights,
    /// On the project's trim rows was chosen, but it has none yet: by
    /// their own heights until it does.
    NoRows,
}

/// Where one strip lies on the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct StripPlace {
    /// Which Strip plug (0 for Strip 1).
    pub strip: usize,
    /// Where it starts, as a share of the height from the top.
    pub from: f32,
    /// Where it ends.
    pub to: f32,
    /// The trim row it lies on, if on the project's rows.
    pub row: Option<String>,
    /// Whether its whole material is shrunk into it (else a band is cut).
    pub shrink: bool,
}

/// Where a Trim sheet box's strips lie.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Each strip with a material in it and a place, top to bottom.
    pub places: Vec<StripPlace>,
    /// Where they come from.
    pub laid: Laid,
    /// Strips with a material in them but no trim row to lie on (past the
    /// project's last row).
    pub past_rows: Vec<usize>,
    /// How many trim rows the project has.
    pub rows: usize,
}

/// Where the strips lie: `lay` is the Strips on setting, `heights` and
/// `fits` each strip's, `plugged` which Strip plugs have a material in
/// them, `rows` the project's trim rows.
pub fn layout(
    lay: u32,
    heights: &[i32; STRIPS],
    fits: &[u32; STRIPS],
    plugged: &[bool; STRIPS],
    rows: &TrimRows,
) -> Layout {
    let laid = match (lay, rows.is_empty()) {
        (ON_ROWS, false) => Laid::OnRows,
        (ON_ROWS, true) => Laid::NoRows,
        _ => Laid::OwnHeights,
    };
    let mut places = Vec::new();
    let mut past_rows = Vec::new();
    if laid == Laid::OnRows {
        for (i, _) in plugged.iter().enumerate().filter(|(_, p)| **p) {
            match rows.get(i) {
                Some(row) => places.push(StripPlace {
                    strip: i,
                    from: row.from,
                    to: row.to,
                    row: Some(row.name.clone()),
                    shrink: fits[i] == SHRINK,
                }),
                None => past_rows.push(i),
            }
        }
    } else {
        let share = |i: usize| heights[i].max(1) as f32;
        let total: f32 = (0..STRIPS).filter(|i| plugged[*i]).map(share).sum();
        let mut at = 0.0;
        for i in (0..STRIPS).filter(|i| plugged[*i]) {
            let from = at / total;
            at += share(i);
            places.push(StripPlace {
                strip: i,
                from,
                to: at / total,
                row: None,
                shrink: fits[i] == SHRINK,
            });
        }
    }
    Layout {
        places,
        laid,
        past_rows,
        rows: rows.len(),
    }
}

/// [`layout`] for a Trim sheet box with `settings` (kept within range,
/// standards where unset), as the interface shows it.
pub fn layout_of(settings: &Settings, plugged: &[bool; STRIPS], rows: &TrimRows) -> Layout {
    let Some(info) = Registry::standard().get(TRIM_SHEET).map(|k| k.info()) else {
        return layout(OWN_HEIGHTS, &[1; STRIPS], &[CUT; STRIPS], plugged, rows);
    };
    let value = |key: &str| {
        let spec = info.setting(key)?;
        Some(
            settings
                .get(key)
                .map_or_else(|| spec.standard(), |v| spec.nearest(v)),
        )
    };
    let whole = |key: &str| match value(key) {
        Some(SettingValue::Whole(v)) => v,
        _ => 1,
    };
    let pick = |key: &str| match value(key) {
        Some(SettingValue::Choice(v)) => v,
        _ => 0,
    };
    let heights = HEIGHT_KEYS.map(whole);
    let fits = FIT_KEYS.map(pick);
    layout(pick("lay"), &heights, &fits, plugged, rows)
}

/// The Trim sheet box.
#[derive(Debug)]
pub(crate) struct TrimSheet {
    info: BoxInfo,
}

impl TrimSheet {
    pub(crate) fn new() -> Self {
        let mut inputs: Vec<PlugSpec> = STRIP_KEYS
            .iter()
            .zip(STRIP_NAMES)
            .map(|(key, name)| PlugSpec::new(key, name, ValueKind::Material, false))
            .collect();
        inputs.push(PlugSpec::new(
            "between",
            text!("Between strips"),
            ValueKind::Material,
            false,
        ));
        let outputs = vec![
            PlugSpec::new("material_out", text!("Sheet"), ValueKind::Material, false),
            PlugSpec::new("edges", text!("Edges"), ValueKind::Shape, false),
        ];
        let mut settings = vec![choice(
            "lay",
            text!("Strips on"),
            text!(
                "Where the strips lie. The project's trim rows: Strip 1 on the top row of the \
                 UV Editor's Trim menu, Strip 2 on the next, so strips of a model put on a row \
                 get its material. Heights of their own: the strips share the texture by their \
                 Height settings."
            ),
            &[text!("The project's trim rows"), text!("Heights of their own")],
            ON_ROWS,
        )];
        for i in 0..STRIPS {
            settings.push(whole(
                HEIGHT_KEYS[i],
                HEIGHT_NAMES[i],
                text!(
                    "How tall the strip is next to the others (with Heights of their own): 2 is \
                     twice as tall as 1. Heights adding up to 2, 4, 8 or 16 put every strip on \
                     whole pixels."
                ),
                1,
                16,
                1,
            ));
            settings.push(choice(
                FIT_KEYS[i],
                FIT_NAMES[i],
                text!(
                    "Cut to the strip: a band of the material at its real-world size, so every \
                     strip keeps the same scale (bricks, planks, panels). Shrink to fit: the \
                     whole material made small enough to fill the strip's height, repeating \
                     along it (a pipe's round shape, a row of bolts)."
                ),
                &[text!("Cut to the strip"), text!("Shrink to fit")],
                CUT,
            ));
        }
        settings.extend([
            length(
                "gap",
                text!("Gap"),
                text!("How wide the gap between the strips is, in centimetres (0: none)."),
                0.0,
                20.0,
                0.5,
            ),
            choice(
                "edges",
                text!("Edges"),
                text!("How each strip's top and bottom edges go down into the gap."),
                &[text!("Square"), text!("Chamfered"), text!("Rounded")],
                ROUNDED,
            ),
            length(
                "edge_width",
                text!("Edge width"),
                text!("How far in from each edge the bevel reaches, in centimetres."),
                0.0,
                20.0,
                1.5,
            ),
            number(
                "depth",
                text!("Depth"),
                text!(
                    "How deep the gaps and bevelled edges go into the height: 0 keeps every \
                     strip flat to its edge."
                ),
                0.0,
                1.0,
                0.3,
            ),
            colour(
                "gap_colour",
                text!("Gap colour"),
                text!("The gap's colour when nothing is plugged into Between strips."),
                [0.10, 0.09, 0.08],
            ),
        ]);
        Self {
            info: info(
                TRIM_SHEET,
                text!("Trim sheet"),
                Group::Material,
                text!(
                    "Stacks strips of different materials on one texture, each repeating left \
                     to right: a trim sheet, for walls, edges and pipes. Plug a whole material \
                     into each Strip you want, from the top down. The strips lie on the \
                     project's trim rows (the UV Editor's Trim menu), so strips of a model put \
                     on a row get that row's material; a project with no rows yet shares the \
                     height by the strips' own Heights. A thin gap and bevelled edges make each \
                     strip read as a separate piece; plug a material into Between strips to \
                     fill the gaps with it. Edges gives the gaps and bevels as a shape, to \
                     weather only there. On a model the sheet is laid once across the \
                     texture square, never repeated by its real-world size."
                ),
                (inputs, outputs),
                settings,
                &["dirt", "edge_wear", "material_out"],
            ),
        }
    }
}

impl BoxType for TrimSheet {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn uses_trim_rows(&self) -> bool {
        true
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let materials: [Option<&Arc<MaterialMaps>>; STRIPS] =
            STRIP_KEYS.map(|key| job.input(key).and_then(Value::material));
        let plugged = materials.map(|m| m.is_some());
        let heights = HEIGHT_KEYS.map(|key| job.whole(key));
        let fits = FIT_KEYS.map(|key| job.choice(key));
        let laid = layout(
            job.choice("lay"),
            &heights,
            &fits,
            &plugged,
            job.trim_rows(),
        );
        if laid.places.is_empty() {
            return Err(nothing_to_lay(job, &laid));
        }
        let n = job.size().get();
        let mix = Mix {
            gap: job.length_px("gap") / n as f32,
            edge: job.length_px("edge_width") / n as f32,
            edges: job.choice("edges"),
        };
        let rows = mix.rows(&laid.places, n);
        let sources: Vec<Source<'_>> = laid
            .places
            .iter()
            .filter_map(|p| Some(Source::new(job, materials[p.strip]?, p)))
            .collect();
        let gap = match job.input("between").and_then(Value::material) {
            Some(m) => Gap::Material(m),
            None => Gap::Plain(job.colour("gap_colour").map(|c| c.clamp(0.0, 1.0))),
        };
        let sheet = Sheet {
            rows: &rows,
            sources: &sources,
            gap: &gap,
            n,
        };
        let depth = job.number("depth").clamp(0.0, 1.0);
        let material = sheet.material(job, depth);
        let edges = job.fill_grey(|_, y| rows[y as usize].sink.clamp(0.0, 1.0));
        Ok(vec![
            Value::Material(Arc::new(material)),
            Value::Shape(Arc::new(edges)),
        ])
    }
}

/// Why a sheet has no strip to lay, in plain words.
fn nothing_to_lay(job: &Job<'_>, laid: &Layout) -> Refusal {
    match laid.past_rows.first() {
        Some(first) => Refusal::new(trf!(
            "Strip {} and the strips after it lie past the project's last trim row (it has \
             {}), so there is nothing to lay. Plug the materials into the first Strips, add \
             rows in the UV Editor's Trim menu, or set Strips on to Heights of their own.",
            first + 1,
            laid.rows
        )),
        None => Refusal::new(trf!(
            "{} lays one material on each strip: plug a whole material into Strip 1, and more \
             into Strip 2 and on. Between strips only fills the gaps.",
            job.box_name()
        )),
    }
}

/// How the strips' edges are laid at one size.
struct Mix {
    /// The gap between strips, as a share of the texture.
    gap: f32,
    /// How far the bevel reaches in from each edge, as a share.
    edge: f32,
    /// The Edges setting.
    edges: u32,
}

/// What one row of pixels holds.
struct RowMix {
    /// Each strip's share of the row (its index in the places), adding up
    /// to at most 1.
    parts: Vec<(usize, f32)>,
    /// The gap's share (1 less the strips').
    gap: f32,
    /// How far the row sinks: 0 on a strip's flat, 1 in the gap.
    sink: f32,
}

impl Mix {
    /// How far a strip sinks `d` (a share of the texture) in from its
    /// nearest edge.
    fn sink(&self, d: f32) -> f32 {
        if self.edge <= 0.0 || self.edges == SQUARE {
            return 0.0;
        }
        let t = (d / self.edge).clamp(0.0, 1.0);
        match self.edges {
            CHAMFERED => 1.0 - t,
            ROUNDED => 1.0 - (1.0 - (1.0 - t) * (1.0 - t)).max(0.0).sqrt(),
            _ => 0.0,
        }
    }

    /// Every row of pixels of a sheet `n` across with the strips `places`:
    /// how much of each strip and of the gap it holds (by how much of the
    /// row each covers, so edges between whole pixels are smooth), and
    /// how far it sinks.
    fn rows(&self, places: &[StripPlace], n: u32) -> Vec<RowMix> {
        let n_f = n as f32;
        let half = self.gap * 0.5;
        (0..n)
            .map(|y| {
                let (top, bottom) = (y as f32 / n_f, (y + 1) as f32 / n_f);
                let middle = (y as f32 + 0.5) / n_f;
                let mut parts = Vec::new();
                let (mut sink, mut covered) = (0.0, 0.0);
                for (i, p) in places.iter().enumerate() {
                    let (a, b) = (p.from + half, p.to - half);
                    let w = ((b.min(bottom) - a.max(top)) * n_f).max(0.0);
                    if w <= 0.0 {
                        continue;
                    }
                    let d = (middle - a).min(b - middle);
                    let s = if d < 0.0 { 1.0 } else { self.sink(d) };
                    parts.push((i, w));
                    covered += w;
                    sink += w * s;
                }
                // Strips never overlap; this only guards rounding.
                if covered > 1.0 {
                    for part in &mut parts {
                        part.1 /= covered;
                    }
                    sink /= covered;
                    covered = 1.0;
                }
                let gap = 1.0 - covered;
                RowMix {
                    parts,
                    gap,
                    sink: sink + gap,
                }
            })
            .collect()
    }
}

/// Where one strip's pixels come from.
enum Source<'a> {
    /// A band of the material at its own size: the strip's row `y` is the
    /// material's row `y - off`.
    Cut { m: &'a MaterialMaps, off: i64 },
    /// The whole material (averaged down so it shrinks without flicker) laid
    /// `across` times along the strip, its height filling the strip from
    /// `from`, `span` tall.
    Shrunk {
        m: Box<MaterialMaps>,
        across: f32,
        from: f32,
        span: f32,
    },
}

/// A grey map of a material, if it has one.
type PickGrey = fn(&MaterialMaps) -> Option<&Grey>;
/// A colour map of a material.
type PickRgb = fn(&MaterialMaps) -> &Rgb;

impl<'a> Source<'a> {
    fn new(job: &Job<'_>, m: &'a MaterialMaps, place: &StripPlace) -> Self {
        let n = job.size().get() as f32;
        if !place.shrink {
            return Source::Cut {
                m,
                off: (place.from * n).round() as i64,
            };
        }
        let span = (place.to - place.from).max(1.0 / n);
        let across = (1.0 / span).round().clamp(1.0, MOST_ACROSS);
        // Averaged down by the biggest power of two the shrink allows;
        // the rest (under two times) is blended when read.
        let factor = 1u32 << (across.log2().floor() as u32);
        Source::Shrunk {
            m: Box::new(averaged(m, factor)),
            across,
            from: place.from,
            span,
        }
    }

    fn grey(&self, pick: PickGrey, missing: f32, x: u32, y: u32, n: f32) -> f32 {
        match self {
            Source::Cut { m, off } => {
                pick(m).map_or(missing, |g| g.at(i64::from(x), i64::from(y) - off))
            }
            Source::Shrunk {
                m,
                across,
                from,
                span,
            } => {
                let (u, v) = shrunk_place(x, y, n, *across, *from, *span);
                pick(m).map_or(missing, |g| g.sample(u, v))
            }
        }
    }

    fn rgb(&self, pick: PickRgb, x: u32, y: u32, n: f32) -> [f32; 3] {
        match self {
            Source::Cut { m, off } => pick(m).at(i64::from(x), i64::from(y) - off),
            Source::Shrunk {
                m,
                across,
                from,
                span,
            } => {
                let (u, v) = shrunk_place(x, y, n, *across, *from, *span);
                pick(m).sample_rgb(u, v)
            }
        }
    }

    fn material(&self) -> &MaterialMaps {
        match self {
            Source::Cut { m, .. } => m,
            Source::Shrunk { m, .. } => m,
        }
    }
}

/// Where pixel `(x, y)` of a sheet `n` across reads a shrunk material.
fn shrunk_place(x: u32, y: u32, n: f32, across: f32, from: f32, span: f32) -> (f32, f32) {
    let u = (x as f32 + 0.5) / n * across;
    let v = ((y as f32 + 0.5) / n - from) / span;
    (u, v)
}

/// `m` made `f` times smaller (a power of two), each pixel the average of
/// the `f` × `f` it covers, so a material shrunk into a strip never
/// flickers; never smaller than [`Size::MIN`].
fn averaged(m: &MaterialMaps, f: u32) -> MaterialMaps {
    let n = m.size();
    let f = f.min(n / Size::MIN).max(1);
    let Ok(small) = Size::new(n / f) else {
        return m.clone();
    };
    if f == 1 {
        return m.clone();
    }
    let side = small.get();
    let area = (f * f) as f32;
    let grey = |g: &Grey| -> Arc<Grey> {
        let pixels = (0..side * side)
            .map(|i| {
                let (x, y) = (i % side * f, i / side * f);
                let mut sum = 0.0;
                for dy in 0..f {
                    for dx in 0..f {
                        sum += g.at(i64::from(x + dx), i64::from(y + dy));
                    }
                }
                sum / area
            })
            .collect();
        Arc::new(Image::from_pixels(small, pixels).unwrap_or_else(|| g.clone()))
    };
    let rgb = |c: &Rgb| -> Arc<Rgb> {
        let pixels = (0..side * side)
            .map(|i| {
                let (x, y) = (i % side * f, i / side * f);
                let mut sum = [0.0; 3];
                for dy in 0..f {
                    for dx in 0..f {
                        let p = c.at(i64::from(x + dx), i64::from(y + dy));
                        for (s, p) in sum.iter_mut().zip(p) {
                            *s += p;
                        }
                    }
                }
                sum.map(|s| s / area)
            })
            .collect();
        Arc::new(Image::from_pixels(small, pixels).unwrap_or_else(|| c.clone()))
    };
    MaterialMaps {
        colour: rgb(&m.colour),
        height: grey(&m.height),
        roughness: grey(&m.roughness),
        metal: grey(&m.metal),
        normal: rgb(&m.normal),
        occlusion: grey(&m.occlusion),
        curvature: grey(&m.curvature),
        opacity: m.opacity.as_deref().map(grey),
        translucency: m.translucency.as_deref().map(grey),
    }
}

/// What fills the gaps between the strips.
enum Gap<'a> {
    /// The material plugged into Between strips.
    Material(&'a MaterialMaps),
    /// A plain colour, half high, matt.
    Plain([f32; 3]),
}

impl Gap<'_> {
    fn grey(&self, pick: PickGrey, plain: f32, missing: f32, x: u32, y: u32) -> f32 {
        match self {
            Gap::Material(m) => pick(m).map_or(missing, |g| g.at(i64::from(x), i64::from(y))),
            Gap::Plain(_) => plain,
        }
    }

    fn rgb(&self, pick: PickRgb, plain: [f32; 3], x: u32, y: u32) -> [f32; 3] {
        match self {
            Gap::Material(m) => pick(m).at(i64::from(x), i64::from(y)),
            Gap::Plain(_) => plain,
        }
    }
}

/// A sheet being made: its rows, its strips' sources and its gap.
struct Sheet<'s, 'a> {
    rows: &'s [RowMix],
    sources: &'s [Source<'a>],
    gap: &'s Gap<'a>,
    n: u32,
}

impl Sheet<'_, '_> {
    /// One grey map of the sheet: each strip's by its share of the row,
    /// the gap's (`plain` with nothing in Between strips) by its share;
    /// `missing` where a material lacks the map.
    fn grey(&self, job: &Job<'_>, pick: PickGrey, plain: f32, missing: f32) -> Grey {
        let n = self.n as f32;
        job.fill_grey(|x, y| {
            let row = &self.rows[y as usize];
            let strips: f32 = row
                .parts
                .iter()
                .map(|(i, w)| w * self.sources[*i].grey(pick, missing, x, y, n))
                .sum();
            strips + row.gap * self.gap.grey(pick, plain, missing, x, y)
        })
    }

    /// One colour map of the sheet, as [`Sheet::grey`].
    fn rgb(&self, job: &Job<'_>, pick: PickRgb, plain: [f32; 3]) -> Rgb {
        let n = self.n as f32;
        job.fill_rgb(|x, y| {
            let row = &self.rows[y as usize];
            let mut out = [0.0; 3];
            let mut add = |c: [f32; 3], w: f32| {
                for (o, c) in out.iter_mut().zip(c) {
                    *o += c * w;
                }
            };
            for (i, w) in &row.parts {
                add(self.sources[*i].rgb(pick, x, y, n), *w);
            }
            add(self.gap.rgb(pick, plain, x, y), row.gap);
            out
        })
    }

    /// Whether any strip or the gap has map `pick`.
    fn any(&self, pick: PickGrey) -> bool {
        self.sources.iter().any(|s| pick(s.material()).is_some())
            || matches!(self.gap, Gap::Material(m) if pick(m).is_some())
    }

    /// The whole sheet: every map mixed by row, the height sunk `depth`
    /// at the gaps and bevels, and the normals, occlusion and curvature
    /// following the sink.
    fn material(&self, job: &Job<'_>, depth: f32) -> MaterialMaps {
        let plain = match self.gap {
            Gap::Plain(c) => *c,
            Gap::Material(_) => [0.0; 3],
        };
        let raise = job.fill_grey(|_, y| -self.rows[y as usize].sink * depth);
        let colour = self.rgb(job, |m| &m.colour, plain).map(clamp_rgb);
        let height = self.grey(job, |m| Some(&m.height), GAP_HEIGHT, 0.5);
        let height = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            (height.at(i, j) + raise.at(i, j)).clamp(0.0, 1.0)
        });
        let roughness = self
            .grey(job, |m| Some(&m.roughness), GAP_ROUGHNESS, 0.5)
            .map(clamp);
        let metal = self.grey(job, |m| Some(&m.metal), 0.0, 0.0).map(clamp);
        let mixed = self.rgb(job, |m| &m.normal, [0.0, 0.0, 1.0]);
        let bent = normal(job, &raise, BEND);
        let normals = job.fill_rgb(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            let [bx, by, bz] = mixed.at(i, j);
            let [sx, sy, sz] = bent.at(i, j);
            glam::Vec3::new(bx + sx, by + sy, bz.max(0.0) * sz)
                .normalize_or(glam::Vec3::Z)
                .to_array()
        });
        let open = occlusion(job, &raise, 1.0);
        let occl = self.grey(job, |m| Some(&m.occlusion), 1.0, 1.0);
        let occl = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            (occl.at(i, j) * open.at(i, j)).clamp(0.0, 1.0)
        });
        let bends = curvature(job, &raise);
        let curv = self.grey(job, |m| Some(&m.curvature), 0.5, 0.5);
        let curv = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            (curv.at(i, j) + bends.at(i, j) - 0.5).clamp(0.0, 1.0)
        });
        let opacity = self.any(|m| m.opacity.as_deref()).then(|| {
            Arc::new(
                self.grey(job, |m| m.opacity.as_deref(), 1.0, 1.0)
                    .map(clamp),
            )
        });
        let translucency = self.any(|m| m.translucency.as_deref()).then(|| {
            Arc::new(
                self.grey(job, |m| m.translucency.as_deref(), 0.0, 0.0)
                    .map(clamp),
            )
        });
        MaterialMaps {
            colour: Arc::new(colour),
            height: Arc::new(height),
            roughness: Arc::new(roughness),
            metal: Arc::new(metal),
            normal: Arc::new(normals),
            occlusion: Arc::new(occl),
            curvature: Arc::new(curv),
            opacity,
            translucency,
        }
    }
}

fn clamp(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

fn clamp_rgb(c: [f32; 3]) -> [f32; 3] {
    c.map(clamp)
}

/// The plain words for where a laid-out strip lies, for the interface:
/// "Strip 2: 25% to 50% down (on the row Bolts)".
pub fn describe(place: &StripPlace) -> String {
    let (from, to) = (place.from * 100.0, place.to * 100.0);
    let fit = if place.shrink {
        tr("shrunk to fit")
    } else {
        tr("cut to the strip")
    };
    match &place.row {
        Some(row) => trf!(
            "Strip {}: {:.1}% to {:.1}% down, on the row {}, {}",
            place.strip + 1,
            from,
            to,
            row,
            fit
        ),
        None => trf!(
            "Strip {}: {:.1}% to {:.1}% down, {}",
            place.strip + 1,
            from,
            to,
            fit
        ),
    }
}

#[cfg(test)]
mod tests;
