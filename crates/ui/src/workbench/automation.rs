//! Every setting of the automatic UV tools, in the UV menu's last section
//! ([decision 0110](../../../../docs/decisions/0110-controllable-automation.md)):
//! Seam suggestions, Unwrapping, Relaxing, Packing and Mending.
//!
//! Nothing the tools decide is hidden or random: each choice is a setting
//! here, and the same shape and settings always give the same result. The
//! first four are the app's (remembered between sessions, and recorded
//! with each edit so it replays the same later); mending is the
//! project's, saved with it, since it works on every edit as the shape is
//! worked out.

use super::Workbench;
use super::unwrapping::{
    LIVE_UNWRAP_HELP, LIVE_UNWRAP_LABEL, PackQuality, STRAIGHT_STRIPS_HELP, STRAIGHT_STRIPS_LABEL,
    SeamHiding, UnwrapWay,
};
use egui::Ui;

/// The UV menu's names for the five settings submenus.
pub(crate) const SEAMS_SETTINGS_LABEL: &str = "Seam suggestions";
pub(crate) const UNWRAP_SETTINGS_LABEL: &str = "Unwrapping";
pub(crate) const RELAX_SETTINGS_LABEL: &str = "Relaxing";
pub(crate) const PACK_SETTINGS_LABEL: &str = "Packing";
pub(crate) const MEND_SETTINGS_LABEL: &str = "Mending";

/// The heading over them.
pub(crate) const SETTINGS_HEADING: &str = "Settings of the automatic tools";

/// The names of the settings inside them.
pub(crate) const CORNER_ANGLE_LABEL: &str = "Cut corners sharper than";
pub(crate) const HARD_EDGES_LABEL: &str = "Cut along hard edges";
pub(crate) const HIDING_LABEL: &str = "Hide cuts";
pub(crate) const BUDGET_ON_LABEL: &str = "Stretch budget for every piece";
pub(crate) const METHOD_LABEL: &str = "Method";
pub(crate) const STRIP_ANGLE_LABEL: &str = "Strip corners within";
pub(crate) const RELAX_STEPS_LABEL: &str = "Strength";
pub(crate) const QUALITY_LABEL: &str = "Fit";
pub(crate) const MEND_LABEL: &str = "Mend UVs after each edit";
pub(crate) const UNEVEN_LABEL: &str = "Mend when stretched";
pub(crate) const SIZE_OFF_LABEL: &str = "Mend when sized off";

/// Marks mending changes apart from every other undo step, so a drag of
/// a limit is one step.
const MENDING_KEY: u64 = 0x4d45_4e44_0000_0001;

