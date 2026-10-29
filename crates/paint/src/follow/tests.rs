//! Tests for paint following the model.

use super::*;
use crate::surface::PaintTriangle;
use crate::{Layer, Mask};

/// A square polygon `face` of side 10 at height `z`, cut into two
/// triangles, laid on the texture as the square from `uv` to `uv + side`.
fn square(face: u32, z: f32, uv: Vec2, side: f32) -> Vec<PaintTriangle> {
    let p = [
        Vec3::new(0.0, 0.0, z),
        Vec3::new(10.0, 0.0, z),
        Vec3::new(10.0, 10.0, z),
        Vec3::new(0.0, 10.0, z),
    ];
    let q = [
        uv,
        uv + Vec2::new(side, 0.0),
        uv + Vec2::splat(side),
        uv + Vec2::new(0.0, side),
    ];
    [[0, 1, 2], [0, 2, 3]]
        .into_iter()
        .map(|[a, b, c]| PaintTriangle {
            points: [p[a], p[b], p[c]],
            uvs: [q[a], q[b], q[c]],
            normal: Vec3::Z,
            slot: 0,
            face,
            corners: [a, b, c].map(|i| face * 4 + i as u32),
        })
        .collect()
}

fn surface(squares: &[Vec<PaintTriangle>]) -> PaintSurface {
    PaintSurface::from_triangles(squares.concat())
}

/// The texel in the middle of UV square `uv` .. `uv + side` on a 128 px
/// texture.
fn middle(uv: Vec2, side: f32) -> (u32, u32) {
    let c = uv + Vec2::splat(side / 2.0);
    ((c.x * 128.0) as u32, ((1.0 - c.y) * 128.0) as u32)
}

/// A set of one paint layer with UV square `uv` .. `uv + side` filled in
/// `rgba`, and another square filled in blue.
fn painted(uv: Vec2, side: f32, rgba: [u8; 4], other: Vec2) -> TextureSet {
    let mut set = TextureSet::new(128).unwrap();
    let mut image = Image::new(128).unwrap();
    for (corner, colour) in [(uv, rgba), (other, [0, 0, 255, 255])] {
        let (x0, y1) = ((corner.x * 128.0) as u32, ((1.0 - corner.y) * 128.0) as u32);
        let n = (side * 128.0) as u32;
        for y in y1 - n..y1 {
            for x in x0..x0 + n {
                image.set_pixel(x, y, colour);
            }
        }
    }
    set.layers.push(Layer::paint("Paint", image));
    set
}

const RED: [u8; 4] = [255, 0, 0, 255];
const A: Vec2 = Vec2::new(0.1, 0.1);
const B: Vec2 = Vec2::new(0.6, 0.6);
const OTHER: Vec2 = Vec2::new(0.1, 0.6);

#[test]
fn paint_moves_with_a_polygon_whose_uvs_moved() {
    let before = surface(&[square(7, 0.0, A, 0.2), square(8, 5.0, OTHER, 0.2)]);
    let after = surface(&[square(7, 0.0, B, 0.2), square(8, 5.0, OTHER, 0.2)]);
    let set = painted(A, 0.2, RED, OTHER);
    let shape = |s| Shape {
        id: 1,
        surface: s,
        slots: &[0],
    };
    let moved = follow(&set, &[shape(&before)], &[shape(&after)]).expect("UVs moved");
    let image = moved.layers[0].image().unwrap();
    let (x, y) = middle(B, 0.2);
    assert_eq!(
        image.pixel(x, y),
        RED,
        "the paint is where the polygon went"
    );
    // The unchanged polygon's paint is exactly as it was.
    let old = set.layers[0].image().unwrap();
    for y in 0..128 {
        for x in 0..128 {
            if old.pixel(x, y) == [0, 0, 255, 255] {
                assert_eq!(image.pixel(x, y), old.pixel(x, y));
            }
        }
    }
    // Nothing changed: nothing to do.
    assert!(follow(&set, &[shape(&before)], &[shape(&before)]).is_none());
}

#[test]
fn a_new_polygon_takes_the_paint_nearest_it() {
    let before = surface(&[square(7, 0.0, A, 0.2)]);
    // Polygon 7 is gone; polygon 9 sits just above where it was.
    let after = surface(&[square(9, 0.5, B, 0.2)]);
    let set = painted(A, 0.2, RED, OTHER);
    let shape = |s| Shape {
        id: 1,
        surface: s,
        slots: &[0],
    };
    let moved = follow(&set, &[shape(&before)], &[shape(&after)]).unwrap();
    let (x, y) = middle(B, 0.2);
    assert_eq!(moved.layers[0].image().unwrap().pixel(x, y), RED);
}

