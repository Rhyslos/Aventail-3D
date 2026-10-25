//! The workbench's part in Quadify now ([decision 0150](../../../../docs/decisions/0150-quadify.md)):
//! the one-off Quadify, in the Mesh menu's Clean up group and command
//! search (no key). In the Polygons mode it joins pairs among the
//! selected polygons; in the Model mode, across the selected shapes. It is
//! recorded per object as an `Operation::Quadify` holding the object's
//! Smoothing at the time, one undo step in all; its settings (remembered
//! for the next one) can be changed under Last edit.

use super::Workbench;
use super::cleaning::chosen;
use crate::commands::CommandId;
use crate::palette::Mode;
use crate::text::tr;
use crate::tool_menu::Ready;
use scene::{Operation, QuadifyOptions};

/// The loosest settings: whether there is anything at all to join.
const LOOSEST: QuadifyOptions = QuadifyOptions {
    max_angle: mesh::quadify::MAX_ANGLE,
    max_distortion: mesh::quadify::MAX_DISTORTION,
    keep_seams: false,
    keep_hard_edges: false,
    keep_materials: false,
};

impl Workbench {
    /// Whether Quadify now can be used: in the Model mode (whole shapes)
    /// or the Polygons mode (the selected polygons), with something
    /// selected.
    pub(super) fn quadify_readiness(&self) -> Ready {
        if !CommandId::QuadifyNow.modes().contains(&self.mode) {
            return Ready::NotNow(
                tr(
                    "Quadify now works on whole shapes in the Model mode, or on the selected \
                     polygons in the Polygons mode (press 3)",
                )
                .into(),
            );
        }
        if self.document.selection().is_empty() {
            return Ready::NotNow(tr("Select one or more shapes first").into());
        }
        Ready::Yes
    }

    /// The objects Quadify now works on, with their shapes: in the
    /// Polygons mode, each selected shape with its selected elements; in
    /// the Model mode, each selected shape whole (no elements). Curves'
    /// solids, instances and groups are left out.
    fn quadify_targets(
        &self,
    ) -> Vec<(
        scene::ObjectId,
        Option<scene::ElementSelection>,
        std::sync::Arc<scene::Mesh>,
    )> {
        if self.mode == Mode::Polygons {
            return self
                .selected_parts()
                .into_iter()
                .map(|(id, selected, mesh)| (id, Some(selected), mesh))
                .collect();
        }
        let scene = self.document.scene();
        self.document
            .selection()
            .into_iter()
            .filter_map(|id| {
                let kind = &scene.object(id)?.kind;
                // An instance shows its original's shape: quadify that.
                if kind.curve().is_some() || matches!(kind, scene::ObjectKind::Instance(_)) {
                    return None;
                }
                Some((id, None, scene.evaluated(id)?.mesh))
            })
            .collect()
    }

    /// Quadify now: joins pairs of triangles into quads, the squarest
    /// first, with the settings last used.
    pub(super) fn run_quadify_now(&mut self) {
        let options = self.attributes.quadify.clamped();
        let polygons = self.mode == Mode::Polygons;
        let mut ops = Vec::new();
        let mut pairs = 0;
        let mut any_chosen = false;
        for (id, selected, mesh) in self.quadify_targets() {
            let Some(hardness) = self.document.scene().object(id).map(|o| o.hardness) else {
                continue;
            };
            let (faces, anchors) = match selected {
                Some(selected) => {
                    let c = chosen(Mode::Polygons, &selected, &mesh);
                    if c.faces.is_empty() {
                        continue;
                    }
                    (c.faces, c.anchors)
                }
                None => (Vec::new(), None),
            };
            any_chosen = true;
            let within = polygons.then_some(faces.as_slice());
            let found = mesh::quadify::pairs(&mesh, within, &options, hardness).len();
            // Recorded even when these settings join nothing, as long as
            // looser ones would: its settings can then be changed under
            // Last edit. Nothing at all to join isn't recorded.
            if found == 0 && mesh::quadify::pairs(&mesh, within, &LOOSEST, hardness).is_empty() {
                continue;
            }
            pairs += found;
            ops.push((
                id,
                Operation::Quadify {
                    faces,
                    options,
                    hardness,
                    anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(if any_chosen {
                tr("Quadify now: there are no two neighbouring triangles to join.").into()
            } else if polygons {
                tr(
                    "Quadify now: select some polygons first, or press Tab for the Model mode to \
                     quadify whole shapes.",
                )
                .into()
            } else {
                tr("Quadify now: select one or more shapes first.").into()
            });
            return;
        }
        let label = match ops.as_slice() {
            [(_, op)] => op.describe(),
            _ => tr("Quadify").to_string(),
        };
        if self.record_each(&label, ops) {
            self.note = Some(if pairs == 0 {
                tr(
                    "Quadify now joined nothing with these settings: raise Max angle or Max shape \
                     distortion under Last edit, or untick a Keep setting there.",
                )
                .into()
            } else if pairs == 1 {
                tr(
                    "Joined 1 pair of triangles into a quad. Ctrl + Z takes it back; change the \
                     settings under Last edit.",
                )
                .into()
            } else {
                crate::trf!(
                    "Joined {} pairs of triangles into quads. Ctrl + Z takes it back; change \
                     the settings under Last edit.",
                    pairs
                )
            });
        }
    }
}
