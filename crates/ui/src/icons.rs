//! Small vector icons for the palettes, drawn with egui's shapes.
//!
//! Drawn in code rather than shipped as images: nothing to license, sharp
//! at any interface scale, and coloured by the theme. Each icon is
//! designed on a 24 × 24 grid and scaled into the rectangle it is given.
//! These are original drawings; no icons are copied from other software.

use egui::epaint::PathStroke;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, vec2};
use std::f32::consts::{PI, TAU};

/// Every icon the palettes use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// Edit → Undo.
    Undo,
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
    /// Knife: a blade cutting a line.
    Knife,
    /// Loop cut: a new loop around a shape.
    LoopCut,
    /// New group: a dashed outline with a plus.
    NewGroup,
    /// Create a shape (the shape list): a cube with a plus.
    ShapeList,
    /// Cancel, in the middle of the tool menu: a circle with a line.
    Cancel,
}

/// Every icon, for tests and previews.
pub const ALL: [Icon; 57] = [
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
    Icon::AllGizmos,
    Icon::MakeEditable,
    Icon::Group,
    Icon::Ungroup,
    Icon::Delete,
    Icon::FrameSelected,
    Icon::Extrude,
    Icon::Inset,
    Icon::Bevel,
    Icon::Bridge,
    Icon::FillHole,
    Icon::FlipNormals,
    Icon::HardEdges,
    Icon::SoftEdges,
    Icon::Dissolve,
    Icon::Weld,
    Icon::Connect,
    Icon::Knife,
    Icon::LoopCut,
    Icon::NewGroup,
    Icon::ShapeList,
    Icon::Cancel,
];

/// Draws `icon` into `rect` in `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    painter.extend(shapes(rect, icon, color));
}

/// How a line is drawn. Every line has the same width; they differ only
/// in tone, dashes and colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ink {
    /// The icon's colour.
    Line,
    /// The icon's colour, faint: a part in the background.
    Faint,
    /// Dashed: something about to be, or no longer, there.
    Dashed,
    /// The X axis's red.
    AxisX,
    /// The Y axis's blue.
    AxisY,
    /// The Z axis's green.
    AxisZ,
}

/// How a shape is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paint {
    /// The icon's colour.
    Solid,
    /// A light tone of it: the part a tool changes.
    Tone,
    /// A stronger tone: the darker side of a flat-shaded shape.
    Shade,
    /// Smooth shading, faint on the left to strong on the right.
    Shading,
    /// The X axis's red.
    AxisX,
    /// The Z axis's green.
    AxisZ,
}

/// Collects shapes on the 24 × 24 design grid, scaled into a rectangle.
struct Pen {
    origin: Pos2,
    scale: f32,
    color: Color32,
    shapes: Vec<Shape>,
}

