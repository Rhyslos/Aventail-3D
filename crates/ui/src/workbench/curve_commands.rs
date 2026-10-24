//! The curve commands (decision 0139): whether each can be used now and
//! why not, and carrying them out. Split, the handle types and Round
//! corners work on the selected points (the Points mode); Reverse, Open or
//! close and Even spacing on whole selected curves (the Model or Points
//! mode). M and A on two curve end points join them (weld, or a straight
//! stretch); Join objects on curves joins them end to end.

use super::Workbench;
use crate::commands::CommandId;
use crate::palette::Mode;
use crate::text::tr;
use crate::tool_menu::Ready;
use scene::{Command, CurveEdit, CurvePart, CurvePointId, ObjectId, PartKind, PointKind};

/// The handle kind a command gives.
fn kind_of(command: CommandId) -> Option<PointKind> {
    match command {
        CommandId::CurveSharp => Some(PointKind::Sharp),
        CommandId::CurveSmooth => Some(PointKind::Smooth),
        CommandId::CurveBroken => Some(PointKind::Broken),
        CommandId::CurveAuto => Some(PointKind::Auto),
        _ => None,
    }
}

/// Whether `command` is one of the curve commands.
pub(super) fn is_curve_command(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::CurveSplit
            | CommandId::CurveReverse
            | CommandId::CurveOpenClose
            | CommandId::CurveHandles
            | CommandId::CurveRound
            | CommandId::CurveEven
    ) || kind_of(command).is_some()
}

impl Workbench {
    /// The selected curve objects, in the order they were selected.
    pub(super) fn selected_curves(&self) -> Vec<ObjectId> {
        let scene = self.document.scene();
        let chosen = self.document.selection();
        self.document
            .selection_order()
            .iter()
            .copied()
            .filter(|id| chosen.contains(id))
            .filter(|id| scene.object(*id).is_some_and(|o| o.kind.curve().is_some()))
            .collect()
    }

    /// Whether only curves are selected (at least one): the E menu shows
    /// the curve ring in the Points mode.
    pub(super) fn only_curves_selected(&self) -> bool {
        let selection = self.document.selection();
        !selection.is_empty() && self.selected_curves().len() == selection.len()
    }

    /// The selected curves' selected points (not handles), in the Points
    /// mode.
    fn selected_curve_points(&self) -> Vec<(ObjectId, Vec<CurvePointId>)> {
        if self.mode != Mode::Points {
            return Vec::new();
        }
        self.selected_curves()
            .into_iter()
            .filter_map(|id| {
                let points: Vec<CurvePointId> = self
                    .document
                    .elements(id)?
                    .points()
                    .map(|v| CurvePart::from_code(v.get()))
                    .filter(|p| p.kind == PartKind::Point)
                    .map(|p| p.point)
                    .collect();
                (!points.is_empty()).then_some((id, points))
            })
            .collect()
    }

    /// The curve `id`'s points now.
    fn curve_now(&self, id: ObjectId) -> Option<scene::Curve> {
        Some(
            self.document
                .scene()
                .object(id)?
                .kind
                .curve()?
                .shape
                .curve(),
        )
    }

    /// Whether a curve command can be used now, and if not, why.
    pub(super) fn curve_readiness(&self, command: CommandId) -> Ready {
        let not_now = |why: &str| Ready::NotNow(why.into());
        let whole = matches!(
            command,
            CommandId::CurveReverse | CommandId::CurveOpenClose | CommandId::CurveEven
        );
        if self.selected_curves().is_empty() {
            return not_now(tr("Select a curve first"));
        }
        if whole {
            if !matches!(self.mode, Mode::Model | Mode::Points) {
                return not_now(tr(
                    "This works on whole curves: press 1 for the Points mode",
                ));
            }
            if command == CommandId::CurveOpenClose {
                let small = self.selected_curves().iter().all(|id| {
                    self.curve_now(*id)
                        .is_some_and(|c| !c.closed() && c.points().len() < 3)
                });
                if small {
                    return not_now(tr("A closed curve needs at least three points"));
                }
            }
            return Ready::Yes;
        }
        if self.mode != Mode::Points {
            return not_now(tr(
                "This works on a curve's points: press 1 for the Points mode and select some",
            ));
        }
        let chosen = self.selected_curve_points();
        if chosen.is_empty() {
            return not_now(tr("Select some of the curve's points first"));
        }
        match command {
            CommandId::CurveSplit => {
                let inside = chosen.iter().any(|(id, points)| {
                    self.curve_now(*id)
                        .is_some_and(|c| points.iter().any(|p| c.end_of(*p).is_none()))
                });
                if inside {
                    Ready::Yes
                } else {
                    not_now(tr(
                        "Select a point inside the curve to cut at: an open curve's ends cut \
                         nothing",
                    ))
                }
            }
            CommandId::CurveRound => {
                let any = chosen.iter().any(|(id, points)| {
                    self.curve_now(*id)
                        .is_some_and(|c| points.iter().any(|p| c.can_round(*p)))
                });
                if any {
                    Ready::Yes
                } else {
                    not_now(tr(
                        "Select sharp corners inside the curve: not its ends, not points with \
                         handles (they are round already), not points in a straight line",
                    ))
                }
            }
            _ => Ready::Yes,
        }
    }

