//! The workbench's part in Connect, Bridge and Weld
//! ([decision 0051](../../../../docs/decisions/0051-connect-bridge-weld.md)).
//!
//! The controls, chosen by the project lead:
//!
//! - **A** connects the selected points (Points mode) or edges (Edges
//!   mode) across their polygons, at once (it was J until the project
//!   lead moved it within easy reach, [decision 0057](../../../../docs/decisions/0057-after-phase-4-testing.md)).
//! - **C** bridges at once when the selection can be bridged: two open
//!   borders in the Edges mode, or two polygons in the Polygons mode.
//!   Otherwise C makes Bridge the tool: drag from one open edge to another
//!   and a polygon joins them.
//! - **M** welds the selected points at their middle; **Ctrl + M** makes
//!   Weld the tool: drag a point onto another and it melts into that one
//!   (all the selected points, if the dragged one is selected).
//!
//! Each is recorded per object as an `Operation` with anchors, one undo
//! step in all.

use super::Workbench;
use crate::components::ViewPointer;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::transforming::Tool;
use crate::viewport::{ToolDragEvent, ToolPath};
use glam::Vec3;
use mesh::Anchors;
use scene::{Command, EdgeKey, Element, ElementKind, FaceId, ObjectId, Operation, VertexId};

/// Something picked to drag from or onto: which object, which element,
/// and where it is in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Picked {
    id: ObjectId,
    element: Element,
    at: Vec3,
}

/// A Bridge or Weld drag in progress.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PairSession {
    from: Picked,
    /// Where the mouse was last.
    pointer: ViewPointer,
    /// What the drag would end on if let go now: another point or edge of
    /// the same object under the mouse. The line snaps to it.
    onto: Option<Picked>,
}

/// "Connect 3 points", "Weld 1 point".
pub(super) fn counted(verb: &str, n: usize, thing: &str) -> String {
    if n == 1 {
        format!("{verb} 1 {thing}")
    } else {
        format!("{verb} {n} {thing}s")
    }
}

impl Workbench {
    /// Each selected object with a shape, with its selected elements as
    /// they fit the shape now.
    pub(super) fn selected_parts(
        &self,
    ) -> Vec<(
        ObjectId,
        scene::ElementSelection,
        std::sync::Arc<scene::Mesh>,
    )> {
        self.document
            .selection()
            .into_iter()
            .filter_map(|id| {
                let mesh = self.document.scene().evaluated(id)?.mesh;
                let selected = self.document.elements(id)?;
                let valid = selected.valid_in(&mesh, mesh.topology());
                Some((id, valid, mesh))
            })
            .collect()
    }

    /// Records one operation per object as one undo step called `label`.
    pub(super) fn record_each(&mut self, label: &str, ops: Vec<(ObjectId, Operation)>) -> bool {
        let key = self.new_step_key();
        self.record_each_in(label, ops, key)
    }

    /// A merge key for a new undo step made of several edits.
    pub(super) fn new_step_key(&mut self) -> u64 {
        self.sessions += 1;
        (1 << 57) | self.sessions
    }

    /// Records each edit on its object as part of the undo step `key`
    /// (edits recorded with the same key are taken back together).
    pub(super) fn record_each_in(
        &mut self,
        label: &str,
        ops: Vec<(ObjectId, Operation)>,
        key: u64,
    ) -> bool {
        if ops.is_empty() {
            return false;
        }
        for (id, operation) in ops {
            if let Err(why) =
                self.document
                    .execute_labelled(Command::Record { id, operation }, label, Some(key))
            {
                self.note = Some(format!("Couldn't do that: {why}."));
                return false;
            }
        }
        true
    }

    /// A: connects the selected points or edges.
    pub(super) fn run_connect(&mut self) {
        let mut ops = Vec::new();
        let mut count = 0;
        let edges_mode = self.mode == Mode::Edges;
        for (id, selected, mesh) in self.selected_parts() {
            if edges_mode {
                let edges: Vec<EdgeKey> = selected.edges().collect();
                if edges.len() < 2 {
                    continue;
                }
                count += edges.len();
                let ends: Vec<Vec<VertexId>> = edges
                    .iter()
                    .map(|e| {
                        let (a, b) = e.ends();
                        vec![a, b]
                    })
                    .collect();
                ops.push((
                    id,
                    Operation::Connect {
                        points: Vec::new(),
                        edges,
                        cuts: self.attributes.join.connect_cuts,
                        anchors: Anchors::record(&mesh, ElementKind::Edges, &ends),
                    },
                ));
            } else {
                let points: Vec<VertexId> = selected.points().collect();
                if points.len() < 2 {
                    continue;
                }
                count += points.len();
                let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
                ops.push((
                    id,
                    Operation::Connect {
                        points,
                        edges: Vec::new(),
                        cuts: 1,
                        anchors: Anchors::record(&mesh, ElementKind::Points, &places),
                    },
                ));
            }
        }
        let thing = if edges_mode { "edge" } else { "point" };
        if ops.is_empty() {
            self.note = Some(format!(
                "Connect: select two or more {thing}s on the same polygon first."
            ));
            return;
        }
        let label = counted("Connect", count, thing);
        if self.record_each(&label, ops) {
            self.note = Some(format!("{label}. Ctrl + Z takes it back."));
        }
    }

