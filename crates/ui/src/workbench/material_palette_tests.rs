//! Palette map in the Material mode (decision 0281), used through the
//! real interface: the box colours with the project's palettes (the
//! painter's), Attributes lists them with their swatches and choosing one
//! is one undo step, a palette changed in the project changes the box's
//! picture, and a box naming a palette the project hasn't got says why
//! and fixes it with a click. No keys or mouse actions of its own.

use super::material_mode_tests::{NOISE, graph, page, undo};
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec2;
use matgraph::{BoxId, GraphEdit, GraphId, PlugRef, SettingValue, Value};
use paint::palette::Palette;
use scene::Command;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The Palette map added to [`page`]'s graph.
const MAP: BoxId = BoxId(6);

fn palette(name: &str, colours: &[[u8; 3]]) -> Palette {
    Palette {
        name: name.into(),
        colours: colours.to_vec(),
    }
}

fn set_palettes(harness: &mut H, palettes: Vec<Palette>) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetPalettes(palettes))
        .unwrap();
    harness.run();
}

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

/// [`page`] with palettes Game (dark, light) and Moss (greens), and a
/// Palette map fed by Noise, selected.
fn palette_page() -> H {
    let mut harness = page();
    set_palettes(
        &mut harness,
        vec![
            palette("Game", &[[20, 20, 20], [230, 230, 230]]),
            palette("Moss", &[[40, 90, 30], [120, 170, 60]]),
        ],
    );
    edit(
        &mut harness,
        GraphEdit::AddBox {
            kind: "palette_map".into(),
            at: Vec2::new(900.0, 520.0),
        },
    );
    edit(
        &mut harness,
        GraphEdit::Connect {
            from: PlugRef::new(NOISE, "height"),
            to: PlugRef::new(MAP, "in"),
        },
    );
    harness.state_mut().workbench.material().canvas.selected = [MAP].into();
    harness.run();
    harness
        .get_by_label_contains(crate::material::WHAT_IT_DOES)
        .scroll_to_me();
    harness.run();
    harness
}

/// The Palette map's live picture's colours, once worked out.
fn picture_colours(harness: &mut H) -> Vec<[f32; 3]> {
    let bench = &mut harness.state_mut().workbench;
    let pictures = &bench.material().pictures;
    assert!(!pictures.waiting());
    let Some(Value::Colour(c)) = pictures.value(MAP) else {
        panic!("the Palette map made no picture");
    };
    let mut colours: Vec<[f32; 3]> = c.pixels().to_vec();
    colours.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    colours.dedup();
    colours
}

fn byte(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| f32::from(v) / 255.0)
}

/// Whether a list (combo box) shows a value containing `text`.
fn list_showing(harness: &H, text: &str) -> bool {
    harness
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .any(|n| n.accesskit_node().value().is_some_and(|v| v.contains(text)))
}

/// Opens the list showing a value containing `text`.
fn open_list(harness: &mut H, text: &str) {
    harness
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains(text)))
        .unwrap()
        .click();
    harness.run();
}

fn chosen(harness: &H) -> Option<SettingValue> {
    graph(harness).get(MAP).unwrap().settings.get("palette")
}

#[test]
fn a_new_palette_map_colours_with_the_projects_first_palette() {
    let mut harness = palette_page();
    assert_eq!(
        picture_colours(&mut harness),
        [byte([20, 20, 20]), byte([230, 230, 230])],
        "only the first palette's swatches"
    );
    assert!(
        list_showing(&harness, "First palette (Game)"),
        "Attributes names the palette it follows"
    );
}

#[test]
fn choosing_a_palette_in_attributes_is_one_undo_step() {
    let mut harness = palette_page();
    let steps = harness.state().workbench.document().undo_steps();
    open_list(&mut harness, "First palette (Game)");
    harness.get_by_label("Moss").click();
    harness.run();
    assert_eq!(chosen(&harness), Some(SettingValue::Palette("Moss".into())));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert_eq!(
        picture_colours(&mut harness),
        [byte([40, 90, 30]), byte([120, 170, 60])]
    );
    undo(&mut harness);
    assert_eq!(chosen(&harness), None);
}

#[test]
fn a_palette_changed_in_the_project_changes_the_box() {
    let mut harness = palette_page();
    let before = picture_colours(&mut harness);
    // As the painter's colour picker does: one palette's colours change.
    set_palettes(
        &mut harness,
        vec![
            palette("Game", &[[200, 0, 0], [0, 0, 200]]),
            palette("Moss", &[[40, 90, 30], [120, 170, 60]]),
        ],
    );
    let after = picture_colours(&mut harness);
    assert_ne!(before, after);
    assert_eq!(after, [byte([0, 0, 200]), byte([200, 0, 0])]);
    undo(&mut harness);
    assert_eq!(picture_colours(&mut harness), before);
}

#[test]
fn a_missing_palette_says_why_and_a_fix_chooses_another() {
    let mut harness = palette_page();
    edit(
        &mut harness,
        GraphEdit::SetSetting {
            node: MAP,
            key: "palette".into(),
            value: SettingValue::Palette("Lava".into()),
        },
    );
    {
        let bench = &mut harness.state_mut().workbench;
        let document = bench.document.clone();
        let g = bench.material().current(&document).unwrap().clone();
        let problems = bench.material().problems(&g);
        assert!(problems[&MAP].reason.contains("Lava"));
    }
    assert!(list_showing(&harness, "Lava (not in this project)"));
    harness.get_by_label("Use the palette Moss").scroll_to_me();
    harness.run();
    harness.get_by_label("Use the palette Moss").click();
    harness.run();
    assert_eq!(chosen(&harness), Some(SettingValue::Palette("Moss".into())));
}

#[test]
fn a_project_without_palettes_is_offered_the_starters() {
    let mut harness = palette_page();
    set_palettes(&mut harness, Vec::new());
    let fix = harness.get_by_label("Add the starter palettes to this project");
    fix.scroll_to_me();
    harness.run();
    harness
        .get_by_label("Add the starter palettes to this project")
        .click();
    harness.run();
    let held = harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .palettes()
        .len();
    assert_eq!(held, paint::palette::starters().len());
    // The first starter's swatches, light to dark in its list: the box
    // works now.
    assert!(!picture_colours(&mut harness).is_empty());
}
