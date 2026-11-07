//! The node canvas: where everything on it is this frame, what the
//! pointer is over, and drawing it all in order (decision 0258).

use super::drags::{Drag, Press};
use super::look::{self, FRAME_TITLE, PLUG_RADIUS};
use super::note::Note;
use super::search::Search;
use super::view::View;
use super::{CanvasKey, MaterialEditor};
use crate::text::tr;
use crate::theme;
use egui::{Pos2, Rect, Sense, Ui};
use glam::Vec2;
use matgraph::{BoxId, BoxInfo, FrameId, Graph, GraphId, PlugRef, Registry, ValueKind};
use scene::Document;
use std::collections::BTreeSet;

/// The canvas's background.
pub(crate) const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(0x1f, 0x1f, 0x1f);
/// How close (on screen) the pointer must be to a plug to take it.
const PLUG_REACH: f32 = 10.0;
/// How close (on screen) the pointer must be to a frame's edge to resize
/// it.
const EDGE_REACH: f32 = 6.0;

/// The canvas's own state.
#[derive(Debug, Default)]
pub(crate) struct Canvas {
    /// Its pan and zoom.
    pub(crate) view: View,
    /// Where it was drawn this frame and last frame.
    pub(crate) rect: Option<Rect>,
    pub(crate) last_rect: Option<Rect>,
    /// The selected boxes.
    pub(crate) selected: BTreeSet<BoxId>,
    /// The selected comment frame (its settings in Attributes).
    pub(crate) frame: Option<FrameId>,
    /// A press not yet a click or a drag.
    pub(crate) press: Option<Press>,
    /// What is being dragged.
    pub(crate) drag: Option<Drag>,
    /// The box search, while open.
    pub(crate) search: Option<Search>,
    /// "Why it won't work", while shown.
    pub(crate) note: Option<Note>,
    /// A frame's title being typed.
    pub(crate) renaming: Option<(FrameId, String, bool)>,
    /// The box whose picture the preview shows instead of the material.
    pub(crate) looking_at: Option<BoxId>,
    /// The graph framed when first shown.
    pub(crate) framed: Option<GraphId>,
    /// Frame it to be read along from its left (a starter graph just
    /// opened), rather than whole.
    pub(crate) read_along: bool,
    /// Where the overlays (note, search, title box) were drawn, so a
    /// press on them isn't a press on the canvas.
    pub(crate) overlays: Vec<Rect>,
}

impl Canvas {
    /// Whether Esc has something to close: the search, a note, a drag or
    /// a title being typed.
    pub(crate) fn closable(&self) -> bool {
        self.search.is_some() || self.note.is_some() || self.drag.is_some()
    }
}

/// A plug as drawn this frame.
#[derive(Debug, Clone)]
pub(crate) struct PlugPlace {
    pub(crate) plug: PlugRef,
    pub(crate) kind: ValueKind,
    pub(crate) output: bool,
    pub(crate) at: Pos2,
    pub(crate) name: String,
    pub(crate) needed: bool,
}

/// A box as drawn this frame.
#[derive(Debug, Clone)]
pub(crate) struct BoxPlace {
    pub(crate) id: BoxId,
    pub(crate) rect: Rect,
    pub(crate) picture: Rect,
    pub(crate) plugs: Vec<PlugPlace>,
    pub(crate) name: String,
    /// The kind of its first output (for its header's line), if any.
    pub(crate) makes: Option<ValueKind>,
}

/// A frame as drawn this frame.
#[derive(Debug, Clone)]
pub(crate) struct FramePlace {
    pub(crate) id: FrameId,
    pub(crate) rect: Rect,
    pub(crate) title: Rect,
}

/// Where everything is on screen this frame.
#[derive(Debug, Clone, Default)]
pub(crate) struct Places {
    pub(crate) boxes: Vec<BoxPlace>,
    /// Biggest first, so smaller (inner) frames are drawn over them.
    pub(crate) frames: Vec<FramePlace>,
}

