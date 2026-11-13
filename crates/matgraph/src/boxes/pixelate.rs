//! Pixelate (Phase 12, milestone 24, [decision 0287](../../../../docs/decisions/0287-pixel-art-output.md)):
//! a texture made of a few big square pixels, for pixel-art games.
//!
//! The texture is cut into a grid of Pixels × Pixels squares, the same
//! number every way, and each square takes one value: the average of the
//! picture under it, or the picture at its middle. A whole material is
//! cut on the same grid, map by map, so its colour, height, roughness and
//! normals line up square for square.
//!
//! Then, on the grid itself (so every step works on whole squares):
//!
//! 1. **Limit to a palette:** each square's colour becomes the closest of
//!    a project palette's swatches (Palette map's Nearest, decision
//!    0281), with its 4 × 4 dither pattern laid on the squares when asked.
//! 2. **Tidy stray pixels:** a square unlike all eight around it (a lone
//!    speck) takes the colour, and every other map, of the square most
//!    common round it. Dithering's checks are kept: their corners match.
//! 3. **Outline:** a one-square dark line round shapes (from the Outline
//!    from plug, else the material's opacity, else its height), on the
//!    shape's own edge or just outside it. Outside a see-through shape it
//!    is made solid, as sprite outlines are.
//!
//! The grid always starts at the texture's corner and holds a whole
//! number of squares, and its neighbours wrap round, so the result tiles
//! exactly, square for square ([`cell_of`]). Pixel counts are multiples
//! of four, so the dither pattern repeats evenly too.
//!
//! Put it last, just before Material out: the export then writes the
//! textures at exactly Pixels × Pixels ([`pixel_grid`]), one texel a
//! square, set to be shown crisp (no smoothing) in the engine.

use super::palette_map::{bayer, nearest, oklab};
use super::{choice, colour, info, number, switch};
use crate::graph::{Graph, PlugRef};
use crate::image::{Grey, Image, Rgb};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec, Registry};
use crate::settings::{SettingKind, SettingSpec, SettingValue};
use crate::value::{MaterialMaps, Value};
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Pixelate box's kind id.
pub const PIXELATE: &str = "pixelate";

/// The pixel counts offered, squares across (and down), in the Pixels
/// setting's order: all multiples of four, so the dither tiles.
pub const PIXEL_COUNTS: [u32; 9] = [16, 24, 32, 48, 64, 96, 128, 192, 256];

/// The Pixels setting's standard: 32 across.
const STANDARD_PIXELS: u32 = 2;

/// The Sample setting's choices.
const AVERAGE: u32 = 0;

/// The Outline setting's choices.
const OUTLINE_OFF: u32 = 0;
const OUTLINE_INSIDE: u32 = 1;

/// How far apart (in Oklab, squared) two colours may be and still count
/// as the same for tidying: about one step of an 8-bit channel.
const SAME: f32 = 0.0004;

/// How much the dither pushes a colour before it is snapped (Palette
/// map's Nearest uses the same).
const COLOUR_DITHER: f32 = 0.25;

/// A palette's swatches (sRGB, 0 to 1) and the same in Oklab.
type Swatches = (Vec<[f32; 3]>, Vec<[f32; 3]>);

/// The Pixelate box.
#[derive(Debug)]
pub(crate) struct Pixelate {
    info: BoxInfo,
}

