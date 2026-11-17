//! The Export window's General section for an STL file, for 3D printing
//! ([decision 0298](../../../../../docs/decisions/0298-stl-export.md)):
//! the print units, merging into one solid, the print's size, and the
//! print check in plain words. Textures, materials, UVs and levels of
//! detail don't apply, so their rows aren't shown.

use super::window::{custom_settings, objects_row, settings_line};
use super::{ExportChoice, plural, thousands};
use crate::dialog;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use fileio::export::{ExportSummary, PrintUnits};
use fileio::stl::check::PrintCheck;

/// The most print check findings listed; the rest are counted.
const MOST_LISTED: usize = 8;

/// Draws the section. `failed` is why the last export couldn't merge
/// its shapes, if it couldn't; `unmerged` is set when the person asks to
/// export them unmerged.
pub(super) fn printing(
    ui: &mut Ui,
    choice: &mut ExportChoice,
    selected: bool,
    (summary, check): (ExportSummary, &PrintCheck),
    failed: Option<&str>,
    unmerged: &mut bool,
) {
    dialog::heading(ui, tr("General"));
    dialog::help(
        ui,
        match choice.preset {
            Some(preset) => format!("{}: {}.", preset.name(), preset.description()),
            None => tr("Any program: pick the kind of file and how it's set up below.").to_string(),
        },
    );
    ui.add_space(4.0);
    if let Some(why) = failed {
        merge_failure(ui, why, unmerged);
    }
    objects_row(ui, choice, selected);
    match choice.preset {
        Some(preset) => {
            dialog::row(ui, tr("File settings"), |ui| {
                dialog::locked(ui, &settings_line(preset)).on_hover_text(tr(
                    "Set by Made for. Choose Custom settings to change them.",
                ));
            });
        }
        None => custom_settings(ui, choice),
    }
    units_row(ui, choice);
    let several = summary.objects > 1;
    dialog::row(ui, tr("Merge"), |ui| {
        ui.add_enabled_ui(several, |ui| {
            ui.checkbox(&mut choice.merge, tr("Merge into one solid"))
                .on_hover_text(tr(
                    "Join every shape into one closed solid with a Union boolean before writing \
                     the file, so overlapping parts don't leave walls inside the print that \
                     confuse the slicer. Done on copies: your scene doesn't change",
                ))
        })
        .response
        .on_disabled_hover_text(tr("Only one shape goes out: there is nothing to merge"));
    });
    dialog::help(
        ui,
        tr(
            "Textures, materials, UVs and levels of detail don't go into an STL file: a print is \
             only the shape.",
        ),
    );
    dialog::divider(ui);
    dialog::heading(ui, tr("What goes out"));
    ui.add_space(2.0);
    let units = choice.print_units;
    let size = check.size().map(|s| s * units.per_centimetre());
    let mut tiles = vec![
        (
            thousands(summary.objects),
            plural(summary.objects, tr("object"), tr("objects")),
        ),
        (
            thousands(summary.triangles),
            plural(summary.triangles, tr("triangle"), tr("triangles")),
        ),
    ];
    if let Some(size) = size {
        tiles.push((
            format!(
                "{} × {} × {}",
                length(size.x),
                length(size.y),
                length(size.z)
            ),
            units.symbol(),
        ));
    }
    dialog::tiles(ui, &tiles);
    ui.add_space(4.0);
    if size.is_some() {
        dialog::help(
            ui,
            tr(
                "The print's width × depth × height on the bed, standing as it stands on the grid \
                 here (Z up). Check it fits your printer.",
            ),
        );
    }
    print_check(ui, choice, check, several);
}

/// Why the last export couldn't merge its shapes, at the top of the
/// section, with the offer to export them unmerged.
fn merge_failure(ui: &mut Ui, why: &str, unmerged: &mut bool) {
    ui.label(
        RichText::new(crate::trf!("The last export stopped: {}.", why))
            .color(theme::WARNING)
            .size(12.0),
    );
    if ui
        .button(tr("Export without merging"))
        .on_hover_text(tr(
            "Write the same file with each shape as it is, side by side in one file. Most \
             slicers still print it; overlapping parts may leave walls inside",
        ))
        .clicked()
    {
        *unmerged = true;
    }
    ui.add_space(4.0);
}

/// The Print units row.
fn units_row(ui: &mut Ui, choice: &mut ExportChoice) {
    let hint = |u: PrintUnits| match u {
        PrintUnits::Millimetres => {
            tr("1 in the file is 1 mm: what every slicer reads (PrusaSlicer, Cura, Bambu Studio)")
        }
        PrintUnits::Centimetres => {
            tr("1 in the file is 1 cm: a slicer that reads millimetres makes it 10 times smaller")
        }
        PrintUnits::Metres => tr("1 in the file is 1 m"),
        PrintUnits::Inches => tr("1 in the file is 1 inch: for a slicer set to inches"),
    };
    let options: Vec<(PrintUnits, &str, &str)> = PrintUnits::ALL
        .iter()
        .map(|u| (*u, u.name(), hint(*u)))
        .collect();
    dialog::row(ui, tr("Print units"), |ui| {
        dialog::segmented(ui, &mut choice.print_units, &options);
    });
    dialog::help(
        ui,
        tr(
            "An STL file doesn't say what its numbers mean, and slicers read them as \
             millimetres: keep Millimetres and the print comes out the size it is here.",
        ),
    );
}

/// The print check: what would trouble a slicer, with what to do, and
/// what stops a merge. Warnings only: the export still goes ahead.
fn print_check(ui: &mut Ui, choice: &ExportChoice, check: &PrintCheck, several: bool) {
    ui.add_space(6.0);
    dialog::heading(ui, tr("Print check"));
    let warn = |ui: &mut Ui, text: &str| {
        ui.label(RichText::new(text).color(theme::WARNING).size(12.0));
    };
    if check.issues.is_empty() {
        dialog::help(
            ui,
            tr("Nothing found: every shape is closed, with its polygons facing out."),
        );
        return;
    }
    for issue in check.issues.iter().take(MOST_LISTED) {
        warn(ui, &issue.words());
    }
    if check.issues.len() > MOST_LISTED {
        warn(
            ui,
            &crate::trf!("And {} more like these.", check.issues.len() - MOST_LISTED),
        );
    }
    if several && choice.merge && check.stops_merge() {
        warn(
            ui,
            tr(
                "Merging needs every shape to be a closed solid, so it will stop at these: fix \
                 them, or turn off Merge into one solid.",
            ),
        );
    }
    dialog::help(
        ui,
        tr("These are warnings: the file is still written if you export now."),
    );
}

/// A length for the size tile: one decimal under 1 000, none above.
fn length(value: f32) -> String {
    if value.abs() < 1000.0 {
        let text = format!("{value:.1}");
        text.strip_suffix(".0").map(str::to_string).unwrap_or(text)
    } else {
        thousands(value.round().max(0.0) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::length;

    #[test]
    fn lengths_read_plainly() {
        assert_eq!(length(1000.0), "1,000");
        assert_eq!(length(120.0), "120");
        assert_eq!(length(45.55), "45.5");
        assert_eq!(length(0.25), "0.2");
        assert_eq!(length(12345.6), "12,346");
    }
}
