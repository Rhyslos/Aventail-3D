//! Assemblies on the shape canvas (Phase 12 milestone 12, decision 0274):
//! copying and pasting pieces (plain or linked, here, on another canvas
//! or in another tab), grouping and ungrouping them, flipping them, and
//! giving them colours and heights. Each is one undo step.

use super::report;
use crate::material::MaterialEditor;
use crate::text::tr;
use glam::Vec2;
use scene::Document;
use shapes::{Change, Drawing, GroupId, PieceCopies, PieceId};
use std::sync::Arc;

/// How far a paste lands from where the pieces were copied when the
/// pointer isn't over the canvas (a share of the texture), so the copy
/// shows.
const PASTE_STEP: f32 = 0.04;

impl MaterialEditor {
    /// The selected pieces, in the drawing's order.
    pub(crate) fn chosen_pieces(&self, drawing: &Drawing) -> Vec<PieceId> {
        let Some(s) = self.shape.as_ref() else {
            return Vec::new();
        };
        drawing
            .pieces()
            .iter()
            .filter(|p| s.selected.contains(&p.id))
            .map(|p| p.id)
            .collect()
    }

    /// The group whose pieces are exactly the ones selected, if any.
    pub(crate) fn chosen_group(&self, drawing: &Drawing) -> Option<GroupId> {
        let chosen = self.chosen_pieces(drawing);
        let group = drawing.piece(*chosen.first()?)?.group?;
        (drawing.group_pieces(group) == chosen).then_some(group)
    }

