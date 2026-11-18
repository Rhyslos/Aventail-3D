//! Tests of the mask and symmetry (decision 0310).

use super::*;
use crate::mask::{self, FULL};
use crate::tests::{evaluated, grid};
use glam::{Mat4, Vec3};
use mesh::{Mesh, PrimitiveKind, VertexId};
use std::sync::Arc;

fn dab(kind: BrushKind, centre: Vec3, radius: f32) -> Dab {
    Dab {
        kind,
        centre,
        radius,
        strength: 1.0,
        falloff: Falloff::Smooth,
        invert: false,
    }
}

fn sphere() -> Arc<Mesh> {
    Arc::new(PrimitiveKind::Sphere.standard().mesh())
}

/// The point of `mesh` nearest `p`.
fn nearest(mesh: &Mesh, p: Vec3) -> VertexId {
    mesh.vertices()
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
        .unwrap()
        .0
}

/// The standard sphere's middle (it sits on the ground).
const MIDDLE: Vec3 = Vec3::new(0.0, 0.0, 50.0);

/// Where a line from `from` (measured from the sphere's middle) towards
/// the middle (a little off, as a camera's never runs exactly along an
/// axis) meets the live shape.
fn hit(live: &LiveSculpt, from: Vec3) -> Vec3 {
    let way = -from + Vec3::new(0.013, 0.017, 0.011) * from.length();
    live.hit(MIDDLE + from, way)
        .expect("the line meets the shape")
        .point
}

/// A mask with the points of `mesh` for which `inside` holds fully
/// masked.
fn mask_where(mesh: &Mesh, inside: impl Fn(Vec3) -> bool + Sync) -> Mask {
    mask::set_where(mesh, &Mask::zero(mesh.point_slot_count()), FULL, inside)
}

/// The largest distance between a point's place and its partner's place
/// reflected across X = 0, over a shape whose points are symmetric.
fn asymmetry(input: &Mesh, now: &Mesh) -> f32 {
    let places: Vec<Vec3> = input
        .point_slots()
        .map(|p| p.unwrap_or(Vec3::NAN))
        .collect();
    let pairs = symmetry::mirror_pairs(&places, 0, true, 1e-3).expect("a symmetric shape");
    pairs
        .into_iter()
        .map(|(i, j)| {
            let a = now.position(VertexId::from_raw(i as u32)).unwrap();
            let b = now.position(VertexId::from_raw(j as u32)).unwrap();
            a.distance(b * Vec3::new(-1.0, 1.0, 1.0))
        })
        .fold(0.0, f32::max)
}

#[test]
fn the_mask_brush_masks_by_its_falloff_and_ctrl_takes_it_away() {
    let input = Arc::new(grid(20));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    assert!(!live.has_mask());
    assert!(live.mask_values().is_none());
    let changed = live.dab(&dab(BrushKind::Mask, Vec3::ZERO, 3.0));
    assert!(changed > 0);
    // The shape doesn't move; the middle is fully masked, the rim free.
    assert_eq!(live.mesh(), &*input);
    let middle = nearest(&input, Vec3::ZERO).get() as usize;
    let far = nearest(&input, Vec3::new(8.0, 8.0, 0.0)).get() as usize;
    assert_eq!(live.mask().get(middle), FULL);
    assert_eq!(live.mask().get(far), 0);
    assert_eq!(live.mask_values().unwrap()[middle], FULL);
    let s = live.finish().expect("a mask stroke is an undo step");
    assert_eq!(s.mask().get(middle), FULL);
    assert!(s.moves().is_zero());
    // Ctrl takes it away.
    live.dab(&Dab {
        invert: true,
        ..dab(BrushKind::Mask, Vec3::ZERO, 3.0)
    });
    assert_eq!(live.mask().get(middle), 0);
    // Esc takes the stroke back; undo brings the sculpt's mask back.
    live.cancel();
    assert_eq!(live.mask().get(middle), FULL);
    live.follow(&Sculpt::new(&input));
    assert!(!live.has_mask());
}