impl Pen {
    fn new(rect: Rect, color: Color32) -> Self {
        let scale = rect.width().min(rect.height()) / 24.0;
        Self {
            origin: rect.center() - vec2(12.0 * scale, 12.0 * scale),
            scale,
            color,
            shapes: Vec::new(),
        }
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x * self.scale, y * self.scale)
    }

    fn width(&self) -> f32 {
        1.6 * self.scale
    }

    fn points(&self, list: &[(f32, f32)]) -> Vec<Pos2> {
        list.iter().map(|&(x, y)| self.p(x, y)).collect()
    }

    /// Points along an ellipse arc. Angles in radians: 0 points right, and
    /// angles grow clockwise on screen because y points down.
    fn arc(&self, centre: (f32, f32), radii: (f32, f32), from: f32, to: f32) -> Vec<Pos2> {
        let steps = 24;
        (0..=steps)
            .map(|i| {
                let t = from + (to - from) * i as f32 / steps as f32;
                self.p(centre.0 + radii.0 * t.cos(), centre.1 + radii.1 * t.sin())
            })
            .collect()
    }

    fn line(&mut self, points: Vec<Pos2>) {
        let stroke = PathStroke::new(self.width(), self.color);
        self.shapes.push(Shape::line(points, stroke));
    }

    fn polyline(&mut self, list: &[(f32, f32)]) {
        let points = self.points(list);
        self.line(points);
    }

    fn closed(&mut self, points: Vec<Pos2>) {
        let stroke = PathStroke::new(self.width(), self.color);
        self.shapes.push(Shape::closed_line(points, stroke));
    }

    fn polygon(&mut self, list: &[(f32, f32)]) {
        let points = self.points(list);
        self.closed(points);
    }

    fn ellipse(&mut self, centre: (f32, f32), radii: (f32, f32)) {
        let points = self.arc(centre, radii, 0.0, TAU);
        self.closed(points);
    }

    fn dot(&mut self, x: f32, y: f32, radius: f32) {
        let centre = self.p(x, y);
        self.shapes.push(Shape::circle_filled(
            centre,
            radius * self.scale,
            self.color,
        ));
    }

    fn square(&mut self, min: (f32, f32), max: (f32, f32), faint: bool) {
        let rect = Rect::from_min_max(self.p(min.0, min.1), self.p(max.0, max.1));
        let color = if faint {
            self.color.gamma_multiply(0.45)
        } else {
            self.color
        };
        self.shapes.push(Shape::rect_stroke(
            rect,
            0.0,
            Stroke::new(self.width(), color),
            StrokeKind::Middle,
        ));
    }

    fn fill(&mut self, min: (f32, f32), max: (f32, f32), strength: f32) {
        let rect = Rect::from_min_max(self.p(min.0, min.1), self.p(max.0, max.1));
        let color = self.color.gamma_multiply(strength);
        self.shapes.push(Shape::rect_filled(rect, 0.0, color));
    }

    /// A line through `list`, closed back to its start if `closed`.
    fn stroke(&mut self, list: &[(f32, f32)], closed: bool, ink: Ink) {
        let color = match ink {
            Ink::Line => self.color,
            Ink::Faint => self.color.gamma_multiply(0.4),
            Ink::Dashed => self.color.gamma_multiply(0.75),
            Ink::AxisX => crate::theme::tint::AXIS_X,
            Ink::AxisY => crate::theme::tint::AXIS_Y,
            Ink::AxisZ => crate::theme::tint::AXIS_Z,
        };
        let mut points = self.points(list);
        if ink == Ink::Dashed {
            if closed && let Some(&first) = points.first() {
                points.push(first);
            }
            let stroke = Stroke::new(self.width(), color);
            let (dash, gap) = (1.6 * self.scale, 2.6 * self.scale);
            self.shapes
                .extend(Shape::dashed_line(&points, stroke, dash, gap));
            return;
        }
        let stroke = PathStroke::new(self.width(), color);
        self.shapes.push(if closed {
            Shape::closed_line(points, stroke)
        } else {
            Shape::line(points, stroke)
        });
    }

    /// A filled convex shape through `list`.
    fn shape(&mut self, list: &[(f32, f32)], paint: Paint) {
        let points = self.points(list);
        let flat = |color: Color32| Shape::convex_polygon(points.clone(), color, Stroke::NONE);
        let shape = match paint {
            Paint::Solid => flat(self.color),
            Paint::Tone => flat(self.color.gamma_multiply(0.3)),
            Paint::Shade => flat(self.color.gamma_multiply(0.62)),
            Paint::AxisX => flat(crate::theme::tint::AXIS_X),
            Paint::AxisZ => flat(crate::theme::tint::AXIS_Z),
            Paint::Shading => {
                // A fan of triangles whose corners fade from left to right.
                let (left, right) = points
                    .iter()
                    .fold((f32::MAX, f32::MIN), |(l, r), p| (l.min(p.x), r.max(p.x)));
                let alpha = |x: f32| {
                    let t = ((x - left) / (right - left).max(1e-3)).clamp(0.0, 1.0);
                    Color32::from_white_alpha((255.0 * (0.12 + 0.48 * t)) as u8)
                };
                let mut mesh = egui::Mesh::default();
                for p in &points {
                    mesh.colored_vertex(*p, alpha(p.x));
                }
                for i in 1..points.len().saturating_sub(1) as u32 {
                    mesh.add_triangle(0, i, i + 1);
                }
                Shape::mesh(mesh)
            }
        };
        self.shapes.push(shape);
    }

    /// A filled circle.
    fn disc(&mut self, centre: (f32, f32), radius: f32, paint: Paint) {
        let color = match paint {
            Paint::Tone => self.color.gamma_multiply(0.3),
            _ => self.color,
        };
        let centre = self.p(centre.0, centre.1);
        self.shapes
            .push(Shape::circle_filled(centre, radius * self.scale, color));
    }

    /// A circle or ellipse's outline.
    fn ring(&mut self, centre: (f32, f32), radii: (f32, f32), ink: Ink) {
        let list: Vec<(f32, f32)> = (0..48)
            .map(|i| {
                let a = i as f32 / 48.0 * TAU;
                (centre.0 + radii.0 * a.cos(), centre.1 + radii.1 * a.sin())
            })
            .collect();
        self.stroke(&list, true, ink);
    }

    fn segment(&mut self, from: (f32, f32), to: (f32, f32), width: f32, strength: f32) {
        let stroke = Stroke::new(self.width() * width, self.color.gamma_multiply(strength));
        let points = [self.p(from.0, from.1), self.p(to.0, to.1)];
        self.shapes.push(Shape::line_segment(points, stroke));
    }
}

