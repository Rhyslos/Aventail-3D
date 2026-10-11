//! Selecting with the mouse and keys ([decision 0029](../../../docs/decisions/0029-selection.md)):
//! the select tools, the gestures the viewport hands over, and what they
//! do to the document.
//!
//! In the Model mode gestures select whole objects (not an undo step, as
//! in the Object Manager). In the Points, Edges and Polygons modes they
//! select elements of the selected objects, and each change is an undo
//! step.
//!
//! The controls, chosen by the project lead:
//!
//! | Action | Control |
//! |---|---|
//! | Select | Click (replaces); click on nothing to unselect |
//! | Add or remove one | Ctrl + click |
//! | Box, lasso, brush | Drag replaces, Shift + drag adds, Ctrl + drag removes |
//! | Switch Box → Lasso → Brush | `Q` |
//! | Loop, ring | Double-click, Shift + double-click (Ctrl adds) |
//! | Grow, shrink | Ctrl + wheel (with the brush: its size) |
//! | Select all, invert | Ctrl + A, Ctrl + I |
//! | Unselect elements | Esc |

use crate::components::{ViewPointer, kind_for};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use egui::{Key, KeyboardShortcut, Modifiers};
use glam::Vec2;
use mesh::{Topology, selection as ops};
use scene::{Combine, Document, Element, ElementKind, ElementSelection, ObjectId};
use std::collections::BTreeMap;
use tools::{Camera, Hovered, PickScene, Region};

/// The key that switches between the select tools.
pub const SELECT_TOOL_KEY: Key = Key::Q;
/// Select all: every object, or every element of the edited objects.
pub const SELECT_ALL_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
/// Invert the selection.
pub const INVERT_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::I);

/// The brush's radius as standard, in screen points.
pub const BRUSH_RADIUS: f32 = 20.0;
/// The smallest brush.
pub const BRUSH_MIN: f32 = 4.0;
/// The largest brush.
pub const BRUSH_MAX: f32 = 200.0;
/// How much one wheel step changes the brush (a factor).
pub const BRUSH_STEP: f32 = 1.15;

/// What a drag with the left mouse button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectTool {
    /// Drag a box.
    #[default]
    Box,
    /// Draw around what to select.
    Lasso,
    /// Paint over what to select (live selection).
    Brush,
}

impl SelectTool {
    /// Every tool, in the order `Q` steps through them.
    pub const ALL: [Self; 3] = [Self::Box, Self::Lasso, Self::Brush];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Box => "Box selection",
            Self::Lasso => "Lasso selection",
            Self::Brush => "Live selection",
        }
    }

    /// What it does, in plain words.
    pub fn description(self) -> &'static str {
        match self {
            Self::Box => "drag a box around what to select",
            Self::Lasso => "draw a loop around what to select",
            Self::Brush => "paint over what to select; Ctrl + wheel changes the brush size",
        }
    }

    /// The tool `Q` goes to next.
    pub fn next(self) -> Self {
        match self {
            Self::Box => Self::Lasso,
            Self::Lasso => Self::Brush,
            Self::Brush => Self::Box,
        }
    }
}

/// How a click combines with what is selected: Ctrl flips, Shift adds,
/// otherwise it replaces.
pub(crate) fn click_combine(keys: Modifiers) -> Combine {
    if keys.command {
        Combine::Toggle
    } else if keys.shift {
        Combine::Add
    } else {
        Combine::Replace
    }
}

/// How a drag combines with what is selected: Ctrl removes, Shift adds,
/// otherwise it replaces.
pub(crate) fn drag_combine(keys: Modifiers) -> Combine {
    if keys.command {
        Combine::Remove
    } else if keys.shift {
        Combine::Add
    } else {
        Combine::Replace
    }
}

