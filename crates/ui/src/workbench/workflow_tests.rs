//! Workflow tests: whole stories, as a newcomer would work through them
//! (Phase 9, "a safety net for quality").
//!
//! The other test files each check one tool or one panel. These check
//! that the pieces work *together*, in the order a person really uses
//! them: make a shape, change its size, cut into it, undo and redo, save
//! and open it again, unwrap it, and so on. Each test is one story, read
//! top to bottom, with a short note on every step, and it checks what the
//! person would see along the way: what is selected, how many shapes
//! there are, the message in the status bar, and the names of the undo
//! steps.
//!
//! The stories drive the real interface wherever they can: key presses,
//! clicks and drags in the viewport, menus by their labels, typing into
//! the Attributes panel. The document is only read to check results.
//! Where something can't be reached without a real window (the system's
//! file dialog), the test plays the app's part and says so.

use super::modelling_tests::{H, drag, drag_by, note, on_screen, press};
use super::tests::{empty, fresh, main_menu, uv_group};
use super::*;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec3;
use scene::{ElementKind, ObjectId, Primitive};

// ------------------------------------------------------------ helpers ---

/// A left click at `at`, holding `keys`, as a person would: move there,
/// press, let go.
fn click(harness: &mut H, at: Pos2, keys: Modifiers) {
    harness.event(Event::ModifiersChanged(keys));
    harness.event(Event::PointerMoved(at));
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: keys,
        });
        harness.run();
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

/// Clicks into the Attributes field called `label`, replaces its text
/// with `text` and presses Enter (the field sits right of its label, and
/// both carry the name).
fn type_into(harness: &mut H, label: &str, text: &str) {
    let at = harness
        .get_all_by_label(label)
        .max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .unwrap_or_else(|| panic!("no field {label}"))
        .rect()
        .center();
    click(harness, at, Modifiers::NONE);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

/// Ctrl + `key`.
fn ctrl(harness: &mut H, key: Key) {
    press(harness, Modifiers::COMMAND, key);
}

/// The first selected object.
fn selected(harness: &H) -> ObjectId {
    *harness
        .state()
        .workbench
        .document()
        .selection()
        .first()
        .expect("an object is selected")
}

/// The first selected object's shape, as modelled (without a mirror).
fn shape_of_selected(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    doc.scene().evaluated(selected(harness)).unwrap().mesh
}

/// The first selected object's settings, when it is a primitive.
fn primitive_of_selected(harness: &H) -> Primitive {
    let doc = harness.state().workbench.document();
    match doc.scene().object(selected(harness)).unwrap().kind {
        scene::ObjectKind::Primitive(p) => p,
        ref other => panic!("not a primitive: {other:?}"),
    }
}

/// How many of `kind` are selected on the first selected object.
fn selected_count(harness: &H, kind: ElementKind) -> usize {
    let doc = harness.state().workbench.document();
    doc.elements(selected(harness)).map_or(0, |s| s.count(kind))
}

/// The name of the step Ctrl + Z would take back now.
fn undo_label(harness: &H) -> String {
    harness
        .state()
        .workbench
        .document()
        .undo_label()
        .unwrap_or_default()
        .to_string()
}

fn object_count(harness: &H) -> usize {
    harness.state().workbench.document().scene().len()
}

/// Whether the Attributes panel says how many edits are recorded.
fn edit_list_says(harness: &H, edits: usize) -> bool {
    let text = match edits {
        1 => "1 edit recorded on top of the shape".to_string(),
        n => format!("{n} edits recorded on top of the shape"),
    };
    harness.query_by_label_contains(&text).is_some()
}

/// The middle of a polygon.
fn centre_of(mesh: &scene::Mesh, face: scene::FaceId) -> Vec3 {
    let corners = mesh.face(face).unwrap();
    corners
        .iter()
        .map(|v| mesh.position(*v).unwrap())
        .sum::<Vec3>()
        / corners.len() as f32
}

/// The polygons facing straight up whose middle is at `height`.
fn top_faces(mesh: &scene::Mesh, height: f32) -> Vec<scene::FaceId> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).z > 0.99 && (centre_of(mesh, *f).z - height).abs() < 0.1)
        .collect()
}

