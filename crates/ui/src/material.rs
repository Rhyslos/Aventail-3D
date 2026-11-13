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
//! - `giving`: what a material's textures become when a graph is given
//!   to it (decision 0262).
//! - `export_set`: giving the material to the selected shapes and
//!   exporting it as a texture set, with the export check.
//! - `own_boxes`, `own_rows`, `own_shelf`: your own boxes (decision 0263):
//!   making one (Alt + G), opening one back up (Shift + G), going inside
//!   one, its plugs and sliders in Attributes, and the library on the
//!   shelf.
//! - `starters`: the starter graphs (decision 0264): the Starter graphs
//!   window and the shelf's cards, opening a copy of one.
//! - `shape_canvas`: the shape canvas (decision 0273), where a Shape
//!   box's drawing is made; it takes the canvas's place while open.
//! - `player`: the preview's player (decision 0277): plays the graph's
//!   moving boxes, and chooses the moment exports bake.
//! - `game_rows`: settings marked "change in the game" in Attributes.
//! - `bypassing`: bypassing a box (decision 0278): its switch, its
//!   right-click menu and the Attributes tick.
//! - `rerouting`: reroute dots on wires (decision 0278): where wires run,
//!   adding, moving and removing dots.

pub(crate) mod box_rows;
pub(crate) mod budget;
pub(crate) mod bypassing;
mod canvas;
mod drags;
mod drawing;
mod edits;
pub(crate) mod export_set;
pub(crate) mod game_rows;
pub(crate) mod giving;
mod graphs_bar;
pub(crate) mod laying;
mod letting_go;
pub(crate) mod look;
mod note;
pub(crate) mod own_boxes;
mod own_rows;
pub(crate) mod own_shelf;
#[cfg(test)]
mod paste_tests;
pub(crate) mod pictures;
pub(crate) mod player;
pub(crate) mod preview;
pub(crate) mod rerouting;
mod search;
pub(crate) mod shape_canvas;
pub(crate) mod starters;
pub(crate) mod tidy;
pub(crate) mod tiling;
pub(crate) mod view;

use egui::{Context, Key, KeyboardShortcut, Modifiers};
use matgraph::{Graph, GraphId, Registry};
use pictures::PictureJob;
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
    /// The preview's player, for graphs that move (decision 0277).
    pub(crate) player: player::Player,
    /// Boxes copied, to paste, in this tab or another (shared by every
    /// tab, decision 0268).
    pub(crate) clipboard: Option<Arc<edits::Copied>>,
    /// The project's number in the run (`Workbench::project`): boxes
    /// copied in another project bring their own boxes along.
    pub(crate) project: u64,
    /// Keys pressed over the canvas this frame.
    keys: Vec<CanvasKey>,
    /// Counts undo steps made here, for their merge keys.
    steps: u64,
    /// Counts presses in Attributes: each starts a new undo step there.
    sessions: u64,
    /// The graph the player was last set to its moment for.
    player_for: Option<GraphId>,
    /// Whether the Material mode was on last frame.
    pub(crate) on: bool,
    /// The layout in use before the Material mode, to go back to.
    pub(crate) layout_before: Option<crate::workbench::LayoutChoice>,
    /// The graph's name being typed, while renaming it, and whether the
    /// field still has to take the keyboard.
    renaming_graph: Option<(String, bool)>,
    /// Texture set exports (decision 0262).
    pub(crate) export_set: export_set::SetExport,
    /// Asked this frame: give the material to the selected shapes.
    pub(crate) give_asked: bool,
    /// Asked this frame: export the material's textures (pick a folder).
    pub(crate) export_asked: bool,
    /// The own boxes opened, outermost first (decision 0263): the canvas
    /// shows the boxes inside the last.
    pub(crate) inside: Vec<own_boxes::Inside>,
    /// Making your own box (Alt + G): the window's choices.
    pub(crate) making: Option<own_boxes::Making>,
    /// Asked this frame: a box file to write or read (the app asks
    /// where).
    pub(crate) box_file_asked: Option<own_boxes::BoxFileAsk>,
    /// The project's registry as of this frame (the standard boxes and
    /// its own), for drawing and finding things; changes are checked
    /// with the document's own.
    registry: Option<Arc<Registry>>,
    /// Your library of own boxes, kept with the app's settings (decision
    /// 0263): on the shelf in every project.
    pub(crate) library: Vec<fileio::own_box_file::BoxFile>,
    /// Own boxes' pictures as textures, by id.
    own_textures: OwnTextures,
    /// Whether the Starter graphs window is open (decision 0264).
    pub(crate) starters_open: bool,
    /// The starters' card pictures as textures.
    starter_textures: starters::StarterTextures,
    /// The shelf's tab: the starter graphs (true) or your own boxes;
    /// `None` until one is picked (your own boxes when there are any).
    pub(crate) shelf_starters: Option<bool>,
    /// The shape canvas, while open on a Shape box (decision 0273).
    pub(crate) shape: Option<shape_canvas::ShapeCanvas>,
    /// Asked this frame: a reference image for the shape canvas (the app
    /// asks which).
    pub(crate) reference_asked: bool,
    /// The undo merge key of a field on the shape canvas being changed (a
    /// slider dragged, a name typed).
    shape_field: Option<u64>,
    /// Shape pieces copied, to paste on any Shape box's canvas, in this
    /// tab or another (decision 0274).
    pub(crate) pieces_clipboard: Option<Arc<shapes::PieceCopies>>,
}

