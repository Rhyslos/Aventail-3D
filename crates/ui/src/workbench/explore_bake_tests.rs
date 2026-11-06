//! Exploratory checks of the UV suite (bake): written by a test pass to look
//! for problems. Each test names what it tries.

#[allow(unused_imports)]
use super::tests;

use super::baker::{PreviewOf, preview_of, run_order, stacked_pieces};
use super::seams_tests::add;
use super::*;
use bake::maps::MapKind;
use egui_kittest::kittest::Queryable;
use glam::{Mat4, Vec2, Vec3};

type H = egui_kittest::Harness<'static, tests::State>;

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("explore-bake-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn files_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn small_texture(harness: &mut H, size: u32) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTexture(scene::TextureSettings {
            size,
            ..Default::default()
        }))
        .unwrap();
}

fn rename(harness: &mut H, id: scene::ObjectId, name: &str) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Rename {
            id,
            name: name.to_string(),
        })
        .unwrap();
}

fn place(harness: &mut H, id: scene::ObjectId, matrix: Mat4) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(
            id,
            scene::Transform::from_matrix(matrix).unwrap(),
        )]))
        .unwrap();
    harness.run();
}

fn centre(mesh: &scene::Mesh) -> Vec3 {
    mesh.vertices().map(|(_, p)| p).sum::<Vec3>() / mesh.vertex_count() as f32
}

#[test]
#[ignore = "BUG: two objects whose names make the same file name (same name, or 'crate low' vs 'crate_low') overwrite each other's maps in a kit bake"]
fn two_pieces_with_the_same_name_get_files_of_their_own() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    rename(&mut harness, a, "crate low");
    rename(&mut harness, b, "crate_low");
    place(&mut harness, b, Mat4::from_translation(Vec3::X * 500.0));
    let dir = folder("same-name");
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([a, b]);
    w.baker.maps = [true, false, false, false, false, false, false];
    w.bake_into(dir.clone());
    w.finish_bake();
    let said = w.baker.last.clone().unwrap_or_default();
    let names = files_in(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(said.contains("Baked 2 maps"), "{said}");
    // Two maps reported: two files should be on disk.
    assert_eq!(names.len(), 2, "said '{said}' but wrote {names:?}");
}

#[test]
fn file_names_from_odd_object_names_are_safe() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    rename(&mut harness, a, "../Kiste \"groß\"/x.y");
    let dir = folder("odd-name");
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([a]);
    w.baker.maps = [false, false, false, false, true, false, false];
    w.baker.godot = true;
    w.bake_into(dir.clone());
    w.finish_bake();
    let names = files_in(&dir);
    let parent_has_strays = files_in(dir.parent().unwrap())
        .iter()
        .any(|n| n.starts_with("Kiste"));
    let tres = names.iter().find(|n| n.ends_with(".tres")).cloned();
    let text = tres
        .map(|t| std::fs::read_to_string(dir.join(t)).unwrap())
        .unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!parent_has_strays);
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names.iter().all(|n| !n.contains('/') && !n.contains('"')));
    assert!(names.iter().any(|n| n.contains("groß")), "{names:?}");
    // Without a normal map ticked: the material names only the ORM.
    assert!(text.contains("orm_texture") && !text.contains("normal_texture"));
}

#[test]
#[ignore = "LIKELY BUG: the Godot .tres never sets ao_enabled, and Godot's BaseMaterial3D only reads the ORM's red (occlusion) channel when ambient occlusion is enabled"]
fn the_godot_material_turns_on_its_occlusion() {
    let text = fileio::bake_file::godot_material(Some("a_normal.png"), Some("a_orm.png"));
    assert!(text.contains("ao_enabled = true"), "{text}");
}

#[test]
fn the_godot_material_is_well_formed() {
    let text = fileio::bake_file::godot_material(
        Some("C:\\Users\\me\\game\\a_normal.png"),
        Some("res://kit/a\"b_orm.png"),
    );
    assert!(text.contains("path=\"C:/Users/me/game/a_normal.png\""));
    assert!(text.contains("path=\"res://kit/ab_orm.png\""));
    // Every quote is balanced on every line.
    for line in text.lines() {
        assert_eq!(line.matches('"').count() % 2, 0, "{line}");
    }
    assert!(text.contains("load_steps=3"));
}

#[test]
fn the_orm_roughness_and_metal_reach_their_extremes() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.baker.godot = true;
    for (rough, metal, want) in [
        (0.0, false, [0u8, 0]),
        (1.0, true, [255, 255]),
        (7.0, false, [255, 0]),
    ] {
        w.baker.roughness = rough;
        w.baker.metal = metal;
        assert_eq!(w.bake_order().unwrap().godot, Some((want[0], want[1])));
    }
    w.baker.roughness = f32::NAN;
    // A broken number doesn't panic; it becomes something.
    let _ = w.bake_order().unwrap().godot;
}

