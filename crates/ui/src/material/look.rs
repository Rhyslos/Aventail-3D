//! How the node canvas looks: the size of a box and where its plugs sit
//! (in canvas units, the same as screen points at full zoom), and the
//! colour of each kind of wire (decision 0258, the mockups' colours).

use egui::Color32;
use glam::Vec2;
use matgraph::{BoxId, BoxInfo, Graph, Registry, ValueKind};

/// How wide every box is.
pub(crate) const BOX_WIDTH: f32 = 140.0;
/// The title band across a box's top.
pub(crate) const HEADER: f32 = 22.0;
/// The gap around a box's picture.
pub(crate) const PAD: f32 = 8.0;
/// The picture's side: the box's width less its gaps.
pub(crate) const PICTURE: f32 = BOX_WIDTH - 2.0 * PAD;
/// The height of one row of plugs under the picture.
pub(crate) const ROW: f32 = 18.0;
/// The space under the last row.
const FOOT: f32 = 6.0;
/// How far apart the tidy layout's columns are: a box and room for its
/// wires.
pub(crate) const COLUMN: f32 = 220.0;
/// The tidy layout's step down a column.
pub(crate) const STEP: f32 = 10.0;
/// The space the tidy layout keeps between boxes in a column.
pub(crate) const GAP: f32 = 20.0;
/// A plug's dot, on screen at full zoom.
pub(crate) const PLUG_RADIUS: f32 = 5.0;
/// A comment frame's title band, at full zoom.
pub(crate) const FRAME_TITLE: f32 = 28.0;
/// The room a new frame leaves round its boxes.
pub(crate) const FRAME_MARGIN: f32 = 20.0;
/// A frame's note on the canvas, under its title (decision 0264): its
/// font size and the gap at its sides, at full zoom. The starter graphs
/// leave room for their notes with these (`fileio::starter_graphs`).
pub(crate) const NOTE_SIZE: f32 = 12.0;
pub(crate) const NOTE_PAD: f32 = 8.0;
/// Below this zoom a frame's note is too small to read and isn't drawn.
pub(crate) const NOTE_ZOOM: f32 = 0.45;

/// The colour of wires and plugs carrying `kind`: shape grey, height
/// amber, colour blue, whole material green.
pub(crate) fn kind_colour(kind: ValueKind) -> Color32 {
    match kind {
        ValueKind::Shape => Color32::from_rgb(0xb4, 0xb4, 0xb4),
        ValueKind::Height => Color32::from_rgb(0xe8, 0xb3, 0x4a),
        ValueKind::Colour => Color32::from_rgb(0x5a, 0x95, 0xf2),
        ValueKind::Material => Color32::from_rgb(0x90, 0xbd, 0x5a),
    }
}

/// The red of a plug that won't take a wire, and of a box that can't
/// run.
pub(crate) const REFUSED: Color32 = Color32::from_rgb(0xe8, 0x5a, 0x50);
/// The ring round boxes and frames matching a search (decision 0279).
pub(crate) const FOUND: Color32 = Color32::from_rgb(0xff, 0xe0, 0x6a);
/// The green ring of a plug that takes the wire being dragged.
pub(crate) const FITS: Color32 = Color32::from_rgb(0x8f, 0xe3, 0x9a);

/// How many rows of plugs a box of kind `info` has (at least one; a kind
/// this app doesn't know shows one).
pub(crate) fn rows(info: Option<&BoxInfo>) -> usize {
    info.map_or(1, |i| i.inputs.len().max(i.outputs.len()).max(1))
}

/// Where the rows of plugs start, down from a box's top.
const fn rows_top() -> f32 {
    HEADER + PAD / 2.0 + PICTURE + PAD / 2.0
}

/// A box's size.
pub(crate) fn box_size(info: Option<&BoxInfo>) -> Vec2 {
    Vec2::new(BOX_WIDTH, rows_top() + rows(info) as f32 * ROW + FOOT)
}

/// The size of a box of kind `kind` (an id in `registry`).
pub(crate) fn size_of(registry: &Registry, kind: &str) -> Vec2 {
    box_size(registry.get(kind).map(|t| t.info()))
}

/// Box `id`'s rectangle in `graph`, as its two corners.
pub(crate) fn box_rect(graph: &Graph, registry: &Registry, id: BoxId) -> Option<(Vec2, Vec2)> {
    let b = graph.get(id)?;
    Some((b.at, b.at + size_of(registry, &b.kind)))
}

/// The picture's top left corner, from the box's.
pub(crate) fn picture_offset() -> Vec2 {
    Vec2::new(PAD, HEADER + PAD / 2.0)
}

/// Where input `index` sits, from the box's corner: on its left edge.
pub(crate) fn input_offset(index: usize) -> Vec2 {
    Vec2::new(0.0, rows_top() + (index as f32 + 0.5) * ROW)
}

/// Where output `index` sits, from the box's corner: on its right edge.
pub(crate) fn output_offset(index: usize) -> Vec2 {
    Vec2::new(BOX_WIDTH, rows_top() + (index as f32 + 0.5) * ROW)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_own_colour() {
        let colours: std::collections::BTreeSet<_> = ValueKind::ALL
            .iter()
            .map(|k| kind_colour(*k).to_array())
            .collect();
        assert_eq!(colours.len(), 4);
        assert_eq!(
            kind_colour(ValueKind::Height),
            Color32::from_rgb(0xe8, 0xb3, 0x4a)
        );
    }

    #[test]
    fn plugs_sit_on_the_edges_under_the_picture() {
        let registry = Registry::standard();
        let blend = registry.get("blend").unwrap().info();
        let size = box_size(Some(blend));
        assert_eq!(rows(Some(blend)), 3);
        for i in 0..3 {
            let at = input_offset(i);
            assert_eq!(at.x, 0.0);
            assert!(at.y > picture_offset().y + PICTURE && at.y < size.y);
        }
        assert_eq!(output_offset(0).x, BOX_WIDTH);
        assert_eq!(rows(None), 1, "an unknown kind still has a row");
    }
}