#[test]
fn a_fully_masked_shape_doesnt_move_under_any_brush_or_the_gizmo() {
    let input = Arc::new(grid(6));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let all = Mask::from_values(&vec![FULL; input.point_slot_count()]);
    live.set_mask(all).unwrap();
    for kind in BrushKind::ALL {
        if kind == BrushKind::Mask {
            continue;
        }
        for invert in [false, true] {
            let d = Dab {
                invert,
                ..dab(kind, Vec3::new(0.3, 0.2, 0.0), 2.5)
            };
            live.dab(&d);
            // A grabbing brush's second dab moves what it holds.
            live.dab(&Dab {
                centre: Vec3::new(1.0, 0.5, 1.0),
                ..d
            });
            assert_eq!(live.mesh(), &*input, "{kind:?}");
            live.finish();
        }
    }
    live.begin_move();
    assert_eq!(live.move_unmasked(Mat4::from_translation(Vec3::Z)), 0);
    assert_eq!(live.mesh(), &*input);
    assert_eq!(live.unmasked_middle(), None);
}

#[test]
fn a_partly_masked_point_moves_partly() {
    let input = Arc::new(grid(20));
    let free = LiveSculpt::new(Arc::clone(&input), None);
    let mut half = LiveSculpt::new(Arc::clone(&input), None);
    let mut free = free;
    half.set_mask(Mask::from_values(&vec![128; input.point_slot_count()]));
    let d = dab(BrushKind::Draw, Vec3::ZERO, 3.0);
    free.dab(&d);
    half.dab(&d);
    let v = nearest(&input, Vec3::ZERO);
    let (a, b) = (
        free.mesh().position(v).unwrap().z,
        half.mesh().position(v).unwrap().z,
    );
    let share = 1.0 - 128.0 / 255.0;
    assert!((b - a * share).abs() < 1e-5, "{a} {b}");
    let s = half.finish().unwrap();
    assert_eq!(&evaluated(&input, &s), half.mesh());
}

#[test]
fn the_gizmo_moves_the_unmasked_part_fading_through_partly_masked_points() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    // The left half masked, the column at x = 0 half masked.
    let values: Vec<u8> = input
        .point_slots()
        .map(|p| match p {
            Some(p) if p.x < -0.1 => FULL,
            Some(p) if p.x.abs() < 0.1 => 128,
            _ => 0,
        })
        .collect();
    live.set_mask(Mask::from_values(&values)).unwrap();
    let middle = live.unmasked_middle().unwrap();
    assert!(middle.x > 0.5, "{middle}");
    live.begin_move();
    live.move_unmasked(Mat4::from_translation(Vec3::new(0.0, 0.0, 2.0)));
    // Moving again goes from where the move began, not from the last.
    live.move_unmasked(Mat4::from_translation(Vec3::new(0.0, 0.0, 4.0)));
    for (v, p) in input.vertices() {
        let z = live.mesh().position(v).unwrap().z;
        let want = if p.x < -0.1 {
            0.0
        } else if p.x.abs() < 0.1 {
            4.0 * (1.0 - 128.0 / 255.0)
        } else {
            4.0
        };
        assert!((z - want).abs() < 1e-4, "{p} {z}");
    }
    let s = live.finish().unwrap();
    assert_eq!(&evaluated(&input, &s), live.mesh());
    assert_eq!(s.mask().get(0), FULL, "the mask stays");
}

#[test]
fn grow_then_shrink_gives_the_mask_back_and_invert_turns_it_over() {
    let input = grid(20);
    let square = mask_where(&input, |p| p.x.abs() < 3.5 && p.y.abs() < 3.5);
    let grown = mask::grow(&input, &square);
    assert!(
        grown.values().iter().filter(|v| **v > 0).count()
            > square.values().iter().filter(|v| **v > 0).count()
    );
    assert_eq!(mask::shrink(&input, &grown), square);
    let inverted = mask::invert(&square);
    for (a, b) in square.values().into_iter().zip(inverted.values()) {
        assert_eq!(u16::from(a) + u16::from(b), 255);
    }
    assert_eq!(mask::invert(&inverted), square);
    // Clearing is an empty mask: nothing held.
    assert!(Mask::zero(5).is_zero());
}

#[test]
fn soften_blurs_the_edge_and_sharpen_hardens_it() {
    let input = grid(10);
    let square = mask_where(&input, |p| p.x < 0.0);
    let soft = mask::soften(&input, &square);
    let values = soft.values();
    assert!(values.iter().any(|v| *v > 0 && *v < FULL), "a soft edge");
    let sharp = mask::sharpen(&soft);
    let partly = |m: &Mask| m.values().iter().filter(|v| **v > 0 && **v < FULL).count();
    assert!(partly(&sharp) <= partly(&soft));
    assert_eq!(mask::sharpen(&square), square, "a hard mask stays");
}

