//! The workbench's part in the modelling tools set by dragging anywhere in
//! a view: Extrude and Inset ([decision 0038](../../../../docs/decisions/0038-extrude-and-inset.md)).
//!
//! A drag is one undo step, as with the gizmos: each frame takes back the
//! frame before (`Document::abandon`) and records the edit again with the
//! new distance. Letting go at no distance leaves nothing behind.

use super::Workbench;
use crate::commands::CommandId;
use crate::components::ViewPointer;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::transforming::Tool;
use crate::viewport::ToolDragEvent;
use egui::Ui;
use glam::{Mat4, Vec2, Vec3};
use mesh::Anchors;
use scene::{
    Bounds, Command, Document, EdgeKey, ElementKind, FaceId, Grouping, ObjectId, Operation,
};

/// Merge keys for modelling tool drags: kept apart from other merge keys.
const MODELLING_KEYS: u64 = 1 << 60;

/// What a drag works on, in one object.
#[derive(Debug, Clone, PartialEq)]
enum Elements {
    Polygons(Vec<FaceId>),
    Edges(Vec<EdgeKey>),
}

/// One object's part in a drag, as it was when the drag began.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    id: ObjectId,
    elements: Elements,
    /// How many world centimetres one of the object's own is (its scale,
    /// averaged over the three axes).
    scale: f32,
    anchors: Option<Anchors>,
}

/// A modelling tool's drag in progress.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ModellingSession {
    tool: Tool,
    key: u64,
    parts: Vec<Part>,
    /// The middle of what is worked on, in the world: where a screen
    /// point is measured in centimetres.
    pivot: Vec3,
    /// For Extrude, the way the polygons (or edges) face on average, in
    /// the world: dragging that way on screen pushes them out. `None` for
    /// Inset, or when they face every way at once.
    direction: Option<Vec3>,
    grouping: Grouping,
    /// The distance last shown, in world centimetres.
    shown: Option<f32>,
}

/// How long a world step is measured on screen, in points, to see how much
/// of the extrude direction lies across the screen.
const PROBE: f32 = 50.0;

/// Below this part of the direction lying across the screen (it points
/// almost straight at the viewer), dragging right means "more" instead.
const MIN_ACROSS: f32 = 0.2;

impl ModellingSession {
    /// The distance (world centimetres) a drag of `drag` screen points
    /// means, seen through `pointer`'s camera.
    ///
    /// For Extrude, the drag counts along the way the polygons face on
    /// screen, scaled so the polygons follow the mouse: drag a polygon on
    /// the left side to the left and it comes out. When they face the
    /// viewer almost straight on (or for Inset), right means more.
    fn distance(&self, drag: Vec2, pointer: &ViewPointer) -> f32 {
        let camera = &pointer.camera;
        let per_point = camera.world_per_pixel(self.pivot, pointer.size);
        if let Some(direction) = self.direction {
            let step = per_point * PROBE;
            let ends = (
                camera.project(self.pivot, pointer.size),
                camera.project(self.pivot + direction * step, pointer.size),
            );
            if let (Some(a), Some(b)) = ends {
                let across = (b - a).length() / PROBE;
                if across > MIN_ACROSS && across.is_finite() {
                    let way = (b - a) / (b - a).length();
                    return drag.dot(way) * per_point / across;
                }
            }
        }
        drag.x * per_point
    }

    /// The operation for `part` at `distance` (world centimetres).
    fn operation(&self, part: &Part, distance: f32) -> Operation {
        let local = distance / part.scale;
        let anchors = part.anchors.clone();
        match (&part.elements, self.tool) {
            (Elements::Polygons(faces), Tool::Inset) => Operation::InsetPolygons {
                faces: faces.clone(),
                distance: local,
                grouping: self.grouping,
                anchors,
            },
            (Elements::Polygons(faces), _) => Operation::ExtrudePolygons {
                faces: faces.clone(),
                distance: local,
                grouping: self.grouping,
                anchors,
            },
            (Elements::Edges(edges), _) => Operation::ExtrudeEdges {
                edges: edges.clone(),
                distance: local,
                grouping: self.grouping,
                anchors,
            },
        }
    }

    /// The undo step's name: "Extrude 4 polygons".
    fn label(&self) -> String {
        let (polygons, edges) =
            self.parts
                .iter()
                .fold((0, 0), |(p, e), part| match &part.elements {
                    Elements::Polygons(f) => (p + f.len(), e),
                    Elements::Edges(x) => (p, e + x.len()),
                });
        let count = |n: usize, thing: &str| {
            if n == 1 {
                format!("1 {thing}")
            } else {
                format!("{n} {thing}s")
            }
        };
        let what = if edges > 0 {
            count(edges, "edge")
        } else {
            count(polygons, "polygon")
        };
        format!("{} {what}", self.tool.name())
    }

