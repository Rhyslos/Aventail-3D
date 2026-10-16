//! The workbench's part in the modelling tools set by dragging anywhere in
//! a view: Extrude and Inset ([decision 0038](../../../../docs/decisions/0038-extrude-and-inset.md))
//! and Bevel ([decision 0040](../../../../docs/decisions/0040-bevel.md)).
//!
//! A drag is one undo step, as with the gizmos: each frame takes back the
//! frame before (`Document::abandon`) and records the edit again with the
//! new distance. Letting go at no distance leaves nothing behind.

use super::Workbench;
use crate::attributes::BevelLook;
use crate::commands::CommandId;
use crate::components::{self, ViewPointer};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::transforming::Tool;
use crate::viewport::ToolDragEvent;
use egui::Ui;
use glam::{Mat4, Vec2, Vec3};
use mesh::Anchors;
use scene::{
    BevelShape, Bounds, Command, Document, EdgeKey, ElementKind, FaceId, Grouping, ObjectId,
    Operation, VertexId,
};
use std::collections::{HashMap, HashSet};

/// Ctrl + M: Weld onto a target (decision 0051).
pub(crate) const WELD_TARGET_KEYS: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::M);

/// Merge keys for modelling tool drags: kept apart from other merge keys.
const MODELLING_KEYS: u64 = 1 << 60;

/// What a drag works on, in one object.
#[derive(Debug, Clone, PartialEq)]
enum Elements {
    Polygons(Vec<FaceId>),
    Edges(Vec<EdgeKey>),
    Points(Vec<VertexId>),
}

/// One object's part in a drag, as it was when the drag began.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    id: ObjectId,
    elements: Elements,
    /// How many world centimetres one of the object's own is (its scale,
    /// averaged over the three axes).
    scale: f32,
    /// How much the object is stretched along each of its own axes, for
    /// a bend that keeps its shape as seen (decision 0081).
    stretch: Vec3,
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
    /// Bevel's rows and profile.
    bevel: BevelLook,
    /// Groove's width and sides (decision 0086).
    groove: crate::attributes::GrooveLook,
    /// Extrude's bend (decision 0050).
    bend: scene::Bend,
    /// Extrude's way, end size, steps and flat end (decision 0070).
    extrude: scene::ExtrudeShape,
    /// For Bevel, the selected edges (or points, as edges of no length) in
    /// the world: the drag is measured away from them.
    reach: Vec<(Vec3, Vec3)>,
    /// Where the drag started, in the view's screen points.
    start: Option<Vec2>,
    /// The distance last shown, in world centimetres.
    shown: Option<f32>,
}

/// How long a world step is measured on screen, in points, to see how much
/// of the extrude direction lies across the screen.
const PROBE: f32 = 50.0;

/// Below this part of the direction lying across the screen (it points
/// almost straight at the viewer), dragging right means "more" instead.
const MIN_ACROSS: f32 = 0.2;

/// Closer than this to the middle of the polygons (in screen points), an
/// Inset drag has no way to follow, and right means more.
const MIN_TO_CENTRE: f32 = 8.0;

