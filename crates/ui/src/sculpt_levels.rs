//! Detail levels in Sculpt mode ([decision 0315](../../../docs/decisions/0315-detail-levels.md)):
//! what the Sculpt tab's Detail levels section shows and asks for. The
//! workbench sets what is shown each frame and carries out what is
//! asked.

use crate::paint_bar::weak;
use crate::sculpt_masks::SculptAction;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use sculpt::ExportLevel;
use sculpt::levels::{HEAVY_POLYGONS, MAX_LEVELS, MAX_POLYGONS};

/// Accessible name of the list choosing the level exports and bakes take.
pub(crate) const EXPORT_LABEL: &str = crate::text!("Detail level for export and baking");
/// The most lower levels Rebuild lower levels makes at once.
pub(crate) const MOST_REBUILT: usize = 4;

/// The detail levels of the shape being sculpted, as the Sculpt tab
/// shows them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LevelSettings {
    /// The level shown and the finest there is (`None` with nothing to
    /// sculpt).
    pub(crate) levels: Option<(usize, usize)>,
    /// How many polygons the level shown has.
    pub(crate) polygons: u64,
    /// How many one more level would have.
    pub(crate) next_polygons: u64,
    /// The level exports and bakes take.
    pub(crate) export: ExportLevel,
    /// A level for exports and bakes chosen in the tab, for the
    /// workbench.
    pub(crate) export_asked: Option<ExportLevel>,
    /// How many lower levels Rebuild lower levels makes.
    pub(crate) rebuild: usize,
    /// Whether a level is being worked out (the buttons wait).
    pub(crate) busy: bool,
}

impl Default for LevelSettings {
    fn default() -> Self {
        Self {
            levels: None,
            polygons: 0,
            next_polygons: 0,
            export: ExportLevel::Shown,
            export_asked: None,
            rebuild: 2,
            busy: false,
        }
    }
}

/// `polygons` in words: "48 thousand polygons".
pub(crate) fn polygons_text(polygons: u64) -> String {
    if polygons >= 1_000_000 {
        crate::trf!(
            "{} million polygons",
            format!("{:.1}", polygons as f64 / 1e6)
        )
    } else if polygons >= 1_000 {
        crate::trf!("{} thousand polygons", (polygons as f64 / 1e3).round())
    } else {
        crate::trf!("{} polygons", polygons)
    }
}

/// The name of an export choice.
fn export_name(export: ExportLevel) -> String {
    match export {
        ExportLevel::Shown => tr("The level shown").to_string(),
        ExportLevel::Finest => tr("The finest level").to_string(),
        ExportLevel::Level(level) => crate::trf!("Level {}", level),
    }
}

/// What the Detail levels section says when closed: the level shown.
pub(crate) fn aside(settings: &LevelSettings) -> String {
    match settings.levels {
        Some((_, 0)) | None => tr("none").to_string(),
        Some((shown, top)) => crate::trf!("level {} of {}", shown, top),
    }
}

/// A button for `action`, with its explanation and `more` after it.
fn action_button(
    ui: &mut Ui,
    action: SculptAction,
    text: RichText,
    more: &str,
    asked: &mut Option<SculptAction>,
) {
    let mut hover = tr(action.explanation()).to_string();
    if !more.is_empty() {
        hover.push_str(". ");
        hover.push_str(more);
    }
    let button = ui.button(text).on_hover_text(hover);
    button.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr(action.name()))
    });
    if button.clicked() {
        *asked = Some(action);
    }
}

/// The Sculpt tab's Detail levels section.
pub(crate) fn rows(ui: &mut Ui, settings: &mut LevelSettings, asked: &mut Option<SculptAction>) {
    let Some((shown, top)) = settings.levels else {
        ui.label(
            RichText::new(tr("Select a shape to sculpt to give it detail levels."))
                .color(theme::TEXT_WEAK),
        );
        return;
    };
    ui.add_enabled_ui(!settings.busy, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(shown > 0, |ui| {
                action_button(ui, SculptAction::LowerLevel, RichText::new("◀"), "", asked);
            });
            ui.label(crate::trf!("Level {} of {}", shown, top));
            ui.add_enabled_ui(shown < top, |ui| {
                action_button(ui, SculptAction::HigherLevel, RichText::new("▶"), "", asked);
            });
            weak(ui, &polygons_text(settings.polygons));
        });
        ui.label(
            RichText::new(tr(
                "Level 0 is the shape as it came in. Each level above has four times the \
                 polygons: sculpt big forms on a low level, small detail on a high one.",
            ))
            .color(theme::TEXT_WEAK),
        );
        let next = settings.next_polygons;
        let (text, more) = if top >= MAX_LEVELS {
            (
                RichText::new(tr("Add a level")),
                crate::trf!("There are {} levels already, the most there can be", MAX_LEVELS),
            )
        } else if next > MAX_POLYGONS {
            (
                RichText::new(tr("Add a level")).color(theme::WARNING),
                crate::trf!(
                    "One more level would have {}: past {} million it is refused",
                    polygons_text(next),
                    MAX_POLYGONS / 1_000_000
                ),
            )
        } else if next > HEAVY_POLYGONS {
            (
                RichText::new(tr("Add a level")).color(theme::WARNING),
                crate::trf!(
                    "Heavy: one more level would have {}. You'll be asked first",
                    polygons_text(next)
                ),
            )
        } else {
            (
                RichText::new(tr("Add a level")),
                crate::trf!("One more level would have {}", polygons_text(next)),
            )
        };
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(top < MAX_LEVELS, |ui| {
                action_button(ui, SculptAction::AddLevel, text, &more, asked);
            });
            ui.add_enabled_ui(shown < top, |ui| {
                let action = SculptAction::DeleteHigherLevels;
                action_button(ui, action, RichText::new(tr(action.name())), "", asked);
            });
            ui.add_enabled_ui(shown > 0, |ui| {
                let action = SculptAction::DeleteLowerLevels;
                action_button(ui, action, RichText::new(tr(action.name())), "", asked);
            });
        });
        ui.horizontal_wrapped(|ui| {
            let action = SculptAction::RebuildLowerLevels;
            action_button(ui, action, RichText::new(tr(action.name())), "", asked);
            ui.add(
                egui::DragValue::new(&mut settings.rebuild)
                    .range(1..=MOST_REBUILT)
                    .suffix(tr(" levels")),
            )
            .on_hover_text(tr(
                "How many lower levels Rebuild lower levels makes: each has a quarter of the \
                 polygons of the one above",
            ));
        });
        ui.horizontal_wrapped(|ui| {
            weak(ui, tr("For export and baking"));
            let mut choice = settings.export;
            let mut choices = vec![ExportLevel::Shown, ExportLevel::Finest];
            choices.extend((0..=top).map(|k| ExportLevel::Level(k as u8)));
            let combo = egui::ComboBox::from_id_salt("sculpt_export_level")
                .selected_text(export_name(choice))
                .show_ui(ui, |ui| {
                    for c in choices {
                        ui.selectable_value(&mut choice, c, export_name(c));
                    }
                })
                .response
                .on_hover_text(tr(
                    "Which detail level an export takes, and a bake when this shape is the \
                     high-detail one: the level shown, the finest, or one by number. The view \
                     keeps showing the level you sculpt on",
                ));
            combo.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(EXPORT_LABEL))
            });
            if choice != settings.export {
                settings.export_asked = Some(choice);
            }
        });
    });
}
