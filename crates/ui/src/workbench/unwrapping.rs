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

/// How UV tools behave, chosen in the UV menu and remembered. Every
/// choice the automatic tools make is here (decision 0110), except
/// mending, which is saved with the project.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// The way Unwrap lays pieces flat (decision 0109).
    pub unwrap_method: UnwrapWay,
    /// How far (degrees) a quad's corners may be from square for its
    /// piece to unwrap as a straight strip.
    pub strip_angle: f32,
    /// Suggest seams: corners sharper than this (degrees) are cut.
    pub seam_corner_angle: f32,
    /// Suggest seams: whether hard edges are cut.
    pub seam_hard_edges: bool,
    /// Where the cuts that make pieces lie flat are hidden (Suggest seams
    /// and Unwrap).
    pub seam_hiding: SeamHiding,
    /// How many steps Relax takes.
    pub relax_steps: u32,
    /// How finely Pack fits pieces.
    pub pack_quality: PackQuality,
    /// Whether Suggest seams and Unwrap hold pieces to a stretch budget
    /// (decision 0111).
    pub budget_on: bool,
    /// That budget: the most a piece's polygons may be off, as a factor.
    pub budget: f32,
    /// The budget the Stretch budget submenu gives the selection.
    pub piece_budget: f32,
}

/// Where cuts that make pieces lie flat are hidden (decision 0110).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SeamHiding {
    /// Underneath and at the back: the standard.
    #[default]
    BelowAndBehind,
    /// Underneath only.
    Below,
    /// At the back only.
    Behind,
    /// Nowhere in particular: the shortest cuts.
    Shortest,
}

impl SeamHiding {
    /// Every choice, in menu order.
    pub const ALL: [Self; 4] = [
        Self::BelowAndBehind,
        Self::Below,
        Self::Behind,
        Self::Shortest,
    ];

    /// Its name in the menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::BelowAndBehind => "Underneath and at the back",
            Self::Below => "Underneath",
            Self::Behind => "At the back",
            Self::Shortest => "Anywhere (shortest cuts)",
        }
    }

    /// What it does.
    pub fn help(self) -> &'static str {
        match self {
            Self::BelowAndBehind => {
                "Cuts go where a prop is least seen: facing down, or away from the Front view"
            }
            Self::Below => "Cuts go on the parts facing down, wherever they face round the sides",
            Self::Behind => {
                "Cuts go on the parts facing away from the Front view, top or bottom alike"
            }
            Self::Shortest => {
                "Cuts are as short as can be, wherever they fall: for shapes seen from all round"
            }
        }
    }

    /// The `uv` crate's name for it.
    pub(crate) fn hide(self) -> scene::HideSeams {
        match self {
            Self::BelowAndBehind => scene::HideSeams::BelowAndBehind,
            Self::Below => scene::HideSeams::Below,
            Self::Behind => scene::HideSeams::Behind,
            Self::Shortest => scene::HideSeams::Shortest,
        }
    }
}

/// How finely Pack fits pieces (decision 0110).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PackQuality {
    /// A coarser grid: quick, a little looser.
    Fast,
    /// A grid as fine as half the padding (at most 512 cells across): the
    /// standard.
    #[default]
    Standard,
    /// The finest grid (1,024 cells across): the tightest fit, slowest.
    Best,
}

impl PackQuality {
    /// Every choice, in menu order.
    pub const ALL: [Self; 3] = [Self::Fast, Self::Standard, Self::Best];

    /// Its name in the menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::Fast => "Fast",
            Self::Standard => "Standard",
            Self::Best => "Best",
        }
    }

    /// What it does.
    pub fn help(self) -> &'static str {
        match self {
            Self::Fast => {
                "Pieces are fitted on a coarse grid: quick, with a little more space lost"
            }
            Self::Standard => "Pieces are fitted on a grid as fine as half the padding",
            Self::Best => "Pieces are fitted on the finest grid: the tightest fit, and the slowest",
        }
    }

    /// How many cells across the packing grid is, for a texture of `size`
    /// pixels with `padding` pixels between pieces.
    pub(crate) fn cells(self, size: u32, padding: u32) -> usize {
        let cell_px = (padding / 2).max(1);
        let standard = (size / cell_px).clamp(128, 512) as usize;
        match self {
            Self::Fast => (standard / 2).max(64),
            Self::Standard => standard,
            Self::Best => uv::pack::MAX_CELLS,
        }
    }
}

/// The way Unwrap lays pieces flat, chosen in UV → Unwrap method and
/// remembered (decision 0109).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UnwrapWay {
    /// Every small piece keeps its shape (least squares conformal maps):
    /// fast, and what the app always did.
    #[default]
    KeepShapes,
    /// Shapes kept, with the angles evened out over the whole piece first
    /// (angle-based flattening): less drift on curved pieces.
    AngleBased,
    /// Every part keeps its size (as rigid as possible): pixels the same
    /// size all over.
    KeepSizes,
    /// Each piece tries all three and keeps the one that lies most evenly.
    Automatic,
}

