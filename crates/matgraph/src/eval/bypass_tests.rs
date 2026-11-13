//! Tests for bypassed boxes (milestone 16): each output passes on the
//! right input for its kind, outputs with nothing to pass say why in the
//! boxes after them, a bypassed moving box doesn't fill the cache while
//! the preview plays, switching bypass off finds the old results kept,
//! and bypass works inside your own boxes.

use super::*;
use crate::ValueKind;
use crate::graph::GraphEdit;
use glam::Vec2;

fn registry() -> &'static Registry {
    Registry::standard()
}

fn add(g: &mut Graph, kind: &str) -> BoxId {
    let id = BoxId(g.next_id());
    g.apply(
        registry(),
        &GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(id.0 as f32 * 220.0, 0.0),
        },
    )
    .unwrap();
    id
}

fn wire(g: &mut Graph, from: (BoxId, &str), to: (BoxId, &str)) {
    g.apply(
        registry(),
        &GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        },
    )
    .unwrap();
}

fn bypass(g: &mut Graph, node: BoxId, on: bool) {
    g.apply(registry(), &GraphEdit::Bypass { node, on })
        .unwrap();
}

fn size() -> Size {
    Size::new(32).unwrap()
}

/// Output `plug` of box `id` in `run`.
fn output(g: &Graph, run: &Evaluation, id: BoxId, plug: &str) -> Option<Value> {
    let info = g.info_of(registry(), id).unwrap();
    let at = info.outputs.iter().position(|o| o.key == plug).unwrap();
    let result = run.outcome(id).unwrap().as_ref().unwrap();
    result.outputs[at].clone()
}

#[test]
fn a_bypassed_box_passes_its_input_straight_through() {
    let mut g = Graph::new("Through");
    let noise = add(&mut g, "noise");
    let blur = add(&mut g, "blur");
    let mfh = add(&mut g, "material_from_height");
    wire(&mut g, (noise, "height"), (blur, "in"));
    wire(&mut g, (blur, "out"), (mfh, "height"));
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node: blur,
            key: "radius".into(),
            value: crate::SettingValue::Number(12.0),
        },
    )
    .unwrap();
    let mut e = Evaluator::with_threads(2);
    let on = e.run(registry(), &g, size(), &[]);
    bypass(&mut g, blur, true);
    let off = e.run(registry(), &g, size(), &[]);
    assert_eq!(
        output(&g, &off, blur, "out"),
        output(&g, &off, noise, "height"),
        "Blur gives Noise's height as it is"
    );
    assert_eq!(off.picture(blur), off.picture(noise));
    assert!(off.problems().next().is_none());
    // What comes after sees the difference.
    assert_ne!(
        output(&g, &off, mfh, "material"),
        output(&g, &on, mfh, "material")
    );
}

#[test]
fn each_output_takes_the_input_of_its_own_kind() {
    let mut g = Graph::new("Kinds");
    let noise = add(&mut g, "noise");
    let fill = add(&mut g, "colour_fill");
    let mfh = add(&mut g, "material_from_height");
    let transform = add(&mut g, "transform");
    wire(&mut g, (noise, "height"), (mfh, "height"));
    wire(&mut g, (noise, "height"), (transform, "in"));
    wire(&mut g, (fill, "colour"), (transform, "colour"));
    wire(&mut g, (mfh, "material"), (transform, "material"));
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node: transform,
            key: "across".into(),
            value: crate::SettingValue::Number(0.25),
        },
    )
    .unwrap();
    bypass(&mut g, transform, true);
    let run = Evaluator::with_threads(1).run(registry(), &g, size(), &[]);
    assert_eq!(
        output(&g, &run, transform, "out"),
        output(&g, &run, noise, "height")
    );
    assert_eq!(
        output(&g, &run, transform, "colour_out"),
        output(&g, &run, fill, "colour")
    );
    assert_eq!(
        output(&g, &run, transform, "material_out"),
        output(&g, &run, mfh, "material")
    );
}

