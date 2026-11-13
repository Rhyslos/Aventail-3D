//! Milestone 26's Trim sheet box (decision 0289). The checks every box
//! gets (in range, seamless with standard settings, cores, extreme
//! settings, flat inputs) are in `tests_each.rs`; here: where the strips
//! lie (own heights, the project's rows, none), cutting and shrinking,
//! gaps and bevels, the refusals, the rows reaching the box through a
//! project's registry, and what makes a graph a trim sheet. The speed
//! check is ignored: `cargo test -p matgraph --release -- --ignored
//! trim_sheet_speed --nocapture`.

use super::*;
use crate::boxes::tests_each::{assert_in_range, assert_seamless, input, registry, size, with};
use crate::graph::{Graph, GraphEdit, PlugRef};
use crate::trim::{TrimBand, TrimRows, is_trim_sheet};
use crate::{Evaluator, GraphStore, ValueKind};
use glam::Vec2;

use SettingValue::{Choice, Number, Whole};

/// A material at `n` pixels whose colour is `colour(x, y)` (each 0 to 1
/// across the texture), half high, flat.
fn painted(n: u32, colour: impl Fn(f32, f32) -> [f32; 3]) -> Value {
    let s = size(n);
    let grey = |v| Arc::new(Image::filled(s, v));
    let pixels = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            colour((x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32)
        })
        .collect();
    Value::Material(Arc::new(MaterialMaps {
        colour: Arc::new(Image::from_pixels(s, pixels).unwrap()),
        height: grey(0.5),
        roughness: grey(0.3),
        metal: grey(0.0),
        normal: Arc::new(Image::filled(s, [0.0, 0.0, 1.0])),
        occlusion: grey(1.0),
        curvature: grey(0.5),
        opacity: None,
        translucency: None,
    }))
}

fn plain(n: u32, c: [f32; 3]) -> Value {
    painted(n, move |_, _| c)
}

const RED: [f32; 3] = [1.0, 0.0, 0.0];
const BLUE: [f32; 3] = [0.0, 0.0, 1.0];
const GREEN: [f32; 3] = [0.0, 1.0, 0.0];

/// Settings with no gap and no bevel, so strips meet edge to edge.
fn bare(more: &[(&str, SettingValue)]) -> Settings {
    let mut pairs: Vec<(&str, SettingValue)> = vec![
        ("lay", Choice(OWN_HEIGHTS)),
        ("gap", Number(0.0)),
        ("depth", Number(0.0)),
        ("edges", Choice(SQUARE)),
    ];
    pairs.extend(more.iter().cloned());
    with(&pairs)
}

/// The Trim sheet run at `n` pixels with `inputs` by key, `settings` and
/// the project's trim `rows`.
fn sheet(
    n: u32,
    inputs: &[(&str, Value)],
    settings: &Settings,
    rows: &TrimRows,
) -> Result<Vec<Value>, Refusal> {
    let kind = registry().get(TRIM_SHEET).unwrap();
    let info = kind.info();
    let values = info
        .inputs
        .iter()
        .map(|p| {
            inputs
                .iter()
                .find(|(k, _)| *k == p.key)
                .map(|(_, v)| v.clone())
        })
        .collect();
    let job = Job::new(info, size(n), 7, 2, settings, values).with_trim_rows(rows.clone());
    kind.run(&job)
}

fn material(values: &[Value]) -> &MaterialMaps {
    values[0].material().unwrap()
}

fn band(name: &str, from: f32, to: f32) -> TrimBand {
    TrimBand {
        name: name.into(),
        from,
        to,
    }
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4)
}

