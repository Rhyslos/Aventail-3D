//! Selecting UVs the smart ways, isolating pieces, the UV Editor keeping
//! its own selection, and copying UVs between matching pieces
//! ([decision 0239](../../../../docs/decisions/0239-uv-selection-and-copying.md)).
//! All in the UV menu's Select group, without keys (the project lead's
//! choice: isolate and sync in the menus).

use super::Workbench;
use super::joining::counted;
use super::unwrapping::targets;
use crate::palette::Mode;
use crate::text::tr;
use egui::Ui;
use scene::{EdgeKey, Element, ElementSelection, FaceId, ObjectId, Operation, VertexId};

/// The UV menu's Select group.
pub(crate) const UV_SELECT_MENU: &str = crate::text!("Select in UVs");
/// How stretched a polygon must be for Select stretched (as a factor).
pub(crate) const STRETCHED_AT: f32 = 1.5;
/// How alike pieces must be for Select similar (a part).
pub(crate) const SIMILAR_WITHIN: f32 = 0.02;

/// One of the smart selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvSelect {
    Overlapping,
    Mirrored,
    Stretched,
    Similar,
    Grow,
    Shrink,
    Path,
}

impl UvSelect {
    pub(crate) const ALL: [Self; 7] = [
        Self::Overlapping,
        Self::Mirrored,
        Self::Stretched,
        Self::Similar,
        Self::Grow,
        Self::Shrink,
        Self::Path,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Overlapping => tr("Select overlapping"),
            Self::Mirrored => tr("Select mirrored"),
            Self::Stretched => tr("Select stretched"),
            Self::Similar => tr("Select similar pieces"),
            Self::Grow => tr("Grow in the texture"),
            Self::Shrink => tr("Shrink in the texture"),
            Self::Path => tr("Shortest path in the texture"),
        }
    }

    pub(crate) fn help(self) -> &'static str {
        match self {
            Self::Overlapping => tr(
                "Select the polygons lying over others in the texture (shown red): they'd share \
                 the same paint",
            ),
            Self::Mirrored => tr("Select the pieces laid face down in the texture (shown purple)"),
            Self::Stretched => tr(
                "Select the polygons stretched one and a half times or more: the ones the \
                 Stretch view shows red",
            ),
            Self::Similar => tr(
                "Add every piece like the selected ones: the same polygons and about the same \
                 size (bolts, planks, repeated parts)",
            ),
            Self::Grow => tr(
                "Add the polygons next to the selection in the texture: it never grows across a \
                 seam",
            ),
            Self::Shrink => tr("Take away the polygons at the selection's rim in the texture"),
            Self::Path => tr(
                "With two points selected, select the edges between them by the shortest way in \
                 the texture, never across a seam",
            ),
        }
    }
}

/// Copy UVs' name, and what it does.
pub(crate) const COPY_UVS_LABEL: &str = crate::text!("Copy UVs to similar pieces");
pub(crate) const COPY_UVS_HELP: &str = crate::text!(
    "Lay every selected piece out like the first one (the piece holding the lowest-numbered \
     selected polygon), each where it was: for pieces made the same way, such as copies of a \
     part. Pieces made differently are skipped."
);
pub(crate) const ISOLATE_LABEL: &str = crate::text!("Isolate the selected pieces");
pub(crate) const SYNC_LABEL: &str = crate::text!("UV Editor follows the viewport's selection");

impl Workbench {
    /// UV → Select in UVs.
    pub(super) fn uv_select_entries(&mut self, ui: &mut Ui) {
        for what in UvSelect::ALL {
            if matches!(what, UvSelect::Grow) {
                ui.separator();
            }
            if ui.button(what.name()).on_hover_text(what.help()).clicked() {
                self.run_uv_select(what);
                ui.close();
            }
        }
        ui.separator();
        let mut isolate = self.uv_editor.isolating();
        if ui
            .checkbox(&mut isolate, tr(ISOLATE_LABEL))
            .on_hover_text(tr(
                "Show only the pieces holding the selection in the UV Editor, to work on them \
                 without the rest in the way. A view setting: nothing changes.",
            ))
            .changed()
        {
            self.uv_editor.set_isolating(isolate, &self.document);
        }
        let mut sync = !self.uv_editor.own_selection();
        if ui
            .checkbox(&mut sync, tr(SYNC_LABEL))
            .on_hover_text(tr(
                "On (standard): selecting in the UV Editor and the viewport is one selection. \
                 Off: the UV Editor keeps its own, so you can pick UVs without losing the \
                 viewport's; the gizmo, the brushes and Frame selected in the editor use it.",
            ))
            .changed()
        {
            self.uv_editor.set_own_selection(!sync, &self.document);
        }
        ui.separator();
        if ui
            .button(tr(COPY_UVS_LABEL))
            .on_hover_text(tr(COPY_UVS_HELP))
            .clicked()
        {
            self.run_copy_uvs();
            ui.close();
        }
    }