#[test]
fn a_bake_pads_its_pieces_past_their_edges() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 128);
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [false, false, false, false, true, false, false];
    let order = w.bake_order().unwrap();
    let padding = order.padding;
    assert!(padding >= 2);
    let dir = folder("padding");
    let progress = bake::Progress::default();
    let step = std::sync::atomic::AtomicUsize::new(0);
    let written = run_order(&order, &dir, &progress, &step).unwrap();
    let png = fileio::image_file::open_image(&written[0]).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!((png.width, png.height), (128, 128));
    let at = |x: u32, y: u32| {
        let i = ((y * 128 + x) * 4) as usize;
        [
            png.rgba[i],
            png.rgba[i + 1],
            png.rgba[i + 2],
            png.rgba[i + 3],
        ]
    };
    // The bake itself (before padding), to know where the pieces are.
    let surface = paint::PaintSurface::shaded(&order.low, order.low_hardness);
    let mut slots: Vec<u32> = surface.triangles().iter().map(|t| t.slot).collect();
    slots.sort_unstable();
    slots.dedup();
    let source = bake::Source {
        surface: &surface,
        slots: &slots,
        size: 128,
    };
    let raw = bake::maps::bake_map(
        &source,
        None,
        MapKind::Id,
        &order.settings,
        &bake::Progress::default(),
    )
    .unwrap();
    let reach = (padding / 2).max(1) as i32;
    for y in 0..128i32 {
        for x in 0..128i32 {
            if raw.pixel(x as u32, y as u32)[3] == 0 {
                continue;
            }
            for (dx, dy) in [(reach, 0), (-reach, 0), (0, reach), (0, -reach)] {
                let (px, py) = (x + dx, y + dy);
                if !(0..128).contains(&px) || !(0..128).contains(&py) {
                    continue;
                }
                let p = at(px as u32, py as u32);
                assert!(p[..3] != [0, 0, 0], "unpadded at {px},{py} next to a piece");
                assert_eq!(p[3], 255);
            }
        }
    }
}

/// An L-shaped polygon (one UV piece whose box has an empty corner) and a
/// small square laid in that empty corner of the texture.
fn l_and_square() -> scene::Mesh {
    let mut m = scene::Mesh::new();
    let l = [
        (0.0, 0.0),
        (2.0, 0.0),
        (2.0, 1.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 2.0),
    ];
    let lv: Vec<_> = l
        .iter()
        .map(|(x, y)| m.add_vertex(Vec3::new(*x * 50.0, *y * 50.0, 0.0)).unwrap())
        .collect();
    m.add_face(&lv).unwrap();
    let sq = [(1.2, 1.2), (1.8, 1.2), (1.8, 1.8), (1.2, 1.8)];
    let sv: Vec<_> = sq
        .iter()
        .map(|(x, y)| m.add_vertex(Vec3::new(*x * 50.0, *y * 50.0, 80.0)).unwrap())
        .collect();
    m.add_face(&sv).unwrap();
    let uvs: Vec<Vec2> = l
        .iter()
        .chain(sq.iter())
        .map(|(x, y)| Vec2::new(*x * 0.45, *y * 0.45))
        .collect();
    assert!(mesh::uvs::set_uvs(&mut m, uvs));
    m
}

#[test]
#[ignore = "BUG: 'Leave out stacked and mirrored copies' compares UV boxes, so a piece packed in the empty corner of an L-shaped (or ring) piece is left out of the bake though nothing overlaps it"]
fn a_piece_in_another_piece_s_empty_corner_is_not_left_out() {
    let m = l_and_square();
    assert_eq!(mesh::uvs::islands(&m).len(), 2);
    assert!(stacked_pieces(&m).is_empty(), "{:?}", stacked_pieces(&m));
}

/// Two identical cubes in one object, the second sitting on the first,
/// their UVs stacked; the top cube's polygons in material slot 1.
fn stacked_tower() -> (scene::Mesh, usize) {
    let cube = scene::Primitive::Cube(Default::default()).mesh();
    let height = {
        let zs: Vec<f32> = cube.vertices().map(|(_, p)| p.z).collect();
        zs.iter().copied().fold(f32::MIN, f32::max) - zs.iter().copied().fold(f32::MAX, f32::min)
    };
    let mut two = scene::Mesh::new();
    mesh::parts::append(&mut two, &cube, Mat4::IDENTITY);
    let one = two.face_count();
    mesh::parts::append(&mut two, &cube, Mat4::from_translation(Vec3::Z * height));
    (two, one)
}

