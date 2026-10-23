//! Editing UVs on the model ([decision 0103](../../../../docs/decisions/0103-uvs-on-the-model.md)):
//! in the UV mode, the move, rotate and scale tools drag the selected
//! islands' UVs right on the shape in the viewport, so the texture (with
//! the Checker view, say) slides, turns and grows under the mouse. What
//! moves is the texture as seen on the model, so the UVs move the other
//! way (a texture slid right means its UVs moved left).
//!
//! The same keys as everywhere (decision 0095): W, R, S and T, then drag
//! anywhere in the viewport. A press on an island that isn't selected
//! selects it first. The drag is shown live and is one undo step; Shift
//! makes it finer, Esc or a right click cancels. How a move on screen
//! becomes a move in the texture is read from the islands themselves: the
//! best straight-line fit from where their corners are on screen to where
//! they are in the texture, so the texture follows the mouse on the
//! surface you are looking at.

use super::Workbench;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::transforming::Tool;
use crate::uv_editor::gizmo::{self, UvTarget};
use crate::viewport::ToolDragEvent;
use glam::{Affine2, Mat2, Vec2};
use scene::{Element, ElementKind};

/// Marks on-model UV drags' undo merge keys apart from every other.
const ON_MODEL_KEY: u64 = 0x5556_4d44_0000_0000;

/// How fast a drag turns (radians per screen point) and grows (a power of
/// e per screen point).
const TURN_PER_POINT: f32 = 0.01;
const GROW_PER_POINT: f32 = 0.005;

/// An on-model drag in progress.
#[derive(Debug, Clone)]
pub(super) struct UvModelDrag {
    tool: Tool,
    targets: Vec<UvTarget>,
    /// The middle of the islands' UVs, which turning and scaling go
    /// about.
    pivot: Vec2,
    /// Screen points to texture: the islands' fit.
    to_texture: Mat2,
    key: u64,
    shown: Option<Affine2>,
}

impl UvModelDrag {
    /// The change for a drag of `drag` screen points (down is positive).
    fn change(&self, drag: Vec2, snap: Option<&crate::transforming::SnapSettings>) -> Affine2 {
        match self.tool {
            Tool::Rotate => {
                // A mirrored fit turns the other way, so the texture
                // always turns the way the mouse goes.
                let way = if self.to_texture.determinant() < 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let mut angle = drag.x * TURN_PER_POINT * way;
                if let Some(s) = snap {
                    let step = s.angle.to_radians();
                    if step > 0.0 {
                        angle = (angle / step).round() * step;
                    }
                }
                gizmo::about(self.pivot, Affine2::from_angle(angle))
            }
            Tool::Scale => {
                let mut grow = (drag.x * GROW_PER_POINT).exp() - 1.0;
                if let Some(s) = snap
                    && s.scale > 0.0
                {
                    grow = (grow / s.scale).round() * s.scale;
                }
                // Dragging right makes the texture look bigger on the
                // model: the UVs shrink.
                let k = (1.0 + grow).max(1e-3);
                gizmo::about(self.pivot, Affine2::from_scale(Vec2::splat(1.0 / k)))
            }
            // The texture follows the mouse: the UVs move the other way.
            _ => Affine2::from_translation(-(self.to_texture * drag)),
        }
    }
}

/// The best linear fit taking screen points to texture points, from pairs
/// of (screen, texture); `None` without enough spread to tell.
fn fit(pairs: &[(Vec2, Vec2)]) -> Option<Mat2> {
    if pairs.len() < 3 {
        return None;
    }
    let n = pairs.len() as f32;
    let (ms, mt) = pairs
        .iter()
        .fold((Vec2::ZERO, Vec2::ZERO), |(a, b), (s, t)| (a + *s, b + *t));
    let (ms, mt) = (ms / n, mt / n);
    // Least squares: M = (Σ t sᵀ)(Σ s sᵀ)⁻¹ about the means.
    let mut ss = Mat2::ZERO;
    let mut ts = Mat2::ZERO;
    for (s, t) in pairs {
        let (s, t) = (*s - ms, *t - mt);
        ss += Mat2::from_cols(s * s.x, s * s.y);
        ts += Mat2::from_cols(t * s.x, t * s.y);
    }
    if ss.determinant().abs() < 1e-6 {
        return None;
    }
    let m = ts * ss.inverse();
    m.is_finite().then_some(m)
}

impl Workbench {
    /// Whether viewport drags edit UVs on the model now: the UV mode with
    /// a move, rotate or scale tool.
    pub(super) fn edits_uvs_on_model(&self) -> bool {
        self.mode == Mode::Uv
            && matches!(
                self.tool,
                Tool::Move | Tool::Rotate | Tool::Scale | Tool::All
            )
    }

    /// Selects the island under the press, if it isn't selected.
    fn select_island_under_the_press(&mut self) {
        let Some((id, Element::Polygon(face))) = self.components.hovered() else {
            return;
        };
        if !self.document.selection().contains(&id)
            || self
                .document
                .elements(id)
                .is_some_and(|s| s.contains(Element::Polygon(face)))
        {
            return;
        }
        let Some(mesh) = self.document.scene().evaluated(id).map(|e| e.mesh) else {
            return;
        };
        let island = mesh::uvs::islands(&mesh)
            .into_iter()
            .find(|i| i.contains(&face))
            .unwrap_or_else(|| vec![face]);
        let changes: Vec<_> = self
            .document
            .selection()
            .into_iter()
            .map(|object| {
                let mut selection = self.document.elements(object).cloned().unwrap_or_default();
                selection.clear_kind(ElementKind::Polygons);
                if object == id {
                    for f in &island {
                        selection.insert(Element::Polygon(*f));
                    }
                }
                (object, selection)
            })
            .collect();
        self.document
            .select_elements(changes, tr("Select pieces"), None);
    }