    /// C: bridges the selection at once if it can be bridged (two open
    /// borders, or two polygons); otherwise makes Bridge the tool for
    /// dragging from one open edge to another.
    pub(super) fn run_bridge(&mut self) {
        let (segments, twist) = self.attributes.join.bridge;
        let mut ops = Vec::new();
        for (id, selected, mesh) in self.selected_parts() {
            match self.mode {
                Mode::Polygons => {
                    let faces: Vec<FaceId> = selected.polygons().collect();
                    if faces.len() != 2 {
                        continue;
                    }
                    let corners: Vec<Vec<VertexId>> = faces
                        .iter()
                        .filter_map(|f| Some(mesh.face(*f)?.to_vec()))
                        .collect();
                    ops.push((
                        id,
                        Operation::Bridge {
                            edges: Vec::new(),
                            faces,
                            segments,
                            twist,
                            anchors: Anchors::record(&mesh, ElementKind::Polygons, &corners),
                        },
                    ));
                }
                Mode::Edges => {
                    let topology = mesh.topology();
                    let edges: Vec<EdgeKey> = selected
                        .edges()
                        .filter(|e| topology.is_border(*e))
                        .collect();
                    if edges.len() < 2 {
                        continue;
                    }
                    let ends: Vec<Vec<VertexId>> = edges
                        .iter()
                        .map(|e| {
                            let (a, b) = e.ends();
                            vec![a, b]
                        })
                        .collect();
                    ops.push((
                        id,
                        Operation::Bridge {
                            edges,
                            faces: Vec::new(),
                            segments,
                            twist,
                            anchors: Anchors::record(&mesh, ElementKind::Edges, &ends),
                        },
                    ));
                }
                _ => {}
            }
        }
        if ops.is_empty() {
            self.set_tool(Tool::Bridge);
            return;
        }
        let label = if self.mode == Mode::Polygons {
            "Bridge 2 polygons"
        } else {
            "Bridge 2 borders"
        };
        if self.record_each(label, ops) {
            self.note = Some(format!(
                "{label}. Segments and Twist are in the Attributes panel; Ctrl + Z takes it back."
            ));
        }
    }

