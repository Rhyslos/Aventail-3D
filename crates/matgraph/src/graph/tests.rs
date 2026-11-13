//! Tests for graphs: edits, wires and why they're refused, loops, frames,
//! and rebuilding from parts (with damaged parts).

use super::*;
use crate::kind::Fit;
use crate::refusal::{Fix, Refusal};
use crate::settings::SettingValue;
use glam::Vec2;

fn registry() -> &'static Registry {
    Registry::standard()
}

/// Adds a box of `kind` and gives its id.
fn add(graph: &mut Graph, kind: &str) -> BoxId {
    let id = BoxId(graph.next_id());
    let at = Vec2::new(id.0 as f32 * 200.0, 0.0);
    graph
        .apply(
            registry(),
            &GraphEdit::AddBox {
                kind: kind.into(),
                at,
            },
        )
        .unwrap();
    id
}

fn plug(id: BoxId, key: &str) -> PlugRef {
    PlugRef::new(id, key)
}

fn connect(graph: &mut Graph, from: PlugRef, to: PlugRef) -> Result<(), Refusal> {
    graph.apply(registry(), &GraphEdit::Connect { from, to })
}

#[test]
fn boxes_get_ids_in_order_that_are_never_reused() {
    let mut g = Graph::new("Stone");
    let a = add(&mut g, "noise");
    let b = add(&mut g, "levels");
    assert_eq!((a, b), (BoxId(1), BoxId(2)));
    g.apply(registry(), &GraphEdit::RemoveBoxes(vec![b]))
        .unwrap();
    assert_eq!(add(&mut g, "levels"), BoxId(3));
    // Frames share the counter.
    g.apply(
        registry(),
        &GraphEdit::AddFrame(Frame::new("Bricks", Vec2::ZERO, Vec2::splat(500.0))),
    )
    .unwrap();
    assert!(g.frame(FrameId(4)).is_some());
    assert_eq!(add(&mut g, "noise"), BoxId(5));
}

