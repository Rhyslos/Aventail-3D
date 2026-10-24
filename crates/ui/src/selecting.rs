//! Selecting with the mouse and keys ([decision 0029](../../../docs/decisions/0029-selection.md)):
//! the select tools, the gestures the viewport hands over, and what they
//! do to the document.
//!
//! In the Model mode gestures select whole objects; in the Points, Edges
//! and Polygons modes, elements of the selected objects. Every change is
//! an undo step.
//!
//! The controls, chosen by the project lead. Which keys add and remove
//! follows one [`SelectKeys`] pattern, chosen in Settings:
//!
//! | Action | Control (standard pattern) |
//! |---|---|
//! | Select | Click or drag (replaces); click on nothing to unselect |
//! | Add | Ctrl + click or drag (Ctrl + click on something selected unselects it) |
//! | Remove | Shift + click or drag |
//! | Switch Box → Lasso → Brush | `Q` |
//! | Loop, ring | Double-click, Alt + double-click (with the add and remove keys too) |
//! | Grow, shrink | Ctrl + wheel (with the brush: its size; with Bevel or Loop cut: its count) |
//! | Select all, invert | Ctrl + A, Ctrl + I |
//! | Unselect elements | Esc |

use crate::components::{ViewPointer, kind_for};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use egui::{Key, KeyboardShortcut, Modifiers};
use glam::Vec2;
use mesh::{Topology, selection as ops};
use scene::{Combine, Document, Element, ElementKind, ElementSelection, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tools::{Camera, Hovered, PickScene, Region};

/// The key that switches between the select tools.
pub const SELECT_TOOL_KEY: Key = Key::Q;
/// Select all: every object, or every element of the edited objects.
pub const SELECT_ALL_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
/// Invert the selection.
pub const INVERT_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::I);
/// Select connected: the whole part the selection is on (the project
/// lead's choice, decision 0118).
pub const CONNECTED_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::A);

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
    /// Paint over what to select (Paint select).
    Brush,
}

impl SelectTool {
    /// Every tool, in the order `Q` steps through them.
    pub const ALL: [Self; 3] = [Self::Box, Self::Lasso, Self::Brush];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Box => tr("Box selection"),
            Self::Lasso => tr("Lasso selection"),
            Self::Brush => tr("Paint select"),
        }
    }

    /// What it does, in plain words.
    pub fn description(self) -> &'static str {
        match self {
            Self::Box => tr("drag a box around what to select"),
            Self::Lasso => tr("draw a loop around what to select"),
            Self::Brush => tr("paint over what to select; Ctrl + wheel changes the brush size"),
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

/// The keys that add to and take from the selection: one pattern for
/// clicks, drags, the brush and double-clicks, chosen in Settings
/// (decision 0029). A plain click or drag always replaces the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectKeys {
    /// Ctrl adds, Shift removes (the project lead's choice; standard).
    #[default]
    CtrlAddsShiftRemoves,
    /// Shift adds, Ctrl removes.
    ShiftAddsCtrlRemoves,
    /// Ctrl flips: what is selected is unselected and the rest selected.
    CtrlFlips,
    /// Ctrl adds, Alt removes. Alt + drag still moves the view while
    /// "Alt + mouse works in every scheme" is on.
    CtrlAddsAltRemoves,
}

