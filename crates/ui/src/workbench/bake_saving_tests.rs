//! Tests for the Bake window's settings kept with the project (decision
//! 0254).

use super::bake_saving::BakeSettings;
use super::baker::Baker;
use super::seams_tests::{add, steps};
use super::tests::fresh;
use bake::maps::CAGE_RANGE;

/// Changed settings, as a project would keep them.
fn changed(w: &mut super::Workbench, high: scene::ObjectId) {
    w.baker.maps = [false, true, false, false, false, true, false];
    w.baker.high = Some(high);
    w.baker.godot = true;
    w.baker.roughness = 0.3;
    w.baker.metal = true;
    w.baker.cage = 0.05;
    w.baker.green_down = true;
    w.baker.skip_stacked = false;
    w.baker.show_cage = false;
}

#[test]
fn a_change_is_kept_with_the_project_without_an_undo_step() {
    let mut harness = fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.run();
    let before = steps(&harness);
    assert_eq!(
        harness.state().workbench.document().scene().bake_settings(),
        "",
        "standard settings keep nothing"
    );
    let saved = harness.state().workbench.document().revision();
    changed(&mut harness.state_mut().workbench, id);
    harness.run();
    let w = &harness.state().workbench;
    assert_eq!(steps(&harness), before, "not a step to undo");
    assert!(w.document().revision() > saved, "a change to save");
    let kept = BakeSettings::read(w.document().scene().bake_settings());
    assert_eq!(kept, BakeSettings::of(&w.baker));
    assert!(kept.godot && kept.green_down);
}

#[test]
fn an_opened_project_gives_the_window_its_settings_and_is_not_changed() {
    let mut harness = fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    changed(&mut harness.state_mut().workbench, id);
    harness.run();
    let bytes = fileio::project::to_bytes(harness.state().workbench.document()).unwrap();
    let wanted = BakeSettings::of(&harness.state().workbench.baker);
    // Another project, with the standard settings, then this one opened.
    let mut harness = fresh();
    assert_eq!(
        BakeSettings::of(&harness.state().workbench.baker),
        BakeSettings::default()
    );
    let doc = fileio::project::from_bytes(&bytes).unwrap();
    harness
        .state_mut()
        .workbench
        .opened(doc, std::path::PathBuf::from("kit.avtl"));
    harness.run();
    harness.run();
    let w = &harness.state().workbench;
    assert_eq!(BakeSettings::of(&w.baker), wanted);
    assert_eq!(w.baker.high, Some(id));
    assert!(!w.has_unsaved_changes(), "opening it isn't a change");
    // A project from before (none kept): the standard settings.
    let mut harness2 = fresh();
    changed(&mut harness2.state_mut().workbench, id);
    harness2
        .state_mut()
        .workbench
        .opened(scene::Document::new(), std::path::PathBuf::from("old.avtl"));
    harness2.run();
    let w = &harness2.state().workbench;
    assert_eq!(BakeSettings::of(&w.baker), BakeSettings::default());
    assert!(!w.has_unsaved_changes());
}

#[test]
fn the_settings_text_round_trips() {
    let mut baker = Baker::default();
    baker.maps = [true, false, true, false, false, false, true];
    baker.high = Some(scene::ObjectId::from_raw(42));
    baker.by_name = true;
    baker.roughness = 0.123;
    baker.cage = 0.0314;
    let text = BakeSettings::of(&baker).text();
    assert!(!text.is_empty());
    let mut back = Baker::default();
    BakeSettings::read(&text).give(&mut back);
    assert_eq!(BakeSettings::of(&back), BakeSettings::of(&baker));
    assert_eq!(BakeSettings::default().text(), "");
    assert_eq!(BakeSettings::read(""), BakeSettings::default());
    // Settings missing (an older text) read as standard; unknown map
    // names are ignored.
    let partial = BakeSettings::read("(godot: true, maps: [\"normal\", \"sparkle\"])");
    let mut b = Baker::default();
    partial.give(&mut b);
    assert!(b.godot);
    assert_eq!(b.maps, [true, false, false, false, false, false, false]);
    assert_eq!(b.cage, Baker::default().cage);
}

/// Fuzz: any text, damaged or random, gives settings within range and
/// never panics.
#[test]
fn any_settings_text_gives_usable_settings() {
    let good = {
        let mut baker = Baker::default();
        baker.godot = true;
        BakeSettings::of(&baker).text()
    };
    let pieces = [
        "(",
        ")",
        "[",
        "]",
        ",",
        ":",
        "cage",
        "roughness",
        "maps",
        "high",
        "Some(",
        "None",
        "NaN",
        "inf",
        "-inf",
        "-1e30",
        "1e30",
        "\"normal\"",
        "true",
        "7",
        "\"",
    ];
    let mut seed: u64 = 0xba4e_5e77;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..2000 {
        let mut text = String::new();
        if next() % 2 == 0 {
            let cut = (next() as usize) % (good.len() + 1);
            text.push_str(good.get(..cut).unwrap_or(&good));
        }
        for _ in 0..(next() % 10) {
            text.push_str(pieces[(next() as usize) % pieces.len()]);
        }
        let mut baker = Baker::default();
        BakeSettings::read(&text).give(&mut baker);
        assert!((0.0..=1.0).contains(&baker.roughness), "{text}");
        assert!(CAGE_RANGE.contains(&baker.cage), "{text}");
    }
    for bad in [
        "(cage: NaN, roughness: inf)",
        "(cage: -5.0, roughness: 9.0)",
        "(cage: 1e30)",
    ] {
        let mut baker = Baker::default();
        BakeSettings::read(bad).give(&mut baker);
        assert!((0.0..=1.0).contains(&baker.roughness), "{bad}");
        assert!(CAGE_RANGE.contains(&baker.cage), "{bad}");
    }
}