// ------------------------------------------------------------ stories ---

/// Model a crate: a box, sized in the Attributes panel, with a recess
/// for its lid cut into the top and its outer edges rounded off. Then
/// every edit is taken back one by one, and done again.
#[test]
fn model_a_crate_then_undo_and_redo_every_edit() {
    let mut harness = empty();

    // B opens the shape list on the cube; Enter creates it, selected.
    press(&mut harness, Modifiers::NONE, Key::B);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(object_count(&harness), 1);
    assert_eq!(undo_label(&harness), "Create cube");
    let id = selected(&harness);

    // Size it in the Attributes panel: 1.2 m wide, 0.8 m deep, 0.6 m
    // high (lengths are typed in metres).
    type_into(&mut harness, "Width", "1.2");
    type_into(&mut harness, "Depth", "0.8");
    type_into(&mut harness, "Height", "0.6");
    let Primitive::Cube(params) = primitive_of_selected(&harness) else {
        panic!("a cube");
    };
    assert_eq!(params.size, Vec3::new(120.0, 80.0, 60.0));
    assert_eq!(undo_label(&harness), "Change cube settings");
    let mesh = shape_of_selected(&harness);
    assert!((mesh.bounds().unwrap().max.z - 60.0).abs() < 1e-3);

    // 3 for the Polygons mode, and a click on the middle of the top
    // selects the top polygon.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    assert_eq!(
        harness.state().workbench.mode(),
        crate::palette::Mode::Polygons
    );
    let top_middle = on_screen(&harness, Vec3::new(0.0, 0.0, 60.0));
    click(&mut harness, top_middle, Modifiers::NONE);
    assert_eq!(selected_count(&harness, ElementKind::Polygons), 1);
    let top = shape_of_selected(&harness);
    let chosen = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .unwrap()
        .polygons()
        .next()
        .unwrap();
    assert!(top.face_normal(chosen).z > 0.99, "the top polygon");

    // I for Inset: a drag from near the top's left side towards its
    // middle shrinks a smaller polygon inside it, for the lid's rim.
    press(&mut harness, Modifiers::NONE, Key::I);
    let from = on_screen(&harness, Vec3::new(-45.0, 0.0, 60.0));
    let towards = (top_middle - from).normalized() * 30.0;
    drag_by(&mut harness, from, towards);
    assert_eq!(undo_label(&harness), "Inset 1 polygon");
    assert!(
        note(&harness).starts_with("Inset 1 polygon"),
        "{}",
        note(&harness)
    );
    assert_eq!(shape_of_selected(&harness).face_count(), 6 + 4);
    assert!(edit_list_says(&harness, 1), "the edit list shows the inset");

    // D for Extrude: a drag down and left pushes the inner polygon into
    // the box, making the recess.
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, -60.0);
    assert_eq!(undo_label(&harness), "Extrude 1 polygon");
    let recessed = shape_of_selected(&harness);
    assert_eq!(recessed.face_count(), 10 + 4);
    let floor = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .unwrap()
        .polygons()
        .next()
        .unwrap();
    let floor_height = centre_of(&recessed, floor).z;
    assert!(floor_height < 59.0, "pushed in: {floor_height}");
    assert!(
        (recessed.bounds().unwrap().max.z - 60.0).abs() < 1e-3,
        "the rim stays at the top"
    );
    assert!(edit_list_says(&harness, 2), "the edit list shows both");

    // 2 for the Edges mode; click the four outer edges round the top
    // (Ctrl + click adds to the selection).
    press(&mut harness, Modifiers::NONE, Key::Num2);
    let outer = [
        Vec3::new(60.0, 0.0, 60.0),
        Vec3::new(0.0, -40.0, 60.0),
        Vec3::new(-60.0, 0.0, 60.0),
        Vec3::new(0.0, 40.0, 60.0),
    ];
    for (k, middle) in outer.iter().enumerate() {
        let at = on_screen(&harness, *middle);
        let keys = if k == 0 {
            Modifiers::NONE
        } else {
            Modifiers::COMMAND
        };
        click(&mut harness, at, keys);
        assert_eq!(selected_count(&harness, ElementKind::Edges), k + 1);
    }

    // V for Bevel: a drag from the right edge outwards, away from it,
    // rounds all four off.
    press(&mut harness, Modifiers::NONE, Key::V);
    assert!(
        note(&harness).starts_with("Bevel: drag away"),
        "{}",
        note(&harness)
    );
    let from = on_screen(&harness, outer[0]);
    let outwards = (from - top_middle).normalized() * 50.0;
    drag_by(&mut harness, from, outwards);
    assert_eq!(undo_label(&harness), "Bevel 4 edges");
    let finished = harness.state().workbench.document().scene().clone();
    let finished_faces = shape_of_selected(&harness).face_count();
    assert!(finished_faces > 14, "the bevel adds polygons");
    assert!(edit_list_says(&harness, 3), "the edit list shows all three");

    // Ctrl + Z, again and again, until nothing is left: the status bar
    // names each step as it is taken back.
    let mut undone = Vec::new();
    while harness.state().workbench.document().undo_steps() > 0 {
        ctrl(&mut harness, Key::Z);
        let said = note(&harness);
        let what = said
            .strip_prefix("Undone: ")
            .unwrap_or_else(|| panic!("the status bar names the step: {said}"))
            .trim_end_matches('.')
            .to_string();
        undone.push(what);
        assert!(undone.len() < 50, "undo never ends: {undone:?}");
    }
    assert_eq!(object_count(&harness), 0, "back to an empty scene");
    // The edits came back off in the opposite order to how they were
    // made (selection steps come in between).
    let edits: Vec<&str> = undone
        .iter()
        .map(String::as_str)
        .filter(|s| !s.starts_with("Select") && !s.starts_with("Unselect"))
        .collect();
    assert_eq!(
        edits,
        [
            "Bevel 4 edges",
            "Extrude 1 polygon",
            "Inset 1 polygon",
            "Change cube settings",
            "Change cube settings",
            "Change cube settings",
            "Create cube",
        ],
        "every step: {undone:?}"
    );

    // Ctrl + Y does it all again, and the crate is exactly as it was.
    while harness.state().workbench.document().redo_steps() > 0 {
        ctrl(&mut harness, Key::Y);
        assert!(note(&harness).starts_with("Redone: "), "{}", note(&harness));
    }
    assert_eq!(*harness.state().workbench.document().scene(), finished);
    let doc = harness.state().workbench.document();
    assert_eq!(
        doc.scene().evaluated(id).unwrap().mesh.face_count(),
        finished_faces
    );
    // Redoing the cube's creation selects it again, as it was.
    assert_eq!(selected(&harness), id);
    assert!(edit_list_says(&harness, 3));
}

