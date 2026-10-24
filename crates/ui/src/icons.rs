//! Small vector icons for the palettes, drawn with egui's shapes.
//!
//! Drawn in code rather than shipped as images: nothing to license, sharp
//! at any interface scale, and coloured by the theme. Each icon is
//! designed on a 24 × 24 grid and scaled into the rectangle it is given.
//! These are original drawings; no icons are copied from other software.

mod booleans;
mod curves;
mod general;
mod modelling;
mod modifiers;
mod pen;
mod primitives;
mod tools;
mod uv;

use egui::{Color32, Painter, Rect, Shape};
use pen::Pen;

/// Every icon the palettes use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// Edit → Undo.
    Undo,
    /// Pivot to centre.
    PivotToCentre,
    /// Pivot to world origin.
    PivotToOrigin,
    /// Pivot to object.
    PivotToObject,
    /// Object to world origin.
    ObjectToOrigin,
    /// Edit → Redo.
    Redo,
    /// Paint select: paint to select.
    PaintSelect,
    /// Box selection: drag a box.
    BoxSelection,
    /// Lasso selection: draw around what to select.
    LassoSelection,
    /// Select only what can be seen: a box whose hidden edges are left
    /// out.
    VisibleOnly,
    /// Move tool.
    Move,
    /// Scale tool.
    Scale,
    /// Rotate tool.
    Rotate,
    /// World or object coordinates.
    World,
    /// Cube primitive, and Model mode.
    Cube,
    /// Cylinder primitive.
    Cylinder,
    /// Plane primitive.
    Plane,
    /// Sphere primitive.
    Sphere,
    /// Null object (an empty group).
    Null,
    /// Snapping.
    Snap,
    /// Symmetry.
    Symmetry,
    /// Command search.
    Search,
    /// Object mode: work on whole objects and their axes.
    ObjectMode,
    /// Points mode.
    Points,
    /// Edges mode.
    Edges,
    /// Polygons mode.
    Polygons,
    /// UV mode.
    Uv,
    /// Workplane.
    Workplane,
    /// Solo: show only the selection.
    Solo,
    /// Grid detail: the ground grid's fine lines.
    GridDetail,
    /// Zoom (dolly) the view: an up-and-down double arrow.
    Dolly,
    /// Make a view fill the whole viewport area.
    Maximise,
    /// A menu: three short lines.
    Menu,
    /// The mouse, for the navigation controls.
    Mouse,
    /// Shown in the viewport: an open eye.
    Shown,
    /// Hidden from the viewport: a closed eye.
    Hidden,
    /// The disc primitive: a flat circle seen at an angle.
    Disc,
    /// The tube primitive: a cylinder with a hole.
    Tube,
    /// The cone primitive.
    Cone,
    /// The icosphere primitive: a ball of triangles.
    Icosphere,
    /// The torus primitive: a ring.
    Torus,
    /// The capsule primitive: a rounded pill.
    Capsule,
    /// The dome primitive: half a ball.
    Dome,
    /// The pipe elbow primitive: a bent pipe.
    Elbow,
    /// The pie sector primitive: a slice.
    Sector,
    /// The gear primitive: a toothed wheel.
    Gear,
    /// The hex grid primitive: hexagons side by side.
    HexGrid,
    /// The hex sphere primitive: a ball with a hexagon on it.
    HexSphere,
    /// The stairs primitive: steps.
    Stairs,
    /// The arch primitive: a doorway.
    Arch,
    /// Move, rotate and scale at once.
    AllGizmos,
    /// Make editable: a cube showing its points.
    MakeEditable,
    /// Group objects: two shapes held together.
    Group,
    /// Ungroup: two shapes moving apart.
    Ungroup,
    /// Delete: a bin.
    Delete,
    /// Frame the selection: corners around a point.
    FrameSelected,
    /// Extrude: a polygon lifted from its place, with new sides.
    Extrude,
    /// Inset: a polygon with a smaller one inside it.
    Inset,
    /// Bevel: a corner cut off.
    Bevel,
    /// Groove: a thin line cut into a surface.
    Groove,
    /// Bridge: two pieces joined by a span.
    Bridge,
    /// Fill hole: an outline filled in.
    FillHole,
    /// Flip normals: a surface with arrows on both sides.
    FlipNormals,
    /// Hard edges: two sides meeting at a sharp crease.
    HardEdges,
    /// Soft edges: a smooth bend.
    SoftEdges,
    /// Dissolve: an edge fading out of a polygon.
    Dissolve,
    /// Weld: two points meeting in one.
    Weld,
    /// Connect: a new edge across a polygon.
    Connect,
    /// Slit: a blade cutting a line.
    Slit,
    /// Loop cut: a new loop around a shape.
    LoopCut,
    /// New group: a dashed outline with a plus.
    NewGroup,
    /// Create a shape (the shape list): a cube with a plus.
    ShapeList,
    /// Cancel, in the middle of the tool menu: a circle with a line.
    Cancel,
    /// Duplicate: a square with a copy behind it.
    Duplicate,
    /// Instance: a square with a dashed copy behind it, joined to it.
    Instance,
    /// Join objects: two squares sharing a side, made one.
    Join,
    /// Apply transforms: a turned, stretched box made square again.
    ApplyTransforms,
    /// Add an edit layer: a stack of sheets with a new one on top.
    EditLayer,
    /// Union: a square and a circle made one solid.
    Union,
    /// Subtract: a square with a round bite taken out by a dashed circle.
    Subtract,
    /// Intersect: only where a square and a circle overlap is left.
    Intersect,
    /// Remesh (decision 0142): a round shape covered by an even grid.
    Remesh,
    /// Subdivide (decision 0143): a dashed square cage and the rounded
    /// shape it makes.
    Subdivide,
    /// Live bevel (decision 0149): a box's corner with its edge rounded
    /// off, and a small circular arrow: worked out again as it changes.
    LiveBevel,
    /// Lattice (decision 0144): a cage of points pulled out on one side,
    /// the shape inside swelling with it.
    Lattice,
    /// Decimate (decision 0145): many small triangles becoming a few big
    /// ones.
    Decimate,
    /// Scatter (decision 0146): little shapes standing on a curved
    /// surface, each its own way.
    Scatter,
    /// Bend along curve (decision 0147): a striped bar following a
    /// curve drawn under it.
    BendAlongCurve,
    /// Shrinkwrap (decision 0147): a sheet pulled down onto a hump,
    /// arrows pressing it on.
    Shrinkwrap,
    /// Displace (decision 0157): a block whose top is pushed into bumps
    /// of different sizes, arrows up and down beside it.
    Displace,
    /// Weighted normals (decision 0148): a bevelled box seen from the
    /// side, the arrows on its flat top all straight up, those on its
    /// rounded corners fanning round.
    WeightedNormals,
    /// The game bevel (decision 0156): a block with its top corners
    /// rounded, its normals straight up from the flat top, and a tick
    /// beside it: ready for a game.
    GameBevel,
    /// Quadify (decision 0150): a square whose diagonal, the edge between
    /// its two triangles, is going (dashed).
    Quadify,
    /// Weld by distance (decision 0152): two pieces' edges side by side,
    /// their points pulled together onto one.
    WeldByDistance,
    /// Array (decision 0153): three boxes in a row.
    Array,
    /// Shell (decision 0154): a thin curved sheet given thickness, its
    /// rim showing.
    Shell,
    /// Creases (decision 0143): a shape sharp along one edge.
    Creases,
    /// Crease edges: the sharp edge drawn bold.
    CreaseEdges,
    /// Uncrease: rounded all round, the old sharp edge dashed.
    Uncrease,
    /// Turn UVs left 90°: a tile with an arrow going anticlockwise.
    UvTurnLeft,
    /// Turn UVs right 90°: a tile with an arrow going clockwise.
    UvTurnRight,
    /// Flip UVs left to right: a shape and its mirror across a dashed line.
    UvFlipU,
    /// Flip UVs upside down: a shape and its mirror across a dashed line.
    UvFlipV,
    /// Mark seams: a line stitched across.
    MarkSeam,
    /// Clear seams: a dashed line with its stitches gone.
    ClearSeam,
    /// Unwrap: a box opened out flat.
    Unwrap,
    /// Relax: a tangled grid smoothed out.
    Relax,
    /// Straighten: a wobbly line made straight.
    Straighten,
    /// Pack: pieces fitted into a square.
    Pack,
    /// Pin UVs: a pin pushed into a point.
    UvPin,
    /// Unpin UVs: the pin pulled out, dashed.
    UvUnpin,
    /// The turn and flip group: a tile half turned and half mirrored.
    UvTurnFlip,
    /// The stitch and cut group: two edges, one joined, one split.
    UvStitchCut,
    /// Stitch UVs: two edges drawn together.
    UvStitch,
    /// Cut UVs: one edge split in two.
    UvCut,
    /// Draw curve: a curve pulled out by its handle, with a pen's nib.
    DrawCurve,
    /// A ready-made line: two points joined.
    CurveLine,
    /// A ready-made circle: a ring through four points.
    CurveCircle,
    /// A ready-made arc: a quarter circle round its corner.
    CurveArc,
    /// A ready-made spiral: a spring climbing.
    CurveSpiral,
    /// Split curve: a curve with a gap cut in it.
    CurveSplit,
    /// Reverse curve: a curve with an arrow turned back along it.
    CurveReverse,
    /// Open or close curve: a loop with a dashed gap.
    CurveOpenClose,
    /// The handle type group: a point with its handle across it.
    CurveHandles,
    /// Sharp points: a corner with no handles.
    CurveSharp,
    /// Smooth points: a bend with its handles in one line.
    CurveSmooth,
    /// Broken handles: a kink with two handles at an angle.
    CurveBroken,
    /// Auto handles: a bend through three points, handles dashed.
    CurveAuto,
    /// Round corners: a corner, dashed, cut by an arc.
    CurveRound,
    /// Even spacing: points evenly along a curve.
    CurveEven,
}

