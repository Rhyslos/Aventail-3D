//! Corners where plain polygons fan between bevelled edges
//! ([decision 0156](../../../../docs/decisions/0156-game-bevel.md)).

use super::{Slides, Wedge, slide_by, towards};
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::BTreeSet;

/// Where two or more bevelled edges meet at `v` with plain polygons
/// fanned between them (as booleans leave a box's corners: a fan of
/// triangles in each face), the plain edges between two plain polygons
/// slide too, though no polygon asks them to: the corner is cut away, and
/// the point kept would fold the corner patch round it (found by the game
/// bevel, decision 0156). Each slides out to where it leaves every bevel's
/// strip that lies in its polygon's plane; with none in the plane, as far
/// as the slides asked at the point on average. Only for points the
/// polygons close all the way round.
pub(super) fn slide_plain_fans(
    original: &Mesh,
    v: VertexId,
    wedges: &[Wedge],
    bevelled: &impl Fn(VertexId, VertexId) -> bool,
    width: impl Fn(EdgeKey, FaceId) -> f32,
    asks: &mut Slides,
    must_fit: &mut impl FnMut(f32, VertexId, VertexId),
) {
    let here: BTreeSet<VertexId> = wedges
        .iter()
        .flat_map(|w| [w.before, w.after])
        .filter(|far| bevelled(*far, v))
        .collect();
    if here.len() < 2 || asks.0.is_empty() {
        return;
    }
    let (sum, count) = asks
        .0
        .values()
        .fold((0.0, 0), |(s, c), (sum, count)| (s + sum, c + count));
    let average = sum / count.max(1) as f32;
    for w in wedges {
        if bevelled(w.before, v) || bevelled(v, w.after) {
            continue;
        }
        let normal = original.face_normal(w.face);
        for far in [w.before, w.after] {
            if asks.has(far) {
                continue;
            }
            let d = towards(original, v, far);
            let mut along = 0.0f32;
            for e in &here {
                let d_e = towards(original, v, *e);
                if d_e.dot(normal).abs() > 0.1 {
                    continue;
                }
                let perp = normal.cross(d_e).normalize_or_zero();
                along = along.max(slide_by(width(EdgeKey::new(v, *e), w.face), d.dot(perp)));
            }
            if along <= 0.0 {
                along = average;
            }
            if along.is_finite() && along > 0.0 {
                must_fit(along, v, far);
                asks.ask(far, along);
            }
        }
    }
}