#[test]
fn own_heights_share_the_texture_and_skip_empty_strips() {
    let mut heights = [1; STRIPS];
    heights[3] = 2;
    let mut plugged = [false; STRIPS];
    for i in [0, 1, 3] {
        plugged[i] = true;
    }
    let laid = layout(
        OWN_HEIGHTS,
        &heights,
        &[CUT; STRIPS],
        &plugged,
        &TrimRows::default(),
    );
    assert_eq!(laid.laid, Laid::OwnHeights);
    let spans: Vec<(usize, f32, f32)> = laid
        .places
        .iter()
        .map(|p| (p.strip, p.from, p.to))
        .collect();
    assert_eq!(spans, [(0, 0.0, 0.25), (1, 0.25, 0.5), (3, 0.5, 1.0)]);
    assert!(laid.past_rows.is_empty());
    // Rows chosen but none made: their own heights, said so.
    let none = layout(
        ON_ROWS,
        &heights,
        &[CUT; STRIPS],
        &plugged,
        &TrimRows::default(),
    );
    assert_eq!(none.laid, Laid::NoRows);
    assert_eq!(none.places.len(), 3);
    // Nothing plugged in: nothing laid.
    let empty = layout(
        OWN_HEIGHTS,
        &heights,
        &[CUT; STRIPS],
        &[false; STRIPS],
        &TrimRows::default(),
    );
    assert!(empty.places.is_empty());
}

#[test]
fn strips_lie_on_the_projects_rows_in_order() {
    let rows = TrimRows::new([
        band("Seal", 0.5, 0.75),
        band("Bolts", 0.0, 0.125),
        band("Panel edge", 0.125, 0.5),
    ]);
    let mut plugged = [false; STRIPS];
    for i in [0, 2, 5] {
        plugged[i] = true;
    }
    let mut fits = [CUT; STRIPS];
    fits[2] = SHRINK;
    let laid = layout(ON_ROWS, &[1; STRIPS], &fits, &plugged, &rows);
    assert_eq!(laid.laid, Laid::OnRows);
    assert_eq!(laid.rows, 3);
    assert_eq!(laid.places.len(), 2);
    assert_eq!(laid.places[0].row.as_deref(), Some("Bolts"));
    assert_eq!((laid.places[0].from, laid.places[0].to), (0.0, 0.125));
    assert_eq!(laid.places[1].strip, 2);
    assert_eq!(laid.places[1].row.as_deref(), Some("Seal"));
    assert!(laid.places[1].shrink && !laid.places[0].shrink);
    // Strip 6 has no sixth row.
    assert_eq!(laid.past_rows, [5]);
    // Heights of their own ignore the rows.
    let own = layout(OWN_HEIGHTS, &[1; STRIPS], &fits, &plugged, &rows);
    assert_eq!(own.laid, Laid::OwnHeights);
    assert!(own.places.iter().all(|p| p.row.is_none()));
    assert!(
        describe(&laid.places[0]).contains("Bolts"),
        "{}",
        describe(&laid.places[0])
    );
    assert!(
        describe(&own.places[1]).contains("33.3%"),
        "{}",
        describe(&own.places[1])
    );
}

#[test]
fn layout_of_reads_the_settings_and_their_standards() {
    let mut plugged = [false; STRIPS];
    plugged[0] = true;
    plugged[1] = true;
    let standard = layout_of(&Settings::new(), &plugged, &TrimRows::default());
    assert_eq!(standard.laid, Laid::NoRows);
    assert_eq!(standard.places[0].to, 0.5);
    let s = with(&[
        ("lay", Choice(OWN_HEIGHTS)),
        ("height2", Whole(3)),
        ("fit2", Choice(SHRINK)),
        // Out of range: kept within.
        ("height1", Whole(-5)),
    ]);
    let laid = layout_of(&s, &plugged, &TrimRows::default());
    assert_eq!(laid.laid, Laid::OwnHeights);
    assert_eq!(laid.places[0].to, 0.25);
    assert!(laid.places[1].shrink);
}

