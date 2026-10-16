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
    /// Packing (decision 0101): the gap between pieces, in texture pixels.
    pub padding: u32,
    /// Whether packing may turn pieces.
    pub rotate: bool,
    /// Whether packing stacks identical pieces on one another.
    pub stack: bool,
    /// Whether each material gets a texture of its own (packed apart).
    pub per_material: bool,
    /// Whether each object gets a texture of its own (packed apart).
    pub per_object: bool,
}

impl Default for UvSettings {
    fn default() -> Self {
        Self {
            live_unwrap: true,
            straighten_strips: true,
            padding: 8,
            rotate: true,
            stack: false,
            per_material: true,
            per_object: false,
        }
    }
}

/// The most padding packing takes, in pixels.
pub(crate) const MAX_PADDING: u32 = 256;

/// The UV menu's names for the two switches, and what they do.
pub(crate) const LIVE_UNWRAP_LABEL: &str = "Unwrap as seams change";
pub(crate) const LIVE_UNWRAP_HELP: &str = "When on, marking or clearing seams lays the pieces \
     around them flat again at once. Off: only the seam marks change, until you Unwrap.";
pub(crate) const STRAIGHT_STRIPS_LABEL: &str = "Straighten strips when unwrapping";
pub(crate) const STRAIGHT_STRIPS_HELP: &str = "When on, a piece made only of square-cornered quads \
     (a cylinder's side, a bevel's strip) unwraps as a perfectly straight grid";

/// Which texture a packed island goes on: the object's (when each gets
/// its own) and the material's name (when each gets its own).
pub(crate) type TextureGroup = (Option<ObjectId>, String);

/// The label of the packing padding field.
pub(crate) const PADDING_LABEL: &str = "Padding";

/// The UV menu's names for setting and matching texel density, and what
/// they do (decision 0100).
pub(crate) const SET_DENSITY_LABEL: &str = "Set texel density";
pub(crate) const SET_DENSITY_HELP: &str = "Scale the selected pieces (or whole objects) so each \
     has the project's texel density (UV → Texture), each about its own middle";