#[test]
fn unknown_kinds_and_a_second_material_out_are_refused_with_reasons() {
    let mut g = Graph::new("Stone");
    let unknown = g.apply(
        registry(),
        &GraphEdit::AddBox {
            kind: "teapot".into(),
            at: Vec2::ZERO,
        },
    );
    assert!(unknown.unwrap_err().reason.contains("teapot"));
    let out = add(&mut g, "material_out");
    let again = g
        .apply(
            registry(),
            &GraphEdit::AddBox {
                kind: "material_out".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap_err();
    assert!(again.reason.contains("one material"), "{}", again.reason);
    assert_eq!(again.fixes, vec![Fix::LookAt(out)]);
    let far = g.apply(
        registry(),
        &GraphEdit::AddBox {
            kind: "noise".into(),
            at: Vec2::new(f32::NAN, 0.0),
        },
    );
    assert!(far.is_err());
    assert_eq!(g.box_count(), 1);
}

#[test]
fn wires_that_fit_connect_and_replace_what_was_there() {
    let mut g = Graph::new("Stone");
    let (n1, n2, levels) = (
        add(&mut g, "noise"),
        add(&mut g, "noise"),
        add(&mut g, "levels"),
    );
    connect(&mut g, plug(n1, "height"), plug(levels, "in")).unwrap();
    connect(&mut g, plug(n2, "height"), plug(levels, "in")).unwrap();
    assert_eq!(g.wires().len(), 1);
    assert_eq!(g.wire_into(&plug(levels, "in")).unwrap().from.node, n2);
    // A shape into a height plug converts, with a note.
    let tiles = add(&mut g, "tiles");
    let how = g
        .check_connection(registry(), &plug(tiles, "shape"), &plug(levels, "in"))
        .unwrap();
    assert!(matches!(how, Fit::Converts(note) if note.contains("height")));
    g.apply(registry(), &GraphEdit::Disconnect(plug(levels, "in")))
        .unwrap();
    assert!(g.wires().is_empty());
    assert!(
        g.apply(registry(), &GraphEdit::Disconnect(plug(levels, "in")))
            .is_err()
    );
}

#[test]
fn a_colour_into_a_shape_plug_says_why_and_offers_fixes() {
    let mut g = Graph::new("Stone");
    let fill = add(&mut g, "colour_fill");
    let tiles = add(&mut g, "tiles");
    let blend = add(&mut g, "blend");
    let refusal = connect(&mut g, plug(fill, "colour"), plug(blend, "where")).unwrap_err();
    assert!(
        refusal.reason.contains("inside or outside"),
        "{}",
        refusal.reason
    );
    assert!(refusal.reason.contains("Blend"), "{}", refusal.reason);
    // Tiles makes a shape: connect it instead.
    assert!(
        refusal.fixes.contains(&Fix::ConnectInstead {
            from: plug(tiles, "shape")
        }),
        "{:?}",
        refusal.fixes
    );
    for fix in &refusal.fixes {
        assert!(!fix.describe(registry(), &g).is_empty());
    }
    assert!(g.wires().is_empty());
}

#[test]
fn a_height_into_material_out_offers_material_from_height_between() {
    let mut g = Graph::new("Stone");
    let noise = add(&mut g, "noise");
    let out = add(&mut g, "material_out");
    let refusal = connect(&mut g, plug(noise, "height"), plug(out, "material")).unwrap_err();
    assert!(refusal.fixes.contains(&Fix::InsertBox {
        kind: "material_from_height".into()
    }));
    let text = Fix::InsertBox {
        kind: "material_from_height".into(),
    }
    .describe(registry(), &g);
    assert_eq!(text, "Put a Material from height box between them");
}

#[test]
fn loops_and_a_box_feeding_itself_are_refused() {
    let mut g = Graph::new("Stone");
    let (a, b, c) = (
        add(&mut g, "levels"),
        add(&mut g, "levels"),
        add(&mut g, "levels"),
    );
    connect(&mut g, plug(a, "out"), plug(b, "in")).unwrap();
    connect(&mut g, plug(b, "out"), plug(c, "in")).unwrap();
    let refusal = connect(&mut g, plug(c, "out"), plug(a, "in")).unwrap_err();
    assert!(refusal.reason.contains("loop"), "{}", refusal.reason);
    assert!(connect(&mut g, plug(a, "out"), plug(a, "in")).is_err());
    assert!(!g.has_loop());
    assert_eq!(g.order(&[c]), vec![a, b, c]);
}

#[test]
fn missing_plugs_and_boxes_are_refused() {
    let mut g = Graph::new("Stone");
    let (n, l) = (add(&mut g, "noise"), add(&mut g, "levels"));
    assert!(connect(&mut g, plug(n, "nope"), plug(l, "in")).is_err());
    assert!(connect(&mut g, plug(n, "height"), plug(l, "nope")).is_err());
    assert!(connect(&mut g, plug(BoxId(99), "height"), plug(l, "in")).is_err());
    // An output can't be plugged into as an input.
    assert!(connect(&mut g, plug(l, "out"), plug(n, "height")).is_err());
}

#[test]
fn removing_a_box_takes_its_wires_and_dissolving_keeps_the_wire_through() {
    let mut g = Graph::new("Stone");
    let (n, l, m) = (
        add(&mut g, "noise"),
        add(&mut g, "levels"),
        add(&mut g, "material_from_height"),
    );
    connect(&mut g, plug(n, "height"), plug(l, "in")).unwrap();
    connect(&mut g, plug(l, "out"), plug(m, "height")).unwrap();
    let mut removed = g.clone();
    removed
        .apply(registry(), &GraphEdit::RemoveBoxes(vec![l]))
        .unwrap();
    assert!(removed.wires().is_empty());
    g.apply(registry(), &GraphEdit::DissolveBox(l)).unwrap();
    assert_eq!(g.wires().len(), 1);
    assert_eq!(
        g.wire_into(&plug(m, "height")).unwrap().from,
        plug(n, "height")
    );
    assert!(g.apply(registry(), &GraphEdit::DissolveBox(l)).is_err());
}

#[test]
fn settings_are_checked_against_the_box() {
    let mut g = Graph::new("Stone");
    let n = add(&mut g, "noise");
    let set = |g: &mut Graph, key: &str, value| {
        g.apply(
            registry(),
            &GraphEdit::SetSetting {
                node: n,
                key: key.into(),
                value,
            },
        )
    };
    set(&mut g, "scale", SettingValue::Whole(8)).unwrap();
    assert_eq!(
        g.get(n).unwrap().settings.get("scale"),
        Some(SettingValue::Whole(8))
    );
    let refusal = set(&mut g, "scale", SettingValue::Whole(1000)).unwrap_err();
    assert_eq!(refusal.fixes, vec![Fix::UseNearest]);
    assert!(set(&mut g, "nope", SettingValue::Whole(1)).is_err());
    assert!(set(&mut g, "roughness", SettingValue::Number(f32::NAN)).is_err());
    assert!(set(&mut g, "seed", SettingValue::Whole(-77)).is_ok());
}

#[test]
fn frames_hold_the_boxes_inside_them_and_nest() {
    let mut g = Graph::new("Stone");
    let inside = add(&mut g, "noise"); // at (200, 0)
    let outside = add(&mut g, "noise"); // at (400, 0)
    let big = Frame::new("Bricks", Vec2::new(-10.0, -10.0), Vec2::new(500.0, 100.0));
    let small = Frame::new("Gaps", Vec2::new(100.0, -5.0), Vec2::new(250.0, 50.0));
    g.apply(registry(), &GraphEdit::AddFrame(big)).unwrap();
    g.apply(registry(), &GraphEdit::AddFrame(small)).unwrap();
    let (big, small) = (FrameId(3), FrameId(4));
    assert_eq!(g.boxes_in(big), vec![inside, outside]);
    assert_eq!(g.boxes_in(small), vec![inside]);
    let mut renamed = g.frame(small).unwrap().clone();
    renamed.title = "Grout".into();
    renamed.note = "The lines between bricks.".into();
    renamed.colour = [200, 120, 40];
    g.apply(
        registry(),
        &GraphEdit::ChangeFrame {
            id: small,
            frame: renamed.clone(),
        },
    )
    .unwrap();
    assert_eq!(g.frame(small), Some(&renamed));
    let backwards = Frame::new("Bad", Vec2::splat(10.0), Vec2::ZERO);
    assert!(
        g.apply(registry(), &GraphEdit::AddFrame(backwards))
            .is_err()
    );
    let mut long = renamed;
    long.note = "x".repeat(MAX_NOTE + 1);
    assert!(
        g.apply(
            registry(),
            &GraphEdit::ChangeFrame {
                id: small,
                frame: long
            }
        )
        .is_err()
    );
    g.apply(registry(), &GraphEdit::RemoveFrame(big)).unwrap();
    assert!(g.apply(registry(), &GraphEdit::RemoveFrame(big)).is_err());
    assert_eq!(g.box_count(), 2);
}

#[test]
fn renames_seeds_and_moves() {
    let mut g = Graph::new("Stone");
    let n = add(&mut g, "noise");
    g.apply(registry(), &GraphEdit::Rename("Cobbles".into()))
        .unwrap();
    assert!(
        g.apply(registry(), &GraphEdit::Rename("  ".into()))
            .is_err()
    );
    g.apply(registry(), &GraphEdit::SetSeed(42)).unwrap();
    g.apply(
        registry(),
        &GraphEdit::MoveBoxes {
            ids: vec![n],
            by: Vec2::new(5.0, 7.0),
        },
    )
    .unwrap();
    assert_eq!((g.name(), g.seed()), ("Cobbles", 42));
    assert_eq!(g.get(n).unwrap().at, Vec2::new(205.0, 7.0));
    assert!(
        g.apply(
            registry(),
            &GraphEdit::MoveBoxes {
                ids: vec![n],
                by: Vec2::splat(f32::INFINITY),
            },
        )
        .is_err()
    );
    for edit in [GraphEdit::SetSeed(1), GraphEdit::RemoveBoxes(vec![n, n])] {
        assert!(!edit.describe().is_empty());
    }
}

/// A graph with every part: boxes, wires, settings, frames.
pub(crate) fn rich() -> Graph {
    let mut g = Graph::new("Stone tiles");
    let tiles = add(&mut g, "tiles");
    let noise = add(&mut g, "noise");
    let blend = add(&mut g, "blend");
    let mat = add(&mut g, "material_from_height");
    let out = add(&mut g, "material_out");
    connect(&mut g, plug(tiles, "shape"), plug(blend, "bottom")).unwrap();
    connect(&mut g, plug(noise, "height"), plug(blend, "top")).unwrap();
    connect(&mut g, plug(tiles, "shape"), plug(blend, "where")).unwrap();
    connect(&mut g, plug(blend, "out"), plug(mat, "height")).unwrap();
    connect(&mut g, plug(mat, "material"), plug(out, "material")).unwrap();
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node: blend,
            key: "amount".into(),
            value: SettingValue::Number(0.3),
        },
    )
    .unwrap();
    g.apply(
        registry(),
        &GraphEdit::AddFrame(Frame::new("Shapes", Vec2::ZERO, Vec2::splat(700.0))),
    )
    .unwrap();
    g.apply(registry(), &GraphEdit::SetSeed(9)).unwrap();
    g
}

