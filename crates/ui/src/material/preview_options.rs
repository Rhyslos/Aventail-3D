//! The preview's options bar (decision 0291): **Light and backdrop**
//! (where the light comes from, what is behind the shape, and for the
//! realistic look the surroundings and exposure) and **Compare** (the
//! material beside a picture kept from before, or another graph's).
//! **Reset the view** shows under the picture once it is turned, moved
//! or zoomed (`preview_picture`).
//!
//! Nothing here changes the graph: these are ways of looking at it, so
//! they are not undo steps and are not saved with the project.

use super::MaterialEditor;
use super::comparing::{CompareHow, CompareWith};
use super::preview::{PreviewLook, PreviewShape};
use crate::text::tr;
use crate::viewport::{Environment, SceneLook};
use matgraph::GraphId;
use scene::Document;

/// The bar's accessible names.
pub(crate) const LIGHT_MENU: &str = crate::text!("Light and backdrop");
pub(crate) const COMPARE_MENU: &str = crate::text!("Compare");
pub(crate) const RESET_VIEW: &str = crate::text!("Reset the view");
pub(crate) const LIGHT_TURN: &str = crate::text!("Turn the light");
pub(crate) const KEEP_BEFORE: &str = crate::text!("Keep this as before");
pub(crate) const COMPARE_NOTHING: &str = crate::text!("Nothing");
pub(crate) const COMPARE_BEFORE: &str = crate::text!("Before");
pub(crate) const SIDE_BY_SIDE: &str = crate::text!("Side by side");
pub(crate) const SLIDER: &str = crate::text!("Slider");

/// The most the realistic look's exposure is turned up or down, in stops
/// (as the viewport's).
const MAX_EXPOSURE: f32 = 4.0;

/// What is behind the shape in the preview, with the light that goes
/// with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Backdrop {
    /// The light grey studio with a horizon (the standard).
    #[default]
    Studio,
    /// One even middle grey: nothing to throw the colours off.
    Grey,
    /// A dark grey, as many game menus and night scenes are.
    Dark,
    /// A cool blue-grey sky, a warm light and cool shade.
    Dusk,
}

impl Backdrop {
    /// Every choice, in the order listed.
    pub(crate) const ALL: [Backdrop; 4] = [Self::Studio, Self::Grey, Self::Dark, Self::Dusk];

    /// Its name in the list.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Studio => tr("Studio"),
            Self::Grey => tr("Even grey"),
            Self::Dark => tr("Dark"),
            Self::Dusk => tr("Dusk"),
        }
    }

    /// What it is good for, for its tooltip.
    pub(crate) fn explanation(self) -> &'static str {
        match self {
            Self::Studio => tr(
                "A light grey studio with a horizon and a strong light from above: shows the \
                 shape and the bumps clearly (the standard)",
            ),
            Self::Grey => tr(
                "One even middle grey behind the shape, in the studio's light: nothing around \
                 it to throw your eye off when judging colours",
            ),
            Self::Dark => tr(
                "A dark grey behind, with a softer light: how the material reads against a \
                 dark scene",
            ),
            Self::Dusk => tr(
                "A cool blue-grey sky, a warm light and cool blue shade: how the material \
                 takes coloured light",
            ),
        }
    }

    /// The views' look it draws with: its backdrop's colours and lights.
    pub(crate) fn look(self) -> SceneLook {
        match self {
            Self::Studio => SceneLook::STUDIO,
            Self::Grey => SceneLook {
                top: EVEN_GREY,
                horizon: EVEN_GREY,
                bottom: EVEN_GREY,
                horizon_line: false,
                ..SceneLook::STUDIO
            },
            Self::Dark => SceneLook::CLASSIC,
            Self::Dusk => SceneLook::DUSK,
        }
    }
}

/// The even grey backdrop: sRGB 118, about 18 % of white's light, the
/// middle grey photographers set colours against.
const EVEN_GREY: [u8; 3] = [118, 118, 118];

/// How the preview lights the 3D shapes, beyond its look (Unlit, Lit,
/// Toon, Realistic).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Lighting {
    /// What is behind the shape.
    pub(crate) backdrop: Backdrop,
    /// How far the light is turned round the shape, in degrees (0 to
    /// 360): Shift + right drag on the picture, or the slider.
    pub(crate) light_turn: f32,
    /// What surrounds the shape in the realistic look.
    pub(crate) environment: Environment,
    /// The realistic look's exposure, in stops.
    pub(crate) exposure: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            backdrop: Backdrop::Studio,
            light_turn: 0.0,
            environment: Environment::Studio,
            exposure: 0.0,
        }
    }
}