fn mean_occlusion(surface: &paint::PaintSurface, slots: &[u32]) -> f32 {
    let source = bake::Source {
        surface,
        slots,
        size: 64,
    };
    let image = bake::bake(&[source], bake::BakeKind::Occlusion, &Default::default()).unwrap();
    let mut sum = 0.0;
    let mut n = 0.0;
    for y in 0..64 {
        for x in 0..64 {
            let p = image.pixel(x, y);
            if p[3] > 0 {
                sum += f32::from(p[0]);
                n += 1.0;
            }
        }
    }
    sum / n
}

#[test]
#[ignore = "BUG: copies left out as stacked are taken out of the shape entirely, so they no longer cast occlusion onto the piece that is baked (a stacked tower's contact face bakes open)"]
fn left_out_copies_still_shade_the_piece_baked() {
    let (mut two, one) = stacked_tower();
    let stacked = stacked_pieces(&two);
    assert_eq!(stacked.len(), one, "the top cube is the copy");
    // What the occlusion should be: the bottom cube with the top one
    // still there, casting its shadow.
    let top: Vec<scene::FaceId> = stacked.clone();
    mesh::material::set_material(&mut two, &top, 1);
    let whole = paint::PaintSurface::of(&two);
    let right = mean_occlusion(&whole, &[0]);
    // What the baker does: the top cube deleted first.
    let mut left = two.clone();
    left.delete_faces(&stacked);
    let baked = mean_occlusion(&paint::PaintSurface::of(&left), &[0]);
    assert!(
        (baked - right).abs() < 6.0,
        "with the copy {right:.0}, as baked {baked:.0}"
    );
}

#[test]
#[ignore = "BUG: the painted cage push isn't scaled with the object: on a scaled object the bake reaches far less than the cyan cage drawn on the model"]
fn the_painted_cage_reaches_as_far_as_it_is_drawn_on_a_scaled_object() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let local = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(low)
        .unwrap()
        .mesh;
    let c = centre(&local);
    place(&mut harness, low, Mat4::from_scale(Vec3::splat(4.0)));
    // The high cube 40 units out on every face: inside the drawn cage
    // (about 48 out), outside what the bake reaches (about 31).
    place(
        &mut harness,
        high,
        Mat4::from_translation(c * 4.0)
            * Mat4::from_scale(Vec3::splat(4.8))
            * Mat4::from_translation(-c),
    );
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([low]);
    w.start_cage_brush();
    let points: Vec<(scene::ObjectId, scene::VertexId)> =
        local.vertices().map(|(v, _)| (low, v)).collect();
    for stroke in 1..=10 {
        w.cage_dab(Some(stroke), points.clone());
    }
    w.stop_cage_brush();
    let w = &harness.state().workbench;
    let scene = w.document().scene();
    let painted = scene.evaluated(low).unwrap().mesh;
    // The cage as drawn, in the world: how far out of the low surface.
    let matrix = scene.world_matrix(low);
    let drawn = super::cage::cage_lines(&painted, w.baker.cage)
        .iter()
        .map(|[a, _]| {
            let p = matrix.transform_point3(*a);
            let q = matrix.transform_point3(c);
            (p - q).abs().max_element() - 200.0
        })
        .fold(f32::MAX, f32::min);
    assert!(drawn > 45.0, "drawn {drawn}");
    let mut w = harness.state_mut();
    let w = &mut w.workbench;
    w.baker.high = Some(high);
    let order = w.bake_order().unwrap();
    let (_, problems) =
        preview_of(&order, PreviewOf::Problems, &bake::Progress::default()).unwrap();
    let problems = problems.unwrap();
    // The high cube lies inside the drawn cage: nothing should be missed.
    assert!(
        problems.missed * 20 < problems.texels,
        "drawn cage {drawn:.0} units out, but {problems:?}"
    );
}

#[test]
fn a_painted_cage_pulled_far_in_doesn_t_break_the_bake() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let local = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(low)
        .unwrap()
        .mesh;
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([low]);
    w.start_cage_brush();
    w.cage_brush.as_mut().unwrap().ctrl = true;
    let points: Vec<(scene::ObjectId, scene::VertexId)> =
        local.vertices().map(|(v, _)| (low, v)).collect();
    for stroke in 1..=30 {
        w.cage_dab(Some(stroke), points.clone());
    }
    w.stop_cage_brush();
    w.baker.high = Some(high);
    w.baker.maps = [true, false, false, false, false, false, true];
    let order = w.bake_order().unwrap();
    let dir = folder("pulled-in");
    let written = run_order(
        &order,
        &dir,
        &bake::Progress::default(),
        &std::sync::atomic::AtomicUsize::new(0),
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(written.len(), 2);
}

