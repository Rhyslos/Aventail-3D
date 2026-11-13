//! The stylised light boxes in the Material mode (decision 0286), used
//! through the real interface: Toon bands found in the box search off a
//! dropped wire, Painted highlight and Colour steps added from "usually
//! comes next", a box with nothing to find its light from saying why on
//! the canvas, and Toon bands and Colour steps keeping to the project's
//! palettes. No keys or mouse actions of their own.

use super::material_mode_tests::{NOISE, box_at, click, drag, empty_spot, graph, page, plug, undo};
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use matgraph::{BoxId, GraphEdit, GraphId, PlugRef, SettingValue, Value};
use paint::palette::Palette;
use scene::Command;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The boxes added after [`page`]'s five.
const TOON: BoxId = BoxId(6);
const HIGHLIGHT: BoxId = BoxId(7);
const STEPS: BoxId = BoxId(8);

fn edit(harness: &mut H, edit: GraphEdit) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::EditGraph {
            id: GraphId(1),
            edit,
        })
        .unwrap();
    harness.run();
}

fn picture(harness: &mut H, id: BoxId) -> Option<Value> {
    let bench = &mut harness.state_mut().workbench;
    let pictures = &bench.material().pictures;
    assert!(!pictures.waiting());
    pictures.value(id).cloned()
}

fn problem(harness: &mut H, id: BoxId) -> Option<String> {
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    let g = bench.material().current(&document).unwrap().clone();
    bench
        .material()
        .problems(&g)
        .get(&id)
        .map(|r| r.reason.clone())
}

/// Clicks the "usually comes next" button `name` in Attributes for the
/// selected box.
fn add_next(harness: &mut H, name: &str) {
    harness.get_by_label(name).scroll_to_me();
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
}

/// [`page`] with Toon bands found in the search off Noise's height.
fn toon_page() -> H {
    let mut harness = page();
    let from = plug(&mut harness, NOISE, "height", true);
    let spot = empty_spot(&mut harness);
    drag(&mut harness, from, spot, Modifiers::NONE);
    for c in ["t", "o", "o", "n"] {
        harness.event(egui::Event::Text(c.to_string()));
        harness.run();
    }
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness
}

#[test]
fn toon_bands_come_from_the_search_and_light_the_height() {
    let mut harness = toon_page();
    let g = graph(&harness);
    assert_eq!(g.get(TOON).unwrap().kind, "toon_bands");
    assert_eq!(
        g.wire_into(&PlugRef::new(TOON, "height"))
            .unwrap()
            .from
            .node,
        NOISE
    );
    // Nothing to shade: the bands' tones alone, a colour.
    assert!(matches!(
        picture(&mut harness, TOON),
        Some(Value::Colour(_))
    ));
    assert_eq!(problem(&mut harness, TOON), None);
    undo(&mut harness);
    assert_eq!(graph(&harness).box_count(), 5, "one step");
}

#[test]
fn what_comes_next_builds_a_cel_look_and_says_what_is_missing() {
    let mut harness = toon_page();
    let (toon, _) = box_at(&mut harness, TOON);
    click(
        &mut harness,
        toon.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    assert!(harness.query_by_label("Bands").is_some(), "its settings");
    add_next(&mut harness, "Painted highlight");
    let g = graph(&harness);
    assert_eq!(g.get(HIGHLIGHT).unwrap().kind, "painted_highlight");
    let fed = g.wire_into(&PlugRef::new(HIGHLIGHT, "colour")).unwrap();
    assert_eq!(fed.from, PlugRef::new(TOON, "colour_out"));
    // Only a colour in: it can't tell where the edges are, and says how.
    let why = problem(&mut harness, HIGHLIGHT).unwrap();
    assert!(why.contains("Height or Material"), "{why}");
    edit(
        &mut harness,
        GraphEdit::Connect {
            from: PlugRef::new(NOISE, "height"),
            to: PlugRef::new(HIGHLIGHT, "height"),
        },
    );
    assert_eq!(problem(&mut harness, HIGHLIGHT), None);
    // Painted highlight's "usually comes next" offers Colour steps.
    add_next(&mut harness, "Colour steps");
    let g = graph(&harness);
    assert_eq!(g.get(STEPS).unwrap().kind, "colour_steps");
    assert_eq!(
        g.wire_into(&PlugRef::new(STEPS, "colour")).unwrap().from,
        PlugRef::new(HIGHLIGHT, "colour_out")
    );
    // Its picture is the first output it makes: the colour, as only a
    // colour is plugged in.
    assert!(matches!(
        picture(&mut harness, STEPS),
        Some(Value::Colour(_))
    ));
}

#[test]
fn toon_bands_and_colour_steps_keep_to_the_projects_palette() {
    let mut harness = toon_page();
    let swatches = [[30, 30, 60], [120, 140, 90], [240, 230, 200]];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetPalettes(vec![Palette {
            name: "Game".into(),
            colours: swatches.to_vec(),
        }]))
        .unwrap();
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    edit(
        &mut harness,
        GraphEdit::SetSetting {
            node: TOON,
            key: "colours".into(),
            value: SettingValue::Choice(1),
        },
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    let wanted: Vec<[f32; 3]> = swatches
        .iter()
        .map(|c| c.map(|v| f32::from(v) / 255.0))
        .collect();
    let Some(Value::Colour(c)) = picture(&mut harness, TOON) else {
        panic!("Toon bands made no picture");
    };
    assert!(c.pixels().iter().all(|p| wanted.contains(p)));
    // Colour steps snapping the bands to the same palette.
    edit(
        &mut harness,
        GraphEdit::AddBox {
            kind: "colour_steps".into(),
            at: glam::Vec2::new(1200.0, 0.0),
        },
    );
    let new = BoxId(7);
    edit(
        &mut harness,
        GraphEdit::Connect {
            from: PlugRef::new(TOON, "colour_out"),
            to: PlugRef::new(new, "colour"),
        },
    );
    edit(
        &mut harness,
        GraphEdit::SetSetting {
            node: new,
            key: "snap".into(),
            value: SettingValue::Switch(true),
        },
    );
    let Some(Value::Colour(c)) = picture(&mut harness, new) else {
        panic!("Colour steps made no picture");
    };
    assert!(c.pixels().iter().all(|p| wanted.contains(p)));
}
