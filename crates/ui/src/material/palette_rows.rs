//! The palette setting's control (Palette map, Phase 12, milestone 19,
//! [decision 0281](../../../../docs/decisions/0281-palette-map.md)): a
//! list of the project's palettes (the painter's, made in the Paint
//! mode's colour picker), with the chosen one's swatches beside it so you
//! see which colours the box will use.

use crate::text::tr;
use crate::theme;
use egui::{Color32, RichText, Sense, Ui, vec2};
use matgraph::{Palette, Palettes};

/// The most swatches drawn beside the list.
const MAX_SHOWN: usize = 24;

/// A swatch's side, in points.
const SWATCH: f32 = 12.0;

/// What the list shows for the palette called `name` (none: the
/// project's first).
pub(crate) fn shown_name(palettes: &Palettes, name: &str) -> String {
    if name.is_empty() {
        match palettes.find("") {
            Some(first) => crate::trf!("First palette ({})", first.name),
            None => tr("First palette").to_string(),
        }
    } else if palettes.find(name).is_some() {
        name.to_string()
    } else {
        crate::trf!("{} (not in this project)", name)
    }
}

/// The palette setting's control: the palette chosen now is `name`; the
/// name chosen, if it changed.
pub(crate) fn palette_picker(
    ui: &mut Ui,
    key: &str,
    about: &str,
    palettes: &Palettes,
    name: &str,
) -> Option<String> {
    let mut picked = name.to_string();
    egui::ComboBox::from_id_salt(("material palette", key))
        .selected_text(shown_name(palettes, name))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut picked, String::new(), shown_name(palettes, ""));
            for p in palettes.iter() {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut picked, p.name.clone(), &p.name);
                    swatches(ui, p, 8);
                });
            }
            if palettes.is_empty() {
                crate::dialog::help(
                    ui,
                    tr(
                        "This project has no palettes yet: make one in the Paint mode's colour \
                         picker (Palette, then New palette).",
                    ),
                );
            }
        })
        .response
        .on_hover_text(about);
    if let Some(p) = palettes.find(name) {
        swatches(ui, p, MAX_SHOWN);
    } else {
        ui.label(
            RichText::new(tr("Not in this project"))
                .small()
                .color(super::look::REFUSED),
        );
    }
    (picked != name).then_some(picked)
}

/// Up to `most` of `palette`'s colours as small squares, with "+n" for
/// the rest.
fn swatches(ui: &mut Ui, palette: &Palette, most: usize) {
    ui.spacing_mut().item_spacing.x = 1.0;
    for c in palette.colours.iter().take(most) {
        let (rect, _) = ui.allocate_exact_size(vec2(SWATCH, SWATCH), Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, Color32::from_rgb(c[0], c[1], c[2]));
    }
    let more = palette.colours.len().saturating_sub(most);
    if more > 0 {
        ui.label(
            RichText::new(format!("+{more}"))
                .small()
                .color(theme::TEXT_WEAK),
        );
    }
    if palette.colours.is_empty() {
        ui.label(
            RichText::new(tr("No colours yet"))
                .small()
                .color(theme::TEXT_WEAK),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_names_the_first_palette_and_a_missing_one() {
        let palettes = Palettes::new([Palette {
            name: "Game".into(),
            colours: vec![[1, 2, 3]],
        }]);
        assert_eq!(shown_name(&palettes, ""), "First palette (Game)");
        assert_eq!(shown_name(&palettes, "Game"), "Game");
        assert_eq!(shown_name(&palettes, "Lava"), "Lava (not in this project)");
        assert_eq!(shown_name(&Palettes::default(), ""), "First palette");
    }
}
