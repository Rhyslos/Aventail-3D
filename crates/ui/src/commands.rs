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
    /// Join the selected objects into one.
    Join,
    /// Apply the selected objects' rotation and scale (decision 0089).
    ApplyTransforms,
    // ------------------------------------------- pivots (the Object mode)
    /// Move the selected objects' pivots to the middle of their shapes.
    PivotToCentre,
    /// Move the selected objects' pivots to the world origin.
    PivotToOrigin,
    /// Move the selected objects' pivots to another object's pivot,
    /// picked with a click.
    PivotToObject,
    /// Move the selected objects so their pivots sit at the world origin.
    ObjectToOrigin,
    // --------------------------------------------- points, edges, polygons
    /// Push polygons or edges out, building new sides.
    Extrude,
    /// Shrink polygons inward, leaving a ring of new polygons.
    Inset,
    /// Cut corners off edges or points.
    Bevel,
    /// Cut a thin groove along edges, or raise a ridge (decision 0086).
    Groove,
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
    Slit,
    /// Cut a new loop of edges around a shape.
    LoopCut,
    // ------------------------------------------- UVs (the UV Editor's ring)
    /// Turn the selected UVs a quarter turn anticlockwise.
    UvTurnLeft,
    /// Turn the selected UVs a quarter turn clockwise.
    UvTurnRight,
    /// Mirror the selected UVs left to right.
    UvFlipU,
    /// Mirror the selected UVs upside down.
    UvFlipV,
    /// Mark the selected edges as seams.
    MarkSeam,
    /// Clear the seams from the selected edges.
    ClearSeam,
    /// Lay the selected parts out flat again along their seams.
    Unwrap,
    /// Even out the selected UVs.
    Relax,
    /// Straighten the selected UVs into a grid.
    Straighten,
    /// Fit every island into the texture square.
    Pack,
    /// Pin the selected UVs, so unwrapping and relaxing leave them be
    /// (decision 0109).
    UvPin,
    /// Unpin the selected UVs.
    UvUnpin,
    /// The group holding the turns and flips: choosing it shows them.
    UvTurnFlip,
    /// The group holding stitching and cutting: choosing it shows them.
    UvStitchCut,
    /// Join the selected UV edges to their partners.
    UvStitch,
    /// Split the selected UV edges apart.
    UvCut,
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
                "Duplicate as instance",
                Icon::Instance,
                None,
                "Make instances of the selected objects right where they are: they show the \
                 original's shape and follow its changes. Or Alt + C, then Alt + V.",
                None,
            ),
            Self::Join => (
                "Join objects",
                Icon::Join,
                None,
                "Join the selected shapes into the first of them, as one plain mesh, each part \
                 staying where it was",
                None,
            ),
            Self::ApplyTransforms => (
                "Apply rotation and scale",
                Icon::ApplyTransforms,
                None,
                "Turn the selected objects back to 0° and 100% while they look exactly the \
                 same: their shapes take the rotation and scale, so edits measure as they look. \
                 Mesh → Apply transforms has position too.",
                None,
            ),
            Self::PivotToCentre => (
                "Pivot to centre",
                Icon::PivotToCentre,
                None,
                "Move the selected objects' pivots to the middle of their shapes; the shapes \
                 stay where they are",
                None,
            ),
            Self::PivotToOrigin => (
                "Pivot to world origin",
                Icon::PivotToOrigin,
                None,
                "Move the selected objects' pivots to the world origin, where the red and \
                 blue lines cross; the shapes stay where they are",
                None,
            ),
            Self::PivotToObject => (
                "Pivot to object…",
                Icon::PivotToObject,
                None,
                "Then click another object: the selected objects' pivots move to its pivot, \
                 and the shapes stay where they are. Esc cancels.",
                None,
            ),
            Self::ObjectToOrigin => (
                "Object to world origin",
                Icon::ObjectToOrigin,
                None,
                "Move the selected objects, shape and pivot together, so their pivots sit at \
                 the world origin",
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
            Self::Groove => (
                "Groove",
                Icon::Groove,
                plain(Key::K),
                "Cut a thin groove into the shape along the selected edges, like a panel \
                 line, or raise a ridge out of it: drag in or out",
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
                plain(Key::J),
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
                plain(Key::A),
                "Join the selected points or edges with a new edge across their polygon",
                None,
            ),
            Self::Slit => (
                "Slit",
                Icon::Slit,
                plain(Key::X),
                "Cut new edges along a line: click or drag its points over the shape; \
                 Esc or Enter finishes, or click the first point to close a loop",
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
            Self::UvTurnLeft => (
                "Turn left 90°",
                Icon::UvTurnLeft,
                None,
                "Turn the selected UVs a quarter turn to the left (anticlockwise), about \
                 their middle",
                None,
            ),
            Self::UvTurnRight => (
                "Turn right 90°",
                Icon::UvTurnRight,
                None,
                "Turn the selected UVs a quarter turn to the right (clockwise), about their \
                 middle",
                None,
            ),
            Self::UvFlipU => (
                "Flip left to right",
                Icon::UvFlipU,
                None,
                "Mirror the selected UVs left to right, about their middle",
                None,
            ),
            Self::UvFlipV => (
                "Flip upside down",
                Icon::UvFlipV,
                None,
                "Mirror the selected UVs top to bottom, about their middle",
                None,
            ),
            Self::MarkSeam => (
                "Mark seams",
                Icon::MarkSeam,
                None,
                "Mark the selected edges as seams: where the surface is cut open to lie \
                 flat. With polygons selected, the edges around them.",
                None,
            ),
            Self::ClearSeam => (
                "Clear seams",
                Icon::ClearSeam,
                None,
                "Take the seams off the selected edges (or the selected polygons' edges), so the \
                 surface stays joined there when unwrapped",
                None,
            ),
            Self::Unwrap => (
                "Unwrap",
                Icon::Unwrap,
                None,
                "Lay the pieces holding the selection (or whole objects) flat along their \
                 seams, with as little stretching as can be, at the object's texel density",
                None,
            ),
            Self::Relax => (
                "Relax",
                Icon::Relax,
                None,
                "Even out the selected polygons' UVs, so a texture stretches less over them",
                None,
            ),
            Self::Straighten => (
                "Straighten",
                Icon::Straighten,
                None,
                "Line the pieces made of quads up into a straight grid, for strips and panels",
                None,
            ),
            Self::Pack => (
                "Pack",
                Icon::Pack,
                None,
                "Fit the islands holding the selection (or whole objects) into the texture \
                 square as large as they go, with padding between them (UV → Packing)",
                None,
            ),
            Self::UvPin => (
                "Pin",
                Icon::UvPin,
                None,
                "Pin the selected UVs where they are: unwrapping lays the rest out around them, \
                 and relaxing leaves them alone",
                None,
            ),
            Self::UvUnpin => (
                "Unpin",
                Icon::UvUnpin,
                None,
                "Unpin the selected UVs, so unwrapping and relaxing may move them again",
                None,
            ),
            Self::UvTurnFlip => (
                "Turn & flip",
                Icon::UvTurnFlip,
                None,
                "Turn the selected UVs a quarter turn, or mirror them: choose this to see the \
                 four",
                None,
            ),
            Self::UvStitchCut => (
                "Stitch & cut",
                Icon::UvStitchCut,
                None,
                "Join UV edges to their partners, or split them apart: choose this to see both",
                None,
            ),
            Self::UvStitch => (
                "Stitch",
                Icon::UvStitch,
                plain(Key::C),
                "Join the selected UV edges to their partners on the model, bringing the \
                 smaller piece over to the bigger",
                Some("Arrives in Phase 8, milestone 8"),
            ),
            Self::UvCut => (
                "Cut",
                Icon::UvCut,
                plain(Key::X),
                "Split the selected UV edges, so the pieces either side can move apart",
                Some("Arrives in Phase 8, milestone 8"),
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

    /// The commands a group holds (empty for a command that isn't a
    /// group). Choosing a group in the tool menu shows its commands in
    /// their place (decision 0109).
    pub const fn members(self) -> &'static [CommandId] {
        match self {
            Self::UvTurnFlip => &[
                Self::UvTurnLeft,
                Self::UvTurnRight,
                Self::UvFlipU,
                Self::UvFlipV,
            ],
            Self::UvStitchCut => &[Self::UvStitch, Self::UvCut],
            _ => &[],
        }
    }

    /// Whether it is a group of commands.
    pub const fn is_group(self) -> bool {
        !self.members().is_empty()
    }

    /// Whether it is built.
    pub const fn is_built(self) -> bool {
        self.info().arrives.is_none()
    }

    /// The modes whose tool menu holds it.
    pub const fn modes(self) -> &'static [Mode] {
        use Mode::{Edges, Model, Object, Points, Polygons, Uv};
        match self {
            // The gizmos and framing work on UVs too, in the UV Editor.
            Self::Move | Self::Rotate | Self::AllGizmos | Self::FrameSelected => {
                &[Model, Object, Uv]
            }
            Self::Group | Self::Ungroup | Self::Delete | Self::NewGroup | Self::ShapeList => {
                &[Model, Object]
            }
            // A pivot has no size.
            Self::Scale => &[Model, Uv],
            // The Object mode's ring makes room for the pivot commands.
            Self::MakeEditable
            | Self::Duplicate
            | Self::Instance
            | Self::Join
            | Self::ApplyTransforms => &[Model],
            Self::PivotToCentre
            | Self::PivotToOrigin
            | Self::PivotToObject
            | Self::ObjectToOrigin => &[Object],
            Self::Extrude | Self::Bridge => &[Edges, Polygons],
            Self::Inset | Self::FlipNormals => &[Polygons],
            Self::Bevel | Self::Connect => &[Points, Edges],
            Self::Groove => &[Edges],
            Self::FillHole => &[Edges, Polygons],
            Self::HardEdges | Self::SoftEdges => &[Edges, Polygons],
            Self::Weld => &[Points],
            Self::DeleteElements | Self::Dissolve | Self::Slit | Self::LoopCut => {
                &[Points, Edges, Polygons]
            }
            Self::UvTurnLeft
            | Self::UvTurnRight
            | Self::UvFlipU
            | Self::UvFlipV
            | Self::MarkSeam
            | Self::ClearSeam
            | Self::Unwrap
            | Self::Relax
            | Self::Straighten
            | Self::Pack
            | Self::UvPin
            | Self::UvUnpin
            | Self::UvTurnFlip
            | Self::UvStitchCut
            | Self::UvStitch
            | Self::UvCut => &[Uv],
        }
    }
}