#[test]
fn a_graph_survives_its_parts() {
    let g = rich();
    let back = Graph::from_parts(g.to_parts(), registry()).unwrap();
    assert_eq!(back, g);
}

#[test]
fn damaged_parts_are_refused() {
    let good = rich().to_parts();
    let mut cases: Vec<GraphParts> = Vec::new();
    let mut p = good.clone();
    p.name = String::new();
    cases.push(p);
    let mut p = good.clone();
    p.boxes[1].0 = p.boxes[0].0;
    cases.push(p);
    let mut p = good.clone();
    p.next_id = 2;
    cases.push(p);
    let mut p = good.clone();
    p.boxes[0].1.at = Vec2::NAN;
    cases.push(p);
    let mut p = good.clone();
    p.wires[0].to.node = BoxId(77);
    cases.push(p);
    let mut p = good.clone();
    p.wires[0].from.plug = "nope".into();
    cases.push(p);
    let mut p = good.clone();
    let doubled = p.wires[0].clone();
    p.wires.push(doubled);
    cases.push(p);
    let mut p = good.clone();
    p.frames[0].1.max = Vec2::splat(-1e9);
    cases.push(p);
    let mut p = good.clone();
    p.frames[0].0 = FrameId(p.boxes[0].0.0);
    cases.push(p);
    // A colour into a shape plug.
    let mut p = good.clone();
    p.boxes.push((
        BoxId(50),
        GraphBox {
            kind: "colour_fill".into(),
            at: Vec2::ZERO,
            settings: Settings::new(),
            shown: Default::default(),
            in_game: Default::default(),
            drawing: None,
            painting: None,
            photo: None,
            bypassed: false,
        },
    ));
    p.next_id = 51;
    p.wires.retain(|w| w.to.plug != "where");
    let blend = p.boxes.iter().find(|(_, b)| b.kind == "blend").unwrap().0;
    p.wires.push(Wire {
        from: PlugRef::new(BoxId(50), "colour"),
        to: PlugRef::new(blend, "where"),
    });
    cases.push(p);
    for (i, parts) in cases.into_iter().enumerate() {
        assert!(Graph::from_parts(parts, registry()).is_err(), "case {i}");
    }
}

