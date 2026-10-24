//! The Scatter modifier's rows in the Attributes panel
//! ([decision 0146](../../../../docs/decisions/0146-scatter.md)): the
//! shapes scattered, where the copies go and how many, their spacing, how
//! they stand, turn and size, and which polygons of the surface take them.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::{Document, FaceId, ObjectId, PlaceOn, ScatterOptions, ScatterSettings};

/// The name of the choice of where copies go.
const PLACE_ON: &str = crate::text!("Place on");

/// A place's name.
fn place_name(place: PlaceOn) -> &'static str {
    match place {
        PlaceOn::Points => tr("Points"),
        PlaceOn::Centres => tr("Polygon centres"),
        PlaceOn::Random => tr("Random over the surface"),
    }
}

/// The shapes a scatter spreads, by name, with a button selecting them.
pub(super) fn shapes_row(
    ui: &mut Ui,
    doc: &Document,
    shapes: &[ObjectId],
    out: &mut Vec<DocRequest>,
) {
    label(
        ui,
        tr("Shapes"),
        tr(
            "The shapes scattered: each copy is one of them, picked at random. They stay shapes \
             of their own; edit one and every copy follows. Hide them to keep them out of \
             exports: hidden, they still scatter.",
        ),
    );
    ui.horizontal_wrapped(|ui| {
        let names: Vec<String> = shapes
            .iter()
            .filter_map(|id| doc.scene().object(*id).map(|o| o.name.clone()))
            .collect();
        ui.label(names.join(", "));
        if ui
            .small_button(tr("Select shapes"))
            .on_hover_text(tr("Select the shapes scattered, to change or move them"))
            .clicked()
        {
            out.push(DocRequest::Select(shapes.to_vec()));
        }
    });
    ui.end_row();
}

/// Scatter's settings (in a two-column grid). `chosen` is the polygons
/// selected on the surface in the Polygons mode (`None` in other modes),
/// for *Use selected*.
pub(super) fn scatter_rows(
    ui: &mut Ui,
    settings: &mut ScatterSettings,
    units: Units,
    rows: &mut Rows,
    chosen: Option<&[FaceId]>,
) {
    let options = &mut settings.options;
    label(
        ui,
        tr(PLACE_ON),
        tr(
            "Where the copies go: one on each point, one on the middle of each polygon, or a \
             number spread at random, evenly by area",
        ),
    );
    let current = options.place_on;
    let response = egui::ComboBox::from_id_salt(ui.id().with("scatter_place"))
        .selected_text(place_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for place in PlaceOn::ALL {
                if ui
                    .selectable_label(place == current, place_name(place))
                    .clicked()
                    && place != current
                {
                    options.place_on = place;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(PLACE_ON)));
    ui.end_row();
    if options.place_on == PlaceOn::Random {
        label(
            ui,
            tr("Count"),
            tr("How many copies to spread. Spacing can leave fewer, where they don't fit."),
        );
        rows.take(stepper::big_count(
            ui,
            &mut options.count,
            (1, ScatterOptions::MAX_COPIES),
            10,
            tr("Count"),
        ));
        ui.end_row();
    }
    label(
        ui,
        tr("Seed"),
        tr(
            "Another number gives another spread, and other turns and sizes. The same number \
             always gives the same result.",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut options.seed,
        (0, 99_999),
        tr("Seed"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Spacing"),
        tr(
            "The least distance between two copies, from spot to spot: a copy that would stand \
             closer is left out. 0 lets them overlap.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.spacing,
        (0.0, ScatterOptions::MAX_DISTANCE),
        units,
        tr("Spacing"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Align to surface"),
        tr(
            "Stand each copy on the surface: the shape's up turns to face out from the surface \
             where it stands, as a rock on a slope. Off, every copy stays upright.",
        ),
    );
    rows.toggle(ui, &mut options.align, tr("Align to surface"));
    ui.end_row();
    label(
        ui,
        tr("Random turn"),
        tr(
            "Turn each copy round its up direction by a random angle up to this: 360° for any \
             way at all, 0 for all the same way",
        ),
    );
    rows.take(stepper::degrees(
        ui,
        &mut options.random_turn,
        (0.0, 360.0),
        tr("Random turn"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Smallest size"),
        tr(
            "Each copy gets a random size between Smallest and Largest: 100% is the shape's own \
             size",
        ),
    );
    rows.take(stepper::percent(
        ui,
        &mut options.scale_min,
        (ScatterOptions::MIN_SCALE, ScatterOptions::MAX_SCALE),
        tr("Smallest size"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Largest size"),
        tr("The largest random size a copy gets"),
    );
    rows.take(stepper::percent(
        ui,
        &mut options.scale_max,
        (ScatterOptions::MIN_SCALE, ScatterOptions::MAX_SCALE),
        tr("Largest size"),
    ));
    ui.end_row();
    // Keep the two in order as either is dragged past the other.
    if options.scale_min > options.scale_max {
        std::mem::swap(&mut options.scale_min, &mut options.scale_max);
    }
    label(
        ui,
        tr("Offset"),
        tr(
            "Lift each copy off the surface along its outward direction, or sink it in below 0 \
             (a copy stands on its pivot: put a shape's pivot at its foot)",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.offset,
        (-ScatterOptions::MAX_DISTANCE, ScatterOptions::MAX_DISTANCE),
        units,
        tr("Offset"),
    ));
    ui.end_row();
    polygons_row(ui, settings, rows, chosen);
}

/// Which polygons of the surface take copies: all, or those chosen, with
/// buttons to choose the selected ones or go back to all.
fn polygons_row(
    ui: &mut Ui,
    settings: &mut ScatterSettings,
    rows: &mut Rows,
    chosen: Option<&[FaceId]>,
) {
    label(
        ui,
        tr("On polygons"),
        tr(
            "Copies go on the whole surface, or only on polygons you choose: select them in the \
             Polygons mode (press 3) and click Use selected",
        ),
    );
    ui.horizontal_wrapped(|ui| {
        if settings.polygons.is_empty() {
            ui.label(tr("Whole surface"));
        } else {
            ui.label(crate::trf!("{} chosen", settings.polygons.len()));
        }
        let selected = chosen.filter(|c| !c.is_empty());
        let note = if selected.is_some() {
            tr("Put copies only on the polygons selected now")
        } else {
            tr("Select polygons of this shape in the Polygons mode (press 3) first")
        };
        if ui
            .add_enabled(
                selected.is_some(),
                egui::Button::new(tr("Use selected")).small(),
            )
            .on_hover_text(note)
            .on_disabled_hover_text(note)
            .clicked()
            && let Some(selected) = selected
        {
            settings.polygons = selected.to_vec();
            rows.changed = true;
            rows.began = true;
        }
        if ui
            .add_enabled(
                !settings.polygons.is_empty(),
                egui::Button::new(tr("Whole surface")).small(),
            )
            .on_hover_text(tr("Put copies over the whole surface again"))
            .clicked()
        {
            settings.polygons.clear();
            rows.changed = true;
            rows.began = true;
        }
    });
    ui.end_row();
}
