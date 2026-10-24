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
use crate::text::tr;
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
    /// Round the selected shapes' outside sizes to whole steps of the
    /// project's kit grid (decision 0160).
    SizeToKitGrid,
    // ------------------------------------- booleans (the modifiers palette)
    /// Add the other selected shapes to the first (decision 0135).
    Union,
    /// Cut the other selected shapes out of the first.
    Subtract,
    /// Keep only where the first selected shape and the others overlap.
    Intersect,
    // ------------------------------- modifiers (the modifiers palette)
    /// Add a remesh modifier to the selected shapes (decision 0142).
    Remesh,
    /// Add a subdivision smoothing modifier to the selected shapes
    /// (decision 0143).
    Subdivide,
    /// Add a live bevel modifier to the selected shapes (decision 0149).
    LiveBevel,
    /// Add a lattice cage round the selected shapes, each with a Lattice
    /// modifier following it (decision 0144).
    Lattice,
    /// Add a decimate modifier to the selected shapes (decision 0145).
    Decimate,
    /// Add a quadify modifier to the selected shapes (decision 0150).
    Quadify,
    /// Add a weld by distance modifier to the selected shapes (decision
    /// 0152).
    WeldByDistance,
    /// Add an array modifier to the selected shapes (decision 0153).
    Array,
    /// Add a shell modifier to the selected shapes (decision 0154).
    Shell,
    /// Add a struts modifier to the selected shapes (decision 0159).
    Struts,
    /// Scatter the other selected shapes over the one selected first
    /// (decision 0146).
    Scatter,
    /// Bend the shape selected first along the curve selected after it
    /// (decision 0147).
    BendAlongCurve,
    /// Wrap the shape selected first onto the shape selected after it
    /// (decision 0147).
    Shrinkwrap,
    /// Add a displace modifier to the selected shapes (decision 0157).
    Displace,
    /// Add a bend, twist or taper modifier to the selected shapes
    /// (decision 0158): a bend, its kind changed in the Attributes panel.
    BendTwistTaper,
    /// Add a weighted normals modifier to the selected shapes (decision
    /// 0148).
    WeightedNormals,
    /// The game bevel (decision 0156): a Live bevel with soft bevels and
    /// Weighted normals last, the shapes shaded by their marked edges, in
    /// one step.
    GameBevel,
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
    /// The group holding the pivot presets (decision 0160), in the Edit
    /// pivot ring: choosing it shows them.
    PivotPresets,
    /// Move the selected objects' pivots to the bottom centre of their
    /// shapes.
    PivotToBottomCentre,
    /// Move the selected objects' pivots to a bottom corner of their
    /// shapes: the front left one, lowest along X and Y.
    PivotToBottomCorner,
    /// Move the selected objects' pivots to the top centre of their shapes.
    PivotToTopCentre,
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
    /// The group holding Crease edges and Uncrease (decision 0143), in
    /// the Edges mode's ring: choosing it shows them.
    Creases,
    /// Crease the selected edges fully, for the Subdivide modifier.
    CreaseEdges,
    /// Take the selected edges' creases away.
    Uncrease,
    /// Delete the selected points, edges or polygons, leaving a hole.
    DeleteElements,
    /// Remove edges or points, keeping the surface closed.
    Dissolve,
    /// Melt points into one.
    Weld,
    /// Join pairs of triangles into quads once, as a recorded edit: the
    /// selected polygons in the Polygons mode, whole shapes in the Model
    /// mode (decision 0150).
    QuadifyNow,
    /// Weld points closer together than a distance once, as a recorded
    /// edit: the selected elements' points in the Points, Edges and
    /// Polygons modes, whole shapes in the Model mode (decision 0152).
    WeldByDistanceNow,
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
    // ------------------------------------- curves (decision 0139)
    /// Cut a curve in two at the selected points.
    CurveSplit,
    /// Turn curves round: last point first.
    CurveReverse,
    /// Close curves into loops, or open them.
    CurveOpenClose,
    /// The group holding the four handle kinds: choosing it shows them.
    CurveHandles,
    /// Sharp points: no handles.
    CurveSharp,
    /// Smooth points: handles in one line.
    CurveSmooth,
    /// Broken handles: each moves on its own.
    CurveBroken,
    /// Auto handles: they follow the neighbours.
    CurveAuto,
    /// Round sharp corners into arcs.
    CurveRound,
    /// Spread points evenly along curves.
    CurveEven,
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
    pub fn info(self) -> CommandInfo {
        let (name, icon, shortcut, help, arrives) = match self {
            Self::Move => (
                tr("Move"),
                Icon::Move,
                plain(Key::W),
                tr("Move the selection: drag the gizmo's arrows, squares or centre"),
                None,
            ),
            Self::Rotate => (
                tr("Rotate"),
                Icon::Rotate,
                plain(Key::R),
                tr("Rotate the selection: drag the gizmo's rings"),
                None,
            ),
            Self::Scale => (
                tr("Scale"),
                Icon::Scale,
                plain(Key::S),
                tr("Scale the selection: drag the gizmo's boxes, or its centre for all axes"),
                None,
            ),
            Self::AllGizmos => (
                tr("Transform"),
                Icon::AllGizmos,
                plain(Key::T),
                tr("Move, rotate and scale at once: all three gizmos"),
                None,
            ),
            Self::MakeEditable => (
                tr("Bake into shape"),
                Icon::MakeEditable,
                Some(MAKE_EDITABLE_SHORTCUT),
                tr(
                    "Turn the selected shapes into plain meshes, ready for editing points, \
                 edges and polygons. Their settings and recorded edits are baked in. \
                 Undo brings the settings back.",
                ),
                None,
            ),
            Self::Group => (
                tr("Group objects"),
                Icon::Group,
                Some(GROUP_SHORTCUT),
                tr("Put the selected objects into a new group"),
                None,
            ),
            Self::Ungroup => (
                tr("Ungroup"),
                Icon::Ungroup,
                Some(UNGROUP_SHORTCUT),
                tr("Take the selected groups' contents out, and remove the groups"),
                None,
            ),
            Self::Delete => (
                tr("Delete objects"),
                Icon::Delete,
                Some(DELETE_SHORTCUT),
                tr("Delete the selected objects (Backspace does it too)"),
                None,
            ),
            Self::NewGroup => (
                tr("New group"),
                Icon::NewGroup,
                None,
                tr("Create an empty group, to hold and organise other objects"),
                None,
            ),
            Self::ShapeList => (
                tr("Create a shape…"),
                Icon::ShapeList,
                plain(Key::B),
                tr("Open the shape list: a cube, sphere, cylinder and the rest"),
                None,
            ),
            Self::FrameSelected => (
                tr("Frame selected"),
                Icon::FrameSelected,
                plain(Key::Z),
                tr("Turn and zoom the view so the selection fills it"),
                None,
            ),
            Self::Duplicate => (
                tr("Duplicate"),
                Icon::Duplicate,
                None,
                tr(
                    "Copy the selected objects, with their edits, right where they are. Or Ctrl + C, \
                 then Ctrl + V to paste copies.",
                ),
                None,
            ),
            Self::Instance => (
                tr("Duplicate as instance"),
                Icon::Instance,
                None,
                tr(
                    "Make instances of the selected objects right where they are: they show the \
                 original's shape and follow its changes. Or Alt + C, then Alt + V.",
                ),
                None,
            ),
            Self::Join => (
                tr("Join objects"),
                Icon::Join,
                None,
                tr(
                    "Join the selected shapes into the first of them, as one plain mesh, each part \
                 staying where it was",
                ),
                None,
            ),
            Self::ApplyTransforms => (
                tr("Apply rotation and scale"),
                Icon::ApplyTransforms,
                None,
                tr(
                    "Turn the selected objects back to 0° and 100% while they look exactly the \
                 same: their shapes take the rotation and scale, so edits measure as they look. \
                 Mesh → Apply transforms has position too.",
                ),
                None,
            ),
            Self::Union => (
                tr("Union"),
                Icon::Union,
                None,
                tr(
                    "Add the other selected shapes to the one selected first, as one solid.                  They stay shapes of their own, drawn as outlines: move or edit one and the                  result follows.",
                ),
                None,
            ),
            Self::Subtract => (
                tr("Subtract"),
                Icon::Subtract,
                None,
                tr(
                    "Cut the other selected shapes out of the one selected first, like a hole                  or a notch. The cutters stay shapes of their own, drawn as outlines: move or                  edit one and the cut follows.",
                ),
                None,
            ),
            Self::Intersect => (
                tr("Intersect"),
                Icon::Intersect,
                None,
                tr(
                    "Keep only where the shape selected first and the other selected shapes                  overlap. They stay shapes of their own, drawn as outlines: move or edit one                  and the result follows.",
                ),
                None,
            ),
            // A modifier's name and description come from the scene's
            // list of kinds (decision 0142).
            Self::Remesh => {
                let kind = scene::ModifierKind::Remesh(Default::default());
                (kind.name(), Icon::Remesh, None, kind.description(), None)
            }
            Self::Subdivide => {
                let kind = scene::ModifierKind::Subdivide(Default::default());
                (kind.name(), Icon::Subdivide, None, kind.description(), None)
            }
            Self::LiveBevel => {
                let kind = scene::ModifierKind::LiveBevel(Default::default());
                (kind.name(), Icon::LiveBevel, None, kind.description(), None)
            }
            Self::Decimate => {
                let kind = scene::ModifierKind::Decimate(Default::default());
                (kind.name(), Icon::Decimate, None, kind.description(), None)
            }
            Self::Quadify => {
                let kind = scene::ModifierKind::Quadify(Default::default());
                (kind.name(), Icon::Quadify, None, kind.description(), None)
            }
            Self::WeldByDistance => {
                let kind = scene::ModifierKind::WeldByDistance(Default::default());
                (
                    kind.name(),
                    Icon::WeldByDistance,
                    None,
                    kind.description(),
                    None,
                )
            }
            Self::Array => {
                let kind = scene::ModifierKind::Array(Default::default());
                (kind.name(), Icon::Array, None, kind.description(), None)
            }
            Self::Shell => {
                let kind = scene::ModifierKind::Shell(Default::default());
                (kind.name(), Icon::Shell, None, kind.description(), None)
            }
            Self::Struts => {
                let kind = scene::ModifierKind::Struts(Default::default());
                (kind.name(), Icon::Struts, None, kind.description(), None)
            }
            Self::Displace => {
                let kind = scene::ModifierKind::Displace(Default::default());
                (kind.name(), Icon::Displace, None, kind.description(), None)
            }
            // Named for all three kinds: it adds a bend, whose kind is
            // changed in the Attributes panel.
            Self::BendTwistTaper => {
                let kind = scene::ModifierKind::BendTwistTaper(Default::default());
                (
                    tr("Bend, twist or taper"),
                    Icon::BendTwistTaper,
                    None,
                    kind.description(),
                    None,
                )
            }
            Self::WeldByDistanceNow => (
                tr("Weld by distance now"),
                Icon::WeldByDistance,
                None,
                tr(
                    "Melt points closer together than a distance into one, once: the selected \
                     points (or those of the selected edges or polygons), or whole shapes in the \
                     Model mode. Change its settings under Last edit.",
                ),
                None,
            ),
            Self::QuadifyNow => (
                tr("Quadify now"),
                Icon::Quadify,
                None,
                tr(
                    "Join pairs of triangles into four-sided polygons once, the squarest first: \
                 the selected polygons in the Polygons mode, whole shapes in the Model mode. \
                 Change its settings under Last edit.",
                ),
                None,
            ),
            Self::WeightedNormals => {
                let kind = scene::ModifierKind::WeightedNormals(Default::default());
                (
                    kind.name(),
                    Icon::WeightedNormals,
                    None,
                    kind.description(),
                    None,
                )
            }
            Self::GameBevel => (
                tr("Game bevel"),
                Icon::GameBevel,
                None,
                tr(
                    "Ready the selected shapes for a game in one step, without baking: a Live \
                     bevel rounds every edge sharper than 30° (2 cm, 2 rows) and shades round, \
                     and Weighted normals keeps the big faces flat. Change any of it afterwards \
                     in the Attributes panel.",
                ),
                None,
            ),
            // Named for what it does with the selection (decision 0146).
            Self::Scatter => {
                let kind = scene::ModifierKind::Scatter(Default::default());
                (
                    tr("Scatter onto surface"),
                    Icon::Scatter,
                    None,
                    kind.description(),
                    None,
                )
            }
            // Bend along curve is named as its modifier is; Shrinkwrap says
            // where to (decision 0147).
            Self::BendAlongCurve => {
                let kind = scene::ModifierKind::BendAlongCurve(Default::default());
                (
                    kind.name(),
                    Icon::BendAlongCurve,
                    None,
                    kind.description(),
                    None,
                )
            }
            Self::Shrinkwrap => {
                let kind = scene::ModifierKind::Shrinkwrap(Default::default());
                (
                    tr("Shrinkwrap onto"),
                    Icon::Shrinkwrap,
                    None,
                    kind.description(),
                    None,
                )
            }
            // Adding a lattice makes its cage too (decision 0144).
            Self::Lattice => {
                let kind = scene::ModifierKind::Lattice(Default::default());
                (
                    tr("Add lattice"),
                    Icon::Lattice,
                    None,
                    kind.description(),
                    None,
                )
            }
            Self::PivotToCentre => (
                tr("Pivot to centre"),
                Icon::PivotToCentre,
                None,
                tr(
                    "Move the selected objects' pivots to the middle of their shapes; the shapes \
                 stay where they are",
                ),
                None,
            ),
            Self::PivotToOrigin => (
                tr("Pivot to world origin"),
                Icon::PivotToOrigin,
                None,
                tr(
                    "Move the selected objects' pivots to the world origin, where the red and \
                 blue lines cross; the shapes stay where they are",
                ),
                None,
            ),
            Self::PivotToObject => (
                tr("Pivot to object…"),
                Icon::PivotToObject,
                None,
                tr(
                    "Then click another object: the selected objects' pivots move to its pivot, \
                 and the shapes stay where they are. Esc cancels.",
                ),
                None,
            ),
            Self::ObjectToOrigin => (
                tr("Object to world origin"),
                Icon::ObjectToOrigin,
                None,
                tr(
                    "Move the selected objects, shape and pivot together, so their pivots sit at \
                 the world origin",
                ),
                None,
            ),
            Self::PivotPresets
            | Self::PivotToBottomCentre
            | Self::PivotToBottomCorner
            | Self::PivotToTopCentre
            | Self::SizeToKitGrid => crate::kit::command_info(self),
            Self::Extrude => (
                tr("Extrude"),
                Icon::Extrude,
                plain(Key::D),
                tr(
                    "Push the selected polygons or edges out, building new sides between \
                 the old place and the new. With points selected along an open edge, \
                 the edges between them grow new polygons",
                ),
                None,
            ),
            Self::Inset => (
                tr("Inset"),
                Icon::Inset,
                plain(Key::I),
                tr(
                    "Shrink the selected polygons inward, leaving a ring of new polygons \
                 around each",
                ),
                None,
            ),
            Self::Bevel => (
                tr("Bevel"),
                Icon::Bevel,
                plain(Key::V),
                tr(
                    "Cut the corner off the selected edges or points, with a width, \
                 segments and roundness",
                ),
                None,
            ),
            Self::Groove => (
                tr("Groove"),
                Icon::Groove,
                plain(Key::K),
                tr(
                    "Cut a thin groove into the shape along the selected edges, like a panel \
                 line, or raise a ridge out of it: drag in or out",
                ),
                None,
            ),
            Self::Bridge => (
                tr("Bridge"),
                Icon::Bridge,
                plain(Key::C),
                tr(
                    "Build polygons between two open borders or two polygons; with nothing to \
                 bridge selected, drag from one open edge to another",
                ),
                None,
            ),
            Self::FillHole => (
                tr("Fill hole"),
                Icon::FillHole,
                plain(Key::J),
                tr(
                    "Close the open holes touching the selection with a polygon each; with \
                 nothing selected, every hole",
                ),
                None,
            ),
            Self::FlipNormals => (
                tr("Flip normals"),
                Icon::FlipNormals,
                None,
                tr("Turn the selected polygons inside out, for when they face the wrong way"),
                None,
            ),
            Self::HardEdges => (
                tr("Mark hard edges"),
                Icon::HardEdges,
                None,
                tr(
                    "Shade the selected edges (or the selected polygons' edges) as crisp edges: \
                 the light breaks there",
                ),
                None,
            ),
            Self::SoftEdges => (
                tr("Mark soft edges"),
                Icon::SoftEdges,
                None,
                tr(
                    "Shade the selected edges (or the selected polygons' edges) smoothly: the light \
                 flows across, so a curve looks round",
                ),
                None,
            ),
            Self::Creases => (
                tr("Creases"),
                Icon::Creases,
                None,
                tr(
                    "Crease the selected edges, so the Subdivide modifier keeps them sharp, or \
                 take their creases away. Choose this to see both.",
                ),
                None,
            ),
            Self::CreaseEdges => (
                tr("Crease edges"),
                Icon::CreaseEdges,
                None,
                tr(
                    "Crease the selected edges fully, so the Subdivide modifier keeps them sharp. \
                 Set a softer crease in the Attributes panel's Crease field.",
                ),
                None,
            ),
            Self::Uncrease => (
                tr("Uncrease"),
                Icon::Uncrease,
                None,
                tr("Take the selected edges' creases away, so the Subdivide modifier smooths them"),
                None,
            ),
            Self::DeleteElements => (
                tr("Delete points, edges or polygons"),
                Icon::Delete,
                plain(Key::Delete),
                tr(
                    "Delete the selected points, edges or polygons, and the polygons using them, \
                 leaving a hole",
                ),
                None,
            ),
            Self::Dissolve => (
                tr("Dissolve"),
                Icon::Dissolve,
                plain(Key::Backspace),
                tr(
                    "Take the selected points, edges or polygons away but keep the surface \
                 closed: the polygons around them merge into one",
                ),
                None,
            ),
            Self::Weld => (
                tr("Weld"),
                Icon::Weld,
                plain(Key::M),
                tr(
                    "Melt the selected points into one at their middle (selected polygons: each \
                 touching group at its middle, such as an inset cap into a fan of triangles); \
                 Ctrl + M to drag a point or polygon onto a point",
                ),
                None,
            ),
            Self::Connect => (
                tr("Connect"),
                Icon::Connect,
                plain(Key::A),
                tr("Join the selected points or edges with a new edge across their polygon"),
                None,
            ),
            Self::Slit => (
                tr("Slit"),
                Icon::Slit,
                plain(Key::X),
                tr(
                    "Cut new edges along a line: click or drag its points over the shape; \
                 Enter finishes, Esc cancels, or click the first point to close a loop",
                ),
                None,
            ),
            Self::LoopCut => (
                tr("Loop cut"),
                Icon::LoopCut,
                plain(Key::F),
                tr(
                    "Cut a new loop of edges all the way around a shape: point at an edge, \
                 then click, or drag to slide it",
                ),
                None,
            ),
            Self::UvTurnLeft => (
                tr("Turn left 90°"),
                Icon::UvTurnLeft,
                None,
                tr(
                    "Turn the selected UVs a quarter turn to the left (anticlockwise), about \
                 their middle",
                ),
                None,
            ),
            Self::UvTurnRight => (
                tr("Turn right 90°"),
                Icon::UvTurnRight,
                None,
                tr(
                    "Turn the selected UVs a quarter turn to the right (clockwise), about their \
                 middle",
                ),
                None,
            ),
            Self::UvFlipU => (
                tr("Flip left to right"),
                Icon::UvFlipU,
                None,
                tr("Mirror the selected UVs left to right, about their middle"),
                None,
            ),
            Self::UvFlipV => (
                tr("Flip upside down"),
                Icon::UvFlipV,
                None,
                tr("Mirror the selected UVs top to bottom, about their middle"),
                None,
            ),
            Self::MarkSeam => (
                tr("Mark seams"),
                Icon::MarkSeam,
                None,
                tr(
                    "Mark the selected edges as seams: where the surface is cut open to lie \
                 flat. With polygons selected, the edges around them.",
                ),
                None,
            ),
            Self::ClearSeam => (
                tr("Clear seams"),
                Icon::ClearSeam,
                None,
                tr(
                    "Take the seams off the selected edges (or the selected polygons' edges), so the \
                 surface stays joined there when unwrapped",
                ),
                None,
            ),
            Self::Unwrap => (
                tr("Unwrap"),
                Icon::Unwrap,
                None,
                tr(
                    "Lay the pieces holding the selection (or whole objects) flat along their \
                 seams, with as little stretching as can be, at the object's texel density",
                ),
                None,
            ),
            Self::Relax => (
                tr("Relax"),
                Icon::Relax,
                None,
                tr("Even out the selected polygons' UVs, so a texture stretches less over them"),
                None,
            ),
            Self::Straighten => (
                tr("Straighten"),
                Icon::Straighten,
                None,
                tr("Line the pieces made of quads up into a straight grid, for strips and panels"),
                None,
            ),
            Self::Pack => (
                tr("Pack"),
                Icon::Pack,
                None,
                tr(
                    "Fit the islands holding the selection (or whole objects) into the texture \
                 square as large as they go, with padding between them (UV → Packing)",
                ),
                None,
            ),
            Self::UvPin => (
                tr("Pin"),
                Icon::UvPin,
                None,
                tr(
                    "Pin the selected UVs where they are: unwrapping lays the rest out around them, \
                 and relaxing leaves them alone",
                ),
                None,
            ),
            Self::UvUnpin => (
                tr("Unpin"),
                Icon::UvUnpin,
                None,
                tr("Unpin the selected UVs, so unwrapping and relaxing may move them again"),
                None,
            ),
            Self::UvTurnFlip => (
                tr("Turn & flip"),
                Icon::UvTurnFlip,
                None,
                tr(
                    "Turn the selected UVs a quarter turn, or mirror them: choose this to see the \
                 four",
                ),
                None,
            ),
            Self::CurveSplit
            | Self::CurveReverse
            | Self::CurveOpenClose
            | Self::CurveHandles
            | Self::CurveSharp
            | Self::CurveSmooth
            | Self::CurveBroken
            | Self::CurveAuto
            | Self::CurveRound
            | Self::CurveEven => curve_info(self),
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
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// The commands a group holds (empty for a command that isn't a
    /// group). Choosing a group in the tool menu shows its commands in
    /// their place (decision 0109).
    pub const fn members(self) -> &'static [CommandId] {
        match self {
            Self::CurveHandles => &[
                Self::CurveSharp,
                Self::CurveSmooth,
                Self::CurveBroken,
                Self::CurveAuto,
            ],
            Self::Creases => &[Self::CreaseEdges, Self::Uncrease],
            Self::PivotPresets => &[
                Self::PivotToBottomCentre,
                Self::PivotToBottomCorner,
                Self::PivotToCentre,
                Self::PivotToTopCentre,
            ],
            Self::UvTurnFlip => &[
                Self::UvTurnLeft,
                Self::UvTurnRight,
                Self::UvFlipU,
                Self::UvFlipV,
            ],
            _ => &[],
        }
    }

    /// Whether it is a group of commands.
    pub const fn is_group(self) -> bool {
        !self.members().is_empty()
    }

    /// Whether it is built.
    pub fn is_built(self) -> bool {
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
            | Self::ObjectToOrigin
            | Self::PivotPresets
            | Self::PivotToBottomCentre
            | Self::PivotToBottomCorner
            | Self::PivotToTopCentre => &[Object],
            // Shapes are sized in the Model mode, from the Create menu.
            Self::SizeToKitGrid => &[Model],
            // Booleans work on whole objects whatever the mode; they live
            // in the modifiers palette and the Mesh menu, not the tool
            // menu (decision 0135).
            // So do modifiers (decision 0142).
            Self::Union
            | Self::Subtract
            | Self::Intersect
            | Self::Remesh
            | Self::Subdivide
            | Self::LiveBevel
            | Self::Lattice
            | Self::Decimate
            | Self::Quadify
            | Self::WeldByDistance
            | Self::Array
            | Self::Shell
            | Self::Struts
            | Self::Scatter
            | Self::BendAlongCurve
            | Self::Shrinkwrap
            | Self::Displace
            | Self::BendTwistTaper
            | Self::WeightedNormals
            | Self::GameBevel => &[Model, Object, Points, Edges, Polygons],
            // Whole shapes, or the selected polygons (decision 0150).
            Self::QuadifyNow => &[Model, Polygons],
            // Whole shapes, or the selected elements' points (decision 0152).
            Self::WeldByDistanceNow => &[Model, Points, Edges, Polygons],
            Self::Extrude => &[Points, Edges, Polygons],
            Self::Bridge => &[Edges, Polygons],
            Self::Inset | Self::FlipNormals => &[Polygons],
            Self::Bevel | Self::Connect => &[Points, Edges],
            Self::Groove => &[Edges],
            Self::FillHole => &[Edges, Polygons],
            Self::HardEdges | Self::SoftEdges => &[Edges, Polygons],
            // Creases are set on edges (decision 0143).
            Self::Creases | Self::CreaseEdges | Self::Uncrease => &[Edges],
            Self::Weld => &[Points, Polygons],
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
            | Self::UvTurnFlip => &[Uv],
            // The curve ring, in the Points mode with curves selected
            // (decision 0139). Reverse, open or close and even spacing
            // work on whole curves in the Model mode too, from the menus.
            Self::CurveSplit
            | Self::CurveReverse
            | Self::CurveOpenClose
            | Self::CurveHandles
            | Self::CurveSharp
            | Self::CurveSmooth
            | Self::CurveBroken
            | Self::CurveAuto
            | Self::CurveRound
            | Self::CurveEven => &[Points],
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
    Some(CommandId::PivotPresets),
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

/// Places of the modelling ring that hold another command in the Edges
/// mode, where the command [`MODELLING_RING`] has there works on polygons
/// only: Flip normals' place, beside Hard and Soft edges, holds the
/// creases (decision 0143).
pub const EDGES_ONLY: [(usize, CommandId); 1] = [(5, CommandId::Creases)];

/// The tool menu's places for UVs: in the UV mode, and with the mouse
/// over the UV Editor in the Points, Edges and Polygons modes (the menu
/// follows where you work, decision 0097). The gizmos and framing keep
/// their places from the object ring; seams at the right, unwrapping at
/// the bottom, packing, turning and pinning at the left. Turning and
/// flipping share one place: choosing it shows what it holds (decision
/// 0109).
pub const UV_RING: [Option<CommandId>; RING_PLACES] = [
    Some(CommandId::Move),
    Some(CommandId::Rotate),
    Some(CommandId::Scale),
    Some(CommandId::AllGizmos),
    Some(CommandId::FrameSelected),
    Some(CommandId::MarkSeam),
    Some(CommandId::ClearSeam),
    Some(CommandId::Unwrap),
    Some(CommandId::Relax),
    Some(CommandId::Straighten),
    Some(CommandId::Pack),
    Some(CommandId::UvTurnFlip),
    Some(CommandId::UvPin),
    Some(CommandId::UvUnpin),
    // Stitch and cut take this place when they're built (Phase 8): no
    // placeholder until then (decision 0126).
    None,
];

/// The tool menu's places for a curve's points (decision 0139): in the
/// Points mode when only curves are selected. Weld, Connect and Delete
/// keep their places from the modelling ring; the handle kinds share one
/// place, opened like the UV ring's turns.
pub const CURVE_RING: [Option<CommandId>; RING_PLACES] = [
    Some(CommandId::CurveHandles),
    Some(CommandId::CurveRound),
    Some(CommandId::CurveEven),
    None,
    None,
    Some(CommandId::CurveReverse),
    Some(CommandId::CurveOpenClose),
    None,
    Some(CommandId::DeleteElements),
    None,
    Some(CommandId::Weld),
    Some(CommandId::Connect),
    Some(CommandId::CurveSplit),
    None,
    None,
];

/// The curve ring's entries (decision 0139).
pub fn curve_menu() -> Vec<Slot> {
    CURVE_RING
        .iter()
        .enumerate()
        .filter_map(|(place, command)| {
            Some(Slot {
                command: (*command)?,
                place,
                places: RING_PLACES,
            })
        })
        .collect()
}

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
        Mode::Uv => UV_RING,
    };
    let mut slots: Vec<Slot> = ring
        .iter()
        .enumerate()
        .filter_map(|(place, command)| Some((place, (*command)?)))
        .filter(|(_, c)| c.modes().contains(&mode))
        .map(|(place, command)| Slot {
            command,
            place,
            places: RING_PLACES,
        })
        .collect();
    if mode == Mode::Edges {
        for (place, command) in EDGES_ONLY {
            if !slots.iter().any(|s| s.place == place) {
                slots.push(Slot {
                    command,
                    place,
                    places: RING_PLACES,
                });
            }
        }
        slots.sort_by_key(|s| s.place);
    }
    slots
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

/// The commands with no place on the tool menu: the booleans (decision
/// 0135) and the modifiers (decision 0142), found in the modifiers
/// palette, the Mesh menu and command search; and Quadify now (decision
/// 0150) and Weld by distance now (decision 0152), in the Mesh menu's
/// Clean up group and command search; and the game bevel (decision 0156),
/// beside the modifiers; and Size to kit grid (decision 0160), in the
/// Create menu's Modular kit list.
pub const OFF_THE_RING: [CommandId; 23] = [
    CommandId::Union,
    CommandId::Subtract,
    CommandId::Intersect,
    CommandId::Remesh,
    CommandId::Subdivide,
    CommandId::LiveBevel,
    CommandId::Lattice,
    CommandId::Decimate,
    CommandId::Quadify,
    CommandId::WeldByDistance,
    CommandId::Array,
    CommandId::Shell,
    CommandId::Struts,
    CommandId::Scatter,
    CommandId::BendAlongCurve,
    CommandId::Shrinkwrap,
    CommandId::Displace,
    CommandId::BendTwistTaper,
    CommandId::WeightedNormals,
    CommandId::GameBevel,
    CommandId::QuadifyNow,
    CommandId::WeldByDistanceNow,
    CommandId::SizeToKitGrid,
];

/// Every command, for tests and the menus.
pub const ALL: [CommandId; 87] = [
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
    CommandId::SizeToKitGrid,
    CommandId::Union,
    CommandId::Subtract,
    CommandId::Intersect,
    CommandId::Remesh,
    CommandId::Subdivide,
    CommandId::LiveBevel,
    CommandId::Lattice,
    CommandId::Decimate,
    CommandId::Quadify,
    CommandId::WeldByDistance,
    CommandId::Array,
    CommandId::Shell,
    CommandId::Struts,
    CommandId::Scatter,
    CommandId::BendAlongCurve,
    CommandId::Shrinkwrap,
    CommandId::Displace,
    CommandId::BendTwistTaper,
    CommandId::WeightedNormals,
    CommandId::GameBevel,
    CommandId::PivotToCentre,
    CommandId::PivotToOrigin,
    CommandId::PivotToObject,
    CommandId::ObjectToOrigin,
    CommandId::PivotPresets,
    CommandId::PivotToBottomCentre,
    CommandId::PivotToBottomCorner,
    CommandId::PivotToTopCentre,
    CommandId::Extrude,
    CommandId::Inset,
    CommandId::Bevel,
    CommandId::Groove,
    CommandId::Bridge,
    CommandId::FillHole,
    CommandId::FlipNormals,
    CommandId::HardEdges,
    CommandId::SoftEdges,
    CommandId::Creases,
    CommandId::CreaseEdges,
    CommandId::Uncrease,
    CommandId::DeleteElements,
    CommandId::Dissolve,
    CommandId::Weld,
    CommandId::QuadifyNow,
    CommandId::WeldByDistanceNow,
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
    CommandId::CurveSplit,
    CommandId::CurveReverse,
    CommandId::CurveOpenClose,
    CommandId::CurveHandles,
    CommandId::CurveSharp,
    CommandId::CurveSmooth,
    CommandId::CurveBroken,
    CommandId::CurveAuto,
    CommandId::CurveRound,
    CommandId::CurveEven,
];

/// Name, icon, key, help and arrival of a curve command (decision 0139).
pub(crate) type Info = (
    &'static str,
    Icon,
    Option<KeyboardShortcut>,
    &'static str,
    Option<&'static str>,
);

/// What is shown about the curve commands (decision 0139). None has a key
/// of its own: they are in the E menu, the Mesh menu's Curve list and
/// command search.
fn curve_info(command: CommandId) -> Info {
    let (name, icon, help) = match command {
        CommandId::CurveSplit => (
            tr("Split curve"),
            Icon::CurveSplit,
            tr(
                "Cut the curve in two at the selected points: each piece becomes a curve of its \
                 own, with the same sweep or lathe. A closed curve opens there.",
            ),
        ),
        CommandId::CurveReverse => (
            tr("Reverse curve"),
            Icon::CurveReverse,
            tr(
                "Turn the selected curves round, so they run from their last point to their \
                 first: a sweep's twist and caps follow",
            ),
        ),
        CommandId::CurveOpenClose => (
            tr("Open or close curve"),
            Icon::CurveOpenClose,
            tr(
                "Close the selected curves into loops, the last point joining the first with a \
                 straight stretch, or open closed ones again",
            ),
        ),
        CommandId::CurveHandles => (
            tr("Handle type"),
            Icon::CurveHandles,
            tr(
                "How the selected points' handles behave: sharp, smooth, broken or auto. \
                 Choose this to see the four.",
            ),
        ),
        CommandId::CurveSharp => (
            tr("Sharp points"),
            Icon::CurveSharp,
            tr("Take the selected points' handles away: a corner"),
        ),
        CommandId::CurveSmooth => (
            tr("Smooth points"),
            Icon::CurveSmooth,
            tr(
                "Keep the selected points' handles in one line, so the curve bends through \
                 without a kink: dragging one handle turns the other",
            ),
        ),
        CommandId::CurveBroken => (
            tr("Broken handles"),
            Icon::CurveBroken,
            tr(
                "Let the selected points' two handles move on their own, for a kink with \
                 curved sides",
            ),
        ),
        CommandId::CurveAuto => (
            tr("Auto handles"),
            Icon::CurveAuto,
            tr(
                "Let the selected points' handles shape themselves from their neighbours, a \
                 third of the way to each, following as the neighbours move",
            ),
        ),
        CommandId::CurveRound => (
            tr("Round corners"),
            Icon::CurveRound,
            tr(
                "Turn the selected sharp points into rounded corners of the radius set in the \
                 Attributes panel, each cut back to what fits between its neighbours",
            ),
        ),
        _ => (
            tr("Even spacing"),
            Icon::CurveEven,
            tr(
                "Spread the selected curves' points evenly along them, keeping the shape as \
                 near as can be: as many points as the Attributes panel's Even spacing says",
            ),
        ),
    };
    (name, icon, None, help, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_command_is_listed_once_and_has_a_place() {
        let all: HashSet<_> = ALL.into_iter().collect();
        assert_eq!(all.len(), ALL.len());
        for command in ALL {
            let in_group = UV_RING
                .iter()
                .chain(CURVE_RING.iter())
                .chain(PIVOT_RING.iter())
                .flatten()
                .chain(EDGES_ONLY.iter().map(|(_, c)| c))
                .any(|g| g.members().contains(&command));
            let placed = OBJECT_RING.contains(&Some(command))
                || CURVE_RING.contains(&Some(command))
                || PIVOT_RING.contains(&Some(command))
                || MODELLING_RING.contains(&command)
                || UV_RING.contains(&Some(command))
                || OFF_THE_RING.contains(&command)
                || EDGES_ONLY.iter().any(|(_, c)| *c == command)
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
        assert_eq!(CommandId::UvPin.info().shortcut, None);
    }

    #[test]
    fn groups_open_their_commands_around_their_own_place() {
        let ring = menu_for(Mode::Uv);
        for group in [CommandId::UvTurnFlip] {
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
        // The curve ring's handle kinds open the same way.
        let curve = curve_menu();
        let place = curve
            .iter()
            .find(|s| s.command == CommandId::CurveHandles)
            .unwrap()
            .place;
        let inside = group_menu(CommandId::CurveHandles, &curve);
        assert_eq!(inside.len(), 4);
        assert!(inside.iter().any(|s| s.place == place));
    }

    #[test]
    fn the_curve_ring_keeps_the_modelling_rings_places_for_what_they_share() {
        for slot in curve_menu() {
            if let Some(at) = MODELLING_RING.iter().position(|c| *c == slot.command) {
                assert_eq!(at, slot.place, "{:?}", slot.command);
            }
            assert_eq!(slot.places, RING_PLACES);
        }
        let commands: Vec<_> = curve_menu().iter().map(|s| s.command).collect();
        for command in [
            CommandId::Weld,
            CommandId::Connect,
            CommandId::DeleteElements,
        ] {
            assert!(commands.contains(&command), "{command:?}");
        }
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
        // The UV ring: the gizmos where the object ring has them, and no
        // placeholder for what isn't built (decision 0126).
        let uv: Vec<_> = menu_for(Mode::Uv).iter().map(|s| s.command).collect();
        assert_eq!(uv, UV_RING.iter().flatten().copied().collect::<Vec<_>>());
        for (place, command) in UV_RING.iter().enumerate().take(5) {
            assert_eq!(OBJECT_RING[place], *command);
        }
        assert!(ALL.iter().all(|c| c.is_built()));
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
                let extra = mode == Mode::Edges && EDGES_ONLY.contains(&(slot.place, slot.command));
                assert!(
                    MODELLING_RING[slot.place] == slot.command || extra,
                    "{slot:?}"
                );
                assert_eq!(slot.places, MODELLING_RING.len());
            }
        }
        // The creases take a place left empty in the Edges mode only.
        for (place, command) in EDGES_ONLY {
            assert!(!MODELLING_RING[place].modes().contains(&Mode::Edges));
            assert!(command.is_group());
            let edges = menu_for(Mode::Edges);
            assert!(
                edges
                    .iter()
                    .any(|s| s.place == place && s.command == command)
            );
            let inside = group_menu(command, &edges);
            assert_eq!(inside.len(), command.members().len());
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
        let uv: Vec<_> = UV_RING.iter().flatten().copied().collect();
        let curve: Vec<_> = CURVE_RING.iter().flatten().copied().collect();
        let kinds = CommandId::CurveHandles.members();
        let edges: Vec<_> = menu_for(Mode::Edges).iter().map(|s| s.command).collect();
        let creases = CommandId::Creases.members();
        let presets = CommandId::PivotPresets.members();
        for ring in [
            presets,
            &objects[..],
            &pivots[..],
            &MODELLING_RING[..],
            &edges[..],
            creases,
            &uv[..],
            turns,
            &curve[..],
            kinds,
        ] {
            let icons: HashSet<_> = ring.iter().map(|c| c.info().icon).collect();
            assert_eq!(icons.len(), ring.len());
        }
    }
}
