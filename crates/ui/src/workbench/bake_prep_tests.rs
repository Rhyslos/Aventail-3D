//! Tests for getting objects ready for a bake (decisions 0250, 0251 and
//! 0253): file names of their own, the painted cage scaled with the
//! object, and the stacked copies left out. Regression tests for the
//! problems a test pass of the baker found.

use super::bake_prep::{cage_in_the_world, stacked_pieces, unique_names};
use super::baker::{PreviewOf, preview_of, run_order};
use super::seams_tests::add;
use super::*;
use glam::{Mat4, Vec2, Vec3};

type H = egui_kittest::Harness<'static, tests::State>;

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bake-prep-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn files_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| Some(e.ok()?.file_name().to_string_lossy().into_owned()))
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

/// A new object made of `mesh`, selected.
fn add_mesh(harness: &mut H, mesh: scene::Mesh) -> scene::ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Mesh(
        std::sync::Arc::new(mesh),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    id
}

#[test]
fn names_are_made_unique_ignoring_case() {
    let mut names: Vec<String> = ["crate", "Crate", "crate", "crate_2", "box"]
        .iter()
        .map(ToString::to_string)
        .collect();
    unique_names(&mut names);
    assert_eq!(names, ["crate", "Crate_2", "crate_3", "crate_2_2", "box"]);
    let mut none: Vec<String> = Vec::new();
    unique_names(&mut none);
}

#[test]
fn two_pieces_with_the_same_name_get_files_of_their_own() {
    // Regression: "crate low" and "crate_low" both made crate_low_*.png,
    // the second writing over the first.
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
    assert_eq!(names, ["crate_low_2_normal.png", "crate_low_normal.png"]);
}