    /// Carries out a curve command (its readiness already checked).
    pub(super) fn run_curve_command(&mut self, command: CommandId) {
        if let Some(kind) = kind_of(command) {
            let edits = self
                .selected_curve_points()
                .into_iter()
                .map(|(id, points)| (id, CurveEdit::SetKind { points, kind }))
                .collect();
            let label = crate::trf!("Handle type: {}", kind.name());
            self.run_curve_edits(edits, &label);
            return;
        }
        match command {
            CommandId::CurveHandles => self.tool_menu.open_group(command),
            CommandId::CurveSplit => self.split_curves(),
            CommandId::CurveRound => {
                let radius = self.attributes.curve.radius;
                let edits = self
                    .selected_curve_points()
                    .into_iter()
                    .map(|(id, points)| (id, CurveEdit::RoundCorners { points, radius }))
                    .collect();
                if self.run_curve_edits(edits, tr("Round corners")) {
                    self.note = Some(crate::trf!(
                        "Corners rounded: radius {} (cut back where it doesn't fit). Change \
                         the radius under Round corners in the Attributes panel. Ctrl + Z \
                         takes it back.",
                        self.attributes.curve.radius_text(self.units())
                    ));
                }
            }
            CommandId::CurveReverse => {
                let edits = self
                    .selected_curves()
                    .into_iter()
                    .map(|id| (id, CurveEdit::Reverse))
                    .collect();
                self.run_curve_edits(edits, tr("Reverse curve"));
            }
            CommandId::CurveOpenClose => {
                let edits = self
                    .selected_curves()
                    .into_iter()
                    .filter_map(|id| {
                        let curve = self.curve_now(id)?;
                        Some(if curve.closed() {
                            (id, CurveEdit::SetClosed(false))
                        } else {
                            (id, CurveEdit::CloseEnds { weld: false })
                        })
                    })
                    .collect();
                self.run_curve_edits(edits, tr("Open or close curve"));
            }
            CommandId::CurveEven => {
                let wanted = self.attributes.curve.count;
                let edits = self
                    .selected_curves()
                    .into_iter()
                    .filter_map(|id| {
                        let count = wanted.unwrap_or(self.curve_now(id)?.points().len());
                        Some((id, CurveEdit::Resample { count }))
                    })
                    .collect();
                self.run_curve_edits(edits, tr("Even spacing"));
            }
            _ => {}
        }
    }

    /// Carries out `edits`, one per curve, as one undo step called
    /// `label`, and says so (or why not). Returns whether it was done.
    fn run_curve_edits(&mut self, edits: Vec<(ObjectId, CurveEdit)>, label: &str) -> bool {
        if edits.is_empty() {
            return false;
        }
        let key = self.new_step_key();
        for (id, edit) in edits {
            if let Err(why) =
                self.document
                    .execute_labelled(Command::EditCurve { id, edit }, label, Some(key))
            {
                self.note = Some(crate::trf!("Couldn't do that: {}.", why));
                return false;
            }
        }
        self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        true
    }

    /// Split curve: each selected curve cut at its selected points.
    fn split_curves(&mut self) {
        let key = self.new_step_key();
        let mut pieces = Vec::new();
        for (id, points) in self.selected_curve_points() {
            let command = Command::SplitCurve { id, points };
            match self
                .document
                .execute_labelled(command, tr("Split curve"), Some(key))
            {
                Ok(()) => pieces.extend(self.document.selection()),
                Err(why) => {
                    self.note = Some(crate::trf!("Couldn't do that: {}.", why));
                    return;
                }
            }
        }
        self.note = Some(crate::trf!(
            "Curve split: {} curves now, each with the same settings. Ctrl + Z takes it back.",
            pieces.len()
        ));
    }

    /// M or A with curve points selected (decision 0139): two end points
    /// of open curves are joined, melted into one at their middle (`weld`)
    /// or by a straight stretch. Returns `false` when no curve points are
    /// selected, so the mesh tools carry on.
    pub(super) fn join_curve_ends(&mut self, weld: bool) -> bool {
        let chosen: Vec<(ObjectId, CurvePointId)> = self
            .selected_curve_points()
            .into_iter()
            .flat_map(|(id, points)| points.into_iter().map(move |p| (id, p)))
            .collect();
        if chosen.is_empty() {
            return false;
        }
        let verb = if weld { tr("Weld") } else { tr("Connect") };
        let ends = chosen
            .iter()
            .all(|(id, p)| self.curve_now(*id).is_some_and(|c| c.end_of(*p).is_some()));
        let [first, second] = chosen.as_slice() else {
            self.note = Some(crate::trf!(
                "{} on curves joins two end points: select just the two ends (of one curve, \
                 or of two curves).",
                verb
            ));
            return true;
        };
        if !ends {
            self.note = Some(crate::trf!(
                "{} on curves joins end points: both points must be ends of open curves.",
                verb
            ));
            return true;
        }
        let command = Command::JoinCurveEnds {
            first: *first,
            second: *second,
            weld,
        };
        match self.document.execute(command) {
            Ok(()) => {
                self.note = Some(
                    match (weld, first.0 == second.0) {
                        (true, true) => tr("Ends welded: the curve is a loop now."),
                        (false, true) => tr("Ends connected: the curve is a loop now."),
                        (true, false) => tr("Curves joined at their ends, welded at the middle."),
                        (false, false) => {
                            tr("Curves joined by a straight stretch between their ends.")
                        }
                    }
                    .to_string()
                        + " "
                        + tr("Ctrl + Z takes it back."),
                );
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't do that: {}.", why)),
        }
        true
    }
}
