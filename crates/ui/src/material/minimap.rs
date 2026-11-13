//! The minimap (milestone 17, decision 0279): a small map of the whole
//! graph in the canvas's bottom right corner, for graphs too big to see
//! at once. It shows the comment frames in their colours, the wires
//! (through their reroute dots, as the canvas draws them), the boxes (the
//! selected ones and search matches marked) and the part on screen as a
//! rectangle. A click or a drag on it moves the view there.
//!
//! It hides itself while the whole graph fits on screen, and the canvas
//! bar's View menu switches it off. Inside your own box it maps the
//! boxes inside.

use super::MaterialEditor;
use super::canvas::Places;
use super::look;
use super::view::View;
use crate::text::tr;
use crate::theme;
use egui::{Color32, Context, Painter, Pos2, Rect, Stroke, StrokeKind};
use glam::Vec2;
use matgraph::{BoxId, FrameId, Graph, Registry};
use std::collections::BTreeSet;

/// The View menu's switch.
pub(crate) const MINIMAP_LABEL: &str = crate::text!("Minimap");

/// The biggest the map is on screen.
const MOST: egui::Vec2 = egui::vec2(200.0, 136.0);
/// The share of the canvas it may take each way, on a small canvas.
const SHARE: f32 = 0.25;
/// Its room from the canvas's edges.
const MARGIN: f32 = 12.0;
/// The room inside its border.
const INSET: f32 = 6.0;
/// The smallest canvas it is shown on.
const LEAST_CANVAS: egui::Vec2 = egui::vec2(320.0, 220.0);
/// The room kept round the graph on the map, on the canvas.
const ROOM: f32 = 60.0;

/// Where the map is and what it shows: the canvas from `lo`, `scale`
/// screen points per canvas unit, drawn inside `area`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MapPlace {
    /// The map's panel on screen, border included.
    pub(crate) rect: Rect,
    /// Where the mapped part of the canvas is drawn.
    area: Rect,
    /// The canvas point at the area's top left.
    lo: Vec2,
    /// Screen points per canvas unit.
    scale: f32,
}

impl MapPlace {
    /// A map of the canvas from `lo` to `hi`, in the bottom right corner
    /// of the canvas `canvas`, its shape kept; `None` when the canvas is
    /// too small for one or there is nothing to map.
    pub(crate) fn of(canvas: Rect, lo: Vec2, hi: Vec2) -> Option<Self> {
        if canvas.width() < LEAST_CANVAS.x || canvas.height() < LEAST_CANVAS.y {
            return None;
        }
        let size = hi - lo;
        if !(size.x > 0.0 && size.y > 0.0 && size.is_finite()) {
            return None;
        }
        let most = egui::vec2(
            MOST.x.min(canvas.width() * SHARE),
            MOST.y.min(canvas.height() * SHARE),
        ) - egui::Vec2::splat(2.0 * INSET);
        let scale = (most.x / size.x).min(most.y / size.y);
        let drawn = egui::vec2(size.x * scale, size.y * scale);
        // A long thin graph still gets a map wide or tall enough to hit.
        let outer = (drawn + egui::Vec2::splat(2.0 * INSET)).max(egui::vec2(60.0, 40.0));
        let rect = Rect::from_min_size(canvas.max - outer - egui::Vec2::splat(MARGIN), outer);
        let area = Rect::from_center_size(rect.center(), drawn);
        Some(Self {
            rect,
            area,
            lo,
            scale,
        })
    }

    /// Canvas point `p` on the map.
    pub(crate) fn on_map(&self, p: Vec2) -> Pos2 {
        let s = (p - self.lo) * self.scale;
        self.area.min + egui::vec2(s.x, s.y)
    }

    /// The canvas point under the screen point `s` on the map (kept to
    /// the mapped part).
    pub(crate) fn on_canvas(&self, s: Pos2) -> Vec2 {
        let s = self.area.clamp(s) - self.area.min;
        self.lo + Vec2::new(s.x, s.y) / self.scale
    }
}

/// The part of the canvas on screen, as two corners on the canvas.
pub(crate) fn visible(view: &View, rect: Rect) -> (Vec2, Vec2) {
    (
        view.on_canvas(rect, rect.min),
        view.on_canvas(rect, rect.max),
    )
}

