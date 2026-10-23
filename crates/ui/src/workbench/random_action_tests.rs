//! Random action tests: hundreds of interface actions picked at random,
//! with rules checked after every one (Phase 9, "a safety net for
//! quality").
//!
//! The story tests (`workflow_tests.rs`) check what should happen when a
//! person does things sensibly. These check what must *never* happen,
//! however oddly the app is used: keys pressed in strange orders, clicks
//! and drags anywhere, tools started and abandoned, undo in the middle of
//! things. A person exploring the app does exactly this, and it is where
//! crashes and broken shapes hide.
//!
//! The "random" choices come from a fixed recipe (a seed), so every run
//! makes the same choices and a failure can be replayed exactly. When a
//! rule breaks, the test prints the seed and every action up to that
//! point.
//!
//! The rules, checked after every action:
//! - the app didn't crash;
//! - every object in the scene is listed once in its tree;
//! - the selection names only objects that exist;
//! - no object's position, turn or size, and no point of any shape, is
//!   "not a number" or infinite;
//! - every shape is well formed: each polygon has at least three corners,
//!   all of them points that exist, none visited twice (shown shapes too,
//!   cut by their booleans);
//! - the mode is one of the app's modes;
//! - after Esc, at most one menu, list or window is still open;
//! - and every few actions: Ctrl + Z then Ctrl + Y brings back exactly the
//!   same scene and selection.
//!
//! (Selected points, edges and polygons may name ones a shape no longer
//! has, by design: see `Document::elements`. So that is not a rule.)

use super::modelling_tests::H;
use super::tests::{main_menu, quick_harness};
use super::*;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Runs frames until the window is still, or for a few frames if it keeps
/// moving (framing the view glides for a while, which is fine).
fn settle(harness: &mut H) {
    let _ = harness.run_ok();
}

/// A tiny, predictable random number maker (a linear congruential
/// generator): the same seed always gives the same numbers.
struct Lcg(u64);

impl Lcg {
    /// A number from 0 up to (not including) `below`.
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % below.max(1)
    }

    /// A number between `low` and `high`.
    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * (self.next(10_001) as f32 / 10_000.0)
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.next(from.len())]
    }
}

// ------------------------------------------------------------ actions ---

fn key(harness: &mut H, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    settle(harness);
}

/// One notch of Ctrl + wheel at `at`, up or down.
fn ctrl_wheel(harness: &mut H, at: Pos2, up: bool) {
    harness.event(Event::ModifiersChanged(Modifiers::COMMAND));
    harness.event(Event::PointerMoved(at));
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, if up { 1.0 } else { -1.0 }),
        modifiers: Modifiers::COMMAND,
        phase: egui::TouchPhase::Move,
    });
    settle(harness);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    settle(harness);
}

fn pointer(harness: &mut H, at: Pos2, pressed: bool, keys: Modifiers) {
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: keys,
    });
}

/// A random spot in the viewport, a little in from its sides.
fn spot(harness: &H, rng: &mut Lcg) -> Pos2 {
    let rect = harness
        .get_by_label(crate::VIEWPORT_LABEL)
        .rect()
        .shrink(12.0);
    Pos2::new(
        rng.between(rect.min.x, rect.max.x),
        rng.between(rect.min.y, rect.max.y),
    )
}

/// A click at `at` with `keys` held.
fn click(harness: &mut H, at: Pos2, keys: Modifiers) {
    harness.event(Event::ModifiersChanged(keys));
    harness.event(Event::PointerMoved(at));
    settle(harness);
    pointer(harness, at, true, keys);
    settle(harness);
    pointer(harness, at, false, keys);
    settle(harness);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    settle(harness);
}