#[test]
fn masks_and_layers_in_groups_move_too_and_fills_stay() {
    let before = surface(&[square(7, 0.0, A, 0.2)]);
    let after = surface(&[square(7, 0.0, B, 0.2)]);
    let mut set = painted(A, 0.2, RED, OTHER);
    let mut mask = Mask::new(128, true).unwrap();
    mask.image = set.layers[0].image().unwrap().clone();
    set.layers[0].mask = Some(mask);
    let inner = set.layers.remove(0);
    set.layers.push(Layer::group("Group", vec![inner]));
    set.layers.push(Layer::fill("Fill", [1, 2, 3, 4]));
    let shape = |s| Shape {
        id: 1,
        surface: s,
        slots: &[0],
    };
    let moved = follow(&set, &[shape(&before)], &[shape(&after)]).unwrap();
    let (x, y) = middle(B, 0.2);
    let inner = &moved.layers[0].children().unwrap()[0];
    assert_eq!(inner.image().unwrap().pixel(x, y), RED);
    assert_eq!(inner.mask.as_ref().unwrap().image.pixel(x, y), RED);
    assert_eq!(moved.layers[1], set.layers[1]);
}

#[test]
fn other_objects_and_slots_are_left_alone() {
    let before = surface(&[square(7, 0.0, A, 0.2)]);
    let after = surface(&[square(7, 0.0, B, 0.2)]);
    let set = painted(A, 0.2, RED, OTHER);
    // Another object id: no before to take paint from.
    let a = Shape {
        id: 1,
        surface: &before,
        slots: &[0],
    };
    let b = Shape {
        id: 2,
        surface: &after,
        slots: &[0],
    };
    assert!(follow(&set, &[a], &[b]).is_none());
    // Another slot: not this texture set's.
    let b = Shape {
        id: 1,
        surface: &after,
        slots: &[3],
    };
    assert!(follow(&set, &[a], &[b]).is_none());
}

#[test]
fn the_layout_key_follows_uvs_not_points() {
    let one = surface(&[square(7, 0.0, A, 0.2)]);
    let higher = surface(&[square(7, 9.0, A, 0.2)]);
    let moved = surface(&[square(7, 0.0, B, 0.2)]);
    let key = |s| {
        layout_key(&[Shape {
            id: 1,
            surface: s,
            slots: &[0],
        }])
    };
    assert_eq!(key(&one), key(&higher));
    assert_ne!(key(&one), key(&moved));
}

#[test]
fn the_nearest_point_is_found_on_the_right_triangle() {
    let s = surface(&[square(7, 0.0, A, 0.2), square(8, 50.0, B, 0.2)]);
    let (i, w) = s.closest(Vec3::new(2.0, 3.0, 40.0), &[0]).unwrap();
    assert_eq!(s.triangles()[i].face, 8);
    let t = &s.triangles()[i];
    let at = t.points[0] * w[0] + t.points[1] * w[1] + t.points[2] * w[2];
    assert!(at.distance(Vec3::new(2.0, 3.0, 50.0)) < 1e-3, "{at}");
    assert!(s.closest(Vec3::ZERO, &[5]).is_none());
}

#[test]
fn a_polygon_packed_smaller_averages_its_old_texels() {
    // Fine black and white stripes, one texel each, on a polygon made
    // half the size on the texture: grey, not stripes picked at random.
    let before = surface(&[square(7, 0.0, A, 0.25)]);
    let after = surface(&[square(7, 0.0, B, 0.125)]);
    let mut set = TextureSet::new(128).unwrap();
    let mut image = Image::new(128).unwrap();
    for y in 0..128 {
        for x in 0..128 {
            let v = if x % 2 == 0 { 0 } else { 255 };
            image.set_pixel(x, y, [v, v, v, 255]);
        }
    }
    set.layers.push(Layer::paint("Stripes", image));
    let shape = |s| Shape {
        id: 1,
        surface: s,
        slots: &[0],
    };
    let moved = follow(&set, &[shape(&before)], &[shape(&after)]).unwrap();
    let image = moved.layers[0].image().unwrap();
    let (x, y) = middle(B, 0.125);
    for dx in 0..4 {
        let [v, ..] = image.pixel(x + dx, y);
        assert!((90..=165).contains(&v), "grey, not a stripe: {v}");
    }
}

#[test]
fn a_new_polygon_with_an_old_ones_number_takes_the_nearest_paint() {
    // Subdivide makes the shape afresh, numbering its polygons from 0
    // again: a new polygon 7 has nothing to do with the old polygon 7.
    let before = surface(&[square(7, 0.0, A, 0.2), square(8, 5.0, OTHER, 0.2)]);
    let mut renumbered = square(7, 5.0, B, 0.2);
    for t in &mut renumbered {
        t.corners = t.corners.map(|c| c + 100);
    }
    let after = surface(&[renumbered]);
    let set = painted(A, 0.2, RED, OTHER);
    let shape = |s| Shape {
        id: 1,
        surface: s,
        slots: &[0],
    };
    let moved = follow(&set, &[shape(&before)], &[shape(&after)]).unwrap();
    let (x, y) = middle(B, 0.2);
    assert_eq!(
        moved.layers[0].image().unwrap().pixel(x, y),
        [0, 0, 255, 255],
        "it lies where polygon 8 was: blue, not polygon 7's red"
    );
}
