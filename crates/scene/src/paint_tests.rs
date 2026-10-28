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
