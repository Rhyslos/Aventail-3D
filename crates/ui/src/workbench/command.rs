//! The workbench's part in the command list: whether each command can be
//! used now, carrying one out (from the tool menu, a key or a menu), and
//! the tool menu's keys and drawing ([decision 0037](../../../../docs/decisions/0037-tool-menu-and-command-list.md)).

use super::Workbench;
use crate::commands::{CommandId, group_menu, menu_for};
use crate::objects::{self, DocRequest};
use crate::palette::Mode;
use crate::text::tr;
use crate::tool_menu::{Entry, KeyOutcome, Ready, ToolMenuSettings};
use crate::transforming::Tool;
use egui::Ui;
use scene::{Command, ObjectKind};

impl Workbench {
    /// The workbench with the remembered tool menu settings.
    #[must_use]
    pub fn with_tool_menu(mut self, settings: ToolMenuSettings) -> Self {
        self.tool_menu.settings = settings;
        self
    }

    /// How the tool menu looks and opens, for remembering between
    /// sessions.
    pub fn tool_menu_settings(&self) -> ToolMenuSettings {
        self.tool_menu.settings
    }

    /// Whether the tool menu is showing.
    pub fn is_tool_menu_open(&self) -> bool {
        self.tool_menu.is_open()
    }

    /// Whether `command` can be used now, and if not, why.
    pub(crate) fn readiness(&self, command: CommandId) -> Ready {
        if let Some(when) = command.info().arrives {
            return Ready::Later(when);
        }
        let selection = self.document.selection();
        let nothing = || Ready::NotNow(tr("Nothing is selected").into());
        match command {
            CommandId::MakeEditable if selection.is_empty() => nothing(),
            CommandId::MakeEditable if !objects::can_make_editable(&self.document, &selection) => {
                Ready::NotNow(
                    tr("Nothing to bake into the shape: groups and meshes have no settings to bake")
                        .into(),
                )
            }
            CommandId::Group
            | CommandId::Delete
            | CommandId::Duplicate
            | CommandId::Instance
            | CommandId::Join
            | CommandId::ApplyTransforms
            | CommandId::PivotToCentre
            | CommandId::PivotToOrigin
            | CommandId::PivotToObject
            | CommandId::ObjectToOrigin
                if selection.is_empty() =>
            {
                nothing()
            }
            CommandId::Ungroup if selection.is_empty() => nothing(),
            CommandId::Ungroup if !objects::has_group(&self.document, &selection) => {
                Ready::NotNow(tr("Nothing to ungroup: only groups can be ungrouped").into())
            }
            CommandId::Extrude if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Extrude works on polygons or edges: press 3 for the Polygons mode, \
                 or 2 for Edges")
                    .into(),
            ),
            CommandId::Inset if !command.modes().contains(&self.mode) => {
                Ready::NotNow(tr("Inset works on polygons: press 3 for the Polygons mode").into())
            }
            CommandId::Bevel if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Bevel works on edges or points: press 2 for the Edges mode, or 1 for Points")
                    .into(),
            ),
            CommandId::Groove if !command.modes().contains(&self.mode) => {
                Ready::NotNow(tr("Groove works on edges: press 2 for the Edges mode").into())
            }
            CommandId::LoopCut if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Loop cut works in the Points, Edges or Polygons mode: press 1, 2 or 3").into(),
            ),
            CommandId::Connect if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Connect works on points or edges: press 1 for the Points mode, or 2 for Edges")
                    .into(),
            ),
            CommandId::Bridge if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Bridge works on edges or polygons: press 2 for the Edges mode, or 3 for \
                 Polygons")
                    .into(),
            ),
            CommandId::Weld if !command.modes().contains(&self.mode) => {
                Ready::NotNow(tr("Weld works on points: press 1 for the Points mode").into())
            }
            CommandId::DeleteElements | CommandId::Dissolve
                if !command.modes().contains(&self.mode) =>
            {
                Ready::NotNow(crate::trf!("{} works in the Points, Edges or Polygons mode: press 1, 2 or 3", command.name()))
            }
            CommandId::FillHole if !command.modes().contains(&self.mode) => {
                Ready::NotNow(tr("Fill hole works in the Edges or Polygons mode: press 2 or 3").into())
            }
            CommandId::HardEdges | CommandId::SoftEdges
                if !command.modes().contains(&self.mode) =>
            {
                Ready::NotNow(crate::trf!("{} works on edges or polygons: press 2 for the Edges mode, or 3 for \
                     Polygons", command.name()))
            }
            CommandId::FlipNormals if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Flip normals works on polygons: press 3 for the Polygons mode").into(),
            ),
            CommandId::Slit if !command.modes().contains(&self.mode) => Ready::NotNow(
                tr("Slit works in the Points, Edges or Polygons mode: press 1, 2 or 3").into(),
            ),
            CommandId::MarkSeam | CommandId::ClearSeam
                if crate::uv_editor::element_kind(self.mode).is_none() =>
            {
                Ready::NotNow(crate::trf!("{} works on selected edges or polygons: press 2 for the Edges mode, or 3 \
                     for Polygons", command.name()))
            }
            CommandId::UvTurnLeft
            | CommandId::UvTurnRight
            | CommandId::UvFlipU
            | CommandId::UvFlipV
            | CommandId::UvPin
            | CommandId::UvUnpin
                if crate::uv_editor::element_kind(self.mode).is_none() =>
            {
                Ready::NotNow(crate::trf!("{} works on selected UVs: pick the UV mode, or press 1, 2 or 3 and select \
                     in the UV Editor", command.name()))
            }
            _ => Ready::Yes,
        }
    }

    /// Carries out `command`, or says in the status bar why it can't be.
    pub(crate) fn run_command(
        &mut self,
        command: CommandId,
        ctx: &egui::Context,
        requests: &mut Vec<DocRequest>,
    ) {
        match self.readiness(command) {
            Ready::Yes => {}
            Ready::NotNow(why) => {
                self.note = Some(format!("{why}."));
                return;
            }
            Ready::Later(when) => {
                self.note = Some(format!("{}: {}.", command.name(), when.to_lowercase()));
                return;
            }
        }
        let selection = self.document.selection();
        match command {
            CommandId::Move => self.set_tool(Tool::Move),
            CommandId::Rotate => self.set_tool(Tool::Rotate),
            CommandId::Scale => self.set_tool(Tool::Scale),
            CommandId::AllGizmos => self.set_tool(Tool::All),
            CommandId::Extrude => self.set_tool(Tool::Extrude),
            CommandId::Inset => self.set_tool(Tool::Inset),
            CommandId::Bevel => self.set_tool(Tool::Bevel),
            CommandId::Groove => self.set_tool(Tool::Groove),
            CommandId::LoopCut => self.set_tool(Tool::LoopCut),
            CommandId::Slit => self.set_tool(Tool::Slit),
            CommandId::Connect => self.run_connect(),
            CommandId::Bridge => self.run_bridge(),
            CommandId::Weld => self.run_weld(),
            CommandId::DeleteElements => self.run_delete_elements(),
            CommandId::Dissolve => self.run_dissolve(),
            CommandId::FillHole => self.run_fill_hole(),
            CommandId::FlipNormals => self.run_flip(),
            CommandId::HardEdges => self.run_hardness(true),
            CommandId::SoftEdges => self.run_hardness(false),
            CommandId::MakeEditable => {
                requests.push(DocRequest::Run(Command::MakeEditable(selection)));
            }
            CommandId::Group => requests.push(DocRequest::Run(Command::Group(selection))),
            CommandId::Ungroup => requests.push(DocRequest::Run(Command::Ungroup(selection))),
            CommandId::Delete => requests.push(DocRequest::Run(Command::Delete(selection))),
            CommandId::Join => {
                let shapes = selection
                    .iter()
                    .filter(|id| self.document.scene().evaluated(**id).is_some())
                    .count();
                if shapes < 2 {
                    self.note = Some(tr("Join: select two or more shapes first.").into());
                } else {
                    requests.push(DocRequest::Run(Command::Join(selection)));
                }
            }
            CommandId::ApplyTransforms => {
                requests.push(DocRequest::Run(Command::ApplyTransforms {
                    ids: selection,
                    parts: scene::ApplyParts::ROTATION_AND_SCALE,
                }));
            }
            CommandId::Duplicate | CommandId::Instance => {
                requests.push(DocRequest::Run(Command::Duplicate {
                    ids: selection,
                    instances: command == CommandId::Instance,
                }));
            }
            CommandId::NewGroup => {
                requests.push(DocRequest::Run(Command::Create(ObjectKind::Group)));
            }
            CommandId::ShapeList => {
                let at = ctx
                    .input(|i| i.pointer.latest_pos())
                    .unwrap_or_else(|| ctx.content_rect().center());
                self.shape_picker.open_at(at + egui::vec2(12.0, 12.0));
            }
            CommandId::FrameSelected if self.uv_menu => {
                self.uv_editor.frame(crate::uv_editor::UvFrame::Selected);
            }
            CommandId::FrameSelected => self.viewport.frame(crate::Frame::Selected),
            CommandId::UvTurnLeft => self.change_uvs(UvChange::TurnLeft),
            CommandId::UvTurnRight => self.change_uvs(UvChange::TurnRight),
            CommandId::UvFlipU => self.change_uvs(UvChange::FlipU),
            CommandId::UvFlipV => self.change_uvs(UvChange::FlipV),
            CommandId::UvTurnFlip => self.tool_menu.open_group(command),
            CommandId::UvPin => self.run_pins(true),
            CommandId::UvUnpin => self.run_pins(false),
            CommandId::MarkSeam => self.run_seams(true),
            CommandId::ClearSeam => self.run_seams(false),
            CommandId::Unwrap => self.run_uv_tool(super::unwrapping::UvTool::Unwrap),
            CommandId::Relax => self.run_uv_tool(super::unwrapping::UvTool::Relax),
            CommandId::Straighten => self.run_uv_tool(super::unwrapping::UvTool::Straighten),
            CommandId::Pack => self.run_pack(),
            CommandId::PivotToCentre => self.run_placement(super::pivots::Placement::PivotToCentre),
            CommandId::PivotToOrigin => self.run_placement(super::pivots::Placement::PivotToOrigin),
            CommandId::ObjectToOrigin => {
                self.run_placement(super::pivots::Placement::ObjectToOrigin);
            }
            CommandId::PivotToObject => self.start_pivot_to_object(),
        }
    }

    /// The mode whose ring the tool menu shows: the UV ring over the UV
    /// Editor, otherwise the current mode's.
    pub(crate) fn menu_mode(&self) -> Mode {
        if self.uv_menu { Mode::Uv } else { self.mode }
    }

    /// Turns or flips the selected UVs about their middle, as one undo
    /// step.
    pub(crate) fn change_uvs(&mut self, change: UvChange) {
        use crate::uv_editor::gizmo;
        let (targets, bounds) = gizmo::uv_targets(&self.document, self.mode);
        let Some((min, max)) = bounds else {
            self.note = Some(crate::trf!(
                "{}: select some UVs first.",
                change.command().name()
            ));
            return;
        };
        let transform = gizmo::about((min + max) / 2.0, change.transform());
        let ops = targets
            .iter()
            .map(|t| (t.id, t.operation(transform)))
            .collect();
        let label = change.command().name();
        if self.record_each(label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }

    /// Pins (or unpins) the selected UVs, as one undo step (decision
    /// 0109).
    pub(crate) fn run_pins(&mut self, pin: bool) {
        use crate::uv_editor::gizmo;
        let command = if pin {
            CommandId::UvPin
        } else {
            CommandId::UvUnpin
        };
        let (targets, _) = gizmo::uv_targets(&self.document, self.mode);
        if targets.is_empty() {
            self.note = Some(crate::trf!("{}: select some UVs first.", command.name()));
            return;
        }
        let count: usize = targets
            .iter()
            .map(|t| t.faces.len().max(t.points.len()))
            .sum();
        let noun = if targets.iter().all(|t| t.faces.is_empty()) {
            tr("point")
        } else {
            tr("polygon")
        };
        let ops = targets
            .into_iter()
            .map(|t| {
                (
                    t.id,
                    scene::Operation::SetUvPins {
                        faces: t.faces,
                        points: t.points,
                        pin,
                        anchors: t.anchors,
                    },
                )
            })
            .collect();
        let verb = if pin {
            tr("Pin the UVs of")
        } else {
            tr("Unpin the UVs of")
        };
        let label = super::joining::counted(verb, count, noun);
        if self.record_each(&label, ops) {
            let why = if pin {
                tr(" Unwrapping lays the rest out around them; relaxing leaves them alone.")
            } else {
                ""
            };
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.{}", label, why));
        }
    }

    /// The tool menu's entries in the current mode.
    pub(crate) fn tool_menu_entries(&self) -> Vec<Entry> {
        let ring = menu_for(self.menu_mode());
        let slots = match self.tool_menu.group() {
            Some(group) => group_menu(group, &ring),
            None => ring,
        };
        slots
            .into_iter()
            .map(|slot| Entry {
                slot,
                ready: self.readiness(slot.command),
            })
            .collect()
    }

    /// The tool menu's keys (not while typing). The shape list and the
    /// tool menu never show at once.
    pub(super) fn tool_menu_keys(
        &mut self,
        ui: &Ui,
        typing: bool,
        entries: &[Entry],
        requests: &mut Vec<DocRequest>,
    ) {
        let was_open = self.tool_menu.is_open();
        match self.tool_menu.keys(ui, typing, entries) {
            Some(KeyOutcome::Chose(command)) => self.run_command(command, ui.ctx(), requests),
            Some(KeyOutcome::NoTools) => {
                self.note = Some(crate::trf!(
                    "The {} mode has no tools yet.",
                    self.mode.name()
                ));
            }
            None => {}
        }
        if self.tool_menu.is_open() && !was_open {
            self.shape_picker.close();
        }
        if self.shape_picker.is_open() {
            self.tool_menu.close();
        }
    }

    /// Draws the tool menu, if open, and carries out the tool clicked.
    pub(super) fn tool_menu_show(
        &mut self,
        ctx: &egui::Context,
        entries: &[Entry],
        requests: &mut Vec<DocRequest>,
    ) {
        if let Some(command) = self.tool_menu.show(ctx, self.menu_mode(), entries) {
            self.run_command(command, ctx, requests);
        }
    }
}

/// A quarter turn or a flip of the selected UVs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvChange {
    TurnLeft,
    TurnRight,
    FlipU,
    FlipV,
}

impl UvChange {
    /// The command that makes it.
    pub(crate) fn command(self) -> CommandId {
        match self {
            Self::TurnLeft => CommandId::UvTurnLeft,
            Self::TurnRight => CommandId::UvTurnRight,
            Self::FlipU => CommandId::UvFlipU,
            Self::FlipV => CommandId::UvFlipV,
        }
    }

    /// The change about the origin (the texture's V points up, so left is
    /// anticlockwise).
    pub(crate) fn transform(self) -> glam::Affine2 {
        use std::f32::consts::FRAC_PI_2;
        match self {
            Self::TurnLeft => glam::Affine2::from_angle(FRAC_PI_2),
            Self::TurnRight => glam::Affine2::from_angle(-FRAC_PI_2),
            Self::FlipU => glam::Affine2::from_scale(glam::Vec2::new(-1.0, 1.0)),
            Self::FlipV => glam::Affine2::from_scale(glam::Vec2::new(1.0, -1.0)),
        }
    }
}
