//! The Tool section: the settings of the tool in use (Extrude, Bevel,
//! Connect and Bridge, Loop cut, Groove, Slit, Weld), and the settings
//! some tools keep for their next use (Hard edges by angle's angle too).

use super::tool_rows::{
    bend_rows, bevel_option_rows, bridge_rows, cuts_row, extrude_rows, groove_rows, keep_together,
    profile_row, segments_row, through_row,
};
use super::{AttributesPanel, LABEL_WIDTH, section};
use crate::text::tr;
use crate::theme;
use crate::transforming::Tool;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use scene::Grouping;

/// Hard edges by angle's angle in degrees, for its next use: edges whose
/// polygons meet more sharply than this become hard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HardAngle(pub(crate) f32);

impl Default for HardAngle {
    fn default() -> Self {
        Self(HARD_ANGLE)
    }
}

/// Hard edges by angle's standard angle, in degrees: the corners of boxy
/// shapes come out hard and curves made of 12 or more sides soft.
pub(crate) const HARD_ANGLE: f32 = mesh::hardness::DEFAULT_ANGLE;

/// Connect's and Bridge's settings, for their next use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct JoinLook {
    /// How many cuts Connect makes along each edge.
    pub(crate) connect_cuts: u32,
    /// Bridge's rows and twist.
    pub(crate) bridge: (u32, i32),
}

impl Default for JoinLook {
    fn default() -> Self {
        Self {
            connect_cuts: 1,
            bridge: (1, 0),
        }
    }
}

/// Loop cut's settings, set in the Attributes panel before a cut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopLook {
    /// How many evenly spaced cuts.
    pub(crate) cuts: u32,
}

impl Default for LoopLook {
    fn default() -> Self {
        Self { cuts: 1 }
    }
}

/// Groove's width (in world centimetres) and sides, set in the Attributes
/// panel before a drag (decision 0086); the drag sets the depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GrooveLook {
    pub(crate) width: f32,
    pub(crate) sides: scene::GrooveSides,
}

impl Default for GrooveLook {
    /// A centimetre wide, straight sides.
    fn default() -> Self {
        Self {
            width: 1.0,
            sides: scene::GrooveSides::Straight,
        }
    }
}

/// Bevel's rows and profile, set in the Attributes panel before a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BevelLook {
    pub(crate) segments: u32,
    pub(crate) profile: f32,
    /// Skew, even width, stopping at overlaps and welding (decision 0085).
    pub(crate) options: scene::BevelOptions,
}

impl Default for BevelLook {
    fn default() -> Self {
        Self {
            segments: 1,
            profile: 1.0,
            options: scene::BevelOptions::default(),
        }
    }
}

impl AttributesPanel {
    /// Extrude's, Inset's, Bevel's or Loop cut's own settings, while it is
    /// the tool in use: they apply to the next drag or cut.
    pub(super) fn tool_section(&mut self, ui: &mut Ui, units: Units) {
        section(ui, self.tool.name());
        ui.push_id("tool", |ui| {
            egui::Grid::new("attributes_tool")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    if self.tool == Tool::Slit {
                        through_row(ui, &mut self.slit_through);
                        ui.end_row();
                    } else if self.tool == Tool::Bridge {
                        bridge_rows(ui, &mut self.join.bridge);
                    } else if self.tool == Tool::Weld || self.tool == Tool::DrawCurve {
                        // Nothing to set: the hint says how.
                    } else if self.tool == Tool::LoopCut {
                        cuts_row(ui, &mut self.loop_cut.cuts);
                        ui.end_row();
                    } else if self.tool == Tool::Bevel {
                        let mut look = self.bevel;
                        segments_row(ui, &mut look.segments);
                        ui.end_row();
                        profile_row(ui, &mut look.profile);
                        ui.end_row();
                        bevel_option_rows(ui, &mut look.options, units);
                        self.bevel = look;
                    } else if self.tool == Tool::Groove {
                        let mut look = self.groove;
                        groove_rows(ui, &mut look.width, &mut look.sides, units);
                        self.groove = look;
                    } else {
                        let mut together = self.grouping == Grouping::Together;
                        if keep_together(ui, &mut together) {
                            self.grouping = grouping_of(together);
                        }
                        ui.end_row();
                        if self.tool == Tool::Extrude {
                            let mut shape = self.extrude;
                            extrude_rows(ui, &mut shape, !self.bend.bends());
                            self.extrude = shape;
                            let mut bend = self.bend;
                            bend_rows(ui, &mut bend, units);
                            self.bend = bend;
                        }
                    }
                });
        });
        let hint = if self.tool == Tool::Bridge {
            tr(
                "Drag from one open edge to another. With two borders or two polygons selected, C \
             bridges them at once.",
            )
        } else if self.tool == Tool::Weld {
            tr(
                "Drag a point (or a polygon, in the Polygons mode) onto a point; a selected one \
             brings the whole selection.",
            )
        } else if self.tool == Tool::Slit {
            tr(
                "Click or drag points over the shape. Esc or Enter finishes the line; click the first point to close it into a loop.",
            )
        } else if self.tool == Tool::LoopCut {
            tr(
                "Point at an edge to see the loop; click to cut, or drag along the edge to slide it.",
            )
        } else if self.tool == Tool::DrawCurve {
            tr(
                "Click for a sharp point, press and drag for a smooth bend. Enter or a right click \
                 finishes, Esc cancels, Backspace takes the last point back; click the first point \
                 to close the curve. Its sweep or lathe is set once it is made.",
            )
        } else if self.tool == Tool::Bevel {
            tr("Drag away from the selected edges or points to widen. Hold Shift for fine steps.")
        } else if self.tool == Tool::Groove {
            tr(
                "Drag into the surface for a groove, out of it for a ridge. Hold Shift for fine steps.",
            )
        } else {
            tr("Drag anywhere in the viewport. Hold Shift for fine steps.")
        };
        ui.label(RichText::new(hint).small().color(theme::TEXT_WEAK));
    }
}

pub(super) fn grouping_of(together: bool) -> Grouping {
    if together {
        Grouping::Together
    } else {
        Grouping::Separate
    }
}