impl Pixelate {
    pub(crate) fn new() -> Self {
        let (mut inputs, outputs) = super::moving::plugs();
        inputs.push(PlugSpec::new(
            "shape",
            text!("Outline from"),
            ValueKind::Shape,
            false,
        ));
        Self {
            info: info(
                PIXELATE,
                text!("Pixelate"),
                Group::Material,
                text!(
                    "Turns a texture into a few big square pixels, for pixel-art games. Pixels \
                     is how many squares across (and down) the whole texture has. A whole \
                     material is cut on the same squares, so its colour, height and roughness \
                     line up. Limit to a palette keeps every colour to one of the project's \
                     palettes, Tidy stray pixels removes lone specks, and Outline draws a dark \
                     line one pixel wide round shapes. Put it last, just before Material out: \
                     Export textures… then writes the textures at exactly that many pixels, \
                     set to stay crisp in the game."
                ),
                (inputs, outputs),
                vec![
                    SettingSpec {
                        key: Cow::Borrowed("pixels"),
                        name: Label::Key(text!("Pixels")),
                        about: Label::Key(text!(
                            "How many big pixels across (and down) one repeat of the texture \
                             has. Fewer gives a chunkier, more retro look."
                        )),
                        kind: SettingKind::Choice {
                            options: vec![
                                Label::Key(text!("16 × 16")),
                                Label::Key(text!("24 × 24")),
                                Label::Key(text!("32 × 32")),
                                Label::Key(text!("48 × 48")),
                                Label::Key(text!("64 × 64")),
                                Label::Key(text!("96 × 96")),
                                Label::Key(text!("128 × 128")),
                                Label::Key(text!("192 × 192")),
                                Label::Key(text!("256 × 256")),
                            ],
                            standard: STANDARD_PIXELS,
                        },
                    },
                    choice(
                        "sample",
                        text!("Each pixel takes"),
                        text!(
                            "The average of the picture under it (smoother, keeps thin lines \
                             as a blend), or the picture at its middle (crisper, but thin \
                             lines may vanish or double)."
                        ),
                        &[text!("The average"), text!("The middle")],
                        AVERAGE,
                    ),
                    switch(
                        "limit",
                        text!("Limit to a palette"),
                        text!(
                            "Turns every pixel into the closest colour of one of the project's \
                             palettes, as pixel-art games keep to a few colours."
                        ),
                    ),
                    SettingSpec {
                        key: Cow::Borrowed("palette"),
                        name: Label::Key(text!("Palette")),
                        about: Label::Key(text!(
                            "Which of the project's palettes, with Limit to a palette on."
                        )),
                        kind: SettingKind::Palette {
                            standard: String::new(),
                        },
                    },
                    number(
                        "dither",
                        text!("Dither"),
                        text!(
                            "With Limit to a palette, mixes neighbouring colours in a checked \
                             pattern of big pixels where one meets the next, the classic \
                             pixel-art way to show a blend with few colours."
                        ),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    switch(
                        "tidy",
                        text!("Tidy stray pixels"),
                        text!(
                            "A lone pixel unlike every pixel round it (a speck) takes the \
                             colour of its neighbours, as a pixel artist would clean it up."
                        ),
                    ),
                    choice(
                        "outline",
                        text!("Outline"),
                        text!(
                            "A dark line one pixel wide round shapes: on the shape's own edge, \
                             or just outside it (as sprites are outlined). The shapes come from \
                             Outline from, else the material's opacity, else its height."
                        ),
                        &[
                            text!("None"),
                            text!("On the edge"),
                            text!("Outside the edge"),
                        ],
                        OUTLINE_OFF,
                    ),
                    colour(
                        "outline_colour",
                        text!("Outline colour"),
                        text!(
                            "The outline's colour; with Limit to a palette, the palette's \
                             closest."
                        ),
                        [0.13, 0.10, 0.12],
                    ),
                    number(
                        "level",
                        text!("Outline level"),
                        text!(
                            "Where a shape's edge is: greys above it are inside the shape. \
                             Lower it to outline more of a height, raise it for only the tops."
                        ),
                        0.0,
                        1.0,
                        0.5,
                    ),
                ],
                &["material_out", "opacity"],
            ),
        }
    }
}

/// The square of the grid `n` across that pixel `x` of a picture `size`
/// across falls in. Squares start at the picture's corner, so a picture
/// of any size cut on the same grid lines up.
pub fn cell_of(x: u32, size: u32, n: u32) -> u32 {
    let size = size.max(1);
    ((u64::from(x) * u64::from(n) / u64::from(size)) as u32).min(n.saturating_sub(1))
}

/// The first pixel of square `i` of the grid `n` across a picture
/// `size` across, and the pixel after its last.
pub fn cell_span(i: u32, size: u32, n: u32) -> (u32, u32) {
    let n = u64::from(n.max(1));
    let start = |i: u64| (i * u64::from(size)).div_ceil(n) as u32;
    (start(u64::from(i)), start(u64::from(i) + 1).min(size))
}

/// The pixel in the middle of square `i` (the first of the middle two),
/// or the nearest pixel when the square holds none (a grid finer than
/// the picture).
pub fn cell_middle(i: u32, size: u32, n: u32) -> u32 {
    let (a, b) = cell_span(i, size, n);
    if b > a {
        a + (b - a - 1) / 2
    } else {
        a.min(size.saturating_sub(1))
    }
}

/// The squares across `graph`'s textures when they are pixel art: a
/// Pixelate box (not bypassed) plugged straight into Material out's
/// material. The export then writes its textures at this size, crisp.
pub fn pixel_grid(registry: &Registry, graph: &Graph) -> Option<u32> {
    let out = graph.first_of(crate::eval::MATERIAL_OUT)?;
    let wire = graph.wire_into(&PlugRef::new(out, "material"))?;
    let node = graph.get(wire.from.node)?;
    if node.kind != PIXELATE || node.bypassed || wire.from.plug != "material_out" {
        return None;
    }
    let spec = registry.get(PIXELATE)?.info().setting("pixels")?.clone();
    let chosen = node
        .settings
        .get("pixels")
        .map_or_else(|| spec.standard(), |v| spec.nearest(v));
    match chosen {
        SettingValue::Choice(i) => PIXEL_COUNTS.get(i as usize).copied(),
        _ => None,
    }
}

/// A grid of squares, `n` across, each holding one value.
struct Grid<T> {
    n: u32,
    cells: Vec<T>,
}

impl<T: Copy> Grid<T> {
    /// Square `(x, y)`, wrapping round.
    fn at(&self, x: i64, y: i64) -> T {
        let n = i64::from(self.n);
        self.cells[(y.rem_euclid(n) * n + x.rem_euclid(n)) as usize]
    }
}

/// How a picture's squares are given their value.
#[derive(Clone, Copy)]
struct Cutting {
    /// Squares across.
    n: u32,
    /// The average of each square, else its middle.
    average: bool,
}

impl Cutting {
    /// `image` cut into squares: each the average of what is under it
    /// (with `sum` and `scale`), or its middle pixel.
    fn cut<P: Copy + Default>(
        self,
        image: &Image<P>,
        sum: impl Fn(P, P) -> P,
        scale: impl Fn(P, f32) -> P,
    ) -> Grid<P> {
        let size = image.size();
        let n = self.n;
        let mut cells = Vec::with_capacity((n * n) as usize);
        for cy in 0..n {
            for cx in 0..n {
                let value = if self.average {
                    let (x0, x1) = cell_span(cx, size, n);
                    let (y0, y1) = cell_span(cy, size, n);
                    let (x1, y1) = (x1.max(x0 + 1), y1.max(y0 + 1));
                    let mut total = P::default();
                    for y in y0..y1 {
                        for x in x0..x1 {
                            total = sum(total, image.at(i64::from(x), i64::from(y)));
                        }
                    }
                    scale(total, 1.0 / ((x1 - x0) * (y1 - y0)) as f32)
                } else {
                    image.at(
                        i64::from(cell_middle(cx, size, n)),
                        i64::from(cell_middle(cy, size, n)),
                    )
                };
                cells.push(value);
            }
        }
        Grid { n, cells }
    }