impl Lighting {
    /// The light turned by `degrees` more, kept from 0 to 360.
    pub(crate) fn turn_light(&mut self, degrees: f32) {
        if degrees.is_finite() {
            self.light_turn = (self.light_turn + degrees).rem_euclid(360.0);
        }
    }

    /// The exposure as drawn: within range, 0 if unusable.
    pub(crate) fn usable_exposure(&self) -> f32 {
        if self.exposure.is_finite() {
            self.exposure.clamp(-MAX_EXPOSURE, MAX_EXPOSURE)
        } else {
            0.0
        }
    }
}

/// Whether `look` has a light to turn (Unlit has none).
pub(crate) fn has_light(look: PreviewLook) -> bool {
    look != PreviewLook::Unlit
}

impl MaterialEditor {
    /// The bar above the picture: Light and backdrop, and Compare.
    pub(crate) fn preview_options_bar(&mut self, ui: &mut egui::Ui, document: &Document) {
        ui.horizontal_wrapped(|ui| {
            if self.preview.shape != PreviewShape::Flat {
                crate::dialog::menu(ui, tr(LIGHT_MENU), |ui| self.light_menu(ui))
                    .response
                    .on_hover_text(tr(
                        "Where the light comes from and what is behind the shape. Shift + right \
                         drag on the picture turns the light too",
                    ));
            }
            let title = match self.compare_title(document) {
                Some(with) => crate::trf!("{}: {}", tr(COMPARE_MENU), with),
                None => tr(COMPARE_MENU).to_string(),
            };
            crate::dialog::menu(ui, title, |ui| self.compare_menu(ui, document))
                .response
                .on_hover_text(tr(
                    "Show the material next to a picture kept from before, or next to another \
                     graph's material, to see what a change did",
                ));
        });
    }

    /// The Light and backdrop menu.
    fn light_menu(&mut self, ui: &mut egui::Ui) {
        let look = self.preview.look;
        let lighting = &mut self.preview.lighting;
        ui.label(egui::RichText::new(tr("Backdrop")).color(crate::theme::TEXT_WEAK));
        for backdrop in Backdrop::ALL {
            if ui
                .add(egui::Button::selectable(
                    lighting.backdrop == backdrop,
                    backdrop.label(),
                ))
                .on_hover_text(backdrop.explanation())
                .clicked()
            {
                lighting.backdrop = backdrop;
            }
        }
        ui.separator();
        ui.add_enabled_ui(has_light(look), |ui| {
            let slider = ui
                .add(
                    egui::Slider::new(&mut lighting.light_turn, 0.0..=360.0)
                        .text(tr(LIGHT_TURN))
                        .suffix("°")
                        .step_by(1.0),
                )
                .on_hover_text(tr(
                    "Turn the light round the shape to see the bumps and the shine from every \
                     side. Shift + right drag on the picture does the same",
                ))
                .on_disabled_hover_text(tr(
                    "Unlit has no light to turn: choose Lit, Toon or Realistic",
                ));
            slider.widget_info(|| {
                egui::WidgetInfo::slider(true, f64::from(lighting.light_turn), tr(LIGHT_TURN))
            });
        });
        if look != PreviewLook::Realistic {
            return;
        }
        ui.separator();
        ui.label(
            egui::RichText::new(tr(crate::viewport::ENVIRONMENT_LABEL))
                .color(crate::theme::TEXT_WEAK),
        )
        .on_hover_text(tr(
            "What surrounds the shape: the light it gets from all round and what shiny \
             surfaces reflect",
        ));
        for environment in Environment::ALL {
            ui.selectable_value(&mut lighting.environment, environment, environment.label());
        }
        ui.add(
            egui::Slider::new(&mut lighting.exposure, -MAX_EXPOSURE..=MAX_EXPOSURE)
                .text(tr(crate::viewport::EXPOSURE_LABEL))
                .step_by(0.1),
        )
        .on_hover_text(tr(
            "How bright the preview is, in stops: each step up doubles the light, each step \
             down halves it",
        ));
    }