#[test]
fn each_strip_shows_its_material_and_the_gap_its_colour() {
    let n = 64;
    let s = bare(&[("gap", Number(10.0))]);
    let out = sheet(
        n,
        &[("strip1", plain(n, RED)), ("strip2", plain(n, BLUE))],
        &s,
        &TrimRows::default(),
    )
    .unwrap();
    assert_in_range("trim", &out);
    let m = material(&out);
    // 10 cm of 100 is 6.4 pixels: rows 0 to 2 and 61 to 63 are gap, as
    // are 29 to 34 round the middle.
    assert!(close(m.colour.at(5, 12), RED));
    assert!(close(m.colour.at(5, 50), BLUE));
    let gap = [0.10, 0.09, 0.08];
    for y in [0, 1, 31, 32, 62, 63] {
        assert!(
            close(m.colour.at(9, y), gap),
            "row {y}: {:?}",
            m.colour.at(9, y)
        );
    }
    let edges = out[1].grey().unwrap();
    assert_eq!(edges.at(0, 12), 0.0);
    assert_eq!(edges.at(0, 32), 1.0);
    // A row partly in the gap is partly gap: smooth, not a jagged step.
    let part = m.colour.at(0, 3)[0];
    assert!(part > 0.1 && part < 0.99, "{part}");
}

#[test]
fn a_material_between_strips_fills_the_gaps() {
    let n = 64;
    let s = bare(&[("gap", Number(10.0))]);
    let out = sheet(
        n,
        &[
            ("strip1", plain(n, RED)),
            ("strip2", plain(n, BLUE)),
            ("between", plain(n, GREEN)),
        ],
        &s,
        &TrimRows::default(),
    )
    .unwrap();
    assert!(close(material(&out).colour.at(3, 32), GREEN));
    assert!(close(material(&out).colour.at(3, 12), RED));
}

#[test]
fn cut_keeps_the_materials_size_from_its_top_row() {
    let n = 64;
    // Stripes four pixels tall, each row its own shade.
    let stripes = painted(n, |_, v| [v, 0.5, 1.0 - v]);
    let s = bare(&[("height2", Whole(3))]);
    let out = sheet(
        n,
        &[("strip1", plain(n, RED)), ("strip2", stripes.clone())],
        &s,
        &TrimRows::default(),
    )
    .unwrap();
    let m = material(&out);
    let source = stripes.material().unwrap();
    // Strip 2 starts a quarter down, at row 16: its row 16 + k is the
    // material's row k, pixel for pixel.
    for k in [0, 5, 20, 47] {
        assert_eq!(m.colour.at(7, 16 + k), source.colour.at(7, k), "row {k}");
    }
}

#[test]
fn shrink_fits_the_whole_material_into_the_strip() {
    let n = 128;
    // Red at the top of the material, blue at the bottom; dark at the
    // left, light at the right.
    let ramp = painted(n, |u, v| [1.0 - v, u, v]);
    let s = bare(&[("fit2", Choice(SHRINK)), ("height1", Whole(3))]);
    let out = sheet(
        n,
        &[("strip1", plain(n, GREEN)), ("strip2", ramp)],
        &s,
        &TrimRows::default(),
    )
    .unwrap();
    let m = material(&out);
    // Strip 2 is the bottom quarter (rows 96 to 127): the material's top
    // near its top, its bottom near its bottom.
    let top = m.colour.at(10, 98);
    let bottom = m.colour.at(10, 125);
    assert!(top[0] > 0.8 && top[2] < 0.2, "{top:?}");
    assert!(bottom[2] > 0.8 && bottom[0] < 0.2, "{bottom:?}");
    // Shrunk four times, it repeats four times along: the same at a
    // quarter of the way across.
    let a = m.colour.at(5, 110);
    let b = m.colour.at(5 + 32, 110);
    assert!(close(a, b), "{a:?} {b:?}");
    assert!(m.colour.at(20, 110)[1] > m.colour.at(5, 110)[1]);
}