/// Every icon, for tests and previews.
pub const ALL: [Icon; 130] = [
    Icon::Undo,
    Icon::Redo,
    Icon::PaintSelect,
    Icon::BoxSelection,
    Icon::LassoSelection,
    Icon::VisibleOnly,
    Icon::Move,
    Icon::Scale,
    Icon::Rotate,
    Icon::World,
    Icon::Cube,
    Icon::Cylinder,
    Icon::Plane,
    Icon::Sphere,
    Icon::Null,
    Icon::Snap,
    Icon::Symmetry,
    Icon::Search,
    Icon::ObjectMode,
    Icon::Points,
    Icon::Edges,
    Icon::Polygons,
    Icon::Uv,
    Icon::Workplane,
    Icon::Solo,
    Icon::GridDetail,
    Icon::Dolly,
    Icon::Maximise,
    Icon::Menu,
    Icon::Mouse,
    Icon::Shown,
    Icon::Hidden,
    Icon::Disc,
    Icon::Tube,
    Icon::Cone,
    Icon::Icosphere,
    Icon::Torus,
    Icon::Capsule,
    Icon::Dome,
    Icon::Elbow,
    Icon::Sector,
    Icon::Gear,
    Icon::HexGrid,
    Icon::HexSphere,
    Icon::Stairs,
    Icon::Arch,
    Icon::AllGizmos,
    Icon::MakeEditable,
    Icon::Group,
    Icon::Ungroup,
    Icon::Delete,
    Icon::FrameSelected,
    Icon::Extrude,
    Icon::Inset,
    Icon::Bevel,
    Icon::Groove,
    Icon::Bridge,
    Icon::FillHole,
    Icon::FlipNormals,
    Icon::HardEdges,
    Icon::SoftEdges,
    Icon::Dissolve,
    Icon::Weld,
    Icon::Connect,
    Icon::Slit,
    Icon::LoopCut,
    Icon::NewGroup,
    Icon::ShapeList,
    Icon::Cancel,
    Icon::Duplicate,
    Icon::Instance,
    Icon::Join,
    Icon::PivotToCentre,
    Icon::PivotToOrigin,
    Icon::PivotToObject,
    Icon::ObjectToOrigin,
    Icon::ApplyTransforms,
    Icon::EditLayer,
    Icon::Union,
    Icon::Subtract,
    Icon::Intersect,
    Icon::Remesh,
    Icon::Subdivide,
    Icon::LiveBevel,
    Icon::Lattice,
    Icon::Decimate,
    Icon::Scatter,
    Icon::BendAlongCurve,
    Icon::Shrinkwrap,
    Icon::Displace,
    Icon::WeightedNormals,
    Icon::GameBevel,
    Icon::Quadify,
    Icon::WeldByDistance,
    Icon::Array,
    Icon::Shell,
    Icon::Creases,
    Icon::CreaseEdges,
    Icon::Uncrease,
    Icon::UvTurnLeft,
    Icon::UvTurnRight,
    Icon::UvFlipU,
    Icon::UvFlipV,
    Icon::MarkSeam,
    Icon::ClearSeam,
    Icon::Unwrap,
    Icon::Relax,
    Icon::Straighten,
    Icon::Pack,
    Icon::UvPin,
    Icon::UvUnpin,
    Icon::UvTurnFlip,
    Icon::UvStitchCut,
    Icon::UvStitch,
    Icon::UvCut,
    Icon::DrawCurve,
    Icon::CurveLine,
    Icon::CurveCircle,
    Icon::CurveArc,
    Icon::CurveSpiral,
    Icon::CurveSplit,
    Icon::CurveReverse,
    Icon::CurveOpenClose,
    Icon::CurveHandles,
    Icon::CurveSharp,
    Icon::CurveSmooth,
    Icon::CurveBroken,
    Icon::CurveAuto,
    Icon::CurveRound,
    Icon::CurveEven,
];

