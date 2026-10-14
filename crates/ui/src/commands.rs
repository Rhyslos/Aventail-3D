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
    /// Copy the selected objects in place.
    Duplicate,
    /// Make instances of the selected objects in place.
    Instance,
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
                "Transform",
                Icon::AllGizmos,
                plain(Key::T),
                "Move, rotate and scale at once: all three gizmos",
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
                Icon::NewGroup,
                None,
                "Create an empty group, to hold and organise other objects",
                None,
            ),
            Self::ShapeList => (
                "Create a shape…",
                Icon::ShapeList,
                plain(Key::B),
                "Open the shape list: a cube, sphere, cylinder and the rest",
                None,
            ),
            Self::FrameSelected => (
                "Frame selected",
                Icon::FrameSelected,
                plain(Key::Z),
                "Turn and zoom the view so the selection fills it",
                None,
            ),
            Self::Duplicate => (
                "Duplicate",
                Icon::Duplicate,
                None,
                "Copy the selected objects, with their edits, right where they are. Or Ctrl + C, \
                 then Ctrl + V to paste copies.",
                None,
            ),
            Self::Instance => (
                "Instance",
                Icon::Instance,
                None,
                "Make instances of the selected objects right where they are: they show the \
                 original's shape and follow its changes. Or Alt + C, then Alt + V.",
                None,
            ),
            Self::Extrude => (
                "Extrude",
                Icon::Extrude,
                plain(Key::D),
                "Push the selected polygons or edges out, building new sides between \
                 the old place and the new",
                None,
            ),
            Self::Inset => (
                "Inset",
                Icon::Inset,
                plain(Key::I),
                "Shrink the selected polygons inward, leaving a ring of new polygons \
                 around each",
                None,
            ),
            Self::Bevel => (
                "Bevel",
                Icon::Bevel,
                plain(Key::V),
                "Cut the corner off the selected edges or points, with a width, \
                 segments and roundness",
                None,
            ),
            Self::Bridge => (
                "Bridge",
                Icon::Bridge,
                plain(Key::C),
                "Build polygons between two open borders or two polygons; with nothing to \
                 bridge selected, drag from one open edge to another",
                None,
            ),
            Self::FillHole => (
                "Fill hole",
                Icon::FillHole,
                None,
                "Close the open holes touching the selection with a polygon each; with \
                 nothing selected, every hole",
                None,
            ),
            Self::FlipNormals => (
                "Flip normals",
                Icon::FlipNormals,
                None,
                "Turn the selected polygons inside out, for when they face the wrong way",
                None,
            ),
            Self::HardEdges => (
                "Hard edges",
                Icon::HardEdges,
                None,
                "Shade the selected edges (or the selected polygons' edges) as crisp edges: \
                 the light breaks there",
                None,
            ),
            Self::SoftEdges => (
                "Soft edges",
                Icon::SoftEdges,
                None,
                "Shade the selected edges (or the selected polygons' edges) smoothly: the light \
                 flows across, so a curve looks round",
                None,
            ),
            Self::DeleteElements => (
                "Delete",
                Icon::Delete,
                plain(Key::Delete),
                "Delete the selected points, edges or polygons, and the polygons using them, \
                 leaving a hole",
                None,
            ),
            Self::Dissolve => (
                "Dissolve",
                Icon::Dissolve,
                plain(Key::Backspace),
                "Take the selected points, edges or polygons away but keep the surface \
                 closed: the polygons around them merge into one",
                None,
            ),
            Self::Weld => (
                "Weld",
                Icon::Weld,
                plain(Key::M),
                "Melt the selected points into one at their middle; Ctrl + M to drag a point \
                 onto another",
                None,
            ),
            Self::Connect => (
                "Connect",
                Icon::Connect,
                plain(Key::J),
                "Join the selected points or edges with a new edge across their polygon",
                None,
            ),
            Self::Knife => (
                "Slit",
                Icon::Knife,
                plain(Key::X),
                "Cut new edges along a line: click or drag its points over the shape; \
                 Esc or Enter finishes",
                None,
            ),
            Self::LoopCut => (
                "Loop cut",
                Icon::LoopCut,
                plain(Key::F),
                "Cut a new loop of edges all the way around a shape: point at an edge, \
                 then click, or drag to slide it",
                None,
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
            | Self::FrameSelected
            | Self::Duplicate
            | Self::Instance => &[Model, Object],
            Self::Extrude | Self::Bridge => &[Edges, Polygons],
            Self::Inset | Self::FlipNormals => &[Polygons],
            Self::Bevel | Self::Connect => &[Points, Edges],
            Self::FillHole => &[Edges, Polygons],
            Self::HardEdges | Self::SoftEdges => &[Edges, Polygons],
            Self::Weld => &[Points],
            Self::DeleteElements | Self::Dissolve | Self::Knife | Self::LoopCut => {
                &[Points, Edges, Polygons]
            }
        }
    }
}