/// Everything in the graph, as two corners on the canvas: its boxes,
/// frames and reroute dots; `None` for an empty graph.
pub(crate) fn extent(graph: &Graph, registry: &Registry) -> Option<(Vec2, Vec2)> {
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for (id, _) in graph.boxes() {
        if let Some((a, b)) = look::box_rect(graph, registry, id) {
            lo = lo.min(a);
            hi = hi.max(b);
        }
    }
    for (_, f) in graph.frames() {
        lo = lo.min(f.min);
        hi = hi.max(f.max);
    }
    for (_, dots) in graph.all_bends() {
        for d in dots {
            lo = lo.min(*d);
            hi = hi.max(*d);
        }
    }
    (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
}

/// Whether the whole of `lo` to `hi` is on screen.
pub(crate) fn fits(view: &View, rect: Rect, lo: Vec2, hi: Vec2) -> bool {
    let (a, b) = visible(view, rect);
    lo.cmpge(a).all() && hi.cmple(b).all()
}

/// The map for this frame: the graph with room round it, and the part on
/// screen, so the rectangle is always on the map. `None` when the whole
/// graph fits on screen (or the canvas is too small).
pub(crate) fn map_of(
    view: &View,
    rect: Rect,
    graph: &Graph,
    registry: &Registry,
) -> Option<MapPlace> {
    let (lo, hi) = extent(graph, registry)?;
    if fits(view, rect, lo, hi) {
        return None;
    }
    let (a, b) = visible(view, rect);
    let room = Vec2::splat(ROOM);
    MapPlace::of(rect, (lo - room).min(a), (hi + room).max(b))
}

/// Moves `view` so the canvas point `p` is in the middle of `rect`,
/// keeping its zoom.
pub(crate) fn centre_on(view: &mut View, rect: Rect, p: Vec2) {
    let middle = Vec2::new(rect.width(), rect.height()) / 2.0;
    view.pan = middle - p * view.zoom;
}

/// What the map marks.
pub(crate) struct Marks<'a> {
    pub(crate) selected: &'a BTreeSet<BoxId>,
    pub(crate) found: &'a BTreeSet<BoxId>,
    pub(crate) found_frames: &'a BTreeSet<FrameId>,
    pub(crate) accent: Color32,
}

/// Draws the map: frames, wires, boxes, then the part on screen.
pub(crate) fn draw(
    painter: &Painter,
    map: &MapPlace,
    view: &View,
    rect: Rect,
    graph: &Graph,
    places: &Places,
    marks: &Marks<'_>,
) {
    painter.rect_filled(
        map.rect,
        6.0,
        Color32::from_rgba_unmultiplied(0x16, 0x16, 0x16, 235),
    );
    painter.rect_stroke(
        map.rect,
        6.0,
        Stroke::new(1.0, theme::TEXT_WEAK.gamma_multiply(0.5)),
        StrokeKind::Inside,
    );
    let painter = painter.with_clip_rect(map.rect.shrink(1.0).intersect(painter.clip_rect()));
    let to_map = |s: Pos2| map.on_map(view.on_canvas(rect, s));
    for place in &places.frames {
        let Some(frame) = graph.frame(place.id) else {
            continue;
        };
        let [r, g, b] = frame.colour;
        let r_map = Rect::from_min_max(map.on_map(frame.min), map.on_map(frame.max));
        painter.rect_filled(r_map, 1.5, Color32::from_rgba_unmultiplied(r, g, b, 70));
        let edge = if marks.found_frames.contains(&place.id) {
            Stroke::new(1.5, look::FOUND)
        } else {
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 200))
        };
        painter.rect_stroke(r_map, 1.5, edge, StrokeKind::Inside);
    }
    for wire in &places.wires {
        let points: Vec<Pos2> = wire.points.iter().map(|p| to_map(*p)).collect();
        let colour = look::kind_colour(wire.kind).gamma_multiply(0.75);
        painter.add(egui::Shape::line(points, Stroke::new(1.0, colour)));
    }
    for b in &places.boxes {
        let r_map = Rect::from_min_max(to_map(b.rect.min), to_map(b.rect.max));
        // Never thinner than a line, so every box shows.
        let r_map = Rect::from_center_size(r_map.center(), r_map.size().max(egui::vec2(3.0, 3.0)));
        let fill = if marks.selected.contains(&b.id) {
            marks.accent
        } else if marks.found.contains(&b.id) {
            look::FOUND
        } else {
            Color32::from_gray(0x8a)
        };
        painter.rect_filled(r_map, 1.0, fill);
    }
    let (a, b) = visible(view, rect);
    let shown = Rect::from_min_max(map.on_map(a), map.on_map(b));
    painter.rect_filled(shown, 2.0, Color32::from_white_alpha(14));
    painter.rect_stroke(
        shown,
        2.0,
        Stroke::new(1.5, theme::TEXT_STRONG),
        StrokeKind::Inside,
    );
}

