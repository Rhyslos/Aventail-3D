//! Picking seams in the UV Editor (decision 0295): with Blend seams, a
//! click near either side of a seam on the texture marks its line, Shift +
//! click marks more or unmarks, a click away from seams says why, and
//! pointing at a seam in either view lights both its sides in the editor
//! and the line on the model.

use super::paint_layers_tests::set;
use super::seam_blend_tests::{blending_the_cube, click, cube_surface, top_blend, wait};
use crate::viewport::SeamLook;
use egui::Modifiers;
use glam::Vec2;
use paint::seams::{Seam, line_of, seams_of};
use std::collections::BTreeSet;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The cube's seams, as the Blend seams tool finds them.
fn cube_seams() -> Vec<Seam> {
    seams_of(&cube_surface(), &[0])
}

/// Where the middle of `seam`'s side `side` is in the UV Editor.
fn side_on_editor(harness: &H, seam: &Seam, side: usize) -> egui::Pos2 {
    let uvs = seam.sides[side].uvs;
    harness
        .state()
        .workbench
        .uv_editor
        .screen_of((uvs[0] + uvs[1]) / 2.0)
        .expect("the UV Editor is shown")
}

/// The ids of the seam line `start` is on.
fn line_ids(seams: &[Seam], start: usize) -> BTreeSet<[u32; 2]> {
    line_of(seams, start)
        .into_iter()
        .map(|i| seams[i].points)
        .collect()
}

/// The marked seams of the top layer, by their points.
fn marked(harness: &H) -> BTreeSet<[u32; 2]> {
    top_blend(harness)
        .map(|b| b.seams.iter().map(|s| s.points).collect())
        .unwrap_or_default()
}

/// Moves the mouse to `at` and runs a few frames.
fn hover(harness: &mut H, at: egui::Pos2) {
    harness.event(egui::Event::PointerMoved(at));
    for _ in 0..3 {
        harness.step();
    }
}

#[test]
fn the_uv_editor_shows_both_sides_of_every_seam_while_the_tool_is_in_use() {
    let mut harness = blending_the_cube();
    let lines = harness
        .state()
        .workbench
        .uv_editor
        .seam_lines()
        .expect("seams shown in the UV Editor")
        .clone();
    let seams = cube_seams();
    assert_eq!(lines.all.len(), seams.len() * 2, "two lines a seam");
    assert!(lines.all.iter().all(|l| l.look == SeamLook::Faint));
    for seam in &seams {
        for side in seam.sides {
            assert!(lines.all.iter().any(|l| l.uvs == side.uvs));
        }
    }
    // Another tool: they go.
    harness.key_press(egui::Key::Num1);
    harness.run();
    assert!(harness.state().workbench.uv_editor.seam_lines().is_none());
}

#[test]
fn pointing_at_one_side_lights_both_sides_and_the_line_on_the_model() {
    let mut harness = blending_the_cube();
    let seams = cube_seams();
    let at = side_on_editor(&harness, &seams[0], 0);
    hover(&mut harness, at);
    let workbench = &harness.state().workbench;
    let flat = &workbench.uv_editor.seam_lines().unwrap().hovered;
    let line = line_of(&seams, 0);
    assert_eq!(flat.len(), line.len() * 2, "both sides of each edge");
    for &i in &line {
        for side in seams[i].sides {
            assert!(flat.iter().any(|l| l.uvs == side.uvs), "side {side:?}");
        }
    }
    assert!(flat.iter().all(|l| l.look == SeamLook::Hovered));
    let on_model = &workbench.viewport.seam_lines().unwrap().hovered;
    assert_eq!(on_model.len(), line.len(), "the line lit on the model too");
}

#[test]
fn pointing_at_a_seam_on_the_model_lights_it_in_the_uv_editor() {
    let mut harness = blending_the_cube();
    let seam = super::seam_blend_tests::seams_in_view(&harness)[0];
    let at = super::modelling_tests::on_screen(&harness, (seam.ends[0] + seam.ends[1]) / 2.0);
    hover(&mut harness, at);
    let flat = &harness
        .state()
        .workbench
        .uv_editor
        .seam_lines()
        .unwrap()
        .hovered;
    for side in seam.sides {
        assert!(flat.iter().any(|l| l.uvs == side.uvs));
    }
}