impl ModellingSession {
    /// The distance (world centimetres) a drag of `drag` screen points
    /// means, seen through `pointer`'s camera. Drags follow the pull
    /// (decision 0039):
    ///
    /// - **Extrude** counts the drag along the way the polygons face on
    ///   screen, scaled so they follow the mouse: drag a polygon on the
    ///   left side to the left and it comes out.
    /// - **Inset** counts the drag towards the middle of the polygons on
    ///   screen, from where the drag started: pull towards it and they
    ///   shrink, back out and they grow again.
    /// - When there is no way to follow (the polygons face the viewer
    ///   straight on, or the drag started on their middle), right means
    ///   more.
    fn distance(&self, drag: Vec2, pointer: &ViewPointer) -> f32 {
        let camera = &pointer.camera;
        let per_point = camera.world_per_pixel(self.pivot, pointer.size);
        if self.tool == Tool::Bevel
            && let Some(start) = self.start
        {
            // Bevel follows the pull away from the selected edges: how
            // much further from the nearest of them the mouse is than
            // where the drag began.
            let away = |at: Vec2| -> Option<f32> {
                self.reach
                    .iter()
                    .filter_map(|(a, b)| {
                        let a = camera.project(*a, pointer.size)?;
                        let b = camera.project(*b, pointer.size)?;
                        Some(to_segment(at, a, b))
                    })
                    .reduce(f32::min)
            };
            if let (Some(from), Some(now)) = (away(start), away(start + drag)) {
                return (now - from) * per_point;
            }
        }
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
        if self.tool == Tool::Inset
            && let (Some(start), Some(centre)) =
                (self.start, camera.project(self.pivot, pointer.size))
        {
            // Inset follows the pull towards the middle of the polygons:
            // drag towards it and the border follows the mouse inward.
            let way = centre - start;
            if way.length() > MIN_TO_CENTRE {
                return drag.dot(way / way.length()) * per_point;
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
                stretch: part.stretch,
            },
            (Elements::Polygons(faces), _) if self.bend.bends() => Operation::ExtrudeBent {
                faces: faces.clone(),
                // The straight part after the bend goes only outward.
                distance: local.max(0.0),
                grouping: self.grouping,
                bend: self.bend,
                stretch: part.stretch,
                anchors,
            },
            (Elements::Polygons(faces), _) => Operation::ExtrudePolygons {
                faces: faces.clone(),
                distance: local,
                grouping: self.grouping,
                shape: self.extrude,
                anchors,
                stretch: part.stretch,
            },
            (Elements::Edges(edges), Tool::Bevel) => Operation::BevelEdges {
                edges: edges.clone(),
                shape: BevelShape {
                    width: local,
                    segments: self.bevel.segments,
                    profile: self.bevel.profile,
                    options: self.bevel.options,
                },
                anchors,
                stretch: part.stretch,
            },
            (Elements::Edges(edges), Tool::Groove) => Operation::Groove {
                edges: edges.clone(),
                shape: scene::GrooveShape {
                    width: self.groove.width / part.scale,
                    depth: local,
                    sides: self.groove.sides,
                },
                anchors,
                stretch: part.stretch,
            },
            (Elements::Points(points), _) => Operation::BevelPoints {
                points: points.clone(),
                width: local,
                anchors,
                stretch: part.stretch,
            },
            (Elements::Edges(edges), _) => Operation::ExtrudeEdges {
                edges: edges.clone(),
                distance: local,
                grouping: self.grouping,
                anchors,
                stretch: part.stretch,
            },
        }
    }

