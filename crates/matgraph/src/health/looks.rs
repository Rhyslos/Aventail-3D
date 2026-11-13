//! The health check's warnings about how a material looks: colours too
//! dark or too bright to light well, detail that turns to mush at a
//! distance, a repeat that shows, and joins that show.

use super::{Action, Concern, Feed, Looking, Remedy, Warning, length_words, measure};
use crate::eval::real_size_cm;
use crate::graph::PlugRef;
use crate::seams::SeamCheck;
use crate::settings::SettingValue;
use crate::value::{MaterialMaps, Value};
use words::{tr, trf};

/// How much of a material may be too dark or too bright before it is
/// worth saying (a tenth: small dark cracks and bright glints are fine).
const TOO_MUCH: f32 = 0.1;
/// What one screen pixel covers 1 m away, in centimetres: a 1080-pixel
/// tall picture with a 60° view, a common game camera.
const PIXEL_CM_AT_A_METRE: f32 = 0.1069;
/// Closer than this (metres), a material keeping less than half its light
/// and dark has turned to mush where players will see it: the far side of
/// a courtyard, a street or a big hall.
const MUSH_METRES: f32 = 50.0;
/// Less spread of light and dark than this: a plain material, nothing to
/// fade.
const PLAIN: f32 = 0.03;
/// How strong the light and dark changing once across a repeat may be
/// ([`measure::slowest_waves`], in lightness from 0 to 1) before the
/// repeat shows as a grid of patches: about eight steps in a hundred.
const PATCHY: f32 = 0.08;
/// The Clouds or Noise Scale that makes patches small enough not to mark
/// a repeat.
const MANY_PATCHES: i32 = 6;

/// A share as a whole percentage, for people.
fn percent(share: f32) -> i64 {
    (share * 100.0).round() as i64
}

pub(super) fn colours(looking: &Looking<'_>, m: &MaterialMaps, out: &mut Vec<Warning>) {
    let range = measure::colour_range(m);
    let wire = looking.colour_wire();
    let look_at = |words_for: &dyn Fn(String) -> String| -> Vec<Remedy> {
        let id = wire
            .as_ref()
            .map(|(from, _)| from.node)
            .or_else(|| looking.feeding_out().map(|p| p.node));
        id.map(|id| Remedy {
            words: words_for(looking.name(id)),
            action: Action::LookAt(id),
        })
        .into_iter()
        .collect()
    };
    let lighten = |by: f32| -> Option<Remedy> {
        let (from, to) = wire.clone()?;
        Some(Remedy {
            words: if by > 0.0 {
                trf!(
                    "Put a Hue and saturation box before {}'s colour, lighter",
                    looking.name(to.node)
                )
            } else {
                trf!(
                    "Put a Hue and saturation box before {}'s colour, darker",
                    looking.name(to.node)
                )
            },
            action: Action::InsertBox {
                from,
                to,
                kind: "hue_saturation".into(),
                input: "in".into(),
                output: "out".into(),
                settings: vec![("lightness".into(), SettingValue::Number(by))],
                feed: None,
            },
        })
    };
    // Lift the darks (or lower the brights) more when more is wrong.
    let amount = |share: f32| if share > 0.3 { 0.2 } else { 0.1 };
    if range.dark >= TOO_MUCH {
        let mut remedies: Vec<Remedy> = lighten(amount(range.dark)).into_iter().collect();
        remedies.extend(look_at(&|name| {
            trf!("Look at {}, which makes the colour", name)
        }));
        out.push(Warning {
            concern: Concern::TooDark,
            words: trf!(
                "{}% of this material is darker than charcoal (about 30 in 255). Under light \
                 it stays nearly black, so its shading and bumps can't show there; real \
                 surfaces are seldom that dark. Lighten its darkest colours. (A game showing \
                 it unlit, as many hand-painted ones do, can keep them.)",
                percent(range.dark)
            ),
            remedies,
            boxes: Vec::new(),
        });
    }
    if range.bright >= TOO_MUCH {
        let mut remedies: Vec<Remedy> = lighten(-amount(range.bright)).into_iter().collect();
        remedies.extend(look_at(&|name| {
            trf!("Look at {}, which makes the colour", name)
        }));
        out.push(Warning {
            concern: Concern::TooBright,
            words: trf!(
                "{}% of this material is brighter than fresh snow (about 240 in 255). Lit, it \
                 turns plain white and loses its detail, and it glares next to everything \
                 else. Darken its brightest colours. (A game showing it unlit can keep them.)",
                percent(range.bright)
            ),
            remedies,
            boxes: Vec::new(),
        });
    }
    // Only when there is a fair amount of metal: a few metal pixels at
    // a blend's edge don't count.
    if range.metal >= 0.05 && range.metal_dark >= TOO_MUCH * 2.0 {
        out.push(Warning {
            concern: Concern::DarkMetal,
            words: trf!(
                "{}% of its metal is darker than any real metal (about 180 in 255). Bare \
                 metal shows its colour as what it reflects, so dark metal looks like black \
                 plastic in the Realistic look and in game engines. Lighten the metal's \
                 colour, or make those parts less metal.",
                percent(range.metal_dark)
            ),
            remedies: look_at(&|name| trf!("Look at {}, which makes the colour", name)),
            boxes: Vec::new(),
        });
    }
}