    fn grey(self, image: &Grey) -> Grid<f32> {
        self.cut(image, |a, b| a + b, |a, k| a * k)
    }

    fn rgb(self, image: &Rgb) -> Grid<[f32; 3]> {
        self.cut(
            image,
            |a, b| [a[0] + b[0], a[1] + b[1], a[2] + b[2]],
            |a, k| a.map(|v| v * k),
        )
    }
}

/// The Oklab distance (squared) between two colours.
fn apart(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (a, b) = (oklab(a), oklab(b));
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

/// For each square, the square it takes its values from: itself, or for
/// a lone speck (unlike all eight squares round it), the neighbour (of
/// the four beside it) most squares round it match, the closest in
/// colour on a tie.
fn tidied(colours: &Grid<[f32; 3]>) -> Vec<usize> {
    let n = i64::from(colours.n);
    let index = |x: i64, y: i64| (y.rem_euclid(n) * n + x.rem_euclid(n)) as usize;
    let around: [(i64, i64); 8] = [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ];
    let mut out = Vec::with_capacity(colours.cells.len());
    for y in 0..n {
        for x in 0..n {
            let me = colours.at(x, y);
            if around
                .iter()
                .any(|(dx, dy)| apart(me, colours.at(x + dx, y + dy)) < SAME)
            {
                out.push(index(x, y));
                continue;
            }
            let best = around[..4]
                .iter()
                .map(|(dx, dy)| {
                    let c = colours.at(x + dx, y + dy);
                    let count = around
                        .iter()
                        .filter(|(ex, ey)| apart(c, colours.at(x + ex, y + ey)) < SAME)
                        .count();
                    (index(x + dx, y + dy), count, apart(me, c))
                })
                .max_by(|a, b| a.1.cmp(&b.1).then(b.2.total_cmp(&a.2)));
            out.push(best.map_or(index(x, y), |b| b.0));
        }
    }
    out
}

/// Which squares the outline covers: on the edge, squares inside the
/// shape beside one outside it; outside, squares outside beside one
/// inside (the four beside each, wrapping).
fn outlined(inside: &Grid<bool>, on_edge: bool) -> Vec<bool> {
    let n = i64::from(inside.n);
    let mut out = Vec::with_capacity(inside.cells.len());
    for y in 0..n {
        for x in 0..n {
            let me = inside.at(x, y);
            let beside = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|(dx, dy)| inside.at(x + dx, y + dy) != me);
            out.push(beside && me == on_edge);
        }
    }
    out
}

impl BoxType for Pixelate {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(super::moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let size = job.size().get();
        let chosen = job.choice("pixels") as usize;
        let wanted = PIXEL_COUNTS[chosen.min(PIXEL_COUNTS.len() - 1)];
        // A grid finer than the picture is the picture's own pixels.
        let cutting = Cutting {
            n: wanted.min(size),
            average: job.choice("sample") == AVERAGE,
        };
        let n = cutting.n;
        let height = job.grey("in");
        let colour = job.input("colour").and_then(Value::colour);
        let material = job.input("material").and_then(Value::material);
        if height.is_none() && colour.is_none() && material.is_none() {
            // The evaluator says so first, with ways to fix it.
            return Err(Refusal::new(words::trf!(
                "{} has nothing plugged in.",
                job.box_name()
            )));
        }

        // The palette, when asked: its swatches and their Oklab colours.
        let palette: Option<Swatches> = if job.switch("limit") {
            let name = job.palette("palette");
            let p = job.palettes().usable(&job.box_name(), "palette", &name)?;
            let rgb: Vec<[f32; 3]> = p
                .colours
                .iter()
                .map(|c| c.map(|v| f32::from(v) / 255.0))
                .collect();
            let labs = rgb.iter().map(|c| oklab(*c)).collect();
            (!rgb.is_empty()).then_some((rgb, labs))
        } else {
            None
        };
        let dither = job.number("dither").clamp(0.0, 1.0);
        let snapped = |c: [f32; 3], x: u32, y: u32| -> [f32; 3] {
            match &palette {
                Some((rgb, labs)) => {
                    let push = bayer(x, y) * dither * COLOUR_DITHER;
                    let c = c.map(|v| (v + push).clamp(0.0, 1.0));
                    rgb[nearest(labs, oklab(c))]
                }
                None => c.map(|v| v.clamp(0.0, 1.0)),
            }
        };
        let limited = |grid: Grid<[f32; 3]>| -> Grid<[f32; 3]> {
            let cells = grid
                .cells
                .iter()
                .enumerate()
                .map(|(i, c)| snapped(*c, i as u32 % grid.n, i as u32 / grid.n))
                .collect();
            Grid { n: grid.n, cells }
        };

        // The outline's squares, from the shape plugged in, else the
        // material's opacity, else a height.
        let outline = job.choice("outline");
        let lines: Option<Vec<bool>> = if outline == OUTLINE_OFF {
            None
        } else {
            let source = job
                .grey("shape")
                .or_else(|| material.and_then(|m| m.opacity.as_ref()))
                .or(height)
                .or_else(|| material.map(|m| &m.height));
            let Some(source) = source else {
                return Err(Refusal::new(words::trf!(
                    "{}'s outline needs to know where shapes are, and a colour has no inside \
                     or outside: plug a shape into Outline from, or plug in a height or a \
                     material, or set Outline to None.",
                    job.box_name()
                )));
            };
            let level = job.number("level");
            let greys = cutting.grey(source);
            let inside = Grid {
                n,
                cells: greys.cells.iter().map(|v| *v > level).collect(),
            };
            Some(outlined(&inside, outline == OUTLINE_INSIDE))
        };
        let line_colour = match &palette {
            Some((rgb, labs)) => rgb[nearest(labs, oklab(job.colour("outline_colour")))],
            None => job.colour("outline_colour"),
        };

        // Where each square takes its values from, once tidied (by the
        // colour that will show: the material's, else the colour's).
        let shown = material.map(|m| &m.colour).or(colour);
        let shown_grid = shown.map(|c| limited(cutting.rgb(c)));
        let from: Option<Vec<usize>> = if job.switch("tidy") {
            shown_grid.as_ref().map(tidied)
        } else {
            None
        };
        let source_of = |i: usize| from.as_ref().map_or(i, |f| f[i]);
        let is_line = |i: usize| lines.as_ref().is_some_and(|l| l[i]);
        let cell = |x: u32, y: u32| (cell_of(y, size, n) * n + cell_of(x, size, n)) as usize;

        let grey_out =
            |grid: &Grid<f32>| Arc::new(job.fill_grey(|x, y| grid.cells[source_of(cell(x, y))]));
        let colour_out = |grid: &Grid<[f32; 3]>| {
            Arc::new(job.fill_rgb(|x, y| {
                let i = cell(x, y);
                if is_line(i) {
                    line_colour
                } else {
                    grid.cells[source_of(i)]
                }
            }))
        };

        let height_out = height.map(|g| Value::Height(grey_out(&cutting.grey(g))));
        let colour_value = colour.map(|c| {
            let grid = match (material, &shown_grid) {
                (None, Some(g)) => Grid {
                    n: g.n,
                    cells: g.cells.clone(),
                },
                _ => limited(cutting.rgb(c)),
            };
            Value::Colour(colour_out(&grid))
        });
        let material_out = material.map(|m| {
            let colours = match &shown_grid {
                Some(g) => Grid {
                    n: g.n,
                    cells: g.cells.clone(),
                },
                None => limited(cutting.rgb(&m.colour)),
            };
            let normals = {
                let mut grid = cutting.rgb(&m.normal);
                for v in &mut grid.cells {
                    *v = glam::Vec3::from_array(*v)
                        .normalize_or(glam::Vec3::Z)
                        .to_array();
                }
                grid
            };
            // The outline outside a see-through shape is made solid.
            let opacity = m.opacity.as_ref().map(|g| {
                let grid = cutting.grey(g);
                Arc::new(job.fill_grey(|x, y| {
                    let i = cell(x, y);
                    if is_line(i) {
                        1.0
                    } else {
                        grid.cells[source_of(i)]
                    }
                }))
            });
            Value::Material(Arc::new(MaterialMaps {
                colour: colour_out(&colours),
                height: grey_out(&cutting.grey(&m.height)),
                roughness: grey_out(&cutting.grey(&m.roughness)),
                metal: grey_out(&cutting.grey(&m.metal)),
                normal: Arc::new(job.fill_rgb(|x, y| normals.cells[source_of(cell(x, y))])),
                occlusion: grey_out(&cutting.grey(&m.occlusion)),
                curvature: grey_out(&cutting.grey(&m.curvature)),
                opacity,
                translucency: m.translucency.as_ref().map(|g| grey_out(&cutting.grey(g))),
            }))
        });
        Ok(vec![height_out, colour_value, material_out])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_start_at_the_corner_and_cover_every_pixel_once() {
        for size in [16, 32, 256, 2048] {
            for n in PIXEL_COUNTS {
                let mut seen = vec![0u32; n as usize];
                for x in 0..size {
                    let c = cell_of(x, size, n);
                    assert!(c < n);
                    seen[c as usize] += 1;
                    let (a, b) = cell_span(c, size, n);
                    assert!((a..b).contains(&x), "{size} {n} {x}");
                }
                if size >= n {
                    // Every square holds pixels, the counts differing by
                    // one at most, and its middle is its own.
                    let lo = seen.iter().min().unwrap();
                    let hi = seen.iter().max().unwrap();
                    assert!(*lo >= 1 && hi - lo <= 1, "{size} {n}");
                    for i in 0..n {
                        assert_eq!(cell_of(cell_middle(i, size, n), size, n), i);
                    }
                }
            }
        }
        assert_eq!(cell_of(0, 512, 32), 0);
        assert_eq!(cell_of(15, 512, 32), 0);
        assert_eq!(cell_of(16, 512, 32), 1);
        assert_eq!(cell_of(511, 512, 32), 31);
        assert_eq!(cell_middle(0, 512, 32), 7);
        // Nothing breaks on nothing.
        assert_eq!(cell_of(5, 0, 0), 0);
    }

    #[test]
    fn a_lone_speck_is_tidied_but_checks_are_kept() {
        let n = 8u32;
        let a = [0.2, 0.3, 0.1];
        let b = [0.9, 0.8, 0.7];
        let mut cells = vec![a; 64];
        cells[3 * 8 + 3] = b;
        let from = tidied(&Grid { n, cells });
        assert_ne!(from[3 * 8 + 3], 3 * 8 + 3);
        assert!(from.iter().enumerate().filter(|(i, f)| *i != **f).count() == 1);
        // A checked dither: every square has matching corners.
        let checks: Vec<[f32; 3]> = (0..64)
            .map(|i| if (i % 8 + i / 8) % 2 == 0 { a } else { b })
            .collect();
        let from = tidied(&Grid { n, cells: checks });
        assert!(from.iter().enumerate().all(|(i, f)| i == *f));
    }

    #[test]
    fn the_outline_is_on_the_edge_or_just_outside() {
        let n = 8u32;
        let cells: Vec<bool> = (0..64)
            .map(|i| (2..6).contains(&(i % 8)) && (2..6).contains(&(i / 8)))
            .collect();
        let inside = Grid { n, cells };
        let on = outlined(&inside, true);
        let out = outlined(&inside, false);
        // On the edge: the ring of 12 inside; outside: the 16 beside it
        // (the corners touch only diagonally).
        assert_eq!(on.iter().filter(|v| **v).count(), 12);
        assert_eq!(out.iter().filter(|v| **v).count(), 16);
        assert!(on[2 * 8 + 2] && !on[3 * 8 + 3]);
        assert!(out[8 + 2] && !out[8 + 1]);
    }
}
