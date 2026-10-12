//! The command list: every command the tool menu offers, defined once,
//! with its name, icon, key, explanation and (for one not built yet) the
//! phase that brings it ([decision 0037](../../../docs/decisions/0037-tool-menu-and-command-list.md)).
//!
//! The tool menu (`E`), the menus, the palette's tooltips and the keys
//! all read from here, so a command looks, is named and is explained the
//! same everywhere. To change a command's name, icon or key, change it
//! here.

use crate::icons::Icon;
use crate::objects::{DELETE_SHORTCUT, GROUP_SHORTCUT, MAKE_EDITABLE_SHORTCUT, UNGROUP_SHORTCUT};
use crate::palette::Mode;
use egui::{Key, KeyboardShortcut, Modifiers};

/// A command the app offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    // ----------------------------------------------------- whole objects
    /// The move gizmo.
    Move,
    /// The rotate gizmo.
    Rotate,
    /// The scale gizmo.
    Scale,
    /// All three gizmos at once.
    AllGizmos,
    /// Turn shapes into plain meshes.
    MakeEditable,
    /// Put the selected objects into a new group.
    Group,
    /// Take the selected groups' contents out.
    Ungroup,
    /// Delete the selected objects.
    Delete,
    /// Create an empty group.
    NewGroup,
    /// Open the shape list.
    ShapeList,
    /// Frame the selection in the view.
    FrameSelected,
    // --------------------------------------------- points, edges, polygons
    /// Push polygons or edges out, building new sides.
    Extrude,
    /// Shrink polygons inward, leaving a ring of new polygons.
    Inset,
    /// Cut corners off edges or points.
    Bevel,
    /// Build polygons between two open edges or holes.
    Bridge,
    /// Close a hole with a polygon.
    FillHole,
    /// Turn polygons inside out.
    FlipNormals,
    /// Mark edges as sharp creases.
    HardEdges,
    /// Mark edges as smooth.
    SoftEdges,
    /// Delete the selected points, edges or polygons, leaving a hole.
    DeleteElements,
    /// Remove edges or points, keeping the surface closed.
    Dissolve,
    /// Melt points into one.
    Weld,
    /// Join points or edges with a new edge across their polygon.
    Connect,
    /// Cut new edges along a line.
    Knife,
    /// Cut a new loop of edges around a shape.
    LoopCut,
}

/// Everything shown about a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandInfo {
    /// Its name, as shown in menus.
    pub name: &'static str,
    /// Its icon.
    pub icon: Icon,
    /// Its key, if it has one.
    pub shortcut: Option<KeyboardShortcut>,
    /// What it does, in plain words.
    pub help: &'static str,
    /// When it arrives, for a command not built yet (`None` once built).
    pub arrives: Option<&'static str>,
}

/// A key on its own.
const fn plain(key: Key) -> Option<KeyboardShortcut> {
    Some(KeyboardShortcut::new(Modifiers::NONE, key))
}

/// The phase the modelling tools arrive in.
const PHASE_4: Option<&str> = Some("Arrives in Phase 4");

