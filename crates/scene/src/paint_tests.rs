//! Texture sets in the scene (decision 0177): set, undone, redone,
//! refused, and their undo memory.

use crate::{Command, Document};
use paint::{Image, Layer, TextureSet};

fn set_with_pixel(size: u32, rgba: [u8; 4]) -> TextureSet {
    let mut image = Image::new(size).unwrap();
    image.set_pixel(0, 0, rgba);
    let mut set = TextureSet::new(size).unwrap();
    set.layers.push(Layer::paint("Paint", image));
    set
}

#[test]
fn a_texture_set_is_given_undone_and_redone() {
    let mut doc = Document::new();
    let set = set_with_pixel(64, [1, 2, 3, 255]);
    doc.execute(Command::SetTextureSet {
        material: "Wood".into(),
        set: Some(set.clone()),
    })
    .unwrap();
    assert_eq!(doc.scene().paint().set("Wood").map(|s| &**s), Some(&set));
    doc.undo().unwrap();
    assert!(doc.scene().paint().is_empty());
    doc.redo().unwrap();
    assert_eq!(doc.scene().paint().set("Wood").map(|s| &**s), Some(&set));
    // Taken away.
    doc.execute(Command::SetTextureSet {
        material: "Wood".into(),
        set: None,
    })
    .unwrap();
    assert!(doc.scene().paint().is_empty());
}

#[test]
fn the_same_set_again_is_no_step() {
    let mut doc = Document::new();
    let set = set_with_pixel(64, [1, 2, 3, 255]);
    let give = || Command::SetTextureSet {
        material: "Wood".into(),
        set: Some(set.clone()),
    };
    doc.execute(give()).unwrap();
    let revision = doc.revision();
    doc.execute(give()).unwrap();
    assert_eq!(doc.revision(), revision);
}

#[test]
fn unsound_sets_and_empty_names_are_refused() {
    let mut doc = Document::new();
    let mut set = set_with_pixel(64, [0; 4]);
    set.layers[0].opacity = f32::NAN;
    assert!(
        doc.execute(Command::SetTextureSet {
            material: "Wood".into(),
            set: Some(set),
        })
        .is_err()
    );
    assert!(
        doc.execute(Command::SetTextureSet {
            material: " ".into(),
            set: Some(set_with_pixel(64, [0; 4])),
        })
        .is_err()
    );
    assert!(doc.scene().paint().is_empty());
}

#[test]
fn undo_memory_counts_only_changed_tiles() {
    let set = set_with_pixel(512, [9; 4]);
    let mut changed = set.clone();
    changed.layers[0]
        .image_mut()
        .unwrap()
        .set_pixel(0, 0, [8; 4]);
    let share = |s: &TextureSet| Some(std::sync::Arc::new(s.clone()));
    let edit = crate::tree::Edit::SetTextureSet {
        material: "Wood".into(),
        before: share(&set),
        after: share(&changed),
    };
    let bytes = crate::memory::edit(&edit);
    // One tile each side, not the whole 512 × 512 image.
    assert!(bytes >= 2 * paint::TILE_BYTES, "{bytes}");
    assert!(bytes < 3 * paint::TILE_BYTES, "{bytes}");
}

#[test]
fn palettes_change_in_one_undo_step_and_bad_ones_are_refused() {
    use paint::palette::{MAX_PALETTES, Palette};
    let mut doc = Document::new();
    let palette = Palette {
        name: "Crate".into(),
        colours: vec![[227, 170, 102]],
    };
    doc.execute(Command::SetPalettes(vec![palette.clone()]))
        .unwrap();
    assert_eq!(doc.scene().paint().palettes(), [palette]);
    doc.undo().unwrap();
    assert!(doc.scene().paint().palettes().is_empty());
    doc.redo().unwrap();
    assert_eq!(doc.scene().paint().palettes().len(), 1);
    let too_many = vec![Palette::new("x"); MAX_PALETTES + 1];
    assert!(doc.execute(Command::SetPalettes(too_many)).is_err());
    let long = Palette::new("n".repeat(paint::palette::MAX_NAME + 1));
    assert!(doc.execute(Command::SetPalettes(vec![long])).is_err());
}

#[test]
fn a_change_brings_paint_with_it_in_one_step() {
    // Decision 0189: what follows a change joins its step.
    let mut doc = Document::new();
    doc.execute(Command::Create(crate::ObjectKind::Group))
        .unwrap();
    let made = doc.changes_made();
    let steps = doc.undo_steps();
    doc.execute_with_last(Command::SetTextureSet {
        material: "Wood".into(),
        set: Some(set_with_pixel(64, [9, 9, 9, 255])),
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps, "joined the group's step");
    assert_eq!(doc.changes_made(), made, "not a change of its own");
    doc.undo().unwrap();
    assert!(doc.scene().paint().is_empty());
    assert!(doc.scene().walk().is_empty(), "both undone");
    doc.redo().unwrap();
    assert!(doc.scene().paint().set("Wood").is_some());
    assert_eq!(doc.changes_made(), made, "undo and redo aren't changes");
    // With nothing to join, a step of its own.
    let mut empty = Document::new();
    empty
        .execute_with_last(Command::SetTextureSet {
            material: "Wood".into(),
            set: Some(set_with_pixel(64, [9, 9, 9, 255])),
        })
        .unwrap();
    assert_eq!(empty.undo_steps(), 1);
}

#[test]
fn a_strokes_frames_are_one_change_keeping_no_tiles_between() {
    // Every frame of a stroke sets the texture set under one key: the step
    // keeps the set from before the stroke and after it, not each frame's.
    let mut doc = Document::new();
    for shade in 1..=5u8 {
        doc.execute_merging(
            Command::SetTextureSet {
                material: "Wood".into(),
                set: Some(set_with_pixel(64, [shade, 0, 0, 255])),
            },
            7,
        )
        .unwrap();
    }
    assert_eq!(doc.undo_steps(), 1);
    let one = doc.history_memory();
    assert!(one < 3 * paint::TILE_BYTES, "{one}");
    doc.undo().unwrap();
    assert!(doc.scene().paint().is_empty());
    doc.redo().unwrap();
    let set = doc.scene().paint().set("Wood").unwrap();
    assert_eq!(set.layers[0].image().unwrap().pixel(0, 0), [5, 0, 0, 255]);
}