/// A cylinder's flat end made into even triangles: inset the cap, then
/// weld the middle polygon down to a point, so every polygon on the top
/// becomes a triangle meeting at the centre.
#[test]
fn turn_a_cylinder_cap_into_even_triangles() {
    let mut harness = empty();

    // The palette's cylinder button: a cylinder, selected.
    harness.get_by_label("Create cylinder").click();
    harness.run();
    assert_eq!(object_count(&harness), 1);
    assert_eq!(undo_label(&harness), "Create cylinder");
    // Its top is one flat polygon with a corner for each of 16 sides.
    let before = shape_of_selected(&harness);
    let cap = top_faces(&before, 100.0);
    assert_eq!(cap.len(), 1);
    assert_eq!(before.face(cap[0]).unwrap().len(), 16);

    // 3 for Polygons, click the middle of the top to select the cap.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let middle = on_screen(&harness, Vec3::new(0.0, 0.0, 100.0));
    click(&mut harness, middle, Modifiers::NONE);
    assert_eq!(selected_count(&harness, ElementKind::Polygons), 1);

    // I, and a drag from near the rim towards the middle: a ring of
    // quads round a smaller cap.
    press(&mut harness, Modifiers::NONE, Key::I);
    let from = on_screen(&harness, Vec3::new(-45.0, 0.0, 100.0));
    drag_by(&mut harness, from, (middle - from).normalized() * 25.0);
    assert_eq!(undo_label(&harness), "Inset 1 polygon");
    assert_eq!(top_faces(&shape_of_selected(&harness), 100.0).len(), 17);

    // M welds the selected (inner) polygon to its middle: one point.
    press(&mut harness, Modifiers::NONE, Key::M);
    assert_eq!(undo_label(&harness), "Weld 1 polygon");
    let after = shape_of_selected(&harness);
    let top: Vec<scene::FaceId> = after
        .faces()
        .map(|(f, _)| f)
        .filter(|f| after.face_normal(*f).z > 0.99)
        .collect();
    assert_eq!(top.len(), 16, "one triangle per side");
    for face in &top {
        assert_eq!(
            after.face(*face).unwrap().len(),
            3,
            "every top face is a triangle"
        );
    }
    // They all meet at the middle of the top.
    let centre = after
        .vertices()
        .find(|(_, p)| p.distance(Vec3::new(0.0, 0.0, 100.0)) < 1e-2)
        .map(|(v, _)| v)
        .expect("a point in the middle");
    assert!(
        top.iter()
            .all(|f| after.face(*f).unwrap().contains(&centre))
    );
    // And the cylinder is still closed and healthy.
    let report = mesh::MeshReport::new(&after);
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
}

