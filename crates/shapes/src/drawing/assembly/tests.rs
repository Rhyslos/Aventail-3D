//! Tests for assemblies: places kept apart from shapes, linked copies
//! sharing one shape, plain copies, flipping, groups and the order, and
//! a fuzz run of them all.

use super::*;
use crate::path::{Knot, Path};
use crate::{Basic, PieceKind};
use glam::Vec2;

fn leaf() -> Path {
    Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.3)),
            Knot::smooth(Vec2::new(0.4, 0.4), Vec2::new(0.0, 0.05)),
            Knot::corner(Vec2::new(0.5, 0.5)),
            Knot::smooth(Vec2::new(0.6, 0.4), Vec2::new(0.0, -0.05)),
        ],
        true,
    )
}

/// One leaf, and two linked copies of it moved apart.
fn twig() -> (Drawing, [PieceId; 3]) {
    let d = Drawing::new()
        .with_piece("Leaf", leaf(), PieceKind::Add)
        .unwrap();
    let first = d.pieces()[0].id;
    let copies = d.copied(&[first]);
    let (d, a) = d
        .with_pasted(&copies, true, &Change::moved(Vec2::new(0.2, 0.0)))
        .unwrap();
    let (d, b) = d
        .with_pasted(&copies, true, &Change::moved(Vec2::new(-0.2, 0.1)))
        .unwrap();
    (d, [first, a[0], b[0]])
}

fn ids(d: &Drawing) -> Vec<u32> {
    d.pieces().iter().map(|p| p.id.0).collect()
}

#[test]
fn a_linked_copy_shares_its_shape_and_keeps_its_place() {
    let (d, [a, b, c]) = twig();
    assert_eq!(d.linked_count(a), 3);
    assert_eq!(d.piece(b).unwrap().path, d.piece(a).unwrap().path);
    let middle = |d: &Drawing, id| d.piece(id).unwrap().placed().middle();
    assert!(middle(&d, b).distance(middle(&d, a) + Vec2::new(0.2, 0.0)) < 1e-6);
    // Turn one copy, then drag a point of another: every copy's shape
    // changes, each staying where it is (and turned as it was).
    let d = d
        .with_change(&[c], &Change::turned(middle(&d, c), 0.5))
        .unwrap();
    let placed = d.piece(b).unwrap().placed();
    let moved = placed.with_knot(0, Knot::corner(placed.knots()[0].at + Vec2::new(0.0, -0.1)));
    let d = d.with_path(b, moved.clone()).unwrap();
    assert_eq!(d.piece(b).unwrap().placed(), moved);
    let shape = d.piece(a).unwrap().path.clone();
    assert!(shape.knots()[0].at.distance(Vec2::new(0.5, 0.2)).abs() < 1e-5);
    assert_eq!(d.piece(c).unwrap().path, shape);
    let turned = d.piece(c).unwrap().place;
    assert!((turned.matrix.x_axis.y - 0.5f32.sin()).abs() < 1e-5);
    // Changing what one does changes them all.
    let cut = d.with_kind(c, PieceKind::Cut).unwrap();
    assert!(cut.pieces().iter().all(|p| p.kind == PieceKind::Cut));
    // Each keeps its own colour and height.
    let coloured = d.with_colour(b, [0.2, 0.8, 0.1]).unwrap();
    assert_eq!(
        coloured.piece(a).unwrap().colour,
        crate::drawing::STANDARD_COLOUR
    );
    assert_eq!(coloured.piece(b).unwrap().colour, [0.2, 0.8, 0.1]);
    let low = d.with_height(c, 7.0).unwrap();
    assert_eq!(low.piece(c).unwrap().height, 1.0);
    assert_eq!(
        low.with_height(c, -1.0).unwrap().piece(c).unwrap().height,
        0.0
    );
    assert_eq!(d.with_height(c, f32::NAN), Err(Problem::OutOfPlace));
    assert_eq!(
        d.with_colour(c, [f32::INFINITY, 0.0, 0.0]),
        Err(Problem::OutOfPlace)
    );
}

