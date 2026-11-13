//! Palette map (Phase 12, milestone 19, [decision 0281](../../../../docs/decisions/0281-palette-map.md)):
//! colours greys, or snaps colours, with one of the project's palettes,
//! so every material keeps to the game's colours.
//!
//! - **Steps:** the greys are cut into as many even bands as the palette
//!   has swatches (between First and Last), each band one swatch, in the
//!   chosen order.
//! - **Smooth:** the swatches are spread evenly from black to white and
//!   blended between, like a gradient made of the palette.
//! - **Nearest:** each colour becomes the palette's swatch closest to it
//!   by eye (in the Oklab colour space), which is how pixel art limits
//!   its colours.
//!
//! Dither mixes neighbouring swatches in a fixed 4 × 4 pattern where one
//! band meets the next (Steps and Nearest), as old games did; the pattern
//! repeats every four pixels, so the texture still tiles.

use super::{choice, info, number, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::settings::{SettingKind, SettingSpec};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The highest swatch number First and Last can name (a palette holds
/// at most this many colours in the painter).
pub(crate) const MAX_SWATCH: i32 = 1024;

/// The ordered dither pattern: a 4 × 4 Bayer matrix.
const BAYER: [f32; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];

/// How far Dither at 1 pushes a colour before Nearest picks a swatch.
const COLOUR_DITHER: f32 = 0.25;

/// The mode setting's choices.
const STEPS: u32 = 0;
const SMOOTH: u32 = 1;
const NEAREST: u32 = 2;

/// The order setting's choices.
const AS_LISTED: u32 = 0;
const REVERSED: u32 = 1;
const DARK_TO_LIGHT: u32 = 2;
const LIGHT_TO_DARK: u32 = 3;

/// The Palette map box.
#[derive(Debug)]
pub(crate) struct PaletteMap {
    info: BoxInfo,
}

impl PaletteMap {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "palette_map",
                text!("Palette map"),
                Group::Colour,
                text!(
                    "Colours a height with one of the project's palettes (the painter's, in \
                     the Paint mode's colour picker), so the material keeps to the game's \
                     colours. Steps gives each band of greys one swatch; Smooth blends between \
                     them; Nearest snaps a colour to the closest swatch, as pixel art does. \
                     Change the palette in the painter and every Palette map using it follows."
                ),
                (
                    vec![
                        PlugSpec::new("in", text!("In"), ValueKind::Height, false),
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                    ],
                    vec![PlugSpec::new(
                        "out",
                        text!("Colour"),
                        ValueKind::Colour,
                        false,
                    )],
                ),
                vec![
                    SettingSpec {
                        key: Cow::Borrowed("palette"),
                        name: Label::Key(text!("Palette")),
                        about: Label::Key(text!(
                            "Which of the project's palettes to colour with. First palette \
                             follows whichever is first in the colour picker's list."
                        )),
                        kind: SettingKind::Palette {
                            standard: String::new(),
                        },
                    },
                    choice(
                        "mode",
                        text!("Mode"),
                        text!(
                            "Steps: each band of greys one swatch. Smooth: blended between \
                             swatches. Nearest: each colour snapped to the closest swatch (plug \
                             a colour in)."
                        ),
                        &[text!("Steps"), text!("Smooth"), text!("Nearest")],
                        STEPS,
                    ),
                    choice(
                        "order",
                        text!("Order"),
                        text!(
                            "Which swatch dark greys take: the palette's first (As listed), its \
                             last (Reversed), or the darkest or lightest swatch, whatever the \
                             palette's order."
                        ),
                        &[
                            text!("As listed"),
                            text!("Reversed"),
                            text!("Dark to light"),
                            text!("Light to dark"),
                        ],
                        DARK_TO_LIGHT,
                    ),
                    whole(
                        "from",
                        text!("First swatch"),
                        text!(
                            "The first of the palette's swatches used, counting from 1 as the \
                             colour picker shows them."
                        ),
                        1,
                        MAX_SWATCH,
                        1,
                    ),
                    whole(
                        "to",
                        text!("Last swatch"),
                        text!(
                            "The last of the palette's swatches used; past the palette's end \
                             means its last."
                        ),
                        1,
                        MAX_SWATCH,
                        MAX_SWATCH,
                    ),
                    number(
                        "dither",
                        text!("Dither"),
                        text!(
                            "Mixes neighbouring swatches in a fine checked pattern where one \
                             meets the next, for a retro or pixel-art look (Steps and Nearest)."
                        ),
                        0.0,
                        1.0,
                        0.0,
                    ),
                ],
                &["material_from_height", "hue_saturation", "mix_materials"],
            ),
        }
    }
}

