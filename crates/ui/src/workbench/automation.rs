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
    LIVE_UNWRAP_HELP, LIVE_UNWRAP_LABEL, MAX_PADDING, PADDING_LABEL, PackQuality,
    STRAIGHT_STRIPS_HELP, STRAIGHT_STRIPS_LABEL, SeamHiding, UnwrapWay,
};
use crate::text::tr;
use egui::Ui;

/// The UV menu's names for the five settings submenus.
pub(crate) const SEAMS_SETTINGS_LABEL: &str = crate::text!("Seam suggestions");
pub(crate) const UNWRAP_SETTINGS_LABEL: &str = crate::text!("Unwrapping");
pub(crate) const RELAX_SETTINGS_LABEL: &str = crate::text!("Relaxing");
pub(crate) const PACK_SETTINGS_LABEL: &str = crate::text!("Packing");
pub(crate) const MEND_SETTINGS_LABEL: &str = crate::text!("Mending");

/// The heading over them.
pub(crate) const SETTINGS_HEADING: &str = crate::text!("Settings of the automatic tools");

/// The names of the settings inside them.
pub(crate) const CORNER_ANGLE_LABEL: &str = crate::text!("Cut corners sharper than");
pub(crate) const HARD_EDGES_LABEL: &str = crate::text!("Cut along hard edges");
pub(crate) const HIDING_LABEL: &str = crate::text!("Hide cuts");
pub(crate) const BUDGET_ON_LABEL: &str = crate::text!("Stretch budget for every piece");
pub(crate) const METHOD_LABEL: &str = crate::text!("Method");
pub(crate) const STRIP_ANGLE_LABEL: &str = crate::text!("Strip corners within");
pub(crate) const RELAX_STEPS_LABEL: &str = crate::text!("Strength");
pub(crate) const QUALITY_LABEL: &str = crate::text!("Fit");
pub(crate) const MEND_LABEL: &str = crate::text!("Mend UVs after each edit");
pub(crate) const UNEVEN_LABEL: &str = crate::text!("Mend when stretched");
pub(crate) const SIZE_OFF_LABEL: &str = crate::text!("Mend when sized off");

/// Marks mending changes apart from every other undo step, so a drag of
/// a limit is one step.
const MENDING_KEY: u64 = 0x4d45_4e44_0000_0001;

impl Workbench {
    /// The settings section of the UV menu: a heading and five submenus.
    pub(super) fn automation_menus(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new(tr(SETTINGS_HEADING))
                .small()
                .color(crate::theme::TEXT_WEAK),
        );
        ui.separator();
        let mut view = UvSettingsView {
            uv_settings: &mut self.uv_settings,
            document: &mut self.document,
        };
        view.menus(ui);
    }
}

/// The automatic UV tools' settings, and the document whose mending
/// settings they hold: what the UV menu's Settings submenus and the UV
/// settings panel (issue #115) show and change.
pub(crate) struct UvSettingsView<'a> {
    pub(crate) uv_settings: &'a mut super::unwrapping::UvSettings,
    pub(crate) document: &'a mut scene::Document,
}

