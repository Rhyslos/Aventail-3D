//! The Trim sheet box in the Material mode (decision 0289), used through
//! the real interface: found in the box search off a material's dropped
//! wire and wired into Strip 1 in one undo step; its Attributes saying
//! where each strip lies, and making the project's trim rows match them
//! in one undo step; the strips following the UV Editor's rows; and a
//! trim sheet given to a cube laid once across its texture, whatever the
//! real-world size. No keys or mouse actions of their own.

use super::graph_giving_tests::{page as cube_page, set, wait};
use super::material_mode_tests::{NOISE, box_at, click, drag, empty_spot, graph, page, plug, undo};
use crate::material::export_set::GIVE_TO_SELECTED;
use crate::material::trim_rows::MATCH_LABEL;
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, Registry, SettingValue, Value};
use scene::Command;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Material from height, added after [`page`]'s five.
const MATERIAL: BoxId = BoxId(6);
/// The Trim sheet found in the search.
const TRIM: BoxId = BoxId(7);

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

/// [`page`] with Noise made into a material, and a Trim sheet added by
/// typing "trim" into the box search opened by dropping that material's
/// wire on the canvas.
fn with_a_sheet() -> H {
    let mut harness = page();
    edit(
        &mut harness,
        GraphEdit::AddBox {
            kind: "material_from_height".into(),
            at: Vec2::new(500.0, 0.0),
        },
    );
    edit(
        &mut harness,
        GraphEdit::Connect {
            from: PlugRef::new(NOISE, "height"),
            to: PlugRef::new(MATERIAL, "height"),
        },
    );
    let from = plug(&mut harness, MATERIAL, "material", true);
    let spot = empty_spot(&mut harness);
    drag(&mut harness, from, spot, Modifiers::NONE);
    for c in "trim".chars() {
        harness.event(egui::Event::Text(c.to_string()));
        harness.run();
    }
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness
}

