//! The boxes H (frame everything) and Z (frame the selection) fit in the
//! view. The scene's own boxes are of each object's shape before its
//! modifiers; an Array's copies, a Shell's thickness or a Displace's bumps
//! reach beyond that, so the box of what is drawn after the modifiers is
//! added for every object that has some shown in the viewport.

use super::{DrawItem, SceneSummary};
use scene::{Bounds, ObjectId, Purpose, Scene};

/// `scene`'s box around `ids` (and what is inside them), grown to take in
/// the drawn shapes of those of `items` with modifiers shown.
fn shown_bounds(scene: &Scene, items: &[DrawItem], ids: &[ObjectId]) -> Option<Bounds> {
    let mut total = scene.bounds_of(ids.iter().copied());
    let wanted: std::collections::HashSet<ObjectId> =
        ids.iter().flat_map(|id| scene.subtree(*id)).collect();
    for item in items {
        if !wanted.contains(&item.id) || !scene.has_modifiers_for(item.id, Purpose::Viewport) {
            continue;
        }
        let Some(local) = item.drawn.bounds() else {
            continue;
        };
        let Some(world) =
            Bounds::around(local.corners().map(|c| item.transform.transform_point3(c)))
        else {
            continue;
        };
        total = Some(total.map_or(world, |t| t.union(world)));
    }
    total
}

impl SceneSummary {
    /// The boxes around everything shown and around `selection`, for
    /// framing, as drawn (modifiers and all).
    pub(super) fn framing(
        scene: &Scene,
        items: &[DrawItem],
        selection: &[ObjectId],
    ) -> (Option<Bounds>, Option<Bounds>) {
        let roots: Vec<ObjectId> = scene.children(None).to_vec();
        (
            shown_bounds(scene, items, &roots),
            shown_bounds(scene, items, selection),
        )
    }
}