#[test]
fn a_loop_in_a_file_is_refused() {
    let mut p = Graph::new("Loop").to_parts();
    p.next_id = 3;
    for id in [1, 2] {
        p.boxes.push((
            BoxId(id),
            GraphBox {
                kind: "levels".into(),
                at: Vec2::ZERO,
                settings: Settings::new(),
                shown: Default::default(),
                in_game: Default::default(),
                drawing: None,
                painting: None,
                photo: None,
                bypassed: false,
            },
        ));
    }
    p.wires.push(Wire {
        from: PlugRef::new(BoxId(1), "out"),
        to: PlugRef::new(BoxId(2), "in"),
    });
    p.wires.push(Wire {
        from: PlugRef::new(BoxId(2), "out"),
        to: PlugRef::new(BoxId(1), "in"),
    });
    let why = Graph::from_parts(p, registry()).unwrap_err();
    assert!(why.contains("loop"), "{why}");
}

#[test]
fn settings_from_files_are_kept_in_range_and_unknown_kinds_are_kept() {
    let mut p = rich().to_parts();
    let blend = p.boxes.iter_mut().find(|(_, b)| b.kind == "blend").unwrap();
    blend.1.settings.set("amount", SettingValue::Number(50.0));
    blend.1.settings.set("gone", SettingValue::Whole(3));
    p.boxes.push((
        BoxId(p.next_id),
        GraphBox {
            kind: "from_the_future".into(),
            at: Vec2::ZERO,
            settings: Settings::new(),
            shown: Default::default(),
            in_game: Default::default(),
            drawing: None,
            painting: None,
            photo: None,
            bypassed: false,
        },
    ));
    p.next_id += 1;
    let g = Graph::from_parts(p, registry()).unwrap();
    let (_, blend) = g.boxes().find(|(_, b)| b.kind == "blend").unwrap();
    assert_eq!(
        blend.settings.get("amount"),
        Some(SettingValue::Number(1.0))
    );
    assert_eq!(blend.settings.get("gone"), None);
    assert!(g.first_of("from_the_future").is_some());
}

