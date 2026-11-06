//! The Material mode ([decision 0258](../../../docs/decisions/0258-node-canvas.md),
//! the plan in `docs/plans/phase-12.md`): the node canvas where a
//! material graph is built, the preview beside it, and the chosen box's
//! settings in Attributes.
//!
//! The canvas draws a project's graph (`matgraph`, kept by the scene):
//! boxes with live pictures, wires coloured by what they carry, comment
//! frames, and the app keeping the layout tidy left to right. Every
//! change is a `scene::Command::EditGraph`, so undo works; when the
//! engine refuses one, a note says why in plain words and offers ways to
//! make it work.
//!
//! - `look`: box sizes, plug places and wire colours.
//! - `view`: pan and zoom.
//! - `tidy`: the tidy left-to-right layout.
//! - `pictures`: working the graph out on another thread, and the boxes'
//!   textures.
//! - `canvas`, `drags`, `letting_go`, `drawing`: the canvas itself.
//! - `search`: the box search (Space or right-click).
//! - `note`: "why it won't work", with its fixes.
//! - `edits`: carrying out changes, fixes, copying and pasting.
//! - `preview`: the preview beside the canvas: the material on the
//!   selected model, a ball, a cube or flat, the seam check and the
//!   texture budget (decision 0260).
//! - `laying`: laying the material on a shape at its real-world size.
//! - `budget`: the texture budget's numbers and warnings.
//! - `box_rows`: the chosen box's or frame's settings in Attributes.
//! - `graphs_bar`: choosing, making and renaming graphs.

mod box_rows;
pub(crate) mod budget;
mod canvas;
mod drags;
mod drawing;
mod edits;
mod graphs_bar;
pub(crate) mod laying;
mod letting_go;
pub(crate) mod look;
mod note;
pub(crate) mod pictures;
pub(crate) mod preview;
mod search;
pub(crate) mod tidy;
pub(crate) mod view;

use egui::{Context, Key, KeyboardShortcut, Modifiers};
use matgraph::{Graph, GraphId};
use scene::Document;
use std::sync::Arc;

pub(crate) use canvas::Canvas;

/// Space over the canvas: the box search (the command search elsewhere).
pub(crate) const BOX_SEARCH_KEY: Key = Key::Space;
/// C: a comment frame round the selected boxes.
pub(crate) const COMMENT_KEY: Key = Key::C;
/// Z: frame the selected boxes, or all.
pub(crate) const FRAME_KEY: Key = Key::Z;
/// Delete: remove the selected boxes.
pub(crate) const REMOVE_KEY: Key = Key::Delete;
/// Backspace: remove the selected boxes, keeping the wire through.
pub(crate) const DISSOLVE_KEY: Key = Key::Backspace;
/// Ctrl + C: copy the selected boxes.
pub(crate) const COPY_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
/// Ctrl + V: paste.
pub(crate) const PASTE_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);
/// Alt + C: copy for a linked copy.
pub(crate) const LINKED_COPY_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::C);
/// Alt + V: paste a linked copy.
pub(crate) const LINKED_PASTE_KEYS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::ALT, Key::V);
/// Alt + G: make your own box of the selected boxes (milestone 7).
pub(crate) const OWN_BOX_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::G);
/// Shift + G: open your own box back up (milestone 7).
pub(crate) const OPEN_BOX_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::SHIFT, Key::G);

/// The canvas panel's accessible name.
pub(crate) const CANVAS_LABEL: &str = crate::text!("Material graph canvas");
/// The button that makes a new graph.
pub(crate) const NEW_GRAPH_LABEL: &str = crate::text!("New graph");
/// The box search's field.
pub(crate) const BOX_SEARCH_LABEL: &str = crate::text!("Find a box");
/// The heading over the boxes that usually come next.
pub(crate) const COMES_NEXT: &str = crate::text!("Usually comes next");
/// The heading over a box's own explanation.
pub(crate) const WHAT_IT_DOES: &str = crate::text!("What does this do?");

/// A key pressed over the canvas, taken before the rest of the app sees
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanvasKey {
    Search,
    Comment,
    Frame,
    Remove,
    Dissolve,
    Copy,
    Paste,
    LinkedCopy,
    LinkedPaste,
    OwnBox,
    OpenBox,
    Escape,
}