    /// Ctrl + C (or Alt + C): copies the selected pieces, to paste here,
    /// on another Shape box's canvas or in another tab.
    pub(crate) fn copy_pieces(&mut self, document: &Document, note: &mut Option<String>) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let chosen = self.chosen_pieces(&drawing);
        if chosen.is_empty() {
            *note = Some(tr("Select pieces first: click one, or drag round several.").into());
            return;
        }
        self.pieces_clipboard = Some(Arc::new(drawing.copied(&chosen)));
        *note = Some(crate::trf!(
            "Copied {} pieces. Ctrl + V pastes copies of their own; Alt + V pastes linked \
             copies (change the shape of one and every linked copy changes).",
            chosen.len()
        ));
    }

    /// Ctrl + V (plain) or Alt + V (`linked`): pastes the copied pieces on
    /// top, their middle at `at` on the canvas (or a little way from
    /// where they were), and selects them.
    pub(crate) fn paste_pieces(
        &mut self,
        document: &mut Document,
        linked: bool,
        at: Option<Vec2>,
        note: &mut Option<String>,
    ) {
        let Some(copies) = self.pieces_clipboard.clone() else {
            *note = Some(tr("Nothing copied yet: select pieces and press Ctrl + C.").into());
            return;
        };
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let by = paste_move(&copies, at);
        match drawing.with_pasted(&copies, linked, &Change::moved(by)) {
            Ok((d, ids)) => {
                let label = if linked {
                    tr("Paste linked copies")
                } else {
                    tr("Paste pieces")
                };
                let done = self.set_drawing(document, d, label, None);
                if done.is_ok() {
                    if let Some(s) = self.shape.as_mut() {
                        s.selected = ids.iter().copied().collect();
                        s.editing = None;
                    }
                    *note = Some(if linked {
                        crate::trf!(
                            "Pasted {} linked copies: change the shape of one and every linked \
                             copy changes. Each keeps its own place, colour and height.",
                            ids.len()
                        )
                    } else {
                        crate::trf!("Pasted {} pieces.", ids.len())
                    });
                }
                report(done, note);
            }
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// Alt + G: gathers the selected pieces into a group.
    pub(crate) fn group_chosen(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let chosen = self.chosen_pieces(&drawing);
        if chosen.len() < 2 {
            *note = Some(
                tr("Select two pieces or more to group them: Ctrl + click adds a piece.").into(),
            );
            return;
        }
        let name = crate::trf!("Group {}", drawing.next_id().0);
        match drawing.with_group(&chosen, &name) {
            Ok((d, _)) => {
                let done = self.set_drawing(document, d, tr("Group pieces"), None);
                report(done, note);
            }
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// Shift + G: takes apart the groups the selected pieces are in.
    pub(crate) fn ungroup_chosen(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let mut groups: Vec<GroupId> = self
            .chosen_pieces(&drawing)
            .iter()
            .filter_map(|id| drawing.piece(*id)?.group)
            .collect();
        groups.dedup();
        if groups.is_empty() {
            *note = Some(tr("None of the selected pieces is in a group.").into());
            return;
        }
        let mut d = (*drawing).clone();
        for g in groups {
            match d.without_group(g) {
                Ok(next) => d = next,
                Err(why) => return self.shape_note(tr(why.describe()), Vec::new(), note),
            }
        }
        let done = self.set_drawing(document, d, tr("Ungroup pieces"), None);
        report(done, note);
    }

    /// Flips the selected pieces about their middle: across (left for
    /// right) or up and down.
    pub(crate) fn flip_chosen(
        &mut self,
        document: &mut Document,
        across: bool,
        note: &mut Option<String>,
    ) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let chosen = self.chosen_pieces(&drawing);
        if chosen.is_empty() {
            return;
        }
        let label = if across {
            tr("Flip pieces across")
        } else {
            tr("Flip pieces up and down")
        };
        match drawing.flipped(&chosen, across) {
            Ok(d) => report(self.set_drawing(document, d, label, None), note),
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// Gives the selected pieces `colour`, or `height`: one undo step
    /// while `key` stays the same (a colour picked or a slider dragged).
    pub(crate) fn colour_chosen(
        &mut self,
        document: &mut Document,
        colour: Option<[f32; 3]>,
        height: Option<f32>,
        key: Option<u64>,
        note: &mut Option<String>,
    ) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        let mut d = (*drawing).clone();
        for id in self.chosen_pieces(&drawing) {
            let next = match (colour, height) {
                (Some(c), _) => d.with_colour(id, c),
                (None, Some(h)) => d.with_height(id, h),
                (None, None) => return,
            };
            match next {
                Ok(next) => d = next,
                Err(why) => return self.shape_note(tr(why.describe()), Vec::new(), note),
            }
        }
        let label = if colour.is_some() {
            tr("Colour pieces")
        } else {
            tr("Change pieces' heights")
        };
        report(self.set_drawing(document, d, label, key), note);
    }

    /// Gives piece `id` a shape of its own, no longer linked.
    pub(crate) fn unlink_piece(
        &mut self,
        document: &mut Document,
        id: PieceId,
        note: &mut Option<String>,
    ) {
        let Some(drawing) = self.shape_drawing(document) else {
            return;
        };
        match drawing.with_unlinked(id) {
            Ok(d) => report(
                self.set_drawing(document, d, tr("Unlink a piece"), None),
                note,
            ),
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// The pieces to select when piece `id` is clicked on the canvas: its
    /// whole group, or it alone.
    pub(crate) fn clicked_pieces(drawing: &Drawing, id: PieceId) -> Vec<PieceId> {
        match drawing.piece(id).and_then(|p| p.group) {
            Some(g) => drawing.group_pieces(g),
            None => vec![id],
        }
    }
}

/// The move pasting `copies` makes: their middle to `at`, or a little way
/// from where they were.
fn paste_move(copies: &PieceCopies, at: Option<Vec2>) -> Vec2 {
    let middle = copies.middle().unwrap_or(Vec2::splat(0.5));
    match at {
        Some(at) if at.is_finite() => at - middle,
        _ => Vec2::splat(PASTE_STEP),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::{Basic, PieceKind};

    #[test]
    fn a_paste_lands_at_the_pointer_or_a_step_away() {
        let d = Drawing::new()
            .with_piece(
                "Leaf",
                Basic::Circle.path(Vec2::splat(0.3), 0.1),
                PieceKind::Add,
            )
            .unwrap();
        let copies = d.copied(&[PieceId(1)]);
        assert!(paste_move(&copies, Some(Vec2::splat(0.7))).distance(Vec2::splat(0.4)) < 1e-5);
        assert_eq!(paste_move(&copies, None), Vec2::splat(PASTE_STEP));
        assert_eq!(
            paste_move(&copies, Some(Vec2::NAN)),
            Vec2::splat(PASTE_STEP)
        );
    }
}