/// Two clicks a frame apart: a double-click.
fn double_click(harness: &mut H, at: Pos2, keys: Modifiers) {
    harness.event(Event::ModifiersChanged(keys));
    harness.event(Event::PointerMoved(at));
    harness.step();
    for _ in 0..2 {
        pointer(harness, at, true, keys);
        harness.step();
        pointer(harness, at, false, keys);
        harness.step();
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    settle(harness);
}

/// A left drag from `from` by `by`, in a few steps, with `keys` held.
fn drag(harness: &mut H, from: Pos2, by: egui::Vec2, keys: Modifiers) {
    harness.event(Event::ModifiersChanged(keys));
    harness.event(Event::PointerMoved(from));
    settle(harness);
    pointer(harness, from, true, keys);
    settle(harness);
    for k in 1..=4 {
        harness.event(Event::PointerMoved(from + by * (k as f32 / 4.0)));
        settle(harness);
    }
    pointer(harness, from + by, false, keys);
    settle(harness);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    settle(harness);
}

const MENUS: [&str; 11] = [
    "File", "Edit", "Create", "Modes", "Select", "Tools", "Mesh", "UV", "View", "Window", "Help",
];

const PALETTE: [&str; 11] = [
    "Undo",
    "Redo",
    "Create cube",
    "Create cylinder",
    "Create plane",
    "Create sphere",
    "Snapping",
    "Symmetry",
    "Move",
    "Scale",
    "Rotate",
];

/// Does one random action; returns what it did, in words, for the log.
/// Whether it ended with Esc is the second value.
fn act(harness: &mut H, rng: &mut Lcg) -> (String, bool) {
    let held = [Modifiers::NONE, Modifiers::COMMAND, Modifiers::SHIFT];
    match rng.next(24) {
        0 => {
            let k = rng.pick(&[Key::Num1, Key::Num2, Key::Num3, Key::Tab]);
            key(harness, Modifiers::NONE, k);
            (format!("key {k:?}"), false)
        }
        1 => {
            let k = rng.pick(&[Key::Q, Key::W, Key::R, Key::S, Key::T]);
            key(harness, Modifiers::NONE, k);
            (format!("key {k:?}"), false)
        }
        2 => {
            key(harness, Modifiers::NONE, Key::E);
            key(harness, Modifiers::NONE, Key::Escape);
            ("E, Esc".into(), true)
        }
        3 => {
            let modifiers = rng.pick(&[Modifiers::NONE, Modifiers::ALT]);
            key(harness, modifiers, Key::B);
            if rng.next(2) == 0 {
                key(harness, Modifiers::NONE, Key::Enter);
                (format!("B ({modifiers:?}), Enter"), false)
            } else {
                key(harness, Modifiers::NONE, Key::Escape);
                (format!("B ({modifiers:?}), Esc"), true)
            }
        }
        4 | 5 => {
            let at = spot(harness, rng);
            let keys = rng.pick(&held);
            click(harness, at, keys);
            (format!("click at {at:?} holding {keys:?}"), false)
        }
        6 => {
            let from = spot(harness, rng);
            let to = spot(harness, rng);
            let keys = rng.pick(&held);
            drag(harness, from, to - from, keys);
            (format!("drag {from:?} to {to:?} holding {keys:?}"), false)
        }
        7 => {
            let at = spot(harness, rng);
            let keys = rng.pick(&[Modifiers::NONE, Modifiers::ALT]);
            double_click(harness, at, keys);
            (format!("double-click at {at:?} holding {keys:?}"), false)
        }
        8 => {
            let k = rng.pick(&[Key::Z, Key::Y]);
            key(harness, Modifiers::COMMAND, k);
            (format!("Ctrl + {k:?}"), false)
        }
        9 => {
            key(harness, Modifiers::NONE, Key::Delete);
            ("Delete".into(), false)
        }
        10 | 11 => {
            let k = rng.pick(&[Key::D, Key::I, Key::V]);
            key(harness, Modifiers::NONE, k);
            let from = spot(harness, rng);
            let by = egui::vec2(rng.between(-60.0, 60.0), rng.between(-60.0, 60.0));
            drag(harness, from, by, Modifiers::NONE);
            (format!("key {k:?}, drag {from:?} by {by:?}"), false)
        }
        12 => {
            key(harness, Modifiers::NONE, Key::M);
            ("M".into(), false)
        }
        13 => {
            let (modifiers, k) = rng.pick(&[
                (Modifiers::COMMAND, Key::A),
                (Modifiers::COMMAND, Key::I),
                (Modifiers::ALT, Key::A),
            ]);
            key(harness, modifiers, k);
            (format!("{modifiers:?} + {k:?}"), false)
        }
        14 => {
            key(harness, Modifiers::NONE, Key::Space);
            key(harness, Modifiers::NONE, Key::Escape);
            ("Space, Esc".into(), true)
        }
        15 => {
            let k = rng.pick(&[Key::H, Key::Z]);
            key(harness, Modifiers::NONE, k);
            (format!("key {k:?}"), false)
        }
        16 => {
            let menu = rng.pick(&MENUS);
            main_menu(harness, menu).click();
            settle(harness);
            key(harness, Modifiers::NONE, Key::Escape);
            (format!("menu {menu}, Esc"), true)
        }
        17 => {
            let at = spot(harness, rng);
            let up = rng.next(2) == 0;
            ctrl_wheel(harness, at, up);
            (
                format!("Ctrl + wheel {} at {at:?}", if up { "up" } else { "down" }),
                false,
            )
        }
        18 => {
            let label = rng.pick(&PALETTE);
            let found = harness.query_all_by_label(label).next().is_some();
            if found {
                harness.query_all_by_label(label).next().unwrap().click();
                settle(harness);
            }
            (format!("palette {label} (found: {found})"), false)
        }
        19 => {
            // A boolean on whatever is selected (decision 0135): the first
            // selected kept, the others cutting it.
            let label = rng.pick(&["Union", "Subtract", "Intersect"]);
            let found = harness.query_all_by_label(label).next().is_some();
            if found {
                harness.query_all_by_label(label).next().unwrap().click();
                settle(harness);
            }
            (format!("boolean {label} (found: {found})"), false)
        }
        20 | 21 => {
            // Aimed at a shape, so selections and edits happen often.
            let at = object_spot(harness, rng).unwrap_or_else(|| spot(harness, rng));
            let keys = rng.pick(&held);
            click(harness, at, keys);
            (
                format!("click on a shape at {at:?} holding {keys:?}"),
                false,
            )
        }
        22 => {
            let k = rng.pick(&[Key::D, Key::I, Key::V]);
            key(harness, Modifiers::NONE, k);
            let from = object_spot(harness, rng).unwrap_or_else(|| spot(harness, rng));
            let by = egui::vec2(rng.between(-60.0, 60.0), rng.between(-60.0, 60.0));
            drag(harness, from, by, Modifiers::NONE);
            (
                format!("key {k:?}, drag from a shape at {from:?} by {by:?}"),
                false,
            )
        }
        _ => {
            key(harness, Modifiers::NONE, Key::Escape);
            ("Esc".into(), true)
        }
    }
}

/// Where a random shape's middle is on screen, near enough; `None` with no
/// shapes or when it is out of view.
fn object_spot(harness: &H, rng: &mut Lcg) -> Option<Pos2> {
    let workbench = &harness.state().workbench;
    let scene = workbench.document().scene();
    let ids: Vec<scene::ObjectId> = scene.walk().into_iter().map(|(id, _)| id).collect();
    if ids.is_empty() {
        return None;
    }
    let id = ids[rng.next(ids.len())];
    let local = scene.evaluated(id)?.mesh.bounds()?;
    let world = scene.world_matrix(id).transform_point3(local.centre());
    // Somewhere within the shape's size of its middle.
    let size = (local.max - local.min) * 0.3;
    let nudge = glam::Vec3::new(
        rng.between(-size.x, size.x),
        rng.between(-size.y, size.y),
        rng.between(-size.z, size.z),
    );
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let view = glam::Vec2::new(rect.width(), rect.height());
    let at = workbench.viewport().camera().project(world + nudge, view)?;
    let at = rect.min + egui::vec2(at.x, at.y);
    rect.shrink(4.0).contains(at).then_some(at)
}

// -------------------------------------------------------------- rules ---

/// Whether a shape is well formed; why not if it isn't.
fn check_shape(what: &str, mesh: &scene::Mesh) -> Result<(), String> {
    for (v, p) in mesh.vertices() {
        if !p.is_finite() {
            return Err(format!("{what}: point {v:?} is at {p}"));
        }
    }
    for (f, corners) in mesh.faces() {
        if corners.len() < 3 {
            return Err(format!(
                "{what}: polygon {f:?} has {} corners",
                corners.len()
            ));
        }
        for (k, v) in corners.iter().enumerate() {
            if mesh.position(*v).is_none() {
                return Err(format!("{what}: polygon {f:?} uses missing point {v:?}"));
            }
            if corners[..k].contains(v) {
                return Err(format!("{what}: polygon {f:?} visits point {v:?} twice"));
            }
        }
    }
    Ok(())
}

/// The rules that must hold after every action.
fn check(harness: &H) -> Result<(), String> {
    let workbench = &harness.state().workbench;
    let scene = workbench.document().scene();
    let walk = scene.walk();
    if walk.len() != scene.len() {
        return Err(format!(
            "the tree lists {} objects, the scene has {}",
            walk.len(),
            scene.len()
        ));
    }
    for id in workbench.document().selection() {
        if !scene.contains(id) {
            return Err(format!("object {id:?} is selected but not in the scene"));
        }
    }
    for (id, _) in &walk {
        let Some(object) = scene.object(*id) else {
            return Err(format!("object {id:?} is listed but missing"));
        };
        let t = object.transform;
        if !(t.position.is_finite() && t.rotation.is_finite() && t.scale.is_finite()) {
            return Err(format!("object {id:?} has a transform of {t:?}"));
        }
        if let Some(shape) = scene.evaluated(*id) {
            check_shape(&format!("object {id:?}"), &shape.mesh)?;
        }
        if let Some(shown) = scene.shown_shape(*id) {
            check_shape(&format!("object {id:?} as shown"), &shown)?;
        }
    }
    if !crate::palette::Mode::ALL.contains(&workbench.mode()) {
        return Err(format!("unknown mode {:?}", workbench.mode()));
    }
    Ok(())
}

/// How many menus, lists and windows are open.
fn open_things(harness: &H) -> usize {
    let workbench = &harness.state().workbench;
    usize::from(egui::Popup::is_any_open(&harness.ctx))
        + usize::from(workbench.window_open())
        + usize::from(workbench.shape_picker().is_open())
}

/// What undo then redo must bring back exactly.
fn snapshot(harness: &H) -> (scene::Scene, Vec<scene::ObjectId>) {
    let doc = harness.state().workbench.document();
    (doc.scene().clone(), doc.selection())
}

/// Ctrl + Z then Ctrl + Y; the document must be as it was. Skipped while
/// typing (the keys belong to the text) or with nothing to undo. Both the
/// scene and the selection are compared.
fn undo_then_redo(harness: &mut H) -> Result<(), String> {
    let typing = harness.ctx.memory(|m| m.focused().is_some());
    if typing || harness.state().workbench.document().undo_steps() == 0 {
        return Ok(());
    }
    let before = snapshot(harness);
    let label = harness
        .state()
        .workbench
        .document()
        .undo_label()
        .unwrap_or_default()
        .to_string();
    key(harness, Modifiers::COMMAND, Key::Z);
    key(harness, Modifiers::COMMAND, Key::Y);
    let after = snapshot(harness);
    if after.0 != before.0 {
        return Err(format!("undo then redo of \"{label}\" changed the scene"));
    }
    if after.1 != before.1 {
        return Err(format!(
            "undo then redo of \"{label}\" changed the selection: {:?} to {:?}",
            before.1, after.1
        ));
    }
    Ok(())
}

/// Runs `actions` random actions from `seed`, checking the rules after
/// each, and returns the window as it is at the end. Panics with the seed
/// and the actions so far if a rule breaks.
fn run_seed(seed: u64, actions: usize) -> H {
    let mut rng = Lcg(seed);
    let mut harness = quick_harness();
    let mut log: Vec<String> = Vec::new();
    // Set RANDOM_ACTIONS_TRACE=1 to print the state after every action,
    // when replaying a failure.
    let trace = std::env::var_os("RANDOM_ACTIONS_TRACE").is_some();
    let fail = |log: &[String], why: &str| -> ! {
        panic!(
            "seed {seed}: {why}\nactions, oldest first:\n{}",
            log.iter()
                .enumerate()
                .map(|(n, a)| format!("{n:4}: {a}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    for n in 0..actions {
        let done = catch_unwind(AssertUnwindSafe(|| act(&mut harness, &mut rng)));
        let (what, esc) = match done {
            Ok(done) => done,
            Err(panic) => {
                let why = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                log.push("(the next action)".into());
                fail(&log, &format!("the app panicked: {why}"));
            }
        };
        if trace {
            let doc = harness.state().workbench.document();
            println!(
                "{n:4}: {what} -> mode {:?}, selected {:?}, undo {:?}, {} steps",
                harness.state().workbench.mode(),
                doc.selection(),
                doc.undo_label(),
                doc.undo_steps()
            );
        }
        log.push(what);
        if let Err(why) = check(&harness) {
            fail(&log, &why);
        }
        if esc && open_things(&harness) > 1 {
            fail(
                &log,
                &format!("{} things open after Esc", open_things(&harness)),
            );
        }
        if n % 10 == 9 {
            log.push("(check: Ctrl + Z, Ctrl + Y)".into());
            let checked = catch_unwind(AssertUnwindSafe(|| undo_then_redo(&mut harness)));
            match checked {
                Ok(Ok(())) => {}
                Ok(Err(why)) => fail(&log, &why),
                Err(_) => fail(&log, "the app panicked during undo and redo"),
            }
            if let Err(why) = check(&harness) {
                fail(&log, &why);
            }
        }
    }
    harness
}

/// How many actions each seed runs.
const ACTIONS: usize = 250;

#[test]
fn random_actions_keep_the_rules_seed_1() {
    let _ = run_seed(1, ACTIONS);
}

#[test]
fn random_actions_keep_the_rules_seed_2() {
    let _ = run_seed(2, ACTIONS);
}

#[test]
fn random_actions_keep_the_rules_seed_3() {
    let _ = run_seed(3, ACTIONS);
}

#[test]
fn random_actions_keep_the_rules_seed_4() {
    let _ = run_seed(4, ACTIONS);
}

#[test]
fn random_actions_keep_the_rules_seed_5() {
    let _ = run_seed(5, ACTIONS);
}

#[test]
fn random_actions_keep_the_rules_seed_6() {
    let _ = run_seed(6, ACTIONS);
}

/// The rules themselves notice what they look for (a rule that can never
/// fail checks nothing): an open menu is counted, and Esc closes it.
#[test]
fn the_rules_see_an_open_menu() {
    let mut harness = quick_harness();
    assert_eq!(open_things(&harness), 0);
    main_menu(&harness, "File").click();
    settle(&mut harness);
    assert_eq!(open_things(&harness), 1);
    key(&mut harness, Modifiers::NONE, Key::Space);
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(open_things(&harness), 0);
    assert!(check(&harness).is_ok());
}

// ------------------------------------------------- found by these tests ---

/// Found by the random actions (fixed): redoing a step that chose the
/// selection itself (here, creating a cube, which selects it) brought the
/// object back unselected. Undo then redo returns exactly to where the
/// person was.
#[test]
fn redo_of_a_create_selects_the_shape_again() {
    let mut harness = super::tests::empty();
    // B, Enter: a cube, selected.
    key(&mut harness, Modifiers::NONE, Key::B);
    key(&mut harness, Modifiers::NONE, Key::Enter);
    let made = harness.state().workbench.document().selection();
    assert_eq!(made.len(), 1);
    // Ctrl + Z takes it away; Ctrl + Y brings it back...
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    key(&mut harness, Modifiers::COMMAND, Key::Y);
    // ...and it should be selected, as it was.
    assert_eq!(harness.state().workbench.document().selection(), made);
}

/// The same bug the other way round (fixed): undoing a Delete brought the
/// deleted shape back, but not selected as it was.
#[test]
fn undo_of_a_delete_selects_the_shapes_again() {
    let mut harness = super::tests::fresh();
    // Ctrl + A selects the cube; Delete deletes it.
    key(&mut harness, Modifiers::COMMAND, Key::A);
    let chosen = harness.state().workbench.document().selection();
    assert_eq!(chosen.len(), 1);
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert!(harness.state().workbench.document().scene().is_empty());
    // Ctrl + Z: the cube is back, and should be selected again.
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(harness.state().workbench.document().scene().len(), 1);
    assert_eq!(harness.state().workbench.document().selection(), chosen);
}