/// Symmetry: switch it on, pull a corner out on one side and see the
/// other side follow, then make the mirrored half real with Apply mirror.
#[test]
fn model_with_symmetry_then_apply_the_mirror() {
    let mut harness = fresh();

    // Click the cube to select it.
    let front = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    click(&mut harness, front, Modifiers::NONE);
    let id = selected(&harness);

    // The palette's Symmetry button: a live mirror across X.
    harness.get_by_label("Symmetry").click();
    harness.run();
    assert!(
        note(&harness).starts_with("Symmetry on, across X"),
        "{}",
        note(&harness)
    );
    assert!(harness.state().workbench.selected_mirror().is_some());

    // 1 for Points; click the top front right corner.
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let at = on_screen(&harness, corner);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(selected_count(&harness, ElementKind::Points), 1);

    // W for Move, and drag the gizmo's X arrow to the right.
    press(&mut harness, Modifiers::NONE, Key::W);
    let gizmo = harness
        .state()
        .workbench
        .viewport()
        .gizmo()
        .expect("a gizmo");
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let camera = *harness.state().workbench.viewport().camera();
    let length = gizmo.length(&camera, size);
    let arrow = on_screen(&harness, gizmo.origin + Vec3::X * length * 0.8);
    let right =
        (on_screen(&harness, Vec3::X * 100.0) - on_screen(&harness, Vec3::ZERO)).normalized();
    drag_by(&mut harness, arrow, right * 60.0);
    assert_eq!(undo_label(&harness), "Move 1 point");
    let moved = shape_of_selected(&harness)
        .vertices()
        .map(|(_, p)| p)
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    assert!(moved.x > 55.0, "pulled out to the right: {moved}");

    // The mirror followed: as shown, the left side has the same corner
    // pulled out the other way.
    let shown = harness
        .state()
        .workbench
        .document()
        .scene()
        .shown_shape(id)
        .unwrap();
    let twin = Vec3::new(-moved.x, moved.y, moved.z);
    assert!(shown.vertices().any(|(_, p)| p.distance(twin) < 1e-2));

    // The Symmetry menu (the arrow beside the button) → Apply mirror.
    harness
        .get_by_label(crate::palette::SYMMETRY_MENU_LABEL)
        .click();
    harness.run();
    harness
        .get_by_label(crate::palette::APPLY_MIRROR_LABEL)
        .click();
    harness.run();
    assert_eq!(undo_label(&harness), "Apply mirror");
    assert!(
        note(&harness).starts_with("Mirror applied"),
        "{}",
        note(&harness)
    );
    // Symmetry is off, and the shape itself now has both halves.
    assert!(harness.state().workbench.selected_mirror().is_none());
    let whole = shape_of_selected(&harness);
    assert!(whole.vertices().any(|(_, p)| p.distance(moved) < 1e-2));
    assert!(whole.vertices().any(|(_, p)| p.distance(twin) < 1e-2));
    assert_eq!(whole.face_count(), shown.face_count());
}