impl CommandId {
    /// Everything shown about the command.
    pub const fn info(self) -> CommandInfo {
        let (name, icon, shortcut, help, arrives) = match self {
            Self::Move => (
                "Move",
                Icon::Move,
                plain(Key::W),
                "Move the selection: drag the gizmo's arrows, squares or centre",
                None,
            ),
            Self::Rotate => (
                "Rotate",
                Icon::Rotate,
                plain(Key::R),
                "Rotate the selection: drag the gizmo's rings",
                None,
            ),
            Self::Scale => (
                "Scale",
                Icon::Scale,
                plain(Key::S),
                "Scale the selection: drag the gizmo's boxes, or its centre for all axes",
                None,
            ),
            Self::AllGizmos => (
                "Move, rotate and scale",
                Icon::AllGizmos,
                plain(Key::T),
                "All three gizmos at once",
                None,
            ),
            Self::MakeEditable => (
                "Make editable",
                Icon::MakeEditable,
                Some(MAKE_EDITABLE_SHORTCUT),
                "Turn the selected shapes into plain meshes, ready for editing points, \
                 edges and polygons. Their settings and recorded edits are baked in. \
                 Undo brings the settings back.",
                None,
            ),
            Self::Group => (
                "Group objects",
                Icon::Group,
                Some(GROUP_SHORTCUT),
                "Put the selected objects into a new group",
                None,
            ),
            Self::Ungroup => (
                "Ungroup",
                Icon::Ungroup,
                Some(UNGROUP_SHORTCUT),
                "Take the selected groups' contents out, and remove the groups",
                None,
            ),
            Self::Delete => (
                "Delete",
                Icon::Delete,
                Some(DELETE_SHORTCUT),
                "Delete the selected objects (Backspace does it too)",
                None,
            ),
            Self::NewGroup => (
                "New group",
                Icon::Null,
                None,
                "Create an empty group, to hold and organise other objects",
                None,
            ),
            Self::ShapeList => (
                "Create a shape…",
                Icon::Cube,
                plain(Key::B),
                "Open the shape list: a cube, sphere, cylinder and the rest",
                None,
            ),
            Self::FrameSelected => (
                "Frame selected",
                Icon::FrameSelected,
                plain(Key::F),
                "Turn and zoom the view so the selection fills it",
                None,
            ),
            Self::Extrude => (
                "Extrude",
                Icon::Extrude,
                plain(Key::D),
                "Push the selected polygons or edges out, building new sides between \
                 the old place and the new",
                PHASE_4,
            ),
            Self::Inset => (
                "Inset",
                Icon::Inset,
                plain(Key::I),
                "Shrink the selected polygons inward, leaving a ring of new polygons \
                 around each",
                PHASE_4,
            ),
            Self::Bevel => (
                "Bevel",
                Icon::Bevel,
                plain(Key::V),
                "Cut the corner off the selected edges or points, with a width, \
                 segments and roundness",
                PHASE_4,
            ),
            Self::Bridge => (
                "Bridge",
                Icon::Bridge,
                None,
                "Build polygons between two open edges or two holes",
                PHASE_4,
            ),
            Self::FillHole => (
                "Fill hole",
                Icon::FillHole,
                None,
                "Close an open hole with a polygon",
                PHASE_4,
            ),
            Self::FlipNormals => (
                "Flip normals",
                Icon::FlipNormals,
                None,
                "Turn the selected polygons inside out, for when they face the wrong way",
                PHASE_4,
            ),
            Self::HardEdges => (
                "Hard edges",
                Icon::HardEdges,
                None,
                "Shade the selected edges as sharp creases",
                PHASE_4,
            ),
            Self::SoftEdges => (
                "Soft edges",
                Icon::SoftEdges,
                None,
                "Shade the selected edges smoothly",
                PHASE_4,
            ),
            Self::DeleteElements => (
                "Delete",
                Icon::Delete,
                None,
                "Delete the selected points, edges or polygons, leaving a hole",
                PHASE_4,
            ),
            Self::Dissolve => (
                "Dissolve",
                Icon::Dissolve,
                None,
                "Remove the selected edges or points but keep the surface closed",
                PHASE_4,
            ),
            Self::Weld => (
                "Weld",
                Icon::Weld,
                None,
                "Melt the selected points into one, at their middle or onto a target point",
                PHASE_4,
            ),
            Self::Connect => (
                "Connect",
                Icon::Connect,
                None,
                "Join the selected points or edges with a new edge across their polygon",
                PHASE_4,
            ),
            Self::Knife => (
                "Knife",
                Icon::Knife,
                None,
                "Cut new edges wherever you draw a line",
                PHASE_4,
            ),
            Self::LoopCut => (
                "Loop cut",
                Icon::LoopCut,
                None,
                "Cut a new loop of edges all the way around a shape",
                PHASE_4,
            ),
        };
        CommandInfo {
            name,
            icon,
            shortcut,
            help,
            arrives,
        }
    }