impl SelectKeys {
    /// Every pattern, in the order Settings lists them.
    pub const ALL: [Self; 4] = [
        Self::CtrlAddsShiftRemoves,
        Self::ShiftAddsCtrlRemoves,
        Self::CtrlFlips,
        Self::CtrlAddsAltRemoves,
    ];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::CtrlAddsShiftRemoves => tr("Ctrl adds, Shift removes"),
            Self::ShiftAddsCtrlRemoves => tr("Shift adds, Ctrl removes"),
            Self::CtrlFlips => tr("Ctrl flips"),
            Self::CtrlAddsAltRemoves => tr("Ctrl adds, Alt removes"),
        }
    }

    /// What it means, in plain words.
    pub fn summary(self) -> &'static str {
        match self {
            Self::CtrlAddsShiftRemoves => {
                tr("Ctrl + click or drag adds; Shift + click or drag removes. \
                 Ctrl + click on something selected unselects it.")
            }
            Self::ShiftAddsCtrlRemoves => {
                tr("Shift + click or drag adds; Ctrl + click or drag removes. \
                 Shift + click on something selected unselects it.")
            }
            Self::CtrlFlips => tr(
                "Ctrl + click or drag flips: what was selected is unselected, \
                 and the rest selected.",
            ),
            Self::CtrlAddsAltRemoves => tr(
                "Ctrl + click or drag adds; Alt + click removes. Ctrl + click on \
                 something selected unselects it. Alt + drag moves the view while \
                 \"Alt + mouse works in every scheme\" is on.",
            ),
        }
    }

    /// How a click (`click`) or a drag combines with what is selected,
    /// with `keys` held. The add key on a click flips the one thing
    /// clicked, so clicking something selected unselects it.
    pub fn combine(self, keys: Modifiers, click: bool) -> Combine {
        let add = if click { Combine::Toggle } else { Combine::Add };
        let (adds, removes) = match self {
            Self::CtrlAddsShiftRemoves => (keys.command, keys.shift),
            Self::ShiftAddsCtrlRemoves => (keys.shift, keys.command),
            Self::CtrlFlips => {
                return if keys.command {
                    Combine::Toggle
                } else {
                    Combine::Replace
                };
            }
            Self::CtrlAddsAltRemoves => (keys.command, keys.alt),
        };
        if removes {
            Combine::Remove
        } else if adds {
            add
        } else {
            Combine::Replace
        }
    }

    /// How a double-click's loop or ring combines (Alt asks for the ring,
    /// so it never means "remove" here).
    pub fn combine_loop(self, keys: Modifiers) -> Combine {
        let keys = Modifiers { alt: false, ..keys };
        match self.combine(keys, false) {
            // Ctrl flips a loop as a whole: add it, or take it away.
            Combine::Toggle => Combine::Toggle,
            how => how,
        }
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
    /// What a flipping brush stroke has already flipped, so going over
    /// it again doesn't flip it back.
    pub(crate) stroke: &'a mut StrokeMemory,
}

/// The brush stroke in progress and what it has flipped.
#[derive(Debug, Default)]
pub(crate) struct StrokeMemory {
    stroke: u64,
    flipped: std::collections::BTreeSet<(ObjectId, Option<Element>)>,
}

impl StrokeMemory {
    /// Keeps only what `stroke` hasn't flipped yet, and remembers it.
    fn fresh<T>(
        &mut self,
        stroke: Option<u64>,
        how: Combine,
        found: Vec<T>,
        key: impl Fn(&T) -> (ObjectId, Option<Element>),
    ) -> Vec<T> {
        let (Some(stroke), Combine::Toggle) = (stroke, how) else {
            return found;
        };
        if self.stroke != stroke {
            self.stroke = stroke;
            self.flipped.clear();
        }
        found
            .into_iter()
            .filter(|f| self.flipped.insert(key(f)))
            .collect()
    }
}