#[test]
fn the_preview_follows_a_change_of_object() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([a]);
    w.baker.open = true;
    w.baker.preview.on = true;
    w.baker.preview.of = PreviewOf::Map(MapKind::Position);
    w.update_bake_preview(&ctx);
    // Switched mid-run.
    w.document_mut().set_selection([b]);
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    let first = std::sync::Arc::as_ptr(w.viewport().picture().unwrap());
    w.finish_bake_preview(&ctx);
    assert!(!w.bake_preview_running());
    assert_eq!(
        std::sync::Arc::as_ptr(w.viewport().picture().unwrap()),
        first
    );
}

#[test]
#[ignore = "CONFUSING: with the selection cleared, the last bake preview stays painted on the model"]
fn the_preview_goes_when_nothing_is_selected() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    w.baker.open = true;
    w.baker.preview.on = true;
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert!(w.viewport().picture().is_some());
    w.document_mut().clear_selection();
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert!(w.viewport().picture().is_none());
}

#[test]
#[ignore = "CONFUSING: Preview of Problems with 'Bake from: The object itself' shows nothing and says nothing (no hint that it needs a high-detail object)"]
fn problems_without_a_high_object_say_why() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    {
        let w = &mut harness.state_mut().workbench;
        w.baker.open = true;
        w.baker.preview.on = true;
        w.baker.preview.of = PreviewOf::Problems;
        w.update_bake_preview(&ctx);
        w.finish_bake_preview(&ctx);
    }
    harness.run();
    harness.run();
    assert!(
        harness
            .query_all_by_label_contains("high-detail")
            .next()
            .is_some(),
        "no word about needing a high-detail object"
    );
}

#[test]
fn a_second_bake_while_one_runs_is_ignored_and_cancel_says_so() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 128);
    add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let dir = folder("twice");
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [false, true, false, true, false, false, false];
    w.bake_into(dir.clone());
    w.bake_into(dir.join("second"));
    assert!(w.baking_maps());
    w.finish_bake();
    assert!(!dir.join("second").exists());
    let said = w.baker.last.clone().unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(said.contains("Baked 2 maps"), "{said}");
}

#[test]
fn vertex_colours_then_undo_puts_them_back() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [false, false, true, false, false, false, false];
    w.run_bake_vertex_colours();
    assert!(mesh::colours::has_colours(&super::seams_tests::shape(
        &harness
    )));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .undo()
        .expect("an undo step");
    harness.run();
    assert!(!mesh::colours::has_colours(&super::seams_tests::shape(
        &harness
    )));
}

#[test]
fn the_painted_cage_reaches_a_high_shape_beyond_the_plain_cage() {
    // The control for the scaled case: unscaled, 10 strokes of push let
    // the bake reach a high cube 12 units out (the plain cage reaches 3.5).
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let local = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(low)
        .unwrap()
        .mesh;
    let c = centre(&local);
    place(
        &mut harness,
        high,
        Mat4::from_translation(c)
            * Mat4::from_scale(Vec3::splat(1.24))
            * Mat4::from_translation(-c),
    );
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([low]);
    w.baker.high = Some(high);
    let order = w.bake_order().unwrap();
    let (_, before) = preview_of(&order, PreviewOf::Problems, &bake::Progress::default()).unwrap();
    assert_eq!(before.unwrap().missed, before.unwrap().texels);
    w.start_cage_brush();
    let points: Vec<(scene::ObjectId, scene::VertexId)> =
        local.vertices().map(|(v, _)| (low, v)).collect();
    for stroke in 1..=10 {
        w.cage_dab(Some(stroke), points.clone());
    }
    w.stop_cage_brush();
    let order = w.bake_order().unwrap();
    let (_, after) = preview_of(&order, PreviewOf::Problems, &bake::Progress::default()).unwrap();
    let after = after.unwrap();
    assert!(after.missed * 20 < after.texels, "{after:?}");
}

#[test]
#[ignore = "CONFUSING: with the low and its chosen high both selected, the window says 'Onto: 2 objects, each on its own' though only the low is baked"]
fn onto_counts_only_the_objects_baked() {
    let mut harness = tests::fresh();
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    {
        let w = &mut harness.state_mut().workbench;
        w.document_mut().set_selection([low, high]);
        w.baker.high = Some(high);
        w.baker.open = true;
        assert_eq!(w.bake_orders().unwrap().len(), 1);
    }
    harness.run();
    harness.run();
    assert!(
        harness
            .query_all_by_label_contains("2 objects")
            .next()
            .is_none(),
        "the window says two objects are baked onto"
    );
}
