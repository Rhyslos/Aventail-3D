//! The health check: each warning found when it should be and not when
//! it shouldn't, its remedies doing what they say, and graphs that can't
//! be checked (nothing plugged in, broken numbers) giving nothing.

use super::*;
use crate::boxes::trim_sheet::{ON_ROWS, OWN_HEIGHTS, SHRINK};
use crate::image::{Image, Size};
use crate::trim::{TrimBand, TrimRows};
use crate::value::MaterialMaps;
use crate::{Evaluator, ValueKind};
use std::sync::Arc;

use SettingValue::{Choice, Colour, Number, Whole};

fn add(g: &mut Graph, registry: &Registry, kind: &str) -> BoxId {
    let id = BoxId(g.next_id());
    g.apply(
        registry,
        &GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(id.0 as f32 * 200.0, 0.0),
        },
    )
    .unwrap();
    id
}

fn wire(g: &mut Graph, registry: &Registry, from: (BoxId, &str), to: (BoxId, &str)) {
    g.apply(
        registry,
        &GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        },
    )
    .unwrap();
}

fn set(g: &mut Graph, registry: &Registry, node: BoxId, key: &str, value: SettingValue) {
    g.apply(
        registry,
        &GraphEdit::SetSetting {
            node,
            key: key.into(),
            value,
        },
    )
    .unwrap();
}

fn checked(registry: &Registry, g: &Graph, n: u32, others: &[PixelArt]) -> Vec<Warning> {
    let evaluation = Evaluator::new().run(registry, g, Size::new(n).unwrap(), &[]);
    check(registry, g, &evaluation, others)
}

fn concerns(warnings: &[Warning]) -> Vec<Concern> {
    warnings.iter().map(|w| w.concern).collect()
}

fn find(warnings: &[Warning], concern: Concern) -> &Warning {
    warnings
        .iter()
        .find(|w| w.concern == concern)
        .unwrap_or_else(|| panic!("no {concern:?} in {:?}", concerns(warnings)))
}

/// Carries out `remedy`'s edits on `g`.
fn apply(registry: &Registry, g: &mut Graph, remedy: &Remedy) {
    for edit in remedy.action.edits(g) {
        g.apply(registry, &edit).unwrap();
    }
}

/// The parts of a material: planks for the height, a colour, Material
/// from height and Material out at `size_cm`. Returns the graph, the
/// colour's box and Material from height.
struct Parts {
    g: Graph,
    colour: BoxId,
    material: BoxId,
    out: BoxId,
}

/// Planks' shape as the height, a Colour fill of `colour`, into Material
/// from height and Material out at `size_cm`.
fn planks_in(registry: &Registry, colour: [f32; 3], size_cm: f32) -> Parts {
    let mut g = Graph::new("Planks");
    let planks = add(&mut g, registry, "planks");
    let fill = add(&mut g, registry, "colour_fill");
    set(&mut g, registry, fill, "colour", Colour(colour));
    let material = add(&mut g, registry, "material_from_height");
    let out = add(&mut g, registry, MATERIAL_OUT);
    set(&mut g, registry, out, REAL_SIZE, Number(size_cm));
    wire(&mut g, registry, (planks, "shape"), (material, "height"));
    wire(&mut g, registry, (fill, "colour"), (material, "colour"));
    wire(&mut g, registry, (material, "material"), (out, "material"));
    Parts {
        g,
        colour: fill,
        material,
        out,
    }
}

/// `maker`'s greys (with `settings`) coloured black to white by a
/// Gradient map and used as both colour and height.
fn greys_in(
    registry: &Registry,
    maker: &str,
    settings: &[(&str, SettingValue)],
    size_cm: f32,
) -> Parts {
    let mut g = Graph::new("Greys");
    let make = add(&mut g, registry, maker);
    for (key, value) in settings {
        set(&mut g, registry, make, key, value.clone());
    }
    let map = add(&mut g, registry, "gradient_map");
    let material = add(&mut g, registry, "material_from_height");
    let out = add(&mut g, registry, MATERIAL_OUT);
    set(&mut g, registry, out, REAL_SIZE, Number(size_cm));
    wire(&mut g, registry, (make, "height"), (map, "in"));
    wire(&mut g, registry, (make, "height"), (material, "height"));
    wire(&mut g, registry, (map, "colour"), (material, "colour"));
    wire(&mut g, registry, (material, "material"), (out, "material"));
    Parts {
        g,
        colour: map,
        material,
        out,
    }
}