pub(crate) const MATCH_DENSITY_LABEL: &str = "Match texel density";
pub(crate) const MATCH_DENSITY_HELP: &str = "Scale the selected pieces (or whole objects) so they \
     share one texel density: the average of theirs";

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
pub(super) fn anchors(mesh: &scene::Mesh, faces: &[FaceId]) -> Option<Anchors> {
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
    pub(super) fn uv_tool_op(
        &self,
        tool: UvTool,
        mesh: &scene::Mesh,
        faces: Vec<FaceId>,
    ) -> Operation {
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

    /// Set texel density (`to_aim`: to the project's) or Match texel
    /// density (to the average of the pieces'), as one undo step.
    pub(crate) fn run_texel_density(&mut self, to_aim: bool) {
        let name = if to_aim {
            SET_DENSITY_LABEL
        } else {
            MATCH_DENSITY_LABEL
        };
        let shapes = self.own_shapes();
        if shapes.is_empty() {
            self.note = Some(format!("{name}: select the objects first."));
            return;
        }
        let texture = self.document.scene().texture();
        let chosen: Vec<(ObjectId, std::sync::Arc<scene::Mesh>, Vec<FaceId>)> = shapes
            .into_iter()
            .map(|(id, mesh)| {
                let faces = targets(&mesh, self.document.elements(id), self.mode);
                (id, mesh, faces)
            })
            .filter(|(_, mesh, faces)| !faces.is_empty() && mesh::uvs::has_uvs(mesh))
            .collect();
        let (mut flat, mut real) = (0.0, 0.0);
        for (_, mesh, faces) in &chosen {
            if let Some(uvs) = mesh::uvs::Uvs::of(mesh) {
                let (f, r) = uv::density::areas(mesh, faces, &uvs);
                flat += f;
                real += r;
            }
        }
        if flat <= 1e-20 || real <= 1e-20 {
            self.note = Some(format!("{name}: the selection has no UVs to measure."));
            return;
        }
        let density = if to_aim {
            texture.uv_per_unit()
        } else {
            (flat / real).sqrt()
        };
        let mut count = 0;
        let ops: Vec<(ObjectId, Operation)> = chosen
            .into_iter()
            .map(|(id, mesh, faces)| {
                count += faces.len();
                let anchors = anchors(&mesh, &faces);
                (
                    id,
                    Operation::SetTexelDensity {
                        faces,
                        density,
                        anchors,
                    },
                )
            })
            .collect();
        let label = counted("Set the texel density of", count, "polygon");
        if self.record_each(&label, ops) {
            self.note = Some(format!(
                "{label}: {:.0} px/m. Ctrl + Z takes it back.",
                texture.pixels_per_metre(density)
            ));
        }
    }

    /// Pack: fits the islands holding the selection (or whole objects)
    /// into the texture square, as one undo step: all together, or each
    /// material's (and each object's) on a texture of their own.
    pub(crate) fn run_pack(&mut self) {
        let shapes = self.own_shapes();
        if shapes.is_empty() {
            self.note = Some("Pack: select the objects first.".into());
            return;
        }
        self.pack_shapes(shapes, false, None);
    }

    /// Packs `shapes`' pieces: all of each when `whole`, else those
    /// holding the selection (whole objects without one); as part of the
    /// undo step `key` if given, else as a step of its own.
    pub(crate) fn pack_shapes(
        &mut self,
        shapes: Vec<(ObjectId, std::sync::Arc<scene::Mesh>)>,
        whole: bool,
        key: Option<u64>,
    ) -> bool {
        let settings = self.uv_settings;
        let texture = self.document.scene().texture();
        let padding_px = settings.padding.min(MAX_PADDING);
        let cell_px = (padding_px / 2).max(1);
        let options = uv::pack::PackOptions {
            padding: padding_px as f32 / texture.size as f32,
            rotate: settings.rotate,
            stack: settings.stack,
            cells: (texture.size / cell_px).clamp(128, 512) as usize,
        };
        // Islands, grouped by the texture they go on.
        let mut groups: std::collections::BTreeMap<TextureGroup, Vec<(ObjectId, Vec<FaceId>)>> =
            std::collections::BTreeMap::new();
        let mut meshes = std::collections::HashMap::new();
        for (id, mesh) in shapes {
            if !mesh::uvs::has_uvs(&mesh) {
                continue;
            }
            let faces = if whole {
                mesh.faces().map(|(f, _)| f).collect()
            } else {
                targets(&mesh, self.document.elements(id), self.mode)
            };
            for island in uv::pack::islands_of(&mesh, &faces) {
                groups
                    .entry(self.texture_group(id, &mesh, &island))
                    .or_default()
                    .push((id, island));
            }
            meshes.insert(id, mesh);
        }
        let mut by_object: std::collections::BTreeMap<ObjectId, Vec<(Vec<FaceId>, glam::Affine2)>> =
            std::collections::BTreeMap::new();
        let mut islands = 0;
        for (_, members) in groups {
            let pieces: Vec<uv::pack::Piece> = members
                .iter()
                .map(|(id, faces)| uv::pack::piece_of(&meshes[id], faces))
                .collect();
            let Some(moves) = uv::pack::pack(&pieces, &options) else {
                continue;
            };
            for ((id, faces), t) in members.into_iter().zip(moves) {
                islands += 1;
                by_object.entry(id).or_default().push((faces, t));
            }
        }
        if by_object.is_empty() {
            self.note = Some("Pack: the selection has no UVs with any area to pack.".into());
            return false;
        }
        let ops: Vec<(ObjectId, Operation)> = by_object
            .into_iter()
            .map(|(id, islands)| (id, Operation::PackUvs { islands }))
            .collect();
        let label = counted("Pack", islands, "island");
        let key = key.unwrap_or_else(|| self.new_step_key());
        let done = self.record_each_in(&label, ops, key);
        if done {
            self.note = Some(format!(
                "{label}, {padding_px} px apart. Ctrl + Z takes it back."
            ));
        }
        done
    }

    /// Which texture each piece of `mesh` (object `id`) goes on, as Pack
    /// groups them: the object's when each object gets its own, and its
    /// material's name when each material does.
    pub(crate) fn texture_group(
        &self,
        id: ObjectId,
        mesh: &scene::Mesh,
        piece: &[FaceId],
    ) -> TextureGroup {
        let settings = self.uv_settings;
        let material = if settings.per_material {
            let slot = piece
                .first()
                .map_or(0, |f| mesh::material::slot_of(mesh, *f));
            self.document
                .scene()
                .object(id)
                .and_then(|o| o.materials.get(slot as usize).map(|m| m.name.clone()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        (settings.per_object.then_some(id), material)
    }

    /// The UV menu's Packing entries: the padding and the switches.
    pub(crate) fn packing_menu(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.uv_settings;
        ui.horizontal(|ui| {
            ui.label(PADDING_LABEL);
            ui.add(
                egui::DragValue::new(&mut s.padding)
                    .range(0..=MAX_PADDING)
                    .suffix(" px"),
            )
            .on_hover_text(
                "The gap between pieces, in texture pixels (half of it to the square's edges). \
                 Keeps colours from bleeding between pieces when the texture is shrunk; 8 px \
                 suits a 2048 texture.",
            );
        });
        ui.checkbox(&mut s.rotate, "Turn pieces to fit")
            .on_hover_text(
                "Pieces may be turned (lined up with the square, and a quarter turn) to fit better",
            );
        ui.checkbox(&mut s.stack, "Stack identical pieces")
            .on_hover_text("Pieces of exactly the same shape share the same texture space");
        ui.checkbox(&mut s.per_material, "A texture per material")
            .on_hover_text(
                "Each material's pieces fill a square of their own, as separate texture sets",
            );
        ui.checkbox(&mut s.per_object, "A texture per object")
            .on_hover_text("Each object's pieces fill a square of their own");
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
