//! Split polygons off into an object of their own
//! ([decision 0055](../../../../docs/decisions/0055-basics.md)). Joining
//! objects is a command (`CommandId::Join`).

use super::Workbench;
use crate::palette::Mode;
use crate::text::tr;
use mesh::Anchors;
use scene::{Command, ElementKind, FaceId, VertexId};

impl Workbench {
    /// Mesh → Split off: the selected polygons of the selected object
    /// become an object of their own.
    pub(super) fn run_split_off(&mut self) {
        if self.mode != Mode::Polygons {
            self.note =
                Some(tr("Split off works on polygons: press 3 for the Polygons mode.").into());
            return;
        }
        let parts: Vec<_> = self
            .selected_parts()
            .into_iter()
            .filter(|(_, selected, _)| selected.polygons().next().is_some())
            .collect();
        let [(id, selected, mesh)] = parts.as_slice() else {
            self.note = Some(if parts.is_empty() {
                tr("Split off: select some polygons first.").into()
            } else {
                tr("Split off: select polygons on one object at a time.").into()
            });
            return;
        };
        let faces: Vec<FaceId> = selected.polygons().collect();
        let places: Vec<Vec<VertexId>> = faces
            .iter()
            .filter_map(|f| mesh.face(*f).map(<[VertexId]>::to_vec))
            .collect();
        let anchors = Anchors::record(mesh, ElementKind::Polygons, &places);
        let n = faces.len();
        match self.document.execute(Command::SplitOff {
            id: *id,
            faces,
            anchors,
        }) {
            Ok(()) => {
                self.note = Some(crate::trf!(
                    "Split off {} polygon{} into a new object. Ctrl + Z takes it back.",
                    n,
                    if n == 1 { "" } else { "s" }
                ));
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't split that off: {}.", why)),
        }
    }
}