/// A small, repeatable random number source.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

#[test]
fn random_edits_never_crash_or_make_a_loop() {
    // Fuzz: a long run of random edits, many nonsensical. Each is either
    // made or refused with a reason; the graph always stays sound.
    let kinds = [
        "noise",
        "tiles",
        "levels",
        "blend",
        "colour_fill",
        "material_from_height",
    ];
    let plugs = [
        "height", "shape", "in", "out", "bottom", "top", "where", "colour", "material",
    ];
    let mut rng = Lcg(5);
    let mut g = Graph::new("Fuzz");
    for _ in 0..3000 {
        let pick = |rng: &mut Lcg, n: usize| (rng.next() as usize) % n;
        let id = BoxId(rng.next() % (g.next_id() + 2));
        let edit = match pick(&mut rng, 7) {
            0 | 1 => GraphEdit::AddBox {
                kind: kinds[pick(&mut rng, kinds.len())].into(),
                at: Vec2::new(rng.next() as f32, 0.0),
            },
            2 | 3 => GraphEdit::Connect {
                from: PlugRef::new(id, plugs[pick(&mut rng, plugs.len())]),
                to: PlugRef::new(
                    BoxId(rng.next() % (g.next_id() + 2)),
                    plugs[pick(&mut rng, plugs.len())],
                ),
            },
            4 => GraphEdit::DissolveBox(id),
            5 => GraphEdit::RemoveBoxes(vec![id]),
            _ => GraphEdit::SetSetting {
                node: id,
                key: ["scale", "amount", "low", "mode"][pick(&mut rng, 4)].into(),
                value: SettingValue::Number(rng.next() as f32 - 1e9),
            },
        };
        match g.apply(registry(), &edit) {
            Ok(()) => {}
            Err(refusal) => assert!(!refusal.reason.is_empty()),
        }
        assert!(!g.has_loop());
    }
    // Whatever it became, it rebuilds from its parts.
    assert_eq!(Graph::from_parts(g.to_parts(), registry()).unwrap(), g);
}

