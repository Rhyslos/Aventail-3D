//! The recipes the starter graph files are made from: each starter built
//! box by box, laid out tidily left to right in comment frames, each
//! frame with a note saying what its step does and why.
//!
//! `UPDATE_STARTERS=1 cargo test -p fileio starter` writes the files
//! (`crates/fileio/starters/`) again from these recipes.

mod leaves;
pub(super) mod materials;

use glam::Vec2;
use matgraph::{
    BoxId, ColourGradient, Frame, GradientStop, Graph, GraphEdit, PlugRef, Registry, SettingValue,
};

/// How far apart the canvas's columns are (the node canvas's tidy
/// layout, `ui::material::look::COLUMN`).
pub(super) const COLUMN: f32 = 220.0;
/// How wide a box is on the canvas (`look::BOX_WIDTH`).
pub(super) const BOX_WIDTH: f32 = 140.0;
/// A box's height without its rows of plugs, and each row
/// (`look::box_size`).
const BOX_TOP: f32 = 160.0;
const BOX_ROW: f32 = 18.0;
/// The gap kept between boxes in a column (`look::GAP`).
const GAP: f32 = 20.0;
/// A frame's title band and its margin round its boxes (`look`).
const FRAME_TITLE: f32 = 28.0;
const FRAME_MARGIN: f32 = 20.0;
/// The gap between frames stacked in one column.
const LANE_GAP: f32 = 50.0;

/// How a frame's note is laid out on the canvas
/// (`ui::material::drawing`): its font size, the gap round it, and a
/// generous guess of a letter's width and a line's height there, so the
/// room kept for it is never too small (a test in the ui crate checks
/// with the real font).
pub(super) const NOTE_PAD: f32 = 8.0;
const LETTER: f32 = 6.4;
const LINE: f32 = 16.0;

/// The colours of the frames, by step: the working order's groups.
pub(super) const MAKE: [u8; 3] = [86, 110, 140];
pub(super) const SHAPE: [u8; 3] = [150, 115, 60];
pub(super) const DETAIL: [u8; 3] = [120, 95, 150];
pub(super) const COLOUR: [u8; 3] = [160, 80, 95];
pub(super) const FINISH: [u8; 3] = [90, 130, 70];

/// A setting's value, shortly.
pub(super) fn num(v: f32) -> SettingValue {
    SettingValue::Number(v)
}
pub(super) fn whole(v: i32) -> SettingValue {
    SettingValue::Whole(v)
}
pub(super) fn choice(v: u32) -> SettingValue {
    SettingValue::Choice(v)
}
pub(super) fn switch(v: bool) -> SettingValue {
    SettingValue::Switch(v)
}
pub(super) fn colour(hex: u32) -> SettingValue {
    SettingValue::Colour(rgb(hex))
}
/// A colour from `0xRRGGBB`.
pub(super) fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}
/// A gradient from places along it and `0xRRGGBB` colours.
pub(super) fn gradient(stops: &[(f32, u32)]) -> SettingValue {
    let stops: Vec<GradientStop> = stops
        .iter()
        .map(|&(at, c)| GradientStop { at, colour: rgb(c) })
        .collect();
    SettingValue::Gradient(ColourGradient::new(&stops).expect("2 to 8 stops"))
}

/// One step's frame, before it is laid out.
struct Step {
    title: String,
    colour: [u8; 3],
    note: String,
    /// Stacked under the frames above it in the same columns.
    lane: u32,
    boxes: Vec<BoxId>,
}

/// A starter graph being built.
pub(super) struct Recipe {
    graph: Graph,
    registry: Registry,
    columns: std::collections::BTreeMap<BoxId, i32>,
    steps: Vec<Step>,
}

impl Recipe {
    /// A recipe for a graph named `name`.
    pub(super) fn new(name: &str) -> Self {
        Self {
            graph: Graph::new(name),
            registry: Registry::standard().clone(),
            columns: Default::default(),
            steps: Vec::new(),
        }
    }