#[test]
fn a_shape_passes_on_as_a_height_when_no_height_goes_in() {
    let mut g = Graph::new("Bevel");
    let tiles = add(&mut g, "tiles");
    let bevel = add(&mut g, "bevel");
    wire(&mut g, (tiles, "shape"), (bevel, "shape"));
    bypass(&mut g, bevel, true);
    let run = Evaluator::with_threads(1).run(registry(), &g, size(), &[]);
    let shape = output(&g, &run, tiles, "shape").unwrap();
    let height = output(&g, &run, bevel, "height").unwrap();
    assert_eq!(height.kind(), ValueKind::Height);
    assert_eq!(Some(height), shape.converted(ValueKind::Height));
}

#[test]
fn an_output_with_nothing_to_pass_on_says_why_after_it() {
    // Material from height makes a material from a height: bypassed, it
    // has no material to pass on.
    let mut g = Graph::new("Nothing");
    let noise = add(&mut g, "noise");
    let mfh = add(&mut g, "material_from_height");
    let out = add(&mut g, "material_out");
    wire(&mut g, (noise, "height"), (mfh, "height"));
    wire(&mut g, (mfh, "material"), (out, "material"));
    bypass(&mut g, mfh, true);
    let mut e = Evaluator::with_threads(1);
    let run = e.run(registry(), &g, size(), &[]);
    assert!(
        run.outcome(mfh).unwrap().is_ok(),
        "the bypassed box itself is fine"
    );
    let why = run.outcome(out).unwrap().as_ref().unwrap_err();
    assert!(
        why.reason.starts_with("Material from height is bypassed"),
        "{}",
        why.reason
    );
    assert!(why.reason.contains("makes that itself"), "{}", why.reason);
    assert_eq!(why.fixes[0], Fix::StopBypass(mfh));
    assert!(e.material(registry(), &g, size()).is_err());
    // A maker bypassed has nothing at all to pass on.
    let mut g = Graph::new("Maker");
    let noise = add(&mut g, "noise");
    let levels = add(&mut g, "levels");
    wire(&mut g, (noise, "height"), (levels, "in"));
    bypass(&mut g, noise, true);
    let run = e.run(registry(), &g, size(), &[]);
    assert_eq!(run.picture(noise), None);
    let why = run.outcome(levels).unwrap().as_ref().unwrap_err();
    assert!(
        why.reason.starts_with("Noise is bypassed"),
        "{}",
        why.reason
    );
}

#[test]
fn an_empty_input_passes_nothing_and_a_broken_one_waits() {
    // Levels needs its input; bypassed with nothing plugged in it doesn't
    // refuse itself, but what it feeds says to plug one in.
    let mut g = Graph::new("Empty");
    let levels = add(&mut g, "levels");
    let blur = add(&mut g, "blur");
    wire(&mut g, (levels, "out"), (blur, "in"));
    bypass(&mut g, levels, true);
    let run = Evaluator::with_threads(1).run(registry(), &g, size(), &[]);
    assert!(run.outcome(levels).unwrap().is_ok());
    let why = run.outcome(blur).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("Plug one in"), "{}", why.reason);
    // Fed by a box that can't run: it waits, as any box would.
    let mut g = Graph::new("Waits");
    let empty = add(&mut g, "levels");
    let levels = add(&mut g, "levels");
    wire(&mut g, (empty, "out"), (levels, "in"));
    bypass(&mut g, levels, true);
    let run = Evaluator::with_threads(1).run(registry(), &g, size(), &[]);
    let why = run.outcome(levels).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("waits for"), "{}", why.reason);
}