/// Draws `icon` into `rect` in `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    painter.extend(shapes(rect, icon, color));
}

/// The shapes that make up `icon`, scaled into `rect`.
pub fn shapes(rect: Rect, icon: Icon, color: Color32) -> Vec<Shape> {
    let mut pen = Pen::new(rect, color);
    match icon {
        Icon::Undo
        | Icon::Redo
        | Icon::PaintSelect
        | Icon::BoxSelection
        | Icon::LassoSelection
        | Icon::VisibleOnly
        | Icon::World
        | Icon::Null
        | Icon::Snap
        | Icon::Symmetry
        | Icon::Search
        | Icon::ObjectMode
        | Icon::Points
        | Icon::Edges
        | Icon::Polygons
        | Icon::Uv
        | Icon::Workplane
        | Icon::Solo
        | Icon::GridDetail
        | Icon::Dolly
        | Icon::Maximise
        | Icon::Menu
        | Icon::Mouse
        | Icon::Shown
        | Icon::Hidden => general::draw(&mut pen, icon),
        Icon::Cube
        | Icon::Cylinder
        | Icon::Plane
        | Icon::Sphere
        | Icon::Disc
        | Icon::Tube
        | Icon::Cone
        | Icon::Icosphere
        | Icon::Torus
        | Icon::Capsule
        | Icon::Dome
        | Icon::Elbow
        | Icon::Sector
        | Icon::Gear
        | Icon::HexGrid
        | Icon::HexSphere
        | Icon::Stairs
        | Icon::Arch => primitives::draw(&mut pen, icon),
        Icon::Move
        | Icon::Rotate
        | Icon::Scale
        | Icon::AllGizmos
        | Icon::FrameSelected
        | Icon::MakeEditable
        | Icon::Delete
        | Icon::Ungroup
        | Icon::Group
        | Icon::NewGroup
        | Icon::ShapeList
        | Icon::PivotToCentre
        | Icon::PivotToOrigin
        | Icon::PivotToObject
        | Icon::ObjectToOrigin => tools::draw(&mut pen, icon),
        Icon::Extrude
        | Icon::Inset
        | Icon::Bevel
        | Icon::Groove
        | Icon::Bridge
        | Icon::FillHole
        | Icon::FlipNormals
        | Icon::HardEdges
        | Icon::SoftEdges
        | Icon::Dissolve
        | Icon::Weld
        | Icon::Connect
        | Icon::Slit
        | Icon::LoopCut
        | Icon::Cancel
        | Icon::Duplicate
        | Icon::Instance
        | Icon::Join
        | Icon::ApplyTransforms
        | Icon::EditLayer => modelling::draw(&mut pen, icon),
        Icon::Union | Icon::Subtract | Icon::Intersect => booleans::draw(&mut pen, icon),
        Icon::Remesh
        | Icon::Subdivide
        | Icon::LiveBevel
        | Icon::Lattice
        | Icon::Decimate
        | Icon::Scatter
        | Icon::BendAlongCurve
        | Icon::Shrinkwrap
        | Icon::Displace
        | Icon::WeightedNormals
        | Icon::GameBevel
        | Icon::Quadify
        | Icon::WeldByDistance
        | Icon::Array
        | Icon::Shell
        | Icon::Creases
        | Icon::CreaseEdges
        | Icon::Uncrease => modifiers::draw(&mut pen, icon),
        Icon::DrawCurve
        | Icon::CurveLine
        | Icon::CurveCircle
        | Icon::CurveArc
        | Icon::CurveSpiral => curves::draw(&mut pen, icon),
        Icon::CurveSplit
        | Icon::CurveReverse
        | Icon::CurveOpenClose
        | Icon::CurveHandles
        | Icon::CurveSharp
        | Icon::CurveSmooth
        | Icon::CurveBroken
        | Icon::CurveAuto
        | Icon::CurveRound
        | Icon::CurveEven => curves::draw_editing(&mut pen, icon),
        Icon::UvTurnLeft
        | Icon::UvTurnRight
        | Icon::UvFlipU
        | Icon::UvFlipV
        | Icon::MarkSeam
        | Icon::ClearSeam
        | Icon::Unwrap
        | Icon::Relax
        | Icon::Straighten
        | Icon::Pack
        | Icon::UvPin
        | Icon::UvUnpin
        | Icon::UvTurnFlip
        | Icon::UvStitch
        | Icon::UvStitchCut
        | Icon::UvCut => uv::draw(&mut pen, icon),
    }
    pen.shapes
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn every_icon_draws_inside_its_box() {
        let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(24.0, 24.0));
        for icon in ALL {
            let drawn = shapes(rect, icon, Color32::WHITE);
            assert!(!drawn.is_empty(), "{icon:?} draws nothing");
            for shape in &drawn {
                // Allow for half a stroke width over the edge.
                assert!(
                    rect.expand(2.0).contains_rect(shape.visual_bounding_rect()),
                    "{icon:?} draws outside its box"
                );
            }
        }
    }

    #[test]
    fn icons_scale_with_their_box() {
        let small = Rect::from_min_size(pos2(0.0, 0.0), vec2(12.0, 12.0));
        let large = Rect::from_min_size(pos2(0.0, 0.0), vec2(48.0, 48.0));
        let width = |rect| {
            shapes(rect, Icon::Cube, Color32::WHITE)
                .iter()
                .map(Shape::visual_bounding_rect)
                .fold(Rect::NOTHING, |a, b| a.union(b))
                .width()
        };
        assert!(width(large) > 3.0 * width(small));
    }

    #[test]
    fn a_non_square_box_keeps_icons_square_and_centred() {
        let wide = Rect::from_min_size(pos2(0.0, 0.0), vec2(48.0, 24.0));
        let bounds = shapes(wide, Icon::Polygons, Color32::WHITE)
            .iter()
            .map(Shape::visual_bounding_rect)
            .fold(Rect::NOTHING, |a, b| a.union(b));
        assert!((bounds.center().x - 24.0).abs() < 0.5);
        assert!((bounds.width() - bounds.height()).abs() < 0.5);
    }
}
