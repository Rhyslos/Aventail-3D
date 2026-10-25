//! The Curve points section of the Attributes panel
//! ([decision 0139](../../../../docs/decisions/0139-spline-editing.md)): in
//! the Points mode, the selected points' handle type, Round corners with
//! its radius and Even spacing with its count. The buttons ask the
//! workbench to run the same commands as the menus, so the status bar
//! says what happened and undo works the same.

use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::commands::CommandId;
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use scene::{CurvePart, CurvePointId, Document, ObjectId, PartKind, PointKind};

/// The curve commands' settings, and what the section asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CurveLook {
    /// Round corners' radius, in centimetres.
    pub(crate) radius: f32,
    /// How many points Even spacing puts along a curve; `None` keeps as
    /// many as it has.
    pub(crate) count: Option<usize>,
    /// Whether the Points mode is on (set by the workbench each frame).
    pub(crate) points_mode: bool,
    /// The command a button asked for, for the workbench to run.
    pub(crate) asked: Option<CommandId>,
}

impl Default for CurveLook {
    fn default() -> Self {
        Self {
            radius: 10.0,
            count: None,
            points_mode: false,
            asked: None,
        }
    }
}

impl CurveLook {
    /// The radius as the units show it.
    pub(crate) fn radius_text(&self, units: Units) -> String {
        units.format(self.radius)
    }
}

/// The command that gives `kind`.
fn command_for(kind: PointKind) -> CommandId {
    match kind {
        PointKind::Sharp => CommandId::CurveSharp,
        PointKind::Smooth => CommandId::CurveSmooth,
        PointKind::Broken => CommandId::CurveBroken,
        PointKind::Auto => CommandId::CurveAuto,
    }
}

impl AttributesPanel {
    /// The Curve points section, for curve `id` in the Points mode.
    pub(super) fn curve_points_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        units: Units,
    ) {
        let Some(curve) = doc
            .scene()
            .object(id)
            .and_then(|o| o.kind.curve())
            .map(|c| c.shape.curve())
        else {
            return;
        };
        let chosen: Vec<CurvePointId> = doc
            .elements(id)
            .map(|s| {
                s.points()
                    .map(|v| CurvePart::from_code(v.get()))
                    .filter(|p| p.kind == PartKind::Point && curve.point(p.point).is_some())
                    .map(|p| p.point)
                    .collect()
            })
            .unwrap_or_default();
        section(ui, tr("Curve points"));
        egui::Grid::new("attributes_curve_points")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                label(
                    ui,
                    tr("Selected"),
                    tr("Click points, or drag a box round them; Ctrl adds, Shift takes away"),
                );
                ui.label(match chosen.len() {
                    0 => tr("none").to_string(),
                    1 => tr("1 point").to_string(),
                    n => crate::trf!("{} points", n),
                });
                ui.end_row();
                self.handle_type_row(ui, &curve, &chosen);
                self.round_row(ui, &curve, &chosen, units);
                self.even_row(ui, &curve);
            });
    }

    /// The handle type of the selected points: four buttons, the one they
    /// all share lit.
    fn handle_type_row(&mut self, ui: &mut Ui, curve: &scene::Curve, chosen: &[CurvePointId]) {
        label(ui, tr("Handle type"), CommandId::CurveHandles.info().help);
        let kinds: Vec<PointKind> = chosen
            .iter()
            .filter_map(|p| curve.point(*p).map(|p| p.kind))
            .collect();
        let shared = kinds
            .first()
            .filter(|k| kinds.iter().all(|other| other == *k))
            .copied();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for kind in PointKind::ALL {
                let command = command_for(kind);
                let response = ui.add_enabled(
                    !chosen.is_empty(),
                    egui::Button::selectable(shared == Some(kind), kind.name()),
                );
                let response = response
                    .on_hover_text(command.info().help)
                    .on_disabled_hover_text(tr("Select some of the curve's points first"));
                if response.clicked() {
                    self.curve.asked = Some(command);
                }
            }
        });
        ui.end_row();
    }

    /// Round corners: its radius, and the button.
    fn round_row(
        &mut self,
        ui: &mut Ui,
        curve: &scene::Curve,
        chosen: &[CurvePointId],
        units: Units,
    ) {
        label(ui, tr("Round corners"), CommandId::CurveRound.info().help);
        let can = chosen.iter().any(|p| curve.can_round(*p));
        ui.horizontal(|ui| {
            let max = mesh::curve::MAX_RADIUS.min(100_000.0);
            stepper::length(
                ui,
                &mut self.curve.radius,
                (0.01, max),
                units,
                tr("Corner radius"),
            );
            let response = ui
                .add_enabled(can, egui::Button::new(tr("Round")))
                .on_hover_text(CommandId::CurveRound.info().help)
                .on_disabled_hover_text(tr(
                    "Select sharp corners inside the curve: not its ends, not points with \
                     handles (they are round already), not points in a straight line",
                ));
            if response.clicked() {
                self.curve.asked = Some(CommandId::CurveRound);
            }
        });
        ui.end_row();
    }

    /// Even spacing: how many points, and the button.
    fn even_row(&mut self, ui: &mut Ui, curve: &scene::Curve) {
        label(ui, tr("Even spacing"), CommandId::CurveEven.info().help);
        ui.horizontal(|ui| {
            let least = if curve.closed() { 3 } else { 2 };
            let mut count = self.curve.count.unwrap_or(curve.points().len()) as u32;
            let touched = stepper::count(
                ui,
                &mut count,
                (least, mesh::curve::MAX_CURVE_POINTS as u32),
                tr("Points along"),
            );
            if touched.changed {
                self.curve.count = Some(count as usize);
            }
            if ui
                .button(tr("Space evenly"))
                .on_hover_text(CommandId::CurveEven.info().help)
                .clicked()
            {
                self.curve.asked = Some(CommandId::CurveEven);
            }
        });
        ui.end_row();
        if self.curve.count.is_none() {
            ui.label("");
            ui.label(
                RichText::new(tr("As many points as it has now, spread evenly"))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
            ui.end_row();
        }
    }
}
