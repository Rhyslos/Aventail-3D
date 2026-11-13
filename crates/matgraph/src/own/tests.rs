//! Tests for your own boxes: making one keeps the picture, opening it up
//! gives the boxes back, sliders, linked boxes, own boxes inside own
//! boxes, the refusals, damaged definitions, and a fuzz run of random
//! selections.

use super::*;
use crate::graph::{Frame, GraphEdit};
use crate::image::Size;
use crate::settings::SettingValue;
use crate::value::MaterialMaps;
use glam::Vec2;

fn standard() -> &'static Registry {
    Registry::standard()
}

fn add(g: &mut Graph, kind: &str, x: f32) -> BoxId {
    let id = BoxId(g.next_id());
    g.apply(
        standard(),
        &GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, 0.0),
        },
    )
    .unwrap();
    id
}

fn wire(g: &mut Graph, from: (BoxId, &str), to: (BoxId, &str)) {
    g.apply(
        standard(),
        &GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        },
    )
    .unwrap();
}

fn set(g: &mut Graph, node: BoxId, key: &str, value: SettingValue) {
    g.apply(
        standard(),
        &GraphEdit::SetSetting {
            node,
            key: key.into(),
            value,
        },
    )
    .unwrap();
}

/// Noise and Tiles blended, levelled, made into a material: the boxes.
struct Stone {
    graph: Graph,
    noise: BoxId,
    tiles: BoxId,
    blend: BoxId,
    levels: BoxId,
    mat: BoxId,
}

fn stone() -> Stone {
    let mut g = Graph::new("Stone");
    let noise = add(&mut g, "noise", 0.0);
    let tiles = add(&mut g, "tiles", 0.0);
    let blend = add(&mut g, "blend", 200.0);
    let levels = add(&mut g, "levels", 400.0);
    let mat = add(&mut g, "material_from_height", 600.0);
    let out = add(&mut g, "material_out", 800.0);
    wire(&mut g, (tiles, "shape"), (blend, "bottom"));
    wire(&mut g, (noise, "height"), (blend, "top"));
    wire(&mut g, (tiles, "shape"), (blend, "where"));
    wire(&mut g, (blend, "out"), (levels, "in"));
    wire(&mut g, (levels, "out"), (mat, "height"));
    wire(&mut g, (mat, "material"), (out, "material"));
    set(&mut g, levels, "low", SettingValue::Number(0.2));
    set(&mut g, noise, "seed", SettingValue::Whole(7));
    set(&mut g, out, "size_cm", SettingValue::Number(150.0));
    g.apply(standard(), &GraphEdit::SetSeed(42)).unwrap();
    Stone {
        graph: g,
        noise,
        tiles,
        blend,
        levels,
        mat,
    }
}

fn material(registry: &Registry, graph: &Graph) -> Arc<MaterialMaps> {
    Evaluator::with_threads(2)
        .material(registry, graph, Size::nearest(32))
        .unwrap()
}

/// `selection` of `graph` made into an own box with `sliders` chosen (by
/// setting key): the graph after, the box, and the registry knowing it.
fn made(
    graph: &Graph,
    selection: &[BoxId],
    sliders: &[&str],
) -> (Graph, Arc<OwnBox>, Registry, BoxId) {
    made_with(graph, standard(), &[], selection, sliders)
}

fn made_with(
    graph: &Graph,
    registry: &Registry,
    known: &[Arc<OwnBox>],
    selection: &[BoxId],
    sliders: &[&str],
) -> (Graph, Arc<OwnBox>, Registry, BoxId) {
    let mut proposal = Proposal::of(graph, registry, &selection.iter().copied().collect()).unwrap();
    for s in &mut proposal.sliders {
        s.chosen = sliders.contains(&s.setting.as_str());
    }
    proposal.name = "Rocky".into();
    let id = fresh_id("Rocky", |id| known.iter().any(|o| o.id == id));
    let (own, edits) = proposal.make(graph, registry, id, None).unwrap();
    let own = Arc::new(own);
    let mut all: Vec<Arc<OwnBox>> = known.to_vec();
    all.push(own.clone());
    let registry = registry_with(&all);
    let placed = BoxId(graph.next_id());
    let mut after = graph.clone();
    for edit in &edits {
        after.apply(&registry, edit).unwrap();
    }
    (after, own, registry, placed)
}