#[test]
fn a_plain_copy_has_a_shape_of_its_own() {
    let d = Drawing::new()
        .with_piece("Leaf", leaf(), PieceKind::Add)
        .unwrap()
        .with_colour(PieceId(1), [0.1, 0.5, 0.2])
        .unwrap();
    let (d, new) = d
        .with_pasted(
            &d.copied(&[PieceId(1)]),
            false,
            &Change::moved(Vec2::X * 0.1),
        )
        .unwrap();
    let copy = new[0];
    assert_eq!(d.linked_count(copy), 1);
    assert_eq!(d.piece(copy).unwrap().colour, [0.1, 0.5, 0.2]);
    assert_eq!(d.piece(copy).unwrap().name, "Leaf");
    let placed = d.piece(copy).unwrap().placed();
    let d2 = d
        .with_path(copy, placed.with_knot(0, Knot::corner(Vec2::new(0.6, 0.1))))
        .unwrap();
    assert_eq!(d2.piece(PieceId(1)).unwrap().path, leaf());
}

#[test]
fn unlinking_gives_one_copy_its_own_shape() {
    let (d, [a, b, c]) = twig();
    let d = d.with_unlinked(b).unwrap();
    assert_eq!(d.linked_count(a), 2);
    assert_eq!(d.linked_count(b), 1);
    let d = d
        .with_shape(b, Basic::Square.path(Vec2::splat(0.5), 0.1))
        .unwrap();
    assert_eq!(d.piece(a).unwrap().path, leaf());
    assert_eq!(d.piece(c).unwrap().path, leaf());
    assert!(d.with_unlinked(PieceId(99)).is_err());
}

#[test]
fn pasting_into_another_canvas_links_the_copies_to_each_other() {
    let (d, [a, b, _]) = twig();
    let copies = d.copied(&[a, b]);
    // Another drawing with a different shape under the same link number.
    let other = Drawing::new()
        .with_piece(
            "Square",
            Basic::Square.path(Vec2::splat(0.5), 0.2),
            PieceKind::Add,
        )
        .unwrap();
    let (o, new) = other.with_pasted(&copies, true, &Change::IDENTITY).unwrap();
    assert_eq!(o.linked_count(new[0]), 2);
    assert_eq!(o.linked_count(PieceId(1)), 1);
    assert_ne!(
        o.piece(new[0]).unwrap().link,
        o.piece(PieceId(1)).unwrap().link
    );
    // Pasted plain, nothing is linked.
    let (p, new) = other
        .with_pasted(&copies, false, &Change::IDENTITY)
        .unwrap();
    assert_eq!(p.linked_count(new[0]), 1);
    assert_eq!(Drawing::from_parts(o.to_parts()), Ok(o));
}

#[test]
fn flipping_turns_pieces_over_about_their_middle() {
    let (d, [a, b, _]) = twig();
    let before = d.middle_of(&[a, b]).unwrap();
    let f = d.flipped(&[a, b], true).unwrap();
    assert!(f.piece(a).unwrap().place.flips());
    assert!(f.middle_of(&[a, b]).unwrap().distance(before) < 1e-5);
    // Flipped across, each piece swapped sides.
    let was = d.piece(a).unwrap().placed().middle();
    let now = f.piece(a).unwrap().placed().middle();
    assert!((now.x - (2.0 * before.x - was.x)).abs() < 1e-5);
    assert!((now.y - was.y).abs() < 1e-5);
    // Twice is back where it was; up and down flips the other way.
    let back = f.flipped(&[a, b], true).unwrap();
    assert!(back.piece(a).unwrap().placed().middle().distance(was) < 1e-5);
    let up = d.flipped(&[a], false).unwrap();
    assert!(up.piece(a).unwrap().place.flips());
    // The mask of a flipped piece still fills.
    let mask = crate::raster::mask(&f, 32);
    assert!(mask.iter().any(|v| *v > 0.9));
    assert!(d.flipped(&[PieceId(99)], true).is_err());
}