    /// Adds a box of kind `kind` in column `column`, with `settings`.
    pub(super) fn add(
        &mut self,
        kind: &str,
        column: i32,
        settings: &[(&str, SettingValue)],
    ) -> BoxId {
        let id = BoxId(self.graph.next_id());
        self.edit(GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::ZERO,
        });
        for (key, value) in settings {
            self.edit(GraphEdit::SetSetting {
                node: id,
                key: (*key).into(),
                value: *value,
            });
        }
        self.columns.insert(id, column);
        id
    }

    /// A wire from `from`'s output to `to`'s input.
    pub(super) fn wire(&mut self, from: (BoxId, &str), to: (BoxId, &str)) {
        assert!(
            self.columns[&to.0] > self.columns[&from.0],
            "wires run left to right: {from:?} to {to:?}"
        );
        self.edit(GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        });
    }

    /// Gives Shape box `id` the drawing `drawing`.
    pub(super) fn draw(&mut self, id: BoxId, drawing: shapes::Drawing) {
        self.edit(GraphEdit::SetDrawing {
            node: id,
            drawing: std::sync::Arc::new(drawing),
        });
    }

    /// Shows `keys` of box `id` in the painter.
    pub(super) fn show(&mut self, id: BoxId, keys: &[&str]) {
        for key in keys {
            self.edit(GraphEdit::ShowInPainter {
                node: id,
                key: (*key).into(),
                shown: true,
            });
        }
    }

    /// A comment frame titled `title` round `boxes`, with `note`, in lane
    /// `lane` (0 at the top; a higher lane sits under the lower ones in
    /// the same columns).
    pub(super) fn step(
        &mut self,
        title: &str,
        colour: [u8; 3],
        lane: u32,
        note: &str,
        boxes: &[BoxId],
    ) {
        self.steps.push(Step {
            title: title.into(),
            colour,
            note: note.into(),
            lane,
            boxes: boxes.to_vec(),
        });
    }

    fn edit(&mut self, edit: GraphEdit) {
        if let Err(why) = self.graph.apply(&self.registry, &edit) {
            panic!("{edit:?}: {}", why.reason);
        }
    }

    fn height(&self, id: BoxId) -> f32 {
        let kind = &self.graph.get(id).unwrap().kind;
        let info = self.registry.get(kind).unwrap().info();
        let rows = info.inputs.len().max(info.outputs.len()).max(1);
        BOX_TOP + rows as f32 * BOX_ROW
    }

    /// The graph, laid out: each step's frame in its columns, its note
    /// under the title, its boxes stacked under the note.
    pub(super) fn finish(mut self) -> Graph {
        let every: std::collections::BTreeSet<BoxId> =
            self.steps.iter().flat_map(|s| s.boxes.clone()).collect();
        assert_eq!(
            every.len(),
            self.graph.box_count(),
            "every box sits in one frame"
        );
        // The lowest point used so far in each column.
        let mut bottom: std::collections::BTreeMap<i32, f32> = Default::default();
        let mut steps = std::mem::take(&mut self.steps);
        steps.sort_by_key(|s| s.lane);
        let mut moves = Vec::new();
        let mut frames = Vec::new();
        for step in steps {
            let cols: Vec<i32> = step.boxes.iter().map(|b| self.columns[b]).collect();
            let (first, last) = (*cols.iter().min().unwrap(), *cols.iter().max().unwrap());
            let left = first as f32 * COLUMN - FRAME_MARGIN;
            let right = last as f32 * COLUMN + BOX_WIDTH + FRAME_MARGIN;
            let top = (first..=last)
                .filter_map(|c| bottom.get(&c))
                .fold(0.0f32, |a, b| a.max(*b + LANE_GAP));
            let note = note_height(&step.note, right - left);
            let boxes_top = snap(top + FRAME_TITLE + note + FRAME_MARGIN);
            let mut low = boxes_top;
            for c in first..=last {
                let mut y = boxes_top;
                for b in step.boxes.iter().filter(|b| self.columns[b] == c) {
                    moves.push((*b, Vec2::new(c as f32 * COLUMN, y)));
                    y = snap(y + self.height(*b) + GAP);
                }
                low = low.max(y - GAP);
            }
            let frame_bottom = low + FRAME_MARGIN;
            for c in first..=last {
                bottom.insert(c, frame_bottom);
            }
            let mut frame = Frame::new(
                step.title,
                Vec2::new(left, top),
                Vec2::new(right, frame_bottom),
            );
            frame.colour = step.colour;
            frame.note = step.note;
            frames.push(frame);
        }
        for (id, at) in moves {
            let by = at - self.graph.get(id).unwrap().at;
            self.edit(GraphEdit::MoveBoxes { ids: vec![id], by });
        }
        for frame in frames {
            self.edit(GraphEdit::AddFrame(frame));
        }
        self.graph
    }
}

/// `v` up to the canvas's step of 10.
fn snap(v: f32) -> f32 {
    (v / 10.0).ceil() * 10.0
}

/// The room a note takes in a frame `width` wide: words wrapped at a
/// generous letter width.
pub(super) fn note_height(note: &str, width: f32) -> f32 {
    if note.is_empty() {
        return 0.0;
    }
    let per_line = ((width - 2.0 * NOTE_PAD) / LETTER).floor().max(8.0) as usize;
    let mut lines = 0;
    for paragraph in note.split('\n') {
        lines += 1;
        let mut used = 0;
        for word in paragraph.split_whitespace() {
            let len = word.chars().count();
            if used > 0 && used + 1 + len > per_line {
                lines += 1;
                used = len;
            } else {
                used += if used > 0 { 1 } else { 0 } + len;
            }
        }
    }
    lines as f32 * LINE + NOTE_PAD
}