    /// Its name.
    pub const fn name(self) -> &'static str {
        self.info().name
    }

    /// Whether it is built.
    pub const fn is_built(self) -> bool {
        self.info().arrives.is_none()
    }

    /// The modes whose tool menu holds it.
    pub const fn modes(self) -> &'static [Mode] {
        use Mode::{Edges, Model, Object, Points, Polygons};
        match self {
            Self::Move
            | Self::Rotate
            | Self::Scale
            | Self::AllGizmos
            | Self::MakeEditable
            | Self::Group
            | Self::Ungroup
            | Self::Delete
            | Self::NewGroup
            | Self::ShapeList
            | Self::FrameSelected => &[Model, Object],
            Self::Extrude | Self::Bridge => &[Edges, Polygons],
            Self::Inset | Self::FlipNormals => &[Polygons],
            Self::Bevel | Self::Connect | Self::Dissolve => &[Points, Edges],
            Self::FillHole => &[Edges, Polygons],
            Self::HardEdges | Self::SoftEdges => &[Edges],
            Self::Weld => &[Points],
            Self::DeleteElements | Self::Knife | Self::LoopCut => &[Points, Edges, Polygons],
        }
    }
}

/// The tool menu's places for whole objects (the Model and Object
/// modes), clockwise from the top. Every command keeps its place.
pub const OBJECT_RING: [CommandId; 11] = [
    CommandId::Move,
    CommandId::Rotate,
    CommandId::Scale,
    CommandId::AllGizmos,
    CommandId::FrameSelected,
    CommandId::MakeEditable,
    CommandId::Delete,
    CommandId::Ungroup,
    CommandId::Group,
    CommandId::NewGroup,
    CommandId::ShapeList,
];

/// The tool menu's places for points, edges and polygons, clockwise from
/// the top. Every command keeps its place in every mode, so a mode
/// without it leaves a gap rather than moving the others: building at the
/// top right, shading at the right, removing at the bottom, cutting and
/// joining at the left.
pub const MODELLING_RING: [CommandId; 14] = [
    CommandId::Extrude,
    CommandId::Inset,
    CommandId::Bevel,
    CommandId::Bridge,
    CommandId::FillHole,
    CommandId::FlipNormals,
    CommandId::HardEdges,
    CommandId::SoftEdges,
    CommandId::DeleteElements,
    CommandId::Dissolve,
    CommandId::Weld,
    CommandId::Connect,
    CommandId::Knife,
    CommandId::LoopCut,
];

/// One entry of the tool menu: a command and its place on the ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The command.
    pub command: CommandId,
    /// Its place, clockwise from the top.
    pub place: usize,
    /// How many places the ring has.
    pub places: usize,
}

/// The tool menu's entries in `mode`, in order (empty when the mode has
/// no tools yet, like UV).
pub fn menu_for(mode: Mode) -> Vec<Slot> {
    let ring: &[CommandId] = match mode {
        Mode::Model | Mode::Object => &OBJECT_RING,
        Mode::Points | Mode::Edges | Mode::Polygons => &MODELLING_RING,
        Mode::Uv => &[],
    };
    ring.iter()
        .enumerate()
        .filter(|(_, c)| c.modes().contains(&mode))
        .map(|(place, &command)| Slot {
            command,
            place,
            places: ring.len(),
        })
        .collect()
}

