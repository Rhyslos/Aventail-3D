//! The Displace modifier's rows in the Attributes panel
//! ([decision 0157](../../../../docs/decisions/0157-displace.md)): the
//! noise's look, strength, size, detail and seed, which way points are
//! pushed, and which points (with the falloff round them).

use super::curve_rows::Rows;
use super::wrap_rows::{PointTexts, points_row};
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::{DisplaceDirection, DisplaceOptions, DisplaceSettings, NoiseKind, VertexId};

/// The name of the choice of noise.
const NOISE: &str = crate::text!("Noise");
/// The name of the choice of direction.
const DIRECTION: &str = crate::text!("Direction");

/// Displace's settings (in a two-column grid). `chosen` is the points
/// selected on the shape in the Points mode (`None` in other modes), for
/// *Use selected*.
pub(super) fn displace_rows(
    ui: &mut Ui,
    settings: &mut DisplaceSettings,
    units: Units,
    rows: &mut Rows,
    chosen: Option<&[VertexId]>,
) {
    noise_row(ui, &mut settings.options, rows);
    let options = &mut settings.options;
    label(
        ui,
        tr("Strength"),
        tr(
            "How far the highest bumps push the points. Below 0, the bumps go the other way: \
             ridges and cells become grooves and dents.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.strength,
        (
            -DisplaceOptions::MAX_STRENGTH,
            DisplaceOptions::MAX_STRENGTH,
        ),
        units,
        tr("Strength"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Scale"),
        tr(
            "How big the bumps are, about from one to the next. Make it a few times the \
             distance between the shape's points: bumps smaller than that can't show.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.scale,
        (DisplaceOptions::MIN_SCALE, DisplaceOptions::MAX_SCALE),
        units,
        tr("Scale"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Detail"),
        tr(
            "How many layers of ever smaller bumps sit on the big ones, each half the size \
             and half the height of the last. 1 is the plainest.",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut options.detail,
        (1, DisplaceOptions::MAX_DETAIL),
        tr("Detail"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Seed"),
        tr(
            "Another number gives other bumps of the same kind. The same number always gives \
             the same bumps.",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut options.seed,
        (0, DisplaceOptions::MAX_SEED),
        tr("Seed"),
    ));
    ui.end_row();
    direction_row(ui, options, rows);
    let texts = PointTexts {
        help: tr(
            "Every point is pushed, or only those you choose, fading out round them over the \
             Falloff: select them in the Points mode (press 1) and click Use selected.",
        ),
        use_selected: tr("Push only the points selected now, and those round them"),
        all: tr("Push every point again"),
    };
    points_row(ui, &mut settings.points, rows, chosen, &texts);
    label(
        ui,
        tr("Falloff"),
        tr(
            "With chosen points, how far from them the push fades out, measured along the \
             surface. 0 gives a sharp edge. Every point is pushed fully when none are chosen.",
        ),
    );
    ui.add_enabled_ui(!settings.points.is_empty(), |ui| {
        rows.take(stepper::length(
            ui,
            &mut settings.options.falloff,
            (0.0, DisplaceOptions::MAX_FALLOFF),
            units,
            tr("Falloff"),
        ));
    });
    ui.end_row();
}

/// The choice of noise.
fn noise_row(ui: &mut Ui, options: &mut DisplaceOptions, rows: &mut Rows) {
    label(
        ui,
        tr(NOISE),
        tr(
            "The bumps' look. Smooth: soft rolling lumps, in and out, for rock and ground. \
             Ridged: sharp crests, for craggy stone and bark. Cells: rounded cells with grooves \
             between them, for cobbles, scales and hammered metal.",
        ),
    );
    let current = options.noise;
    let response = egui::ComboBox::from_id_salt(ui.id().with("displace_noise"))
        .selected_text(current.name())
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for noise in NoiseKind::ALL {
                if ui
                    .selectable_label(noise == current, noise.name())
                    .clicked()
                    && noise != current
                {
                    options.noise = noise;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(NOISE)));
    ui.end_row();
}

/// The choice of which way points are pushed.
fn direction_row(ui: &mut Ui, options: &mut DisplaceOptions, rows: &mut Rows) {
    label(
        ui,
        tr(DIRECTION),
        tr(
            "Which way the points are pushed: out from the surface (along its normals), or \
             straight along one of the shape's own axes, such as Z for ground and terrain.",
        ),
    );
    let current = options.direction;
    let response = egui::ComboBox::from_id_salt(ui.id().with("displace_direction"))
        .selected_text(current.name())
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for direction in DisplaceDirection::ALL {
                if ui
                    .selectable_label(direction == current, direction.name())
                    .clicked()
                    && direction != current
                {
                    options.direction = direction;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(DIRECTION)));
    ui.end_row();
}