#[test]
fn a_click_on_either_side_marks_the_line_in_one_undo_step() {
    let seams = cube_seams();
    for side in 0..2 {
        let mut harness = blending_the_cube();
        let steps = harness.state().workbench.document().undo_steps();
        let at = side_on_editor(&harness, &seams[0], side);
        click(&mut harness, at, Modifiers::NONE);
        assert_eq!(marked(&harness), line_ids(&seams, 0), "side {side}");
        assert_eq!(set(&harness).layers.last().unwrap().name, "Seam blend");
        wait(&mut harness);
        assert_ne!(top_blend(&harness).unwrap().made, 0, "it was made");
        // Marked, both sides in the accent colour.
        let lines = harness
            .state()
            .workbench
            .uv_editor
            .seam_lines()
            .unwrap()
            .clone();
        let lit = lines
            .all
            .iter()
            .filter(|l| l.look == SeamLook::Marked)
            .count();
        assert_eq!(lit, line_of(&seams, 0).len() * 2);
        // One step: one undo takes the layer away.
        let doc = harness.state_mut().workbench.document_mut();
        assert_eq!(doc.undo_steps(), steps + 1);
        doc.undo().unwrap();
        wait(&mut harness);
        assert!(set(&harness).layers.iter().all(|l| l.seam_blend.is_none()));
    }
}

#[test]
fn shift_click_marks_more_and_unmarks_from_the_other_side() {
    let mut harness = blending_the_cube();
    let seams = cube_seams();
    let first = line_ids(&seams, 0);
    let other = (0..seams.len())
        .find(|&i| !first.contains(&seams[i].points))
        .expect("another seam line");
    let at = side_on_editor(&harness, &seams[0], 0);
    click(&mut harness, at, Modifiers::NONE);
    let at = side_on_editor(&harness, &seams[other], 1);
    click(&mut harness, at, Modifiers::SHIFT);
    let both: BTreeSet<_> = first.union(&line_ids(&seams, other)).copied().collect();
    assert_eq!(marked(&harness), both);
    // Shift + click on the first line's other side unmarks it.
    let at = side_on_editor(&harness, &seams[0], 1);
    click(&mut harness, at, Modifiers::SHIFT);
    assert_eq!(marked(&harness), line_ids(&seams, other));
    // A plain click marks in place of the rest.
    let at = side_on_editor(&harness, &seams[0], 0);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(marked(&harness), first);
    let layers = set(&harness)
        .layers
        .iter()
        .filter(|l| l.seam_blend.is_some())
        .count();
    assert_eq!(layers, 1, "still one layer");
}

#[test]
fn a_click_off_the_seams_in_the_uv_editor_marks_nothing_and_says_why() {
    let mut harness = blending_the_cube();
    // Far from every piece.
    let at = harness
        .state()
        .workbench
        .uv_editor
        .screen_of(Vec2::new(-0.6, -0.6))
        .unwrap();
    let inside = harness
        .state()
        .workbench
        .uv_editor
        .drawn()
        .is_some_and(|r| r.contains(at));
    let at = if inside {
        at
    } else {
        // The middle of a piece instead, away from its edges.
        let t = cube_surface().triangles()[0];
        let middle = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
        harness
            .state()
            .workbench
            .uv_editor
            .screen_of(middle)
            .unwrap()
    };
    click(&mut harness, at, Modifiers::NONE);
    assert!(set(&harness).layers.iter().all(|l| l.seam_blend.is_none()));
    assert_eq!(set(&harness).layers.len(), 1, "nothing painted");
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some(crate::text::tr(super::seam_picking::NO_SEAM_ON_TEXTURE))
    );
}

#[test]
fn nothing_selected_shows_no_seams_in_the_uv_editor_and_picks_none() {
    let mut harness = blending_the_cube();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([]);
    for _ in 0..3 {
        harness.step();
    }
    let lines = harness.state().workbench.uv_editor.seam_lines().unwrap();
    assert!(lines.all.is_empty());
    // The 3D view still shows them.
    assert!(
        !harness
            .state()
            .workbench
            .viewport
            .seam_lines()
            .unwrap()
            .all
            .is_empty()
    );
}

#[test]
fn odd_places_and_zooms_never_crash_and_pick_the_nearest_side() {
    let mut harness = blending_the_cube();
    let seams = cube_seams();
    let workbench = &mut harness.state_mut().workbench;
    let drawing = workbench
        .drawing
        .update(&workbench.document, None, &workbench.hidden)
        .drawing
        .clone();
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 40) as f32 / (1u64 << 24) as f32
    };
    let odd = [f32::NAN, f32::INFINITY, -f32::INFINITY, 1e30, -1e30, 0.0];
    for i in 0..300 {
        let uv = if i < 36 {
            Vec2::new(odd[i % 6], odd[i / 6])
        } else {
            Vec2::new(next() * 3.0 - 1.0, next() * 3.0 - 1.0)
        };
        let zoom = [0.0, -5.0, 1e-9, 300.0, 1e9, f32::NAN][i % 6];
        let _ = workbench.pick_flat_seam(&drawing, uv, zoom);
    }
    // Right on a side, the seam it is on.
    for (i, seam) in seams.iter().enumerate() {
        for side in seam.sides {
            let uv = (side.uvs[0] + side.uvs[1]) / 2.0;
            let picked = workbench
                .pick_flat_seam(&drawing, uv, 300.0)
                .expect("a seam there");
            assert!(picked.ids.iter().any(|id| id.points == seams[i].points));
        }
    }
}