/// How many places every ring of the tool menu has. Every mode's ring has
/// the same places at the same size, so a button never moves or changes
/// size between modes (the project lead's choice, 2026-10-12).
pub const RING_PLACES: usize = 14;

/// The tool menu's places for whole objects (the Model and Object
/// modes), clockwise from the top; `None` is an empty place. Delete has
/// the same place as in the modelling ring.
pub const OBJECT_RING: [Option<CommandId>; RING_PLACES] = [
    Some(CommandId::Move),
    Some(CommandId::Rotate),
    Some(CommandId::Scale),
    Some(CommandId::AllGizmos),
    Some(CommandId::FrameSelected),
    Some(CommandId::Duplicate),
    Some(CommandId::MakeEditable),
    Some(CommandId::Instance),
    Some(CommandId::Delete),
    Some(CommandId::Ungroup),
    Some(CommandId::Group),
    Some(CommandId::NewGroup),
    Some(CommandId::ShapeList),
    None,
];

/// The tool menu's places for points, edges and polygons, clockwise from
/// the top. Every command keeps its place in every mode, so a mode
/// without it leaves a gap rather than moving the others: building at the
/// top right, shading at the right, removing at the bottom, cutting and
/// joining at the left.
pub const MODELLING_RING: [CommandId; RING_PLACES] = [
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
    let ring: [Option<CommandId>; RING_PLACES] = match mode {
        Mode::Model | Mode::Object => OBJECT_RING,
        Mode::Points | Mode::Edges | Mode::Polygons => MODELLING_RING.map(Some),
        Mode::Uv => return Vec::new(),
    };
    ring.iter()
        .enumerate()
        .filter_map(|(place, command)| Some((place, (*command)?)))
        .filter(|(_, c)| c.modes().contains(&mode))
        .map(|(place, command)| Slot {
            command,
            place,
            places: RING_PLACES,
        })
        .collect()
}

/// Every command, for tests and the menus.
pub const ALL: [CommandId; 27] = [
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
    CommandId::Duplicate,
    CommandId::Instance,
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
            let placed = OBJECT_RING.contains(&Some(command)) || MODELLING_RING.contains(&command);
            assert!(placed, "{command:?} has no place in the tool menu");
            assert!(!command.modes().is_empty(), "{command:?}");
        }
    }

    #[test]
    fn keys_are_unique_and_match_what_the_keys_do() {
        // A key may be shared only by commands of different modes (Delete
        // deletes objects in the Model mode and elements in the others).
        let mut seen = HashSet::new();
        for command in ALL {
            if let Some(shortcut) = command.info().shortcut {
                for mode in command.modes() {
                    assert!(
                        seen.insert((
                            mode.name(),
                            shortcut.modifiers.command,
                            shortcut.modifiers.alt,
                            shortcut.modifiers.shift,
                            shortcut.logical_key
                        )),
                        "{command:?} shares its key in {mode:?}"
                    );
                }
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
        assert_eq!(CommandId::LoopCut.info().shortcut, plain(Key::F));
        assert_eq!(CommandId::Knife.info().shortcut, plain(Key::X));
        assert_eq!(CommandId::Connect.info().shortcut, plain(Key::J));
        assert_eq!(CommandId::Bridge.info().shortcut, plain(Key::C));
        assert_eq!(CommandId::Weld.info().shortcut, plain(Key::M));
        assert_eq!(CommandId::FrameSelected.info().shortcut, plain(Key::Z));
        assert_eq!(
            CommandId::DeleteElements.info().shortcut,
            plain(Key::Delete)
        );
        assert_eq!(CommandId::Dissolve.info().shortcut, plain(Key::Backspace));
    }

    #[test]
    fn the_menu_holds_no_moving_or_scaling_while_editing() {
        for mode in [Mode::Points, Mode::Edges, Mode::Polygons] {
            let commands: Vec<_> = menu_for(mode).iter().map(|s| s.command).collect();
            assert!(!commands.contains(&CommandId::Move), "{mode:?}");
            assert!(commands.contains(&CommandId::LoopCut), "{mode:?}");
        }
        let model: Vec<_> = menu_for(Mode::Model).iter().map(|s| s.command).collect();
        assert_eq!(
            model,
            OBJECT_RING.iter().flatten().copied().collect::<Vec<_>>()
        );
        // Every ring has the same places, and Delete the same one.
        let delete = |slots: Vec<Slot>| {
            slots
                .iter()
                .find(|s| matches!(s.command, CommandId::Delete | CommandId::DeleteElements))
                .map(|s| (s.place, s.places))
        };
        assert_eq!(
            delete(menu_for(Mode::Model)),
            delete(menu_for(Mode::Points))
        );
        assert_eq!(delete(menu_for(Mode::Model)), Some((8, RING_PLACES)));
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
        let objects: Vec<_> = OBJECT_RING.iter().flatten().copied().collect();
        for ring in [&objects[..], &MODELLING_RING[..]] {
            let icons: HashSet<_> = ring.iter().map(|c| c.info().icon).collect();
            assert_eq!(icons.len(), ring.len());
        }
    }
}