#[test]
fn bevels_sink_the_edges_and_bend_the_normals() {
    let n = 128;
    let s = with(&[
        ("lay", Choice(OWN_HEIGHTS)),
        ("gap", Number(2.0)),
        ("edge_width", Number(10.0)),
        ("depth", Number(0.5)),
    ]);
    let out = sheet(
        n,
        &[("strip1", plain(n, RED)), ("strip2", plain(n, BLUE))],
        &s,
        &TrimRows::default(),
    )
    .unwrap();
    assert_in_range("bevel", &out);
    let m = material(&out);
    // The middle of strip 1 (row 32) is flat and full height; going down
    // to its bottom edge (row 64) the height falls.
    let h = |y: i64| m.height.at(0, y);
    assert!((h(32) - 0.5).abs() < 1e-4);
    assert!(
        h(52) < h(32) && h(60) < h(52) && h(63) < h(60),
        "{} {} {}",
        h(52),
        h(60),
        h(63)
    );
    // The gap is lowest.
    assert!((h(64) - 0.0).abs() < 1e-4, "{}", h(64));
    // Going into strip 1's bottom edge the surface falls away down the
    // texture, so it faces down it (y up the texture); strip 2's top edge
    // faces up it.
    assert!(m.normal.at(0, 58)[1] < -0.05, "{:?}", m.normal.at(0, 58));
    assert!(m.normal.at(0, 70)[1] > 0.05, "{:?}", m.normal.at(0, 70));
    // The gap is less open to light; the bevel's top bends out.
    assert!(m.occlusion.at(0, 64) < m.occlusion.at(0, 32));
    assert!(m.curvature.at(0, 64) < 0.5);
    let edges = out[1].grey().unwrap();
    assert!(edges.at(0, 32) < 1e-5 && edges.at(0, 58) > 0.05);
    // Chamfered sinks in a straight line: halfway is half.
    let chamfer = with(&[
        ("lay", Choice(OWN_HEIGHTS)),
        ("gap", Number(0.0)),
        ("edge_width", Number(20.0)),
        ("edges", Choice(CHAMFERED)),
    ]);
    let out = sheet(
        n,
        &[("strip1", plain(n, RED))],
        &chamfer,
        &TrimRows::default(),
    )
    .unwrap();
    let e = out[1].grey().unwrap();
    // 20 cm of 100 is 25.6 pixels: row 16's middle is 16.5 pixels in.
    assert!(
        (e.at(0, 16) - (1.0 - 16.5 / 25.6)).abs() < 1e-3,
        "{}",
        e.at(0, 16)
    );
    assert!(e.at(0, 64) < 1e-5);
}

#[test]
fn with_no_gap_or_depth_the_strip_is_its_material() {
    let n = 32;
    let m = input(ValueKind::Material, size(n));
    let out = sheet(
        n,
        &[("strip1", m.clone())],
        &bare(&[]),
        &TrimRows::default(),
    )
    .unwrap();
    let got = material(&out);
    let was = m.material().unwrap();
    assert_eq!(got.colour, was.colour);
    assert_eq!(got.height, was.height);
    assert_eq!(got.roughness, was.roughness);
    for (a, b) in got.normal.pixels().iter().zip(was.normal.pixels()) {
        assert!(close(*a, *b), "{a:?} {b:?}");
    }
}

#[test]
fn the_projects_rows_place_the_strips_and_leave_the_rest_gap() {
    let n = 64;
    let rows = TrimRows::new([band("Bolts", 0.0, 0.25), band("Seal", 0.5, 0.75)]);
    let s = with(&[("gap", Number(0.0)), ("depth", Number(0.0))]);
    let out = sheet(
        n,
        &[("strip1", plain(n, RED)), ("strip2", plain(n, BLUE))],
        &s,
        &rows,
    )
    .unwrap();
    let m = material(&out);
    assert!(close(m.colour.at(0, 8), RED));
    assert!(close(m.colour.at(0, 40), BLUE));
    // Between the rows and below the last: the gap.
    let gap = [0.10, 0.09, 0.08];
    assert!(close(m.colour.at(0, 24), gap));
    assert!(close(m.colour.at(0, 60), gap));
}

