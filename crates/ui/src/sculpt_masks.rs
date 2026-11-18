//! Symmetry and masks in Sculpt mode ([decision 0310](../../../docs/decisions/0310-sculpt-symmetry-and-masks.md)):
//! their settings, what the Mask brush draws with (a brush, a box or a
//! lasso), the actions on a whole mask and the symmetry (each a command
//! in the Sculpt menu, the Sculpt tab and command search), and the Sculpt
//! tab's Symmetry and Mask sections.

use crate::text::tr;
use crate::theme;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Ui};
use sculpt::Symmetry;

/// What the Mask brush draws with (9, and 9 again).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MaskTool {
    /// Paints the mask with the brush.
    #[default]
    Brush,
    /// A box dragged on the view masks everything inside it.
    Box,
    /// A lasso drawn on the view masks everything inside it.
    Lasso,
}

impl MaskTool {
    /// Its name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Brush => tr("Mask brush"),
            Self::Box => tr("Box mask"),
            Self::Lasso => tr("Lasso mask"),
        }
    }

    /// The next one, as 9 goes round them.
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Brush => Self::Box,
            Self::Box => Self::Lasso,
            Self::Lasso => Self::Brush,
        }
    }
}

/// Sculpt mode's symmetry settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SymmetrySettings {
    /// On (L).
    pub(crate) on: bool,
    /// Across X (left and right), Y (front and back), Z (up and down).
    pub(crate) axes: [bool; 3],
    /// Radial copies round an axis.
    pub(crate) radial: bool,
    /// The radial axis (0 X, 1 Y, 2 Z).
    pub(crate) radial_axis: usize,
    /// How many radial copies.
    pub(crate) radial_count: u32,
}

impl Default for SymmetrySettings {
    fn default() -> Self {
        Self {
            on: false,
            axes: [true, false, false],
            radial: false,
            radial_axis: 2,
            radial_count: 8,
        }
    }
}

impl SymmetrySettings {
    /// The symmetry a dab is repeated with now.
    pub(crate) fn symmetry(&self) -> Symmetry {
        if !self.on {
            return Symmetry::NONE;
        }
        Symmetry {
            axes: self.axes,
            radial: self.radial.then_some((
                self.radial_axis.min(2),
                self.radial_count.clamp(2, sculpt::symmetry::MOST_RADIAL),
            )),
        }
    }

    /// The axis Make symmetrical works across: the first chosen (X when
    /// none is).
    pub(crate) fn first_axis(&self) -> usize {
        self.axes.iter().position(|a| *a).unwrap_or(0)
    }
}

/// An action of Sculpt mode on a whole mask or the symmetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SculptAction {
    /// Turn the mask over (Ctrl + I).
    InvertMask,
    /// Take the whole mask away (Alt + M).
    ClearMask,
    /// Grow the mask by a ring of points (Ctrl + wheel up).
    GrowMask,
    /// Shrink it by a ring (Ctrl + wheel down).
    ShrinkMask,
    /// Make the mask's soft edge harder.
    SharpenMask,
    /// Soften its edge.
    SoftenMask,
    /// Mask the creases.
    MaskFromCavity,
    /// Mask the open edges.
    MaskFromEdges,
    /// Mask what faces up.
    MaskFacingUp,
    /// Mask what faces down.
    MaskFacingDown,
    /// Mask what faces the view.
    MaskFacingView,
    /// The masked part becomes a new object with a thickness.
    ShapeFromMask,
    /// Symmetry on or off (L).
    Symmetry,
    /// Copy the + side's sculpting onto the − side.
    MakeSymmetricalPlus,
    /// Copy the − side's onto the + side.
    MakeSymmetricalMinus,
}