/// Something the user did in a view to select.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Gesture {
    /// A click.
    Click { pointer: ViewPointer, how: Combine },
    /// A double-click: the loop, or with `ring` the ring, through the
    /// element under the mouse.
    DoubleClick {
        pointer: ViewPointer,
        ring: bool,
        how: Combine,
    },
    /// A finished box or lasso, or a dab of the brush. A brush stroke's
    /// dabs share a `stroke` number, so the stroke undoes in one go.
    Region {
        camera: Camera,
        size: Vec2,
        regions: Vec<Region>,
        how: Combine,
        stroke: Option<u64>,
    },
    /// Ctrl + wheel: grow (positive) or shrink (negative) by this many
    /// steps.
    Grow(i32),
}

/// Brush strokes' undo merge keys: kept apart from the Attribute
/// Manager's gesture numbers.
const STROKE_KEYS: u64 = 1 << 62;

/// Everything selecting needs, for one frame.
pub(crate) struct Selector<'a> {
    pub(crate) document: &'a mut Document,
    pub(crate) drawing: &'a SceneDrawing,
    pub(crate) scene: &'a PickScene,
    pub(crate) mode: Mode,
    pub(crate) visible_only: bool,
}

impl Selector<'_> {
    /// Carries out a gesture. Returns a message for the status bar, if it
    /// needs one.
    pub(crate) fn gesture(&mut self, gesture: Gesture) -> Option<String> {
        match (kind_for(self.mode), gesture) {
            (None, Gesture::Click { pointer, how }) => {
                let found = self
                    .scene
                    .object_under(&pointer.camera, pointer.size, pointer.at)
                    .and_then(|i| self.drawing.items.get(i))
                    .map(|item| item.id);
                self.combine_objects(found, how);
                None
            }
            (
                None,
                Gesture::Region {
                    camera,
                    size,
                    regions,
                    how,
                    ..
                },
            ) => {
                let mut found = Vec::new();
                for region in &regions {
                    for index in self
                        .scene
                        .objects_in(&camera, size, region, self.visible_only)
                    {
                        if let Some(item) = self.drawing.items.get(index)
                            && !found.contains(&item.id)
                        {
                            found.push(item.id);
                        }
                    }
                }
                self.objects_from_region(found, how);
                None
            }
            (None, Gesture::DoubleClick { .. }) => None,
            (None, Gesture::Grow(_)) => Some(
                "Grow and shrink work on points, edges and polygons: press 1, 2 or 3 first.".into(),
            ),
            (Some(kind), Gesture::Click { pointer, how }) => {
                let found = self
                    .scene
                    .under(kind, &pointer.camera, pointer.size, pointer.at);
                self.elements(kind, found.into_iter().collect(), how, None)
            }
            (
                Some(kind),
                Gesture::Region {
                    camera,
                    size,
                    regions,
                    how,
                    stroke,
                },
            ) => {
                let mut found = Vec::new();
                for region in &regions {
                    for hit in
                        self.scene
                            .elements_in(kind, &camera, size, region, self.visible_only)
                    {
                        if !found.contains(&hit) {
                            found.push(hit);
                        }
                    }
                }
                self.elements(kind, found, how, stroke.map(|s| STROKE_KEYS | s))
            }
            (Some(kind), Gesture::DoubleClick { pointer, ring, how }) => {
                self.loop_or_ring(kind, pointer, ring, how)
            }
            (Some(kind), Gesture::Grow(steps)) => self.grow(kind, steps),
        }
    }

    /// Select all (Ctrl + A).
    pub(crate) fn select_all(&mut self) -> Option<String> {
        match kind_for(self.mode) {
            None => {
                let all: Vec<ObjectId> = self
                    .document
                    .scene()
                    .walk()
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect();
                self.document.set_selection(all);
                None
            }
            Some(kind) => self.each_edited(kind, "Select all", |mesh, topology, _| {
                ops::all(mesh, topology, kind)
            }),
        }
    }

    /// Invert the selection (Ctrl + I).
    pub(crate) fn invert(&mut self) -> Option<String> {
        match kind_for(self.mode) {
            None => {
                let document = &self.document;
                let rest: Vec<ObjectId> = document
                    .scene()
                    .walk()
                    .into_iter()
                    .map(|(id, _)| id)
                    .filter(|id| !document.is_selected(*id))
                    .collect();
                self.document.set_selection(rest);
                None
            }
            Some(kind) => self.each_edited(kind, "Invert", |mesh, topology, selected| {
                ops::inverted(mesh, topology, selected, kind)
            }),
        }
    }

    /// Unselects the edited objects' elements of the mode's kind (Esc).
    /// Returns whether it was in an element mode.
    pub(crate) fn clear_elements(&mut self) -> bool {
        let Some(kind) = kind_for(self.mode) else {
            return false;
        };
        self.each_edited(kind, "Deselect", |_, _, _| Vec::new());
        true
    }

    /// Grows or shrinks the selection by `steps`.
    fn grow(&mut self, kind: ElementKind, steps: i32) -> Option<String> {
        if steps == 0 {
            return None;
        }
        let label = if steps > 0 { "Grow" } else { "Shrink" };
        self.each_edited(kind, label, |mesh, topology, selected| {
            let mut current = selected.clone();
            for _ in 0..steps.unsigned_abs() {
                let next = if steps > 0 {
                    ops::grown(mesh, topology, &current, kind)
                } else {
                    ops::shrunk(mesh, topology, &current, kind)
                };
                current.combine(kind, next, Combine::Replace);
            }
            current.of_kind(kind)
        })
    }

    /// Replaces each edited object's elements of `kind` with what `work`
    /// works out from its mesh and current selection, as one undo step
    /// named from `action` ("Grow (12 points selected)").
    fn each_edited(
        &mut self,
        kind: ElementKind,
        action: &str,
        work: impl Fn(&scene::Mesh, &Topology, &ElementSelection) -> Vec<Element>,
    ) -> Option<String> {
        let none = ElementSelection::new();
        let mut changes = Vec::new();
        for item in self.drawing.items.iter().filter(|i| i.selected) {
            let topology = Topology::new(&item.mesh);
            let current = self.document.elements(item.id).unwrap_or(&none);
            let mut next = current.clone();
            let chosen = work(&item.mesh, &topology, current);
            next.combine(kind, chosen, Combine::Replace);
            changes.push((item.id, next));
        }
        if changes.is_empty() {
            return Some(nothing_to_edit(kind));
        }
        self.record(kind, changes, action, None);
        None
    }

    /// Combines `found` elements with the edited objects' selections: a
    /// replace also unselects the elements of edited objects where
    /// nothing was found.
    fn elements(
        &mut self,
        kind: ElementKind,
        found: Vec<Hovered>,
        how: Combine,
        merge_key: Option<u64>,
    ) -> Option<String> {
        let mut by_object: BTreeMap<ObjectId, Vec<Element>> = BTreeMap::new();
        for hit in found {
            if let Some(item) = self.drawing.items.get(hit.object) {
                by_object.entry(item.id).or_default().push(hit.element);
            }
        }
        let edited: Vec<ObjectId> = self
            .drawing
            .items
            .iter()
            .filter(|i| i.selected)
            .map(|i| i.id)
            .collect();
        if edited.is_empty() {
            return Some(nothing_to_edit(kind));
        }
        let mut changes = Vec::new();
        for id in edited {
            let hits = by_object.remove(&id).unwrap_or_default();
            if hits.is_empty() && how != Combine::Replace {
                continue;
            }
            let mut next = self.document.elements(id).cloned().unwrap_or_default();
            next.combine(kind, hits, how);
            changes.push((id, next));
        }
        let action = match how {
            Combine::Remove => "Deselect",
            _ => "Select",
        };
        self.record(kind, changes, action, merge_key);
        None
    }

    /// The loop or ring through the element under the mouse.
    fn loop_or_ring(
        &mut self,
        kind: ElementKind,
        pointer: ViewPointer,
        ring: bool,
        how: Combine,
    ) -> Option<String> {
        let ViewPointer { camera, size, at } = pointer;
        // Which edge sets the direction: the one under the mouse; in the
        // Points mode the one passing nearest; in the Polygons mode the
        // hovered polygon's edge nearest the mouse.
        let (index, edge, face) = match kind {
            ElementKind::Edges => match self.scene.under(kind, &camera, size, at)? {
                Hovered {
                    object,
                    element: Element::Edge(edge),
                } => (object, edge, None),
                _ => return None,
            },
            ElementKind::Points => {
                let reach = 2.0 * tools::components::EDGE_RADIUS;
                let (object, edge) = self.scene.edge_near(&camera, size, at, reach)?;
                (object, edge, None)
            }
            ElementKind::Polygons => match self.scene.under(kind, &camera, size, at)? {
                Hovered {
                    object,
                    element: Element::Polygon(face),
                } => {
                    let edge = self
                        .scene
                        .nearest_edge_of(object, face, &camera, size, at)?;
                    (object, edge, Some(face))
                }
                _ => return None,
            },
        };
        let item = self.drawing.items.get(index)?;
        let topology = Topology::new(&item.mesh);
        let elements = if ring {
            ops::ring_through(&item.mesh, &topology, edge, face, kind)
        } else {
            ops::loop_through(&topology, edge, kind)
        };
        let mut next = self.document.elements(item.id).cloned().unwrap_or_default();
        next.combine(kind, elements, how);
        let action = if ring { "Select ring" } else { "Select loop" };
        self.record(kind, vec![(item.id, next)], action, None);
        None
    }

    /// Records the changes as one undo step, named with what is selected
    /// afterwards: "Select loop (8 edges selected)".
    fn record(
        &mut self,
        kind: ElementKind,
        changes: Vec<(ObjectId, ElementSelection)>,
        action: &str,
        merge_key: Option<u64>,
    ) {
        let after: usize = self
            .drawing
            .items
            .iter()
            .filter(|i| i.selected)
            .map(|i| {
                changes
                    .iter()
                    .find(|(id, _)| *id == i.id)
                    .map(|(_, s)| s)
                    .or_else(|| self.document.elements(i.id))
                    .map_or(0, |s| s.count(kind))
            })
            .sum();
        let label = format!("{action} ({} selected)", count(after, kind));
        self.document.select_elements(changes, label, merge_key);
    }

    /// Selects objects for a click: `found` is the object clicked, if any.
    fn combine_objects(&mut self, found: Option<ObjectId>, how: Combine) {
        match (found, how) {
            (None, Combine::Replace) => self.document.clear_selection(),
            (None, _) => {}
            (Some(id), Combine::Replace) => self.document.set_selection([id]),
            (Some(id), Combine::Toggle) => self.document.toggle_selected(id),
            (Some(id), Combine::Add) => {
                if !self.document.is_selected(id) {
                    self.document.toggle_selected(id);
                }
            }
            (Some(id), Combine::Remove) => {
                if self.document.is_selected(id) {
                    self.document.toggle_selected(id);
                }
            }
        }
    }

    /// Selects objects for a box, lasso or brush.
    fn objects_from_region(&mut self, found: Vec<ObjectId>, how: Combine) {
        let mut selection = self.document.selection();
        match how {
            Combine::Replace => selection = found,
            Combine::Add | Combine::Toggle => {
                for id in found {
                    if !selection.contains(&id) {
                        selection.push(id);
                    }
                }
            }
            Combine::Remove => selection.retain(|id| !found.contains(id)),
        }
        self.document.set_selection(selection);
    }
}

/// "1 point" or "12 points".
pub(crate) fn count(n: usize, kind: ElementKind) -> String {
    if n == 1 {
        format!("1 {}", kind.singular())
    } else {
        format!("{n} {}", kind.plural())
    }
}

fn nothing_to_edit(kind: ElementKind) -> String {
    format!(
        "Select an object first: its {} can then be selected.",
        kind.plural()
    )
}

#[cfg(test)]
mod tests;