    /// Makes the scene show the edit at `distance` (world centimetres),
    /// instead of what was shown before.
    fn show(&self, document: &mut Document, distance: f32) -> Result<(), String> {
        document.abandon(self.key);
        if distance.abs() < 1e-4 {
            return Ok(());
        }
        let label = self.label();
        for part in &self.parts {
            let command = Command::Record {
                id: part.id,
                operation: self.operation(part, distance),
            };
            document
                .execute_labelled(command, &label, Some(self.key))
                .map_err(|why| why.to_string())?;
        }
        Ok(())
    }
}

impl Workbench {
    /// D and I: Extrude and Inset, from the command list. Not while typing.
    pub(super) fn modelling_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        for command in [CommandId::Extrude, CommandId::Inset] {
            let pressed = command
                .info()
                .shortcut
                .is_some_and(|keys| ui.input_mut(|i| i.consume_shortcut(&keys)));
            if pressed {
                self.run_command(command, ui.ctx(), &mut Vec::new());
            }
        }
    }

    /// Tells the viewport whether drags go to a modelling tool.
    pub(super) fn place_drag_tool(&mut self) {
        self.viewport.set_drag_tool(self.tool.drags_anywhere());
    }

    /// The grouping the modelling tools use: neighbours together, or each
    /// on its own (the Attributes panel's Keep together switch).
    pub(crate) fn grouping(&self) -> Grouping {
        self.attributes.grouping
    }

    /// Carries out this frame's modelling tool drags.
    pub(super) fn carry_out_tool_drag(&mut self, drawing: &SceneDrawing) {
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { .. } => {
                    self.select_under_the_press();
                    self.modelling_session = self.begin_modelling(drawing);
                }
                ToolDragEvent::Update { drag, pointer } => {
                    let Some(session) = &mut self.modelling_session else {
                        continue;
                    };
                    let mut distance = session.distance(drag, &pointer);
                    if session.tool == Tool::Inset {
                        // Inward only: outward would overlap the
                        // neighbours.
                        distance = distance.max(0.0);
                    }
                    if session.shown == Some(distance) {
                        continue;
                    }
                    session.shown = Some(distance);
                    let session = session.clone();
                    let text = format!("{}: {}", session.label(), self.units.format(distance));
                    self.note = Some(match session.show(&mut self.document, distance) {
                        Ok(()) => {
                            format!("{text}. Shift for fine steps; Esc or a right click cancels.")
                        }
                        Err(why) => format!("Couldn't do that: {why}."),
                    });
                    self.viewport.set_tool_label(Some(text));
                }
                ToolDragEvent::End => {
                    if let Some(session) = self.modelling_session.take() {
                        self.finish_modelling(&session);
                    }
                    self.viewport.set_tool_label(None);
                }
                ToolDragEvent::Cancel => {
                    if let Some(session) = self.modelling_session.take() {
                        session.show(&mut self.document, 0.0).ok();
                        self.note = Some("Cancelled: everything is back where it was.".into());
                    }
                    self.viewport.set_tool_label(None);
                }
            }
        }
    }

    /// Starts a drag of the modelling tool in use, on the selected
    /// polygons (or, for Extrude in the Edges mode, edges). Says in the
    /// status bar why not, when there is nothing to work on.
    fn begin_modelling(&mut self, drawing: &SceneDrawing) -> Option<ModellingSession> {
        let tool = self.tool;
        let kind = match (self.mode, tool) {
            (Mode::Polygons, Tool::Extrude | Tool::Inset) => ElementKind::Polygons,
            (Mode::Edges, Tool::Extrude) => ElementKind::Edges,
            _ => {
                self.note = Some(match tool {
                    Tool::Inset => {
                        "Inset works on polygons: press 3 for the Polygons mode and select some."
                    }
                    _ => {
                        "Extrude works on polygons or edges: press 3 for the Polygons mode \
                         (or 2 for Edges) and select some."
                    }
                }
                .into());
                return None;
            }
        };
        let mut parts = Vec::new();
        let mut points = Vec::new();
        // The way everything faces, summed in the world.
        let mut facing = Vec3::ZERO;
        for item in drawing.items.iter().filter(|i| i.selected) {
            let Some(selected) = self.document.elements(item.id) else {
                continue;
            };
            let valid = selected.valid_in(&item.mesh, item.mesh.topology());
            let (elements, corners): (Elements, Vec<Vec<scene::VertexId>>) = match kind {
                ElementKind::Polygons => {
                    let faces: Vec<FaceId> = valid.polygons().collect();
                    let corners = faces
                        .iter()
                        .filter_map(|f| Some(item.mesh.face(*f)?.to_vec()))
                        .collect();
                    (Elements::Polygons(faces), corners)
                }
                _ => {
                    let edges: Vec<EdgeKey> = valid.edges().collect();
                    let corners = edges
                        .iter()
                        .map(|e| {
                            let (a, b) = e.ends();
                            vec![a, b]
                        })
                        .collect();
                    (Elements::Edges(edges), corners)
                }
            };
            if corners.is_empty() {
                continue;
            }
            facing += world_facing(&item.mesh, item.transform, &elements);
            points.extend(
                corners
                    .iter()
                    .flatten()
                    .filter_map(|v| item.mesh.position(*v))
                    .map(|p| item.transform.transform_point3(p)),
            );
            parts.push(Part {
                id: item.id,
                elements,
                scale: average_scale(item.transform),
                anchors: Anchors::record(&item.mesh, kind, &corners),
            });
        }
        let Some(bounds) = Bounds::around(points) else {
            self.note = Some(format!(
                "{}: select some {} first.",
                tool.name(),
                kind.plural()
            ));
            return None;
        };
        self.sessions += 1;
        Some(ModellingSession {
            tool,
            key: MODELLING_KEYS | self.sessions,
            parts,
            pivot: bounds.centre(),
            direction: (tool == Tool::Extrude)
                .then(|| facing.try_normalize())
                .flatten(),
            grouping: self.grouping(),
            shown: None,
        })
    }

    /// When a drag starts on a polygon (or, for Extrude in the Edges mode,
    /// an edge) of a selected object that isn't selected, it becomes the
    /// selection first, as its own undo step: drag straight on the
    /// polygon you want, without clicking it first. Starting on something
    /// selected, or on empty space, keeps the selection.
    fn select_under_the_press(&mut self) {
        let kind = match (self.mode, self.tool) {
            (Mode::Polygons, Tool::Extrude | Tool::Inset) => ElementKind::Polygons,
            (Mode::Edges, Tool::Extrude) => ElementKind::Edges,
            _ => return,
        };
        let Some((id, element)) = self.components.hovered() else {
            return;
        };
        if element.kind() != kind || !self.document.selection().contains(&id) {
            return;
        }
        if self
            .document
            .elements(id)
            .is_some_and(|s| s.contains(element))
        {
            return;
        }
        let changes: Vec<_> = self
            .document
            .selection()
            .into_iter()
            .map(|object| {
                let mut selection = self.document.elements(object).cloned().unwrap_or_default();
                selection.clear_kind(kind);
                if object == id {
                    selection.insert(element);
                }
                (object, selection)
            })
            .collect();
        let label = format!("Select 1 {}", kind.singular());
        self.document.select_elements(changes, label, None);
    }

    /// After a drag: says what was done, and after extruding edges moves
    /// the edge selection to the new outer edges, so pressing on carries
    /// on from there (polygons keep their ids and stay selected anyway).
    fn finish_modelling(&mut self, session: &ModellingSession) {
        if session.shown.is_none_or(|d| d.abs() < 1e-4) {
            self.note = Some(format!("{}: nothing changed.", session.tool.name()));
            return;
        }
        let mut changes = Vec::new();
        for part in &session.parts {
            let Elements::Edges(edges) = &part.elements else {
                continue;
            };
            let Some(shape) = self.document.scene().evaluated(part.id) else {
                continue;
            };
            let mut selection = self.document.elements(part.id).cloned().unwrap_or_default();
            selection.clear_kind(ElementKind::Edges);
            for edge in outer_edges(&shape.mesh, edges) {
                selection.insert(scene::Element::Edge(edge));
            }
            changes.push((part.id, selection));
        }
        if !changes.is_empty() {
            self.document
                .select_elements(changes, session.label(), Some(session.key));
        }
    }
}