    /// The Compare menu.
    fn compare_menu(&mut self, ui: &mut egui::Ui, document: &Document) {
        let comparing = &mut self.preview.comparing;
        if ui
            .add(egui::Button::selectable(
                comparing.with == CompareWith::Nothing,
                tr(COMPARE_NOTHING),
            ))
            .on_hover_text(tr("Only the material, on its own"))
            .clicked()
        {
            comparing.with = CompareWith::Nothing;
            ui.close();
        }
        if ui
            .add(egui::Button::selectable(
                comparing.with == CompareWith::Before,
                tr(COMPARE_BEFORE),
            ))
            .on_hover_text(tr(
                "The material as it was when you kept it, beside it as it is now: change \
                 settings and see what they did",
            ))
            .clicked()
        {
            if comparing.before.is_none() {
                comparing.keep_asked = true;
            }
            comparing.with = CompareWith::Before;
            ui.close();
        }
        if ui
            .button(tr(KEEP_BEFORE))
            .on_hover_text(tr(
                "Keeps the picture shown now, to compare the material with as you change it",
            ))
            .clicked()
        {
            comparing.keep_asked = true;
            comparing.with = CompareWith::Before;
            ui.close();
        }
        ui.separator();
        ui.label(egui::RichText::new(tr("Another graph")).color(crate::theme::TEXT_WEAK));
        let others: Vec<(GraphId, String)> = document
            .scene()
            .graphs()
            .iter()
            .filter(|(id, _)| Some(*id) != self.graph || !self.inside.is_empty())
            .map(|(id, g)| (id, g.name().to_string()))
            .collect();
        if others.is_empty() {
            crate::dialog::help(
                ui,
                tr("This project has no other graph. Make one with New graph to compare them."),
            );
        }
        for (id, name) in others {
            if ui
                .add(egui::Button::selectable(
                    comparing.with == CompareWith::Graph(id),
                    name,
                ))
                .on_hover_text(tr(
                    "That graph's material beside this one, laid the same way",
                ))
                .clicked()
            {
                comparing.with = CompareWith::Graph(id);
                ui.close();
            }
        }
        ui.separator();
        for (how, name, hint) in [
            (
                CompareHow::Slider,
                tr(SLIDER),
                tr("One picture split down a line: drag on the picture to move the line"),
            ),
            (
                CompareHow::SideBySide,
                tr(SIDE_BY_SIDE),
                tr("Two smaller pictures next to each other, turned together"),
            ),
        ] {
            if ui
                .add(egui::Button::selectable(comparing.how == how, name))
                .on_hover_text(hint)
                .clicked()
            {
                comparing.how = how;
            }
        }
    }

    /// What the material is compared with, in words for the menu's
    /// title; `None` when nothing is.
    fn compare_title(&self, document: &Document) -> Option<String> {
        match self.preview.comparing.with {
            CompareWith::Nothing => None,
            CompareWith::Before => Some(tr(COMPARE_BEFORE).to_string()),
            CompareWith::Graph(id) => document
                .scene()
                .graphs()
                .get(id)
                .map(|g| g.name().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_light_turns_round_and_stays_in_range() {
        let mut lighting = Lighting::default();
        lighting.turn_light(90.0);
        assert_eq!(lighting.light_turn, 90.0);
        lighting.turn_light(300.0);
        assert!((lighting.light_turn - 30.0).abs() < 1e-4);
        lighting.turn_light(-60.0);
        assert!((lighting.light_turn - 330.0).abs() < 1e-4);
        lighting.turn_light(f32::NAN);
        assert!((lighting.light_turn - 330.0).abs() < 1e-4, "ignored");
    }

    #[test]
    fn the_exposure_is_kept_in_range() {
        let mut lighting = Lighting {
            exposure: 9.0,
            ..Lighting::default()
        };
        assert_eq!(lighting.usable_exposure(), MAX_EXPOSURE);
        lighting.exposure = f32::NAN;
        assert_eq!(lighting.usable_exposure(), 0.0);
        lighting.exposure = -1.5;
        assert_eq!(lighting.usable_exposure(), -1.5);
    }

    #[test]
    fn every_backdrop_has_its_own_colours_and_words() {
        let looks: Vec<SceneLook> = Backdrop::ALL.iter().map(|b| b.look()).collect();
        for (i, a) in looks.iter().enumerate() {
            for b in &looks[i + 1..] {
                assert_ne!(a.backdrop(), b.backdrop());
            }
        }
        assert_eq!(Backdrop::default(), Backdrop::Studio);
        assert_eq!(Backdrop::Studio.look(), SceneLook::STUDIO, "as before");
        let [top, horizon, ground] = Backdrop::Grey.look().backdrop();
        assert_eq!(top, horizon);
        assert_eq!(horizon, ground);
        for b in Backdrop::ALL {
            assert!(!b.label().is_empty() && !b.explanation().is_empty());
        }
        assert!(!has_light(PreviewLook::Unlit));
        assert!(has_light(PreviewLook::Toon));
    }
}