#[test]
fn making_an_own_box_keeps_the_picture() {
    let s = stone();
    let before = material(standard(), &s.graph);
    let (after, own, registry, placed) = made(&s.graph, &[s.blend, s.levels], &[]);
    assert_eq!(after.box_count(), s.graph.box_count() - 1);
    assert_eq!(after.get(placed).unwrap().kind, own.id);
    // Tiles feeds two plugs inside: one input; Noise another.
    assert_eq!(own.inputs.len(), 2);
    assert_eq!(own.outputs.len(), 1);
    assert_eq!(own.inner.box_count(), 2);
    assert_eq!(material(&registry, &after), before);
}

#[test]
fn opening_it_back_up_gives_the_boxes_and_the_picture_back() {
    let s = stone();
    let before = material(standard(), &s.graph);
    let (after, _, registry, placed) = made(&s.graph, &[s.blend, s.levels], &["low"]);
    let opened = open_up(&after, &registry, placed).unwrap();
    let mut back = after.clone();
    for edit in &opened.edits {
        back.apply(&registry, edit).unwrap();
    }
    assert_eq!(back.box_count(), s.graph.box_count());
    assert_eq!(back.wires().len(), s.graph.wires().len());
    assert_eq!(opened.boxes.len(), 2);
    assert!(back.get(placed).is_none());
    assert_eq!(material(&registry, &back), before);
    // An app box doesn't open up.
    let why = open_up(&after, &registry, s.mat).unwrap_err();
    assert!(why.reason.contains("Alt + G"), "{}", why.reason);
}

#[test]
fn sliders_start_at_the_inner_value_and_change_the_inside() {
    let s = stone();
    let (mut after, own, registry, placed) = made(&s.graph, &[s.levels], &["low", "invert"]);
    assert_eq!(own.sliders.len(), 2);
    let info = registry.get(&own.id).unwrap().info().clone();
    assert_eq!(info.group, Group::Own);
    assert_eq!(info.settings[0].standard(), SettingValue::Number(0.2));
    assert_eq!(info.name.show(), "Rocky");
    let first = material(&registry, &after);
    set_on(
        &mut after,
        &registry,
        placed,
        "s1",
        SettingValue::Number(0.6),
    );
    assert_ne!(material(&registry, &after), first);
    // Out of range is refused as for any setting.
    let edit = GraphEdit::SetSetting {
        node: placed,
        key: "s1".into(),
        value: SettingValue::Number(4.0),
    };
    assert!(after.apply(&registry, &edit).is_err());
    // The value carries over when it is opened up.
    let opened = open_up(&after, &registry, placed).unwrap();
    let mut back = after.clone();
    for edit in &opened.edits {
        back.apply(&registry, edit).unwrap();
    }
    let levels = back.first_of("levels").unwrap();
    assert_eq!(
        back.get(levels).unwrap().settings.get("low"),
        Some(SettingValue::Number(0.6))
    );
}