/// A side of a frame being resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Edges {
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) bottom: bool,
}

/// What the pointer is over.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Hit {
    Plug(PlugRef, bool, ValueKind),
    Picture(BoxId),
    Badge(BoxId),
    Box(BoxId),
    FrameTitle(FrameId),
    FrameEdge(FrameId, Edges),
    Empty,
}

/// The screen height of a frame's title band: full size zoomed in, and
/// never smaller than readable zoomed out.
pub(crate) fn title_height(zoom: f32) -> f32 {
    (FRAME_TITLE * zoom).max(22.0)
}

/// The box kind's information, if the app knows it.
fn info_of<'r>(registry: &'r Registry, kind: &str) -> Option<&'r BoxInfo> {
    registry.get(kind).map(|t| t.info())
}

impl Places {
    /// Where everything in `graph` is, seen through `view` in `rect`.
    pub(crate) fn of(graph: &Graph, registry: &Registry, view: &View, rect: Rect) -> Self {
        let mut boxes = Vec::new();
        for (id, b) in graph.boxes() {
            let info = info_of(registry, &b.kind);
            let size = look::box_size(info);
            let screen = view.rect_on_screen(rect, b.at, b.at + size);
            let picture_min = b.at + look::picture_offset();
            let picture =
                view.rect_on_screen(rect, picture_min, picture_min + Vec2::splat(look::PICTURE));
            let mut plugs = Vec::new();
            if let Some(info) = info {
                for (i, p) in info.inputs.iter().enumerate() {
                    plugs.push(PlugPlace {
                        plug: PlugRef::new(id, p.key.to_string()),
                        kind: p.kind,
                        output: false,
                        at: view.on_screen(rect, b.at + look::input_offset(i)),
                        name: p.name.show(),
                        needed: p.needed,
                    });
                }
                for (i, p) in info.outputs.iter().enumerate() {
                    plugs.push(PlugPlace {
                        plug: PlugRef::new(id, p.key.to_string()),
                        kind: p.kind,
                        output: true,
                        at: view.on_screen(rect, b.at + look::output_offset(i)),
                        name: p.name.show(),
                        needed: false,
                    });
                }
            }
            boxes.push(BoxPlace {
                id,
                rect: screen,
                picture,
                plugs,
                name: info.map_or_else(|| b.kind.clone(), |i| i.name.show()),
                makes: info.and_then(|i| i.outputs.first().map(|o| o.kind)),
            });
        }
        let mut frames: Vec<FramePlace> = graph
            .frames()
            .map(|(id, f)| {
                let screen = view.rect_on_screen(rect, f.min, f.max);
                let title = Rect::from_min_size(
                    screen.min,
                    egui::vec2(screen.width(), title_height(view.zoom)),
                );
                FramePlace {
                    id,
                    rect: screen,
                    title,
                }
            })
            .collect();
        frames.sort_by(|a, b| {
            let area = |f: &FramePlace| f.rect.area();
            area(b).total_cmp(&area(a)).then(a.id.cmp(&b.id))
        });
        Self { boxes, frames }
    }

    /// The box `id`.
    pub(crate) fn find(&self, id: BoxId) -> Option<&BoxPlace> {
        self.boxes.iter().find(|b| b.id == id)
    }

    /// Plug `plug`'s place.
    pub(crate) fn plug(&self, plug: &PlugRef, output: bool) -> Option<&PlugPlace> {
        self.find(plug.node)?
            .plugs
            .iter()
            .find(|p| p.output == output && p.plug == *plug)
    }