#[test]
fn sizing_to_nothing_is_refused() {
    let (d, [a, ..]) = twig();
    let m = d.piece(a).unwrap().placed().middle();
    let flat = d.with_change(&[a], &Change::scaled(m, Vec2::new(0.0, 1.0)));
    assert_eq!(flat, Err(Problem::Squashed));
    let tiny = d.with_change(&[a], &Change::scaled(m, Vec2::splat(1e-5)));
    assert!(tiny.is_err());
}

#[test]
fn groups_gather_pieces_together_in_the_order() {
    let mut d = Drawing::new();
    for name in ["A", "B", "C", "D"] {
        d = d
            .with_piece(
                name,
                Basic::Circle.path(Vec2::splat(0.5), 0.1),
                PieceKind::Add,
            )
            .unwrap();
    }
    // Group A and C: they come together where C (the topmost) was.
    let (g, group) = d.with_group(&[PieceId(1), PieceId(3)], "Twig").unwrap();
    assert_eq!(ids(&g), vec![2, 1, 3, 4]);
    assert_eq!(g.group(group).unwrap().name, "Twig");
    assert_eq!(g.group_pieces(group), vec![PieceId(1), PieceId(3)]);
    // An ungrouped piece moves past the whole group.
    assert_eq!(ids(&g.with_order(PieceId(2), 1).unwrap()), vec![1, 3, 2, 4]);
    assert_eq!(
        ids(&g.with_order(PieceId(4), -1).unwrap()),
        vec![2, 4, 1, 3]
    );
    // A grouped piece moves only within its group.
    assert_eq!(ids(&g.with_order(PieceId(1), 5).unwrap()), vec![2, 3, 1, 4]);
    assert_eq!(
        ids(&g.with_order(PieceId(1), -5).unwrap()),
        vec![2, 1, 3, 4]
    );
    // The group moves as one.
    assert_eq!(
        ids(&g.with_group_order(group, 1).unwrap()),
        vec![2, 4, 1, 3]
    );
    assert_eq!(
        ids(&g.with_group_order(group, -9).unwrap()),
        vec![1, 3, 2, 4]
    );
    // Moving the group's pieces moves them all.
    let moved = g
        .with_change(&g.group_pieces(group), &Change::moved(Vec2::X * 0.1))
        .unwrap();
    assert!(
        moved.piece(PieceId(3)).unwrap().placed().middle().x - 0.6 < 1e-5
            && moved.piece(PieceId(1)).unwrap().placed().middle().x > 0.59
    );
    // Renaming, ungrouping; an emptied group goes.
    let renamed = g.with_group_name(group, "Branch").unwrap();
    assert_eq!(renamed.group(group).unwrap().name, "Branch");
    assert_eq!(g.with_group_name(group, " "), Err(Problem::BadName));
    let apart = g.without_group(group).unwrap();
    assert!(apart.groups().is_empty());
    assert!(apart.pieces().iter().all(|p| p.group.is_none()));
    let gone = g.without(&[PieceId(1), PieceId(3)]).unwrap();
    assert!(gone.groups().is_empty());
    assert!(g.with_group(&[], "Empty").is_err());
    assert!(g.with_group(&[PieceId(99)], "Nothing").is_err());
}

#[test]
fn a_new_group_never_splits_another() {
    let mut d = Drawing::new();
    for name in ["A", "B", "C", "D"] {
        d = d
            .with_piece(
                name,
                Basic::Circle.path(Vec2::splat(0.5), 0.1),
                PieceKind::Add,
            )
            .unwrap();
    }
    let (g, first) = d
        .with_group(&[PieceId(2), PieceId(3), PieceId(4)], "First")
        .unwrap();
    // C leaves First for a group with A: it lands past First's pieces.
    let (h, second) = g.with_group(&[PieceId(1), PieceId(3)], "Second").unwrap();
    assert_eq!(h.group_pieces(first), vec![PieceId(2), PieceId(4)]);
    assert_eq!(h.group_pieces(second).len(), 2);
    assert_eq!(Drawing::from_parts(h.to_parts()), Ok(h));
}

