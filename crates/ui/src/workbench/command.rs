//! The workbench's part in the command list: whether each command can be
//! used now, carrying one out (from the tool menu, a key or a menu), and
//! the tool menu's keys and drawing ([decision 0037](../../../../docs/decisions/0037-tool-menu-and-command-list.md)).

use super::Workbench;
use crate::commands::{CommandId, menu_for};
use crate::objects::{self, DocRequest};
use crate::palette::Mode;
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
        let nothing = || Ready::NotNow("Nothing is selected".into());
        match command {
            CommandId::MakeEditable if selection.is_empty() => nothing(),
            CommandId::MakeEditable if !objects::can_make_editable(&self.document, &selection) => {
                Ready::NotNow(
                    "Nothing to make editable: groups and meshes have no settings to bake".into(),
                )
            }
            CommandId::Group
            | CommandId::Delete
            | CommandId::Duplicate
            | CommandId::Instance
            | CommandId::Join
                if selection.is_empty() =>
            {
                nothing()
            }
            CommandId::Ungroup if selection.is_empty() => nothing(),
            CommandId::Ungroup if !objects::has_group(&self.document, &selection) => {
                Ready::NotNow("Nothing to ungroup: only groups can be ungrouped".into())
            }
            CommandId::Extrude if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Extrude works on polygons or edges: press 3 for the Polygons mode, \
                 or 2 for Edges"
                    .into(),
            ),
            CommandId::Inset if !command.modes().contains(&self.mode) => {
                Ready::NotNow("Inset works on polygons: press 3 for the Polygons mode".into())
            }
            CommandId::Bevel if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Bevel works on edges or points: press 2 for the Edges mode, or 1 for Points"
                    .into(),
            ),
            CommandId::LoopCut if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Loop cut works in the Points, Edges or Polygons mode: press 1, 2 or 3".into(),
            ),
            CommandId::Connect if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Connect works on points or edges: press 1 for the Points mode, or 2 for Edges"
                    .into(),
            ),
            CommandId::Bridge if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Bridge works on edges or polygons: press 2 for the Edges mode, or 3 for \
                 Polygons"
                    .into(),
            ),
            CommandId::Weld if !command.modes().contains(&self.mode) => {
                Ready::NotNow("Weld works on points: press 1 for the Points mode".into())
            }
            CommandId::DeleteElements | CommandId::Dissolve
                if !command.modes().contains(&self.mode) =>
            {
                Ready::NotNow(format!(
                    "{} works in the Points, Edges or Polygons mode: press 1, 2 or 3",
                    command.name()
                ))
            }
            CommandId::FillHole if !command.modes().contains(&self.mode) => {
                Ready::NotNow("Fill hole works in the Edges or Polygons mode: press 2 or 3".into())
            }
            CommandId::HardEdges | CommandId::SoftEdges
                if !command.modes().contains(&self.mode) =>
            {
                Ready::NotNow(format!(
                    "{} works on edges or polygons: press 2 for the Edges mode, or 3 for \
                     Polygons",
                    command.name()
                ))
            }
            CommandId::FlipNormals if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Flip normals works on polygons: press 3 for the Polygons mode".into(),
            ),
            CommandId::Slit if !command.modes().contains(&self.mode) => Ready::NotNow(
                "Slit works in the Points, Edges or Polygons mode: press 1, 2 or 3".into(),
            ),
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
                    self.note = Some("Join: select two or more shapes first.".into());
                } else {
                    requests.push(DocRequest::Run(Command::Join(selection)));
                }
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
            CommandId::FrameSelected => self.viewport.frame(crate::Frame::Selected),
        }
    }

    /// The tool menu's entries in the current mode.
    pub(crate) fn tool_menu_entries(&self) -> Vec<Entry> {
        menu_for(self.mode)
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
                self.note = Some(match self.mode {
                    Mode::Uv => "The UV mode has no tools yet: UV tools arrive in Phase 8.".into(),
                    mode => format!("The {} mode has no tools yet.", mode.name()),
                });
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
        if let Some(command) = self.tool_menu.show(ctx, self.mode, entries) {
            self.run_command(command, ctx, requests);
        }
    }
}