/// The Material mode's state: the graph shown, the canvas, its pictures
/// and the preview.
#[derive(Debug, Default)]
pub(crate) struct MaterialEditor {
    /// The graph shown.
    pub(crate) graph: Option<GraphId>,
    /// The canvas: its view, selection and what is being done on it.
    pub(crate) canvas: Canvas,
    /// The boxes' live pictures.
    pub(crate) pictures: pictures::Pictures,
    /// How the preview shows the picture.
    pub(crate) preview: preview::Preview,
    /// Boxes copied, to paste.
    clipboard: Option<edits::Copied>,
    /// Keys pressed over the canvas this frame.
    keys: Vec<CanvasKey>,
    /// Counts undo steps made here, for their merge keys.
    steps: u64,
    /// Counts presses in Attributes: each starts a new undo step there.
    sessions: u64,
    /// Whether the Material mode was on last frame.
    pub(crate) on: bool,
    /// The layout in use before the Material mode, to go back to.
    pub(crate) layout_before: Option<crate::workbench::LayoutChoice>,
    /// The graph's name being typed, while renaming it, and whether the
    /// field still has to take the keyboard.
    renaming_graph: Option<(String, bool)>,
}

impl MaterialEditor {
    /// The graph shown, as it is in `document`.
    pub(crate) fn current<'d>(&self, document: &'d Document) -> Option<&'d Arc<Graph>> {
        document.scene().graphs().get(self.graph?)
    }

    /// Every frame in the Material mode, before the panels: keeps the
    /// graph shown one that exists, and sends it to be worked out.
    pub(crate) fn begin_frame(&mut self, ctx: &Context, document: &Document) {
        let graphs = document.scene().graphs();
        if self.graph.is_none_or(|id| graphs.get(id).is_none()) {
            let first = graphs.iter().next().map(|(id, _)| id);
            if first != self.graph {
                self.show_graph(first);
            }
        }
        self.canvas.last_rect = self.canvas.rect.take();
        // Small pictures while anything is held down (a slider, a box).
        let dragging = ctx.input(|i| i.pointer.any_down());
        let graph = self.current(document).cloned();
        self.pictures.update(ctx, graph.as_ref(), dragging);
        if let Some(graph) = &graph {
            self.pictures.keep_only(graph);
        }
    }

    /// Shows graph `id` (a new canvas for it).
    pub(crate) fn show_graph(&mut self, id: Option<GraphId>) {
        if self.graph != id {
            self.graph = id;
            self.canvas = Canvas::default();
            self.renaming_graph = None;
        }
    }

    /// Takes the canvas's keys while the pointer is over it (and nothing
    /// is being typed), before the rest of the app sees them: Space
    /// searches boxes rather than commands, Delete removes boxes rather
    /// than objects, and so on.
    pub(crate) fn take_keys(&mut self, ctx: &Context, typing: bool) {
        self.keys.clear();
        // Where the canvas was drawn last frame (the panels come after).
        let over = self
            .canvas
            .rect
            .or(self.canvas.last_rect)
            .zip(ctx.pointer_hover_pos())
            .is_some_and(|(rect, at)| rect.contains(at));
        if typing || !over || self.graph.is_none() {
            return;
        }
        let shortcuts = [
            (COPY_KEYS, CanvasKey::Copy),
            (PASTE_KEYS, CanvasKey::Paste),
            (LINKED_COPY_KEYS, CanvasKey::LinkedCopy),
            (LINKED_PASTE_KEYS, CanvasKey::LinkedPaste),
            (OWN_BOX_KEYS, CanvasKey::OwnBox),
            (OPEN_BOX_KEYS, CanvasKey::OpenBox),
        ];
        let plain = [
            (BOX_SEARCH_KEY, CanvasKey::Search),
            (COMMENT_KEY, CanvasKey::Comment),
            (FRAME_KEY, CanvasKey::Frame),
            (REMOVE_KEY, CanvasKey::Remove),
            (DISSOLVE_KEY, CanvasKey::Dissolve),
        ];
        ctx.input_mut(|i| {
            for (keys, what) in shortcuts {
                if i.consume_shortcut(&keys) {
                    self.keys.push(what);
                }
            }
            for (key, what) in plain {
                if i.consume_key(Modifiers::NONE, key) {
                    self.keys.push(what);
                }
            }
        });
        // Esc only when there is something on the canvas to close.
        if self.canvas.closable() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.keys.push(CanvasKey::Escape);
        }
    }

    /// The middle of the canvas as last drawn, on the canvas.
    pub(crate) fn canvas_middle(&self) -> glam::Vec2 {
        let rect = self.canvas.rect.or(self.canvas.last_rect);
        rect.map_or(glam::Vec2::ZERO, |r| {
            self.canvas.view.on_canvas(r, r.center())
        })
    }

    /// A new undo merge key, apart from every other step's.
    fn next_key(&mut self) -> u64 {
        /// Marks the material canvas's merge keys apart from every other.
        const GRAPH_KEYS: u64 = 0x4d41_5447_0000_0000;
        self.steps += 1;
        GRAPH_KEYS | self.steps
    }
}