#[test]
fn groups_are_copied_as_new_groups() {
    let mut d = Drawing::new();
    for name in ["A", "B"] {
        d = d
            .with_piece(
                name,
                Basic::Circle.path(Vec2::splat(0.5), 0.1),
                PieceKind::Add,
            )
            .unwrap();
    }
    let (d, group) = d.with_group(&[PieceId(1), PieceId(2)], "Twig").unwrap();
    let (e, new) = d
        .with_pasted(&d.copied(&d.group_pieces(group)), true, &Change::IDENTITY)
        .unwrap();
    assert_eq!(e.groups().len(), 2);
    let copy_group = e.piece(new[0]).unwrap().group.unwrap();
    assert_ne!(copy_group, group);
    assert_eq!(e.group(copy_group).unwrap().name, "Twig");
    assert_eq!(Drawing::from_parts(e.to_parts()), Ok(e));
}

#[test]
fn damaged_links_and_groups_are_refused() {
    let (d, _) = twig();
    let (d, _) = d.with_group(&[PieceId(1), PieceId(2)], "Twig").unwrap();
    let mut mixed = d.to_parts();
    mixed.pieces[1].path = Basic::Square.path(Vec2::splat(0.5), 0.1);
    assert_eq!(Drawing::from_parts(mixed), Err(Problem::BadLink));
    let mut far_link = d.to_parts();
    far_link.pieces[0].link = LinkId(9999);
    assert_eq!(Drawing::from_parts(far_link), Err(Problem::BadLink));
    let mut missing = d.to_parts();
    missing.groups.clear();
    assert_eq!(Drawing::from_parts(missing), Err(Problem::BadGroup));
    let mut apart = d.to_parts();
    assert_eq!(apart.pieces[2].group, None);
    apart.pieces.swap(1, 2);
    assert_eq!(Drawing::from_parts(apart), Err(Problem::BadGroup));
    let mut squashed = d.to_parts();
    squashed.pieces[0].place.matrix = glam::Mat2::ZERO;
    assert_eq!(Drawing::from_parts(squashed), Err(Problem::Squashed));
    let mut wild = d.to_parts();
    wild.pieces[0].colour = [3.0, -1.0, 0.5];
    wild.pieces[0].height = 9.0;
    let fixed = Drawing::from_parts(wild).unwrap();
    assert_eq!(fixed.pieces()[0].colour, [1.0, 0.0, 0.5]);
    assert_eq!(fixed.pieces()[0].height, 1.0);
    let mut empty_group = d.to_parts();
    empty_group.groups.push(Group {
        id: GroupId(1),
        name: "Nobody".into(),
    });
    assert_eq!(Drawing::from_parts(empty_group).unwrap().groups().len(), 1);
}

#[test]
fn colours_and_heights_are_laid_in_order() {
    let d = Drawing::new()
        .with_piece(
            "Under",
            Basic::Square.path(Vec2::splat(0.5), 0.3),
            PieceKind::Add,
        )
        .unwrap()
        .with_piece(
            "Over",
            Basic::Square.path(Vec2::splat(0.5), 0.1),
            PieceKind::Add,
        )
        .unwrap()
        .with_colour(PieceId(1), [1.0, 0.0, 0.0])
        .unwrap()
        .with_colour(PieceId(2), [0.0, 0.0, 1.0])
        .unwrap()
        .with_height(PieceId(1), 0.5)
        .unwrap();
    let l = crate::raster::layers(&d, 32);
    let at = |x: usize, y: usize| y * 32 + x;
    // The middle: the top piece; the ring round it: the one under it;
    // outside: black and nothing.
    assert_eq!(l.colours[at(16, 16)], [0.0, 0.0, 1.0]);
    assert_eq!(l.heights[at(16, 16)], 1.0);
    assert_eq!(l.colours[at(16, 8)], [1.0, 0.0, 0.0]);
    assert_eq!(l.heights[at(16, 8)], 0.5);
    assert_eq!(l.colours[at(1, 1)], [0.0; 3]);
    assert_eq!(l.mask, crate::raster::mask(&d, 32));
    // A cut-out takes colour away as it takes the shape away.
    let bitten = d
        .with_piece(
            "Bite",
            Basic::Square.path(Vec2::splat(0.5), 0.05),
            PieceKind::Cut,
        )
        .unwrap();
    let l = crate::raster::layers(&bitten, 32);
    assert_eq!(l.colours[at(16, 16)], [0.0; 3]);
    assert_eq!(l.mask[at(16, 16)], 0.0);
    assert!(crate::raster::layers(&d, 0).mask.is_empty());
    assert!(crate::raster::layers(&d, u32::MAX).colours.is_empty());
}