#[test]
fn settings_can_be_shown_in_the_painter_and_read_back_in_order() {
    let mut g = Graph::new("Shown");
    let noise = add(&mut g, "noise");
    let out = add(&mut g, "material_out");
    let show = |g: &mut Graph, node, key: &str, shown| {
        g.apply(
            registry(),
            &GraphEdit::ShowInPainter {
                node,
                key: key.into(),
                shown,
            },
        )
    };
    assert!(g.shown(registry()).is_empty());
    show(&mut g, out, "size_cm", true).unwrap();
    show(&mut g, noise, "seed", true).unwrap();
    // A setting a box doesn't have is refused, with why.
    let why = show(&mut g, noise, "nonsense", true).unwrap_err();
    assert!(why.reason.contains("nonsense"));
    let shown = g.shown(registry());
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].node, noise);
    assert_eq!(shown[1].spec.key, "size_cm");
    assert_eq!(shown[1].value, SettingValue::Number(100.0));
    assert_eq!(
        GraphEdit::ShowInPainter {
            node: noise,
            key: "seed".into(),
            shown: false
        }
        .describe(),
        "Hide a setting from the painter"
    );
    show(&mut g, noise, "seed", false).unwrap();
    assert_eq!(g.shown(registry()).len(), 1);
    // Removing the box takes its shown settings with it.
    g.apply(registry(), &GraphEdit::RemoveBoxes(vec![out]))
        .unwrap();
    assert!(g.shown(registry()).is_empty());
}

#[test]
fn a_layers_own_values_change_a_copy_and_wrong_ones_are_left_out() {
    let mut g = Graph::new("Values");
    let out = add(&mut g, "material_out");
    let changed = g.with_settings(
        registry(),
        [
            (out, "size_cm", SettingValue::Number(250.0)),
            // No such box, no such setting, the wrong kind: left out.
            (BoxId(99), "size_cm", SettingValue::Number(3.0)),
            (out, "nonsense", SettingValue::Number(3.0)),
            (out, "size_cm", SettingValue::Switch(true)),
        ],
    );
    assert_eq!(
        changed.get(out).unwrap().settings.get("size_cm"),
        Some(SettingValue::Number(250.0))
    );
    // The graph itself is unchanged.
    assert_eq!(g.get(out).unwrap().settings.get("size_cm"), None);
}

#[test]
fn shown_settings_are_kept_through_parts_and_damaged_ones_dropped() {
    let mut g = Graph::new("Parts");
    let out = add(&mut g, "material_out");
    g.apply(
        registry(),
        &GraphEdit::ShowInPainter {
            node: out,
            key: "size_cm".into(),
            shown: true,
        },
    )
    .unwrap();
    let parts = g.to_parts();
    assert_eq!(Graph::from_parts(parts.clone(), registry()).unwrap(), g);
    // A key the box doesn't have is dropped on opening.
    let mut odd = parts.clone();
    odd.boxes[0].1.shown.insert("gone".into());
    let opened = Graph::from_parts(odd, registry()).unwrap();
    assert_eq!(opened, g);
    // Far too many, or too long, is damage.
    let mut long = parts;
    long.boxes[0].1.shown.insert("x".repeat(500));
    assert!(Graph::from_parts(long, registry()).is_err());
}

fn in_game(graph: &mut Graph, node: BoxId, key: &str, on: bool) -> Result<(), Refusal> {
    graph.apply(
        registry(),
        &GraphEdit::ChangeInGame {
            node,
            key: key.into(),
            on,
        },
    )
}

