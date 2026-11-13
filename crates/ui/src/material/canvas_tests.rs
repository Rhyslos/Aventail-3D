//! Tests for the node canvas: what is under the pointer, and where
//! things were drawn last frame (for the workflow tests).

use super::MaterialEditor;
use super::canvas::{Hit, Places, badge_rect, title_height};
use super::look::FRAME_TITLE;
use super::view::View;
use egui::{Pos2, Rect};
use glam::Vec2;
use matgraph::{BoxId, FrameId, Graph, PlugRef, Registry, ValueKind};
use scene::Document;
use std::collections::BTreeSet;

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

    /// A point halfway along stretch `stretch` of the wire into `to`, as
    /// drawn last frame (workflow tests).
    pub(crate) fn wire_on_screen(
        &self,
        document: &Document,
        to: &PlugRef,
        stretch: usize,
    ) -> Option<Pos2> {
        let places = self.places_now(document)?;
        let wire = places.wires.iter().find(|w| w.to == *to)?;
        let curves = wire.curves(places.zoom, egui::Color32::WHITE);
        Some(curves.get(stretch)?.sample(0.5))
    }

    /// The reroute dots of the wire into `to`, as drawn last frame
    /// (workflow tests).
    pub(crate) fn dots_on_screen(&self, document: &Document, to: &PlugRef) -> Vec<Pos2> {
        self.places_now(document)
            .and_then(|p| p.wires.into_iter().find(|w| w.to == *to))
            .map_or_else(Vec::new, |w| w.dots().to_vec())
    }

    fn places_now(&self, document: &Document) -> Option<Places> {
        let rect = self.canvas.rect.or(self.canvas.last_rect)?;
        let graph = self.current(document)?;
        Some(Places::of(graph, self.registry(), &self.canvas.view, rect))
    }
}

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