impl UvSettingsView<'_> {
    /// The UV settings panel (issue #115): the same five groups as the UV
    /// menu's Settings, each a section that folds open.
    pub(crate) fn panel(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(
                egui::RichText::new(tr(
                    "How the automatic UV tools work: the same settings as UV → Settings.",
                ))
                .small()
                .color(crate::theme::TEXT_WEAK),
            );
            egui::CollapsingHeader::new(tr(SEAMS_SETTINGS_LABEL))
                .default_open(true)
                .show(ui, |ui| self.seams_settings(ui));
            egui::CollapsingHeader::new(tr(UNWRAP_SETTINGS_LABEL))
                .default_open(true)
                .show(ui, |ui| self.unwrap_settings(ui));
            egui::CollapsingHeader::new(tr(RELAX_SETTINGS_LABEL))
                .show(ui, |ui| self.relax_settings(ui));
            egui::CollapsingHeader::new(tr(PACK_SETTINGS_LABEL))
                .show(ui, |ui| self.packing_menu(ui));
            egui::CollapsingHeader::new(tr(MEND_SETTINGS_LABEL))
                .show(ui, |ui| self.mending_settings(ui));
        });
    }

    /// The five settings submenus.
    pub(crate) fn menus(&mut self, ui: &mut Ui) {
        ui.menu_button(tr(SEAMS_SETTINGS_LABEL), |ui| self.seams_settings(ui))
            .response
            .on_hover_text(tr("How Suggest seams picks seams"));
        ui.menu_button(tr(UNWRAP_SETTINGS_LABEL), |ui| self.unwrap_settings(ui))
            .response
            .on_hover_text(tr("How Unwrap lays pieces flat"));
        ui.menu_button(tr(RELAX_SETTINGS_LABEL), |ui| self.relax_settings(ui))
            .response
            .on_hover_text(tr("How much Relax evens UVs out"));
        ui.menu_button(tr(PACK_SETTINGS_LABEL), |ui| self.packing_menu(ui))
            .response
            .on_hover_text(tr("How Pack fits pieces into the square"));
        ui.menu_button(tr(MEND_SETTINGS_LABEL), |ui| self.mending_settings(ui))
            .response
            .on_hover_text(tr(
                "How modelling edits fix the UVs they break (saved with the project)",
            ));
    }

    pub(crate) fn seams_settings(&mut self, ui: &mut Ui) {
        let s = &mut self.uv_settings;
        ui.horizontal(|ui| {
            ui.label(tr(CORNER_ANGLE_LABEL));
            ui.add(
                egui::DragValue::new(&mut s.seam_corner_angle)
                    .range(0.0..=180.0)
                    .speed(0.5)
                    .suffix("°"),
            )
            .on_hover_text(tr(
                "Edges where the surface bends more sharply than this get a seam (66° \
                 standard): a piece bent far round one edge stretches when laid flat. 180° \
                 cuts no corners.",
            ));
        });
        ui.checkbox(&mut s.seam_hard_edges, tr(HARD_EDGES_LABEL))
            .on_hover_text(tr(
                "Every hard edge gets a seam: a normal-map bake needs the UVs split wherever \
                 the shading breaks. Off: only corners and what pieces need to lie flat.",
            ));
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.budget_on, tr(BUDGET_ON_LABEL)).on_hover_text(
                tr("When on, Suggest seams and Unwrap cut each piece further, a slit at a time from \
                 its worst place to its border, until no polygon is more than this many times \
                 off. Pieces given a budget of their own (UV → Stretch budget) keep theirs."),
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
        ui.label(tr(HIDING_LABEL)).on_hover_text(tr(
            "Where cuts go that only make a piece lie flat (Suggest seams, and Unwrap on a \
             closed piece)",
        ));
        for hiding in SeamHiding::ALL {
            ui.radio_value(&mut s.seam_hiding, hiding, hiding.name())
                .on_hover_text(hiding.help());
        }
    }

    pub(crate) fn unwrap_settings(&mut self, ui: &mut Ui) {
        let s = &mut self.uv_settings;
        ui.label(tr(METHOD_LABEL))
            .on_hover_text(tr("How each piece is laid flat"));
        for way in UnwrapWay::ALL {
            ui.radio_value(&mut s.unwrap_method, way, way.name())
                .on_hover_text(way.help());
        }
        ui.separator();
        ui.checkbox(&mut s.straighten_strips, tr(STRAIGHT_STRIPS_LABEL))
            .on_hover_text(tr(STRAIGHT_STRIPS_HELP));
        ui.add_enabled_ui(s.straighten_strips, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr(STRIP_ANGLE_LABEL));
                ui.add(
                    egui::DragValue::new(&mut s.strip_angle)
                        .range(0.0..=uv::unwrap::MAX_STRIP_ANGLE)
                        .speed(0.5)
                        .suffix(tr("° of square")),
                )
                .on_hover_text(tr(
                    "How far a quad's corners may be from square for its piece to count as a \
                     strip (25° standard). Raise it for strips round tight curves; lower it to \
                     straighten only truly square ones.",
                ));
            });
        });
        ui.checkbox(&mut s.live_unwrap, tr(LIVE_UNWRAP_LABEL))
            .on_hover_text(tr(LIVE_UNWRAP_HELP));
        ui.label(
            egui::RichText::new(tr(
                "Where closed pieces are cut open, and the stretch budget: Seam suggestions.",
            ))
            .small()
            .color(crate::theme::TEXT_WEAK),
        );
    }

    pub(crate) fn relax_settings(&mut self, ui: &mut Ui) {
        let s = &mut self.uv_settings;
        ui.horizontal(|ui| {
            ui.label(tr(RELAX_STEPS_LABEL));
            ui.add(
                egui::DragValue::new(&mut s.relax_steps)
                    .range(1..=uv::unwrap::MAX_RELAX_STEPS)
                    .speed(1.0)
                    .suffix(tr(" steps")),
            )
            .on_hover_text(tr(
                "How many steps of evening out Relax takes (60 standard). More evens stretched \
                 UVs out further; a few just smooths them a little.",
            ));
        });
    }

    /// The Packing settings.
    pub(crate) fn pack_quality_entries(&mut self, ui: &mut Ui) {
        ui.label(tr(QUALITY_LABEL))
            .on_hover_text(tr("How finely pieces are fitted together"));
        for quality in PackQuality::ALL {
            ui.radio_value(&mut self.uv_settings.pack_quality, quality, quality.name())
                .on_hover_text(quality.help());
        }
    }

    pub(crate) fn mending_settings(&mut self, ui: &mut Ui) {
        let mut mending = self.document.scene().mending();
        let mut changed = ui
            .checkbox(&mut mending.on, tr(MEND_LABEL))
            .on_hover_text(tr(
                "When on (standard), each modelling edit lays out again the UVs it broke \
                 (squashed, stretched, mirrored or far off in size), around the UVs it kept. \
                 Off: edits leave UVs as they come out, for you to unwrap.",
            ))
            .changed();
        ui.add_enabled_ui(mending.on, |ui| {
            for (label, value, help) in [
                (
                    tr(UNEVEN_LABEL),
                    &mut mending.uneven,
                    tr(
                        "A polygon an edit changed counts as broken when it is stretched this many \
                     times more one way than the other (3× standard). Lower mends more.",
                    ),
                ),
                (
                    tr(SIZE_OFF_LABEL),
                    &mut mending.size_off,
                    tr(
                        "A polygon an edit changed counts as broken when its texture is this many \
                     times bigger or smaller than the polygons' around it (3× standard). Lower \
                     mends more.",
                    ),
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
            egui::RichText::new(tr(
                "Saved with the project; every edit is worked out again.",
            ))
            .small()
            .color(crate::theme::TEXT_WEAK),
        );
        if changed && mending.is_real() {
            let _ = self.document.execute_labelled(
                scene::Command::SetMending(mending),
                tr("Change how edits keep UVs"),
                Some(MENDING_KEY),
            );
        }
    }

    /// The UV menu's Packing entries: the padding and the switches.
    pub(crate) fn packing_menu(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.uv_settings;
        ui.horizontal(|ui| {
            ui.label(tr(PADDING_LABEL));
            ui.add(
                egui::DragValue::new(&mut s.padding)
                    .range(0..=MAX_PADDING)
                    .suffix(tr(" px")),
            )
            .on_hover_text(tr(
                "The gap between pieces, in texture pixels (half of it to the square's edges). \
                 Keeps colours from bleeding between pieces when the texture is shrunk; 8 px \
                 suits a 2048 texture.",
            ));
        });
        ui.checkbox(&mut s.rotate, tr("Turn pieces to fit"))
            .on_hover_text(tr(
                "Pieces may be turned (lined up with the square, and a quarter turn) to fit better",
            ));
        ui.checkbox(&mut s.stack, tr("Stack identical pieces"))
            .on_hover_text(tr(
                "Pieces of exactly the same shape share the same texture space",
            ));
        ui.checkbox(&mut s.per_material, tr("A texture per material"))
            .on_hover_text(tr(
                "Each material's pieces fill a square of their own, as separate texture sets",
            ));
        ui.checkbox(&mut s.per_object, tr("A texture per object"))
            .on_hover_text(tr("Each object's pieces fill a square of their own"));
        ui.separator();
        self.pack_quality_entries(ui);
    }
}