#[test]
fn masks_from_the_edges_the_facing_and_the_creases() {
    let flat = grid(4);
    let edges = mask::from_edges(&flat);
    for (v, p) in flat.vertices() {
        let border = p.x.abs() > 1.9 || p.y.abs() > 1.9;
        assert_eq!(edges.get(v.get() as usize) == FULL, border, "{p}");
    }
    let ball = sphere();
    assert!(
        mask::from_edges(&ball).is_zero(),
        "a closed shape has no open edges"
    );
    let up = mask::from_facing(&ball, Vec3::Z);
    let top = nearest(&ball, Vec3::new(0.0, 0.0, 1000.0)).get() as usize;
    let bottom = nearest(&ball, Vec3::new(0.0, 0.0, -1000.0)).get() as usize;
    assert_eq!(up.get(top), FULL);
    assert_eq!(up.get(bottom), 0);
    assert!(mask::from_facing(&ball, Vec3::ZERO).is_zero());
    // A dent in a flat grid is a crease; the flat parts aren't.
    let mut dented = grid(10);
    let middle = nearest(&dented, Vec3::ZERO);
    dented.set_position(middle, Vec3::new(0.0, 0.0, -0.5));
    let cavity = mask::from_cavity(&dented);
    assert_eq!(cavity.get(middle.get() as usize), FULL);
    let flat_point = nearest(&dented, Vec3::new(3.0, 3.0, 0.0)).get() as usize;
    assert_eq!(cavity.get(flat_point), 0);
}

#[test]
fn the_mask_is_kept_fitted_and_counted_with_the_sculpt() {
    let input = grid(4);
    let s = Sculpt::new(&input);
    let half = mask_where(&input, |p| p.x < 0.0);
    let masked = s.with_mask(half.clone()).unwrap();
    assert_ne!(masked, s);
    assert_eq!(masked.mask(), &half);
    assert!(s.with_mask(Mask::zero(3)).is_none());
    assert_eq!(masked.memory_apart(&s), half.memory_size());
    assert_eq!(s.memory_apart(&masked), 0);
    // A finer shape takes each point's nearest mask value.
    let finer = grid(8);
    let (fitted, found_again) = masked.fitted_to(&finer);
    assert!(found_again);
    for (v, p) in finer.vertices() {
        if p.x < -0.6 {
            assert_eq!(fitted.mask().get(v.get() as usize), FULL, "{p}");
        }
        if p.x > 0.6 {
            assert_eq!(fitted.mask().get(v.get() as usize), 0, "{p}");
        }
    }
    // Moves added later keep the mask.
    let moved = masked.with_moves(Moves::zero(masked.len())).unwrap();
    assert_eq!(moved.mask(), &half);
}

#[test]
fn a_mirrored_stroke_on_a_symmetric_shape_gives_a_symmetric_result() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.set_symmetry(Symmetry {
        axes: [true, false, false],
        radial: None,
    });
    assert_eq!(live.copies().len(), 2);
    let radius = 40.0;
    // Off to one side, and across the middle where the copies overlap.
    for kind in [
        BrushKind::Draw,
        BrushKind::ClayStrips,
        BrushKind::Inflate,
        BrushKind::Smooth,
    ] {
        for c in [Vec3::new(60.0, -20.0, 30.0), Vec3::new(5.0, -90.0, 10.0)] {
            let at = hit(&live, c * 3.0);
            live.dab(&dab(kind, at, radius));
        }
        live.finish();
    }
    // A grab, held on both sides.
    let at = hit(&live, Vec3::new(300.0, 0.0, 0.0));
    live.dab(&dab(BrushKind::Grab, at, radius));
    live.dab(&dab(
        BrushKind::Grab,
        at + Vec3::new(20.0, 0.0, 10.0),
        radius,
    ));
    let s = live.finish().unwrap();
    assert!(
        asymmetry(&input, live.mesh()) < 1e-3,
        "{}",
        asymmetry(&input, live.mesh())
    );
    // The grab pulled the other side out too.
    let left = nearest(&input, Vec3::new(-1000.0, 0.0, 50.0));
    assert!(live.mesh().position(left).unwrap().x < input.position(left).unwrap().x - 5.0);
    assert_eq!(&evaluated(&input, &s), live.mesh());
}

