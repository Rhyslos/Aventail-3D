//! Selecting in the editor: what a click finds, box selection, and
//! changing the selection as one undo step (in the UV mode a polygon
//! brings its whole island).

use super::shape::{UvShape, inside, polygon_area, segment_distance};
use super::{EditorDrag, UvContext, UvEditor};
use crate::palette::Mode;
use crate::text::tr;
use egui::{Pos2, Rect, Response, Ui};
use glam::Vec2;
use scene::{Combine, Document, Element, ElementKind, ElementSelection, ObjectId};
use std::collections::BTreeSet;

/// How close, in screen points, a click must be to a point or an edge.
pub(super) const PICK_DISTANCE: f32 = 9.0;

/// What a click in the editor found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Hit {
    pub(super) id: ObjectId,
    pub(super) element: Element,
}

impl UvEditor {
    /// What is under `at`, by mode: a point or an edge close by, or the
    /// polygon it is inside (the smallest, where they overlap).
    pub(super) fn pick(
        &self,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        at: Pos2,
        mode: Mode,
    ) -> Option<Hit> {
        let kind = element_kind(mode)?;
        let uv = self.to_uv(rect, at);
        match kind {
            ElementKind::Points => {
                let mut best: Option<(f32, Hit)> = None;
                for (id, shape) in shapes {
                    for (_, points) in &shape.polygons {
                        for (v, p) in points {
                            let d = self.to_screen(rect, *p).distance(at);
                            if d <= PICK_DISTANCE && best.is_none_or(|(b, _)| d < b) {
                                best = Some((
                                    d,
                                    Hit {
                                        id: *id,
                                        element: Element::Point(*v),
                                    },
                                ));
                            }
                        }
                    }
                }
                best.map(|(_, h)| h)
            }
            ElementKind::Edges => {
                let mut best: Option<(f32, Hit)> = None;
                for (id, shape) in shapes {
                    for (uvs, key, _) in &shape.edges {
                        let d = segment_distance(
                            at,
                            self.to_screen(rect, uvs[0]),
                            self.to_screen(rect, uvs[1]),
                        );
                        if d <= PICK_DISTANCE && best.is_none_or(|(b, _)| d < b) {
                            best = Some((
                                d,
                                Hit {
                                    id: *id,
                                    element: Element::Edge(*key),
                                },
                            ));
                        }
                    }
                }
                best.map(|(_, h)| h)
            }
            ElementKind::Polygons => {
                let mut best: Option<(f32, Hit)> = None;
                for (id, shape) in shapes {
                    for (face, points) in &shape.polygons {
                        let corners: Vec<Vec2> = points.iter().map(|(_, p)| *p).collect();
                        if inside(&corners, uv) {
                            let area = polygon_area(&corners).abs();
                            if best.is_none_or(|(b, _)| area < b) {
                                best = Some((
                                    area,
                                    Hit {
                                        id: *id,
                                        element: Element::Polygon(*face),
                                    },
                                ));
                            }
                        }
                    }
                }
                best.map(|(_, h)| h)
            }
        }
    }

    /// Clicks and box drags select, as in the viewport.
    pub(super) fn select(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        doc: &mut Document,
        cx: &UvContext<'_>,
    ) {
        let Some(kind) = element_kind(cx.mode) else {
            return;
        };
        let keys = ui.input(|i| i.modifiers);
        if response.drag_started_by(egui::PointerButton::Primary)
            && self.drag.is_none()
            && let Some(from) = ui.input(|i| i.pointer.press_origin())
        {
            self.drag = Some(EditorDrag::Box {
                from,
                how: cx.select_keys.combine(keys, false),
            });
        }
        if let Some(EditorDrag::Box { from, how }) = self.drag
            && (response.drag_stopped() || !ui.input(|i| i.pointer.primary_down()))
        {
            self.drag = None;
            if let Some(to) = ui.input(|i| i.pointer.latest_pos()) {
                let area = Rect::from_two_pos(from, to);
                let mut hits = Vec::new();
                for (id, shape) in shapes {
                    for element in self.in_box(rect, shape, area, kind) {
                        hits.push(Hit { id: *id, element });
                    }
                }
                apply(doc, shapes, &hits, kind, how, cx.mode);
            }
            return;
        }
        if response.clicked_by(egui::PointerButton::Primary)
            && !self.pressed_on_gizmo
            && let Some(at) = response.interact_pointer_pos()
        {
            let how = cx.select_keys.combine(keys, true);
            let hits: Vec<Hit> = self.pick(rect, shapes, at, cx.mode).into_iter().collect();
            apply(doc, shapes, &hits, kind, how, cx.mode);
        }
    }

