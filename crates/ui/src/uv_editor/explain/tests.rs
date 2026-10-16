//! Tests for "Why is this here?" (decision 0116): the explanations'
//! words, and what pointing finds.

use super::*;
use scene::PrimitiveKind;

#[test]
fn every_seam_reason_reads_plainly_with_a_fix() {
    for reason in [
        SeamReason::HardEdge,
        SeamReason::SharpCorner(90.0),
        SeamReason::ToLieFlat,
        SeamReason::Preferred,
        SeamReason::StretchBudget(1.5),
    ] {
        let text = seam_text(Some(reason), true, 66.0, scene::SeamPreference::Free);
        assert!(text.starts_with("Seam\n"), "{text}");
        assert!(text.contains(SEAM_FIX));
    }
    assert!(reason_text(SeamReason::SharpCorner(89.6), 66.0).contains("90°"));
    assert!(reason_text(SeamReason::SharpCorner(89.6), 66.0).contains("66°"));
    assert!(reason_text(SeamReason::StretchBudget(1.5), 66.0).contains("1.50×"));
    let by_hand = seam_text(None, true, 66.0, scene::SeamPreference::Never);
    assert!(by_hand.contains("by hand"), "{by_hand}");
    assert!(by_hand.contains("Never seams here"), "{by_hand}");
    assert!(seam_text(None, false, 66.0, scene::SeamPreference::Free).contains("too big"));
}

#[test]
fn a_pieces_facts_each_get_a_line() {
    let plain = PieceFacts {
        polygons: 1,
        density: Some(512.0),
        aim: 512.0,
        stretch: 1.0,
        ..PieceFacts::default()
    };
    let text = piece_text(&plain);
    assert!(text.starts_with("Piece of 1 polygon\n"), "{text}");
    assert!(text.contains("the texel density aimed for"));
    assert!(text.contains("Its place"));
    assert!(!text.contains("Locked") && !text.contains("red") && !text.contains("Stretched"));
    let busy = PieceFacts {
        polygons: 12,
        locked: true,
        trim_row: Some("Bolts".into()),
        pinned: true,
        own_settings: true,
        density: Some(256.0),
        aim: 512.0,
        overlapping: true,
        mirrored: true,
        stretch: 1.8,
    };
    let text = piece_text(&busy);
    for words in [
        "Piece of 12 polygons",
        "Locked",
        "“Bolts”",
        "pinned",
        "Piece settings",
        "0.50× the aim of 512",
        "overlaps",
        "mirrored",
        "Stretched up to 1.8×",
    ] {
        assert!(text.contains(words), "{words}: {text}");
    }
    assert!(!text.contains("Its place"), "locked pieces stay put");
    let big = piece_text(&PieceFacts {
        density: Some(1024.0),
        ..plain
    });
    assert!(big.contains("2.00× the aim") && big.contains("Set texel density"));
}

#[test]
fn pointing_finds_seams_first_then_pieces() {
    let mesh = Arc::new(PrimitiveKind::Cube.standard().mesh());
    let shape = UvShape::of(Arc::clone(&mesh)).unwrap();
    let id = ObjectId::from_raw(1);
    let shapes = vec![(id, shape.clone())];
    let mut editor = UvEditor::default();
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 600.0));
    editor.fit(rect, Vec2::ZERO, Vec2::ONE);
    // A seam: the middle of one of the drawn seam edges.
    let ([a, b], key, _) = shape
        .edges
        .iter()
        .find(|(_, k, _)| shape.seams.contains(k))
        .copied()
        .unwrap();
    let on_seam = editor.to_screen(rect, (a + b) / 2.0);
    assert_eq!(
        editor.point_at(rect, &shapes, on_seam),
        Some(Pointed::Seam(id, key))
    );
    // A piece: the middle of one, away from its seams.
    let island = &shape.islands[0];
    let middle = mesh::uvs::uv_centre(&mesh, island, &[]).unwrap();
    assert_eq!(
        editor.point_at(rect, &shapes, editor.to_screen(rect, middle)),
        Some(Pointed::Piece(id, island.clone()))
    );
    // Empty texture: nothing.
    assert_eq!(
        editor.point_at(rect, &shapes, editor.to_screen(rect, Vec2::splat(-0.5))),
        None
    );
}

#[test]
fn a_cubes_seams_are_explained_as_hard_edges_or_corners() {
    let doc = scene::Document::new();
    let mesh = Arc::new(PrimitiveKind::Cube.standard().mesh());
    let shape = UvShape::of(Arc::clone(&mesh)).unwrap();
    let id = ObjectId::from_raw(1);
    let shapes = vec![(id, shape.clone())];
    let mut editor = UvEditor::default();
    let key = *shape.seams.iter().next().unwrap();
    let text = editor
        .explanation(
            &Pointed::Seam(id, key),
            &shapes,
            &doc,
            &Default::default(),
            ExplainContext::default(),
        )
        .unwrap();
    assert!(
        text.contains("hard edge") || text.contains("sharper than"),
        "{text}"
    );
    let piece = editor
        .explanation(
            &Pointed::Piece(id, shape.islands[0].clone()),
            &shapes,
            &doc,
            &Default::default(),
            ExplainContext::default(),
        )
        .unwrap();
    assert!(piece.starts_with("Piece of 1 polygon"), "{piece}");
    assert!(piece.contains("px/m"), "{piece}");
    assert!(!editor.explaining());
    editor.set_explaining(true);
    assert!(editor.explaining());
    editor.set_explaining(false);
    assert!(editor.reasons.is_empty(), "forgotten when turned off");
}