/// Every command, for tests and the menus.
pub const ALL: [CommandId; 25] = [
    CommandId::Move,
    CommandId::Rotate,
    CommandId::Scale,
    CommandId::AllGizmos,
    CommandId::MakeEditable,
    CommandId::Group,
    CommandId::Ungroup,
    CommandId::Delete,
    CommandId::NewGroup,
    CommandId::ShapeList,
    CommandId::FrameSelected,
    CommandId::Extrude,
    CommandId::Inset,
    CommandId::Bevel,
    CommandId::Bridge,
    CommandId::FillHole,
    CommandId::FlipNormals,
    CommandId::HardEdges,
    CommandId::SoftEdges,
    CommandId::DeleteElements,
    CommandId::Dissolve,
    CommandId::Weld,
    CommandId::Connect,
    CommandId::Knife,
    CommandId::LoopCut,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_command_is_listed_once_and_has_a_place() {
        let all: HashSet<_> = ALL.into_iter().collect();
        assert_eq!(all.len(), ALL.len());
        for command in ALL {
            let placed = OBJECT_RING.contains(&command) || MODELLING_RING.contains(&command);
            assert!(placed, "{command:?} has no place in the tool menu");
            assert!(!command.modes().is_empty(), "{command:?}");
        }
    }

    #[test]
    fn keys_are_unique_and_match_what_the_keys_do() {
        let mut seen = HashSet::new();
        for command in ALL {
            if let Some(shortcut) = command.info().shortcut {
                assert!(
                    seen.insert((
                        shortcut.modifiers.command,
                        shortcut.modifiers.alt,
                        shortcut.modifiers.shift,
                        shortcut.logical_key
                    )),
                    "{command:?} shares its key"
                );
            }
        }
        // The keys the app already uses are named the same here.
        for (key, tool) in crate::transforming::TOOL_KEYS {
            let command = match tool {
                crate::transforming::Tool::Move => CommandId::Move,
                crate::transforming::Tool::Rotate => CommandId::Rotate,
                crate::transforming::Tool::Scale => CommandId::Scale,
                _ => CommandId::AllGizmos,
            };
            assert_eq!(command.info().shortcut, plain(key), "{command:?}");
        }
        assert_eq!(
            CommandId::ShapeList.info().shortcut,
            plain(crate::shape_picker::SHAPE_LIST_KEY)
        );
        assert_eq!(
            CommandId::FrameSelected.info().shortcut,
            plain(crate::Frame::Selected.key())
        );
        // The keys the project lead chose for tools to come.
        assert_eq!(CommandId::Extrude.info().shortcut, plain(Key::D));
        assert_eq!(CommandId::Inset.info().shortcut, plain(Key::I));
        assert_eq!(CommandId::Bevel.info().shortcut, plain(Key::V));
    }

    #[test]
    fn the_menu_holds_no_moving_or_scaling_while_editing() {
        for mode in [Mode::Points, Mode::Edges, Mode::Polygons] {
            let commands: Vec<_> = menu_for(mode).iter().map(|s| s.command).collect();
            assert!(!commands.contains(&CommandId::Move), "{mode:?}");
            assert!(commands.contains(&CommandId::LoopCut), "{mode:?}");
        }
        let model: Vec<_> = menu_for(Mode::Model).iter().map(|s| s.command).collect();
        assert_eq!(model, OBJECT_RING);
        assert!(menu_for(Mode::Uv).is_empty());
    }

    #[test]
    fn a_tool_keeps_its_place_in_every_mode() {
        for mode in [Mode::Points, Mode::Edges, Mode::Polygons] {
            for slot in menu_for(mode) {
                assert_eq!(MODELLING_RING[slot.place], slot.command);
                assert_eq!(slot.places, MODELLING_RING.len());
            }
        }
        let edges = menu_for(Mode::Edges);
        let polygons = menu_for(Mode::Polygons);
        let place = |slots: &[Slot], c| slots.iter().find(|s| s.command == c).map(|s| s.place);
        assert_eq!(
            place(&edges, CommandId::Extrude),
            place(&polygons, CommandId::Extrude)
        );
    }

    #[test]
    fn names_and_icons_read_well() {
        for command in ALL {
            let info = command.info();
            assert!(!info.name.is_empty() && !info.help.is_empty());
            if let Some(when) = info.arrives {
                assert!(when.starts_with("Arrives"), "{command:?}");
            }
        }
        // Within one ring, every icon is different.
        for ring in [&OBJECT_RING[..], &MODELLING_RING[..]] {
            let icons: HashSet<_> = ring.iter().map(|c| c.info().icon).collect();
            assert_eq!(icons.len(), ring.len());
        }
    }
}