fn set_on(g: &mut Graph, registry: &Registry, node: BoxId, key: &str, value: SettingValue) {
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

#[test]
fn placed_boxes_are_linked_to_their_definition() {
    let s = stone();
    let (mut after, own, registry, placed) = made(&s.graph, &[s.levels], &[]);
    // A second box of it, fed the same way, in place of Material from
    // height's input: both run the same inside.
    let second = BoxId(after.next_id());
    after
        .apply(
            &registry,
            &GraphEdit::AddBox {
                kind: own.id.clone(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
    after
        .apply(
            &registry,
            &GraphEdit::Connect {
                from: PlugRef::new(s.blend, "out"),
                to: PlugRef::new(second, "in1"),
            },
        )
        .unwrap();
    let mut evaluator = Evaluator::with_threads(1);
    let first = evaluator.run(&registry, &after, Size::nearest(32), &[]);
    assert_eq!(first.picture(placed), first.picture(second));
    // Change inside: Levels turned upside down. Both change, and nothing
    // old is reused for them.
    let mut changed = (*own).clone();
    let mut inner = (*changed.inner).clone();
    inner
        .apply(
            &registry,
            &GraphEdit::SetSetting {
                node: s.levels,
                key: "invert".into(),
                value: SettingValue::Switch(true),
            },
        )
        .unwrap();
    changed.inner = Arc::new(inner);
    let registry2 = registry_with(&[Arc::new(changed)]);
    let again = evaluator.run(&registry2, &after, Size::nearest(32), &[]);
    assert_eq!(again.picture(placed), again.picture(second));
    assert_ne!(again.picture(placed), first.picture(placed));
}

#[test]
fn own_boxes_go_inside_own_boxes() {
    let s = stone();
    let before = material(standard(), &s.graph);
    let (after, inner_own, registry, placed) = made(&s.graph, &[s.levels], &["low"]);
    let (outer, outer_own, registry2, placed2) = made_with(
        &after,
        &registry,
        std::slice::from_ref(&inner_own),
        &[s.blend, placed],
        &[],
    );
    assert!(outer_own.uses().contains(&inner_own.id));
    assert_eq!(material(&registry2, &outer), before);
    // Opened up one level at a time, the picture stays.
    let opened = open_up(&outer, &registry2, placed2).unwrap();
    let mut back = outer.clone();
    for edit in &opened.edits {
        back.apply(&registry2, edit).unwrap();
    }
    assert_eq!(material(&registry2, &back), before);
    assert_eq!(super::making::depth(&registry2, &outer_own.id), 2);
}

#[test]
fn a_box_holding_itself_is_left_out_of_the_registry() {
    let s = stone();
    let (_, own, _, _) = made(&s.graph, &[s.levels], &[]);
    let mut selfish = (*own).clone();
    let mut inner = (*selfish.inner).clone();
    let mut registry = Registry::standard().clone();
    // Kept as an unknown kind, as a file would.
    let mut parts = inner.to_parts();
    parts.boxes.push((
        BoxId(parts.next_id),
        crate::GraphBox::new(own.id.clone(), Vec2::ZERO),
    ));
    parts.next_id += 1;
    inner = Graph::from_parts(parts, &registry).unwrap();
    selfish.inner = Arc::new(inner);
    assert!(selfish.check(&registry).is_err());
    registry = registry_with(&[Arc::new(selfish)]);
    assert!(registry.get(&own.id).is_none());
}

#[test]
fn what_inside_can_t_run_says_so() {
    let s = stone();
    let (after, own, _, placed) = made(&s.graph, &[s.levels], &[]);
    // The box giving the output removed inside.
    let mut broken = (*own).clone();
    let mut inner = (*broken.inner).clone();
    inner
        .apply(standard(), &GraphEdit::RemoveBoxes(vec![s.levels]))
        .unwrap();
    broken.inner = Arc::new(inner);
    let broken = broken.normalised(standard());
    assert!(broken.outputs[0].from.is_none());
    assert!(broken.inputs[0].to.is_empty());
    let registry = registry_with(&[Arc::new(broken)]);
    let evaluation = Evaluator::with_threads(1).run(&registry, &after, Size::nearest(32), &[]);
    let why = evaluation.outcome(placed).unwrap().as_ref().unwrap_err();
    assert!(
        why.reason.contains("Nothing inside Rocky"),
        "{}",
        why.reason
    );
    // Not plugged in: the box inside says what it needs.
    let mut alone = Graph::new("Alone");
    let id = BoxId(alone.next_id());
    alone
        .apply(
            &registry_with(std::slice::from_ref(&own)),
            &GraphEdit::AddBox {
                kind: own.id.clone(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
    let registry = registry_with(&[own]);
    let evaluation = Evaluator::with_threads(1).run(&registry, &alone, Size::nearest(32), &[]);
    let why = evaluation.outcome(id).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("nothing plugged"), "{}", why.reason);
}

#[test]
fn selections_that_can_t_be_made_say_why() {
    let s = stone();
    let r = standard();
    let pick = |ids: &[BoxId]| ids.iter().copied().collect::<BTreeSet<_>>();
    let why = Proposal::of(&s.graph, r, &pick(&[])).unwrap_err();
    assert!(why.reason.contains("Select the boxes"));
    let out = s.graph.first_of("material_out").unwrap();
    let why = Proposal::of(&s.graph, r, &pick(&[s.mat, out])).unwrap_err();
    assert!(why.reason.contains("Material out"), "{}", why.reason);
    // Blend and Material from height, without Levels between them: Levels
    // would be fed by the box and feed it.
    let why = Proposal::of(&s.graph, r, &pick(&[s.blend, s.mat])).unwrap_err();
    assert!(why.reason.contains("loop"), "{}", why.reason);
    assert_eq!(why.fixes, vec![crate::Fix::LookAt(s.levels)]);
    // From the model boxes stay outside.
    let mut g = s.graph.clone();
    let curv = add(&mut g, "model_curvature", 0.0);
    let why = Proposal::of(&g, r, &pick(&[curv])).unwrap_err();
    assert!(why.reason.contains("model"), "{}", why.reason);
    // So do the weathering boxes, which read the model's maps too.
    let dirt = add(&mut g, "dirt", 0.0);
    let why = Proposal::of(&g, r, &pick(&[dirt])).unwrap_err();
    assert!(why.reason.contains("Dirt"), "{}", why.reason);
    // No output kept, no name.
    let mut p = Proposal::of(&s.graph, r, &pick(&[s.levels])).unwrap();
    p.outputs[0].keep = false;
    assert!(p.make(&s.graph, r, fresh_id("x", |_| false), None).is_err());
    p.outputs[0].keep = true;
    p.name = "  ".into();
    let why = p
        .make(&s.graph, r, fresh_id("x", |_| false), None)
        .unwrap_err();
    assert!(why.reason.contains("name"));
}

#[test]
fn a_selection_feeding_nothing_gives_out_its_last_boxes() {
    let mut g = Graph::new("Loose");
    let noise = add(&mut g, "noise", 0.0);
    let levels = add(&mut g, "levels", 200.0);
    wire(&mut g, (noise, "height"), (levels, "in"));
    let p = Proposal::of(&g, standard(), &[noise, levels].into()).unwrap();
    assert_eq!(p.outputs.len(), 1);
    assert_eq!(p.outputs[0].from, PlugRef::new(levels, "out"));
    assert!(p.inputs.is_empty());
    // Settings shown in the painter are chosen as sliders to start with.
    let mut g2 = g.clone();
    g2.apply(
        standard(),
        &GraphEdit::ShowInPainter {
            node: noise,
            key: "scale".into(),
            shown: true,
        },
    )
    .unwrap();
    let p = Proposal::of(&g2, standard(), &[noise, levels].into()).unwrap();
    let chosen: Vec<_> = p.sliders.iter().filter(|s| s.chosen).collect();
    assert_eq!(chosen.len(), 1);
    assert_eq!(chosen[0].setting, "scale");
}

#[test]
fn comment_frames_round_only_selected_boxes_go_inside() {
    let s = stone();
    let mut g = s.graph.clone();
    g.apply(
        standard(),
        &GraphEdit::AddFrame(Frame::new(
            "Levels",
            Vec2::new(350.0, -50.0),
            Vec2::new(550.0, 300.0),
        )),
    )
    .unwrap();
    g.apply(
        standard(),
        &GraphEdit::AddFrame(Frame::new(
            "Everything",
            Vec2::new(-50.0, -50.0),
            Vec2::new(2000.0, 300.0),
        )),
    )
    .unwrap();
    let (_, own, _, _) = made(&g, &[s.levels], &[]);
    let titles: Vec<_> = own.inner.frames().map(|(_, f)| f.title.clone()).collect();
    assert_eq!(titles, vec!["Levels".to_string()]);
}

#[test]
fn damaged_definitions_are_refused() {
    let s = stone();
    let (_, own, _, _) = made(&s.graph, &[s.blend, s.levels], &["low"]);
    let r = standard();
    assert!(own.check(r).is_ok());
    let mut bad = (*own).clone();
    bad.id = "own:xyz".into();
    assert!(bad.check(r).is_err());
    let mut bad = (*own).clone();
    bad.name = String::new();
    assert!(bad.check(r).is_err());
    let mut bad = (*own).clone();
    bad.picture = Some(vec![0; 5]);
    assert!(bad.check(r).is_err());
    let mut bad = (*own).clone();
    bad.inputs[0].to.push(PlugRef::new(BoxId(999), "in"));
    assert!(bad.check(r).is_err());
    let mut bad = (*own).clone();
    bad.outputs[0].kind = ValueKind::Colour;
    assert!(bad.check(r).is_err());
    let mut bad = (*own).clone();
    bad.sliders[0].setting = "nothing".into();
    assert!(bad.check(r).is_err());
    let mut bad = (*own).clone();
    bad.sliders[0].key = bad.inputs[0].key.clone();
    assert!(bad.check(r).is_err());
    // Parts round trip.
    let back = OwnBox::from_parts(own.to_parts(), r).unwrap();
    assert_eq!(back, *own);
    assert!(is_own_id(&own.id));
    assert!(!is_own_id("own:"));
    assert!(!is_own_id("noise"));
}

#[test]
fn inside_a_placed_box_its_pictures_follow_what_is_plugged_in() {
    let s = stone();
    let size = Size::nearest(32);
    let mut evaluator = Evaluator::with_threads(1);
    let before = evaluator.run(standard(), &s.graph, size, &[]);
    let (after, _, registry, placed) = made(&s.graph, &[s.blend, s.levels], &[]);
    let (inner, inside) = evaluator
        .run_inside(&registry, &after, &[placed], size)
        .unwrap();
    assert_eq!(inner.seed(), after.seed());
    assert_eq!(inside.picture(s.levels), before.picture(s.levels));
    assert_eq!(inside.picture(s.blend), before.picture(s.blend));
    assert!(
        evaluator
            .run_inside(&registry, &after, &[s.mat], size)
            .is_err()
    );
}

#[test]
fn random_selections_make_boxes_that_keep_the_picture() {
    let s = stone();
    let before = material(standard(), &s.graph);
    let ids: Vec<BoxId> = [s.noise, s.tiles, s.blend, s.levels, s.mat].into();
    // Every subset of the five boxes.
    for mask in 1u32..32 {
        let selection: BTreeSet<BoxId> = ids
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, id)| *id)
            .collect();
        let Ok(proposal) = Proposal::of(&s.graph, standard(), &selection) else {
            continue;
        };
        let id = fresh_id(&format!("{mask}"), |_| false);
        let (own, edits) = proposal.make(&s.graph, standard(), id, None).unwrap();
        let registry = registry_with(&[Arc::new(own)]);
        let mut after = s.graph.clone();
        for edit in &edits {
            after.apply(&registry, edit).unwrap();
        }
        assert_eq!(material(&registry, &after), before, "selection {mask:b}");
        let placed = BoxId(s.graph.next_id());
        let opened = open_up(&after, &registry, placed).unwrap();
        for edit in &opened.edits {
            after.apply(&registry, edit).unwrap();
        }
        assert_eq!(material(&registry, &after), before, "opened {mask:b}");
    }
}

#[test]
fn fresh_ids_are_new() {
    let a = fresh_id("Rocky", |_| false);
    let b = fresh_id("Rocky", |id| id == a);
    assert_ne!(a, b);
    assert!(is_own_id(&a) && is_own_id(&b));
}