/// Unwrap and check: lay the cube's surface out flat for texturing, then
/// ask whether anything would spoil a bake.
#[test]
fn unwrap_pack_and_check_for_baking() {
    let mut harness = fresh();

    // Select the cube with a click.
    let front = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    click(&mut harness, front, Modifiers::NONE);
    let id = selected(&harness);

    // The UV Edit tab at the top right: the UV Editor beside the view,
    // in the UV mode.
    harness.get_by_label("UV Edit").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::UvEdit
    );
    assert_eq!(harness.state().workbench.mode(), crate::palette::Mode::Uv);
    assert!(
        harness
            .query_by_label(crate::uv_editor::UV_EDITOR_LABEL)
            .is_some()
    );

    // UV → Seams → Suggest seams: the app picks where to cut.
    uv_group(&mut harness, UV_SEAMS_MENU);
    harness.get_by_label(seams::SUGGEST_SEAMS_LABEL).click();
    harness.run();
    let shape = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    assert!(
        !mesh::uvs::seams(&shape).is_empty(),
        "seams suggested: {}",
        note(&harness)
    );

    // UV → Lay out → Unwrap: the pieces lie flat along the seams.
    uv_group(&mut harness, UV_LAY_OUT_MENU);
    harness.get_by_label("Unwrap").click();
    harness.run();
    assert!(
        undo_label(&harness).starts_with("Unwrap"),
        "{}",
        undo_label(&harness)
    );

    // UV → Lay out → Pack: the pieces fill the texture square.
    uv_group(&mut harness, UV_LAY_OUT_MENU);
    harness.get_by_label("Pack").click();
    harness.run();
    assert!(
        undo_label(&harness).starts_with("Pack"),
        "{}",
        undo_label(&harness)
    );
    let shape = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    let uvs = mesh::uvs::Uvs::of(&shape).expect("the cube has UVs");
    for (face, corners) in shape.faces() {
        for k in 0..corners.len() {
            let uv = uvs.corner(&shape, face, k).unwrap();
            assert!(
                (-1e-3..=1.001).contains(&uv.x) && (-1e-3..=1.001).contains(&uv.y),
                "inside the texture square: {uv}"
            );
        }
    }

    // UV → Check for baking…: nothing to fix.
    main_menu(&harness, "UV").click();
    harness.run();
    harness.get_by_label("Check for baking…").click();
    harness.run();
    assert!(
        harness
            .query_by_label("Nothing found that would spoil a bake.")
            .is_some()
    );
}

