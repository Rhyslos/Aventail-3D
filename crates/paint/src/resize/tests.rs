//! Tests for changing a texture set's size.

use super::*;
use crate::fill::FillBy;
use crate::set::FillRecord;
use crate::surface::PaintSurface;
use mesh::PrimitiveKind;

fn cube() -> PaintSurface {
    PaintSurface::of(&PrimitiveKind::Cube.standard().mesh())
}

fn shape(surface: &PaintSurface) -> Shape<'_> {
    Shape {
        id: 1,
        surface,
        slots: &[0],
    }
}

fn stroke(kind: BrushKind, dabs: Vec<DabRecord>, fills: Vec<FillRecord>) -> Arc<StrokeRecord> {
    Arc::new(StrokeRecord {
        kind,
        color: [200, 30, 30, 255],
        hardness: 1.0,
        flow: 1.0,
        through: false,
        dabs,
        fills,
        tip: crate::tip::Tip::Round,
    })
}

/// A layer painted from `strokes` at `size`, as painting would leave it.
fn painted_layer(strokes: Vec<Arc<StrokeRecord>>, size: u32, s: &PaintSurface) -> Layer {
    let image = repaint(&strokes, size, &shape(s)).unwrap();
    let mut layer = Layer::paint("Paint", image);
    layer.strokes = strokes;
    layer
}

/// How many texels along a row through texel row `y` change sharply
/// (by more than half) from the one before: a crisp edge has one.
fn hard_steps(image: &Image, y: u32) -> (usize, usize) {
    let (mut sharp, mut soft) = (0, 0);
    for x in 1..image.size() {
        let a = image.pixel(x - 1, y)[3] as i32;
        let b = image.pixel(x, y)[3] as i32;
        let d = (a - b).abs();
        if d > 128 {
            sharp += 1;
        } else if d > 10 {
            soft += 1;
        }
    }
    (sharp, soft)
}

#[test]
fn a_layer_painted_with_strokes_is_painted_again_crisp_at_the_new_size() {
    let s = cube();
    // A flat dab and a polygon fill: both kept as strokes.
    let flat = DabRecord::flat(Vec2::new(0.5, 0.5), 0.05, 1.0);
    let front = s.triangles()[0].face;
    let strokes = vec![
        stroke(BrushKind::Paint, vec![flat], Vec::new()),
        stroke(
            BrushKind::Paint,
            Vec::new(),
            vec![FillRecord {
                face: front,
                by: FillBy::Polygon,
            }],
        ),
    ];
    let mut set = TextureSet::new(64).unwrap();
    set.layers.push(painted_layer(strokes.clone(), 64, &s));
    let bigger = resized(&set, 256, &[shape(&s)]).unwrap();
    assert_eq!(bigger.size, 256);
    let again = repaint(&strokes, 256, &shape(&s)).unwrap();
    let got = bigger.layers[0].image().unwrap();
    for y in (0..256).step_by(7) {
        for x in (0..256).step_by(7) {
            assert_eq!(got.pixel(x, y), again.pixel(x, y), "({x}, {y})");
        }
    }
    // The strokes are kept for next time.
    assert_eq!(bigger.layers[0].strokes, strokes);
}

#[test]
fn a_3d_dab_is_painted_again_where_it_was() {
    let s = cube();
    // A dab on the front of the cube, seen from in front.
    let dab = DabRecord {
        at: Vec3::new(0.0, -50.0, 50.0),
        toward: Vec3::NEG_Y,
        radius: 10.0,
        strength: 1.0,
    };
    let strokes = vec![stroke(BrushKind::Paint, vec![dab], Vec::new())];
    let small = repaint(&strokes, 128, &shape(&s)).unwrap();
    let big = repaint(&strokes, 512, &shape(&s)).unwrap();
    let count = |i: &Image| {
        let mut n = 0usize;
        for y in 0..i.size() {
            for x in 0..i.size() {
                n += usize::from(i.pixel(x, y)[3] > 128);
            }
        }
        n
    };
    let (a, b) = (count(&small), count(&big));
    assert!(a > 10, "{a}");
    // Four times the size across: about sixteen times the texels.
    let ratio = b as f32 / a as f32;
    assert!((12.0..20.0).contains(&ratio), "{a} → {b}: {ratio}");
}

