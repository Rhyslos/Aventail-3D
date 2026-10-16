//! The workbench's part in Loop cut ([decision 0047](../../../../docs/decisions/0047-loop-cut.md)).
//!
//! The controls, chosen by the project lead: point at an edge of a
//! selected object and the new loop shows across the polygons it would
//! cut; click to cut it there, or press and drag along the edge to slide
//! it into place before letting go. With one cut, the loop sits where the
//! mouse is along the edge; with several, they are spread evenly and the
//! drag slides them together. The number of cuts is in the Attributes
//! panel (the mouse wheel is the view's zoom, and the middle button its
//! pan).
//!
//! A cut is one undo step, as with the other tools: each frame of a drag
//! takes back the frame before (`Document::abandon`) and records the cut
//! again at the new place.

use super::Workbench;
use crate::components::{self, ViewPointer};
use crate::drawing::SceneDrawing;
use crate::transforming::Tool;
use crate::viewport::ToolDragEvent;
use glam::{Mat4, Vec2, Vec3};
use mesh::Anchors;
use scene::{Command, Document, EdgeKey, Element, ElementKind, Mesh, ObjectId, Operation};
use std::sync::Arc;

/// Merge keys for loop cuts: kept apart from other merge keys.
const LOOP_CUT_KEYS: u64 = 1 << 59;

/// An edge a loop cut would go across, and where along it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LoopTarget {
    id: ObjectId,
    edge: EdgeKey,
    /// The shape the edge is on, in the object's own space.
    mesh: Arc<Mesh>,
    /// The object's place in the world.
    transform: Mat4,
    /// Where the cut sits across the edge, 0 to 1 from its lower point id.
    position: f32,
}

impl LoopTarget {
    /// The edge's ends in the world, lower point id first.
    fn ends(&self) -> Option<[Vec3; 2]> {
        let (a, b) = self.edge.ends();
        let world = |v| {
            self.mesh
                .position(v)
                .map(|p| self.transform.transform_point3(p))
        };
        Some([world(a)?, world(b)?])
    }

    /// How far along the edge (0 to 1, from its lower point id) the screen
    /// point `at` is, seen through `pointer`'s camera. `None` when the edge
    /// can't be seen.
    fn share_at(&self, pointer: &ViewPointer, at: Vec2) -> Option<f32> {
        let [a, b] = self.ends()?;
        let a = pointer.camera.project(a, pointer.size)?;
        let b = pointer.camera.project(b, pointer.size)?;
        let along = b - a;
        if along.length_squared() < 1e-6 {
            return Some(0.5);
        }
        Some(((at - a).dot(along) / along.length_squared()).clamp(0.0, 1.0))
    }

    /// The new loop's lines, in the world.
    fn preview(&self, cuts: u32) -> Vec<[Vec3; 2]> {
        mesh::loopcut::preview(&self.mesh, self.edge, cuts, self.position)
            .into_iter()
            .map(|[a, b]| {
                [
                    self.transform.transform_point3(a),
                    self.transform.transform_point3(b),
                ]
            })
            .collect()
    }

    /// The edit cutting `cuts` loops here.
    fn operation(&self, cuts: u32) -> Operation {
        let (a, b) = self.edge.ends();
        Operation::LoopCut {
            edge: self.edge,
            cuts,
            position: self.position,
            anchors: Anchors::record(&self.mesh, ElementKind::Edges, &[vec![a, b]]),
        }
    }
}

/// A loop cut being placed: from the press until the button is let go.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LoopCutSession {
    key: u64,
    target: LoopTarget,
    pub(super) cuts: u32,
    /// Where the press was, in the view's screen points, and how far along
    /// the edge that is.
    start: Vec2,
    start_share: f32,
    /// Where the cut started: the slide is measured from here.
    start_position: f32,
}

impl LoopCutSession {
    /// Makes the scene show the cut at the session's position, instead of
    /// what was shown before.
    fn show(&self, document: &mut Document) -> Result<(), String> {
        document.abandon(self.key);
        let command = Command::Record {
            id: self.target.id,
            operation: self.target.operation(self.cuts),
        };
        document
            .execute_labelled(command, label(self.cuts), Some(self.key))
            .map(|_| ())
            .map_err(|why| why.to_string())
    }
}

/// The undo step's name: "Loop cut", "Loop cut (3 cuts)".
fn label(cuts: u32) -> String {
    if cuts == 1 {
        "Loop cut".into()
    } else {
        format!("Loop cut ({cuts} cuts)")
    }
}