    /// The drag's start: what it moves, about where, and how screen
    /// points map to the texture.
    fn begin_uv_model_drag(
        &mut self,
        drawing: &SceneDrawing,
        pointer: &crate::components::ViewPointer,
    ) -> Option<UvModelDrag> {
        let (targets, bounds) = gizmo::uv_targets(&self.document, Mode::Uv);
        let (min, max) = bounds?;
        let mut pairs = Vec::new();
        for target in &targets {
            let Some(item) = drawing.items.iter().find(|i| i.id == target.id) else {
                continue;
            };
            let mesh = &item.mesh;
            let Some(uvs) = mesh::uvs::Uvs::of(mesh) else {
                continue;
            };
            let (starts, _) = mesh.face_layout();
            // Up to a few thousand corners are plenty for the fit.
            let every = (target.faces.len() / 1000).max(1);
            for f in target.faces.iter().step_by(every) {
                let Some(list) = mesh.face(*f) else {
                    continue;
                };
                let start = starts[f.get() as usize] as usize;
                for (k, v) in list.iter().enumerate() {
                    let Some(p) = mesh.position(*v) else {
                        continue;
                    };
                    let world = item.transform.transform_point3(p);
                    if let Some(s) = pointer.camera.project(world, pointer.size) {
                        pairs.push((s, uvs.at(start + k)));
                    }
                }
            }
        }
        let to_texture = fit(&pairs)?;
        self.sessions += 1;
        Some(UvModelDrag {
            tool: self.tool,
            targets,
            pivot: (min + max) / 2.0,
            to_texture,
            key: ON_MODEL_KEY | self.sessions,
            shown: None,
        })
    }

    /// Carries out this frame's on-model UV drags.
    pub(super) fn carry_out_uv_model_drag(&mut self, drawing: &SceneDrawing, ctrl: bool) {
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { pointer } => {
                    self.select_island_under_the_press();
                    self.uv_model_drag = self.begin_uv_model_drag(drawing, &pointer);
                    if self.uv_model_drag.is_none() {
                        self.note = Some(
                            tr("Select the islands to move first (click one on the model or in the \
                             UV Editor).")
                                .into(),
                        );
                    }
                }
                ToolDragEvent::Update { drag, .. } => {
                    let snap_settings = self.viewport.options().snap;
                    let snap = snap_settings.active(ctrl).then_some(&snap_settings);
                    let Some(session) = &mut self.uv_model_drag else {
                        continue;
                    };
                    let change = session.change(drag, snap);
                    if session.shown == Some(change) {
                        continue;
                    }
                    if session.shown.is_some() && !self.document.abandon(session.key) {
                        self.uv_model_drag = None;
                        continue;
                    }
                    session.shown = None;
                    let label = match session.tool {
                        Tool::Rotate => tr("Rotate UVs"),
                        Tool::Scale => tr("Scale UVs"),
                        _ => tr("Move UVs"),
                    };
                    if change != Affine2::IDENTITY {
                        match gizmo::record(
                            &mut self.document,
                            &session.targets,
                            change,
                            label,
                            session.key,
                        ) {
                            Ok(()) => session.shown = Some(change),
                            Err(why) => self.note = Some(crate::trf!("Couldn't do that: {}.", why)),
                        }
                    }
                    self.viewport.set_tool_label(Some(label.to_string()));
                }
                ToolDragEvent::End => {
                    if self.uv_model_drag.take().is_some_and(|s| s.shown.is_some()) {
                        self.note = Some(tr("Done. Ctrl + Z takes it back.").into());
                    }
                    self.viewport.set_tool_label(None);
                }
                ToolDragEvent::Cancel => {
                    if let Some(session) = self.uv_model_drag.take()
                        && session.shown.is_some()
                    {
                        self.document.abandon(session.key);
                        self.note = Some(tr("Cancelled: the UVs are back where they were.").into());
                    }
                    self.viewport.set_tool_label(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fit_finds_a_turned_and_scaled_map() {
        let m = Mat2::from_cols(Vec2::new(0.0, -0.01), Vec2::new(0.02, 0.0));
        let pairs: Vec<(Vec2, Vec2)> = [(0.0, 0.0), (100.0, 0.0), (0.0, 50.0), (80.0, 40.0)]
            .into_iter()
            .map(|(x, y)| {
                let s = Vec2::new(x, y);
                (s, m * s + Vec2::new(0.3, 0.4))
            })
            .collect();
        let found = fit(&pairs).unwrap();
        assert!((found.x_axis - m.x_axis).length() < 1e-5);
        assert!((found.y_axis - m.y_axis).length() < 1e-5);
        // All on one line: can't tell.
        let line: Vec<(Vec2, Vec2)> = (0..5)
            .map(|i| (Vec2::splat(i as f32), Vec2::splat(i as f32)))
            .collect();
        assert!(fit(&line).is_none());
        assert!(fit(&pairs[..2]).is_none());
    }
}
