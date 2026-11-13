//! Assemblies (Phase 12 milestone 12): many pieces on one canvas, copied
//! plain or linked, gathered into groups, and kept in order.
//!
//! - **Linked copies** share one shape: every piece with the same
//!   [`LinkId`] has the same path and does the same thing, and changing
//!   one ([`Drawing::with_path`], [`Drawing::with_kind`]) changes them
//!   all. Each keeps its own place, name, colour, height and mirroring.
//! - **Groups** gather pieces under one name. A group's pieces always sit
//!   together in the order, so the pieces list can show them under the
//!   group and moving the group up or down moves them as one.
//! - **The order** moves a piece past whole groups ([`Drawing::with_order`]):
//!   a piece in a group moves only within it.

use super::{Drawing, MAX_NAME, MAX_PIECES, Piece, PieceId, Problem, check_piece};
use crate::path::Change;
use std::collections::BTreeMap;
use std::ops::Range;

/// The link shared by pieces with one shape (linked copies). A piece on
/// its own has a link no other piece has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkId(pub u32);

/// A group's id in its drawing: never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u32);

/// A group of pieces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Its id.
    pub id: GroupId,
    /// Its name, shown in the pieces list.
    pub name: String,
}

/// Pieces copied from a drawing, to paste into it or another one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PieceCopies {
    /// The pieces, in their drawing's order.
    pub pieces: Vec<Piece>,
    /// The groups they were in.
    pub groups: Vec<Group>,
}

impl PieceCopies {
    /// Whether nothing was copied.
    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// The middle of the copied pieces where they were placed.
    pub fn middle(&self) -> Option<glam::Vec2> {
        bounds_middle(self.pieces.iter())
    }
}

/// The middle of the rectangle round `pieces` where they are placed.
pub(crate) fn bounds_middle<'a>(pieces: impl Iterator<Item = &'a Piece>) -> Option<glam::Vec2> {
    let mut lo = glam::Vec2::splat(f32::INFINITY);
    let mut hi = glam::Vec2::splat(f32::NEG_INFINITY);
    for p in pieces {
        if let Some((a, b)) = p.placed().bounds() {
            lo = lo.min(a);
            hi = hi.max(b);
        }
    }
    (lo.is_finite() && hi.is_finite()).then(|| (lo + hi) * 0.5)
}

impl Drawing {
    /// Its groups.
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// Group `id`.
    pub fn group(&self, id: GroupId) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// The pieces in group `id`, in order.
    pub fn group_pieces(&self, id: GroupId) -> Vec<PieceId> {
        self.pieces
            .iter()
            .filter(|p| p.group == Some(id))
            .map(|p| p.id)
            .collect()
    }

    /// How many pieces share piece `id`'s shape (itself included; 1 when
    /// it isn't linked).
    pub fn linked_count(&self, id: PieceId) -> usize {
        self.piece(id).map_or(0, |piece| {
            self.pieces.iter().filter(|p| p.link == piece.link).count()
        })
    }

    /// The middle of pieces `ids` where they are placed (their
    /// rectangle's middle).
    pub fn middle_of(&self, ids: &[PieceId]) -> Option<glam::Vec2> {
        bounds_middle(self.pieces.iter().filter(|p| ids.contains(&p.id)))
    }

    /// Pieces `ids` (those still there), in order, with their groups, to
    /// paste.
    pub fn copied(&self, ids: &[PieceId]) -> PieceCopies {
        let pieces: Vec<Piece> = self
            .pieces
            .iter()
            .filter(|p| ids.contains(&p.id))
            .cloned()
            .collect();
        let groups = self
            .groups
            .iter()
            .filter(|g| pieces.iter().any(|p| p.group == Some(g.id)))
            .cloned()
            .collect();
        PieceCopies { pieces, groups }
    }

    /// The same drawing with `copies` pasted on top, each placed again by
    /// `by`, and the new pieces' ids. Copies keep their names, colours,
    /// heights and groups (as new groups). With `linked`, each copy is
    /// linked to the piece it was copied from when that piece's shape is
    /// on this canvas, and copies from one shape are linked to each other;
    /// otherwise each copy's shape is its own.
    pub fn with_pasted(
        &self,
        copies: &PieceCopies,
        linked: bool,
        by: &Change,
    ) -> Result<(Self, Vec<PieceId>), Problem> {
        if self.pieces.len() + copies.pieces.len() > MAX_PIECES {
            return Err(Problem::TooMany);
        }
        let mut out = self.clone();
        let mut links = BTreeMap::new();
        let mut groups = BTreeMap::new();
        let mut ids = Vec::with_capacity(copies.pieces.len());
        for copy in &copies.pieces {
            let id = out.take_id();
            let link = if !linked {
                LinkId(id)
            } else if let Some(link) = links.get(&copy.link) {
                *link
            } else {
                let here = self
                    .pieces
                    .iter()
                    .any(|p| p.link == copy.link && p.same_shape(copy));
                let link = if here { copy.link } else { LinkId(id) };
                links.insert(copy.link, link);
                link
            };
            let group = copy.group.map(|old| {
                *groups.entry(old).or_insert_with(|| {
                    let new = GroupId(out.take_id());
                    let name = copies.groups.iter().find(|g| g.id == old).map_or_else(
                        || words::tr(words::text!("Group")).to_string(),
                        |g| g.name.clone(),
                    );
                    out.groups.push(Group { id: new, name });
                    new
                })
            });
            let piece = Piece {
                id: PieceId(id),
                place: copy.place.then(by),
                link,
                group,
                seed: super::first_seed(PieceId(id)),
                ..copy.clone()
            };
            check_piece(&piece)?;
            out.pieces.push(piece);
            ids.push(PieceId(id));
        }
        out.check_count()?;
        out.check_groups()?;
        Ok((out, ids))
    }