    /// What is under the screen point `at`; `problems` are the boxes
    /// showing a badge.
    pub(crate) fn hit(&self, at: Pos2, problems: &BTreeSet<BoxId>) -> Hit {
        // Plugs first: they sit on boxes' edges.
        let nearest = self
            .boxes
            .iter()
            .flat_map(|b| b.plugs.iter())
            .map(|p| (p.at.distance(at), p))
            .filter(|(d, _)| *d <= PLUG_REACH.max(PLUG_RADIUS))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, p)) = nearest {
            return Hit::Plug(p.plug.clone(), p.output, p.kind);
        }
        // The last box drawn is on top.
        for b in self.boxes.iter().rev() {
            if !b.rect.contains(at) {
                continue;
            }
            if problems.contains(&b.id) && badge_rect(b.rect).contains(at) {
                return Hit::Badge(b.id);
            }
            if b.picture.contains(at) {
                return Hit::Picture(b.id);
            }
            return Hit::Box(b.id);
        }
        // The smallest (innermost) frame first.
        for f in self.frames.iter().rev() {
            if f.title.contains(at) {
                return Hit::FrameTitle(f.id);
            }
            let r = f.rect;
            let inside_y = at.y >= r.top() && at.y <= r.bottom() + EDGE_REACH;
            let inside_x = at.x >= r.left() - EDGE_REACH && at.x <= r.right() + EDGE_REACH;
            let edges = Edges {
                left: inside_y && (at.x - r.left()).abs() <= EDGE_REACH,
                right: inside_y && (at.x - r.right()).abs() <= EDGE_REACH,
                bottom: inside_x && (at.y - r.bottom()).abs() <= EDGE_REACH,
            };
            if edges != Edges::default() {
                return Hit::FrameEdge(f.id, edges);
            }
        }
        Hit::Empty
    }
}

/// Where a box's "can't run" badge sits: its header's right end.
pub(crate) fn badge_rect(box_rect: Rect) -> Rect {
    let side = box_rect.width() * look::HEADER / look::BOX_WIDTH;
    Rect::from_min_size(
        egui::pos2(box_rect.right() - side, box_rect.top()),
        egui::vec2(side, side),
    )
}

impl MaterialEditor {
    /// The Material graph panel: the graphs bar, then the canvas.
    pub(crate) fn canvas_tab(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let full = ui.max_rect();
        let bar = Rect::from_min_size(full.min, egui::vec2(full.width(), 34.0));
        let mut bar_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar.shrink2(egui::vec2(8.0, 3.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        ui.painter().rect_filled(bar, 0.0, theme::HEADER);
        self.graphs_bar(&mut bar_ui, document, note);
        let rect = Rect::from_min_max(egui::pos2(full.left(), bar.bottom()), full.max);
        ui.advance_cursor_after_rect(full);
        let Some(graph) = self.current(document).cloned() else {
            self.empty_canvas(ui, rect, document, note);
            self.starters_window(ui.ctx(), document, note);
            return;
        };
        self.canvas.rect = Some(rect);
        let response = ui.interact(
            rect,
            ui.id().with("material canvas"),
            Sense::click_and_drag(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, tr(super::CANVAS_LABEL))
        });
        if self.canvas.framed != self.graph {
            self.canvas.framed = self.graph;
            if std::mem::take(&mut self.canvas.read_along) {
                if let Some((lo, hi)) = self.extent(&graph, false) {
                    self.canvas.view.frame_readable(rect, lo, hi);
                }
            } else {
                self.frame_view(&graph, rect, false);
            }
        }
        self.navigate(ui, rect, &response);
        let keys = std::mem::take(&mut self.keys);
        for key in keys {
            self.use_key(ui.ctx(), key, document, note);
        }
        // Each step below may change the graph: read it again after.
        let graph = self.current(document).cloned().unwrap_or(graph);
        let registry = self.registry_of(document);
        let places = Places::of(&graph, &registry, &self.canvas.view, rect);
        self.pointer(ui, rect, &response, &graph, &places, document, note);
        let graph = self.current(document).cloned().unwrap_or(graph);
        let registry = self.registry_of(document);
        let places = Places::of(&graph, &registry, &self.canvas.view, rect);
        let painter = ui.painter_at(rect);
        self.draw(ui.ctx(), &painter, rect, &graph, &places);
        self.canvas.overlays.clear();
        self.overlays(ui.ctx(), rect, &graph, &places, document, note);
        self.starters_window(ui.ctx(), document, note);
    }

    /// The canvas with no graph yet: what it is for, and a button to
    /// start one.
    fn empty_canvas(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        ui.painter().rect_filled(rect, 0.0, BACKGROUND);
        let middle = Rect::from_center_size(rect.center(), egui::vec2(360.0, 200.0));
        let mut inner = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(middle)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        inner.label(
            egui::RichText::new(tr("No material graph yet"))
                .color(theme::TEXT_STRONG)
                .size(15.0),
        );
        inner.add_space(6.0);
        crate::dialog::help(
            &mut inner,
            tr(
                "A material graph makes a seamless material from boxes joined by wires: \
                patterns first, then shaping, detail, colour and the finish.",
            ),
        );
        inner.add_space(10.0);
        if crate::dialog::main_button(&mut inner, tr(super::starters::START_FROM_LABEL)).clicked() {
            self.starters_open = true;
        }
        inner.add_space(4.0);
        crate::dialog::help(
            &mut inner,
            tr("A copy of a ready-made material, with notes explaining each step."),
        );
        inner.add_space(8.0);
        if inner.button(tr(super::NEW_GRAPH_LABEL)).clicked() {
            self.new_graph(document, note);
        }
    }

    /// Middle drag pans; the wheel zooms round the pointer.
    fn navigate(&mut self, ui: &Ui, rect: Rect, response: &egui::Response) {
        if response.dragged_by(egui::PointerButton::Middle) {
            self.canvas.view.pan_by(response.drag_delta());
        }
        if response.hovered() && !egui::Popup::is_any_open(ui.ctx()) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
            if scroll != 0.0
                && let Some(at) = response.hover_pos()
            {
                self.canvas.view.wheel(rect, at, scroll);
            }
        }
    }

