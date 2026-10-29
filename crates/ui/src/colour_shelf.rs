//! The Colours tab of the brush shelf ([decision 0213](../../../docs/decisions/0213-paint-interface-overhaul.md)):
//! beside the brushes under the views, the colours to paint with. **Used
//! lately** fills in by itself with the colours you paint with; under it,
//! the project's palettes. Click a colour to paint with it; double-click
//! to paint with it and open the colour picker on the brush bar.

use crate::attributes::colour_picker::{PaletteRequest, free_palette_name, to_color};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Sense, Stroke, Ui, vec2};
use paint::palette::Palette;
use paint::palette::hex;

/// How many colours Used lately keeps.
pub(crate) const USED_KEPT: usize = 16;
/// A swatch's side in the shelf.
const SWATCH: f32 = 30.0;
/// Accessible name of the Colours tab's button.
pub(crate) const COLOURS_TAB: &str = crate::text!("Show the colours");
/// Accessible name of the Brushes tab's button.
pub(crate) const BRUSHES_TAB: &str = crate::text!("Show the brushes");
/// The + button's accessible name.
pub(crate) const ADD_LABEL: &str = crate::text!("Add the brush's colour to the palette");

/// Which tab of the shelf is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ShelfTab {
    /// The brush cards.
    #[default]
    Brushes,
    /// The colours.
    Colours,
}

/// The tab buttons at the shelf's top left.
pub(crate) fn tabs(ui: &mut Ui, tab: &mut ShelfTab) {
    for (which, label, name) in [
        (ShelfTab::Brushes, tr("Brushes"), tr(BRUSHES_TAB)),
        (ShelfTab::Colours, tr("Colours"), tr(COLOURS_TAB)),
    ] {
        let on = *tab == which;
        let button = ui.add(egui::Button::selectable(on, label));
        button.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, name));
        if button.clicked() {
            *tab = which;
        }
    }
    ui.separator();
}

/// The Colours tab's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ColourShelf {
    /// The palette shown, by place in the project's list.
    palette: usize,
}

/// What a click in the shelf did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ShelfClick {
    /// A double-click: open the colour picker.
    pub(crate) open_picker: bool,
}

impl ColourShelf {
    /// Draws the tab: Used lately, then the palette with its choices.
    /// `colour` is the brush's; changes to the palettes go to `requests`.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        colour: &mut [u8; 3],
        used: &[[u8; 3]],
        palettes: &[Palette],
        requests: &mut Vec<PaletteRequest>,
        lead: impl FnOnce(&mut Ui),
    ) -> ShelfClick {
        let mut click = ShelfClick::default();
        if self.palette >= palettes.len() {
            self.palette = 0;
        }
        ui.horizontal(|ui| {
            lead(ui);
            ui.label(RichText::new(tr("Used lately")).color(theme::TEXT_WEAK))
                .on_hover_text(tr(
                    "The colours you've painted with, newest first. They're kept by themselves",
                ));
            if used.is_empty() {
                ui.label(
                    RichText::new(tr("none yet: they appear here as you paint"))
                        .color(theme::TEXT_WEAK)
                        .small(),
                );
            }
            for c in used {
                let label = crate::trf!("Used {}", hex(*c));
                if let Some(open) = swatch(ui, *c, *c == *colour, &label) {
                    *colour = *c;
                    click.open_picker |= open;
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            self.palette_choice(ui, palettes, requests);
            let add = ui.button("+");
            add.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr(ADD_LABEL))
            });
            if add.on_hover_text(tr(ADD_LABEL)).clicked() {
                let mut all = palettes.to_vec();
                if all.is_empty() {
                    all.push(Palette::new(tr("Palette")));
                    self.palette = 0;
                }
                let palette = &mut all[self.palette];
                if palette.colours.len() < paint::palette::MAX_COLOURS {
                    palette.colours.push(*colour);
                    requests.push(PaletteRequest::Set(
                        all,
                        crate::text!("Add a colour to a palette"),
                    ));
                }
            }
            if let Some(palette) = palettes.get(self.palette) {
                for (i, c) in palette.colours.iter().enumerate() {
                    let label = hex(*c);
                    let response = swatch_response(ui, *c, *c == *colour, &label);
                    if response.double_clicked() || response.triple_clicked() {
                        *colour = *c;
                        click.open_picker = true;
                    } else if response.clicked() {
                        *colour = *c;
                    }
                    if response.secondary_clicked() {
                        let mut all = palettes.to_vec();
                        all[self.palette].colours.remove(i);
                        requests.push(PaletteRequest::Set(
                            all,
                            crate::text!("Remove a colour from a palette"),
                        ));
                    }
                }
            } else {
                ui.label(
                    RichText::new(tr("No palette yet: + keeps the brush's colour in one."))
                        .color(theme::TEXT_WEAK),
                );
            }
        });
        click
    }

    /// The palette list: which one is shown, and New, Import, Export and
    /// Delete.
    fn palette_choice(
        &mut self,
        ui: &mut Ui,
        palettes: &[Palette],
        requests: &mut Vec<PaletteRequest>,
    ) {
        let shown = palettes
            .get(self.palette)
            .map_or_else(|| tr("Palette").to_string(), |p| p.name.clone());
        egui::ComboBox::from_id_salt("shelf_palette")
            .selected_text(shown)
            .width(120.0)
            .show_ui(ui, |ui| {
                for (i, p) in palettes.iter().enumerate() {
                    ui.selectable_value(&mut self.palette, i, &p.name);
                }
                ui.separator();
                if ui.button(tr("New palette")).clicked() {
                    let mut all = palettes.to_vec();
                    all.push(Palette::new(free_palette_name(palettes)));
                    self.palette = all.len() - 1;
                    requests.push(PaletteRequest::Set(all, crate::text!("Add a palette")));
                }
                if ui.button(tr("Import palette…")).clicked() {
                    requests.push(PaletteRequest::Import);
                }
                if !palettes.is_empty() {
                    if ui.button(tr("Export palette…")).clicked() {
                        requests.push(PaletteRequest::Export(self.palette));
                    }
                    if ui.button(tr("Delete palette")).clicked() {
                        let mut all = palettes.to_vec();
                        all.remove(self.palette);
                        self.palette = 0;
                        requests.push(PaletteRequest::Set(all, crate::text!("Delete a palette")));
                    }
                }
            })
            .response
            .on_hover_text(tr(
                "Colours kept with the project. Click a colour to paint with it, double-click to \
                 open the colour picker too, right click to remove it. Palettes read and write \
                 GIMP palette files (.gpl)",
            ));
    }
}