#[test]
fn a_sheet_with_nothing_to_lay_says_why() {
    let n = 32;
    let only_between = sheet(
        n,
        &[("between", plain(n, RED))],
        &Settings::new(),
        &TrimRows::default(),
    )
    .unwrap_err();
    assert!(
        only_between.reason.contains("Strip 1"),
        "{}",
        only_between.reason
    );
    let rows = TrimRows::new([band("Bolts", 0.0, 0.25), band("Seal", 0.5, 0.75)]);
    let past = sheet(n, &[("strip3", plain(n, RED))], &Settings::new(), &rows).unwrap_err();
    assert!(past.reason.contains("Strip 3"), "{}", past.reason);
    assert!(
        past.reason.contains("Heights of their own"),
        "{}",
        past.reason
    );
    // The same strip with its own heights is fine.
    let own = with(&[("lay", Choice(OWN_HEIGHTS))]);
    assert!(sheet(n, &[("strip3", plain(n, RED))], &own, &rows).is_ok());
}

#[test]
fn see_through_strips_keep_their_opacity() {
    let n = 32;
    let mut leafy = (**input(ValueKind::Material, size(n)).material().unwrap()).clone();
    leafy.opacity = Some(Arc::new(Image::filled(size(n), 0.25)));
    let out = sheet(
        n,
        &[
            ("strip1", Value::Material(Arc::new(leafy))),
            ("strip2", plain(n, BLUE)),
        ],
        &bare(&[]),
        &TrimRows::default(),
    )
    .unwrap();
    let opacity = material(&out).opacity.clone().unwrap();
    assert!((opacity.at(0, 4) - 0.25).abs() < 1e-5);
    assert!((opacity.at(0, 28) - 1.0).abs() < 1e-5);
    assert!(material(&out).translucency.is_none());
}

#[test]
fn trim_sheets_tile_seamlessly_with_other_settings() {
    let n = 64;
    let a = input(ValueKind::Material, size(n));
    let b = plain(n, BLUE);
    let rows = TrimRows::new([band("A", 0.1, 0.3), band("B", 0.3, 0.9)]);
    let cases = [
        // The seam check compares the jump across the edge with the
        // biggest inside. The sheet's top and bottom meet in the middle of
        // a gap, exactly on a pixel's edge, where a narrow groove's two
        // sides face apart; a gap a few pixels wide has a flat floor there.
        (
            with(&[
                ("lay", Choice(OWN_HEIGHTS)),
                ("fit1", Choice(SHRINK)),
                ("gap", Number(5.0)),
            ]),
            TrimRows::default(),
        ),
        (
            with(&[
                ("lay", Choice(OWN_HEIGHTS)),
                ("edges", Choice(CHAMFERED)),
                ("height2", Whole(5)),
                ("gap", Number(5.0)),
            ]),
            TrimRows::default(),
        ),
        (
            with(&[("fit2", Choice(SHRINK)), ("depth", Number(1.0))]),
            rows,
        ),
    ];
    for (i, (s, rows)) in cases.into_iter().enumerate() {
        let out = sheet(
            n,
            &[("strip1", a.clone()), ("strip2", b.clone())],
            &s,
            &rows,
        )
        .unwrap();
        let what = format!("trim case {i}");
        assert_in_range(&what, &out);
        assert_seamless(&what, &out);
    }
}