#[test]
fn a_bypassed_moving_box_doesnt_fill_the_cache_while_playing() {
    let mut g = Graph::new("Lava");
    let noise = add(&mut g, "noise");
    let scroll = add(&mut g, "scroll");
    let mfh = add(&mut g, "material_from_height");
    let out = add(&mut g, "material_out");
    wire(&mut g, (noise, "height"), (scroll, "in"));
    wire(&mut g, (scroll, "out"), (mfh, "height"));
    wire(&mut g, (mfh, "material"), (out, "material"));
    bypass(&mut g, scroll, true);
    let mut e = Evaluator::with_threads(1);
    e.set_time(Some(0.0));
    let first = e.run(registry(), &g, size(), &[]);
    let (entries, memory) = (e.cache().len(), e.cache().memory());
    for step in 1..40 {
        e.set_time(Some(step as f32 * 0.033));
        let run = e.run(registry(), &g, size(), &[]);
        assert_eq!(run.made(), 0, "nothing worked out again at step {step}");
        assert_eq!(run.key(scroll), first.key(scroll));
        assert_eq!(run.key(mfh), first.key(mfh));
    }
    assert_eq!(e.cache().len(), entries);
    assert_eq!(e.cache().memory(), memory);
    // Switched back on, it moves again.
    bypass(&mut g, scroll, false);
    let a = e.run(registry(), &g, size(), &[]);
    e.set_time(Some(0.5));
    let b = e.run(registry(), &g, size(), &[]);
    assert_ne!(a.key(scroll), b.key(scroll));
}

#[test]
fn switching_bypass_off_and_on_finds_both_results_kept() {
    let mut g = Graph::new("Compare");
    let noise = add(&mut g, "noise");
    let blur = add(&mut g, "blur");
    let mfh = add(&mut g, "material_from_height");
    wire(&mut g, (noise, "height"), (blur, "in"));
    wire(&mut g, (blur, "out"), (mfh, "height"));
    let mut e = Evaluator::with_threads(1);
    let with = e.run(registry(), &g, size(), &[]);
    bypass(&mut g, blur, true);
    let without = e.run(registry(), &g, size(), &[]);
    assert_eq!(without.made(), 1, "only Material from height again");
    bypass(&mut g, blur, false);
    let again = e.run(registry(), &g, size(), &[]);
    assert_eq!(again.made(), 0, "Blur's own result was kept");
    assert_eq!(again.key(mfh), with.key(mfh));
    bypass(&mut g, blur, true);
    let back = e.run(registry(), &g, size(), &[]);
    assert_eq!(back.made(), 0);
    assert_eq!(back.key(mfh), without.key(mfh));
    // A bypassed box's settings don't matter while it is bypassed.
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node: blur,
            key: "radius".into(),
            value: crate::SettingValue::Number(9.0),
        },
    )
    .ok();
    let settled = e.run(registry(), &g, size(), &[]);
    assert_eq!(settled.made(), 0);
}

#[test]
fn bypass_works_inside_your_own_box() {
    let mut g = Graph::new("Own");
    let noise = add(&mut g, "noise");
    let blur = add(&mut g, "blur");
    let mfh = add(&mut g, "material_from_height");
    wire(&mut g, (noise, "height"), (blur, "in"));
    wire(&mut g, (blur, "out"), (mfh, "height"));
    bypass(&mut g, blur, true);
    let plain = Evaluator::with_threads(1).run(registry(), &g, size(), &[]);
    let selection = [blur].into_iter().collect();
    let mut proposal = crate::own::Proposal::of(&g, registry(), &selection).unwrap();
    proposal.name = "Soft".into();
    let id = crate::own::fresh_id("Soft", |_| false);
    let (own, edits) = proposal.make(&g, registry(), id, None).unwrap();
    assert!(own.inner.boxes().all(|(_, b)| b.bypassed), "kept inside");
    let own = std::sync::Arc::new(own);
    let with_own = crate::own::registry_with([&own]);
    let mut grouped = g.clone();
    for edit in &edits {
        grouped.apply(&with_own, edit).unwrap();
    }
    let run = Evaluator::with_threads(1).run(&with_own, &grouped, size(), &[]);
    let made = |r: &Evaluation, gr: &Graph| {
        let id = gr.first_of("material_from_height").unwrap();
        r.picture(id).cloned()
    };
    assert_eq!(made(&run, &grouped), made(&plain, &g), "the same picture");
}