/// The View menu's entry framing everything.
pub(crate) const FRAME_ALL_LABEL: &str = crate::text!("Frame the whole graph");
/// The View menu's entry framing the selected boxes.
pub(crate) const FRAME_SELECTED_LABEL: &str = crate::text!("Frame the selected boxes");

impl MaterialEditor {
    /// The canvas bar's View menu: framing, and the minimap's switch.
    pub(super) fn view_menu(&mut self, ui: &mut egui::Ui, document: &scene::Document) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let rect = self.canvas.rect.or(self.canvas.last_rect);
        crate::dialog::menu(ui, tr("View"), |ui| {
            if ui.button(tr(FRAME_ALL_LABEL)).clicked() {
                if let Some(rect) = rect {
                    self.frame_view(&graph, rect, false);
                }
                ui.close();
            }
            if ui
                .add_enabled(
                    !self.canvas.selected.is_empty(),
                    egui::Button::new(tr(FRAME_SELECTED_LABEL))
                        .shortcut_text(super::FRAME_KEY.name()),
                )
                .clicked()
            {
                if let Some(rect) = rect {
                    self.frame_view(&graph, rect, true);
                }
                ui.close();
            }
            ui.separator();
            let mut on = !self.minimap_off;
            if ui
                .checkbox(&mut on, tr(MINIMAP_LABEL))
                .on_hover_text(tr(
                    "A small map of the whole graph in the canvas's corner, shown when the \
                     graph doesn't fit on screen: click or drag on it to go there",
                ))
                .changed()
            {
                self.minimap_off = !on;
            }
        });
    }

    /// The minimap this frame: a press on it, or a drag begun on it,
    /// moves the view there; then it is drawn. Its panel is an overlay,
    /// so the canvas doesn't take the press too.
    pub(super) fn minimap(
        &mut self,
        ctx: &Context,
        painter: &Painter,
        rect: Rect,
        graph: &Graph,
        places: &Places,
    ) {
        let map = if self.minimap_off {
            None
        } else {
            // While dragging on it, it keeps its place, so it doesn't
            // slide under the pointer as the view moves.
            self.canvas
                .map_drag
                .or_else(|| map_of(&self.canvas.view, rect, graph, self.registry()))
        };
        let Some(map) = map else {
            self.canvas.map_drag = None;
            return;
        };
        let (pos, pressed, down) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
            )
        });
        let free = !egui::Popup::is_any_open(ctx) && self.canvas.drag.is_none();
        if pressed && free && pos.is_some_and(|p| map.rect.contains(p)) {
            self.canvas.map_drag = Some(map);
        }
        if let Some(held) = self.canvas.map_drag {
            match pos {
                Some(p) if down => {
                    centre_on(&mut self.canvas.view, rect, held.on_canvas(p));
                    ctx.request_repaint();
                }
                _ => self.canvas.map_drag = None,
            }
        }
        self.canvas.overlays.push(map.rect);
        if pos.is_some_and(|p| map.rect.contains(p)) {
            ctx.set_cursor_icon(if self.canvas.map_drag.is_some() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::PointingHand
            });
        }
        let (found, found_frames) = self.canvas.finding.marked();
        let marks = Marks {
            selected: &self.canvas.selected,
            found: &found,
            found_frames: &found_frames,
            accent: theme::accent(ctx).colors().light,
        };
        draw(
            painter,
            &map,
            &self.canvas.view,
            rect,
            graph,
            places,
            &marks,
        );
        self.canvas.map = Some(map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(1000.0, 700.0))
    }

    #[test]
    fn the_map_sits_in_the_bottom_right_corner_keeping_the_graphs_shape() {
        let map = MapPlace::of(canvas(), Vec2::ZERO, Vec2::new(4000.0, 1000.0)).unwrap();
        assert!(canvas().contains_rect(map.rect));
        assert_eq!(map.rect.max, canvas().max - egui::Vec2::splat(MARGIN));
        assert!(map.rect.width() <= MOST.x + 0.01);
        assert!(map.rect.height() <= MOST.y + 0.01);
        // Four times wider than tall, on the map too.
        let area = Rect::from_min_max(
            map.on_map(Vec2::ZERO),
            map.on_map(Vec2::new(4000.0, 1000.0)),
        );
        assert!((area.width() / area.height() - 4.0).abs() < 0.01);
        assert!(map.rect.contains_rect(area));
    }

    #[test]
    fn map_and_canvas_points_round_trip_and_stay_on_the_map() {
        let map = MapPlace::of(
            canvas(),
            Vec2::new(-500.0, -200.0),
            Vec2::new(1500.0, 800.0),
        )
        .unwrap();
        for p in [
            Vec2::new(-500.0, -200.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1234.0, 567.0),
        ] {
            let back = map.on_canvas(map.on_map(p));
            assert!((back - p).length() < 0.05, "{p} came back as {back}");
        }
        // A point past the map's edge is the nearest point on it.
        let past = map.on_canvas(map.rect.max + egui::vec2(50.0, 50.0));
        assert!((past - Vec2::new(1500.0, 800.0)).length() < 0.05);
    }

    #[test]
    fn no_map_on_a_tiny_canvas_or_for_nothing() {
        let tiny = Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 200.0));
        assert!(MapPlace::of(tiny, Vec2::ZERO, Vec2::splat(5000.0)).is_none());
        assert!(MapPlace::of(canvas(), Vec2::ZERO, Vec2::ZERO).is_none());
        assert!(MapPlace::of(canvas(), Vec2::ZERO, Vec2::new(f32::NAN, 3.0)).is_none());
        // A small canvas gets a smaller map.
        let small = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
        let map = MapPlace::of(small, Vec2::ZERO, Vec2::new(3000.0, 2000.0)).unwrap();
        assert!(map.rect.width() <= 400.0 * SHARE + 0.01);
    }

    #[test]
    fn the_part_on_screen_and_whether_everything_fits() {
        let view = View {
            pan: Vec2::new(0.0, 0.0),
            zoom: 0.5,
        };
        let (a, b) = visible(&view, canvas());
        assert_eq!(a, Vec2::ZERO);
        assert_eq!(b, Vec2::new(2000.0, 1400.0));
        assert!(fits(
            &view,
            canvas(),
            Vec2::splat(10.0),
            Vec2::new(1990.0, 1000.0)
        ));
        assert!(!fits(
            &view,
            canvas(),
            Vec2::splat(10.0),
            Vec2::new(2100.0, 1000.0)
        ));
        assert!(!fits(&view, canvas(), Vec2::splat(-1.0), Vec2::splat(10.0)));
    }

    #[test]
    fn centring_puts_the_point_in_the_middle_at_the_same_zoom() {
        let mut view = View {
            pan: Vec2::new(30.0, -80.0),
            zoom: 0.4,
        };
        centre_on(&mut view, canvas(), Vec2::new(1200.0, -300.0));
        assert_eq!(view.zoom, 0.4);
        let middle = view.on_screen(canvas(), Vec2::new(1200.0, -300.0));
        assert!((middle - canvas().center()).length() < 0.01);
    }

    #[test]
    fn a_click_on_the_map_centres_the_view_on_that_place() {
        let rect = canvas();
        let mut view = View::default();
        let map = MapPlace::of(rect, Vec2::ZERO, Vec2::new(5000.0, 3000.0)).unwrap();
        let clicked = map.on_map(Vec2::new(4000.0, 2500.0));
        centre_on(&mut view, rect, map.on_canvas(clicked));
        let (a, b) = visible(&view, rect);
        let middle = (a + b) / 2.0;
        assert!((middle - Vec2::new(4000.0, 2500.0)).length() < 1.0);
    }

    #[test]
    fn the_map_hides_while_the_whole_graph_is_on_screen() {
        let registry = Registry::standard();
        let mut g = Graph::new("Stone");
        g.apply(
            registry,
            &matgraph::GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::new(100.0, 100.0),
            },
        )
        .unwrap();
        let view = View::default();
        assert!(map_of(&view, canvas(), &g, registry).is_none());
        // Far off to the right: it shows, and holds the part on screen.
        g.apply(
            registry,
            &matgraph::GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::new(6000.0, 100.0),
            },
        )
        .unwrap();
        let map = map_of(&view, canvas(), &g, registry).unwrap();
        let (a, b) = visible(&view, canvas());
        assert!(map.rect.contains(map.on_map(a)));
        assert!(map.rect.contains(map.on_map(b)));
        assert!(map.rect.contains(map.on_map(Vec2::new(6100.0, 150.0))));
        // An empty graph has no map.
        assert!(map_of(&view, canvas(), &Graph::new("Empty"), registry).is_none());
    }
}