/// How light an sRGB colour looks, 0 to 1 (the painter's weights).
fn lightness(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// The swatches used, in the order the greys take them: `from` to `to`
/// (1-based, kept within the palette; the other way round when `from`
/// is past `to`), then put in `order`.
pub(crate) fn swatches(colours: &[[u8; 3]], from: i32, to: i32, order: u32) -> Vec<[f32; 3]> {
    if colours.is_empty() {
        return Vec::new();
    }
    let last = colours.len() - 1;
    let at = |n: i32| (n.max(1) as usize - 1).min(last);
    let (a, b) = (at(from), at(to));
    let mut out: Vec<[f32; 3]> = colours[a.min(b)..=a.max(b)]
        .iter()
        .map(|c| c.map(|v| f32::from(v) / 255.0))
        .collect();
    if a > b {
        out.reverse();
    }
    match order {
        REVERSED => out.reverse(),
        DARK_TO_LIGHT => out.sort_by(|x, y| lightness(*x).total_cmp(&lightness(*y))),
        LIGHT_TO_DARK => out.sort_by(|x, y| lightness(*y).total_cmp(&lightness(*x))),
        // As listed (and a choice from a newer app).
        _ => debug_assert!(order == AS_LISTED || order > LIGHT_TO_DARK),
    }
    out
}

/// The dither pattern's value at pixel `(x, y)`, from just above -½ to
/// just below ½.
fn bayer(x: u32, y: u32) -> f32 {
    (BAYER[((y % 4) * 4 + x % 4) as usize] + 0.5) / 16.0 - 0.5
}

/// Grey `g` (0 to 1) as one of `swatches` in even bands.
fn step(swatches: &[[f32; 3]], g: f32) -> [f32; 3] {
    let n = swatches.len();
    let i = ((g * n as f32).floor().max(0.0) as usize).min(n - 1);
    swatches[i]
}

/// Grey `g` (0 to 1) blended between `swatches` spread evenly from 0 to 1.
fn smooth(swatches: &[[f32; 3]], g: f32) -> [f32; 3] {
    let n = swatches.len();
    if n == 1 {
        return swatches[0];
    }
    let t = g.clamp(0.0, 1.0) * (n - 1) as f32;
    let i = (t.floor() as usize).min(n - 2);
    let f = t - i as f32;
    let (a, b) = (swatches[i], swatches[i + 1]);
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * f)
}

