//! The workbench's part in laying UVs flat ([decision 0099](../../../../docs/decisions/0099-unwrap-relax-straighten.md)):
//! Unwrap, Relax and Straighten, and unwrapping live as seams change.
//!
//! No keys (as with seams, decision 0095): the three are in the UV ring of
//! the tool menu (`E`) and the UV menu. They work on the pieces holding
//! what is selected (polygons, or the polygons around selected edges or
//! points), or on the whole of each selected object when nothing inside
//! it is selected (and in the Model and Object modes).

use super::Workbench;
use super::joining::counted;
use crate::palette::Mode;
use mesh::Anchors;
use scene::{EdgeKey, ElementKind, FaceId, ObjectId, ObjectKind, Operation, VertexId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// How UV tools behave, chosen in the UV menu and remembered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UvSettings {
    /// Whether the pieces around changed seams are unwrapped again at
    /// once (the project lead's choice, decision 0095).
    pub live_unwrap: bool,
    /// Whether strips of square quads come out as straight grids when
    /// unwrapped.
    pub straighten_strips: bool,
}

impl Default for UvSettings {
    fn default() -> Self {
        Self {
            live_unwrap: true,
            straighten_strips: true,
        }
    }
}

/// The UV menu's names for the two switches, and what they do.
pub(crate) const LIVE_UNWRAP_LABEL: &str = "Unwrap as seams change";
pub(crate) const LIVE_UNWRAP_HELP: &str = "When on, marking or clearing seams lays the pieces \
     around them flat again at once. Off: only the seam marks change, until you Unwrap.";
pub(crate) const STRAIGHT_STRIPS_LABEL: &str = "Straighten strips when unwrapping";
pub(crate) const STRAIGHT_STRIPS_HELP: &str = "When on, a piece made only of square-cornered quads \
     (a cylinder's side, a bevel's strip) unwraps as a perfectly straight grid";

/// One of the three tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvTool {
    Unwrap,
    Relax,
    Straighten,
}

impl UvTool {
    fn name(self) -> &'static str {
        match self {
            Self::Unwrap => "Unwrap",
            Self::Relax => "Relax",
            Self::Straighten => "Straighten",
        }
    }
}

/// The polygons `selected` (of `mesh`, in `mode`) points at: the selected
/// polygons, or those around selected edges or points; every polygon when
/// nothing is selected or outside the element modes.
fn targets(
    mesh: &scene::Mesh,
    selected: Option<&scene::ElementSelection>,
    mode: Mode,
) -> Vec<FaceId> {
    let topology = mesh.topology();
    let chosen: BTreeSet<FaceId> = match (selected, mode) {
        (Some(s), Mode::Polygons | Mode::Uv) => s.polygons().collect(),
        (Some(s), Mode::Edges) => s
            .edges()
            .flat_map(|e| topology.edge_faces(e).collect::<Vec<_>>())
            .collect(),
        (Some(s), Mode::Points) => s
            .points()
            .flat_map(|v| topology.vertex_faces(v).iter().copied())
            .collect(),
        _ => BTreeSet::new(),
    };
    if chosen.is_empty() {
        mesh.faces().map(|(f, _)| f).collect()
    } else {
        chosen.into_iter().collect()
    }
}

/// Anchors for `faces` of `mesh`.
fn anchors(mesh: &scene::Mesh, faces: &[FaceId]) -> Option<Anchors> {
    let places: Vec<Vec<VertexId>> = faces
        .iter()
        .filter_map(|f| mesh.face(*f).map(<[VertexId]>::to_vec))
        .collect();
    Anchors::record(mesh, ElementKind::Polygons, &places)
}

impl Workbench {
    /// The UV settings, for remembering between sessions.
    pub fn uv_settings(&self) -> UvSettings {
        self.uv_settings
    }

    /// The workbench with remembered UV settings.
    #[must_use]
    pub fn with_uv_settings(mut self, settings: UvSettings) -> Self {
        self.uv_settings = settings;
        self
    }

    /// The selected objects with a shape of their own (not instances).
    pub(super) fn own_shapes(&self) -> Vec<(ObjectId, std::sync::Arc<scene::Mesh>)> {
        let scene = self.document.scene();
        self.document
            .selection()
            .into_iter()
            .filter(|id| {
                scene.object(*id).is_some_and(|o| {
                    matches!(o.kind, ObjectKind::Primitive(_) | ObjectKind::Mesh(_))
                })
            })
            .filter_map(|id| Some((id, scene.evaluated(id)?.mesh)))
            .collect()
    }

    /// The operation `tool` makes on `faces` of `mesh`.
    fn uv_tool_op(&self, tool: UvTool, mesh: &scene::Mesh, faces: Vec<FaceId>) -> Operation {
        let anchors = anchors(mesh, &faces);
        match tool {
            UvTool::Unwrap => Operation::Unwrap {
                faces,
                strips: self.uv_settings.straighten_strips,
                anchors,
            },
            UvTool::Relax => Operation::RelaxUvs { faces, anchors },
            UvTool::Straighten => Operation::StraightenUvs { faces, anchors },
        }
    }

    /// Unwrap, Relax or Straighten on what is selected, as one undo step.
    pub(crate) fn run_uv_tool(&mut self, tool: UvTool) {
        let shapes = self.own_shapes();
        if shapes.is_empty() {
            self.note = Some(format!("{}: select the objects first.", tool.name()));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        let mut quads_only = true;
        for (id, mesh) in shapes {
            let faces = targets(&mesh, self.document.elements(id), self.mode);
            if faces.is_empty() {
                continue;
            }
            quads_only &= faces
                .iter()
                .all(|f| mesh.face(*f).is_some_and(|l| l.len() == 4));
            count += faces.len();
            ops.push((id, self.uv_tool_op(tool, &mesh, faces)));
        }
        if ops.is_empty() {
            self.note = Some(format!(
                "{}: the selected objects have no polygons.",
                tool.name()
            ));
            return;
        }
        let verb = match tool {
            UvTool::Unwrap => "Unwrap",
            UvTool::Relax => "Relax the UVs of",
            UvTool::Straighten => "Straighten the UVs of",
        };
        let label = counted(verb, count, "polygon");
        if self.record_each(&label, ops) {
            let extra = match tool {
                UvTool::Straighten if !quads_only => {
                    " Only pieces made of quads are straightened; others are left as they were."
                }
                _ => "",
            };
            self.note = Some(format!("{label}. Ctrl + Z takes it back.{extra}"));
        }
    }

    /// With live unwrapping on, the Unwrap that follows marking or clearing
    /// `edges` on `mesh`: the pieces either side of them, laid flat again.
    pub(super) fn live_unwrap(&self, mesh: &scene::Mesh, edges: &[EdgeKey]) -> Option<Operation> {
        if !self.uv_settings.live_unwrap || edges.is_empty() {
            return None;
        }
        let topology = mesh.topology();
        let faces: BTreeSet<FaceId> = edges
            .iter()
            .flat_map(|e| topology.edge_faces(*e).collect::<Vec<_>>())
            .collect();
        (!faces.is_empty())
            .then(|| self.uv_tool_op(UvTool::Unwrap, mesh, faces.into_iter().collect()))
    }
}