    /// The same drawing with piece `id` given a shape of its own: no
    /// longer linked to the others (which stay linked to each other).
    pub fn with_unlinked(&self, id: PieceId) -> Result<Self, Problem> {
        self.piece(id).ok_or(Problem::Gone)?;
        let mut out = self.clone();
        let link = LinkId(out.take_id());
        if let Some(p) = out.pieces.iter_mut().find(|p| p.id == id) {
            p.link = link;
        }
        Ok(out)
    }

    /// The same drawing with pieces `ids` gathered into a new group named
    /// `name` (leaving any group they were in), and its id. The pieces
    /// are brought together in the order where the topmost of them was.
    pub fn with_group(&self, ids: &[PieceId], name: &str) -> Result<(Self, GroupId), Problem> {
        if name.trim().is_empty() || name.len() > MAX_NAME {
            return Err(Problem::BadName);
        }
        if ids.is_empty() || ids.iter().any(|id| self.piece(*id).is_none()) {
            return Err(Problem::Gone);
        }
        let mut out = self.clone();
        let group = GroupId(out.take_id());
        let top = out
            .pieces
            .iter()
            .rposition(|p| ids.contains(&p.id))
            .unwrap_or(0);
        let below = out.pieces[..top]
            .iter()
            .filter(|p| !ids.contains(&p.id))
            .count();
        let (mut chosen, rest): (Vec<Piece>, Vec<Piece>) =
            out.pieces.into_iter().partition(|p| ids.contains(&p.id));
        for p in &mut chosen {
            p.group = Some(group);
        }
        // Never split another group.
        let mut at = below;
        while at > 0 && at < rest.len() && rest[at - 1].group.is_some() {
            if rest[at].group != rest[at - 1].group {
                break;
            }
            at += 1;
        }
        let mut pieces = rest;
        pieces.splice(at..at, chosen);
        out.pieces = pieces;
        out.groups.push(Group {
            id: group,
            name: name.to_string(),
        });
        out.drop_empty_groups();
        Ok((out, group))
    }

    /// The same drawing with group `id` taken apart (its pieces stay
    /// where they are).
    pub fn without_group(&self, id: GroupId) -> Result<Self, Problem> {
        self.group(id).ok_or(Problem::Gone)?;
        let mut out = self.clone();
        for p in out.pieces.iter_mut().filter(|p| p.group == Some(id)) {
            p.group = None;
        }
        out.groups.retain(|g| g.id != id);
        Ok(out)
    }

    /// The same drawing with group `id` named `name`.
    pub fn with_group_name(&self, id: GroupId, name: &str) -> Result<Self, Problem> {
        if name.trim().is_empty() || name.len() > MAX_NAME {
            return Err(Problem::BadName);
        }
        let mut out = self.clone();
        let g = out
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or(Problem::Gone)?;
        g.name = name.to_string();
        Ok(out)
    }

    /// The same drawing with piece `id` moved `by` places up (positive:
    /// toward the top, drawn later) or down the order. A piece in a group
    /// moves within it; one on its own moves past whole groups.
    pub fn with_order(&self, id: PieceId, by: i32) -> Result<Self, Problem> {
        let at = self
            .pieces
            .iter()
            .position(|p| p.id == id)
            .ok_or(Problem::Gone)?;
        let mut out = self.clone();
        if self.pieces[at].group.is_some() {
            let block = self.unit_at(at);
            let to = (at as i64 + i64::from(by)).clamp(block.start as i64, block.end as i64 - 1);
            let piece = out.pieces.remove(at);
            out.pieces.insert(to as usize, piece);
        } else {
            out.move_unit(at..at + 1, by);
        }
        Ok(out)
    }

    /// The same drawing with group `id` moved `by` places up or down the
    /// order, its pieces as one, past pieces and whole groups.
    pub fn with_group_order(&self, id: GroupId, by: i32) -> Result<Self, Problem> {
        let first = self
            .pieces
            .iter()
            .position(|p| p.group == Some(id))
            .ok_or(Problem::Gone)?;
        let mut out = self.clone();
        out.move_unit(self.unit_at(first), by);
        Ok(out)
    }