/// A swatch; `Some(double)` when clicked.
fn swatch(ui: &mut Ui, c: [u8; 3], chosen: bool, label: &str) -> Option<bool> {
    let response = swatch_response(ui, c, chosen, label);
    if response.double_clicked() || response.triple_clicked() {
        Some(true)
    } else if response.clicked() {
        Some(false)
    } else {
        None
    }
}

/// A square of colour `c`, ringed white when `chosen`.
fn swatch_response(ui: &mut Ui, c: [u8; 3], chosen: bool, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(SWATCH, SWATCH), Sense::click());
    ui.painter().rect_filled(rect, 3.0, to_color(c));
    let ring = if chosen {
        Stroke::new(2.0, egui::Color32::WHITE)
    } else if response.hovered() {
        Stroke::new(1.0, theme::TEXT_WEAK)
    } else {
        Stroke::new(1.0, theme::BORDER)
    };
    ui.painter()
        .rect_stroke(rect, 3.0, ring, egui::StrokeKind::Outside);
    let label = label.to_string();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    response.on_hover_text(hex(c))
}

/// `used` with `colour` put first (moved there if it was already in),
/// keeping at most [`USED_KEPT`].
pub(crate) fn note_used(used: &mut Vec<[u8; 3]>, colour: [u8; 3]) {
    used.retain(|c| *c != colour);
    used.insert(0, colour);
    used.truncate(USED_KEPT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_chooses_a_colour_and_a_double_click_asks_for_the_picker() {
        let used = vec![[200, 10, 10], [10, 200, 10]];
        let state = std::rc::Rc::new(std::cell::RefCell::new(([0u8; 3], ShelfClick::default())));
        let seen = state.clone();
        let mut shelf = ColourShelf::default();
        let mut harness = egui_kittest::Harness::builder()
            .with_step_dt(0.02)
            .build_ui(move |ui| {
                let mut s = seen.borrow_mut();
                let mut requests = Vec::new();
                let click = shelf.show(ui, &mut s.0, &used, &[], &mut requests, |_| {});
                s.1.open_picker |= click.open_picker;
            });
        harness.run();
        use egui_kittest::kittest::Queryable;
        harness.get_by_label("Used #0AC80A").click();
        harness.run();
        assert_eq!(state.borrow().0, [10, 200, 10]);
        assert!(!state.borrow().1.open_picker);
        harness.get_by_label("Used #C80A0A").click();
        harness.get_by_label("Used #C80A0A").click();
        harness.run();
        assert_eq!(state.borrow().0, [200, 10, 10]);
        assert!(
            state.borrow().1.open_picker,
            "a double-click asks for the picker"
        );
    }

    #[test]
    fn used_colours_are_newest_first_without_repeats_and_kept_short() {
        let mut used = Vec::new();
        note_used(&mut used, [1, 0, 0]);
        note_used(&mut used, [2, 0, 0]);
        note_used(&mut used, [1, 0, 0]);
        assert_eq!(used, [[1, 0, 0], [2, 0, 0]]);
        for i in 0..40 {
            note_used(&mut used, [i, 9, 9]);
        }
        assert_eq!(used.len(), USED_KEPT);
        assert_eq!(used[0], [39, 9, 9]);
    }
}