/// How far away a material keeps less than half its light and dark (in
/// metres), from its `spreads` at each averaging and `cm_a_pixel`; `None`
/// for a plain material or one that keeps it to the last.
pub(super) fn fades_at(spreads: &[f32], cm_a_pixel: f32) -> Option<(f32, f32)> {
    let first = *spreads.first()?;
    if first.is_nan() || first < PLAIN {
        return None;
    }
    let level = spreads.iter().position(|s| *s < first * 0.5)?;
    let block_cm = (1u32 << level.min(30)) as f32 * cm_a_pixel;
    Some((block_cm / PIXEL_CM_AT_A_METRE, block_cm))
}

pub(super) fn mush(looking: &Looking<'_>, m: &MaterialMaps, look: &[f32], out: &mut Vec<Warning>) {
    let n = m.size() as usize;
    let cm_a_pixel = real_size_cm(looking.registry, looking.graph) / n as f32;
    let spreads = measure::spreads(look, n);
    let Some((metres, block_cm)) = fades_at(&spreads, cm_a_pixel) else {
        return;
    };
    if metres >= MUSH_METRES {
        return;
    }
    let mut remedies = Vec::new();
    if let Some((from, to)) = looking.colour_wire() {
        remedies.push(Remedy {
            words: trf!(
                "Add big patches of light and dark: a Hue and saturation box before {}'s \
                 colour, darker through a Clouds box",
                looking.name(to.node)
            ),
            action: Action::InsertBox {
                from,
                to,
                kind: "hue_saturation".into(),
                input: "in".into(),
                output: "out".into(),
                settings: vec![("lightness".into(), SettingValue::Number(-0.3))],
                feed: Some(Feed {
                    kind: "clouds".into(),
                    output: "height".into(),
                    into: "where".into(),
                    settings: vec![("scale".into(), SettingValue::Whole(8))],
                }),
            },
        });
    }
    out.push(Warning {
        concern: Concern::Mush,
        words: trf!(
            "From about {} m away, half of this material's light and dark is gone: most of \
             it is detail smaller than {}, which the engine blends into one even colour once \
             the material is small on screen. Up close it looks right; from across a room it \
             reads as a flat smear. Give it bigger shapes as well: larger patches of light and \
             dark, stronger differences between whole pieces (stones, planks, tiles), or \
             deeper gaps between them.",
            metres.max(1.0).round(),
            length_words(block_cm)
        ),
        remedies,
        boxes: Vec::new(),
    });
}

