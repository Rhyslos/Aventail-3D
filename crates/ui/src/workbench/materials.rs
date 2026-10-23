//! The workbench's part in materials ([decision 0074](../../../../docs/decisions/0074-material-slots.md)):
//! giving the selected polygons the material whose Assign button was
//! clicked in the Attributes panel, as a recorded edit (so it is undone
//! like any other, and made on both sides with symmetry on).

use super::Workbench;
use super::cleaning::chosen;
use super::joining::counted;
use crate::palette::Mode;
use crate::text::tr;
use scene::{ElementKind, Operation};

impl Workbench {
    /// How many polygons of the one selected object are selected, in the
    /// Polygons mode; `None` in other modes (or with no single object).
    pub(super) fn polygons_selected_for_attributes(&self) -> Option<usize> {
        if self.mode != Mode::Polygons {
            return None;
        }
        let [id] = self.document.selection()[..] else {
            return None;
        };
        Some(
            self.document
                .elements(id)
                .map_or(0, |e| e.count(ElementKind::Polygons)),
        )
    }

    /// Gives the selected polygons of the selected object material `slot`
    /// (its Assign button in the Attributes panel).
    pub(super) fn assign_material(&mut self, slot: u32) {
        if self.mode != Mode::Polygons {
            self.note =
                Some(tr("Assign: switch to the Polygons mode and select polygons first.").into());
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        let mut name = String::new();
        for (id, selected, mesh) in self.selected_parts() {
            let c = chosen(Mode::Polygons, &selected, &mesh);
            if c.faces.is_empty() {
                continue;
            }
            if let Some(material) = self.document.scene().material_list(id).get(slot as usize) {
                name.clone_from(&material.name);
            }
            count += c.faces.len();
            ops.push((
                id,
                Operation::SetMaterial {
                    faces: c.faces,
                    slot,
                    anchors: c.anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(tr("Assign: select some polygons first.").into());
            return;
        }
        let label = crate::trf!("{} {}", counted(tr("Give"), count, "polygon"), name);
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }
}
