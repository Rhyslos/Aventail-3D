//! Fitting rows into the side panels, however narrow or short they are
//! made: a review of the Paint page found rows running off the right edge,
//! names cut mid-letter and the Paint tab's scroll bar pushed out of
//! sight by a row wider than the panel.
//!
//! - [`settings_grid`]: names beside their controls, or above them when
//!   the panel is too narrow for both on one line.
//! - [`side_scroll`]: a panel's scrolling body. It scrolls sideways too,
//!   so a row that still can't fit is reached with a scroll bar instead of
//!   widening the panel past its edge (and its scroll bar with it).
//! - [`side_row`]: a setting's row as dialogs lay it out, its name over
//!   the control when narrow.
//! - [`blank_cell`]: an empty name cell, left out when names are stacked.
//! - [`elided`]: one line of text cut short with "…" to fit a width.

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, ScrollArea, Ui, Vec2};

/// The width most settings grids need for a name beside its control: a
/// name column and a control of a hundred and fifty points or so.
pub(crate) const NARROW: f32 = 270.0;

/// A two-column table of settings with `spacing` between its cells:
/// names on the left and controls on the right, or, with less width than
/// `least` (what its widest name and control need side by side, about
/// [`NARROW`] for most), each name on its own line over its control. `add`
/// draws the rows as for an [`egui::Grid`]: a cell, another, then
/// `ui.end_row()` (which does nothing in the narrow form).
pub(crate) fn settings_grid<R>(
    ui: &mut Ui,
    id: impl egui::AsIdSalt,
    (spacing, least): (Vec2, f32),
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    if ui.available_width() < least {
        ui.vertical(|ui| {
            mark_stacked(ui, true);
            ui.spacing_mut().item_spacing.y = spacing.y.max(2.0);
            add(ui)
        })
        .inner
    } else {
        egui::Grid::new(id)
            .num_columns(2)
            .spacing(spacing)
            .show(ui, |ui| {
                mark_stacked(ui, false);
                add(ui)
            })
            .inner
    }
}

/// An empty name cell, for a control with no name of its own (a tick box
/// carrying its own words): keeps the grid's columns, and is left out
/// when the names are stacked over their controls.
pub(crate) fn blank_cell(ui: &mut Ui) {
    let stacked = ui.data(|d| d.get_temp(ui.id().with(STACKED)).unwrap_or(false));
    if !stacked {
        ui.label("");
    }
}

/// A setting as [`crate::dialog::row`] lays it out (its name in a column
/// as wide as every other setting's, the control `add` draws beside it),
/// or, with less than [`ROW_CONTROL_LEAST`] left beside the name, the
/// name on its own line over the control.
pub(crate) fn side_row<R>(ui: &mut Ui, name: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    if ui.available_width() < crate::dialog::NAME_WIDTH + ROW_CONTROL_LEAST {
        ui.label(egui::RichText::new(name).color(crate::theme::TEXT_WEAK));
        return ui.horizontal_wrapped(|ui| add(ui)).inner;
    }
    crate::dialog::row(ui, name, add)
}

/// The least width a [`side_row`]'s control is given beside its name.
pub(crate) const ROW_CONTROL_LEAST: f32 = 150.0;

/// The key under which a settings grid's body notes whether it is
/// stacked.
const STACKED: &str = "settings_grid_stacked";

fn mark_stacked(ui: &mut Ui, stacked: bool) {
    let id = ui.id().with(STACKED);
    ui.data_mut(|d| d.insert_temp(id, stacked));
}

/// A side panel's scrolling body, called `id`: down, and sideways only
/// when a row is wider than the panel. Rows are laid out to the panel's
/// width all the same (egui asks them to fit before scrolling), so text
/// still wraps.
pub(crate) fn side_scroll(id: impl egui::AsIdSalt) -> ScrollArea {
    ScrollArea::both().id_salt(id).auto_shrink([false, false])
}

/// Draws `text` on one line at `pos` (placed by `align`), cut short with
/// "…" when it is wider than `width`. Gives back where it went and
/// whether it was cut.
pub(crate) fn elided(
    painter: &Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    (font, colour): (FontId, Color32),
    width: f32,
) -> (Rect, bool) {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, colour);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(0.0));
    let galley = painter.layout_job(job);
    let cut = galley.elided;
    let rect = align.anchor_size(pos, galley.size());
    painter.galley(rect.min, galley, colour);
    (rect, cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(width: f32, add: impl FnMut(&mut Ui) + 'static) {
        let mut harness = egui_kittest::Harness::builder()
            .with_size([width, 300.0])
            .build_ui(add);
        harness.run();
    }

    #[test]
    fn a_wide_panel_puts_names_beside_their_controls() {
        run(400.0, |ui| {
            let rows = settings_grid(ui, "wide", (egui::vec2(8.0, 4.0), NARROW), |ui| {
                let name = ui.label("Name");
                let control = ui.button("Control");
                ui.end_row();
                (name.rect, control.rect)
            });
            assert!(rows.1.left() > rows.0.right(), "{rows:?}");
            assert!((rows.0.center().y - rows.1.center().y).abs() < 4.0);
        });
    }

    #[test]
    fn a_narrow_panel_puts_each_name_over_its_control() {
        run(NARROW - 20.0, |ui| {
            let rows = settings_grid(ui, "narrow", (egui::vec2(8.0, 4.0), NARROW), |ui| {
                let name = ui.label("Name");
                let control = ui.button("Control");
                ui.end_row();
                (name.rect, control.rect)
            });
            assert!(rows.1.top() >= rows.0.bottom(), "{rows:?}");
        });
    }

    #[test]
    fn a_blank_name_cell_takes_room_only_beside_controls() {
        for (width, stacked) in [(400.0, false), (NARROW - 20.0, true)] {
            run(width, move |ui| {
                let (first, second) = settings_grid(
                    ui,
                    ("blank", width as u32),
                    (egui::vec2(8.0, 4.0), NARROW),
                    |ui| {
                        let first = ui.button("First").rect;
                        ui.end_row();
                        blank_cell(ui);
                        let second = ui.checkbox(&mut true, "Second").rect;
                        ui.end_row();
                        (first, second)
                    },
                );
                if stacked {
                    // Straight under the first, no empty line between.
                    assert!(second.top() - first.bottom() < 6.0, "{first:?} {second:?}");
                } else {
                    assert!(second.left() > first.left(), "{first:?} {second:?}");
                }
            });
        }
    }

    #[test]
    fn long_text_is_cut_with_an_ellipsis_and_short_text_is_not() {
        run(300.0, |ui| {
            let painter = ui.painter().clone();
            let font = (FontId::proportional(13.0), Color32::WHITE);
            let at = egui::pos2(10.0, 10.0);
            let long = "A layer name far too long to fit the little space it is given";
            let (rect, cut) = elided(&painter, at, Align2::LEFT_TOP, long, font.clone(), 80.0);
            assert!(cut);
            assert!(rect.width() <= 80.5, "{rect:?}");
            let (rect, cut) = elided(&painter, at, Align2::LEFT_TOP, "Moss", font.clone(), 80.0);
            assert!(!cut);
            assert!(rect.width() < 80.0);
            // No room at all: nothing drawn past the start, and no crash.
            let (rect, cut) = elided(&painter, at, Align2::LEFT_TOP, long, font, -5.0);
            assert!(cut);
            assert!(rect.width() <= 16.0, "{rect:?}");
        });
    }
}