impl Workbench {
    /// The edge of a selected object under `pointer`, with where a cut
    /// there would sit. `None` when there is none, or the edge has no
    /// four-sided polygon to cut.
    fn loop_target(&mut self, drawing: &SceneDrawing, pointer: ViewPointer) -> Option<LoopTarget> {
        let (id, element) = self.components.find(ElementKind::Edges, drawing, pointer)?;
        let Element::Edge(edge) = element else {
            return None;
        };
        let item = drawing.items.iter().find(|i| i.id == id)?;
        if mesh::loopcut::ring_polygons(&item.mesh, edge) == 0 {
            return None;
        }
        let mut target = LoopTarget {
            id,
            edge,
            mesh: Arc::clone(&item.mesh),
            transform: item.transform,
            position: 0.5,
        };
        // One cut sits under the mouse; several are spread evenly.
        if self.attributes.loop_cut.cuts == 1 {
            target.position = target.share_at(&pointer, pointer.at).unwrap_or(0.5);
        }
        Some(target)
    }

    /// While Loop cut is in use and nothing is being cut: shows the loop
    /// the edge under the mouse would get.
    pub(super) fn show_loop_cut_preview(&mut self, drawing: &SceneDrawing) {
        let hovering = self.tool == Tool::LoopCut
            && self.loop_cut_session.is_none()
            && components::kind_for(self.mode).is_some();
        let lines = match self.viewport.pointer() {
            Some(pointer) if hovering => self
                .loop_target(drawing, pointer)
                .map(|t| t.preview(self.attributes.loop_cut.cuts))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        self.components.set_tool_lines(lines);
    }

    /// Carries out this frame's Loop cut presses, drags and clicks.
    pub(super) fn carry_out_loop_cut(&mut self, drawing: &SceneDrawing) {
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { pointer } => {
                    self.loop_cut_session = self.begin_loop_cut(drawing, pointer);
                }
                ToolDragEvent::Update { drag, pointer } => {
                    let Some(session) = &mut self.loop_cut_session else {
                        continue;
                    };
                    let at = session.start + drag;
                    let Some(share) = session.target.share_at(&pointer, at) else {
                        continue;
                    };
                    let position =
                        (session.start_position + (share - session.start_share)).clamp(0.0, 1.0);
                    if position == session.target.position {
                        continue;
                    }
                    session.target.position = position;
                    let session = session.clone();
                    self.show_loop_cut(&session);
                }
                ToolDragEvent::End => {
                    if let Some(session) = self.loop_cut_session.take() {
                        self.note = Some(format!(
                            "{} across {}. Change Cuts and Position in the Attributes panel; \
                             Ctrl+Z takes it back.",
                            label(session.cuts),
                            polygons(mesh::loopcut::ring_polygons(
                                &session.target.mesh,
                                session.target.edge
                            ))
                        ));
                    }
                    self.viewport.set_tool_label(None);
                }
                ToolDragEvent::Cancel => {
                    if let Some(session) = self.loop_cut_session.take() {
                        self.document.abandon(session.key);
                        self.note = Some("Cancelled: everything is back where it was.".into());
                    }
                    self.viewport.set_tool_label(None);
                }
            }
        }
    }

    /// Starts a loop cut across the edge under the press, and cuts it.
    /// Says in the status bar why not, when there is no edge to cut.
    fn begin_loop_cut(
        &mut self,
        drawing: &SceneDrawing,
        pointer: ViewPointer,
    ) -> Option<LoopCutSession> {
        let Some(target) = self.loop_target(drawing, pointer) else {
            self.note = Some(
                "Loop cut: point at an edge of a selected object, across four-sided polygons."
                    .into(),
            );
            return None;
        };
        self.sessions += 1;
        let session = LoopCutSession {
            key: LOOP_CUT_KEYS | self.sessions,
            start: pointer.at,
            start_share: target.share_at(&pointer, pointer.at).unwrap_or(0.5),
            start_position: target.position,
            cuts: self.attributes.loop_cut.cuts,
            target,
        };
        self.show_loop_cut(&session);
        Some(session)
    }

    /// Shows the session's cut, saying where it is beside the mouse.
    pub(super) fn show_loop_cut(&mut self, session: &LoopCutSession) {
        let percent = (session.target.position * 100.0).round();
        let text = format!("{}: {percent}%", label(session.cuts));
        match session.show(&mut self.document) {
            Ok(()) => self.viewport.set_tool_label(Some(text)),
            Err(why) => self.note = Some(format!("Couldn't do that: {why}.")),
        }
    }
}

/// "1 polygon", "4 polygons".
fn polygons(n: usize) -> String {
    if n == 1 {
        "1 polygon".into()
    } else {
        format!("{n} polygons")
    }
}