    /// Frames the selected boxes, or every box and frame when none is
    /// selected (Z).
    pub(crate) fn frame_view(&mut self, graph: &Graph, rect: Rect, selected_only: bool) {
        if let Some((lo, hi)) = self.extent(graph, selected_only) {
            self.canvas.view.frame(rect, lo, hi);
        }
    }

    /// The corners round the selected boxes, or every box and frame when
    /// none is selected (or not `selected_only`); `None` for nothing.
    fn extent(&self, graph: &Graph, selected_only: bool) -> Option<(Vec2, Vec2)> {
        let registry = self
            .registry
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(Registry::standard().clone()));
        let registry = &*registry;
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        let chosen = selected_only && !self.canvas.selected.is_empty();
        for (id, _) in graph.boxes() {
            if chosen && !self.canvas.selected.contains(&id) {
                continue;
            }
            if let Some((a, b)) = look::box_rect(graph, registry, id) {
                lo = lo.min(a);
                hi = hi.max(b);
            }
        }
        if !chosen {
            for (_, f) in graph.frames() {
                lo = lo.min(f.min);
                hi = hi.max(f.max);
            }
        }
        (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
    }

    /// Carries out a key pressed over the canvas.
    fn use_key(
        &mut self,
        ctx: &egui::Context,
        key: CanvasKey,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let rect = self.canvas.rect.unwrap_or(Rect::NOTHING);
        let pointer = ctx.pointer_hover_pos().filter(|p| rect.contains(*p));
        let at = pointer.map_or_else(
            || self.canvas.view.on_canvas(rect, rect.center()),
            |p| self.canvas.view.on_canvas(rect, p),
        );
        match key {
            CanvasKey::Search => self.open_search(at, None, document),
            CanvasKey::Comment => self.comment_frame(document, at, note),
            CanvasKey::Frame => {
                if let Some(graph) = self.current(document).cloned() {
                    self.frame_view(&graph, rect, true);
                }
            }
            CanvasKey::Remove => self.remove_selected(document, false, note),
            CanvasKey::Dissolve => self.remove_selected(document, true, note),
            CanvasKey::Copy => self.copy(document, false, note),
            CanvasKey::LinkedCopy => self.copy(document, true, note),
            CanvasKey::Paste => self.paste(document, at, false, note),
            CanvasKey::LinkedPaste => self.paste(document, at, true, note),
            CanvasKey::OwnBox => self.start_making(document, note),
            CanvasKey::OpenBox => self.open_up_selected(document, note),
            CanvasKey::Escape => {
                if self.making.is_some() {
                    self.making = None;
                } else {
                    self.canvas.search = None;
                    self.canvas.note = None;
                    self.cancel_drag(document);
                }
            }
        }
    }
}