/// Where in a repeat a square of `cells` across lies, in plain words.
pub(super) fn place_words(col: usize, row: usize, cells: usize) -> String {
    let third = |i: usize| {
        if i == 0 {
            0
        } else if i + 1 >= cells {
            2
        } else {
            1
        }
    };
    match (third(row), third(col)) {
        (0, 0) => tr("top left"),
        (0, 1) => tr("top"),
        (0, _) => tr("top right"),
        (1, 0) => tr("left side"),
        (1, 1) => tr("middle"),
        (1, _) => tr("right side"),
        (_, 0) => tr("bottom left"),
        (_, 1) => tr("bottom"),
        (_, _) => tr("bottom right"),
    }
    .to_string()
}

pub(super) fn repeats(
    looking: &Looking<'_>,
    m: &MaterialMaps,
    look: &[f32],
    out: &mut Vec<Warning>,
) {
    let n = m.size() as usize;
    if measure::slowest_waves(look, n) <= PATCHY {
        return;
    }
    let Some(patches) = measure::patches(look, n, 4) else {
        return;
    };
    let real = real_size_cm(looking.registry, looking.graph);
    let (col, row) = patches.most;
    // The fewest, biggest patches upstream come from a Clouds or Noise
    // box with a small Scale: more of them hides the repeat.
    let patchy = looking
        .upstream()
        .into_iter()
        .filter(|id| {
            looking
                .graph
                .get(*id)
                .is_some_and(|b| b.kind == "clouds" || b.kind == "noise")
        })
        .filter_map(|id| match looking.setting(id, "scale") {
            Some(SettingValue::Whole(s)) if s < MANY_PATCHES => Some((s, id)),
            _ => None,
        })
        .min();
    let mut remedies = Vec::new();
    if let Some((scale, id)) = patchy {
        let more = (scale * 2).max(MANY_PATCHES);
        remedies.push(Remedy {
            words: trf!(
                "Give {} a Scale of {} instead of {}: smaller patches, many to a repeat",
                looking.name(id),
                more,
                scale
            ),
            action: Action::SetSetting {
                node: id,
                key: "scale".into(),
                value: SettingValue::Whole(more),
            },
        });
    }
    if let Some(from) = looking.feeding_out() {
        remedies.push(Remedy {
            words: trf!("Look at {}", looking.name(from.node)),
            action: Action::LookAt(from.node),
        });
    }
    out.push(Warning {
        concern: Concern::Repeats,
        words: trf!(
            "A {} patch near the {} of each repeat stands out, so the eye finds it again every \
             {} and a surface covered in it shows a grid. Spread the light and dark more \
             evenly, or make the patches smaller so many fit in one repeat.",
            if patches.lighter {
                tr("lighter")
            } else {
                tr("darker")
            },
            place_words(col, row, patches.cells),
            length_words(real)
        ),
        remedies,
        boxes: Vec::new(),
    });
}

pub(super) fn joins(looking: &Looking<'_>, m: &MaterialMaps, trim: bool, out: &mut Vec<Warning>) {
    let mut check = SeamCheck::of(&Value::Material(std::sync::Arc::new(m.clone())));
    if trim {
        // A trim sheet's last strip meets its first in a gap: expected.
        check = check.across_only();
    }
    if !check.shows() {
        return;
    }
    let mut remedies = Vec::new();
    if let (Some(from), Some(out_id)) = (looking.feeding_out(), looking.out) {
        remedies.push(Remedy {
            words: tr("Put a Make seamless box before Material out").to_string(),
            action: Action::InsertBox {
                from,
                to: PlugRef::new(out_id, "material"),
                kind: "seamless".into(),
                input: "material".into(),
                output: "material_out".into(),
                settings: Vec::new(),
                feed: None,
            },
        });
    }
    out.push(Warning {
        concern: Concern::Joins,
        words: check.describe(),
        remedies,
        boxes: Vec::new(),
    });
}