#[test]
fn the_count_said_is_of_maps_not_the_godot_files() {
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let dir = folder("count");
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [true, false, false, false, false, false, false];
    w.baker.godot = true;
    w.bake_into(dir.clone());
    w.finish_bake();
    let said = w.baker.last.clone().unwrap_or_default();
    let names = files_in(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(said.contains("Baked 1 maps and a Godot material"), "{said}");
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
fn a_piece_in_another_piece_s_empty_corner_is_not_left_out() {
    // Regression: pieces were compared by their boxes, so the square in
    // the L's empty corner counted as lying on it.
    let m = l_and_square();
    assert_eq!(mesh::uvs::islands(&m).len(), 2);
    assert!(stacked_pieces(&m).is_empty(), "{:?}", stacked_pieces(&m));
}

#[test]
fn a_square_laid_over_the_l_itself_is_left_out() {
    // The other way round: moved onto the L's filled corner, it is.
    let mut m = l_and_square();
    let count = m.face_layout().1.len();
    let mut uvs: Vec<Vec2> = (0..count)
        .map(|k| {
            let f = m.faces().nth(usize::from(k >= 6)).unwrap().0;
            mesh::uvs::Uvs::of(&m)
                .unwrap()
                .corner(&m, f, k % 6)
                .unwrap()
        })
        .collect();
    for uv in &mut uvs[6..] {
        *uv -= Vec2::splat(0.45);
    }
    assert!(mesh::uvs::set_uvs(&mut m, uvs));
    assert_eq!(stacked_pieces(&m).len(), 1);
}

#[test]
fn shapes_with_no_uv_area_leave_nothing_out() {
    let mut m = l_and_square();
    let count = m.face_layout().1.len();
    assert!(mesh::uvs::set_uvs(&mut m, vec![Vec2::splat(0.3); count]));
    assert!(stacked_pieces(&m).is_empty());
    assert!(stacked_pieces(&scene::Mesh::new()).is_empty());
}

/// Two identical cubes in one object, the second sitting on the first,
/// their UVs stacked.
fn stacked_tower() -> (scene::Mesh, usize) {
    let cube = scene::Primitive::Cube(Default::default()).mesh();
    let zs: Vec<f32> = cube.vertices().map(|(_, p)| p.z).collect();
    let height =
        zs.iter().copied().fold(f32::MIN, f32::max) - zs.iter().copied().fold(f32::MAX, f32::min);
    let mut two = scene::Mesh::new();
    mesh::parts::append(&mut two, &cube, Mat4::IDENTITY);
    let one = two.face_count();
    mesh::parts::append(&mut two, &cube, Mat4::from_translation(Vec3::Z * height));
    (two, one)
}

/// The mean occlusion of `order`'s baked texels.
fn mean_occlusion(order: &super::baker::BakeOrder) -> f32 {
    let surface = paint::PaintSurface::shaded(&order.low, order.low_hardness);
    let slots = order.slots(&surface);
    let source = bake::Source {
        surface: &surface,
        slots: &slots,
        size: 64,
    };
    let image = bake::bake(&[source], bake::BakeKind::Occlusion, &Default::default()).unwrap();
    let (mut sum, mut n) = (0.0, 0.0);
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
fn left_out_copies_still_shade_the_piece_baked() {
    // Regression: the copies were taken out of the shape, so the tower's
    // bottom cube baked open where the top one sits on it.
    let (tower, one) = stacked_tower();
    assert_eq!(
        stacked_pieces(&tower).len(),
        one,
        "the top cube is the copy"
    );
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    let mut alone = tower.clone();
    alone.delete_faces(&stacked_pieces(&tower));
    let alone = add_mesh(&mut harness, alone);
    add_mesh(&mut harness, tower.clone());
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [false, true, false, false, false, false, false];
    let order = w.bake_order().unwrap();
    // The copy is still there, in a slot that isn't baked.
    assert_eq!(order.low.face_count(), tower.face_count());
    let slot = order.left_out.expect("a slot for the copies");
    assert_eq!(mesh::material::faces_with_slot(&order.low, slot).len(), one);
    // Shaded as the whole tower: darker than the bottom cube alone.
    let baked = mean_occlusion(&order);
    w.document_mut().set_selection([alone]);
    let open = mean_occlusion(&w.bake_order().unwrap());
    assert!(
        baked + 2.0 < open,
        "with the copy {baked:.0}, alone {open:.0}"
    );
}

#[test]
fn a_painted_cage_is_scaled_along_each_point_s_facing() {
    let cube = scene::Primitive::Cube(Default::default()).mesh();
    assert!(cage_in_the_world(&cube, Mat4::from_scale(Vec3::splat(4.0))).is_none());
    let mut painted = cube.clone();
    let points: Vec<(scene::VertexId, f32)> = cube.vertices().map(|(v, _)| (v, 10.0)).collect();
    mesh::cage::set_push(&mut painted, &points);
    let even = cage_in_the_world(&painted, Mat4::from_scale(Vec3::splat(4.0))).unwrap();
    for (v, _) in cube.vertices() {
        assert!((mesh::cage::push_of(&even, v) - 40.0).abs() < 1e-3);
    }
    // Stretched along X only: a corner faces out diagonally, so it grows
    // by less than the stretch, more than not at all.
    let stretched =
        cage_in_the_world(&painted, Mat4::from_scale(Vec3::new(3.0, 1.0, 1.0))).unwrap();
    let (v, _) = cube.vertices().next().unwrap();
    let push = mesh::cage::push_of(&stretched, v);
    assert!(push > 10.0 && push < 30.0, "{push}");
    // A mirror keeps it pushing out.
    let mirrored =
        cage_in_the_world(&painted, Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))).unwrap();
    assert!((mesh::cage::push_of(&mirrored, v) - 10.0).abs() < 1e-3);
}

#[test]
fn the_painted_cage_reaches_as_far_as_it_is_drawn_on_a_scaled_object() {
    // Regression: the painted push stayed in the object's own units, so
    // on an object scaled 4× the bake reached a quarter as far as the
    // cage drawn on it.
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
    let c = local.vertices().map(|(_, p)| p).sum::<Vec3>() / local.vertex_count() as f32;
    place(&mut harness, low, Mat4::from_scale(Vec3::splat(4.0)));
    // The high cube 40 units out on every face: inside the drawn cage
    // (about 48 out), past where the bake reached before (about 31).
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
    let scene = w.document().scene();
    let painted = scene.evaluated(low).unwrap().mesh;
    let matrix = scene.world_matrix(low);
    let drawn = super::cage::cage_lines(&painted, w.baker.cage)
        .iter()
        .map(|[a, _]| {
            let p = matrix.transform_point3(*a);
            (p - matrix.transform_point3(c)).abs().max_element() - 200.0
        })
        .fold(f32::MAX, f32::min);
    assert!(drawn > 45.0, "drawn {drawn}");
    w.baker.high = Some(high);
    let order = w.bake_order().unwrap();
    let (_, problems) =
        preview_of(&order, PreviewOf::Problems, &bake::Progress::default()).unwrap();
    let problems = problems.unwrap();
    assert!(
        problems.missed * 20 < problems.texels,
        "drawn cage {drawn:.0} units out, but {problems:?}"
    );
}

#[test]
fn a_kit_with_stacked_copies_still_bakes_every_map() {
    // The copies moved to a slot of their own don't stop a bake.
    let (tower, _) = stacked_tower();
    let mut harness = tests::fresh();
    small_texture(&mut harness, 64);
    add_mesh(&mut harness, tower);
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [true, true, true, true, true, true, true];
    let order = w.bake_order().unwrap();
    let dir = folder("tower");
    let written = run_order(
        &order,
        &dir,
        &bake::Progress::default(),
        &std::sync::atomic::AtomicUsize::new(0),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(written.unwrap().len(), 7);
}