/// Selects box `id` by clicking its title.
fn select(harness: &mut H, id: BoxId) {
    let (made, _) = box_at(harness, id);
    click(
        harness,
        made.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    harness.run();
}

fn trim(harness: &H) -> scene::TrimSheet {
    harness.state().workbench.document().scene().trim().clone()
}

#[test]
fn a_trim_sheet_comes_from_the_search_off_a_material() {
    let mut harness = with_a_sheet();
    let g = graph(&harness);
    assert_eq!(g.get(TRIM).unwrap().kind, "trim_sheet");
    assert_eq!(
        g.wire_into(&PlugRef::new(TRIM, "strip1"))
            .unwrap()
            .from
            .node,
        MATERIAL
    );
    assert!(matches!(
        picture(&mut harness, TRIM),
        Some(Value::Material(_))
    ));
    undo(&mut harness);
    assert_eq!(graph(&harness).box_count(), 6, "one step");
}

#[test]
fn attributes_say_where_the_strips_lie_and_make_them_the_trim_rows() {
    let mut harness = with_a_sheet();
    select(&mut harness, TRIM);
    assert!(
        harness
            .query_by_label_contains("The project has no trim rows yet")
            .is_some()
    );
    assert!(
        harness
            .query_by_label("Strip 1: 0.0% to 100.0% down, cut to the strip")
            .is_some()
    );
    harness.get_by_label(MATCH_LABEL).click();
    harness.run();
    let rows = trim(&harness).rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Strip 1");
    assert_eq!((rows[0].bottom, rows[0].top), (0.0, 1.0));
    // The box now lies on the project's row, and says so.
    harness.run();
    assert!(
        harness
            .query_by_label_contains("On the project's 1 trim rows")
            .is_some()
    );
    assert!(
        harness
            .query_by_label("Strip 1: 0.0% to 100.0% down, on the row Strip 1, cut to the strip")
            .is_some()
    );
    assert!(harness.query_by_label(MATCH_LABEL).is_none());
    // One undo step takes the rows back.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(trim(&harness).rows.is_empty());
    assert_eq!(graph(&harness).box_count(), 7, "the box stays");
}

#[test]
fn the_strips_follow_the_uv_editors_rows() {
    let mut harness = with_a_sheet();
    let before = picture(&mut harness, TRIM).unwrap();
    // Rows made in the UV Editor (decision 0106): the top quarter.
    let sheet = scene::TrimSheet {
        rows: vec![scene::TrimRow {
            name: "Bolts".into(),
            bottom: 0.75,
            top: 1.0,
        }],
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetTrim(sheet))
        .unwrap();
    harness.run();
    let after = picture(&mut harness, TRIM).unwrap();
    assert_ne!(after, before, "made again on the row");
    let m = after.material().unwrap();
    let n = i64::from(m.size());
    // Under the row: the gap's plain colour.
    let gap = m.colour.at(3, n * 3 / 4);
    assert!((gap[0] - 0.10).abs() < 1e-3, "{gap:?}");
    select(&mut harness, TRIM);
    assert!(
        harness
            .query_by_label_contains("on the row Bolts")
            .is_some()
    );
    // Strip 2 has no row.
    edit(
        &mut harness,
        GraphEdit::Connect {
            from: PlugRef::new(MATERIAL, "material"),
            to: PlugRef::new(TRIM, "strip2"),
        },
    );
    assert!(
        harness
            .query_by_label_contains("Strip 2 has no trim row to lie on")
            .is_some()
    );
}

/// Two Colour fills, red and blue, each made a material (with Noise's
/// bumps), into a Trim
/// sheet's Strips 1 and 2 (halves, edge to edge, flat), into Material out
/// at a real-world size of 10 cm.
fn red_and_blue() -> Graph {
    let registry = Registry::standard();
    let mut g = Graph::new("Trim");
    for (kind, x, y) in [
        ("colour_fill", 0.0, 0.0),
        ("colour_fill", 0.0, 300.0),
        ("material_from_height", 300.0, 0.0),
        ("material_from_height", 300.0, 300.0),
        ("trim_sheet", 600.0, 0.0),
        ("material_out", 900.0, 0.0),
        ("noise", 0.0, 600.0),
    ] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, y),
        };
        g.apply(registry, &edit).unwrap();
    }
    let set = |g: &mut Graph, node: u64, key: &str, value: SettingValue| {
        let edit = GraphEdit::SetSetting {
            node: BoxId(node),
            key: key.into(),
            value,
        };
        g.apply(registry, &edit).unwrap();
    };
    set(&mut g, 1, "colour", SettingValue::Colour([1.0, 0.0, 0.0]));
    set(&mut g, 2, "colour", SettingValue::Colour([0.0, 0.0, 1.0]));
    set(&mut g, 5, "lay", SettingValue::Choice(1));
    set(&mut g, 5, "gap", SettingValue::Number(0.0));
    set(&mut g, 5, "depth", SettingValue::Number(0.0));
    set(
        &mut g,
        6,
        matgraph::eval::REAL_SIZE,
        SettingValue::Number(10.0),
    );
    for (from, out, to, input) in [
        (1, "colour", 3, "colour"),
        (2, "colour", 4, "colour"),
        (3, "material", 5, "strip1"),
        (4, "material", 5, "strip2"),
        (5, "material_out", 6, "material"),
        (7, "height", 3, "height"),
        (7, "height", 4, "height"),
    ] {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(BoxId(from), out),
            to: PlugRef::new(BoxId(to), input),
        };
        g.apply(registry, &edit).unwrap();
    }
    g
}

#[test]
fn a_trim_sheet_given_to_a_cube_is_laid_once_across_its_texture() {
    let (mut harness, _) = cube_page(red_and_blue());
    harness.get_by_label(GIVE_TO_SELECTED).click();
    wait(&mut harness);
    let made = set(&harness).unwrap();
    assert_ne!(
        made.layers[0].graph.as_ref().unwrap().made,
        0,
        "made on the cube"
    );
    let paint::Content::Paint(image) = &made.layers[0].content else {
        panic!("a graph fill is paint");
    };
    // Laid by its real-world size (10 cm) it would repeat many times;
    // laid once, every painted texel in the top half is red and in the
    // bottom half blue.
    let (mut red, mut blue) = (0, 0);
    for y in (2..30).chain(34..62) {
        for x in 0..64 {
            let [r, _, b, a] = image.pixel(x, y);
            if a == 0 {
                continue;
            }
            if y < 32 {
                assert!(r > 200 && b < 50, "({x}, {y}): {r} {b}");
                red += 1;
            } else {
                assert!(b > 200 && r < 50, "({x}, {y}): {r} {b}");
                blue += 1;
            }
        }
    }
    assert!(red > 0 && blue > 0, "{red} {blue}");
}