impl SculptAction {
    /// Every action, in menu order.
    pub const ALL: [Self; 15] = [
        Self::Symmetry,
        Self::MakeSymmetricalPlus,
        Self::MakeSymmetricalMinus,
        Self::InvertMask,
        Self::ClearMask,
        Self::GrowMask,
        Self::ShrinkMask,
        Self::SharpenMask,
        Self::SoftenMask,
        Self::MaskFromCavity,
        Self::MaskFromEdges,
        Self::MaskFacingUp,
        Self::MaskFacingDown,
        Self::MaskFacingView,
        Self::ShapeFromMask,
    ];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::InvertMask => crate::text!("Invert mask"),
            Self::ClearMask => crate::text!("Clear mask"),
            Self::GrowMask => crate::text!("Grow mask"),
            Self::ShrinkMask => crate::text!("Shrink mask"),
            Self::SharpenMask => crate::text!("Sharpen mask"),
            Self::SoftenMask => crate::text!("Soften mask"),
            Self::MaskFromCavity => crate::text!("Mask from cavity"),
            Self::MaskFromEdges => crate::text!("Mask from the edges"),
            Self::MaskFacingUp => crate::text!("Mask what faces up"),
            Self::MaskFacingDown => crate::text!("Mask what faces down"),
            Self::MaskFacingView => crate::text!("Mask what faces the view"),
            Self::ShapeFromMask => crate::text!("Make a shape from the mask"),
            Self::Symmetry => crate::text!("Symmetry on or off"),
            Self::MakeSymmetricalPlus => crate::text!("Make symmetrical: + side to − side"),
            Self::MakeSymmetricalMinus => crate::text!("Make symmetrical: − side to + side"),
        }
    }

    /// What it does, in plain words.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::InvertMask => crate::text!(
                "Masked parts become free and free parts masked: mask what you want to keep, then invert to work on everything else"
            ),
            Self::ClearMask => {
                crate::text!("Takes the whole mask away, so every part can be sculpted again")
            }
            Self::GrowMask => {
                crate::text!("Grows the mask by one ring of points (Ctrl + wheel up over the view)")
            }
            Self::ShrinkMask => {
                crate::text!(
                    "Shrinks the mask by one ring of points (Ctrl + wheel down over the view)"
                )
            }
            Self::SharpenMask => crate::text!("Makes the mask's soft edge harder"),
            Self::SoftenMask => {
                crate::text!("Softens the mask's edge, so a move there fades out gently")
            }
            Self::MaskFromCavity => crate::text!(
                "Masks the creases and hollows, so you can sculpt the raised parts without filling them"
            ),
            Self::MaskFromEdges => {
                crate::text!("Masks the edge of an open shape, so it stays where it is")
            }
            Self::MaskFacingUp => crate::text!("Masks the parts that face up"),
            Self::MaskFacingDown => crate::text!("Masks the parts that face down"),
            Self::MaskFacingView => crate::text!("Masks the parts that face you in the view"),
            Self::ShapeFromMask => crate::text!(
                "Makes a new object from the masked part, with a thickness: armour plates, belts and clothes from a body"
            ),
            Self::Symmetry => crate::text!(
                "Repeats every stroke on the other side, across the shape's own planes through its pivot (L)"
            ),
            Self::MakeSymmetricalPlus => crate::text!(
                "Copies the sculpting on the + side of the first symmetry axis onto the − side, for a sculpt that drifted"
            ),
            Self::MakeSymmetricalMinus => crate::text!(
                "Copies the sculpting on the − side of the first symmetry axis onto the + side, for a sculpt that drifted"
            ),
        }
    }

    /// Its keys, if it has any.
    pub fn shortcut(self) -> Option<KeyboardShortcut> {
        match self {
            Self::InvertMask => Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::I)),
            Self::ClearMask => Some(KeyboardShortcut::new(Modifiers::ALT, Key::M)),
            Self::Symmetry => Some(KeyboardShortcut::new(Modifiers::NONE, Key::L)),
            _ => None,
        }
    }

    /// Whether it needs a mask to do anything.
    pub(crate) fn needs_mask(self) -> bool {
        matches!(
            self,
            Self::InvertMask
                | Self::ClearMask
                | Self::GrowMask
                | Self::ShrinkMask
                | Self::SharpenMask
                | Self::SoftenMask
                | Self::ShapeFromMask
        )
    }
}

/// The Sculpt tab's Symmetry section (decision 0310).
pub(crate) fn symmetry_rows(
    ui: &mut Ui,
    settings: &mut SymmetrySettings,
    asked: &mut Option<SculptAction>,
) {
    crate::attributes::section(ui, tr("Symmetry"));
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut settings.on, tr("Symmetry"))
            .on_hover_text(tr(SculptAction::Symmetry.explanation()));
        ui.add_enabled_ui(settings.on, |ui| {
            for (axis, name) in ["X", "Y", "Z"].iter().enumerate() {
                ui.toggle_value(&mut settings.axes[axis], *name)
                    .on_hover_text(tr(
                        "Reflect across the plane square to this axis: X left and right, Y front and back, Z up and down",
                    ));
            }
        });
    });
    ui.add_enabled_ui(settings.on, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut settings.radial, tr("Radial"))
                .on_hover_text(tr(
                    "Repeats each stroke round an axis, for flowers, wheels, crowns and tentacles",
                ));
            ui.add_enabled_ui(settings.radial, |ui| {
                ui.add(
                    egui::DragValue::new(&mut settings.radial_count)
                        .range(2..=sculpt::symmetry::MOST_RADIAL)
                        .suffix(tr(" copies")),
                );
                for (axis, name) in ["X", "Y", "Z"].iter().enumerate() {
                    ui.selectable_value(&mut settings.radial_axis, axis, *name)
                        .on_hover_text(tr("The axis the copies turn round"));
                }
            });
        });
    });
    ui.horizontal_wrapped(|ui| {
        for action in [
            SculptAction::MakeSymmetricalPlus,
            SculptAction::MakeSymmetricalMinus,
        ] {
            if ui
                .button(tr(action.name()))
                .on_hover_text(tr(action.explanation()))
                .clicked()
            {
                *asked = Some(action);
            }
        }
    });
}

/// The Sculpt tab's Mask section (decision 0310).
pub(crate) fn mask_rows(
    ui: &mut Ui,
    tool: &mut MaskTool,
    thickness: &mut f32,
    asked: &mut Option<SculptAction>,
) {
    crate::attributes::section(ui, tr("Mask"));
    ui.horizontal_wrapped(|ui| {
        for t in [MaskTool::Brush, MaskTool::Box, MaskTool::Lasso] {
            ui.selectable_value(tool, t, t.name()).on_hover_text(tr(
                "What the Mask brush (9) draws with: press 9 again to go round them",
            ));
        }
    });
    ui.horizontal_wrapped(|ui| {
        for action in SculptAction::ALL.into_iter().filter(|a| {
            !matches!(
                a,
                SculptAction::Symmetry
                    | SculptAction::MakeSymmetricalPlus
                    | SculptAction::MakeSymmetricalMinus
                    | SculptAction::ShapeFromMask
            )
        }) {
            if ui
                .button(tr(action.name()))
                .on_hover_text(tr(action.explanation()))
                .clicked()
            {
                *asked = Some(action);
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        let action = SculptAction::ShapeFromMask;
        if ui
            .button(tr(action.name()))
            .on_hover_text(tr(action.explanation()))
            .clicked()
        {
            *asked = Some(action);
        }
        ui.label(RichText::new(tr("Thickness")).color(theme::TEXT_WEAK));
        ui.add(
            egui::DragValue::new(thickness)
                .range(0.01..=1000.0)
                .speed(0.05)
                .suffix(" cm"),
        )
        .on_hover_text(tr("How thick the shape made from the mask is"));
    });
}