/// Own boxes' pictures as textures, by id, with what each was made from.
#[derive(Default)]
pub(crate) struct OwnTextures(std::collections::HashMap<String, (u64, egui::TextureHandle)>);

impl std::fmt::Debug for OwnTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OwnTextures").field(&self.0.len()).finish()
    }
}

impl std::ops::Deref for OwnTextures {
    type Target = std::collections::HashMap<String, (u64, egui::TextureHandle)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OwnTextures {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl MaterialEditor {
    /// The graph shown, as it is in `document`: the project's graph, or
    /// the graph inside the own box opened.
    pub(crate) fn current<'d>(&self, document: &'d Document) -> Option<&'d Arc<Graph>> {
        let graphs = document.scene().graphs();
        match self.inside.last() {
            Some(inside) => graphs.own(&inside.own).map(|o| &o.inner),
            None => graphs.get(self.graph?),
        }
    }

    /// The kinds of box the canvas knows this frame: the standard boxes
    /// and the project's own.
    pub(crate) fn registry(&self) -> &Registry {
        self.registry
            .as_deref()
            .unwrap_or_else(|| Registry::standard())
    }

    /// Every frame in the Material mode, before the panels: keeps the
    /// graph shown one that exists, and sends it to be worked out.
    pub(crate) fn begin_frame(&mut self, ctx: &Context, document: &Document) {
        let graphs = document.scene().graphs();
        self.registry = Some(graphs.registry().clone());
        // An own box opened that is gone (undone): back out of it.
        while self
            .inside
            .last()
            .is_some_and(|i| graphs.own(&i.own).is_none())
        {
            self.leave_inside(1);
        }
        if self.graph.is_none_or(|id| graphs.get(id).is_none()) {
            let first = graphs.iter().next().map(|(id, _)| id);
            if first != self.graph {
                self.show_graph(first);
            }
        }
        self.canvas.last_rect = self.canvas.rect.take();
        // Small pictures while anything is held down (a slider, a box),
        // and while the player plays.
        let time = self.tick_player(ctx, document);
        let dragging = ctx.input(|i| i.pointer.any_down()) || self.player.playing;
        let graph = self.current(document).cloned();
        let job = graph.as_ref().map(|g| PictureJob {
            time,
            ..self.picture_job(document, g.clone())
        });
        self.pictures.update(ctx, job, dragging);
        if let Some(graph) = &graph {
            self.pictures.keep_only(graph);
        }
    }

    /// Shows graph `id` (a new canvas for it).
    pub(crate) fn show_graph(&mut self, id: Option<GraphId>) {
        if self.graph != id || !self.inside.is_empty() {
            self.graph = id;
            self.canvas = Canvas::default();
            self.renaming_graph = None;
            self.inside.clear();
            self.making = None;
            self.shape = None;
            self.player = player::Player::default();
            self.player_for = None;
        }
    }

    /// Takes the canvas's keys while the pointer is over it (and nothing
    /// is being typed), before the rest of the app sees them: Space
    /// searches boxes rather than commands, Delete removes boxes rather
    /// than objects, and so on.
    pub(crate) fn take_keys(&mut self, ctx: &Context, typing: bool) {
        self.keys.clear();
        if self.take_shape_keys(ctx, typing) {
            return;
        }
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
        // Esc only when there is something on the canvas to close. (Out
        // of your own box is Tab, the project lead's "back" key.)
        let closable = self.canvas.closable() || self.making.is_some();
        if closable && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
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