#[test]
fn a_well_made_material_is_healthy() {
    let r = Registry::standard();
    let parts = planks_in(r, [0.55, 0.4, 0.25], 200.0);
    let warnings = checked(r, &parts.g, 256, &[]);
    assert!(warnings.is_empty(), "{:?}", concerns(&warnings));
}

#[test]
fn colours_too_dark_or_bright_are_found_and_fixed() {
    let r = Registry::standard();
    let mut parts = planks_in(r, [0.03, 0.03, 0.03], 200.0);
    let warnings = checked(r, &parts.g, 128, &[]);
    let dark = find(&warnings, Concern::TooDark);
    assert!(
        dark.words.contains("darker than charcoal"),
        "{}",
        dark.words
    );
    assert!(!concerns(&warnings).contains(&Concern::TooBright));
    // The first remedy lightens the colour where it goes into Material
    // from height; the second looks at the box making it.
    match &dark.remedies[0].action {
        Action::InsertBox {
            from,
            to,
            kind,
            settings,
            ..
        } => {
            assert_eq!(from.node, parts.colour);
            assert_eq!(*to, PlugRef::new(parts.material, "colour"));
            assert_eq!(kind, "hue_saturation");
            assert_eq!(settings[0], ("lightness".to_string(), Number(0.2)));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(dark.remedies[1].action, Action::LookAt(parts.colour));
    assert!(dark.remedies[0].words.contains("lighter"));
    let remedy = dark.remedies[0].clone();
    apply(r, &mut parts.g, &remedy);
    let after = checked(r, &parts.g, 128, &[]);
    assert!(
        !concerns(&after).contains(&Concern::TooDark),
        "{:?}",
        concerns(&after)
    );

    let parts = planks_in(r, [0.99, 0.99, 0.97], 200.0);
    let warnings = checked(r, &parts.g, 128, &[]);
    let bright = find(&warnings, Concern::TooBright);
    assert!(bright.words.contains("fresh snow"));
    assert!(bright.remedies[0].words.contains("darker"));
    match &bright.remedies[0].action {
        Action::InsertBox { settings, .. } => assert_eq!(settings[0].1, Number(-0.2)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn dark_metal_is_found_but_bright_metal_is_fine() {
    let r = Registry::standard();
    let mut parts = planks_in(r, [0.2, 0.2, 0.2], 200.0);
    set(&mut parts.g, r, parts.material, "metal", Number(1.0));
    let warnings = checked(r, &parts.g, 128, &[]);
    let metal = find(&warnings, Concern::DarkMetal);
    assert!(metal.words.contains("100%"), "{}", metal.words);
    // The same grey isn't too dark for a stone.
    assert!(!concerns(&warnings).contains(&Concern::TooDark));
    let mut parts = planks_in(r, [0.8, 0.8, 0.82], 200.0);
    set(&mut parts.g, r, parts.material, "metal", Number(1.0));
    assert!(checked(r, &parts.g, 128, &[]).is_empty());
}

#[test]
fn fine_noise_turns_to_mush_and_big_patches_help() {
    let r = Registry::standard();
    let mut parts = greys_in(r, "noise", &[("scale", Whole(64))], 300.0);
    let warnings = checked(r, &parts.g, 256, &[]);
    let mush = find(&warnings, Concern::Mush);
    assert!(mush.words.contains(" m away"), "{}", mush.words);
    let remedy = mush.remedies[0].clone();
    match &remedy.action {
        Action::InsertBox {
            feed: Some(feed), ..
        } => assert_eq!(feed.kind, "clouds"),
        other => panic!("{other:?}"),
    }
    let next = parts.g.next_id();
    apply(r, &mut parts.g, &remedy);
    // The two new boxes are wired in: Clouds into Hue and saturation's
    // Where, and that into Material from height's colour.
    let hue = BoxId(next);
    let clouds = BoxId(next + 1);
    assert_eq!(parts.g.get(hue).unwrap().kind, "hue_saturation");
    assert_eq!(parts.g.get(clouds).unwrap().kind, "clouds");
    assert_eq!(
        parts.g.wire_into(&PlugRef::new(hue, "where")).unwrap().from,
        PlugRef::new(clouds, "height")
    );
    assert_eq!(
        parts
            .g
            .wire_into(&PlugRef::new(parts.material, "colour"))
            .unwrap()
            .from,
        PlugRef::new(hue, "out")
    );
    let after = checked(r, &parts.g, 256, &[]);
    assert!(
        !concerns(&after).contains(&Concern::Mush),
        "{:?}",
        concerns(&after)
    );
    // Big shapes keep their light and dark: planks at the same size.
    let planks = planks_in(r, [0.55, 0.4, 0.25], 300.0);
    assert!(!concerns(&checked(r, &planks.g, 256, &[])).contains(&Concern::Mush));
}

#[test]
fn a_patch_marking_the_repeat_is_found_and_more_patches_hide_it() {
    let r = Registry::standard();
    let mut parts = greys_in(r, "clouds", &[("scale", Whole(2))], 200.0);
    let warnings = checked(r, &parts.g, 128, &[]);
    let repeats = find(&warnings, Concern::Repeats);
    assert!(repeats.words.contains("each repeat"), "{}", repeats.words);
    assert!(repeats.words.contains("200 cm"), "{}", repeats.words);
    let maker = BoxId(1);
    assert_eq!(
        repeats.remedies[0].action,
        Action::SetSetting {
            node: maker,
            key: "scale".into(),
            value: Whole(MANY_PATCHES_FOR_TESTS)
        }
    );
    let remedy = repeats.remedies[0].clone();
    apply(r, &mut parts.g, &remedy);
    let after = checked(r, &parts.g, 128, &[]);
    assert!(
        !concerns(&after).contains(&Concern::Repeats),
        "{:?}",
        concerns(&after)
    );
}

const MANY_PATCHES_FOR_TESTS: i32 = 6;

#[test]
fn nothing_to_check_gives_nothing() {
    let r = Registry::standard();
    assert!(checked(r, &Graph::new("Empty"), 32, &[]).is_empty());
    // Material out with nothing in it: no material to look at.
    let mut g = Graph::new("Waiting");
    add(&mut g, r, MATERIAL_OUT);
    assert!(checked(r, &g, 32, &[]).is_empty());
}

#[test]
fn pixel_art_with_pixelate_not_last_is_found() {
    let r = Registry::standard();
    let mut parts = planks_in(r, [0.55, 0.4, 0.25], 200.0);
    let pixelate = add(&mut parts.g, r, PIXELATE);
    let turn = add(&mut parts.g, r, "transform");
    wire(
        &mut parts.g,
        r,
        (parts.material, "material"),
        (pixelate, "material"),
    );
    wire(
        &mut parts.g,
        r,
        (pixelate, "material_out"),
        (turn, "material"),
    );
    wire(
        &mut parts.g,
        r,
        (turn, "material_out"),
        (parts.out, "material"),
    );
    let warnings = checked(r, &parts.g, 64, &[]);
    let late = find(&warnings, Concern::PixelateNotLast);
    assert!(late.words.contains("Transform"), "{}", late.words);
    assert_eq!(late.remedies[0].action, Action::LookAt(pixelate));
    // Bypassed, it doesn't count.
    let mut off = parts.g.clone();
    off.apply(
        r,
        &GraphEdit::Bypass {
            node: pixelate,
            on: true,
        },
    )
    .unwrap();
    assert!(!concerns(&checked(r, &off, 64, &[])).contains(&Concern::PixelateNotLast));
    // Last, the material is pixel art.
    wire(
        &mut parts.g,
        r,
        (pixelate, "material_out"),
        (parts.out, "material"),
    );
    assert!(!concerns(&checked(r, &parts.g, 64, &[])).contains(&Concern::PixelateNotLast));
    assert!(PixelArt::of(r, &parts.g).is_some());
}

#[test]
fn pixel_art_unlike_the_projects_other_pixel_art_is_found() {
    let r = Registry::standard();
    let mut parts = planks_in(r, [0.55, 0.4, 0.25], 200.0);
    let pixelate = add(&mut parts.g, r, PIXELATE);
    wire(
        &mut parts.g,
        r,
        (parts.material, "material"),
        (pixelate, "material"),
    );
    wire(
        &mut parts.g,
        r,
        (pixelate, "material_out"),
        (parts.out, "material"),
    );
    let mine = PixelArt::of(r, &parts.g).unwrap();
    assert_eq!(mine.pixels, 32);
    assert!((mine.per_metre() - 16.0).abs() < 1e-4);
    let same = PixelArt {
        name: "Stone".into(),
        pixels: 64,
        real_size_cm: 400.0,
    };
    assert!(checked(r, &parts.g, 64, std::slice::from_ref(&same)).is_empty());
    let finer = PixelArt {
        name: "Bricks".into(),
        pixels: 32,
        real_size_cm: 100.0,
    };
    let warnings = checked(r, &parts.g, 64, &[same, finer.clone()]);
    let sizes = find(&warnings, Concern::PixelSizes);
    assert!(sizes.words.contains("Bricks has 32"), "{}", sizes.words);
    assert!(sizes.words.contains("16 pixels a metre"), "{}", sizes.words);
    assert_eq!(
        sizes.remedies[0].action,
        Action::SetSetting {
            node: parts.out,
            key: REAL_SIZE.into(),
            value: Number(100.0)
        }
    );
    let remedy = sizes.remedies[0].clone();
    apply(r, &mut parts.g, &remedy);
    assert!(checked(r, &parts.g, 64, &[finer]).is_empty());
}

fn rows(bands: &[(f32, f32)]) -> TrimRows {
    TrimRows::new(bands.iter().enumerate().map(|(i, (from, to))| TrimBand {
        name: format!("Row {}", i + 1),
        from: *from,
        to: *to,
    }))
}

/// A trim sheet of `strips` copies of the planks material, laid `lay`.
fn trim_in(registry: &Registry, strips: usize, lay: u32) -> (Graph, BoxId, BoxId) {
    let mut parts = planks_in(registry, [0.55, 0.4, 0.25], 200.0);
    let sheet = add(&mut parts.g, registry, crate::boxes::trim_sheet::TRIM_SHEET);
    set(&mut parts.g, registry, sheet, "lay", Choice(lay));
    for key in crate::boxes::trim_sheet::STRIP_KEYS.iter().take(strips) {
        wire(
            &mut parts.g,
            registry,
            (parts.material, "material"),
            (sheet, key),
        );
    }
    wire(
        &mut parts.g,
        registry,
        (sheet, "material_out"),
        (parts.out, "material"),
    );
    (parts.g, sheet, parts.material)
}

#[test]
fn a_trim_sheet_and_the_projects_rows_disagreeing_is_found() {
    let plain = Registry::standard().clone();
    // On the project's rows, which it hasn't got.
    let (g, sheet, _) = trim_in(&plain, 2, ON_ROWS);
    let warnings = checked(&plain, &g, 64, &[]);
    let none = find(&warnings, Concern::NoTrimRows);
    assert_eq!(none.remedies[0].action, Action::MakeTrimRows(sheet));
    assert!(none.remedies[0].action.edits(&g).is_empty());
    // More strips than rows.
    let two = plain
        .clone()
        .with_trim_rows(rows(&[(0.0, 0.5), (0.5, 1.0)]));
    let (g, sheet, _) = trim_in(&two, 3, ON_ROWS);
    let warnings = checked(&two, &g, 64, &[]);
    let past = find(&warnings, Concern::PastTrimRows);
    assert!(
        past.words.starts_with("Strip 3 has a material"),
        "{}",
        past.words
    );
    assert_eq!(
        past.remedies[0].action,
        Action::SetSetting {
            node: sheet,
            key: "lay".into(),
            value: Choice(OWN_HEIGHTS)
        }
    );
    // On the rows, two strips on two rows: fine.
    let (g, _, _) = trim_in(&two, 2, ON_ROWS);
    let warnings = checked(&two, &g, 64, &[]);
    assert!(
        !concerns(&warnings).iter().any(|c| matches!(
            c,
            Concern::NoTrimRows | Concern::PastTrimRows | Concern::RowsDiffer
        )),
        "{:?}",
        concerns(&warnings)
    );
    // By their own heights: two even strips match rows halving the
    // texture, and not rows elsewhere.
    let (g, sheet, _) = trim_in(&two, 2, OWN_HEIGHTS);
    assert!(!concerns(&checked(&two, &g, 64, &[])).contains(&Concern::RowsDiffer));
    let thirds = plain.with_trim_rows(rows(&[(0.0, 0.33), (0.33, 1.0)]));
    let warnings = checked(&thirds, &g, 64, &[]);
    let differ = find(&warnings, Concern::RowsDiffer);
    assert_eq!(differ.remedies[0].action, Action::MakeTrimRows(sheet));
    assert_eq!(
        differ.remedies[1].action,
        Action::SetSetting {
            node: sheet,
            key: "lay".into(),
            value: Choice(ON_ROWS)
        }
    );
    // A trim sheet's top-bottom join is expected, never reported.
    assert!(!concerns(&warnings).contains(&Concern::Joins));
}

#[test]
fn a_cut_strip_cutting_through_its_pattern_is_found() {
    let r = Registry::standard().clone();
    let (mut g, _, material) = trim_in(&r, 1, OWN_HEIGHTS);
    set(&mut g, &r, BoxId(1), "rows", Whole(4));
    // One strip fills the whole sheet: whole rows of planks.
    assert!(!concerns(&checked(&r, &g, 128, &[])).contains(&Concern::CutThrough));
    // Four rows of planks on rows cutting a strip at 28%: a sliver of the
    // second row of planks shows at its bottom.
    let thirds = r.with_trim_rows(rows(&[(0.0, 0.28), (0.28, 1.0)]));
    let (mut g, sheet2, _) = trim_in(&thirds, 2, ON_ROWS);
    set(&mut g, &thirds, BoxId(1), "rows", Whole(4));
    let warnings = checked(&thirds, &g, 128, &[]);
    let cut = find(&warnings, Concern::CutThrough);
    assert!(
        cut.words.starts_with("Strip 1 cuts its material"),
        "{}",
        cut.words
    );
    assert!(cut.words.contains("bottom edge"), "{}", cut.words);
    assert_eq!(
        cut.remedies[0].action,
        Action::SetSetting {
            node: sheet2,
            key: "fit1".into(),
            value: Choice(SHRINK)
        }
    );
    assert_eq!(cut.remedies[1].action, Action::LookAt(material));
    let remedy = cut.remedies[0].clone();
    apply(&thirds, &mut g, &remedy);
    let after = checked(&thirds, &g, 128, &[]);
    assert!(
        after
            .iter()
            .all(|w| w.concern != Concern::CutThrough || w.words.starts_with("Strip 2")),
        "{:?}",
        after.iter().map(|w| &w.words).collect::<Vec<_>>()
    );
}

#[test]
fn cuts_are_measured_from_the_lines() {
    // Lines at rows 0–1 and 16–17 of 32: rows 16 apart.
    let lines = [(0usize, 1usize), (16, 17)];
    assert_eq!(trim::cut_of(&lines, 16, 32), None);
    assert_eq!(trim::cut_of(&lines, 32, 32), None);
    // Ending 3 rows into the second row: a sliver.
    let cut = trim::cut_of(&lines, 21, 32).unwrap();
    assert_eq!(cut.bottom, Some((11, 3)));
    assert_eq!(cut.top, None);
    // Ending most of the way through it: a narrower row, fine.
    assert_eq!(trim::cut_of(&lines, 29, 32), None);
    // Ending inside the only row shown: one narrower piece, fine.
    assert_eq!(trim::cut_of(&lines, 8, 32), None);
    // No lines, or no rows: nothing is cut through.
    assert_eq!(trim::cut_of(&[], 8, 32), None);
    assert_eq!(trim::cut_of(&lines, 0, 32), None);
    // A top edge a little above a line: a sliver at the top.
    let off = [(4usize, 5usize), (20, 21)];
    let cut = trim::cut_of(&off, 16, 32).unwrap();
    assert_eq!(cut.top, Some(4));
}

#[test]
fn joins_that_show_are_found_and_a_seamless_box_is_offered() {
    let r = Registry::standard();
    let parts = planks_in(r, [0.55, 0.4, 0.25], 200.0);
    let evaluation = Evaluator::new().run(r, &parts.g, Size::new(32).unwrap(), &[]);
    let looking = Looking {
        registry: r,
        graph: &parts.g,
        evaluation: &evaluation,
        out: Some(parts.out),
    };
    let s = Size::new(32).unwrap();
    let ramp = |x: u32, _| x as f32 / 31.0;
    let flat = Arc::new(Image::filled(s, 0.5f32));
    let m = MaterialMaps {
        colour: Arc::new(crate::parallel::fill(s, 1, |x, y| [ramp(x, y), 0.3, 0.3])),
        height: flat.clone(),
        roughness: flat.clone(),
        metal: Arc::new(Image::filled(s, 0.0)),
        normal: Arc::new(Image::filled(s, [0.0, 0.0, 1.0])),
        occlusion: flat.clone(),
        curvature: flat,
        opacity: None,
        translucency: None,
    };
    let mut out = Vec::new();
    looks::joins(&looking, &m, false, &mut out);
    let joins = find(&out, Concern::Joins);
    assert!(joins.words.contains("left and right"));
    match &joins.remedies[0].action {
        Action::InsertBox { kind, to, .. } => {
            assert_eq!(kind, "seamless");
            assert_eq!(*to, PlugRef::new(parts.out, "material"));
        }
        other => panic!("{other:?}"),
    }
    // Turned on its side and checked as a trim sheet: expected.
    let down = MaterialMaps {
        colour: Arc::new(crate::parallel::fill(s, 1, |x, y| [ramp(y, x), 0.3, 0.3])),
        ..m
    };
    let mut out = Vec::new();
    looks::joins(&looking, &down, true, &mut out);
    assert!(out.is_empty());
    looks::joins(&looking, &down, false, &mut out);
    assert_eq!(concerns(&out), vec![Concern::Joins]);
}

#[test]
fn broken_numbers_give_no_warnings_and_no_crash() {
    let r = Registry::standard();
    let s = Size::new(16).unwrap();
    let broken = Arc::new(Image::filled(s, f32::NAN));
    let m = MaterialMaps {
        colour: Arc::new(Image::filled(s, [f32::NAN; 3])),
        height: broken.clone(),
        roughness: broken.clone(),
        metal: broken.clone(),
        normal: Arc::new(Image::filled(s, [f32::NAN; 3])),
        occlusion: broken.clone(),
        curvature: broken,
        opacity: None,
        translucency: None,
    };
    let g = Graph::new("Broken");
    let evaluation = Evaluator::new().run(r, &g, s, &[]);
    let looking = Looking {
        registry: r,
        graph: &g,
        evaluation: &evaluation,
        out: None,
    };
    let mut out = Vec::new();
    looks::colours(&looking, &m, &mut out);
    let look = measure::look(&m);
    looks::mush(&looking, &m, &look, &mut out);
    looks::repeats(&looking, &m, &look, &mut out);
    assert!(out.is_empty(), "{:?}", concerns(&out));
    assert!(measure::lines_across(&m.height, &look).is_none());
}

#[test]
fn lengths_and_places_read_plainly() {
    assert_eq!(length_words(200.0), "200 cm");
    assert_eq!(length_words(2.345), "2.3 cm");
    assert_eq!(length_words(1250.0), "12.5 m");
    assert_eq!(looks::place_words(0, 0, 4), "top left");
    assert_eq!(looks::place_words(2, 1, 4), "middle");
    assert_eq!(looks::place_words(3, 3, 4), "bottom right");
    assert_eq!(looks::place_words(0, 2, 4), "left side");
}

#[test]
fn fading_is_measured_in_metres() {
    // Halved at the first averaging, one pixel 1 cm: gone 2 cm blocks
    // away, about 19 m.
    let (metres, cm) = looks::fades_at(&[0.2, 0.05, 0.01], 1.0).unwrap();
    assert!((cm - 2.0).abs() < 1e-6);
    assert!((metres - 2.0 / 0.1069).abs() < 0.01);
    // Plain, or kept to the end: nothing.
    assert!(looks::fades_at(&[0.01, 0.0], 1.0).is_none());
    assert!(looks::fades_at(&[0.2, 0.2, 0.2], 1.0).is_none());
    assert!(looks::fades_at(&[], 1.0).is_none());
    assert!(looks::fades_at(&[f32::NAN, 0.0], 1.0).is_none());
}

/// Prints the measures of each test material, to set the limits by
/// (`cargo test -p matgraph health::tests::measures -- --ignored
/// --nocapture`).
#[test]
#[ignore = "prints the measures used to choose the limits"]
fn measures() {
    let r = Registry::standard();
    let cases: Vec<(&str, Parts)> = vec![
        ("planks", planks_in(r, [0.55, 0.4, 0.25], 200.0)),
        (
            "noise 64",
            greys_in(r, "noise", &[("scale", Whole(64))], 300.0),
        ),
        (
            "noise 8",
            greys_in(r, "noise", &[("scale", Whole(8))], 300.0),
        ),
        (
            "clouds 2/1",
            greys_in(
                r,
                "clouds",
                &[("scale", Whole(2)), ("detail", Whole(1))],
                200.0,
            ),
        ),
        (
            "clouds 6/1",
            greys_in(
                r,
                "clouds",
                &[("scale", Whole(6)), ("detail", Whole(1))],
                200.0,
            ),
        ),
        (
            "clouds 8/1",
            greys_in(
                r,
                "clouds",
                &[("scale", Whole(8)), ("detail", Whole(1))],
                200.0,
            ),
        ),
        (
            "clouds 6",
            greys_in(r, "clouds", &[("scale", Whole(6))], 200.0),
        ),
        (
            "clouds 2",
            greys_in(r, "clouds", &[("scale", Whole(2))], 200.0),
        ),
        (
            "clouds 3",
            greys_in(r, "clouds", &[("scale", Whole(3))], 200.0),
        ),
        (
            "noise 64 small",
            greys_in(r, "noise", &[("scale", Whole(64))], 100.0),
        ),
        (
            "noise 32",
            greys_in(r, "noise", &[("scale", Whole(32))], 200.0),
        ),
        (
            "clouds 4",
            greys_in(r, "clouds", &[("scale", Whole(4))], 200.0),
        ),
        ("cells", greys_in(r, "cells", &[], 200.0)),
    ];
    for (name, parts) in cases {
        let evaluation = Evaluator::new().run(r, &parts.g, Size::new(256).unwrap(), &[]);
        let m = evaluation
            .picture(parts.out)
            .and_then(Value::material)
            .unwrap();
        let look = measure::look(m);
        let spreads = measure::spreads(&look, 256);
        let patches = measure::patches(&look, 256, 4);
        println!(
            "{name}: waves {} spreads {spreads:?} patches {patches:?}",
            measure::slowest_waves(&look, 256)
        );
    }
    let _ = ValueKind::Material;
}
