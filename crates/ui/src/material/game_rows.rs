//! "Change in the game" in Attributes (Phase 12, milestone 15,
//! [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)):
//! which of a box's sliders a game should be able to change while it
//! runs. The marks are kept on the graph (`GraphEdit::ChangeInGame`, one
//! undo step each) for the engine translators, which will make them
//! settings of the engine's shader. Until they come, exports bake the
//! sliders at their value and the export check lists them.

use super::MaterialEditor;
use crate::attributes::section;
use crate::dialog::help;
use crate::text::tr;
use egui::Ui;
use matgraph::{BoxId, Graph, GraphEdit};
use scene::Document;

/// The heading over the switches.
pub(crate) const IN_GAME: &str = crate::text!("Change in the game");
/// The start of each switch's accessible name ("Change in the game:
/// Speed").
pub(crate) const IN_GAME_SWITCH: &str = crate::text!("Change in the game:");

impl MaterialEditor {
    /// A box's "Change in the game" switches: one per number, colour or
    /// switch setting.
    pub(super) fn game_rows(
        &mut self,
        ui: &mut Ui,
        b: &matgraph::GraphBox,
        id: BoxId,
        info: &matgraph::BoxInfo,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let markable: Vec<&matgraph::SettingSpec> = info
            .settings
            .iter()
            .filter(|s| matgraph::changes_in_game(s))
            .collect();
        if markable.is_empty() {
            return;
        }
        section(ui, tr(IN_GAME));
        help(
            ui,
            tr(
                "Ticked sliders are kept for a game to change while it runs (a glow that \
                brightens near the player, a river that speeds up). Exports bake them as they \
                are set for now: the engine translators that let a game change them come later.",
            ),
        );
        ui.horizontal_wrapped(|ui| {
            for spec in markable {
                let mut on = b.in_game.contains(spec.key.as_ref());
                let name = spec.name.show();
                let switch = ui
                    .checkbox(&mut on, &name)
                    .on_hover_text(tr("Keep this setting for the game to change"));
                switch.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        true,
                        on,
                        crate::trf!("{} {}", tr(IN_GAME_SWITCH), name),
                    )
                });
                if switch.changed() {
                    let edit = GraphEdit::ChangeInGame {
                        node: id,
                        key: spec.key.to_string(),
                        on,
                    };
                    let label = edit.describe();
                    if let Err(why) = self.run(document, vec![edit], &label, None, false) {
                        *note = Some(why.reason);
                    }
                }
            }
        });
    }

    /// The graph's animation and marked settings, in the graph's own
    /// Attributes: the moment exports bake, and what changes in the game.
    pub(super) fn graph_motion_rows(&mut self, ui: &mut Ui, graph: &Graph, document: &Document) {
        let registry = document.scene().graphs().registry();
        let marked = graph.in_game(registry);
        if !graph.is_animated(registry) && marked.is_empty() {
            return;
        }
        section(ui, tr("Moving and in the game"));
        if graph.is_animated(registry) {
            help(
                ui,
                crate::trf!(
                    "Some boxes move over time. Exports and the painter bake the moment at {} s; \
                     choose another with the preview's player and Bake this moment.",
                    matgraph::baking::seconds(graph.moment())
                ),
            );
        }
        for shown in marked {
            help(
                ui,
                crate::trf!(
                    "{}'s {}: changes in the game",
                    shown.info.name.show(),
                    shown.spec.name.show()
                ),
            );
        }
    }
}