impl UnwrapWay {
    /// Every way, in menu order.
    pub const ALL: [Self; 4] = [
        Self::Automatic,
        Self::KeepShapes,
        Self::AngleBased,
        Self::KeepSizes,
    ];

    /// Its name in the menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::KeepShapes => "Keep shapes",
            Self::AngleBased => "Keep shapes, smoother",
            Self::KeepSizes => "Keep sizes",
            Self::Automatic => "Automatic",
        }
    }

    /// What it does, in plain words.
    pub fn help(self) -> &'static str {
        match self {
            Self::KeepShapes => {
                "Every small part keeps its angles, so a texture's squares stay square. Sizes \
                 may drift across a strongly curved piece. Fast: the standard way."
            }
            Self::AngleBased => {
                "Like Keep shapes, but the angles are first evened out over the whole piece, \
                 so curved pieces (a head, a dome) drift less. A little slower."
            }
            Self::KeepSizes => {
                "Every part keeps its real size, so the texture's pixels are the same size all \
                 over the piece, at the cost of a little bending of angles. The slowest."
            }
            Self::Automatic => {
                "Each piece is laid flat all three ways and keeps the one that lies most evenly. \
                 Always the same choice for the same piece; slowest of all."
            }
        }
    }

    /// The `uv` crate's name for it.
    pub(crate) fn method(self) -> scene::UnwrapMethod {
        match self {
            Self::KeepShapes => scene::UnwrapMethod::Conformal,
            Self::AngleBased => scene::UnwrapMethod::AngleBased,
            Self::KeepSizes => scene::UnwrapMethod::SizeKeeping,
            Self::Automatic => scene::UnwrapMethod::Automatic,
        }
    }
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
            unwrap_method: UnwrapWay::KeepShapes,
            strip_angle: uv::unwrap::SQUARE_WITHIN,
            seam_corner_angle: uv::seams::CORNER_ANGLE,
            seam_hard_edges: true,
            seam_hiding: SeamHiding::BelowAndBehind,
            relax_steps: uv::unwrap::RELAX_STEPS as u32,
            pack_quality: PackQuality::Standard,
            budget_on: false,
            budget: 2.0,
            piece_budget: 1.5,
        }
    }
}

/// The UV menu's name for laying strips on a trim row (decision 0106).
pub(crate) const PUT_ON_TRIM_LABEL: &str = "Put on trim row";

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

/// The UV menu's names for unwrapping one way only, and what they do
/// (decision 0109).
pub(crate) const UNFOLD_ACROSS_LABEL: &str = "Unfold across only";
pub(crate) const UNFOLD_ACROSS_HELP: &str = "Unwrap, but move the UVs only across (U): every \
     point keeps its height in the texture. For evening out a strip along its length.";
pub(crate) const UNFOLD_UP_LABEL: &str = "Unfold up only";
pub(crate) const UNFOLD_UP_HELP: &str = "Unwrap, but move the UVs only up and down (V): every \
     point keeps its place across the texture";

/// One of the UV tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvTool {
    Unwrap,
    /// Unwrap, moving the UVs one way only.
    Unfold(scene::UnfoldAxis),
    Relax,
    Straighten,
}

impl UvTool {
    fn name(self) -> &'static str {
        match self {
            Self::Unwrap => "Unwrap",
            Self::Unfold(scene::UnfoldAxis::Across) => UNFOLD_ACROSS_LABEL,
            Self::Unfold(scene::UnfoldAxis::Up) => UNFOLD_UP_LABEL,
            Self::Relax => "Relax",
            Self::Straighten => "Straighten",
        }
    }
}