impl Selector<'_> {
    /// Carries out a gesture. Returns a message for the status bar, if it
    /// needs one.
    pub(crate) fn gesture(&mut self, gesture: Gesture) -> Option<String> {
        match (kind_for(self.mode), gesture) {
            (None, Gesture::Click { pointer, how }) => {
                // A cutter, drawn as an outline, is picked only where no
                // other shape is under the mouse (decision 0135).
                let items = &self.drawing.items;
                let solid = |i: usize| items.get(i).is_some_and(|item| !item.outline);
                let found = self
                    .scene
                    .object_under_preferring(&pointer.camera, pointer.size, pointer.at, &solid)
                    .and_then(|i| self.drawing.items.get(i))
                    .map(|item| item.id)
                    // A curve's line has no surface to hit (decision 0138).
                    .or_else(|| curve_line_under(self.drawing, &pointer));
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
                    stroke,
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
                for region in &regions {
                    for id in curves_in(self.drawing, &camera, size, region) {
                        if !found.contains(&id) {
                            found.push(id);
                        }
                    }
                }
                let found = self.stroke.fresh(stroke, how, found, |id| (*id, None));
                self.objects_from_region(found, how, stroke.map(|s| STROKE_KEYS | s));
                None
            }
            (None, Gesture::DoubleClick { .. }) => None,
            (None, Gesture::Grow(_)) => Some(
                tr("Grow and shrink work on points, edges and polygons: press 1, 2 or 3 first.")
                    .into(),
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
                let mut seen = std::collections::HashSet::new();
                for region in &regions {
                    for hit in
                        self.scene
                            .elements_in(kind, &camera, size, region, self.visible_only)
                    {
                        if seen.insert(hit) {
                            found.push(hit);
                        }
                    }
                }
                let drawing = self.drawing;
                let found = self.stroke.fresh(stroke, how, found, |hit| {
                    let id = drawing.items.get(hit.object).map(|i| i.id);
                    (
                        id.unwrap_or(ObjectId::from_raw(u64::MAX)),
                        Some(hit.element),
                    )
                });
                self.elements(kind, found, how, stroke.map(|s| STROKE_KEYS | s))
            }
            (Some(kind), Gesture::DoubleClick { pointer, ring, how }) => self
                .toggle_curve_point(kind, pointer)
                .or_else(|| self.loop_or_ring(kind, pointer, ring, how)),
            (Some(kind), Gesture::Grow(steps)) => self.grow(kind, steps),
        }
    }

    /// A double-click on a curve's point (or one of its handles) in the
    /// Points mode switches it between sharp and smooth (decision 0138).
    /// `None` when the double-click isn't on a curve's part.
    fn toggle_curve_point(&mut self, kind: ElementKind, pointer: ViewPointer) -> Option<String> {
        if kind != ElementKind::Points {
            return None;
        }
        let hit = self
            .scene
            .under(kind, &pointer.camera, pointer.size, pointer.at)?;
        let item = self.drawing.items.get(hit.object)?;
        item.curve.as_ref()?;
        let Element::Point(v) = hit.element else {
            return None;
        };
        let part = scene::CurvePart::from_code(v.get());
        let curve = self
            .document
            .scene()
            .object(item.id)?
            .kind
            .curve()?
            .shape
            .curve();
        let smooth = !curve.point(part.point)?.smooth;
        let result = self.document.execute(scene::Command::EditCurve {
            id: item.id,
            edit: scene::CurveEdit::SetSmooth {
                points: vec![part.point],
                smooth,
            },
        });
        Some(match result {
            Ok(()) if smooth => tr(
                "Point made smooth: drag its handles to shape the bend. Double-click again \
                    for a sharp corner.",
            )
            .into(),
            Ok(()) => {
                tr("Point made sharp: a corner. Double-click again for a smooth bend.").into()
            }
            Err(why) => crate::trf!("Couldn't do that: {}.", why),
        })
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
            Some(kind) => self.each_edited(kind, tr("Select all"), |mesh, topology, _| {
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
            Some(kind) => self.each_edited(kind, tr("Invert"), |mesh, topology, selected| {
                ops::inverted(mesh, topology, selected, kind)
            }),
        }
    }

    /// Selects the polygons facing the wrong way round (Select menu →
    /// Select flipped): those disagreeing with their neighbours, or a
    /// closed shape's polygons facing inward. Says how many it found.
    pub(crate) fn select_flipped(&mut self) -> Option<String> {
        let mut found = 0;
        let note = self.each_edited(ElementKind::Polygons, tr("Select flipped"), |mesh, _, _| {
            mesh::cleanup::faces_facing_wrong(mesh)
                .into_iter()
                .map(Element::Polygon)
                .collect()
        });
        if note.is_some() {
            return note;
        }
        for item in self.drawing.items.iter().filter(|i| i.selected) {
            found += mesh::cleanup::faces_facing_wrong(&item.mesh).len();
        }
        Some(if found == 0 {
            tr("No polygon faces the wrong way.").into()
        } else {
            crate::trf!(
                "{} polygon{} facing the wrong way selected: Flip normals turns {}, \
                 or Mesh → Fix normals turns every one.",
                found,
                if found == 1 { "" } else { "s" },
                if found == 1 { tr("it") } else { tr("them") }
            )
        })
    }

    /// Selects the polygons of the edited objects using a material called
    /// `name` (Select menu → Select by material, decision 0074).
    pub(crate) fn select_by_material(&mut self, name: &str) -> Option<String> {
        // Each edited object's slots holding a material of that name.
        // (A slot past the last uses the first material.)
        let slots: BTreeMap<ObjectId, (Vec<u32>, u32)> = self
            .drawing
            .items
            .iter()
            .filter(|i| i.selected)
            .map(|i| {
                let list = self.document.scene().material_list(i.id);
                let slots = (0..list.len() as u32)
                    .filter(|s| list[*s as usize].name == name)
                    .collect();
                (i.id, (slots, list.len() as u32))
            })
            .collect();
        let none = ElementSelection::new();
        let mut changes = Vec::new();
        let mut found = 0;
        for item in self.drawing.items.iter().filter(|i| i.selected) {
            let (wanted, count) = slots.get(&item.id).cloned().unwrap_or_default();
            let mut next = self.document.elements(item.id).unwrap_or(&none).clone();
            let chosen: Vec<Element> = item
                .shown
                .faces()
                .map(|(f, _)| f)
                .filter(|f| {
                    let slot = mesh::material::slot_of(&item.shown, *f);
                    wanted.contains(if slot < count { &slot } else { &0 })
                })
                .map(Element::Polygon)
                .collect();
            found += chosen.len();
            next.combine(ElementKind::Polygons, chosen, Combine::Replace);
            changes.push((item.id, next));
        }
        if changes.is_empty() {
            return Some(nothing_to_edit(ElementKind::Polygons));
        }
        self.record(
            ElementKind::Polygons,
            changes,
            tr("Select by material"),
            None,
        );
        Some(match found {
            0 => crate::trf!("No polygon uses {}.", name),
            1 => crate::trf!("1 polygon using {} selected.", name),
            n => crate::trf!("{} polygons using {} selected.", n, name),
        })
    }

    /// Selects the hard edges (Select menu → Select hard edges).
    pub(crate) fn select_hard_edges(&mut self) -> Option<String> {
        self.each_edited(
            ElementKind::Edges,
            tr("Select hard edges"),
            |mesh, topology, _| {
                mesh.hard_edges()
                    .filter(|e| topology.has_edge(*e))
                    .map(Element::Edge)
                    .collect()
            },
        )
    }

    /// Selects everything joined to the selection (Select → Select
    /// connected, Alt + A), in the mode's kind.
    pub(crate) fn select_connected(&mut self) -> Option<String> {
        let Some(kind) = kind_for(self.mode) else {
            return Some(
                tr(
                    "Select connected works on points, edges or polygons: press 1, 2 or 3 \
                         (or use the UV mode) and select a little of the part first.",
                )
                .into(),
            );
        };
        let none = ElementSelection::new();
        let anything = self
            .drawing
            .items
            .iter()
            .filter(|i| i.selected)
            .any(|i| self.document.elements(i.id).unwrap_or(&none).count(kind) > 0);
        if !anything {
            return Some(crate::trf!(
                "Select connected: select a {} of the part first.",
                kind.singular()
            ));
        }
        self.each_edited(kind, tr("Select connected"), |mesh, topology, selected| {
            ops::connected(mesh, topology, selected, kind)
        })
    }

    /// Selects the whole UV pieces (islands) the selection is in, in the
    /// mode's kind (decision 0124): Alt + A over the UV Editor or in the
    /// UV mode, and UV → Pieces → Select connected.
    pub(crate) fn select_connected_in_uvs(&mut self) -> Option<String> {
        let Some(kind) = kind_for(self.mode) else {
            return Some(
                tr(
                    "Select connected works on points, edges or polygons: press 1, 2 or 3 \
                 (or use the UV mode) and select a little of a piece first.",
                )
                .into(),
            );
        };
        let none = ElementSelection::new();
        let anything = self
            .drawing
            .items
            .iter()
            .filter(|i| i.selected)
            .any(|i| self.document.elements(i.id).unwrap_or(&none).count(kind) > 0);
        if !anything {
            return Some(crate::trf!(
                "Select connected: select a {} of the piece first.",
                kind.singular()
            ));
        }
        self.each_edited(
            kind,
            tr("Select whole pieces"),
            |mesh, topology, selected| ops::connected_in_uvs(mesh, topology, selected, kind),
        )
    }

    /// Selects the seams (Select or UV menu → Select seams).
    pub(crate) fn select_seams(&mut self) -> Option<String> {
        self.each_edited(
            ElementKind::Edges,
            tr("Select seams"),
            |mesh, topology, _| {
                mesh::uvs::seams(mesh)
                    .into_iter()
                    .filter(|e| topology.has_edge(*e))
                    .map(Element::Edge)
                    .collect()
            },
        )
    }

    /// Adds the polygons facing within `angle` degrees of a selected
    /// polygon (Select menu → Select same facing).
    pub(crate) fn select_similar_facing(&mut self, angle: f32) -> Option<String> {
        self.each_edited(
            ElementKind::Polygons,
            tr("Select same facing"),
            |mesh, _, selected| {
                let mut out: std::collections::BTreeSet<scene::FaceId> =
                    selected.polygons().collect();
                let ways: Vec<glam::Vec3> = out.iter().map(|f| mesh.face_normal(*f)).collect();
                for way in ways {
                    out.extend(mesh::cleanup::faces_facing(mesh, way, angle));
                }
                out.into_iter().map(Element::Polygon).collect()
            },
        )
    }

    /// Unselects the edited objects' elements of the mode's kind (Esc).
    /// Returns whether it was in an element mode.
    pub(crate) fn clear_elements(&mut self) -> bool {
        let Some(kind) = kind_for(self.mode) else {
            return false;
        };
        self.each_edited(kind, tr("Deselect"), |_, _, _| Vec::new());
        true
    }

    /// Grows or shrinks the selection by `steps`.
    fn grow(&mut self, kind: ElementKind, steps: i32) -> Option<String> {
        if steps == 0 {
            return None;
        }
        let label = if steps > 0 { tr("Grow") } else { tr("Shrink") };
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
            // Only what is shown: hidden polygons aren't selected.
            let topology = item.shown.topology();
            let current = self.document.elements(item.id).unwrap_or(&none);
            let mut next = current.clone();
            let chosen = work(&item.shown, topology, current);
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
            Combine::Remove => tr("Deselect"),
            _ => tr("Select"),
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
            // A little more forgiving than hovering: a double-click
            // shouldn't need a perfectly steady hand.
            ElementKind::Edges | ElementKind::Points => {
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
        let topology = item.mesh.topology();
        let elements = if ring {
            ops::ring_through(&item.mesh, topology, edge, face, kind)
        } else {
            ops::loop_through(topology, edge, kind)
        };
        let mut next = self.document.elements(item.id).cloned().unwrap_or_default();
        next.combine(kind, elements, how);
        let action = if ring {
            tr("Select ring")
        } else {
            tr("Select loop")
        };
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
        // In the UV mode, whole islands (decision 0096).
        let changes = if self.mode == crate::palette::Mode::Uv && kind == ElementKind::Polygons {
            changes
                .into_iter()
                .map(|(id, after)| {
                    let before = self.document.elements(id).cloned().unwrap_or_default();
                    let whole = self
                        .document
                        .scene()
                        .evaluated(id)
                        .map_or_else(|| after.clone(), |e| to_islands(&e.mesh, &before, &after));
                    (id, whole)
                })
                .collect()
        } else {
            changes
        };
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
        let label = crate::trf!("{} ({} selected)", action, count(after, kind));
        self.document.select_elements(changes, label, merge_key);
    }

    /// Selects objects for a click: `found` is the object clicked, if any.
    fn combine_objects(&mut self, found: Option<ObjectId>, how: Combine) {
        match (found, how) {
            (None, Combine::Replace) => self.document.clear_selection(),
            (None, _) => {}
            (Some(id), how) => self.objects_from_region(vec![id], how, None),
        }
    }

    /// Selects objects for a box, lasso or brush (a brush stroke merges
    /// into one undo step with `merge_key`).
    fn objects_from_region(&mut self, found: Vec<ObjectId>, how: Combine, merge_key: Option<u64>) {
        let mut selection = self.document.selection();
        match how {
            Combine::Replace => selection = found,
            Combine::Add => {
                for id in found {
                    if !selection.contains(&id) {
                        selection.push(id);
                    }
                }
            }
            Combine::Toggle => {
                for id in found {
                    if let Some(at) = selection.iter().position(|s| *s == id) {
                        selection.remove(at);
                    } else {
                        selection.push(id);
                    }
                }
            }
            Combine::Remove => selection.retain(|id| !found.contains(id)),
        }
        self.document.select_objects(selection, merge_key);
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
    crate::trf!(
        "Select an object first: its {} can then be selected.",
        kind.plural()
    )
}

/// `after`, a new polygon selection of `mesh`, made of whole UV islands
/// (decision 0096): an island that gained a polygon is selected whole, one
/// that lost one is let go whole. Points and edges are left alone.
pub(crate) fn to_islands(
    mesh: &scene::Mesh,
    before: &ElementSelection,
    after: &ElementSelection,
) -> ElementSelection {
    let islands = mesh::uvs::islands(mesh);
    if islands.is_empty() {
        return after.clone();
    }
    let mut out = after.clone();
    for island in &islands {
        let had = island
            .iter()
            .filter(|f| before.contains(scene::Element::Polygon(**f)))
            .count();
        let has = island
            .iter()
            .filter(|f| after.contains(scene::Element::Polygon(**f)))
            .count();
        let whole = if has > had {
            true
        } else if has < had {
            false
        } else {
            continue;
        };
        for f in island {
            if whole {
                out.insert(scene::Element::Polygon(*f));
            } else {
                out.remove(scene::Element::Polygon(*f));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;

/// How near (screen points) the mouse must be to a curve's line to pick it.
const CURVE_REACH: f32 = 6.0;

/// Each shown curve's line on screen, in pieces (decision 0138).
fn curve_lines_on_screen<'a>(
    drawing: &'a SceneDrawing,
    camera: &'a Camera,
    size: Vec2,
) -> impl Iterator<Item = (ObjectId, Vec<Option<Vec2>>)> + 'a {
    drawing.items.iter().filter_map(move |item| {
        let curve = item.curve.as_ref()?;
        if !item.transform.is_finite() {
            return None;
        }
        let screen = curve
            .line
            .iter()
            .map(|p| camera.project(item.transform.transform_point3(*p), size))
            .collect();
        Some((item.id, screen))
    })
}

/// The curve whose line passes nearest `pointer`, within [`CURVE_REACH`]:
/// a curve that is only a line has no surface to click.
fn curve_line_under(drawing: &SceneDrawing, pointer: &ViewPointer) -> Option<ObjectId> {
    let mut best: Option<(f32, ObjectId)> = None;
    for (id, line) in curve_lines_on_screen(drawing, &pointer.camera, pointer.size) {
        for pair in line.windows(2) {
            let (Some(a), Some(b)) = (pair[0], pair[1]) else {
                continue;
            };
            let along = b - a;
            let t = if along.length_squared() > 1e-9 {
                ((pointer.at - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = pointer.at.distance(a + along * t);
            if d <= CURVE_REACH && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, id));
            }
        }
    }
    best.map(|(_, id)| id)
}

/// The curves whose line passes through `region`.
fn curves_in(
    drawing: &SceneDrawing,
    camera: &Camera,
    size: Vec2,
    region: &Region,
) -> Vec<ObjectId> {
    curve_lines_on_screen(drawing, camera, size)
        .filter(|(_, line)| {
            line.windows(2).any(|pair| match (pair[0], pair[1]) {
                (Some(a), Some(b)) => region.touches_segment(a, b),
                _ => false,
            })
        })
        .map(|(id, _)| id)
        .collect()
}
