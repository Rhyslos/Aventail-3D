//! The Collision rows' settings for *Wrapped in parts* in the Attributes
//! panel ([decision 0151](../../../../docs/decisions/0151-collision-hulls.md)),
//! and the note saying why an object's collision shapes aren't what its
//! setting asks for.

use super::curve_rows::Rows;
use super::{AttributesPanel, label, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use scene::{Collision, Command, ConvexPartsOptions, ObjectId};

impl AttributesPanel {
    /// Max parts and Detail, for an object wrapped in parts (in the Basic
    /// grid, under Collision).
    pub(super) fn collision_parts_rows(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        object: &scene::Object,
        out: &mut Vec<DocRequest>,
    ) {
        if object.collision != Collision::Parts {
            return;
        }
        let mut options = object.collision_parts;
        let mut rows = Rows::default();
        label(
            ui,
            tr("Max parts"),
            tr(
                "The most parts it is split into. More follow its dents more closely; fewer are \
                 cheaper for the game",
            ),
        );
        rows.take(stepper::count(
            ui,
            &mut options.max_parts,
            (1, ConvexPartsOptions::MOST_PARTS),
            tr("Max parts"),
        ));
        ui.end_row();
        label(
            ui,
            tr("Detail"),
            tr(
                "How small a dent is still followed: higher finds smaller dents, takes longer and \
                 may use more parts",
            ),
        );
        rows.take(stepper::percent(
            ui,
            &mut options.detail,
            (0.0, 1.0),
            tr("Detail"),
        ));
        ui.end_row();
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && options != object.collision_parts {
            out.push(DocRequest::RunMerging(
                Command::SetCollisionParts {
                    ids: vec![id],
                    options,
                },
                self.gesture,
            ));
        }
    }
}

/// Why the object's collision shapes aren't what its setting asks for,
/// in the warning colour, if they aren't.
pub(super) fn collision_note(ui: &mut Ui, scene: &scene::Scene, id: ObjectId) {
    if let Some(why) = scene.collision_problem(id) {
        ui.label(RichText::new(why).color(theme::WARNING));
    }
}