/// How many places every ring of the tool menu has. Every mode's ring has
/// the same places at the same size, so a button never moves or changes
/// size between modes (the project lead's choice, 2026-10-12).
pub const RING_PLACES: usize = 15;

/// The tool menu's places for whole objects (the Model mode), clockwise
/// from the top; `None` is an empty place. Delete has the same place as
/// in the modelling ring.
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
    Some(CommandId::Join),
    Some(CommandId::ApplyTransforms),
];

/// The tool menu's places in the Object mode, where the gizmos move pivots
/// (decision 0068): the commands they share with the Model mode keep
/// their places, and the pivot commands take the places of those that
/// work on shapes. Scale's place is empty: a pivot has no size.
pub const PIVOT_RING: [Option<CommandId>; RING_PLACES] = [
    Some(CommandId::Move),
    Some(CommandId::Rotate),
    None,
    Some(CommandId::AllGizmos),
    Some(CommandId::FrameSelected),
    Some(CommandId::PivotToCentre),
    Some(CommandId::PivotToOrigin),
    Some(CommandId::PivotToObject),
    Some(CommandId::Delete),
    Some(CommandId::Ungroup),
    Some(CommandId::Group),
    Some(CommandId::NewGroup),
    Some(CommandId::ShapeList),
    Some(CommandId::ObjectToOrigin),
    None,
];