    /// The elements of `kind` inside `area` (screen): points, edges with
    /// both ends inside, polygons with their middle inside.
    fn in_box(&self, rect: Rect, shape: &UvShape, area: Rect, kind: ElementKind) -> Vec<Element> {
        let screen = |uv: Vec2| self.to_screen(rect, uv);
        let mut out = BTreeSet::new();
        match kind {
            ElementKind::Points => {
                for (_, points) in &shape.polygons {
                    for (v, uv) in points {
                        if area.contains(screen(*uv)) {
                            out.insert(Element::Point(*v));
                        }
                    }
                }
            }
            ElementKind::Edges => {
                for (uvs, key, _) in &shape.edges {
                    if area.contains(screen(uvs[0])) && area.contains(screen(uvs[1])) {
                        out.insert(Element::Edge(*key));
                    }
                }
            }
            ElementKind::Polygons => {
                for (face, points) in &shape.polygons {
                    let middle = points.iter().map(|(_, p)| *p).sum::<Vec2>() / points.len() as f32;
                    if area.contains(screen(middle)) {
                        out.insert(Element::Polygon(*face));
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

/// The kind of element the editor selects in `mode`: the UV mode works on
/// polygons, a whole island at a time.
pub(crate) fn element_kind(mode: Mode) -> Option<ElementKind> {
    match mode {
        Mode::Points => Some(ElementKind::Points),
        Mode::Edges => Some(ElementKind::Edges),
        Mode::Polygons | Mode::Uv => Some(ElementKind::Polygons),
        Mode::Model | Mode::Object => None,
    }
}

/// Changes the selection by `hits`, combined `how`, as one undo step; in
/// the UV mode each polygon brings its whole island.
pub(super) fn apply(
    doc: &mut Document,
    shapes: &[(ObjectId, UvShape)],
    hits: &[Hit],
    kind: ElementKind,
    how: Combine,
    mode: Mode,
) {
    let mut changes = Vec::new();
    for (id, shape) in shapes {
        let mine: Vec<Element> = hits
            .iter()
            .filter(|h| h.id == *id)
            .map(|h| h.element)
            .collect();
        if mine.is_empty() && how != Combine::Replace {
            continue;
        }
        let mut selection = doc.elements(*id).cloned().unwrap_or_default();
        if mode == Mode::Uv {
            whole_islands(&mut selection, shape, &mine, how);
        } else {
            selection.combine(kind, mine, how);
        }
        changes.push((*id, selection));
    }
    let count: usize = hits.len();
    let label = match (mode, count) {
        (Mode::Uv, _) => tr("Select pieces").to_string(),
        (_, 0) => tr("Deselect").to_string(),
        _ => crate::trf!("Select {} {}", count, kind_name(kind, count)),
    };
    doc.select_elements(changes, label, None);
}

/// Combines whole islands: the islands of `hits` are added, taken away or
/// flipped as one.
pub(super) fn whole_islands(
    selection: &mut ElementSelection,
    shape: &UvShape,
    hits: &[Element],
    how: Combine,
) {
    let islands: BTreeSet<usize> = hits
        .iter()
        .filter_map(|e| match e {
            Element::Polygon(f) => shape.island_of.get(f.get() as usize).copied(),
            _ => None,
        })
        .filter(|i| *i < shape.islands.len())
        .collect();
    if how == Combine::Replace {
        selection.clear_kind(ElementKind::Polygons);
    }
    for i in islands {
        let faces = &shape.islands[i];
        let all = faces
            .iter()
            .all(|f| selection.contains(Element::Polygon(*f)));
        let add = match how {
            Combine::Replace | Combine::Add => true,
            Combine::Remove => false,
            Combine::Toggle => !all,
        };
        for f in faces {
            if add {
                selection.insert(Element::Polygon(*f));
            } else {
                selection.remove(Element::Polygon(*f));
            }
        }
    }
}

fn kind_name(kind: ElementKind, n: usize) -> &'static str {
    match (kind, n == 1) {
        (ElementKind::Points, true) => "point",
        (ElementKind::Points, false) => "points",
        (ElementKind::Edges, true) => "edge",
        (ElementKind::Edges, false) => "edges",
        (ElementKind::Polygons, true) => "polygon",
        (ElementKind::Polygons, false) => "polygons",
    }
}