    /// The selected polygons of each selected object (or, without, every
    /// polygon), with its shape.
    fn uv_select_parts(&self) -> Vec<(ObjectId, std::sync::Arc<scene::Mesh>, Vec<FaceId>, bool)> {
        self.own_shapes()
            .into_iter()
            .filter(|(_, m)| mesh::uvs::has_uvs(m))
            .map(|(id, mesh)| {
                let any = self
                    .document
                    .elements(id)
                    .is_some_and(|s| !s.is_empty());
                let faces = targets(&mesh, self.document.elements(id), self.mode);
                (id, mesh, faces, any)
            })
            .collect()
    }

    /// Carries out a smart selection, as one undo step.
    pub(crate) fn run_uv_select(&mut self, what: UvSelect) {
        let parts = self.uv_select_parts();
        if parts.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select objects with UVs first.",
                what.name()
            ));
            return;
        }
        let mut changes: Vec<(ObjectId, ElementSelection)> = Vec::new();
        let mut count = 0;
        let edges_mode = what == UvSelect::Path;
        for (id, mesh, faces, any) in &parts {
            if edges_mode {
                let points: Vec<VertexId> = self
                    .document
                    .elements(*id)
                    .map(|s| s.points().collect())
                    .unwrap_or_default();
                if points.len() != 2 {
                    continue;
                }
                let Some(path) = uv::select::shortest_path(mesh, points[0], points[1]) else {
                    continue;
                };
                count += path.len();
                changes.push((*id, ElementSelection::of(path.into_iter().map(Element::Edge))));
                continue;
            }
            let picked: Vec<FaceId> = match what {
                UvSelect::Overlapping => uv::select::overlapping(mesh),
                UvSelect::Mirrored => uv::select::mirrored(mesh),
                UvSelect::Stretched => uv::select::stretched(mesh, STRETCHED_AT),
                UvSelect::Similar if *any => uv::select::similar(mesh, faces, SIMILAR_WITHIN),
                UvSelect::Grow if *any => uv::select::grow(mesh, faces),
                UvSelect::Shrink if *any => uv::select::shrink(mesh, faces),
                _ => continue,
            };
            count += picked.len();
            changes.push((*id, ElementSelection::of(picked.into_iter().map(Element::Polygon))));
        }
        if changes.is_empty() || count == 0 {
            self.note = Some(match what {
                UvSelect::Path => crate::trf!(
                    "{}: select two points of one piece (1 for the Points mode).",
                    what.name()
                ),
                UvSelect::Similar | UvSelect::Grow | UvSelect::Shrink => crate::trf!(
                    "{}: select some polygons first (3 for the Polygons mode), or there's \
                     nothing more to find.",
                    what.name()
                ),
                _ => crate::trf!("{}: nothing found. Good news.", what.name()),
            });
            return;
        }
        self.mode = match (what, self.mode) {
            (UvSelect::Path, _) => Mode::Edges,
            (_, Mode::Uv) => Mode::Uv,
            _ => Mode::Polygons,
        };
        let kind = if what == UvSelect::Path {
            tr("edge")
        } else {
            tr("polygon")
        };
        let label = counted(what.name(), count, kind);
        self.document.select_elements(changes, &label, None);
        self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
    }

    /// Copy UVs to similar pieces: the first selected piece's layout onto
    /// the other selected pieces, object by object.
    pub(crate) fn run_copy_uvs(&mut self) {
        let mut ops = Vec::new();
        let mut copied = 0;
        let mut asked = 0;
        for (id, mesh, faces, any) in self.uv_select_parts() {
            if !any {
                continue;
            }
            let mut pieces = uv::pieces::pieces_holding(&mesh, &faces);
            if pieces.len() < 2 {
                continue;
            }
            pieces.sort_by_key(|p| p.iter().min().copied());
            asked += pieces.len() - 1;
            let (corners, count) = uv::select::copy_layout(&mesh, &pieces[0], &pieces[1..]);
            if count > 0 {
                copied += count;
                ops.push((id, Operation::SetUvs { corners }));
            }
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select polygons in two or more pieces made the same way (the first one \
                 is copied).",
                tr(COPY_UVS_LABEL)
            ));
            return;
        }
        let label = counted(tr("Copy UVs to"), copied, tr("piece"));
        if self.record_each(&label, ops) {
            self.note = Some(if copied < asked {
                crate::trf!(
                    "{} ({} skipped: made differently). Ctrl + Z takes it back.",
                    label,
                    asked - copied
                )
            } else {
                crate::trf!("{}. Ctrl + Z takes it back.", label)
            });
        }
    }
}

/// For tests: the edges of a selection.
#[cfg(test)]
pub(super) fn edges_of(selection: &ElementSelection) -> Vec<EdgeKey> {
    selection.edges().collect()
}
