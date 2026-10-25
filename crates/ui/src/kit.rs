//! The modular kit helpers in the interface ([decision 0160](../../../docs/decisions/0160-kit-helpers.md)):
//! what is shown about Size to kit grid and the pivot presets, and the
//! Create menu's **Modular kit** list (the kit grid, Kit-sized shapes,
//! Size to kit grid and snapping to the kit grid).
//!
//! The project keeps the kit grid (`scene::KitGrid`); snapping to it is a
//! Snap setting ([`crate::transforming::SnapSettings::kit_grid`]); the
//! workbench carries out the commands (`workbench::kit`, and
//! `workbench::pivots` for the presets).

use crate::commands::{CommandId, Info};
use crate::icons::Icon;
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::{KIT_GRID_CHOICES, KitGrid};

/// The Create menu's list of kit helpers.
pub(crate) const KIT_MENU: &str = crate::text!("Modular kit");
/// The Snap setting's name, in the Modular kit list and Settings.
pub(crate) const KIT_SNAP_LABEL: &str = crate::text!("Snap moves to the kit grid");
/// What the Snap setting does.
pub(crate) const KIT_SNAP_HELP: &str = crate::text!(
    "While snapping, moves land on the project's kit grid (Create → Modular kit) instead of \
     the grid lines shown, so kit pieces line up edge to edge"
);
/// The Kit-sized shapes switch's name.
pub(crate) const KIT_SIZED_LABEL: &str = crate::text!("Kit-sized shapes");
/// What Kit-sized shapes does.
pub(crate) const KIT_SIZED_HELP: &str = crate::text!(
    "New shapes are made at the kit grid's size instead of 1 m, their outside sizes whole \
     steps of it. Saved with the project."
);

/// What is shown about the kit commands.
pub(crate) fn command_info(command: CommandId) -> Info {
    let (name, icon, help) = match command {
        CommandId::PivotPresets => (
            tr("Pivot presets"),
            Icon::PivotPresets,
            tr(
                "Ready-made places for the pivot on the shape's box: bottom centre, a bottom \
                 corner, centre or top centre. Choose this to see the four.",
            ),
        ),
        CommandId::PivotToBottomCentre => (
            tr("Pivot to bottom centre"),
            Icon::PivotToBottomCentre,
            tr(
                "Move the selected objects' pivots to the middle of their shapes' bottoms, \
                 where props stand; the shapes stay where they are",
            ),
        ),
        CommandId::PivotToBottomCorner => (
            tr("Pivot to bottom corner"),
            Icon::PivotToBottomCorner,
            tr(
                "Move the selected objects' pivots to their shapes' bottom front left corner \
                 (lowest along X and Y), where wall and floor pieces snap; the shapes stay",
            ),
        ),
        CommandId::PivotToTopCentre => (
            tr("Pivot to top centre"),
            Icon::PivotToTopCentre,
            tr(
                "Move the selected objects' pivots to the middle of their shapes' tops, for \
                 pieces that hang from a ceiling; the shapes stay where they are",
            ),
        ),
        _ => (
            tr("Size to kit grid"),
            Icon::SizeToKitGrid,
            tr(
                "Round the selected shapes' outside sizes to whole steps of the kit grid \
                 (Create → Modular kit), so pieces line up. A side under half a step, such as \
                 a wall's thickness, stays.",
            ),
        ),
    };
    (name, icon, None, help, None)
}

/// What was chosen in the Modular kit list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum KitChoice {
    /// The kit grid (or Kit-sized shapes) changed; `began` when a new
    /// drag or click began, so a drag is one undo step.
    Grid { grid: KitGrid, began: bool },
    /// Size to kit grid.
    SizeNow,
}

/// The Modular kit list: the kit grid's steps and a field for any other,
/// Kit-sized shapes, Size to kit grid and snapping to the kit grid
/// (which changes `snap_to_kit` in place). `can_size` says whether Size to
/// kit grid can be used now, or why not.
pub(crate) fn kit_entries(
    ui: &mut Ui,
    grid: KitGrid,
    units: Units,
    snap_to_kit: &mut bool,
    can_size: Result<(), String>,
) -> Option<KitChoice> {
    let mut chosen = None;
    ui.label(egui::RichText::new(tr("Kit grid")).strong())
        .on_hover_text(tr(
            "The step a modular kit's pieces are sized to, saved with the project",
        ));
    for step in KIT_GRID_CHOICES {
        if ui
            .radio(grid.step == step, units.format(step))
            .on_hover_text(crate::trf!(
                "Kit pieces in whole steps of {}",
                units.format(step)
            ))
            .clicked()
        {
            chosen = Some(KitChoice::Grid {
                grid: KitGrid { step, ..grid },
                began: true,
            });
        }
    }
    ui.horizontal(|ui| {
        let custom = !KIT_GRID_CHOICES.contains(&grid.step);
        let _ = ui.radio(custom, tr("Other:"));
        let mut step = grid.step;
        let touched = crate::attributes::stepper::length(
            ui,
            &mut step,
            (mesh::MIN_GRID_STEP, mesh::MAX_GRID_STEP),
            units,
            tr("Kit grid"),
        );
        if touched.changed {
            chosen = Some(KitChoice::Grid {
                grid: KitGrid { step, ..grid },
                began: touched.began,
            });
        }
    });
    ui.separator();
    let mut sized = grid.sized_shapes;
    if ui
        .checkbox(&mut sized, tr(KIT_SIZED_LABEL))
        .on_hover_text(tr(KIT_SIZED_HELP))
        .clicked()
    {
        chosen = Some(KitChoice::Grid {
            grid: KitGrid {
                sized_shapes: sized,
                ..grid
            },
            began: true,
        });
    }
    let info = CommandId::SizeToKitGrid.info();
    let response = ui
        .add_enabled(can_size.is_ok(), egui::Button::new(info.name))
        .on_hover_text(info.help);
    if let Err(why) = can_size {
        response.clone().on_disabled_hover_text(why);
    }
    if response.clicked() {
        chosen = Some(KitChoice::SizeNow);
        ui.close();
    }
    ui.checkbox(snap_to_kit, tr(KIT_SNAP_LABEL))
        .on_hover_text(tr(KIT_SNAP_HELP));
    chosen
}