/// The tool menu's places for points, edges and polygons, clockwise from
/// the top. Every command keeps its place in every mode, so a mode
/// without it leaves a gap rather than moving the others: building at the
/// top right, shading at the right, removing at the bottom, cutting and
/// joining at the left, and Groove (a cut along edges) last, at the top
/// left, so Delete keeps its place in every ring.
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
    CommandId::Slit,
    CommandId::LoopCut,
    CommandId::Groove,
];

/// The tool menu's places for UVs: in the UV mode, and with the mouse
/// over the UV Editor in the Points, Edges and Polygons modes (the menu
/// follows where you work, decision 0097). The gizmos and framing keep
/// their places from the object ring; seams at the right, unwrapping at
/// the bottom, packing, turning and pinning at the left. Turning and
/// flipping share one place, as do stitching and cutting: choosing one
/// shows what it holds (decision 0109).
pub const UV_RING: [CommandId; RING_PLACES] = [
    CommandId::Move,
    CommandId::Rotate,
    CommandId::Scale,
    CommandId::AllGizmos,
    CommandId::FrameSelected,
    CommandId::MarkSeam,
    CommandId::ClearSeam,
    CommandId::Unwrap,
    CommandId::Relax,
    CommandId::Straighten,
    CommandId::Pack,
    CommandId::UvTurnFlip,
    CommandId::UvPin,
    CommandId::UvUnpin,
    CommandId::UvStitchCut,
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

/// The tool menu's entries in `mode`, in order. `Mode::Uv` gives the UV
/// ring, also used over the UV Editor in the other editing modes.
pub fn menu_for(mode: Mode) -> Vec<Slot> {
    let ring: [Option<CommandId>; RING_PLACES] = match mode {
        Mode::Model => OBJECT_RING,
        Mode::Object => PIVOT_RING,
        Mode::Points | Mode::Edges | Mode::Polygons => MODELLING_RING.map(Some),
        Mode::Uv => UV_RING.map(Some),
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

/// The tool menu's entries for `group`, opened in place of the ring that
/// held it: its commands on the same ring, side by side around the
/// group's own place, so the mouse is already near them.
pub fn group_menu(group: CommandId, ring: &[Slot]) -> Vec<Slot> {
    let members = group.members();
    let centre = ring
        .iter()
        .find(|s| s.command == group)
        .map_or(0, |s| s.place);
    let n = members.len();
    members
        .iter()
        .enumerate()
        .map(|(k, command)| Slot {
            command: *command,
            place: (centre + RING_PLACES + k - (n - 1) / 2) % RING_PLACES,
            places: RING_PLACES,
        })
        .collect()
}

/// Every command, for tests and the menus.
pub const ALL: [CommandId; 50] = [
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
    CommandId::Join,
    CommandId::ApplyTransforms,
    CommandId::PivotToCentre,
    CommandId::PivotToOrigin,
    CommandId::PivotToObject,
    CommandId::ObjectToOrigin,
    CommandId::Extrude,
    CommandId::Inset,
    CommandId::Bevel,
    CommandId::Groove,
    CommandId::Bridge,
    CommandId::FillHole,
    CommandId::FlipNormals,
    CommandId::HardEdges,
    CommandId::SoftEdges,
    CommandId::DeleteElements,
    CommandId::Dissolve,
    CommandId::Weld,
    CommandId::Connect,
    CommandId::Slit,
    CommandId::LoopCut,
    CommandId::UvTurnLeft,
    CommandId::UvTurnRight,
    CommandId::UvFlipU,
    CommandId::UvFlipV,
    CommandId::MarkSeam,
    CommandId::ClearSeam,
    CommandId::Unwrap,
    CommandId::Relax,
    CommandId::Straighten,
    CommandId::Pack,
    CommandId::UvPin,
    CommandId::UvUnpin,
    CommandId::UvTurnFlip,
    CommandId::UvStitchCut,
    CommandId::UvStitch,
    CommandId::UvCut,
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
            let in_group = UV_RING.iter().any(|g| g.members().contains(&command));
            let placed = OBJECT_RING.contains(&Some(command))
                || PIVOT_RING.contains(&Some(command))
                || MODELLING_RING.contains(&command)
                || UV_RING.contains(&command)
                || in_group;
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
        assert_eq!(CommandId::Slit.info().shortcut, plain(Key::X));
        assert_eq!(CommandId::Connect.info().shortcut, plain(Key::A));
        assert_eq!(CommandId::FillHole.info().shortcut, plain(Key::J));
        assert_eq!(CommandId::Bridge.info().shortcut, plain(Key::C));
        assert_eq!(CommandId::Weld.info().shortcut, plain(Key::M));
        assert_eq!(CommandId::FrameSelected.info().shortcut, plain(Key::Z));
        assert_eq!(
            CommandId::DeleteElements.info().shortcut,
            plain(Key::Delete)
        );
        assert_eq!(CommandId::Dissolve.info().shortcut, plain(Key::Backspace));
        // In the UV Editor: X cuts, C stitches (the project lead's choice).
        assert_eq!(CommandId::UvCut.info().shortcut, plain(Key::X));
        assert_eq!(CommandId::UvStitch.info().shortcut, plain(Key::C));
        assert_eq!(CommandId::UvPin.info().shortcut, None);
    }

    #[test]
    fn groups_open_their_commands_around_their_own_place() {
        let ring = menu_for(Mode::Uv);
        for group in [CommandId::UvTurnFlip, CommandId::UvStitchCut] {
            assert!(group.is_group());
            let place = ring.iter().find(|s| s.command == group).unwrap().place;
            let inside = group_menu(group, &ring);
            assert_eq!(
                inside.iter().map(|s| s.command).collect::<Vec<_>>(),
                group.members()
            );
            let places: HashSet<_> = inside.iter().map(|s| s.place).collect();
            assert_eq!(places.len(), inside.len(), "{group:?}");
            assert!(places.contains(&place), "{group:?}");
            for slot in &inside {
                assert!(!slot.command.is_group());
                assert_eq!(slot.places, RING_PLACES);
                let away = (slot.place + RING_PLACES - place) % RING_PLACES;
                assert!(away <= 2 || away >= RING_PLACES - 2, "{slot:?}");
            }
        }
        assert!(!CommandId::Unwrap.is_group());
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
        // The UV ring: every place filled, the gizmos where the object
        // ring has them.
        let uv: Vec<_> = menu_for(Mode::Uv).iter().map(|s| s.command).collect();
        assert_eq!(uv, UV_RING.to_vec());
        for (place, command) in UV_RING.iter().enumerate().take(5) {
            assert_eq!(OBJECT_RING[place], Some(*command));
        }
    }

    #[test]
    fn the_object_mode_menu_places_pivots_where_shapes_were() {
        let object: Vec<_> = menu_for(Mode::Object).iter().map(|s| s.command).collect();
        assert_eq!(
            object,
            PIVOT_RING.iter().flatten().copied().collect::<Vec<_>>()
        );
        for command in [
            CommandId::PivotToCentre,
            CommandId::PivotToOrigin,
            CommandId::PivotToObject,
            CommandId::ObjectToOrigin,
        ] {
            assert!(object.contains(&command), "{command:?}");
            assert!(!command.modes().contains(&Mode::Model), "{command:?}");
        }
        // A pivot has no size.
        assert!(!object.contains(&CommandId::Scale));
        // What both modes share keeps its place.
        for (place, command) in OBJECT_RING.iter().enumerate() {
            if let Some(command) = command
                && PIVOT_RING.contains(&Some(*command))
            {
                assert_eq!(PIVOT_RING[place], Some(*command), "{command:?}");
            }
        }
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
        let pivots: Vec<_> = PIVOT_RING.iter().flatten().copied().collect();
        let turns = CommandId::UvTurnFlip.members();
        let stitches = CommandId::UvStitchCut.members();
        for ring in [
            &objects[..],
            &pivots[..],
            &MODELLING_RING[..],
            &UV_RING[..],
            turns,
            stitches,
        ] {
            let icons: HashSet<_> = ring.iter().map(|c| c.info().icon).collect();
            assert_eq!(icons.len(), ring.len());
        }
    }
}
