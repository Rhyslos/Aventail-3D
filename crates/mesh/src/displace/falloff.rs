//! How much of Displace each point gets when it is limited to chosen
//! points ([decision 0157](../../../../docs/decisions/0157-displace.md)):
//! all of it on the chosen points, fading smoothly to none at the
//! **Falloff** distance from them, measured **along the surface** (along
//! the edges, the shortest way), so it never leaks across a gap to a part
//! of the shape that only lies near.

use crate::{Mesh, VertexId};
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

/// Each point's share (by id number, 0 to 1): 1 on the points in `only`,
/// fading to 0 at `falloff` centimetres from them along the edges, 0
/// beyond (and everywhere but on them when `falloff` is 0).
pub(crate) fn weights(mesh: &Mesh, only: &BTreeSet<VertexId>, falloff: f32) -> Vec<f32> {
    let slots = mesh.point_slot_count();
    let mut weights = vec![0.0_f32; slots];
    let mut distance = vec![f32::INFINITY; slots];
    // Smallest distance first; distances are never negative, so their
    // bits sort as the numbers do.
    let mut queue: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    for v in only {
        let at = v.get() as usize;
        if mesh.position(*v).is_some() && at < slots {
            distance[at] = 0.0;
            queue.push(Reverse((0, v.get())));
        }
    }
    if falloff > 0.0 {
        let neighbours = neighbours(mesh);
        while let Some(Reverse((bits, at))) = queue.pop() {
            let d = f32::from_bits(bits);
            let at = at as usize;
            if d > distance[at] {
                continue;
            }
            let Some(here) = mesh.position(VertexId::from_raw(at as u32)) else {
                continue;
            };
            for &next in &neighbours[at] {
                let Some(there) = mesh.position(VertexId::from_raw(next)) else {
                    continue;
                };
                let step = here.distance(there);
                let further = d + step;
                if !further.is_finite() || further >= falloff {
                    continue;
                }
                let slot = next as usize;
                if further < distance[slot] {
                    distance[slot] = further;
                    queue.push(Reverse((further.to_bits(), next)));
                }
            }
        }
    }
    for (weight, d) in weights.iter_mut().zip(&distance) {
        *weight = if *d <= 0.0 {
            1.0
        } else if falloff > 0.0 && *d < falloff {
            let t = 1.0 - d / falloff;
            t * t * (3.0 - 2.0 * t)
        } else {
            0.0
        };
    }
    weights
}

/// Each point's neighbours along the polygons' edges (by id number), each
/// once, in order.
fn neighbours(mesh: &Mesh) -> Vec<Vec<u32>> {
    let mut lists: Vec<Vec<u32>> = vec![Vec::new(); mesh.point_slot_count()];
    for (_, corners) in mesh.faces() {
        for i in 0..corners.len() {
            let (a, b) = (corners[i].get(), corners[(i + 1) % corners.len()].get());
            if a == b {
                continue;
            }
            if let Some(list) = lists.get_mut(a as usize) {
                list.push(b);
            }
            if let Some(list) = lists.get_mut(b as usize) {
                list.push(a);
            }
        }
    }
    for list in &mut lists {
        list.sort_unstable();
        list.dedup();
    }
    lists
}