#[cfg(test)]
impl MaterialEditor {
    /// Where plug `plug` (an output when `output`) was drawn last frame
    /// (workflow tests).
    pub(crate) fn plug_on_screen(
        &self,
        document: &Document,
        plug: &PlugRef,
        output: bool,
    ) -> Option<Pos2> {
        let places = self.places_now(document)?;
        places.plug(plug, output).map(|p| p.at)
    }

    /// Where box `id` and its picture were drawn last frame (workflow
    /// tests).
    pub(crate) fn box_on_screen(&self, document: &Document, id: BoxId) -> Option<(Rect, Rect)> {
        let places = self.places_now(document)?;
        places.find(id).map(|b| (b.rect, b.picture))
    }

    /// Where frame `id` and its title were drawn last frame (workflow
    /// tests).
    pub(crate) fn frame_on_screen(&self, document: &Document, id: FrameId) -> Option<(Rect, Rect)> {
        let places = self.places_now(document)?;
        places
            .frames
            .iter()
            .find(|f| f.id == id)
            .map(|f| (f.rect, f.title))
    }

    fn places_now(&self, document: &Document) -> Option<Places> {
        let rect = self.canvas.rect.or(self.canvas.last_rect)?;
        let graph = self.current(document)?;
        Some(Places::of(graph, self.registry(), &self.canvas.view, rect))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::GraphEdit;

    #[test]
    fn plugs_boxes_and_frames_are_found_under_the_pointer() {
        let registry = Registry::standard();
        let mut g = Graph::new("Stone");
        for edit in [
            GraphEdit::AddBox {
                kind: "levels".into(),
                at: Vec2::new(100.0, 100.0),
            },
            GraphEdit::AddFrame(matgraph::Frame::new(
                "Shapes",
                Vec2::new(0.0, 0.0),
                Vec2::new(600.0, 600.0),
            )),
        ] {
            g.apply(registry, &edit).unwrap();
        }
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 800.0));
        let view = View {
            pan: Vec2::ZERO,
            zoom: 1.0,
        };
        let places = Places::of(&g, registry, &view, rect);
        let none = BTreeSet::new();
        let b = &places.boxes[0];
        let input = b.plugs.iter().find(|p| !p.output).unwrap();
        assert!(matches!(
            places.hit(input.at, &none),
            Hit::Plug(_, false, ValueKind::Height)
        ));
        assert_eq!(
            places.hit(b.picture.center(), &none),
            Hit::Picture(BoxId(1))
        );
        let header = b.rect.left_top() + egui::vec2(30.0, 5.0);
        assert_eq!(places.hit(header, &none), Hit::Box(BoxId(1)));
        let mut problems = BTreeSet::new();
        problems.insert(BoxId(1));
        let badge = badge_rect(b.rect).center();
        assert_eq!(places.hit(badge, &problems), Hit::Badge(BoxId(1)));
        let frame = &places.frames[0];
        assert_eq!(
            places.hit(frame.title.center(), &none),
            Hit::FrameTitle(frame.id)
        );
        let right = egui::pos2(frame.rect.right(), frame.rect.center().y);
        assert!(matches!(places.hit(right, &none), Hit::FrameEdge(_, e) if e.right));
        assert_eq!(places.hit(egui::pos2(900.0, 700.0), &none), Hit::Empty);
    }

    #[test]
    fn frame_titles_stay_readable_zoomed_out() {
        assert_eq!(title_height(1.0), FRAME_TITLE);
        assert!(title_height(0.15) >= 22.0);
    }
}