/// An sRGB channel (0 to 1) as light.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// An sRGB colour in the Oklab colour space, where distances match how
/// different colours look.
pub(crate) fn oklab(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c.map(|v| linear(v.clamp(0.0, 1.0)));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// The index of the swatch (given in Oklab) nearest `c` (Oklab).
fn nearest(swatches: &[[f32; 3]], c: [f32; 3]) -> usize {
    let mut best = (0, f32::INFINITY);
    for (i, s) in swatches.iter().enumerate() {
        let d = (s[0] - c[0]).powi(2) + (s[1] - c[1]).powi(2) + (s[2] - c[2]).powi(2);
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

/// A real number from 0 to 1 (not a number: 0).
fn unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

impl BoxType for PaletteMap {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let name = job.palette("palette");
        let palette = job.palettes().usable(&job.box_name(), "palette", &name)?;
        let used = swatches(
            &palette.colours,
            job.whole("from"),
            job.whole("to"),
            job.choice("order"),
        );
        let mode = job.choice("mode");
        let dither = unit(job.number("dither"));
        let grey = job.grey("in");
        let colour = job.input("colour").and_then(Value::colour);
        // What each pixel starts from: a grey for Steps and Smooth (the
        // colour's lightness when only a colour is plugged in), a colour
        // for Nearest (the grey as a grey colour when only a grey is).
        let grey_at = |x: u32, y: u32| -> f32 {
            match (grey, colour) {
                (Some(g), _) => {
                    let n = i64::from(g.size());
                    unit(g.at(i64::from(x) % n, i64::from(y) % n))
                }
                (None, Some(c)) => {
                    let n = i64::from(c.size());
                    lightness(c.at(i64::from(x) % n, i64::from(y) % n).map(unit))
                }
                (None, None) => 0.0,
            }
        };
        let colour_at = |x: u32, y: u32| -> [f32; 3] {
            match (colour, grey) {
                (Some(c), _) => {
                    let n = i64::from(c.size());
                    c.at(i64::from(x) % n, i64::from(y) % n).map(unit)
                }
                (None, _) => [grey_at(x, y); 3],
            }
        };
        let n = used.len() as f32;
        let out = match mode {
            SMOOTH => job.fill_rgb(|x, y| smooth(&used, grey_at(x, y))),
            NEAREST => {
                let labs: Vec<[f32; 3]> = used.iter().map(|c| oklab(*c)).collect();
                job.fill_rgb(|x, y| {
                    let push = bayer(x, y) * dither * COLOUR_DITHER;
                    let c = colour_at(x, y).map(|v| unit(v + push));
                    used[nearest(&labs, oklab(c))]
                })
            }
            _ => job.fill_rgb(|x, y| step(&used, grey_at(x, y) + bayer(x, y) * dither / n)),
        };
        Ok(vec![Value::Colour(Arc::new(out))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{Image, Size};
    use crate::palettes::{Palette, Palettes};
    use crate::registry::Registry;
    use crate::settings::{SettingValue, Settings};

    const RED: [u8; 3] = [255, 0, 0];
    const DARK: [u8; 3] = [20, 20, 20];
    const MID: [u8; 3] = [120, 120, 120];
    const LIGHT: [u8; 3] = [240, 240, 240];

    fn palettes() -> Palettes {
        Palettes::new([
            Palette {
                name: "Greys".into(),
                colours: vec![LIGHT, DARK, MID],
            },
            Palette {
                name: "Red".into(),
                colours: vec![RED],
            },
        ])
    }

    fn f(c: [u8; 3]) -> [f32; 3] {
        c.map(|v| f32::from(v) / 255.0)
    }

    /// A picture `n` across made by `f(x, y)`.
    fn picture<P: Copy>(n: u32, f: impl Fn(u32, u32) -> P) -> Image<P> {
        let pixels = (0..n * n).map(|i| f(i % n, i / n)).collect();
        Image::from_pixels(Size::new(n).unwrap(), pixels).unwrap()
    }

    /// A height going from 0 at the left to almost 1 at the right.
    fn ramp(n: u32) -> Value {
        Value::Height(Arc::new(picture(n, |x, _| x as f32 / n as f32)))
    }

    fn run(
        settings: &[(&str, SettingValue)],
        inputs: Vec<Option<Value>>,
        p: Palettes,
    ) -> Result<Image<[f32; 3]>, Refusal> {
        let kind = PaletteMap::new();
        let mut s = Settings::new();
        for (k, v) in settings {
            s.set(*k, v.clone());
        }
        let size = Size::new(16).unwrap();
        let job = Job::new(kind.info(), size, 1, 2, &s, inputs).with_palettes(p);
        let mut out = kind.run(&job)?;
        match out.remove(0) {
            Value::Colour(c) => Ok((*c).clone()),
            _ => panic!("not a colour"),
        }
    }

    #[test]
    fn steps_give_each_band_one_swatch_dark_to_light() {
        let out = run(&[], vec![Some(ramp(16)), None], palettes()).unwrap();
        // Three even bands: dark, mid, light.
        assert_eq!(out.at(0, 0), f(DARK));
        assert_eq!(out.at(5, 3), f(DARK));
        assert_eq!(out.at(6, 0), f(MID));
        assert_eq!(out.at(10, 0), f(MID));
        assert_eq!(out.at(11, 0), f(LIGHT));
        assert_eq!(out.at(15, 9), f(LIGHT));
        // As listed: light, dark, mid.
        let listed = run(
            &[("order", SettingValue::Choice(AS_LISTED))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_eq!(listed.at(0, 0), f(LIGHT));
        assert_eq!(listed.at(15, 0), f(MID));
        let reversed = run(
            &[("order", SettingValue::Choice(REVERSED))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_eq!(reversed.at(0, 0), f(MID));
        assert_eq!(reversed.at(15, 0), f(LIGHT));
        let light_first = run(
            &[("order", SettingValue::Choice(LIGHT_TO_DARK))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_eq!(light_first.at(0, 0), f(LIGHT));
        assert_eq!(light_first.at(15, 0), f(DARK));
    }

    #[test]
    fn smooth_blends_between_the_swatches() {
        let out = run(
            &[("mode", SettingValue::Choice(SMOOTH))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_eq!(out.at(0, 0), f(DARK));
        // Halfway is the middle swatch exactly.
        let mid = out.at(8, 0);
        assert!((mid[0] - f(MID)[0]).abs() < 1e-5, "{mid:?}");
        // Between dark and mid, not either.
        let quarter = out.at(4, 0)[0];
        assert!(quarter > f(DARK)[0] && quarter < f(MID)[0]);
        // Never outside the swatches.
        for x in 0..16 {
            let v = out.at(x, 0)[0];
            assert!((f(DARK)[0]..=f(LIGHT)[0] + 1e-6).contains(&v));
        }
    }

    #[test]
    fn nearest_snaps_colours_to_the_closest_swatch() {
        let colours = Value::Colour(Arc::new(picture(16, |x, _| {
            if x < 8 {
                [0.9, 0.1, 0.1]
            } else {
                [0.45, 0.47, 0.5]
            }
        })));
        let mut both = palettes();
        both = Palettes::new(both.iter().cloned().chain([Palette {
            name: "All".into(),
            colours: vec![LIGHT, DARK, MID, RED],
        }]));
        let out = run(
            &[
                ("mode", SettingValue::Choice(NEAREST)),
                ("palette", SettingValue::Palette("All".into())),
            ],
            vec![None, Some(colours)],
            both,
        )
        .unwrap();
        assert_eq!(out.at(0, 0), f(RED));
        assert_eq!(out.at(12, 0), f(MID));
        // Only a grey plugged in: the grey as a colour.
        let greys = run(
            &[("mode", SettingValue::Choice(NEAREST))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_eq!(greys.at(0, 0), f(DARK));
        assert_eq!(greys.at(15, 0), f(LIGHT));
        // Every pixel is one of the swatches.
        for y in 0..16 {
            for x in 0..16 {
                assert!([f(DARK), f(MID), f(LIGHT)].contains(&greys.at(x, y)));
            }
        }
    }

    #[test]
    fn first_and_last_choose_the_swatches_used() {
        // Swatches 2 to 3 (dark and mid) only.
        let out = run(
            &[
                ("from", SettingValue::Whole(2)),
                ("to", SettingValue::Whole(3)),
            ],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_eq!(out.at(0, 0), f(DARK));
        assert_eq!(out.at(15, 0), f(MID));
        assert_eq!(
            swatches(&[LIGHT, DARK, MID], 3, 2, AS_LISTED),
            [f(MID), f(DARK)]
        );
        // Past the end is the last; one swatch is all one colour.
        assert_eq!(swatches(&[LIGHT, DARK, MID], 9, 900, AS_LISTED), [f(MID)]);
        assert_eq!(swatches(&[LIGHT, DARK, MID], -4, 1, AS_LISTED), [f(LIGHT)]);
        assert!(swatches(&[], 1, 4, AS_LISTED).is_empty());
        let red = run(
            &[("palette", SettingValue::Palette("Red".into()))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert!((0..16).all(|x| red.at(x, 3) == f(RED)));
        let smooth_red = run(
            &[
                ("palette", SettingValue::Palette("Red".into())),
                ("mode", SettingValue::Choice(SMOOTH)),
            ],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert!((0..16).all(|x| smooth_red.at(x, 3) == f(RED)));
    }

    #[test]
    fn dither_mixes_neighbours_and_still_tiles() {
        let plain = run(&[], vec![Some(ramp(16)), None], palettes()).unwrap();
        let dithered = run(
            &[("dither", SettingValue::Number(1.0))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap();
        assert_ne!(plain, dithered);
        // Down a column at a band's edge, both swatches show.
        let column: Vec<[f32; 3]> = (0..16).map(|y| dithered.at(5, y)).collect();
        assert!(column.contains(&f(DARK)) && column.contains(&f(MID)));
        // The pattern repeats every four pixels down (the ramp doesn't
        // change down), so it tiles.
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(dithered.at(x, y), dithered.at(x, (y + 4) % 16));
            }
        }
    }

    #[test]
    fn a_missing_palette_is_refused_with_ways_round_it() {
        let gone = run(
            &[("palette", SettingValue::Palette("Stone".into()))],
            vec![Some(ramp(16)), None],
            palettes(),
        )
        .unwrap_err();
        assert!(gone.reason.contains("Stone"));
        assert_eq!(gone.fixes.len(), 2);
        let none = run(&[], vec![Some(ramp(16)), None], Palettes::default()).unwrap_err();
        assert_eq!(none.fixes, [crate::Fix::AddPalettes]);
        // Through an evaluator, the project's registry carries them.
        let registry = Registry::standard().clone().with_palettes(palettes());
        assert!(registry.get("palette_map").unwrap().uses_palettes());
        assert!(!registry.get("gradient_map").unwrap().uses_palettes());
    }

    #[test]
    fn oklab_puts_white_at_one_and_greys_on_the_axis() {
        let white = oklab([1.0; 3]);
        assert!((white[0] - 1.0).abs() < 1e-3, "{white:?}");
        let grey = oklab([0.5; 3]);
        assert!(grey[1].abs() < 1e-3 && grey[2].abs() < 1e-3, "{grey:?}");
        assert!(oklab([0.0; 3])[0].abs() < 1e-6);
    }

    #[test]
    #[ignore = "speed check: run by hand"]
    fn palette_map_speed() {
        let threads = crate::parallel::cores();
        let n = 2048;
        let size = Size::new(n).unwrap();
        let grey = Value::Height(Arc::new(picture(n, |x, y| ((x ^ y) % 256) as f32 / 255.0)));
        let colour = Value::Colour(Arc::new(picture(n, |x, y| {
            [(x % 256) as f32 / 255.0, (y % 256) as f32 / 255.0, 0.5]
        })));
        for swatches in [8u8, 32, 128] {
            let p = Palettes::new([Palette {
                name: "Many".into(),
                colours: (0..swatches).map(|i| [i, 255 - i, i / 2]).collect(),
            }]);
            for mode in [STEPS, SMOOTH, NEAREST] {
                let kind = PaletteMap::new();
                let mut s = Settings::new();
                s.set("mode", SettingValue::Choice(mode));
                s.set("dither", SettingValue::Number(0.5));
                let inputs = vec![Some(grey.clone()), Some(colour.clone())];
                let job =
                    Job::new(kind.info(), size, 1, threads, &s, inputs).with_palettes(p.clone());
                let start = std::time::Instant::now();
                kind.run(&job).unwrap();
                let took = start.elapsed();
                #[allow(clippy::print_stdout)]
                {
                    println!(
                        "mode {mode}, {swatches:>3} swatches {:>8.1} ms at 2048",
                        took.as_secs_f64() * 1000.0
                    );
                }
                assert!(took.as_secs() < 60);
            }
        }
    }
}