#[test]
fn sliders_can_be_marked_to_change_in_the_game() {
    let mut g = Graph::new("Lava");
    let scroll = add(&mut g, "scroll");
    let pulse = add(&mut g, "pulse");
    in_game(&mut g, scroll, "speed", true).unwrap();
    in_game(&mut g, pulse, "high", true).unwrap();
    in_game(&mut g, scroll, "loops", true).unwrap();
    let listed: Vec<(BoxId, &str)> = g
        .in_game(registry())
        .iter()
        .map(|s| (s.node, s.spec.key.as_ref()))
        .collect();
    assert_eq!(
        listed,
        [(scroll, "speed"), (scroll, "loops"), (pulse, "high")]
    );
    in_game(&mut g, scroll, "loops", false).unwrap();
    assert_eq!(g.get(scroll).unwrap().in_game.len(), 1);
    // A choice picks how the box works: always baked, and says why.
    let why = in_game(&mut g, pulse, "shape", true).unwrap_err();
    assert!(why.reason.contains("always baked"), "{}", why.reason);
    // Unmarking one is always allowed; a setting it hasn't is refused.
    in_game(&mut g, pulse, "shape", false).unwrap();
    assert!(in_game(&mut g, pulse, "sparkle", true).is_err());
    assert!(in_game(&mut g, BoxId(99), "speed", true).is_err());
    // Copies keep their marks.
    let copy = g.get(scroll).unwrap().copy_edits(BoxId(7));
    assert!(copy.contains(&GraphEdit::ChangeInGame {
        node: BoxId(7),
        key: "speed".into(),
        on: true
    }));
}

#[test]
fn the_moment_to_bake_is_kept_in_range() {
    let mut g = Graph::new("Lava");
    assert_eq!(g.moment(), 0.0);
    g.apply(registry(), &GraphEdit::SetMoment(2.5)).unwrap();
    assert_eq!(g.moment(), 2.5);
    for wrong in [-1.0, f32::NAN, MAX_MOMENT + 1.0] {
        assert!(g.apply(registry(), &GraphEdit::SetMoment(wrong)).is_err());
    }
    assert_eq!(g.moment(), 2.5);
    assert_eq!(usable_moment(f32::NEG_INFINITY), 0.0);
    assert_eq!(usable_moment(1.0e9), MAX_MOMENT);
}

#[test]
fn marks_and_the_moment_survive_parts_and_damaged_ones_are_dropped() {
    let mut g = Graph::new("Lava");
    let scroll = add(&mut g, "scroll");
    in_game(&mut g, scroll, "speed", true).unwrap();
    g.apply(registry(), &GraphEdit::SetMoment(3.0)).unwrap();
    let back = Graph::from_parts(g.to_parts(), registry()).unwrap();
    assert_eq!(back, g);
    let mut parts = g.to_parts();
    parts.moment = f32::NAN;
    let b = &mut parts.boxes[0].1;
    b.in_game.insert("sparkle".into());
    b.in_game.insert("angle".into());
    // A choice can't be marked, even from a file.
    let mut pulse_parts = parts.clone();
    let back = Graph::from_parts(parts, registry()).unwrap();
    assert_eq!(back.moment(), 0.0);
    let kept: Vec<&String> = back.get(scroll).unwrap().in_game.iter().collect();
    assert_eq!(kept, ["angle", "speed"]);
    pulse_parts.boxes[0].1.kind = "pulse".into();
    pulse_parts.boxes[0].1.in_game = ["shape".to_string(), "rate".to_string()].into();
    let back = Graph::from_parts(pulse_parts, registry()).unwrap();
    let kept: Vec<&String> = back.get(scroll).unwrap().in_game.iter().collect();
    assert_eq!(kept, ["rate"]);
    // Too many marks is a damaged file.
    let mut parts = g.to_parts();
    parts.boxes[0].1.in_game = (0..100).map(|i| format!("k{i}")).collect();
    assert!(Graph::from_parts(parts, registry()).is_err());
}