/// A graph: a material from noise into a Trim sheet's Strip 1, the sheet
/// into Material out (through a Dirt box when `dirt`).
fn trim_graph(dirt: bool) -> (Graph, crate::BoxId, crate::BoxId) {
    let r = registry();
    let mut g = Graph::new("Trim");
    let add = |g: &mut Graph, kind: &str| {
        let id = crate::BoxId(g.next_id());
        g.apply(
            r,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        id
    };
    let noise = add(&mut g, "noise");
    let mfh = add(&mut g, "material_from_height");
    let trim = add(&mut g, TRIM_SHEET);
    let out = add(&mut g, "material_out");
    let wire = |g: &mut Graph, from: (crate::BoxId, &str), to: (crate::BoxId, &str)| {
        g.apply(
            r,
            &GraphEdit::Connect {
                from: PlugRef::new(from.0, from.1),
                to: PlugRef::new(to.0, to.1),
            },
        )
        .unwrap();
    };
    wire(&mut g, (noise, "height"), (mfh, "height"));
    wire(&mut g, (mfh, "material"), (trim, "strip1"));
    if dirt {
        let d = add(&mut g, "dirt");
        wire(&mut g, (trim, "material_out"), (d, "material"));
        wire(&mut g, (d, "material_out"), (out, "material"));
    } else {
        wire(&mut g, (trim, "material_out"), (out, "material"));
    }
    (g, trim, out)
}

#[test]
fn a_graph_is_a_trim_sheet_when_one_feeds_material_out() {
    let r = registry();
    let (g, trim, _) = trim_graph(false);
    assert!(is_trim_sheet(&g));
    assert!(crate::trim::uses_trim_rows(r, &g));
    assert!(is_trim_sheet(&trim_graph(true).0));
    let mut off = g.clone();
    off.apply(
        r,
        &GraphEdit::Bypass {
            node: trim,
            on: true,
        },
    )
    .unwrap();
    assert!(!is_trim_sheet(&off));
    let (mut loose, _, out) = trim_graph(false);
    loose
        .apply(r, &GraphEdit::Disconnect(PlugRef::new(out, "material")))
        .unwrap();
    assert!(!is_trim_sheet(&loose));
    assert!(!is_trim_sheet(&Graph::new("Empty")));
    assert!(!crate::trim::uses_trim_rows(r, &Graph::new("Empty")));
}

#[test]
fn changing_the_projects_rows_remakes_the_sheet_only() {
    let (g, trim, out) = trim_graph(false);
    let mut store = GraphStore::new();
    let mut evaluator = Evaluator::with_threads(2);
    let run = |e: &mut Evaluator, store: &GraphStore| e.run(store.registry(), &g, size(32), &[out]);
    let first = run(&mut evaluator, &store);
    let before = first.picture(trim).unwrap().clone();
    store.set_trim_rows(TrimRows::new([band("Top", 0.0, 0.5)]));
    let second = run(&mut evaluator, &store);
    // The noise and its material are reused; the sheet and Material out
    // are made again, and look different.
    assert_eq!(second.made(), 2, "made {}", second.made());
    assert_ne!(second.picture(trim).unwrap(), &before);
    // The same rows again: nothing new.
    store.set_trim_rows(TrimRows::new([band("Top", 0.0, 0.5)]));
    assert_eq!(run(&mut evaluator, &store).made(), 0);
}

#[test]
#[ignore = "a timing, not a check: run with --release --ignored --nocapture"]
fn trim_sheet_speed() {
    let n = 2048;
    let a = input(ValueKind::Material, size(n));
    let b = plain(n, BLUE);
    for (name, s) in [
        ("cut", with(&[("lay", Choice(OWN_HEIGHTS))])),
        (
            "shrink",
            with(&[
                ("lay", Choice(OWN_HEIGHTS)),
                ("fit1", Choice(SHRINK)),
                ("fit2", Choice(SHRINK)),
            ]),
        ),
    ] {
        let inputs: Vec<(&str, Value)> = STRIP_KEYS
            .iter()
            .enumerate()
            .map(|(i, k)| (*k, if i % 2 == 0 { a.clone() } else { b.clone() }))
            .collect();
        let start = std::time::Instant::now();
        let _ = sheet(n, &inputs, &s, &TrimRows::default()).unwrap();
        println!("trim sheet, eight strips, {name}: {:?}", start.elapsed());
    }
}