/// The shapes that make up `icon`, scaled into `rect`.
pub fn shapes(rect: Rect, icon: Icon, color: Color32) -> Vec<Shape> {
    let mut pen = Pen::new(rect, color);
    match icon {
        Icon::Undo => {
            pen.polyline(&[(9.0, 14.0), (4.0, 9.0), (9.0, 4.0)]);
            let mut curve = pen.points(&[(4.0, 9.0)]);
            curve.extend(pen.arc((14.5, 14.5), (5.5, 5.5), -PI / 2.0, PI / 2.0));
            curve.push(pen.p(11.0, 20.0));
            pen.line(curve);
        }
        Icon::Redo => {
            pen.polyline(&[(15.0, 14.0), (20.0, 9.0), (15.0, 4.0)]);
            let mut curve = pen.points(&[(20.0, 9.0)]);
            curve.extend(pen.arc((9.5, 14.5), (5.5, 5.5), -PI / 2.0, -1.5 * PI));
            curve.push(pen.p(13.0, 20.0));
            pen.line(curve);
        }
        Icon::PaintSelect => {
            // A dashed circle with a plus in it.
            for i in 0..12 {
                let a = i as f32 / 12.0 * TAU;
                let dash = pen.arc((12.0, 12.0), (8.0, 8.0), a, a + TAU / 24.0);
                pen.line(dash);
            }
            pen.polyline(&[(12.0, 9.0), (12.0, 15.0)]);
            pen.polyline(&[(9.0, 12.0), (15.0, 12.0)]);
        }
        Icon::BoxSelection => {
            // A dashed square with a pointer at its corner.
            for i in 0..4 {
                let at = 4.0 + i as f32 * 4.0;
                pen.segment((at, 4.0), (at + 2.5, 4.0), 1.5, 1.0);
                pen.segment((4.0, at), (4.0, at + 2.5), 1.5, 1.0);
                pen.segment((at, 18.0), (at + 2.0, 18.0), 1.5, 1.0);
                pen.segment((18.0, at), (18.0, at + 2.0), 1.5, 1.0);
            }
            pen.polygon(&[(14.0, 14.0), (21.0, 17.5), (17.5, 18.0), (18.5, 21.5)]);
        }
        Icon::LassoSelection => {
            // A loose dashed loop with a tail.
            for i in 0..10 {
                let a = i as f32 / 10.0 * TAU;
                let dash = pen.arc((12.0, 10.0), (9.0, 6.5), a, a + TAU / 20.0);
                pen.line(dash);
            }
            pen.polyline(&[(7.0, 15.0), (6.0, 18.5), (8.5, 21.0)]);
        }
        Icon::VisibleOnly => {
            // A box seen from the front, right and above: the edges in
            // sight solid, the hidden ones barely there.
            pen.polygon(&[(4.0, 9.0), (15.0, 9.0), (15.0, 20.0), (4.0, 20.0)]);
            pen.polyline(&[(4.0, 9.0), (9.0, 4.0), (20.0, 4.0), (15.0, 9.0)]);
            pen.polyline(&[(20.0, 4.0), (20.0, 15.0), (15.0, 20.0)]);
            pen.segment((9.0, 4.0), (9.0, 15.0), 1.0, 0.3);
            pen.segment((9.0, 15.0), (20.0, 15.0), 1.0, 0.3);
            pen.segment((9.0, 15.0), (4.0, 20.0), 1.0, 0.3);
        }
        Icon::World => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.ellipse((12.0, 12.0), (3.8, 9.0));
        }
        Icon::Cube => {
            pen.polygon(&[
                (12.0, 2.5),
                (20.5, 7.0),
                (20.5, 17.0),
                (12.0, 21.5),
                (3.5, 17.0),
                (3.5, 7.0),
            ]);
            pen.polyline(&[(3.5, 7.0), (12.0, 11.5), (20.5, 7.0)]);
            pen.polyline(&[(12.0, 11.5), (12.0, 21.5)]);
        }
        Icon::Cylinder => {
            pen.ellipse((12.0, 5.5), (8.0, 3.0));
            pen.polyline(&[(4.0, 5.5), (4.0, 18.5)]);
            pen.polyline(&[(20.0, 5.5), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
        }
        Icon::Plane => {
            pen.polygon(&[(2.0, 17.0), (8.0, 8.0), (22.0, 8.0), (16.0, 17.0)]);
        }
        Icon::Sphere => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            pen.ellipse((12.0, 12.0), (9.0, 3.5));
        }
        Icon::Disc => {
            pen.ellipse((12.0, 12.0), (9.0, 4.5));
            pen.dot(12.0, 12.0, 1.2);
        }
        Icon::Tube => {
            pen.ellipse((12.0, 5.5), (8.0, 3.0));
            pen.ellipse((12.0, 5.5), (4.0, 1.5));
            pen.polyline(&[(4.0, 5.5), (4.0, 18.5)]);
            pen.polyline(&[(20.0, 5.5), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
        }
        Icon::Cone => {
            pen.polyline(&[(4.0, 18.5), (12.0, 3.0), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
            let back = pen.arc((12.0, 18.5), (8.0, 3.0), PI, TAU);
            pen.line(back);
        }
        Icon::Null => {
            pen.polyline(&[(12.0, 3.0), (12.0, 21.0)]);
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.ellipse((12.0, 12.0), (3.0, 3.0));
        }
        Icon::Snap => {
            // A horseshoe magnet: down the left side, round the bottom
            // (from the left, through the bottom, to the right), and up.
            let mut magnet = pen.points(&[(6.0, 3.0), (6.0, 11.0)]);
            magnet.extend(pen.arc((12.0, 11.0), (6.0, 6.0), PI, 0.0));
            magnet.push(pen.p(18.0, 3.0));
            pen.line(magnet);
            pen.polyline(&[(6.0, 7.0), (10.0, 7.0)]);
            pen.polyline(&[(14.0, 7.0), (18.0, 7.0)]);
        }
        Icon::Symmetry => {
            for i in 0..6 {
                let y = 3.0 + i as f32 * 3.0;
                pen.polyline(&[(12.0, y), (12.0, y + 1.5)]);
            }
            pen.polygon(&[(9.0, 6.0), (3.0, 12.0), (9.0, 18.0)]);
            pen.polygon(&[(15.0, 6.0), (21.0, 12.0), (15.0, 18.0)]);
        }
        Icon::Search => {
            pen.ellipse((11.0, 11.0), (7.0, 7.0));
            pen.polyline(&[(16.0, 16.0), (20.5, 20.5)]);
        }
        Icon::ObjectMode => {
            pen.polyline(&[(12.0, 12.0), (12.0, 3.0)]);
            pen.polyline(&[(12.0, 12.0), (20.0, 17.0)]);
            pen.polyline(&[(12.0, 12.0), (4.0, 17.0)]);
            pen.dot(12.0, 12.0, 2.2);
        }
        Icon::Points => {
            pen.square((5.0, 5.0), (19.0, 19.0), true);
            for &(x, y) in &[(5.0, 5.0), (19.0, 5.0), (5.0, 19.0), (19.0, 19.0)] {
                pen.dot(x, y, 2.4);
            }
        }
        Icon::Edges => {
            pen.square((5.0, 5.0), (19.0, 19.0), true);
            pen.segment((5.0, 19.0), (19.0, 19.0), 2.0, 1.0);
        }
        Icon::Polygons => {
            pen.fill((5.0, 5.0), (19.0, 19.0), 0.6);
        }
        Icon::Uv => {
            pen.square((4.0, 4.0), (20.0, 20.0), false);
            pen.segment((4.0, 12.0), (20.0, 12.0), 1.0, 0.5);
            pen.segment((12.0, 4.0), (12.0, 20.0), 1.0, 0.5);
            pen.fill((6.5, 6.5), (10.0, 10.0), 1.0);
        }
        Icon::Workplane => {
            pen.polygon(&[(3.0, 16.0), (8.0, 8.0), (21.0, 8.0), (16.0, 16.0)]);
            pen.segment((10.0, 12.0), (18.0, 12.0), 1.0, 0.5);
        }
        Icon::Solo => {
            pen.ellipse((12.0, 12.0), (10.0, 6.0));
            pen.ellipse((12.0, 12.0), (3.0, 3.0));
        }
        Icon::GridDetail => {
            // A square of ground with strong 1 m lines and faint fine ones.
            pen.square((4.0, 4.0), (20.0, 20.0), false);
            pen.segment((12.0, 4.0), (12.0, 20.0), 1.0, 1.0);
            pen.segment((4.0, 12.0), (20.0, 12.0), 1.0, 1.0);
            for at in [8.0, 16.0] {
                pen.segment((at, 4.0), (at, 20.0), 0.6, 0.5);
                pen.segment((4.0, at), (20.0, at), 0.6, 0.5);
            }
        }
        Icon::Dolly => {
            pen.polyline(&[(12.0, 3.5), (12.0, 20.5)]);
            pen.polyline(&[(8.0, 7.5), (12.0, 3.5), (16.0, 7.5)]);
            pen.polyline(&[(8.0, 16.5), (12.0, 20.5), (16.0, 16.5)]);
        }
        Icon::Maximise => {
            pen.square((4.5, 4.5), (19.5, 19.5), false);
        }
        Icon::Menu => {
            for y in [7.0, 12.0, 17.0] {
                pen.polyline(&[(4.0, y), (20.0, y)]);
            }
        }
        Icon::Mouse => {
            let mut body = pen.arc((12.0, 9.0), (6.0, 6.0), PI, TAU);
            body.extend(pen.arc((12.0, 15.0), (6.0, 6.0), 0.0, PI));
            pen.closed(body);
            pen.polyline(&[(12.0, 3.0), (12.0, 9.0)]);
        }
        Icon::Shown => {
            // An almond-shaped eye with its pupil.
            // Two arcs meeting at the corners (4, 12) and (20, 12).
            let mut eye = pen.arc((12.0, 15.9), (8.9, 8.9), -2.688, -0.453);
            eye.extend(pen.arc((12.0, 8.1), (8.9, 8.9), 0.453, 2.688));
            pen.closed(eye);
            pen.dot(12.0, 12.0, 2.6);
        }
        // The tools' icons below were picked by the project lead in the icon
        // mockup (2026-10-12) and converted from it point for point.
        Icon::Move => {
            pen.stroke(&[(12.0, 3.0), (12.0, 21.0)], false, Ink::Line);
            pen.stroke(&[(3.0, 12.0), (21.0, 12.0)], false, Ink::Line);
            pen.stroke(&[(9.0, 6.0), (12.0, 3.0), (15.0, 6.0)], false, Ink::Line);
            pen.stroke(&[(9.0, 18.0), (12.0, 21.0), (15.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(6.0, 9.0), (3.0, 12.0), (6.0, 15.0)], false, Ink::Line);
            pen.stroke(&[(18.0, 9.0), (21.0, 12.0), (18.0, 15.0)], false, Ink::Line);
        }
        Icon::Rotate => {
            pen.stroke(
                &[
                    (4.48, 9.26),
                    (5.2, 7.78),
                    (6.21, 6.48),
                    (7.46, 5.41),
                    (8.9, 4.62),
                    (10.48, 4.14),
                    (12.12, 4.0),
                    (13.75, 4.19),
                    (15.31, 4.71),
                    (16.73, 5.55),
                    (17.95, 6.65),
                ],
                false,
                Ink::Line,
            );
            pen.shape(&[(19.53, 9.04), (15.59, 7.41), (20.09, 4.81)], Paint::Solid);
            pen.stroke(
                &[
                    (19.52, 14.74),
                    (18.8, 16.22),
                    (17.79, 17.52),
                    (16.54, 18.59),
                    (15.1, 19.38),
                    (13.52, 19.86),
                    (11.88, 20.0),
                    (10.25, 19.81),
                    (8.69, 19.29),
                    (7.27, 18.45),
                    (6.05, 17.35),
                ],
                false,
                Ink::Line,
            );
            pen.shape(&[(4.47, 14.96), (8.41, 16.59), (3.91, 19.19)], Paint::Solid);
        }
        Icon::Scale => {
            pen.stroke(
                &[(3.0, 12.0), (12.0, 12.0), (12.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Dashed,
            );
            pen.stroke(&[(12.0, 12.0), (20.0, 4.0)], false, Ink::Line);
            pen.stroke(&[(15.0, 4.0), (20.0, 4.0), (20.0, 9.0)], false, Ink::Line);
        }
        Icon::AllGizmos => {
            pen.stroke(&[(9.0, 15.0), (9.0, 5.5)], false, Ink::AxisZ);
            pen.shape(&[(9.0, 2.5), (11.3, 5.49), (6.7, 5.49)], Paint::AxisZ);
            pen.stroke(&[(9.0, 15.0), (17.5, 15.0)], false, Ink::AxisX);
            pen.shape(
                &[(17.5, 13.0), (21.5, 13.0), (21.5, 17.0), (17.5, 17.0)],
                Paint::AxisX,
            );
            pen.stroke(
                &[
                    (10.22, 8.11),
                    (11.59, 8.5),
                    (12.85, 9.16),
                    (13.95, 10.05),
                    (14.84, 11.15),
                    (15.5, 12.41),
                    (15.89, 13.78),
                ],
                false,
                Ink::AxisY,
            );
            pen.disc((9.0, 15.0), 1.6, Paint::Solid);
        }
        Icon::FrameSelected => {
            pen.stroke(&[(3.0, 8.0), (3.0, 3.0), (8.0, 3.0)], false, Ink::Line);
            pen.stroke(&[(16.0, 3.0), (21.0, 3.0), (21.0, 8.0)], false, Ink::Line);
            pen.stroke(
                &[(21.0, 16.0), (21.0, 21.0), (16.0, 21.0)],
                false,
                Ink::Line,
            );
            pen.stroke(&[(8.0, 21.0), (3.0, 21.0), (3.0, 16.0)], false, Ink::Line);
            pen.shape(
                &[(8.0, 8.0), (16.0, 8.0), (16.0, 16.0), (8.0, 16.0)],
                Paint::Tone,
            );
            pen.stroke(
                &[(8.0, 8.0), (16.0, 8.0), (16.0, 16.0), (8.0, 16.0)],
                true,
                Ink::Line,
            );
        }
        Icon::MakeEditable => {
            pen.stroke(
                &[
                    (12.0, 3.0),
                    (20.0, 7.5),
                    (20.0, 16.5),
                    (12.0, 21.0),
                    (4.0, 16.5),
                    (4.0, 7.5),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 7.5), (12.0, 12.0), (20.0, 7.5)], false, Ink::Line);
            pen.stroke(&[(12.0, 12.0), (12.0, 21.0)], false, Ink::Line);
            pen.disc((12.0, 3.0), 1.7, Paint::Solid);
            pen.disc((20.0, 7.5), 1.7, Paint::Solid);
            pen.disc((20.0, 16.5), 1.7, Paint::Solid);
            pen.disc((12.0, 21.0), 1.7, Paint::Solid);
            pen.disc((4.0, 16.5), 1.7, Paint::Solid);
            pen.disc((4.0, 7.5), 1.7, Paint::Solid);
            pen.disc((12.0, 12.0), 1.7, Paint::Solid);
        }
        Icon::Delete => {
            pen.stroke(
                &[(6.5, 9.5), (17.5, 9.5), (16.4, 21.0), (7.6, 21.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(10.5, 13.0), (10.5, 18.0)], false, Ink::Line);
            pen.stroke(&[(13.5, 13.0), (13.5, 18.0)], false, Ink::Line);
            pen.stroke(&[(4.21, 8.69), (17.79, 5.31)], false, Ink::Line);
            pen.stroke(
                &[(9.54, 7.36), (8.94, 4.94), (12.82, 3.97), (13.43, 6.4)],
                false,
                Ink::Line,
            );
        }
        Icon::Ungroup => {
            pen.stroke(
                &[
                    (15.0, 7.0),
                    (15.52, 7.07),
                    (16.0, 7.27),
                    (16.41, 7.59),
                    (16.73, 8.0),
                    (16.93, 8.48),
                    (17.0, 9.0),
                    (17.0, 15.0),
                    (16.93, 15.52),
                    (16.73, 16.0),
                    (16.41, 16.41),
                    (16.0, 16.73),
                    (15.52, 16.93),
                    (15.0, 17.0),
                    (9.0, 17.0),
                    (8.48, 16.93),
                    (8.0, 16.73),
                    (7.59, 16.41),
                    (7.27, 16.0),
                    (7.07, 15.52),
                    (7.0, 15.0),
                    (7.0, 9.0),
                    (7.07, 8.48),
                    (7.27, 8.0),
                    (7.59, 7.59),
                    (8.0, 7.27),
                    (8.48, 7.07),
                    (9.0, 7.0),
                ],
                true,
                Ink::Dashed,
            );
            pen.stroke(
                &[(2.0, 2.0), (7.0, 2.0), (7.0, 7.0), (2.0, 7.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(17.0, 17.0), (22.0, 17.0), (22.0, 22.0), (17.0, 22.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(7.5, 7.5), (7.0, 7.0)], false, Ink::Faint);
        }
        Icon::Group => {
            pen.stroke(
                &[(6.0, 6.0), (11.0, 6.0), (11.0, 11.0), (6.0, 11.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(13.0, 13.0), (18.0, 13.0), (18.0, 18.0), (13.0, 18.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[
                    (18.0, 3.0),
                    (18.78, 3.1),
                    (19.5, 3.4),
                    (20.12, 3.88),
                    (20.6, 4.5),
                    (20.9, 5.22),
                    (21.0, 6.0),
                    (21.0, 18.0),
                    (20.9, 18.78),
                    (20.6, 19.5),
                    (20.12, 20.12),
                    (19.5, 20.6),
                    (18.78, 20.9),
                    (18.0, 21.0),
                    (6.0, 21.0),
                    (5.22, 20.9),
                    (4.5, 20.6),
                    (3.88, 20.12),
                    (3.4, 19.5),
                    (3.1, 18.78),
                    (3.0, 18.0),
                    (3.0, 6.0),
                    (3.1, 5.22),
                    (3.4, 4.5),
                    (3.88, 3.88),
                    (4.5, 3.4),
                    (5.22, 3.1),
                    (6.0, 3.0),
                ],
                true,
                Ink::Dashed,
            );
        }
        Icon::NewGroup => {
            pen.stroke(
                &[
                    (17.0, 4.0),
                    (17.78, 4.1),
                    (18.5, 4.4),
                    (19.12, 4.88),
                    (19.6, 5.5),
                    (19.9, 6.22),
                    (20.0, 7.0),
                    (20.0, 17.0),
                    (19.9, 17.78),
                    (19.6, 18.5),
                    (19.12, 19.12),
                    (18.5, 19.6),
                    (17.78, 19.9),
                    (17.0, 20.0),
                    (7.0, 20.0),
                    (6.22, 19.9),
                    (5.5, 19.6),
                    (4.88, 19.12),
                    (4.4, 18.5),
                    (4.1, 17.78),
                    (4.0, 17.0),
                    (4.0, 7.0),
                    (4.1, 6.22),
                    (4.4, 5.5),
                    (4.88, 4.88),
                    (5.5, 4.4),
                    (6.22, 4.1),
                    (7.0, 4.0),
                ],
                true,
                Ink::Dashed,
            );
            pen.stroke(&[(12.0, 8.5), (12.0, 15.5)], false, Ink::Line);
            pen.stroke(&[(8.5, 12.0), (15.5, 12.0)], false, Ink::Line);
        }
        Icon::ShapeList => {
            pen.stroke(
                &[
                    (10.0, 3.5),
                    (17.0, 7.2),
                    (17.0, 15.0),
                    (10.0, 18.7),
                    (3.0, 15.0),
                    (3.0, 7.2),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(3.0, 7.2), (10.0, 11.0), (17.0, 7.2)], false, Ink::Line);
            pen.stroke(&[(10.0, 11.0), (10.0, 18.7)], false, Ink::Line);
            pen.stroke(&[(19.0, 16.0), (19.0, 22.0)], false, Ink::Line);
            pen.stroke(&[(16.0, 19.0), (22.0, 19.0)], false, Ink::Line);
        }
        Icon::Extrude => {
            pen.shape(
                &[(3.0, 12.0), (14.0, 12.0), (21.0, 8.0), (10.0, 8.0)],
                Paint::Solid,
            );
            pen.stroke(&[(3.0, 13.0), (3.0, 21.0)], false, Ink::Line);
            pen.stroke(&[(14.0, 13.0), (14.0, 21.0)], false, Ink::Line);
            pen.stroke(&[(21.0, 9.0), (21.0, 17.0)], false, Ink::Line);
            pen.stroke(
                &[(3.0, 21.0), (14.0, 21.0), (21.0, 17.0)],
                false,
                Ink::Faint,
            );
            pen.stroke(&[(12.0, 6.5), (12.0, 3.0)], false, Ink::Line);
            pen.shape(&[(12.0, 0.5), (9.0, 4.1), (15.0, 4.1)], Paint::Solid);
        }
        Icon::Inset => {
            pen.stroke(
                &[(2.5, 2.5), (21.5, 2.5), (21.5, 21.5), (2.5, 21.5)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(6.5, 6.5), (17.5, 6.5), (17.5, 17.5), (6.5, 17.5)],
                true,
                Ink::Faint,
            );
            pen.shape(
                &[(10.0, 10.0), (14.0, 10.0), (14.0, 14.0), (10.0, 14.0)],
                Paint::Solid,
            );
        }
        Icon::Bevel => {
            pen.stroke(
                &[
                    (4.0, 21.0),
                    (4.0, 13.0),
                    (4.17, 11.24),
                    (4.69, 9.56),
                    (5.52, 8.0),
                    (6.64, 6.64),
                    (8.0, 5.52),
                    (9.56, 4.69),
                    (11.24, 4.17),
                    (13.0, 4.0),
                    (21.0, 4.0),
                ],
                false,
                Ink::Line,
            );
            pen.stroke(
                &[
                    (8.0, 13.0),
                    (8.1, 12.02),
                    (8.38, 11.09),
                    (8.84, 10.22),
                    (9.46, 9.46),
                    (10.22, 8.84),
                    (11.09, 8.38),
                    (12.02, 8.1),
                    (13.0, 8.0),
                ],
                false,
                Ink::Faint,
            );
        }
        Icon::Bridge => {
            pen.stroke(
                &[(2.0, 7.0), (8.0, 7.0), (8.0, 17.0), (2.0, 17.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(16.0, 7.0), (22.0, 7.0), (22.0, 17.0), (16.0, 17.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(8.0, 7.0), (16.0, 7.0)], false, Ink::Dashed);
            pen.stroke(&[(8.0, 12.0), (16.0, 12.0)], false, Ink::Dashed);
            pen.stroke(&[(8.0, 17.0), (16.0, 17.0)], false, Ink::Dashed);
        }
        Icon::FillHole => {
            pen.stroke(
                &[
                    (4.0, 10.0),
                    (4.0, 17.0),
                    (12.0, 21.5),
                    (20.0, 17.0),
                    (20.0, 10.0),
                ],
                false,
                Ink::Line,
            );
            pen.stroke(&[(12.0, 14.5), (12.0, 21.5)], false, Ink::Line);
            pen.shape(
                &[(4.0, 10.0), (12.0, 14.5), (20.0, 10.0), (12.0, 5.5)],
                Paint::Tone,
            );
            pen.stroke(
                &[(4.0, 10.0), (12.0, 14.5), (20.0, 10.0), (12.0, 5.5)],
                true,
                Ink::Dashed,
            );
            pen.stroke(&[(12.0, 1.5), (12.0, 7.0)], false, Ink::Line);
            pen.shape(&[(12.0, 9.5), (9.8, 6.64), (14.2, 6.64)], Paint::Solid);
        }
        Icon::FlipNormals => {
            pen.stroke(&[(3.0, 12.0), (21.0, 12.0)], false, Ink::Line);
            pen.stroke(&[(8.0, 12.0), (8.0, 4.0)], false, Ink::AxisX);
            pen.stroke(&[(5.5, 6.5), (8.0, 4.0), (10.5, 6.5)], false, Ink::AxisX);
            pen.stroke(&[(16.0, 12.0), (16.0, 20.0)], false, Ink::AxisY);
            pen.stroke(
                &[(13.5, 17.5), (16.0, 20.0), (18.5, 17.5)],
                false,
                Ink::AxisY,
            );
        }
        Icon::HardEdges => {
            pen.shape(
                &[(4.0, 7.0), (8.0, 9.5), (8.0, 20.5), (4.0, 18.0)],
                Paint::Shade,
            );
            pen.shape(
                &[(8.0, 9.5), (16.0, 9.5), (16.0, 20.5), (8.0, 20.5)],
                Paint::Tone,
            );
            pen.stroke(
                &[
                    (4.0, 7.0),
                    (8.0, 4.5),
                    (16.0, 4.5),
                    (20.0, 7.0),
                    (16.0, 9.5),
                    (8.0, 9.5),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 7.0), (4.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(20.0, 7.0), (20.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(8.0, 9.5), (8.0, 20.5)], false, Ink::Line);
            pen.stroke(&[(16.0, 9.5), (16.0, 20.5)], false, Ink::Line);
            pen.stroke(
                &[(4.0, 18.0), (8.0, 20.5), (16.0, 20.5), (20.0, 18.0)],
                false,
                Ink::Line,
            );
        }
        Icon::SoftEdges => {
            pen.ring((12.0, 6.0), (8.0, 3.0), Ink::Line);
            pen.shape(
                &[
                    (4.0, 6.0),
                    (4.0, 18.0),
                    (4.15, 18.59),
                    (4.61, 19.15),
                    (5.35, 19.67),
                    (6.34, 20.12),
                    (7.56, 20.49),
                    (8.94, 20.77),
                    (10.44, 20.94),
                    (12.0, 21.0),
                    (13.56, 20.94),
                    (15.06, 20.77),
                    (16.44, 20.49),
                    (17.66, 20.12),
                    (18.65, 19.67),
                    (19.39, 19.15),
                    (19.85, 18.59),
                    (20.0, 18.0),
                    (20.0, 6.0),
                    (19.85, 6.59),
                    (19.39, 7.15),
                    (18.65, 7.67),
                    (17.66, 8.12),
                    (16.44, 8.49),
                    (15.06, 8.77),
                    (13.56, 8.94),
                    (12.0, 9.0),
                    (10.44, 8.94),
                    (8.94, 8.77),
                    (7.56, 8.49),
                    (6.34, 8.12),
                    (5.35, 7.67),
                    (4.61, 7.15),
                    (4.15, 6.59),
                    (4.0, 6.0),
                ],
                Paint::Shading,
            );
            pen.stroke(&[(4.0, 6.0), (4.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(20.0, 6.0), (20.0, 18.0)], false, Ink::Line);
            pen.stroke(
                &[
                    (4.0, 18.0),
                    (4.15, 18.59),
                    (4.61, 19.15),
                    (5.35, 19.67),
                    (6.34, 20.12),
                    (7.56, 20.49),
                    (8.94, 20.77),
                    (10.44, 20.94),
                    (12.0, 21.0),
                    (13.56, 20.94),
                    (15.06, 20.77),
                    (16.44, 20.49),
                    (17.66, 20.12),
                    (18.65, 19.67),
                    (19.39, 19.15),
                    (19.85, 18.59),
                    (20.0, 18.0),
                ],
                false,
                Ink::Line,
            );
        }
        Icon::Dissolve => {
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(&[(3.0, 21.0), (9.5, 14.5)], false, Ink::Line);
            pen.disc((12.0, 12.0), 1.25, Paint::Solid);
            pen.disc((14.8, 9.2), 1.0, Paint::Solid);
            pen.disc((17.5, 6.5), 0.8, Paint::Solid);
            pen.disc((20.0, 4.0), 0.6, Paint::Solid);
        }
        Icon::Weld => {
            pen.stroke(&[(3.0, 12.0), (9.0, 12.0)], false, Ink::Line);
            pen.disc((3.0, 12.0), 1.8, Paint::Solid);
            pen.disc((9.0, 12.0), 1.8, Paint::Solid);
            pen.stroke(&[(11.5, 12.0), (13.5, 12.0)], false, Ink::Line);
            pen.shape(&[(16.0, 12.0), (13.4, 14.0), (13.4, 10.0)], Paint::Solid);
            pen.disc((19.5, 12.0), 2.6, Paint::Solid);
        }
        Icon::Connect => {
            pen.stroke(
                &[(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 12.0), (20.0, 12.0)], false, Ink::Dashed);
            pen.disc((4.0, 12.0), 2.0, Paint::Solid);
            pen.disc((20.0, 12.0), 2.0, Paint::Solid);
        }
        Icon::Knife => {
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(
                &[
                    (3.0, 15.0),
                    (8.0, 10.0),
                    (12.0, 13.0),
                    (17.0, 8.0),
                    (21.0, 11.0),
                ],
                false,
                Ink::Line,
            );
            pen.disc((3.0, 15.0), 1.6, Paint::Solid);
            pen.disc((21.0, 11.0), 1.6, Paint::Solid);
        }
        Icon::LoopCut => {
            pen.stroke(
                &[
                    (12.0, 3.0),
                    (20.0, 7.5),
                    (20.0, 16.5),
                    (12.0, 21.0),
                    (4.0, 16.5),
                    (4.0, 7.5),
                ],
                true,
                Ink::Faint,
            );
            pen.stroke(&[(4.0, 12.0), (12.0, 16.5), (20.0, 12.0)], false, Ink::Line);
            pen.stroke(&[(12.0, 3.0), (12.0, 7.5)], false, Ink::Faint);
        }
        Icon::Cancel => {
            pen.ring((12.0, 12.0), (9.0, 9.0), Ink::Line);
            pen.stroke(&[(7.5, 12.0), (16.5, 12.0)], false, Ink::Line);
        }
        Icon::Hidden => {
            // A closed eye: the lower lid, with three lashes.
            let lid = pen.arc((12.0, 8.1), (8.9, 8.9), 0.453, 2.688);
            pen.line(lid);
            pen.polyline(&[(12.0, 17.0), (12.0, 20.0)]);
            pen.polyline(&[(7.0, 15.5), (5.5, 18.0)]);
            pen.polyline(&[(17.0, 15.5), (18.5, 18.0)]);
        }
    }
    pen.shapes
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

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