impl Workbench {
    /// The settings section of the UV menu: a heading and five submenus.
    pub(super) fn automation_menus(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new(SETTINGS_HEADING)
                .small()
                .color(crate::theme::TEXT_WEAK),
        );
        ui.separator();
        ui.menu_button(SEAMS_SETTINGS_LABEL, |ui| self.seams_settings(ui))
            .response
            .on_hover_text("How Suggest seams picks seams");
        ui.menu_button(UNWRAP_SETTINGS_LABEL, |ui| self.unwrap_settings(ui))
            .response
            .on_hover_text("How Unwrap lays pieces flat");
        ui.menu_button(RELAX_SETTINGS_LABEL, |ui| self.relax_settings(ui))
            .response
            .on_hover_text("How much Relax evens UVs out");
        ui.menu_button(PACK_SETTINGS_LABEL, |ui| self.packing_menu(ui))
            .response
            .on_hover_text("How Pack fits pieces into the square");
        ui.menu_button(MEND_SETTINGS_LABEL, |ui| self.mending_settings(ui))
            .response
            .on_hover_text("How modelling edits fix the UVs they break (saved with the project)");
    }

    fn seams_settings(&mut self, ui: &mut Ui) {
        let s = &mut self.uv_settings;
        ui.horizontal(|ui| {
            ui.label(CORNER_ANGLE_LABEL);
            ui.add(
                egui::DragValue::new(&mut s.seam_corner_angle)
                    .range(0.0..=180.0)
                    .speed(0.5)
                    .suffix("°"),
            )
            .on_hover_text(
                "Edges where the surface bends more sharply than this get a seam (66° \
                 standard): a piece bent far round one edge stretches when laid flat. 180° \
                 cuts no corners.",
            );
        });
        ui.checkbox(&mut s.seam_hard_edges, HARD_EDGES_LABEL)
            .on_hover_text(
                "Every hard edge gets a seam: a normal-map bake needs the UVs split wherever \
                 the shading breaks. Off: only corners and what pieces need to lie flat.",
            );
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.budget_on, BUDGET_ON_LABEL).on_hover_text(
                "When on, Suggest seams and Unwrap cut each piece further, a slit at a time from \
                 its worst place to its border, until no polygon is more than this many times \
                 off. Pieces given a budget of their own (UV → Stretch budget) keep theirs.",
            );
            ui.add_enabled(
                s.budget_on,
                egui::DragValue::new(&mut s.budget)
                    .range(scene::BUDGET_RANGE)
                    .speed(0.01)
                    .max_decimals(2)
                    .suffix("×"),
            );
        });
        ui.separator();
        ui.label(HIDING_LABEL).on_hover_text(
            "Where cuts go that only make a piece lie flat (Suggest seams, and Unwrap on a \
             closed piece)",
        );
        for hiding in SeamHiding::ALL {
            ui.radio_value(&mut s.seam_hiding, hiding, hiding.name())
                .on_hover_text(hiding.help());
        }
    }

    fn unwrap_settings(&mut self, ui: &mut Ui) {
        let s = &mut self.uv_settings;
        ui.label(METHOD_LABEL)
            .on_hover_text("How each piece is laid flat");
        for way in UnwrapWay::ALL {
            ui.radio_value(&mut s.unwrap_method, way, way.name())
                .on_hover_text(way.help());
        }
        ui.separator();
        ui.checkbox(&mut s.straighten_strips, STRAIGHT_STRIPS_LABEL)
            .on_hover_text(STRAIGHT_STRIPS_HELP);
        ui.add_enabled_ui(s.straighten_strips, |ui| {
            ui.horizontal(|ui| {
                ui.label(STRIP_ANGLE_LABEL);
                ui.add(
                    egui::DragValue::new(&mut s.strip_angle)
                        .range(0.0..=uv::unwrap::MAX_STRIP_ANGLE)
                        .speed(0.5)
                        .suffix("° of square"),
                )
                .on_hover_text(
                    "How far a quad's corners may be from square for its piece to count as a \
                     strip (25° standard). Raise it for strips round tight curves; lower it to \
                     straighten only truly square ones.",
                );
            });
        });
        ui.checkbox(&mut s.live_unwrap, LIVE_UNWRAP_LABEL)
            .on_hover_text(LIVE_UNWRAP_HELP);
        ui.label(
            egui::RichText::new(
                "Where closed pieces are cut open, and the stretch budget: Seam suggestions.",
            )
            .small()
            .color(crate::theme::TEXT_WEAK),
        );
    }

    fn relax_settings(&mut self, ui: &mut Ui) {
        let s = &mut self.uv_settings;
        ui.horizontal(|ui| {
            ui.label(RELAX_STEPS_LABEL);
            ui.add(
                egui::DragValue::new(&mut s.relax_steps)
                    .range(1..=uv::unwrap::MAX_RELAX_STEPS)
                    .speed(1.0)
                    .suffix(" steps"),
            )
            .on_hover_text(
                "How many steps of evening out Relax takes (60 standard). More evens stretched \
                 UVs out further; a few just smooths them a little.",
            );
        });
    }

    /// The Packing settings.
    pub(crate) fn pack_quality_entries(&mut self, ui: &mut Ui) {
        ui.label(QUALITY_LABEL)
            .on_hover_text("How finely pieces are fitted together");
        for quality in PackQuality::ALL {
            ui.radio_value(&mut self.uv_settings.pack_quality, quality, quality.name())
                .on_hover_text(quality.help());
        }
    }

    fn mending_settings(&mut self, ui: &mut Ui) {
        let mut mending = self.document.scene().mending();
        let mut changed = ui
            .checkbox(&mut mending.on, MEND_LABEL)
            .on_hover_text(
                "When on (standard), each modelling edit lays out again the UVs it broke \
                 (squashed, stretched, mirrored or far off in size), around the UVs it kept. \
                 Off: edits leave UVs as they come out, for you to unwrap.",
            )
            .changed();
        ui.add_enabled_ui(mending.on, |ui| {
            for (label, value, help) in [
                (
                    UNEVEN_LABEL,
                    &mut mending.uneven,
                    "A polygon an edit changed counts as broken when it is stretched this many \
                     times more one way than the other (3× standard). Lower mends more.",
                ),
                (
                    SIZE_OFF_LABEL,
                    &mut mending.size_off,
                    "A polygon an edit changed counts as broken when its texture is this many \
                     times bigger or smaller than the polygons' around it (3× standard). Lower \
                     mends more.",
                ),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    changed |= ui
                        .add(
                            egui::DragValue::new(value)
                                .range(uv::mend::LIMITS)
                                .speed(0.05)
                                .max_decimals(1)
                                .suffix("×"),
                        )
                        .on_hover_text(help)
                        .changed();
                });
            }
        });
        ui.label(
            egui::RichText::new("Saved with the project; every edit is worked out again.")
                .small()
                .color(crate::theme::TEXT_WEAK),
        );
        if changed && mending.is_real() {
            let _ = self.document.execute_labelled(
                scene::Command::SetMending(mending),
                "Change how edits keep UVs",
                Some(MENDING_KEY),
            );
        }
    }
}