/// The polygons `selected` (of `mesh`, in `mode`) points at: the selected
/// polygons, or those around selected edges or points; every polygon when
/// nothing is selected or outside the element modes.
pub(super) fn targets(
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

    /// Every choice an unwrap makes, from the UV settings, unfolding one
    /// way when `only` says so.
    pub(crate) fn unwrap_options(&self, only: Option<scene::UnfoldAxis>) -> scene::UnwrapOptions {
        let s = &self.uv_settings;
        scene::UnwrapOptions {
            strips: s.straighten_strips,
            strip_angle: s.strip_angle.clamp(0.0, uv::unwrap::MAX_STRIP_ANGLE),
            method: s.unwrap_method.method(),
            only,
            hide: s.seam_hiding.hide(),
            budget: self.budget(),
        }
    }

    /// The stretch budget for pieces without their own, if on.
    fn budget(&self) -> Option<f32> {
        let s = &self.uv_settings;
        s.budget_on.then(|| {
            s.budget
                .clamp(*scene::BUDGET_RANGE.start(), *scene::BUDGET_RANGE.end())
        })
    }

    /// How Suggest seams works, from the UV settings.
    pub(crate) fn suggest_options(&self) -> uv::SuggestOptions {
        let s = &self.uv_settings;
        uv::SuggestOptions {
            corner_angle: s.seam_corner_angle.clamp(0.0, 180.0),
            hard_edges: s.seam_hard_edges,
            hide: s.seam_hiding.hide(),
            budget: self.budget(),
        }
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
            UvTool::Unwrap | UvTool::Unfold(_) => Operation::Unwrap {
                faces,
                options: self.unwrap_options(match tool {
                    UvTool::Unfold(axis) => Some(axis),
                    _ => None,
                }),
                anchors,
            },
            UvTool::Relax => Operation::RelaxUvs {
                faces,
                steps: self
                    .uv_settings
                    .relax_steps
                    .clamp(1, uv::unwrap::MAX_RELAX_STEPS),
                anchors,
            },
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
            UvTool::Unfold(scene::UnfoldAxis::Across) => "Unfold across",
            UvTool::Unfold(scene::UnfoldAxis::Up) => "Unfold up",
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
        let options = uv::pack::PackOptions {
            padding: padding_px as f32 / texture.size as f32,
            rotate: settings.rotate,
            stack: settings.stack,
            cells: settings.pack_quality.cells(texture.size, padding_px),
        };
        // Islands, grouped by the texture they go on.
        let mut groups: std::collections::BTreeMap<TextureGroup, Vec<(ObjectId, Vec<FaceId>)>> =
            std::collections::BTreeMap::new();
        let mut meshes = std::collections::HashMap::new();
        let mut on_trim = 0;
        for (id, mesh) in shapes {
            if !mesh::uvs::has_uvs(&mesh) {
                continue;
            }
            let faces = if whole {
                mesh.faces().map(|(f, _)| f).collect()
            } else {
                targets(&mesh, self.document.elements(id), self.mode)
            };
            // Pieces on the trim sheet stay on their rows.
            let rows = super::baking::trim_rows(self.document.scene());
            for island in uv::pack::islands_of(&mesh, &faces) {
                if super::baking::on_trim(&mesh, &island, &rows) {
                    on_trim += 1;
                    continue;
                }
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
            self.note = Some(if on_trim > 0 {
                "Pack: every selected piece is on the trim sheet, and stays on its row.".into()
            } else {
                "Pack: the selection has no UVs with any area to pack.".into()
            });
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

    /// UV → Put on trim row: the trim sheet's rows, top to bottom; each
    /// lays the selection's pieces on it.
    pub(crate) fn put_on_trim_menu(&mut self, ui: &mut egui::Ui) {
        let sheet = self.document.scene().trim().clone();
        if sheet.rows.is_empty() {
            ui.add_enabled(false, egui::Button::new("No rows yet"))
                .on_disabled_hover_text(
                    "Make the trim sheet's rows first: the UV Editor's Trim menu, or UV → Trim \
                     sheet",
                );
            return;
        }
        let mut order: Vec<usize> = (0..sheet.rows.len()).collect();
        order.sort_by(|a, b| sheet.rows[*b].top.total_cmp(&sheet.rows[*a].top));
        for i in order {
            if ui
                .button(&sheet.rows[i].name)
                .on_hover_text(
                    "Lay the selected pieces straight along this row, filling its height, from \
                     its left end",
                )
                .clicked()
            {
                self.run_put_on_trim(i);
                ui.close();
            }
        }
    }

    /// Lays the pieces holding the selection (whole objects without one)
    /// on trim row `row`, as one undo step.
    pub(crate) fn run_put_on_trim(&mut self, row: usize) {
        let Some(row) = self.document.scene().trim().rows.get(row).cloned() else {
            return;
        };
        let shapes = self.own_shapes();
        if shapes.is_empty() {
            self.note = Some(format!("{PUT_ON_TRIM_LABEL}: select the objects first."));
            return;
        }
        let mut ops = Vec::new();
        let mut pieces = 0;
        for (id, mesh) in shapes {
            if !mesh::uvs::has_uvs(&mesh) {
                continue;
            }
            let faces = targets(&mesh, self.document.elements(id), self.mode);
            pieces += uv::pack::islands_of(&mesh, &faces).len();
            ops.push((
                id,
                Operation::PlaceOnTrim {
                    anchors: anchors(&mesh, &faces),
                    faces,
                    bottom: row.bottom,
                    top: row.top,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(format!(
                "{PUT_ON_TRIM_LABEL}: the selection has no UVs. Unwrap it first."
            ));
            return;
        }
        let label = format!("Put on {}", row.name);
        if self.record_each(&label, ops) {
            self.note = Some(format!(
                "{} on {}. Slide along it with W; Ctrl + Z takes it back.",
                counted("Laid", pieces, "piece"),
                row.name
            ));
        }
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
        ui.separator();
        self.pack_quality_entries(ui);
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
