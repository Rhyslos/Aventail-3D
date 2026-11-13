//! Water streaks: grime washed down a surface by rain, in long thin
//! streaks running down from where they start (milestone 25, decision
//! 0288).
//!
//! Each pixel looks **uphill**, up to Length, for a place where a streak
//! starts (spots of the box's own, or the shape in Start from), and takes
//! the strongest it finds, fainter the further away: so every start trails
//! a streak below it. It looks in jumps doubling in length (1, 2, 4…
//! pixels), each place taking the best its neighbour uphill found, so a
//! long streak costs a handful of passes, not one per pixel of length. Down is down the texture, or, in the painter,
//! down the model: the way its Up map falls across the texture, so streaks
//! bend round the model's UV layout and run down its sides, not its tops.

use super::super::blur::box_blur;
use super::super::pattern::{smooth, whole_repeats};
use super::super::{choice, colour, info, length, number, seed, uv};
use super::{Coat, Toward, coated, limited, outputs, patches, plugs, seeded, toward};
use crate::image::{Image, Rgb};
use crate::kind::ValueKind;
use crate::model::ModelMap;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use words::text;

/// The model's maps Water streaks reads.
const MAPS: &[ModelMap] = &[ModelMap::Up, ModelMap::FacingUp];

/// The Water streaks box.
#[derive(Debug)]
pub(crate) struct WaterStreaks {
    info: BoxInfo,
}