    /// M: welds the selected points at their middle; in the Polygons
    /// mode, each touching group of selected polygons at its own middle.
    pub(super) fn run_weld(&mut self) {
        if self.mode == crate::palette::Mode::Polygons {
            self.run_weld_polygons();
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, selected, mesh) in self.selected_parts() {
            let points: Vec<VertexId> = selected.points().collect();
            if points.len() < 2 {
                continue;
            }
            count += points.len();
            let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
            ops.push((
                id,
                Operation::Weld {
                    points,
                    target: None,
                    anchors: Anchors::record(&mesh, ElementKind::Points, &places),
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(
                "Weld: select two or more points first, or press Ctrl + M and drag a point \
                 onto another."
                    .into(),
            );
            return;
        }
        let label = counted("Weld", count, "point");
        if self.record_each(&label, ops) {
            self.note = Some(format!("{label} at their middle. Ctrl + Z takes it back."));
        }
    }

    /// M in the Polygons mode: the points of each group of selected
    /// polygons (polygons sharing a point are one group) melt into one at
    /// the group's middle. An inset cap's middle polygon becomes a point
    /// with a fan of triangles round it.
    fn run_weld_polygons(&mut self) {
        let mut ops = Vec::new();
        let mut polygons = 0;
        for (id, selected, mesh) in self.selected_parts() {
            let faces: Vec<scene::FaceId> = selected.polygons().collect();
            for group in polygon_groups(&mesh, &faces) {
                let points: Vec<VertexId> = group
                    .iter()
                    .filter_map(|f| mesh.face(*f))
                    .flatten()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                if points.len() < 2 {
                    continue;
                }
                polygons += group.len();
                let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
                ops.push((
                    id,
                    Operation::Weld {
                        points,
                        target: None,
                        anchors: Anchors::record(&mesh, ElementKind::Points, &places),
                    },
                ));
            }
        }
        if ops.is_empty() {
            self.note = Some(
                "Weld: select a polygon first (its points melt into one at its middle), or \
                 press Ctrl + M and drag a polygon onto a point."
                    .into(),
            );
            return;
        }
        let label = counted("Weld", polygons, "polygon");
        if self.record_each(&label, ops) {
            self.note = Some(format!(
                "{label} into a point at the middle. Ctrl + Z takes it back."
            ));
        }
    }

    /// The element of `kind` under `pointer` on a selected object.
    fn pick(
        &mut self,
        drawing: &SceneDrawing,
        pointer: ViewPointer,
        kind: ElementKind,
    ) -> Option<Picked> {
        let (id, element) = self.components.find(kind, drawing, pointer)?;
        let item = drawing.items.iter().find(|i| i.id == id)?;
        let local = match element {
            Element::Point(v) => item.mesh.position(v)?,
            Element::Edge(e) => {
                let (a, b) = e.ends();
                item.mesh.position(a)?.lerp(item.mesh.position(b)?, 0.5)
            }
            Element::Polygon(f) => {
                let corners = item.mesh.face(f)?;
                let sum: glam::Vec3 = corners.iter().filter_map(|v| item.mesh.position(*v)).sum();
                sum / corners.len().max(1) as f32
            }
        };
        Some(Picked {
            id,
            element,
            at: item.transform.transform_point3(local),
        })
    }

    /// Carries out this frame's Bridge or Weld drags: from an open edge to
    /// another, or from a point onto another.
    pub(super) fn carry_out_pairing(&mut self, drawing: &SceneDrawing) {
        let kind = if self.tool == Tool::Weld {
            ElementKind::Points
        } else {
            ElementKind::Edges
        };
        // Welding in the Polygons mode drags a polygon onto a point.
        let from_kind = if self.tool == Tool::Weld && self.mode == crate::palette::Mode::Polygons {
            ElementKind::Polygons
        } else {
            kind
        };
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { pointer } => {
                    self.pair_session =
                        self.pick(drawing, pointer, from_kind)
                            .map(|from| PairSession {
                                from,
                                pointer,
                                onto: None,
                            });
                    if self.pair_session.is_none() {
                        self.note = Some(match self.tool {
                            Tool::Weld if from_kind == ElementKind::Polygons => {
                                "Weld: press on a polygon of a selected object and drag it \
                                 onto a point."
                                    .into()
                            }
                            Tool::Weld => "Weld: press on a point of a selected object and drag \
                                           it onto another."
                                .into(),
                            _ => "Bridge: press on an open edge of a selected object and drag \
                                  to another."
                                .into(),
                        });
                    }
                }
                ToolDragEvent::Update { pointer, .. } => {
                    let Some(from) = self.pair_session.as_ref().map(|s| s.from) else {
                        continue;
                    };
                    let onto = self
                        .pick(drawing, pointer, kind)
                        .filter(|o| o.id == from.id && o.element != from.element);
                    if let Some(session) = &mut self.pair_session {
                        session.pointer = pointer;
                        session.onto = onto;
                    }
                }
                ToolDragEvent::End => {
                    if let Some(session) = self.pair_session.take() {
                        // What the line snapped to is what it joins.
                        let onto = session
                            .onto
                            .or_else(|| self.pick(drawing, session.pointer, kind));
                        self.finish_pair(drawing, session.from, onto);
                    }
                }
                ToolDragEvent::Cancel => {
                    self.pair_session = None;
                    self.note = Some("Cancelled.".into());
                }
            }
        }
        // The line from where the drag began to the mouse, snapped onto
        // the point or edge it would join, which is ringed.
        let path = self.pair_session.as_ref().map(|s| {
            if let Some(onto) = s.onto {
                return ToolPath {
                    points: vec![s.from.at, onto.at],
                    cursor: Some(onto.at),
                };
            }
            let ray = s.pointer.camera.ray_through(s.pointer.at, s.pointer.size);
            let depth = (s.from.at - ray.origin).dot(ray.direction).max(1.0);
            ToolPath {
                points: vec![s.from.at, ray.at(depth)],
                cursor: None,
            }
        });
        if matches!(self.tool, Tool::Bridge | Tool::Weld) || path.is_some() {
            self.viewport.set_tool_path(path);
        }
    }

    /// Joins what a drag began on to what it ended on.
    fn finish_pair(&mut self, drawing: &SceneDrawing, from: Picked, onto: Option<Picked>) {
        let Some(onto) = onto.filter(|o| o.id == from.id && o.element != from.element) else {
            self.note = Some("Let go on another one of the same object to join them.".into());
            return;
        };
        let Some(item) = drawing.items.iter().find(|i| i.id == from.id) else {
            return;
        };
        let mesh = std::sync::Arc::clone(&item.mesh);
        match (from.element, onto.element) {
            (Element::Edge(a), Element::Edge(b)) => {
                let topology = mesh.topology();
                if !topology.is_border(a) || !topology.is_border(b) {
                    self.note = Some(
                        "Bridge joins open edges: edges with a polygon on one side only.".into(),
                    );
                    return;
                }
                let ends: Vec<Vec<VertexId>> = [a, b]
                    .iter()
                    .map(|e| {
                        let (x, y) = e.ends();
                        vec![x, y]
                    })
                    .collect();
                let (segments, twist) = self.attributes.join.bridge;
                let op = Operation::Bridge {
                    edges: vec![a, b],
                    faces: Vec::new(),
                    segments,
                    twist,
                    anchors: Anchors::record(&mesh, ElementKind::Edges, &ends),
                };
                if self.record_each("Bridge 2 edges", vec![(from.id, op)]) {
                    self.note = Some("Bridge 2 edges. Ctrl + Z takes it back.".into());
                }
            }
            (Element::Point(v), Element::Point(target)) => {
                // A selected point brings the whole selection with it.
                let selected: Vec<VertexId> = self
                    .document
                    .elements(from.id)
                    .map(|s| s.points().collect())
                    .unwrap_or_default();
                let points: Vec<VertexId> = if selected.contains(&v) {
                    selected.into_iter().filter(|p| *p != target).collect()
                } else {
                    vec![v]
                };
                let mut places: Vec<Vec<VertexId>> = points.iter().map(|p| vec![*p]).collect();
                places.push(vec![target]);
                let label = counted("Weld", points.len() + 1, "point");
                let op = Operation::Weld {
                    points,
                    target: Some(target),
                    anchors: Anchors::record(&mesh, ElementKind::Points, &places),
                };
                if self.record_each(&label, vec![(from.id, op)]) {
                    self.note = Some(format!("{label} onto the target. Ctrl + Z takes it back."));
                }
            }
            (Element::Polygon(face), Element::Point(target)) => {
                // A selected polygon brings every selected polygon with it.
                let selected: Vec<scene::FaceId> = self
                    .document
                    .elements(from.id)
                    .map(|s| s.polygons().collect())
                    .unwrap_or_default();
                let faces = if selected.contains(&face) {
                    selected
                } else {
                    vec![face]
                };
                let points: Vec<VertexId> = faces
                    .iter()
                    .filter_map(|f| mesh.face(*f))
                    .flatten()
                    .copied()
                    .filter(|v| *v != target)
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                if points.is_empty() {
                    return;
                }
                let mut places: Vec<Vec<VertexId>> = points.iter().map(|p| vec![*p]).collect();
                places.push(vec![target]);
                let label = counted("Weld", faces.len(), "polygon");
                let op = Operation::Weld {
                    points,
                    target: Some(target),
                    anchors: Anchors::record(&mesh, ElementKind::Points, &places),
                };
                if self.record_each(&label, vec![(from.id, op)]) {
                    self.note = Some(format!("{label} onto the point. Ctrl + Z takes it back."));
                }
            }
            _ => {}
        }
    }
}

/// `faces` in groups that share a point: each group welds to its own
/// middle (decision 0128).
fn polygon_groups(mesh: &scene::Mesh, faces: &[scene::FaceId]) -> Vec<Vec<scene::FaceId>> {
    let mut groups: Vec<(std::collections::BTreeSet<VertexId>, Vec<scene::FaceId>)> = Vec::new();
    for face in faces {
        let Some(corners) = mesh.face(*face) else {
            continue;
        };
        let points: std::collections::BTreeSet<VertexId> = corners.iter().copied().collect();
        // Every group this polygon touches joins into one with it.
        let (touching, apart): (Vec<_>, Vec<_>) = groups
            .into_iter()
            .partition(|(shared, _)| !shared.is_disjoint(&points));
        let mut joined = (points, vec![*face]);
        for (shared, members) in touching {
            joined.0.extend(shared);
            joined.1.extend(members);
        }
        groups = apart;
        groups.push(joined);
    }
    groups.into_iter().map(|(_, faces)| faces).collect()
}