/// For each of `edges` (just extruded), the new outer edge: the far side
/// of the newest polygon along it.
fn outer_edges(mesh: &scene::Mesh, edges: &[EdgeKey]) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    edges
        .iter()
        .filter_map(|edge| {
            let newest = topology.edge_faces(*edge).max()?;
            let corners = mesh.face(newest)?;
            let [_, _, c, d] = corners else {
                return None;
            };
            Some(EdgeKey::new(*c, *d))
        })
        .collect()
}

/// The way `elements` of `mesh` face, summed and in the world (`world` is
/// the object's matrix): polygons by their facing, border edges by the way
/// they grow.
fn world_facing(mesh: &scene::Mesh, world: Mat4, elements: &Elements) -> Vec3 {
    let normals = world.inverse().transpose();
    let to_world = |v: Vec3| normals.transform_vector3(v).normalize_or_zero();
    match elements {
        Elements::Polygons(faces) => faces.iter().map(|f| to_world(mesh.face_normal(*f))).sum(),
        Elements::Edges(edges) => {
            let topology = mesh.topology();
            edges
                .iter()
                .filter_map(|edge| {
                    let [only] = topology.edge_uses(*edge) else {
                        return None;
                    };
                    let (low, high) = edge.ends();
                    let (a, b) = if only.forward {
                        (low, high)
                    } else {
                        (high, low)
                    };
                    let along = mesh.position(b)? - mesh.position(a)?;
                    let outward = along.cross(mesh.face_normal(only.face));
                    Some(world.transform_vector3(outward).normalize_or_zero())
                })
                .sum()
        }
    }
}

/// An object's size compared with its own space: its world matrix's scale,
/// averaged over the three axes (1 when it isn't scaled).
fn average_scale(world: Mat4) -> f32 {
    let scale = (world.x_axis.truncate().length()
        + world.y_axis.truncate().length()
        + world.z_axis.truncate().length())
        / 3.0;
    if scale.is_finite() && scale > 1e-6 {
        scale
    } else {
        1.0
    }
}
