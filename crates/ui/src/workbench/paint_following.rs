//! Paint that follows the model ([decision 0189](../../../../docs/decisions/0189-paint-follows-the-model.md)):
//! after each change, any texture set whose polygons moved on the texture
//! (modelling, UV work, modifiers) has its paint moved with them, as
//! part of the same undo step.
//!
//! The workbench keeps, per texture set, the set and the shapes it was
//! last seen with. When a new change (not an undo or redo) leaves the set
//! as it was but its shapes laid out differently on the texture, the
//! paint is moved from the old shapes to the new (`paint::follow`) and
//! added to the change's step. While a mouse button is held (a drag
//! under way), it waits for the drag to end.
//!
//! The same goes for the project's texture size ([decision 0191](../../../../docs/decisions/0191-texture-size-without-blur.md)):
//! after it changes, each set is made that size (`paint::resize`).

use super::Workbench;
use crate::drawing::SceneDrawing;
use paint::{PaintSurface, TextureSet};
use std::collections::HashMap;
use std::sync::Arc;

/// What a texture set was last seen with.
#[derive(Debug)]
struct Seen {
    set: Arc<TextureSet>,
    key: u64,
    shapes: Vec<(u64, Arc<PaintSurface>, Vec<u32>)>,
}

/// The paint-following state.
#[derive(Debug, Default)]
pub(super) struct Following {
    /// Per material, the set and shapes last seen.
    seen: HashMap<String, Seen>,
    /// The document's revision and changes made when last looked at.
    revision: Option<u64>,
    changes: u64,
    /// Per material, the last time paint moved: the layout it moved from,
    /// the set before and the set after. A change back to that layout
    /// while the set is still the one after (Subdivide turned off again,
    /// say) gives the set before back, exactly and at once, rather than
    /// moving the paint a second time (slow on a dense shape, and each
    /// move blurs a little; a review found both).
    last_move: HashMap<String, (u64, Arc<TextureSet>, Arc<TextureSet>)>,
}

impl Workbench {
    /// Moves paint with its polygons after a change (once no button is
    /// held), each frame after the drawing is made. Whether paint moved
    /// (the views show it from the next frame).
    pub(super) fn follow_paint(&mut self, drawing: &SceneDrawing, button_down: bool) -> bool {
        let revision = self.document.revision();
        if self.following.revision == Some(revision) || button_down {
            return false;
        }
        let mut moved_any = false;
        let fresh_change = self.following.revision.is_some()
            && self.document.changes_made() != self.following.changes;
        let sets: Vec<(String, Arc<TextureSet>)> = self
            .document
            .scene()
            .paint()
            .sets()
            .map(|(m, s)| (m.to_string(), Arc::clone(s)))
            .collect();
        self.following
            .seen
            .retain(|m, _| sets.iter().any(|(n, _)| n == m));
        self.following
            .last_move
            .retain(|m, _| sets.iter().any(|(n, _)| n == m));
        let ids: Vec<scene::ObjectId> = self
            .document
            .scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        for (material, set) in sets {
            let mut shapes = Vec::new();
            for &id in &ids {
                let slots = self.slots_of(id, &material);
                if slots.is_empty() {
                    continue;
                }
                let Some(mesh) = super::brushing::drawn_mesh(drawing, id) else {
                    continue;
                };
                let surface = self.brush_surface(id, &mesh);
                shapes.push((id.get(), surface, slots));
            }
            let key = paint::follow::layout_key(&view(&shapes));
            let mut now = Arc::clone(&set);
            let back = self
                .following
                .last_move
                .get(&material)
                .filter(|(from, _, after)| *from == key && Arc::ptr_eq(after, &set))
                .map(|(_, before, _)| Arc::clone(before));
            if fresh_change
                && let Some(seen) = self.following.seen.get(&material)
                && Arc::ptr_eq(&seen.set, &set)
                && seen.key != key
                && let Some(moved) = back
                    .map(|b| (*b).clone())
                    .or_else(|| paint::follow::follow(&set, &view(&seen.shapes), &view(&shapes)))
            {
                let from = seen.key;
                let command = scene::Command::SetTextureSet {
                    material: material.clone(),
                    set: Some(moved),
                };
                match self.document.execute_with_last(command) {
                    Ok(()) => {
                        moved_any = true;
                        self.note = Some(
                            crate::text::tr(
                                "The paint moved with its polygons (one undo takes both back).",
                            )
                            .to_string(),
                        );
                        if let Some(after) = self.document.scene().paint().set(&material) {
                            now = Arc::clone(after);
                            self.following.last_move.insert(
                                material.clone(),
                                (from, Arc::clone(&set), Arc::clone(after)),
                            );
                        }
                    }
                    Err(why) => {
                        self.note = Some(crate::trf!("Couldn't move the paint: {}.", why));
                    }
                }
            }
            // The project's texture size changed: the set follows it,
            // painted again from its strokes where it can be (decision
            // 0191).
            let size = self.document.scene().texture().size;
            if fresh_change
                && now.size != size
                && let Some(bigger) = paint::resize::resized(&now, size, &view(&shapes))
            {
                let command = scene::Command::SetTextureSet {
                    material: material.clone(),
                    set: Some(bigger),
                };
                match self.document.execute_with_last(command) {
                    Ok(()) => {
                        moved_any = true;
                        if let Some(set) = self.document.scene().paint().set(&material) {
                            now = Arc::clone(set);
                        }
                    }
                    Err(why) => {
                        self.note = Some(crate::trf!("Couldn't resize the paint: {}.", why));
                    }
                }
            }
            // Gradient layers are made again on the moved or resized
            // shapes rather than carried (decision 0194).
            if !Arc::ptr_eq(&now, &set) {
                let made = Arc::clone(&now);
                self.remake_gradients_of(&material, &made);
            }
            self.following.seen.insert(
                material,
                Seen {
                    set: now,
                    key,
                    shapes,
                },
            );
        }
        self.following.revision = Some(self.document.revision());
        self.following.changes = self.document.changes_made();
        moved_any
    }
}

/// The shapes as `paint::follow` takes them.
fn view(shapes: &[(u64, Arc<PaintSurface>, Vec<u32>)]) -> Vec<paint::follow::Shape<'_>> {
    shapes
        .iter()
        .map(|(id, surface, slots)| paint::follow::Shape {
            id: *id,
            surface,
            slots,
        })
        .collect()
}