#[test]
fn a_mirrored_alpha_lands_as_its_mirror_image() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.set_symmetry(Symmetry {
        axes: [true, false, false],
        radial: None,
    });
    // White on its left half only: lopsided, so a copy that weren't
    // turned over would land the wrong way round.
    let values: Vec<u8> = (0..64 * 64)
        .map(|i| if i % 64 < 24 { 255 } else { 0 })
        .collect();
    let alpha = Alpha::new(64, 64, values, "lopsided").unwrap();
    let at = hit(&live, Vec3::new(300.0, -300.0, 0.0));
    live.dab_with(
        &dab(BrushKind::Draw, at, 50.0),
        Some(&DabAlpha {
            alpha: &alpha,
            up: Vec3::Z,
            angle: 0.3,
            follow: false,
        }),
    );
    assert!(asymmetry(&input, live.mesh()) < 1e-3);
}

#[test]
fn radial_symmetry_repeats_the_dab_round_its_axis() {
    for count in [2, 64] {
        let input = sphere();
        let mut live = LiveSculpt::new(Arc::clone(&input), None);
        live.set_symmetry(Symmetry {
            axes: [false; 3],
            radial: Some((2, count)),
        });
        let at = hit(&live, Vec3::new(300.0, 0.0, 50.0));
        assert!(live.dab(&dab(BrushKind::Draw, at, 30.0)) > 0);
        // The copy turned half way round moved as much as the dab.
        let here = nearest(&input, at);
        let there = nearest(&input, Vec3::new(-at.x, -at.y, at.z));
        let moved = |v| {
            live.mesh()
                .position(v)
                .unwrap()
                .distance(input.position(v).unwrap())
        };
        assert!((moved(here) - moved(there)).abs() < 1e-3, "{count}");
        assert!(moved(there) > 0.1, "{count}");
    }
}

#[test]
fn symmetry_also_paints_the_mask_on_the_other_side() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.set_symmetry(Symmetry {
        axes: [true, false, false],
        radial: None,
    });
    let at = hit(&live, Vec3::new(300.0, 0.0, 0.0));
    live.dab(&dab(BrushKind::Mask, at, 30.0));
    let left = nearest(&input, Vec3::new(-1000.0, 0.0, 50.0)).get() as usize;
    let right = nearest(&input, Vec3::new(1000.0, 0.0, 50.0)).get() as usize;
    assert!(live.mask().get(right) > 128);
    assert!(live.mask().get(left).abs_diff(live.mask().get(right)) <= 1);
}

#[test]
fn make_symmetrical_copies_one_side_and_explains_a_lopsided_shape() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let at = hit(&live, Vec3::new(300.0, 100.0, 0.0));
    live.dab(&dab(BrushKind::Draw, at, 40.0));
    live.dab(&dab(BrushKind::Mask, at, 20.0));
    live.finish();
    assert!(asymmetry(&input, live.mesh()) > 1.0);
    let s = live.make_symmetrical(0, true).unwrap().expect("a change");
    assert!(asymmetry(&input, live.mesh()) < 1e-3);
    assert_eq!(&evaluated(&input, &s), live.mesh());
    let here = nearest(&input, at).get() as usize;
    let other = nearest(&input, Vec3::new(-at.x, at.y, at.z)).get() as usize;
    assert!(s.mask().get(here) > 128);
    assert_eq!(
        s.mask().get(other),
        s.mask().get(here),
        "the mask is copied too"
    );
    // Already symmetrical: nothing to do.
    assert!(live.make_symmetrical(0, true).unwrap().is_none());
    // A shape whose points aren't symmetric still mirrors strokes in
    // space, but can't be made symmetrical.
    let mut lopsided = grid(4);
    lopsided.set_position(VertexId::from_raw(0), Vec3::new(-2.3, -2.0, 0.0));
    let lopsided = Arc::new(lopsided);
    let mut live = LiveSculpt::new(Arc::clone(&lopsided), None);
    live.set_symmetry(Symmetry {
        axes: [true, false, false],
        radial: None,
    });
    assert!(live.dab(&dab(BrushKind::Draw, Vec3::new(1.0, 0.0, 0.0), 1.5)) > 0);
    live.finish();
    assert!(matches!(
        live.make_symmetrical(0, true),
        Err(symmetry::NotSymmetric::Unmatched(_))
    ));
}

#[test]
fn whole_mask_changes_are_one_sculpt_each() {
    let input = Arc::new(grid(6));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let half = mask_where(&input, |p| p.y > 0.0);
    let s = live.set_mask(half.clone()).unwrap();
    assert_eq!(s.mask(), &half);
    assert!(live.set_mask(half).is_none(), "nothing changed");
    assert!(live.set_mask(Mask::zero(2)).is_none(), "the wrong length");
    let cleared = live.set_mask(Mask::zero(input.point_slot_count())).unwrap();
    assert!(cleared.mask().is_zero());
}