#[test]
fn layers_that_strokes_dont_explain_are_resized_as_pictures() {
    let s = cube();
    // An imported picture: no strokes.
    let mut image = Image::new(64).unwrap();
    for y in 0..64 {
        for x in 0..32 {
            image.set_pixel(x, y, [0, 200, 0, 255]);
        }
    }
    let mut set = TextureSet::new(64).unwrap();
    set.layers.push(Layer::paint("Picture", image.clone()));
    // A painted layer changed after its strokes (as by an import on top).
    let flat = DabRecord::flat(Vec2::new(0.5, 0.5), 0.05, 1.0);
    let mut changed = painted_layer(
        vec![stroke(BrushKind::Paint, vec![flat], Vec::new())],
        64,
        &s,
    );
    changed.image_mut().unwrap().set_pixel(3, 3, [1, 2, 3, 255]);
    set.layers.push(changed);
    let bigger = resized(&set, 128, &[shape(&s)]).unwrap();
    let picture = bigger.layers[0].image().unwrap();
    assert_eq!(picture.pixel(10, 64)[1], 200);
    assert_eq!(picture.pixel(100, 64)[3], 0);
    // The changed pixel came along, enlarged.
    assert!(bigger.layers[1].image().unwrap().pixel(6, 6)[3] > 0);
}

#[test]
fn smaller_is_averaged_and_bigger_keeps_edges_sharper_by_repainting() {
    let s = cube();
    // Stripes one texel wide: smaller, they average to grey.
    let mut image = Image::new(64).unwrap();
    for y in 0..64 {
        for x in (0..64).step_by(2) {
            image.set_pixel(x, y, [255, 255, 255, 255]);
        }
    }
    let half = resample(&image, 32).unwrap();
    let [_, _, _, a] = half.pixel(10, 10);
    assert!((100..=155).contains(&a), "{a}");
    // A filled polygon enlarged: repainted, its edge stays one texel
    // sharp; as a picture it would soften over several.
    let front = s.triangles()[0].face;
    let fill = stroke(
        BrushKind::Paint,
        Vec::new(),
        vec![FillRecord {
            face: front,
            by: FillBy::Polygon,
        }],
    );
    let mut set = TextureSet::new(64).unwrap();
    set.layers.push(painted_layer(vec![fill], 64, &s));
    let big = resized(&set, 512, &[shape(&s)]).unwrap();
    let image = big.layers[0].image().unwrap();
    let row = (0..512)
        .find(|&y| (0..512).any(|x| image.pixel(x, y)[3] > 0))
        .unwrap()
        + 20;
    let (sharp, soft) = hard_steps(image, row);
    assert!(sharp >= 2 && soft <= 2, "sharp {sharp}, soft {soft}");
    let blurred = resample(set.layers[0].image().unwrap(), 512).unwrap();
    let (_, soft) = hard_steps(&blurred, row);
    assert!(soft > 4, "a picture enlarged softens: {soft}");
}

#[test]
fn several_objects_or_unrepeatable_strokes_resize_as_pictures() {
    let s = cube();
    let flat = DabRecord::flat(Vec2::new(0.5, 0.5), 0.05, 1.0);
    let blur = vec![stroke(BrushKind::Blur, vec![flat], Vec::new())];
    assert!(repaint(&blur, 64, &shape(&s)).is_none());
    let mut set = TextureSet::new(64).unwrap();
    set.layers.push(painted_layer(
        vec![stroke(BrushKind::Paint, vec![flat], Vec::new())],
        64,
        &s,
    ));
    // Two objects share it: which the dabs belong to is unknown.
    let two = [shape(&s), shape(&s)];
    let resized_two = resized(&set, 128, &two).unwrap();
    assert_eq!(resized_two.size, 128);
    assert!(!resized_two.layers[0].image().unwrap().is_empty());
}