/// Save and open again: the shape and its edits come back exactly.
///
/// The system's save and open dialogs (and the file writing, which the
/// app does, not the interface) can't be driven here, so the test plays
/// the app's part: when the interface asks to save or open, it does what
/// the app does (`app/src/project.rs`), with a file in a temporary folder.
#[test]
fn save_a_project_and_open_it_again() {
    let mut harness = fresh();

    // Select the cube, 3 for Polygons, click the top and push it up with
    // Extrude (D and a drag up and to the right).
    let front = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    click(&mut harness, front, Modifiers::NONE);
    let id = selected(&harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let top = on_screen(&harness, Vec3::new(0.0, 0.0, 100.0));
    click(&mut harness, top, Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, 80.0);
    assert_eq!(undo_label(&harness), "Extrude 1 polygon");
    assert!(harness.state().workbench.has_unsaved_changes());
    let made = harness.state().workbench.document().scene().clone();
    let made_shape = shape_of_selected(&harness);

    // Ctrl + S: the first time, the app is asked where to save.
    harness.state_mut().actions.clear();
    ctrl(&mut harness, Key::S);
    assert_eq!(
        harness.state().actions,
        [WorkbenchAction::Save { path: None }]
    );
    // (The app's part: the person picks a file, and the app writes it.)
    let folder = std::env::temp_dir().join(format!("ui-workflow-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("crate.avtl");
    fileio::project::save(&path, harness.state().workbench.document()).unwrap();
    harness.state_mut().workbench.saved(path.clone());
    harness.run();
    assert_eq!(harness.state().workbench.note(), Some("Saved crate.avtl."));
    assert!(!harness.state().workbench.has_unsaved_changes());
    assert!(harness.state().workbench.title().starts_with("crate.avtl"));

    // File → New starts again with an empty scene (nothing to ask: the
    // work is saved).
    main_menu(&harness, "File").click();
    harness.run();
    harness
        .get_all_by_label_contains("New")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .expect("File → New")
        .click();
    harness.run();
    assert_eq!(object_count(&harness), 0);

    // File → Open…: the app is asked which file to open.
    harness.state_mut().actions.clear();
    main_menu(&harness, "File").click();
    harness.run();
    harness
        .get_all_by_label_contains("Open…")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .expect("File → Open…")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        [WorkbenchAction::Open { path: None }]
    );
    // (The app's part: the person picks the file, and the app reads it.)
    let opened = fileio::project::open(&path).unwrap();
    harness.state_mut().workbench.opened(opened, path.clone());
    harness.run();
    let _ = std::fs::remove_dir_all(&folder);

    // The same shape with the same edit, ready to change again.
    assert_eq!(harness.state().workbench.note(), Some("Opened crate.avtl."));
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().len(), 1);
    assert_eq!(doc.scene().object(id), made.object(id), "same object");
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(*shape, *made_shape, "same shape");
    assert_eq!(doc.scene().object(id).unwrap().stack.operation_count(), 1);
    assert_eq!(doc.undo_steps(), 0, "a fresh history");
}

/// Find a command: Space, type part of a name, Enter runs it.
#[test]
fn find_and_run_a_command_by_name() {
    let mut harness = fresh();

    // Space opens the search; typing "sphere" finds Create sphere.
    press(&mut harness, Modifiers::NONE, Key::Space);
    assert!(harness.state().workbench.is_search_open());
    harness
        .get_by_label(crate::command_search::SEARCH_LABEL)
        .type_text("sphere");
    harness.run();
    // Enter runs it: a sphere beside the cube, and the search closes.
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(!harness.state().workbench.is_search_open());
    assert_eq!(object_count(&harness), 2);
    assert_eq!(undo_label(&harness), "Create sphere");

    // Again for a mode: "points mode" switches to the Points mode.
    press(&mut harness, Modifiers::NONE, Key::Space);
    harness
        .get_by_label(crate::command_search::SEARCH_LABEL)
        .type_text("points mode");
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(
        harness.state().workbench.mode(),
        crate::palette::Mode::Points
    );
}

/// New shapes: the More list (Alt + B) and the Create menu, and a gear
/// whose teeth are changed in the Attributes panel.
#[test]
fn create_more_shapes_and_change_a_gears_teeth() {
    let mut harness = empty();
    let more = crate::shape_picker::ShapeList::More.kinds();

    // Alt + B opens the More list on its first shape; B moves to the
    // second; Enter creates it.
    harness.key_press_modifiers(Modifiers::ALT, Key::B);
    harness.run();
    assert!(harness.query_by_label("More shapes").is_some());
    harness.key_press(Key::B);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(object_count(&harness), 1);
    assert_eq!(
        crate::shape_picker::Shape::Primitive(primitive_of_selected(&harness).kind()),
        more[1]
    );
    assert_eq!(
        undo_label(&harness),
        format!("Create {}", more[1].name().to_lowercase())
    );

    // Create → Gear: a gear, selected, with 12 teeth.
    main_menu(&harness, "Create").click();
    harness.run();
    harness.get_by_label("Gear").click();
    harness.run();
    assert_eq!(object_count(&harness), 2);
    let Primitive::Gear(gear) = primitive_of_selected(&harness) else {
        panic!("a gear");
    };
    assert_eq!(gear.teeth, 12);
    let polygons = shape_of_selected(&harness).face_count();

    // Teeth in the Attributes panel: 20.
    type_into(&mut harness, "Teeth", "20");
    let Primitive::Gear(gear) = primitive_of_selected(&harness) else {
        panic!("a gear");
    };
    assert_eq!(gear.teeth, 20);
    assert_eq!(undo_label(&harness), "Change gear settings");
    assert!(
        shape_of_selected(&harness).face_count() > polygons,
        "more teeth, more polygons"
    );
    // Ctrl + Z puts the 12 back.
    ctrl(&mut harness, Key::Z);
    let Primitive::Gear(gear) = primitive_of_selected(&harness) else {
        panic!("a gear");
    };
    assert_eq!(gear.teeth, 12);
}