    /// The undo step's name: "Extrude 4 polygons".
    fn label(&self) -> String {
        let (polygons, edges, points) =
            self.parts
                .iter()
                .fold((0, 0, 0), |(p, e, v), part| match &part.elements {
                    Elements::Polygons(f) => (p + f.len(), e, v),
                    Elements::Edges(x) => (p, e + x.len(), v),
                    Elements::Points(x) => (p, e, v + x.len()),
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
        } else if points > 0 {
            count(points, "point")
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
    /// D, I, V, F, X, A, C, M and J: Extrude, Inset, Bevel, Loop cut,
    /// Slit, Connect, Bridge, Weld and Fill hole, from the command list,
    /// and Ctrl + M for welding onto a target. Not while typing.
    pub(super) fn modelling_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.renaming() {
            return;
        }
        if ui.input_mut(|i| i.consume_shortcut(&WELD_TARGET_KEYS)) {
            if self.mode == crate::palette::Mode::Points {
                self.set_tool(Tool::Weld);
            } else {
                self.note = Some("Weld works on points: press 1 for the Points mode.".into());
            }
        }
        for command in [
            CommandId::Extrude,
            CommandId::Inset,
            CommandId::Bevel,
            CommandId::Groove,
            CommandId::LoopCut,
            CommandId::Slit,
            CommandId::Connect,
            CommandId::Bridge,
            CommandId::Weld,
            CommandId::FillHole,
        ] {
            let pressed = command
                .info()
                .shortcut
                .is_some_and(|keys| ui.input_mut(|i| i.consume_shortcut(&keys)));
            if pressed {
                self.run_command(command, ui.ctx(), &mut Vec::new());
            }
        }
    }

    /// Tells the viewport whether drags (and clicks) go to a modelling
    /// tool, and the element tracker whether the tool finds its own.
    pub(super) fn place_drag_tool(&mut self) {
        // Tools used on the shape work only while editing points, edges or
        // polygons; in the other modes, clicks select objects as usual.
        let editing = components::kind_for(self.mode).is_some();
        let clicks = self.tool.clicks_on_shapes() && editing;
        // Bridge and Weld take a drag only when it starts on a highlighted
        // point or edge (or one is being dragged), so clicks and drags from
        // empty space still select.
        let pairing = self.tool.drags_between_elements()
            && editing
            && (self.components.hovered().is_some() || self.pair_session.is_some());
        self.viewport
            .set_wheel_tool(self.tool.has_wheel_count() && editing);
        self.viewport
            .set_drag_tool(self.tool.drags_anywhere() || pairing || self.edits_uvs_on_model());
        self.viewport.set_click_tool(clicks);
        self.components.set_quiet(clicks);
    }

    /// The grouping the modelling tools use: neighbours together, or each
    /// on its own (the Attributes panel's Keep together switch).
    pub(crate) fn grouping(&self) -> Grouping {
        self.attributes.grouping
    }

    /// Carries out this frame's modelling tool drags.
    pub(super) fn carry_out_tool_drag(&mut self, drawing: &SceneDrawing, keys: egui::Modifiers) {
        let steps = self.viewport.take_tool_steps();
        if steps != 0 {
            self.wheel_count(steps);
        }
        if self.edits_uvs_on_model() || self.uv_model_drag.is_some() {
            return self.carry_out_uv_model_drag(drawing, keys.command);
        }
        if matches!(self.tool, Tool::Bridge | Tool::Weld) || self.pair_session.is_some() {
            return self.carry_out_pairing(drawing);
        }
        if self.tool == Tool::Slit || self.slit_session.is_some() {
            return self.carry_out_slit(drawing, keys);
        }
        if self.tool == Tool::LoopCut || self.loop_cut_session.is_some() {
            return self.carry_out_loop_cut(drawing);
        }
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { pointer } => {
                    self.select_under_the_press();
                    self.modelling_session = self.begin_modelling(drawing);
                    if let Some(session) = &mut self.modelling_session {
                        session.start = Some(pointer.at);
                    }
                }
                ToolDragEvent::Update { drag, pointer } => {
                    let Some(session) = &mut self.modelling_session else {
                        continue;
                    };
                    let mut distance = session.distance(drag, &pointer);
                    if matches!(session.tool, Tool::Inset | Tool::Bevel) {
                        // Inset inward only (outward would overlap the
                        // neighbours); a bevel has no negative width.
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
                        if session.tool == Tool::Extrude {
                            // The next extrude starts straight: a turn
                            // left over would surprise. This one keeps its
                            // bend (still in Last edit).
                            self.attributes.bend.angle = 0.0;
                        }
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

    /// Ctrl + wheel while a tool with a count is in use ([decision 0048](../../../../docs/decisions/0048-ctrl-wheel-changes-a-tools-count.md)):
    /// `steps` notches up (more) or down (fewer), doing what the arrows
    /// beside that count in the Attributes panel do. The count is changed
    /// for the tool's next use and on the edit being dragged; with no drag,
    /// also on the last edit when the tool made it (the panel shows it as
    /// the Last edit then), as one undo step per notch.
    fn wheel_count(&mut self, steps: i32) {
        let dragging = self.modelling_session.is_some() || self.loop_cut_session.is_some();
        // The last edit the tool made, on the one selected object.
        let last = match self.document.selection().as_slice() {
            [id] if !dragging => self.document.scene().object(*id).and_then(|o| {
                let (layer, index, op) = crate::attributes::last_edit(&o.stack)?;
                crate::attributes::made_by(op, self.tool).then(|| (*id, layer, index, op.clone()))
            }),
            _ => None,
        };
        // Extrude's count is its bend's segments while it bends, and its
        // steps otherwise (decision 0070).
        let bends = match (&self.modelling_session, &last) {
            (Some(session), _) => session.bend.bends(),
            (None, Some((.., op))) => matches!(op.inner(), Operation::ExtrudeBent { .. }),
            (None, None) => self.attributes.bend.bends(),
        };
        let (name, max, tool_count) = match self.tool {
            Tool::Bevel => (
                "segments",
                mesh::bevel::MAX_SEGMENTS,
                self.attributes.bevel.segments,
            ),
            Tool::LoopCut => (
                "cuts",
                mesh::loopcut::MAX_CUTS,
                self.attributes.loop_cut.cuts,
            ),
            Tool::Extrude if bends => (
                "bend segments",
                mesh::extrude::MAX_BEND_SEGMENTS,
                self.attributes.bend.segments,
            ),
            Tool::Extrude => (
                "steps",
                mesh::edit::MAX_EXTRUDE_STEPS,
                self.attributes.extrude.steps,
            ),
            Tool::Bridge => (
                "segments",
                mesh::join::MAX_BRIDGE_SEGMENTS,
                self.attributes.join.bridge.0,
            ),
            _ => return,
        };
        let count_of = |op: &Operation| {
            let settings = op.settings()?;
            match (self.tool, op.inner()) {
                (Tool::Bevel, _) => settings.segments,
                // Only a bent extrude has its own segments; a straight one
                // shows the tool's.
                (Tool::Extrude, Operation::ExtrudeBent { .. }) => settings.bend.map(|b| b.segments),
                (Tool::Extrude, Operation::ExtrudePolygons { .. }) => {
                    settings.extrude.map(|e| e.steps)
                }
                (Tool::Extrude, _) => None,
                (Tool::Bridge, _) => settings.bridge.map(|b| b.0),
                _ => settings.cuts,
            }
        };
        let current = last
            .as_ref()
            .and_then(|(.., op)| count_of(op))
            .unwrap_or(tool_count);
        let count = (i64::from(current) + i64::from(steps)).clamp(1, i64::from(max)) as u32;
        match self.tool {
            Tool::Bevel => self.attributes.bevel.segments = count,
            Tool::Extrude if bends => self.attributes.bend.segments = count,
            Tool::Extrude => self.attributes.extrude.steps = count,
            Tool::Bridge => self.attributes.join.bridge.0 = count,
            _ => self.attributes.loop_cut.cuts = count,
        }
        self.note = Some(format!(
            "{}: {count} {name} (Ctrl + wheel for more or fewer).",
            self.tool.name()
        ));
        if let Some(session) = &mut self.modelling_session {
            session.bevel.segments = count;
            if bends {
                session.bend.segments = count;
            } else {
                session.extrude.steps = count;
            }
            if let Some(distance) = session.shown {
                let session = session.clone();
                session.show(&mut self.document, distance).ok();
            }
        } else if let Some(session) = &mut self.loop_cut_session {
            session.cuts = count;
            let session = session.clone();
            self.show_loop_cut(&session);
        } else if let Some((id, layer, index, op)) = last
            && count_of(&op).is_some_and(|c| c != count)
        {
            let settings = op.settings().unwrap_or_default();
            let operation = op
                .with(match self.tool {
                    Tool::Bevel => scene::EditSettings {
                        segments: Some(count),
                        ..settings
                    },
                    Tool::Extrude if bends => scene::EditSettings {
                        bend: settings.bend.map(|b| scene::Bend {
                            segments: count,
                            ..b
                        }),
                        ..settings
                    },
                    Tool::Extrude => scene::EditSettings {
                        extrude: settings
                            .extrude
                            .map(|e| scene::ExtrudeShape { steps: count, ..e }),
                        ..settings
                    },
                    Tool::Bridge => scene::EditSettings {
                        bridge: settings.bridge.map(|(_, t)| (count, t)),
                        ..settings
                    },
                    _ => scene::EditSettings {
                        cuts: Some(count),
                        ..settings
                    },
                })
                // Worked out as the object is stretched now (decisions 0081
                // and 0082).
                .with_stretch(scene::stretch_of(self.document.scene().shape_matrix(id)));
            let command = Command::ReplaceOperation {
                id,
                layer,
                index,
                operation,
            };
            if let Err(why) = self.document.execute(command) {
                self.note = Some(format!("Couldn't do that: {why}."));
            }
        }
    }

    /// Starts a drag of the modelling tool in use, on the selected
    /// polygons (or, for Extrude in the Edges mode, edges). Says in the
    /// status bar why not, when there is nothing to work on.
    fn begin_modelling(&mut self, drawing: &SceneDrawing) -> Option<ModellingSession> {
        let tool = self.tool;
        let kind = match kind_for(self.mode, tool) {
            Some(kind) => kind,
            None => {
                self.note = Some(match tool {
                    Tool::Bevel => {
                        "Bevel works on edges or points: press 2 for the Edges mode (or 1 for \
                         Points) and select some."
                    }
                    Tool::Inset => {
                        "Inset works on polygons: press 3 for the Polygons mode and select some."
                    }
                    Tool::Groove => {
                        "Groove works on edges: press 2 for the Edges mode and select some."
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
        // The extrude's axis in the world, for Along X, Y or Z.
        let axis = self.attributes.extrude.direction.axis();
        let mut along = Vec3::ZERO;
        let mut reach = Vec::new();
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
                ElementKind::Points => {
                    let points: Vec<VertexId> = valid.points().collect();
                    let corners = points.iter().map(|v| vec![*v]).collect();
                    (Elements::Points(points), corners)
                }
                ElementKind::Edges => {
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
            facing += if tool == Tool::Groove {
                // A groove goes into the surface the edges lie on.
                surface_facing(&item.mesh, item.transform, &elements)
            } else {
                world_facing(&item.mesh, item.transform, &elements)
            };
            if let Some(axis) = axis {
                along += item.transform.transform_vector3(axis).normalize_or_zero();
            }
            for c in &corners {
                let world = |v: &VertexId| {
                    item.mesh
                        .position(*v)
                        .map(|p| item.transform.transform_point3(p))
                };
                if let (Some(a), Some(b)) = (c.first().and_then(world), c.last().and_then(world)) {
                    reach.push((a, b));
                }
            }
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
                stretch: scene::stretch_of(item.transform),
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
            direction: matches!(tool, Tool::Extrude | Tool::Groove)
                .then(|| match (axis, along.try_normalize()) {
                    // Along an axis, out the way the polygons face (and so
                    // dragging that way on screen pushes them out).
                    (Some(_), Some(along)) if !self.attributes.extrude_bends() => {
                        if along.dot(facing) < 0.0 {
                            Some(-along)
                        } else {
                            Some(along)
                        }
                    }
                    _ => facing.try_normalize(),
                })
                .flatten(),
            grouping: self.grouping(),
            bevel: self.attributes.bevel,
            groove: self.attributes.groove,
            bend: self.attributes.bend,
            extrude: self.attributes.extrude,
            reach,
            start: None,
            shown: None,
        })
    }

    /// When a drag starts on a polygon (or, for Extrude in the Edges mode,
    /// an edge) of a selected object that isn't selected, it becomes the
    /// selection first, as its own undo step: drag straight on the
    /// polygon you want, without clicking it first. Starting on something
    /// selected, or on empty space, keeps the selection.
    fn select_under_the_press(&mut self) {
        let Some(kind) = kind_for(self.mode, self.tool) else {
            return;
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
        if session.tool != Tool::Extrude {
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
/// of the newest polygon along it. Looks from the newest polygon back and
/// stops once every edge is found, so a heavy shape's connections don't
/// have to be worked out while the drag ends.
fn outer_edges(mesh: &scene::Mesh, edges: &[EdgeKey]) -> Vec<EdgeKey> {
    let mut newest: HashMap<EdgeKey, Option<EdgeKey>> = HashMap::new();
    let wanted: HashSet<EdgeKey> = edges.iter().copied().collect();
    for corners in mesh.face_slots().rev().flatten() {
        if newest.len() == wanted.len() {
            break;
        }
        let n = corners.len();
        for i in 0..n {
            let side = EdgeKey::new(corners[i], corners[(i + 1) % n]);
            if wanted.contains(&side) && !newest.contains_key(&side) {
                let outer = match corners {
                    [_, _, c, d] => Some(EdgeKey::new(*c, *d)),
                    _ => None,
                };
                newest.insert(side, outer);
            }
        }
    }
    edges
        .iter()
        .filter_map(|edge| newest.get(edge).copied().flatten())
        .collect()
}

/// The way the surface around `elements` of `mesh` faces, summed and in
/// the world: for edges, both polygons along each.
fn surface_facing(mesh: &scene::Mesh, world: Mat4, elements: &Elements) -> Vec3 {
    let normals = world.inverse().transpose();
    let to_world = |v: Vec3| normals.transform_vector3(v).normalize_or_zero();
    let Elements::Edges(edges) = elements else {
        return world_facing(mesh, world, elements);
    };
    let topology = mesh.topology();
    edges
        .iter()
        .flat_map(|e| topology.edge_uses(*e).iter().map(|u| u.face))
        .map(|f| to_world(mesh.face_normal(f)))
        .sum()
}

/// The way `elements` of `mesh` face, summed and in the world (`world` is
/// the object's matrix): polygons by their facing, border edges by the way
/// they grow.
fn world_facing(mesh: &scene::Mesh, world: Mat4, elements: &Elements) -> Vec3 {
    let normals = world.inverse().transpose();
    let to_world = |v: Vec3| normals.transform_vector3(v).normalize_or_zero();
    match elements {
        Elements::Polygons(faces) => faces.iter().map(|f| to_world(mesh.face_normal(*f))).sum(),
        Elements::Points(_) => Vec3::ZERO,
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

/// What the modelling tool in use works on in `mode`, if anything.
fn kind_for(mode: Mode, tool: Tool) -> Option<ElementKind> {
    match (mode, tool) {
        (Mode::Polygons, Tool::Extrude | Tool::Inset) => Some(ElementKind::Polygons),
        (Mode::Edges, Tool::Extrude | Tool::Bevel | Tool::Groove) => Some(ElementKind::Edges),
        (Mode::Points, Tool::Bevel) => Some(ElementKind::Points),
        _ => None,
    }
}

/// How far `at` is from the segment from `a` to `b` (all on screen).
fn to_segment(at: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let t = if along.length_squared() > 1e-9 {
        ((at - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    at.distance(a + along * t)
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