    /// The same drawing with piece `id` coloured `colour` (each part
    /// brought within 0 to 1).
    pub fn with_colour(&self, id: PieceId, colour: [f32; 3]) -> Result<Self, Problem> {
        if !colour.iter().all(|c| c.is_finite()) {
            return Err(Problem::OutOfPlace);
        }
        self.changing(id, |p| p.colour = colour.map(|c| c.clamp(0.0, 1.0)))
    }

    /// The same drawing with piece `id` standing `height` high (brought
    /// within 0 to 1).
    pub fn with_height(&self, id: PieceId, height: f32) -> Result<Self, Problem> {
        if !height.is_finite() {
            return Err(Problem::OutOfPlace);
        }
        self.changing(id, |p| p.height = height.clamp(0.0, 1.0))
    }

    /// The same drawing with pieces `ids` flipped about their middle:
    /// across (left for right) or up and down.
    pub fn flipped(&self, ids: &[PieceId], across: bool) -> Result<Self, Problem> {
        let mut lo = glam::Vec2::splat(f32::INFINITY);
        let mut hi = glam::Vec2::splat(f32::NEG_INFINITY);
        for id in ids {
            let (a, b) = self
                .piece(*id)
                .ok_or(Problem::Gone)?
                .placed()
                .bounds()
                .ok_or(Problem::NoInside)?;
            lo = lo.min(a);
            hi = hi.max(b);
        }
        if ids.is_empty() {
            return Ok(self.clone());
        }
        let scale = if across {
            glam::Vec2::new(-1.0, 1.0)
        } else {
            glam::Vec2::new(1.0, -1.0)
        };
        self.with_change(ids, &Change::scaled((lo + hi) * 0.5, scale))
    }

    /// Takes the next id (for a piece, a group or a link).
    pub(super) fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Drops groups with no pieces.
    pub(super) fn drop_empty_groups(&mut self) {
        let pieces = &self.pieces;
        self.groups
            .retain(|g| pieces.iter().any(|p| p.group == Some(g.id)));
    }

    /// Whether linked pieces share one shape, do the same thing and have
    /// the same edge, varying and vein.
    pub(super) fn check_links(&self) -> Result<(), Problem> {
        let mut first: BTreeMap<LinkId, &Piece> = BTreeMap::new();
        for p in &self.pieces {
            let one = first.entry(p.link).or_insert(p);
            if !one.same_shape(p) {
                return Err(Problem::BadLink);
            }
        }
        Ok(())
    }

    /// Whether groups are well made: ids unique, every piece's group
    /// there, names kept, each group's pieces together and none empty.
    pub(super) fn check_groups(&self) -> Result<(), Problem> {
        let mut seen = std::collections::BTreeSet::new();
        for g in &self.groups {
            let bad_name = g.name.trim().is_empty() || g.name.len() > MAX_NAME;
            if g.id.0 == 0 || g.id.0 >= self.next_id || !seen.insert(g.id) || bad_name {
                return Err(Problem::BadGroup);
            }
            let places: Vec<usize> = self
                .pieces
                .iter()
                .enumerate()
                .filter(|(_, p)| p.group == Some(g.id))
                .map(|(i, _)| i)
                .collect();
            let together = places.windows(2).all(|w| w[1] == w[0] + 1);
            if places.is_empty() || !together {
                return Err(Problem::BadGroup);
            }
        }
        if self
            .pieces
            .iter()
            .any(|p| p.group.is_some_and(|g| !seen.contains(&g)))
        {
            return Err(Problem::BadGroup);
        }
        Ok(())
    }

    /// The pieces moving with the one at `index`: its whole group, or it
    /// alone.
    fn unit_at(&self, index: usize) -> Range<usize> {
        let Some(group) = self.pieces[index].group else {
            return index..index + 1;
        };
        let mut start = index;
        while start > 0 && self.pieces[start - 1].group == Some(group) {
            start -= 1;
        }
        let mut end = index + 1;
        while end < self.pieces.len() && self.pieces[end].group == Some(group) {
            end += 1;
        }
        start..end
    }

    /// Moves the pieces at `unit` past `by` units (pieces or whole
    /// groups) up (positive) or down.
    fn move_unit(&mut self, mut unit: Range<usize>, by: i32) {
        for _ in 0..by.unsigned_abs() {
            if by > 0 {
                if unit.end >= self.pieces.len() {
                    break;
                }
                let next = self.unit_at(unit.end);
                self.pieces[unit.start..next.end].rotate_left(unit.len());
                unit = unit.start + next.len()..next.end;
            } else {
                if unit.start == 0 {
                    break;
                }
                let prev = self.unit_at(unit.start - 1);
                self.pieces[prev.start..unit.end].rotate_right(unit.len());
                unit = prev.start..prev.start + unit.len();
            }
        }
    }
}

#[cfg(test)]
mod tests;