impl WaterStreaks {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "water_streaks",
                text!("Water streaks"),
                Group::Weather,
                text!(
                    "Grime washed down by rain in long, thin streaks below where water \
                     gathers: under window sills, rivets, rusty bolts. Plug a whole material \
                     into Material; the streaks start at spots of their own, or where the shape \
                     in Start from is white (Rust's Rust output gives rust streaks). In the \
                     painter they run down the model's sides (Runs: Down the model); in the \
                     Material mode, and with Runs at Down the texture, they run down the \
                     texture. Streaks gives where they are; a shape in Only where keeps them \
                     there."
                ),
                plugs(
                    text!("Streaks"),
                    vec![PlugSpec::new(
                        "start",
                        text!("Start from"),
                        ValueKind::Shape,
                        false,
                    )],
                ),
                vec![
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How many streaks start, with nothing in Start from: 0 is none."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    length(
                        "length",
                        text!("Length"),
                        text!("How far the streaks run down, in centimetres."),
                        1.0,
                        500.0,
                        40.0,
                    ),
                    length(
                        "width",
                        text!("Width"),
                        text!("How wide a streak is, with nothing in Start from, in centimetres."),
                        0.1,
                        50.0,
                        1.5,
                    ),
                    number(
                        "fade",
                        text!("Fade"),
                        text!("How much the streaks fade toward their ends: 0 is not at all."),
                        0.0,
                        1.0,
                        0.7,
                    ),
                    number(
                        "wobble",
                        text!("Wobble"),
                        text!("How much the streaks wander from side to side as they run."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    colour(
                        "streak_colour",
                        text!("Streak colour"),
                        text!("The streaks' colour: dark grime, or pale for lime."),
                        [0.17, 0.15, 0.12],
                    ),
                    number(
                        "carry",
                        text!("Carry the colour"),
                        text!(
                            "How much each streak takes the material's colour from where it \
                             starts (rust streaks below rust): 0 is the Streak colour."
                        ),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    number(
                        "strength",
                        text!("Strength"),
                        text!("How strongly the streaks colour the material."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    number(
                        "roughness",
                        text!("Streak roughness"),
                        text!("How rough the streaks are."),
                        0.0,
                        1.0,
                        0.7,
                    ),
                    choice(
                        "runs",
                        text!("Runs"),
                        text!(
                            "Which way is down: down the model in the painter (down the texture \
                             without a model), or always down the texture."
                        ),
                        &[text!("Down the model"), text!("Down the texture")],
                        0,
                    ),
                    seed(),
                ],
                &["dirt", "material_out"],
            ),
        }
    }
}

/// Which way is uphill at each pixel, in pixels (x right, y down the
/// picture), with how much streaks run there (0 none): the third part.
type Uphill = Image<[f32; 3]>;

/// Uphill down the texture: up the picture everywhere, wandering by
/// `wobble` in waves about `length_px` long.
fn up_the_texture(job: &Job<'_>, wobble: f32, length_px: f32) -> Uphill {
    let n = job.size().get() as f32;
    let waves = whole_repeats(n, length_px.max(1.0), 64) as u32;
    let seed = seeded(job, 3);
    job.fill_rgb(|x, y| {
        let (u, v) = uv(x, y, n);
        let side = (super::super::noise::layered(u, v, waves, 2, 0.5, seed) - 0.5) * 2.0;
        let d = glam::Vec2::new(side * wobble * 0.8, -1.0).normalize_or(glam::Vec2::NEG_Y);
        [d.x, d.y, 1.0]
    })
}

/// Uphill on the model: the way its Up map rises across the texture
/// (smoothed a little), wandering by `wobble`; streaks run only on its
/// sides (not facing up), and nowhere the Up map is flat.
fn up_the_model(job: &Job<'_>, up: &crate::image::Grey, wobble: f32, length_px: f32) -> Uphill {
    let n = job.size().get();
    let smooth_up = box_blur(up, (n / 256).max(1), job.threads());
    let facing = job.model_map(ModelMap::FacingUp).cloned();
    let wander = up_the_texture(job, wobble, length_px);
    job.fill_rgb(|x, y| {
        let (i, j) = (i64::from(x), i64::from(y));
        let g = glam::Vec2::new(
            smooth_up.at(i + 1, j) - smooth_up.at(i - 1, j),
            smooth_up.at(i, j + 1) - smooth_up.at(i, j - 1),
        );
        let steep = g.length();
        if steep < 1e-6 {
            return [0.0, 0.0, 0.0];
        }
        let d = g / steep;
        // Sideways wander, turned to this direction.
        let [side, _, _] = wander.at(i, j);
        let d = (d + d.perp() * side).normalize_or(d);
        let sides = facing
            .as_ref()
            .map_or(1.0, |f| 1.0 - f.at(i, j).clamp(0.0, 1.0));
        [d.x, d.y, sides]
    })
}

impl BoxType for WaterStreaks {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn model_maps(&self) -> &'static [ModelMap] {
        MAPS
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let m = job.needed_material("material")?;
        let n = job.size().get() as f32;
        let amount = job.number("amount").clamp(0.0, 1.0);
        let length_px = job.length_px("length").clamp(1.0, n * 4.0);
        let width_px = job.length_px("width").max(0.5);
        let fade = job.number("fade").clamp(0.0, 1.0);
        let wobble = job.number("wobble").clamp(0.0, 1.0);
        let carry = job.number("carry").clamp(0.0, 1.0);
        let strength = job.number("strength").clamp(0.0, 1.0);
        let streak = job.colour("streak_colour").map(|c| c.clamp(0.0, 1.0));
        // Where streaks start: the shape given, or spots of their own.
        let start = match job.grey("start") {
            Some(s) => job.map_grey(s, |v| v.clamp(0.0, 1.0)),
            None => {
                let spots = patches(job, width_px * 2.0, 2, seeded(job, 1));
                let level = 0.95 - amount * 0.4;
                let any = smooth(0.0, 0.02, amount);
                job.map_grey(&spots, |v| smooth(level - 0.06, level + 0.06, v) * any)
            }
        };
        let down_the_model = job.choice("runs") == 0;
        let uphill = match job.model_map(ModelMap::Up) {
            Some(up) if down_the_model => up_the_model(job, up, wobble, length_px),
            _ => up_the_texture(job, wobble, length_px),
        };
        // Each place's strongest start uphill (and where it is), found
        // by jumps doubling in length: after a jump of k, every place has
        // looked 2k uphill. Fade is a steady fall in strength per pixel.
        let keep_per_px = -(1.0 - 0.95 * fade).ln() / length_px;
        let mut found: Image<[f32; 3]> = job.fill_rgb(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            let runs = uphill.at(i, j)[2];
            [start.at(i, j) * runs, x as f32 + 0.5, y as f32 + 0.5]
        });
        let (mut reached, mut jump) = (0.0, 1.0_f32);
        while reached < length_px - 0.5 {
            let this = jump.min(length_px - reached);
            let keep = (-keep_per_px * this).exp();
            found = job.fill_rgb(|x, y| {
                let (i, j) = (i64::from(x), i64::from(y));
                let here = found.at(i, j);
                let [dx, dy, runs] = uphill.at(i, j);
                if runs <= 0.0 {
                    return here;
                }
                let (fx, fy) = (x as f32 + 0.5 + dx * this, y as f32 + 0.5 + dy * this);
                let there = found.at(fx.floor() as i64, fy.floor() as i64);
                let strength = there[0] * keep;
                if strength > here[0] {
                    [strength, there[1], there[2]]
                } else {
                    here
                }
            });
            reached += this;
            jump *= 2.0;
        }
        let cover = job.fill_grey(|x, y| found.at(i64::from(x), i64::from(y))[0].clamp(0.0, 1.0));
        let cover = limited(job, cover, job.grey("where"));
        let colour: Rgb = job.fill_rgb(|x, y| {
            let [_, fx, fy] = found.at(i64::from(x), i64::from(y));
            let from = m.colour.sample_rgb(fx / n, fy / n);
            toward(streak, from, carry)
        });
        let shown = job.map_grey(&cover, |c| c * strength);
        let coat = Coat {
            cover: &shown,
            colour: &colour,
            roughness: Toward::All(job.number("roughness")),
            metal: None,
            raise: None,
        };
        Ok(outputs(coated(job, m, &coat), cover))
    }
}
