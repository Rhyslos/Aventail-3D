//! Tests for the UV Editor's workings (decision 0096): reading a shape's
//! UVs, picking and whole islands.

use super::*;
use scene::PrimitiveKind;

fn shape(kind: PrimitiveKind) -> UvShape {
    UvShape::of(Arc::new(kind.standard().mesh())).unwrap()
}

#[test]
fn a_cubes_uvs_are_six_islands_all_border() {
    let cube = shape(PrimitiveKind::Cube);
    assert_eq!(cube.islands.len(), 6);
    assert_eq!(cube.polygons.len(), 6);
    // Each side's four edges are drawn once per side: every one borders
    // an island.
    assert_eq!(cube.edges.len(), 24);
    assert!(cube.edges.iter().all(|(_, _, border)| *border));
}

#[test]
fn a_planes_inside_edges_are_drawn_once() {
    let mesh = PrimitiveKind::Plane.standard();
    let plane = UvShape::of(Arc::new(
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 2],
            ..Default::default()
        })
        .mesh(),
    ))
    .unwrap();
    let _ = mesh;
    assert_eq!(plane.islands.len(), 1);
    // 12 edges: 8 round the border, 4 inside.
    assert_eq!(plane.edges.len(), 12);
    assert_eq!(plane.edges.iter().filter(|(_, _, b)| *b).count(), 8);
}

#[test]
fn a_mesh_without_uvs_shows_nothing() {
    let mut mesh = Mesh::new();
    let a = mesh.add_vertex(glam::Vec3::ZERO).unwrap();
    let b = mesh.add_vertex(glam::Vec3::X).unwrap();
    let c = mesh.add_vertex(glam::Vec3::Y).unwrap();
    mesh.add_face(&[a, b, c]).unwrap();
    assert!(UvShape::of(Arc::new(mesh)).is_none());
}

#[test]
fn screen_and_uv_go_back_and_forth() {
    let mut editor = UvEditor::default();
    let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 300.0));
    editor.fit(rect, Vec2::ZERO, Vec2::ONE);
    for uv in [Vec2::ZERO, Vec2::ONE, Vec2::new(0.25, 0.75)] {
        let back = editor.to_uv(rect, editor.to_screen(rect, uv));
        assert!(back.distance(uv) < 1e-4);
    }
    // Up in the texture is up on screen.
    assert!(editor.to_screen(rect, Vec2::Y).y < editor.to_screen(rect, Vec2::ZERO).y);
    // The square fits, with a margin.
    let a = editor.to_screen(rect, Vec2::ZERO);
    let b = editor.to_screen(rect, Vec2::ONE);
    assert!(rect.contains(a) && rect.contains(b));
}

#[test]
fn inside_works_for_any_polygon() {
    // An L, which isn't convex.
    let l = [
        Vec2::new(0.0, 0.0),
        Vec2::new(2.0, 0.0),
        Vec2::new(2.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 2.0),
        Vec2::new(0.0, 2.0),
    ];
    assert!(inside(&l, Vec2::new(0.5, 1.5)));
    assert!(inside(&l, Vec2::new(1.5, 0.5)));
    assert!(!inside(&l, Vec2::new(1.5, 1.5)));
    assert!(!inside(&l, Vec2::new(-0.1, 0.5)));
    // The other way round too.
    let mut reversed = l;
    reversed.reverse();
    assert!(inside(&reversed, Vec2::new(0.5, 1.5)));
}

#[test]
fn segment_distance_measures_to_the_nearest_spot() {
    let (a, b) = (pos2(0.0, 0.0), pos2(10.0, 0.0));
    assert!((segment_distance(pos2(5.0, 3.0), a, b) - 3.0).abs() < 1e-5);
    assert!((segment_distance(pos2(-4.0, 3.0), a, b) - 5.0).abs() < 1e-5);
    assert!((segment_distance(pos2(1.0, 1.0), a, a) - 2f32.sqrt()).abs() < 1e-5);
}

#[test]
fn whole_islands_select_and_let_go_together() {
    let cube = shape(PrimitiveKind::Cube);
    let first = cube.islands[0][0];
    let mut selection = ElementSelection::default();
    whole_islands(
        &mut selection,
        &cube,
        &[Element::Polygon(first)],
        Combine::Replace,
    );
    assert_eq!(
        selection.count(ElementKind::Polygons),
        cube.islands[0].len()
    );
    let second = cube.islands[1][0];
    whole_islands(
        &mut selection,
        &cube,
        &[Element::Polygon(second)],
        Combine::Toggle,
    );
    assert_eq!(
        selection.count(ElementKind::Polygons),
        cube.islands[0].len() + cube.islands[1].len()
    );
    whole_islands(
        &mut selection,
        &cube,
        &[Element::Polygon(first)],
        Combine::Toggle,
    );
    assert_eq!(
        selection.count(ElementKind::Polygons),
        cube.islands[1].len()
    );
}

#[test]
fn the_uv_mode_selects_polygons() {
    assert_eq!(element_kind(Mode::Uv), Some(ElementKind::Polygons));
    assert_eq!(element_kind(Mode::Points), Some(ElementKind::Points));
    assert_eq!(element_kind(Mode::Model), None);
}

#[test]
fn the_stretch_view_colours_and_explains_polygons() {
    let mut mesh = PrimitiveKind::Plane.standard().mesh();
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let even = UvShape::of(Arc::new(mesh.clone())).unwrap();
    let id = ObjectId::from_raw(4);
    let shapes = vec![(id, even)];
    assert_eq!(
        stretch_note(&shapes, None).unwrap(),
        "Stretch: every polygon lies evenly."
    );
    assert_eq!(stretch_color(&shapes[0].1.stretch[0]), theme::STRETCH_EVEN);
    // Squash one polygon's UVs to a third across: stretched, and said so
    // when pointed at.
    mesh::uvs::transform_uvs(
        &mut mesh,
        &faces[..1],
        &[],
        glam::Affine2::from_scale(Vec2::new(1.0 / 3.0, 1.0)),
    );
    let squashed = UvShape::of(Arc::new(mesh)).unwrap();
    let shapes = vec![(id, squashed)];
    let hover = Hit {
        id,
        element: Element::Polygon(faces[0]),
    };
    let note = stretch_note(&shapes, Some(hover)).unwrap();
    assert!(note.starts_with("This polygon is stretched"), "{note}");
    assert!(stretch_note(&shapes, None).unwrap().contains("1 polygon"));
    assert_eq!(stretch_color(&shapes[0].1.stretch[0]), theme::STRETCH_FAR);
    let empty = uv::stretch::Stretch {
        uneven: 1.0,
        size: 1.0,
        flipped: false,
        empty: true,
    };
    assert_eq!(stretch_color(&empty), theme::STRETCH_NO_ROOM);
}