#[test]
fn placing_changes_the_key_and_the_picture() {
    let (d, [a, ..]) = twig();
    let moved = d.with_change(&[a], &Change::moved(Vec2::Y * 0.1)).unwrap();
    assert_ne!(d.key(), moved.key());
    assert_ne!(crate::raster::mask(&d, 32), crate::raster::mask(&moved, 32));
    let coloured = d.with_colour(a, [0.5; 3]).unwrap();
    assert_ne!(d.key(), coloured.key());
}

/// A fuzz run: random copies, links, flips, groups and order changes
/// always leave a drawing that its own parts rebuild, with linked pieces
/// sharing one shape and groups together.
#[test]
fn random_assembly_changes_always_leave_a_drawing_that_rebuilds() {
    let mut state = 12u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as u32
    };
    let mut d = Drawing::new()
        .with_piece("Leaf", leaf(), PieceKind::Add)
        .unwrap();
    for _ in 0..500 {
        let pick = |d: &Drawing, r: u32| {
            d.pieces()
                .get(r as usize % d.pieces().len().max(1))
                .map_or(PieceId(999), |p| p.id)
        };
        let some = |d: &Drawing, r: u32| -> Vec<PieceId> {
            d.pieces()
                .iter()
                .enumerate()
                .filter(|(i, _)| (r >> (i % 16)) & 1 == 1)
                .map(|(_, p)| p.id)
                .collect()
        };
        let v = (next() % 400) as f32 / 1000.0 - 0.2;
        let r = next();
        let result = match next() % 11 {
            0 | 1 => d
                .with_pasted(
                    &d.copied(&some(&d, r)),
                    next() % 2 == 0,
                    &Change::moved(Vec2::splat(v)),
                )
                .map(|(d, _)| d),
            2 => d.flipped(&some(&d, r), next() % 2 == 0),
            3 => d.with_group(&some(&d, r), "G").map(|(d, _)| d),
            4 => match d.groups().first() {
                Some(g) if next() % 2 == 0 => d.without_group(g.id),
                Some(g) => d.with_group_order(g.id, next() as i32 % 5 - 2),
                None => Ok(d.clone()),
            },
            5 => d.with_order(pick(&d, r), next() as i32 % 5 - 2),
            6 => d.with_unlinked(pick(&d, r)),
            7 => {
                let id = pick(&d, r);
                match d.piece(id) {
                    Some(p) => {
                        let placed = p.placed();
                        let k = placed.knots()[0];
                        d.with_path(id, placed.with_knot(0, Knot::corner(k.at + Vec2::splat(v))))
                    }
                    None => Ok(d.clone()),
                }
            }
            8 => d.without(&[pick(&d, r)]),
            9 => d.with_change(&some(&d, r), &Change::turned(Vec2::splat(0.5), v * 3.0)),
            _ => d.with_colour(pick(&d, r), [v, 0.5, 1.0]),
        };
        if let Ok(changed) = result {
            d = changed;
        }
        if d.is_empty() {
            d = d.with_piece("Leaf", leaf(), PieceKind::Add).unwrap();
        }
        assert_eq!(Drawing::from_parts(d.to_parts()).as_ref(), Ok(&d));
        assert_eq!(crate::raster::mask(&d, 16).len(), 256);
    }
}
