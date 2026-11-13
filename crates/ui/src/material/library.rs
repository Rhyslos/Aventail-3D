//! The library browser ([decision 0292](../../../../docs/decisions/0292-library-browser.md)):
//! one window for every material graph and own box there is to use: this
//! project's, your library's (kept with the app's settings, in every
//! project) and the starter graphs. Each is a card with its picture,
//! found by a search of names and tags, or by clicking tags.
//!
//! - A click chooses a card (Ctrl adds more, Shift takes one away, as
//!   everywhere in the app); a double-click opens it: a graph is shown
//!   (a copy is made in the project when it isn't one of the project's),
//!   and an own box is placed on the graph shown.
//! - The chosen card's tags are changed under the cards; for the
//!   project's graphs and boxes each change is an undo step.
//! - **Packs:** the chosen graphs and boxes are written to one file
//!   (`fileio::pack_file`) to share; **Bring in…** reads a pack, a graph
//!   file or a box file into your library.
//!
//! Graphs' pictures are worked out on a thread of their own and kept
//! (`library_pictures`), so scrolling and searching never make work.

use super::MaterialEditor;
use super::library_pictures::CardPictures;
use crate::text::tr;
use crate::theme;
use egui::{Context, RichText, Sense, Ui, vec2};
use fileio::graph_file::GraphFile;
use fileio::own_box_file::BoxFile;
use fileio::pack_file::Pack;
use fileio::starter_graphs::STARTERS;
use matgraph::{Graph, GraphEdit, GraphId, OwnBox, Registry};
use scene::{Command, Document};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The window's title, and the canvas bar's button opening it.
pub(crate) const LIBRARY_LABEL: &str = crate::text!("Library");
/// The search field.
pub(crate) const SEARCH_LABEL: &str = crate::text!("Search the library");
/// The field adding a tag.
pub(crate) const ADD_TAG_LABEL: &str = crate::text!("Add a tag");
/// The button writing a pack.
pub(crate) const EXPORT_PACK: &str = crate::text!("Export as a pack…");
/// The button reading a pack, graph file or box file into your library.
pub(crate) const BRING_IN: &str = crate::text!("Bring in…");
/// The drop-down choosing what is shown.
pub(crate) const SHOW_LABEL: &str = crate::text!("Show");
/// The drop-down choosing where from.
pub(crate) const FROM_LABEL: &str = crate::text!("From");

/// The most graphs your library keeps.
pub(crate) const MAX_LIBRARY_GRAPHS: usize = 256;

/// A card's size, its picture's side, and the gap between cards.
const CARD: egui::Vec2 = vec2(112.0, 140.0);
const PICTURE: f32 = 96.0;

/// One card in the browser.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ItemId {
    /// One of the project's graphs.
    ProjectGraph(GraphId),
    /// One of the project's own boxes, by id.
    ProjectBox(String),
    /// A graph in your library, by name (names there are kept apart).
    LibraryGraph(String),
    /// An own box in your library, by id.
    LibraryBox(String),
    /// A starter graph, by key.
    Starter(&'static str),
}

impl ItemId {
    /// Where it is.
    pub(crate) fn place(&self) -> Place {
        match self {
            Self::ProjectGraph(_) | Self::ProjectBox(_) => Place::Project,
            Self::LibraryGraph(_) | Self::LibraryBox(_) => Place::Library,
            Self::Starter(_) => Place::Starters,
        }
    }

    /// Whether it is a graph (else an own box).
    pub(crate) fn is_graph(&self) -> bool {
        !matches!(self, Self::ProjectBox(_) | Self::LibraryBox(_))
    }
}

/// What the browser shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Show {
    /// Graphs and own boxes.
    #[default]
    Everything,
    /// Graphs only.
    Graphs,
    /// Own boxes only.
    OwnBoxes,
}

impl Show {
    const ALL: [Self; 3] = [Self::Everything, Self::Graphs, Self::OwnBoxes];

    fn name(self) -> &'static str {
        match self {
            Self::Everything => tr("Graphs and own boxes"),
            Self::Graphs => tr("Graphs"),
            Self::OwnBoxes => tr("Own boxes"),
        }
    }
}

/// Where a card's graph or box is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Place {
    /// In this project (saved with it).
    Project,
    /// In your library (kept with the app's settings).
    Library,
    /// One of the starter graphs, shipped with the app.
    Starters,
}

impl Place {
    const ALL: [Self; 3] = [Self::Project, Self::Library, Self::Starters];

    fn name(self) -> &'static str {
        match self {
            Self::Project => tr("This project"),
            Self::Library => tr("Your library"),
            Self::Starters => tr("Starter graphs"),
        }
    }
}

/// A graph kept in your library, with the kinds of box it uses, ready to
/// be pictured.
#[derive(Debug)]
pub(crate) struct LibraryGraph {
    /// The graph and its own boxes.
    pub(crate) file: GraphFile,
    /// The graph, shared with its card's picture.
    pub(crate) graph: Arc<Graph>,
    /// The standard boxes and its own boxes.
    pub(crate) registry: Arc<Registry>,
}

impl LibraryGraph {
    /// `file`, ready to keep.
    pub(crate) fn new(file: GraphFile) -> Arc<Self> {
        let registry = Arc::new(matgraph::own::registry_with(&file.own));
        let graph = Arc::new(file.graph.clone());
        Arc::new(Self {
            file,
            graph,
            registry,
        })
    }

    /// Its name.
    pub(crate) fn name(&self) -> &str {
        self.graph.name()
    }
}

/// One card's words.
#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub(crate) id: ItemId,
    pub(crate) name: String,
    pub(crate) about: String,
    pub(crate) tags: Vec<String>,
}

impl Item {
    /// The card's accessible name: its name, what it is and where.
    pub(crate) fn label(&self) -> String {
        let what = match (&self.id.is_graph(), self.id.place()) {
            (true, Place::Project) => tr("graph in this project"),
            (true, Place::Library) => tr("graph in your library"),
            (_, Place::Starters) => tr("starter graph"),
            (false, Place::Project) => tr("own box in this project"),
            (false, Place::Library) => tr("own box in your library"),
        };
        format!("{}, {}", self.name, what)
    }
}

/// The library browser's state.
#[derive(Debug, Default)]
pub(crate) struct Browser {
    /// Whether the window is open.
    pub(crate) open: bool,
    /// What is typed in the search field.
    pub(crate) query: String,
    /// Graphs, own boxes or both.
    pub(crate) show: Show,
    /// Only from here, or from everywhere.
    pub(crate) from: Option<Place>,
    /// Tags clicked: only cards with all of them show.
    pub(crate) tags: BTreeSet<String>,
    /// The chosen cards.
    pub(crate) chosen: BTreeSet<ItemId>,
    /// The card whose details show under the cards (the last clicked).
    pub(crate) focus: Option<ItemId>,
    /// The tag being typed.
    pub(crate) new_tag: String,
    /// The graphs' pictures.
    pub(crate) pictures: CardPictures,
}

impl Browser {
    /// Whether `item` shows with the search, tags and choices made.
    pub(crate) fn shows(&self, item: &Item) -> bool {
        let kind = match self.show {
            Show::Everything => true,
            Show::Graphs => item.id.is_graph(),
            Show::OwnBoxes => !item.id.is_graph(),
        };
        kind && self.from.is_none_or(|p| p == item.id.place())
            && self.tags.iter().all(|t| item.tags.contains(t))
            && matgraph::tags::matches(&self.query, &item.name, &item.tags)
    }
}

/// What a library file action asks the app for.
#[derive(Debug, Clone)]
pub(crate) enum LibraryAsk {
    /// Write this pack (and a name for its file).
    SavePack(Pack, String),
    /// Read a pack, graph file or box file into your library.
    BringIn,
}

/// What was asked in the window this frame.
enum Asked {
    Click(ItemId, egui::Modifiers),
    Open(ItemId),
    Keep(ItemId),
    Remove(ItemId),
    SetTags(ItemId, Vec<String>),
    ExportPack,
    BringIn,
}

impl MaterialEditor {
    /// Every card there is, in the order shown: the project's graphs and
    /// own boxes, your library's, then the starters.
    pub(crate) fn library_items(&self, document: &Document) -> Vec<Item> {
        let store = document.scene().graphs();
        let mut items = Vec::new();
        for (id, graph) in store.iter() {
            items.push(Item {
                id: ItemId::ProjectGraph(id),
                name: graph.name().to_string(),
                about: String::new(),
                tags: graph.tags().to_vec(),
            });
        }
        for own in store.own_boxes() {
            items.push(own_item(ItemId::ProjectBox(own.id.clone()), own));
        }
        for lib in &self.library_graphs {
            items.push(Item {
                id: ItemId::LibraryGraph(lib.name().to_string()),
                name: lib.name().to_string(),
                about: String::new(),
                tags: lib.graph.tags().to_vec(),
            });
        }
        for file in &self.library {
            items.push(own_item(
                ItemId::LibraryBox(file.main.id.clone()),
                &file.main,
            ));
        }
        for starter in &STARTERS {
            items.push(Item {
                id: ItemId::Starter(starter.key),
                name: tr(starter.name).to_string(),
                about: tr(starter.teaches).to_string(),
                tags: starter.tags.iter().map(|t| tr(t).to_string()).collect(),
            });
        }
        items
    }

    /// The Library window, while it is open.
    pub(crate) fn library_window(
        &mut self,
        ctx: &Context,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        if !self.browser.open {
            return;
        }
        let items = self.library_items(document);
        let existing: BTreeSet<ItemId> = items.iter().map(|i| i.id.clone()).collect();
        // Choices of cards that are gone (undone, removed) are dropped.
        self.browser.chosen.retain(|id| existing.contains(id));
        if self
            .browser
            .focus
            .as_ref()
            .is_some_and(|f| !existing.contains(f))
        {
            self.browser.focus = None;
        }
        let mut open = true;
        let mut asked = Vec::new();
        let rect = self.canvas.rect.or(self.canvas.last_rect);
        let mut window = egui::Window::new(tr(LIBRARY_LABEL))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .max_height(rect.map_or(700.0, |r| r.height() - 20.0));
        if let Some(rect) = rect {
            window = window.default_pos(rect.center() - vec2(310.0, 300.0));
        }
        window.show(ctx, |ui| {
            self.library_filters(ui, &items);
            ui.separator();
            let shown: Vec<&Item> = items.iter().filter(|i| self.browser.shows(i)).collect();
            egui::ScrollArea::vertical()
                .id_salt("library cards")
                .max_height(330.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if shown.is_empty() {
                        crate::dialog::help(
                            ui,
                            tr("Nothing here matches: try fewer words or tags, or show \
                                everything from everywhere."),
                        );
                    }
                    for place in Place::ALL {
                        let here: Vec<&&Item> =
                            shown.iter().filter(|i| i.id.place() == place).collect();
                        if here.is_empty() {
                            continue;
                        }
                        ui.label(RichText::new(place.name()).color(theme::TEXT_WEAK).small());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                            for item in here {
                                let picture = self.card_picture(ui.ctx(), document, &item.id);
                                let chosen = self.browser.chosen.contains(&item.id);
                                let response = library_card(ui, item, chosen, picture);
                                if response.double_clicked() {
                                    asked.push(Asked::Open(item.id.clone()));
                                } else if response.clicked() {
                                    let modifiers = ui.input(|i| i.modifiers);
                                    asked.push(Asked::Click(item.id.clone(), modifiers));
                                }
                            }
                        });
                    }
                });
            ui.separator();
            let focus = self
                .browser
                .focus
                .as_ref()
                .and_then(|f| items.iter().find(|i| &i.id == f));
            match focus {
                Some(item) => self.library_details(ui, item, &mut asked),
                None => {
                    crate::dialog::help(
                        ui,
                        tr(
                            "Click a card to see it here; double-click to open it. Ctrl + \
                            click chooses more, to export together as a pack.",
                        ),
                    );
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                let packable = self
                    .browser
                    .chosen
                    .iter()
                    .filter(|c| c.place() != Place::Starters)
                    .count();
                ui.label(
                    RichText::new(crate::trf!("{} chosen", self.browser.chosen.len()))
                        .color(theme::TEXT_WEAK),
                );
                if ui
                    .add_enabled(packable > 0, egui::Button::new(tr(EXPORT_PACK)))
                    .on_hover_text(tr(
                        "Write the chosen graphs and own boxes to one file, to share or keep",
                    ))
                    .on_disabled_hover_text(tr(
                        "Choose graphs or own boxes of this project or your library first \
                         (starter graphs come with the app)",
                    ))
                    .clicked()
                {
                    asked.push(Asked::ExportPack);
                }
                if ui
                    .button(tr(BRING_IN))
                    .on_hover_text(tr(
                        "Add a pack, a graph file or a box file someone shared to your library",
                    ))
                    .clicked()
                {
                    asked.push(Asked::BringIn);
                }
            });
        });
        if !open {
            self.browser.open = false;
        }
        let busy = ctx.input(|i| i.pointer.any_down());
        self.browser.pictures.update(ctx, busy, &existing);
        for ask in asked {
            self.carry_out_library_ask(document, ask, &items, note);
        }
    }

    /// The search field, what to show, from where, and the tags in use.
    fn library_filters(&mut self, ui: &mut Ui, items: &[Item]) {
        ui.horizontal(|ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.browser.query)
                    .desired_width(200.0)
                    .hint_text(tr(SEARCH_LABEL)),
            );
            field.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, tr(SEARCH_LABEL))
            });
            field.on_hover_text(tr("Every word typed must be in a card's name or tags"));
            let show = self.browser.show;
            crate::dialog::drop_down(ui, tr(SHOW_LABEL), show.name(), "", 170.0, |ui| {
                for choice in Show::ALL {
                    if crate::dialog::drop_down_entry(ui, choice == show, choice.name(), "")
                        .clicked()
                    {
                        self.browser.show = choice;
                        ui.close();
                    }
                }
            });
            let from = self.browser.from;
            let everywhere = tr("Everywhere");
            let chosen = from.map_or(everywhere, Place::name);
            crate::dialog::drop_down(ui, tr(FROM_LABEL), chosen, "", 140.0, |ui| {
                if crate::dialog::drop_down_entry(ui, from.is_none(), everywhere, "").clicked() {
                    self.browser.from = None;
                    ui.close();
                }
                for place in Place::ALL {
                    if crate::dialog::drop_down_entry(ui, from == Some(place), place.name(), "")
                        .clicked()
                    {
                        self.browser.from = Some(place);
                        ui.close();
                    }
                }
            });
        });
        // The tags of the cards that could show, each a switch.
        let mut tags: BTreeSet<String> = BTreeSet::new();
        let without_tags = Browser {
            query: self.browser.query.clone(),
            show: self.browser.show,
            from: self.browser.from,
            ..Browser::default()
        };
        for item in items.iter().filter(|i| without_tags.shows(i)) {
            tags.extend(item.tags.iter().cloned());
        }
        tags.extend(self.browser.tags.iter().cloned());
        if tags.is_empty() {
            return;
        }
        let mut flip = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Tags:")).color(theme::TEXT_WEAK).small());
            for tag in &tags {
                let on = self.browser.tags.contains(tag);
                if ui
                    .selectable_label(on, tag)
                    .on_hover_text(tr("Show only cards with this tag (click again to stop)"))
                    .clicked()
                {
                    flip = Some(tag.clone());
                }
            }
            if !self.browser.tags.is_empty() && ui.link(tr("Clear")).clicked() {
                self.browser.tags.clear();
            }
        });
        if let Some(tag) = flip
            && !self.browser.tags.remove(&tag)
        {
            self.browser.tags.insert(tag);
        }
    }

    /// The chosen card's details: what it is, its tags (changed here)
    /// and what can be done with it.
    fn library_details(&mut self, ui: &mut Ui, item: &Item, asked: &mut Vec<Asked>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&item.name).color(theme::TEXT_STRONG).strong());
            ui.label(
                RichText::new(
                    item.label()
                        .trim_start_matches(&item.name)
                        .trim_start_matches(", "),
                )
                .color(theme::TEXT_WEAK)
                .small(),
            );
        });
        if !item.about.trim().is_empty() {
            crate::dialog::help(ui, &item.about);
        }
        let fixed = item.id.place() == Place::Starters;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Tags:")).color(theme::TEXT_WEAK).small());
            if item.tags.is_empty() {
                ui.label(
                    RichText::new(tr("none yet"))
                        .color(theme::TEXT_WEAK)
                        .small(),
                );
            }
            for tag in &item.tags {
                if fixed {
                    ui.label(RichText::new(tag).color(theme::TEXT_WEAK));
                    continue;
                }
                let button = ui
                    .small_button(format!("{tag}  ×"))
                    .on_hover_text(tr("Take this tag off"));
                button.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        crate::trf!("Take off the tag {}", tag),
                    )
                });
                if button.clicked() {
                    let tags = item.tags.iter().filter(|t| *t != tag).cloned().collect();
                    asked.push(Asked::SetTags(item.id.clone(), tags));
                }
            }
        });
        if fixed {
            crate::dialog::help(
                ui,
                tr(
                    "A starter graph's tags are fixed; open a copy, or keep one in your \
                    library, to tag it your way.",
                ),
            );
        } else {
            ui.horizontal(|ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut self.browser.new_tag)
                        .desired_width(160.0)
                        .hint_text(tr(ADD_TAG_LABEL)),
                );
                field.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, tr(ADD_TAG_LABEL))
                });
                let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let add = ui
                    .button(tr("Add"))
                    .on_hover_text(tr("Add the tag typed (a word or two, like stone or wall)"))
                    .clicked();
                if (enter || add) && !self.browser.new_tag.trim().is_empty() {
                    let typed = std::mem::take(&mut self.browser.new_tag);
                    let mut tags = item.tags.clone();
                    tags.extend(typed.split(',').map(str::to_string));
                    asked.push(Asked::SetTags(item.id.clone(), tags));
                    if enter {
                        field.request_focus();
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            let open = match (&item.id, item.id.place()) {
                (ItemId::ProjectGraph(_), _) => tr("Show it"),
                (_, Place::Starters) | (ItemId::LibraryGraph(_), _) => {
                    tr("Open a copy in this project")
                }
                _ => tr("Place it"),
            };
            if ui
                .button(open)
                .on_hover_text(tr("Double-clicking the card does the same"))
                .clicked()
            {
                asked.push(Asked::Open(item.id.clone()));
            }
            if item.id.place() == Place::Project
                && ui
                    .button(tr(super::own_rows::SAVE_TO_LIBRARY))
                    .on_hover_text(tr("Keep it for every project"))
                    .clicked()
            {
                asked.push(Asked::Keep(item.id.clone()));
            }
            let remove = match item.id.place() {
                Place::Project => Some((
                    tr("Delete from the project"),
                    tr("Undo brings it back (an own box only goes when no graph uses it)"),
                )),
                Place::Library => Some((
                    tr("Remove from your library"),
                    tr("Every project's copies stay as they are"),
                )),
                Place::Starters => None,
            };
            if let Some((words, hint)) = remove
                && ui.button(words).on_hover_text(hint).clicked()
            {
                asked.push(Asked::Remove(item.id.clone()));
            }
        });
    }

    /// Card `id`'s picture as a texture, if it has one yet.
    fn card_picture(
        &mut self,
        ctx: &Context,
        document: &Document,
        id: &ItemId,
    ) -> Option<egui::TextureId> {
        let store = document.scene().graphs();
        match id {
            ItemId::ProjectGraph(g) => {
                let graph = store.get(*g)?;
                self.browser.pictures.picture(id, graph, store.registry())
            }
            ItemId::LibraryGraph(name) => {
                let lib = self.library_graph(name)?.clone();
                self.browser.pictures.picture(id, &lib.graph, &lib.registry)
            }
            ItemId::ProjectBox(own) => {
                let own = store.own(own)?.clone();
                self.own_texture(ctx, &own)
            }
            ItemId::LibraryBox(own) => {
                let own = self
                    .library
                    .iter()
                    .find(|f| f.main.id == *own)?
                    .main
                    .clone();
                self.own_texture(ctx, &own)
            }
            ItemId::Starter(key) => {
                let starter = STARTERS.iter().find(|s| s.key == *key)?;
                self.starter_texture(ctx, starter)
            }
        }
    }

    /// The graph called `name` in your library.
    pub(crate) fn library_graph(&self, name: &str) -> Option<&Arc<LibraryGraph>> {
        self.library_graphs.iter().find(|g| g.name() == name)
    }

    /// Keeps graph file `file` in your library, replacing one with the
    /// same name; says whether it replaced one, or why it can't.
    pub(crate) fn keep_graph(&mut self, file: GraphFile) -> Result<bool, &'static str> {
        let name = file.graph.name().to_string();
        let kept = LibraryGraph::new(file);
        match self.library_graphs.iter().position(|g| g.name() == name) {
            Some(at) => {
                self.library_graphs[at] = kept;
                Ok(true)
            }
            None if self.library_graphs.len() >= MAX_LIBRARY_GRAPHS => {
                Err(tr("Your library is full; remove a graph from it first."))
            }
            None => {
                self.library_graphs.push(kept);
                Ok(false)
            }
        }
    }

    /// Keeps own box file `file` in your library, replacing one with the
    /// same id; says whether it replaced one, or why it can't.
    pub(crate) fn keep_box(&mut self, file: BoxFile) -> Result<bool, &'static str> {
        match self.library.iter().position(|f| f.main.id == file.main.id) {
            Some(at) => {
                self.library[at] = file;
                Ok(true)
            }
            None if self.library.len() >= matgraph::own::MAX_OWN_BOXES => {
                Err(tr("Your library is full; remove a box from it first."))
            }
            None => {
                self.library.push(file);
                Ok(false)
            }
        }
    }

    /// The graph or own box `id` as a file to keep or share; `None` for a
    /// starter, or one that is gone.
    fn library_file(&self, document: &Document, id: &ItemId) -> Option<Result<GraphFile, BoxFile>> {
        let store = document.scene().graphs();
        match id {
            ItemId::ProjectGraph(g) => {
                GraphFile::of(store.get(*g)?, |i| store.own(i).cloned()).map(Ok)
            }
            ItemId::ProjectBox(own) => self.box_file_of(document, own).map(Err),
            ItemId::LibraryGraph(name) => Some(Ok(self.library_graph(name)?.file.clone())),
            ItemId::LibraryBox(own) => self
                .library
                .iter()
                .find(|f| f.main.id == *own)
                .cloned()
                .map(Err),
            ItemId::Starter(_) => None,
        }
    }

    /// The chosen graphs and own boxes as a pack, with a name for its
    /// file: the one card's name, or a plain one for several.
    pub(crate) fn chosen_pack(&self, document: &Document) -> Option<(Pack, String)> {
        let mut pack = Pack::default();
        let mut names = Vec::new();
        for id in &self.browser.chosen {
            match self.library_file(document, id) {
                Some(Ok(graph)) => {
                    names.push(graph.graph.name().to_string());
                    pack.graphs.push(graph);
                }
                Some(Err(own)) => {
                    names.push(own.main.name.clone());
                    pack.boxes.push(own);
                }
                None => {}
            }
        }
        let name = match names.as_slice() {
            [] => return None,
            [one] => one.clone(),
            _ => tr("Material pack").to_string(),
        };
        Some((pack, name))
    }

    /// Carries out what was asked in the window.
    fn carry_out_library_ask(
        &mut self,
        document: &mut Document,
        ask: Asked,
        items: &[Item],
        note: &mut Option<String>,
    ) {
        let browser = &mut self.browser;
        match ask {
            Asked::Click(id, modifiers) => {
                if modifiers.shift {
                    browser.chosen.remove(&id);
                } else if modifiers.command {
                    browser.chosen.insert(id.clone());
                    browser.focus = Some(id);
                } else {
                    browser.chosen = [id.clone()].into();
                    browser.focus = Some(id);
                }
                browser.new_tag.clear();
            }
            Asked::Open(id) => self.open_item(document, &id, note),
            Asked::Keep(id) => {
                let name = items.iter().find(|i| i.id == id).map(|i| i.name.clone());
                let kept = match self.library_file(document, &id) {
                    Some(Ok(graph)) => self.keep_graph(graph),
                    Some(Err(own)) => self.keep_box(own),
                    None => return,
                };
                *note = Some(match kept {
                    Ok(replaced) => {
                        let name = name.unwrap_or_default();
                        if replaced {
                            crate::trf!(
                                "Kept {} in your library, in place of the one there by that name.",
                                name
                            )
                        } else {
                            crate::trf!(
                                "Kept {} in your library: it is there in every project.",
                                name
                            )
                        }
                    }
                    Err(why) => why.to_string(),
                });
            }
            Asked::Remove(id) => self.remove_item(document, &id, note),
            Asked::SetTags(id, tags) => self.set_item_tags(document, &id, tags, note),
            Asked::ExportPack => {
                if let Some((pack, name)) = self.chosen_pack(document) {
                    self.library_file_asked = Some(LibraryAsk::SavePack(pack, name));
                }
            }
            Asked::BringIn => self.library_file_asked = Some(LibraryAsk::BringIn),
        }
    }

    /// Opens card `id`: shows the project's graph, opens a copy of
    /// another, or places an own box on the graph shown.
    fn open_item(&mut self, document: &mut Document, id: &ItemId, note: &mut Option<String>) {
        match id {
            ItemId::ProjectGraph(g) => {
                self.show_graph(Some(*g));
                self.browser.open = false;
            }
            ItemId::LibraryGraph(name) => {
                let Some(lib) = self.library_graph(name).cloned() else {
                    return;
                };
                if let Some(name) = self.open_graph_file(document, &lib.file, name, false, note) {
                    self.browser.open = false;
                    *note = Some(crate::trf!(
                        "Opened a copy of {} from your library: it is this project's now, to \
                         change as you like.",
                        name
                    ));
                }
            }
            ItemId::Starter(key) => {
                if let Some(starter) = STARTERS.iter().find(|s| s.key == *key) {
                    self.open_starter(document, starter, note);
                    if self.graph.is_some() {
                        self.browser.open = false;
                    }
                }
            }
            ItemId::ProjectBox(_) | ItemId::LibraryBox(_) => {
                // Placing says what to do first when no graph is shown.
                if let Some(Err(file)) = self.library_file(document, id) {
                    self.place_own(document, &file, None, note);
                }
            }
        }
    }

    /// Deletes card `id` from the project (an undo step), or removes it
    /// from your library.
    fn remove_item(&mut self, document: &mut Document, id: &ItemId, note: &mut Option<String>) {
        let done = match id {
            ItemId::ProjectGraph(g) => {
                let name = document
                    .scene()
                    .graphs()
                    .get(*g)
                    .map(|g| g.name().to_string())
                    .unwrap_or_default();
                document
                    .execute(Command::SetGraph {
                        id: *g,
                        graph: None,
                    })
                    .map(|()| crate::trf!("Deleted the graph {}. Undo brings it back.", name))
            }
            ItemId::ProjectBox(own) => document
                .execute(Command::SetOwnBox {
                    id: own.clone(),
                    own: None,
                })
                .map(|()| tr("Deleted it from the project. Undo brings it back.").to_string()),
            ItemId::LibraryGraph(name) => {
                self.library_graphs.retain(|g| g.name() != name);
                Ok(crate::trf!("Took {} out of your library.", name))
            }
            ItemId::LibraryBox(own) => {
                let name = self
                    .library
                    .iter()
                    .find(|f| f.main.id == *own)
                    .map(|f| f.main.name.clone())
                    .unwrap_or_default();
                self.library.retain(|f| f.main.id != *own);
                Ok(crate::trf!("Took {} out of your library.", name))
            }
            ItemId::Starter(_) => return,
        };
        *note = Some(match done {
            Ok(words) => {
                self.browser.chosen.remove(id);
                words
            }
            Err(why) => why.to_string(),
        });
    }

    /// Gives card `id` the tags `tags` (tidied): an undo step for the
    /// project's graphs and own boxes, at once for your library's.
    pub(crate) fn set_item_tags(
        &mut self,
        document: &mut Document,
        id: &ItemId,
        tags: Vec<String>,
        note: &mut Option<String>,
    ) {
        let tags = matgraph::tags::tidy_tags(tags.iter().map(String::as_str));
        let store = document.scene().graphs();
        let done = match id {
            ItemId::ProjectGraph(g) => document
                .execute(Command::EditGraph {
                    id: *g,
                    edit: GraphEdit::SetTags(tags),
                })
                .map_err(|why| why.to_string()),
            ItemId::ProjectBox(own) => {
                let Some(own) = store.own(own) else {
                    return;
                };
                let mut changed: OwnBox = (**own).clone();
                changed.tags = tags;
                document
                    .execute_labelled(
                        Command::SetOwnBox {
                            id: changed.id.clone(),
                            own: Some(changed),
                        },
                        tr("Change your own box's tags"),
                        None,
                    )
                    .map_err(|why| why.to_string())
            }
            ItemId::LibraryGraph(name) => {
                let Some(at) = self.library_graphs.iter().position(|g| g.name() == name) else {
                    return;
                };
                let mut file = self.library_graphs[at].file.clone();
                let registry = self.library_graphs[at].registry.clone();
                match file.graph.apply(&registry, &GraphEdit::SetTags(tags)) {
                    Ok(()) => {
                        self.library_graphs[at] = LibraryGraph::new(file);
                        Ok(())
                    }
                    Err(why) => Err(why.reason),
                }
            }
            ItemId::LibraryBox(own) => {
                let Some(file) = self.library.iter_mut().find(|f| f.main.id == *own) else {
                    return;
                };
                let mut main: OwnBox = (*file.main).clone();
                main.tags = tags;
                let main = Arc::new(main);
                for b in &mut file.boxes {
                    if b.id == main.id {
                        *b = main.clone();
                    }
                }
                file.main = main;
                Ok(())
            }
            ItemId::Starter(_) => return,
        };
        if let Err(why) = done {
            *note = Some(why);
        }
    }
}

/// An own box's card words.
fn own_item(id: ItemId, own: &OwnBox) -> Item {
    Item {
        id,
        name: own.name.clone(),
        about: own.about.clone(),
        tags: own.tags.clone(),
    }
}

/// One card: its picture, name and tags, chosen ones outlined in the
/// accent colour.
fn library_card(
    ui: &mut Ui,
    item: &Item,
    chosen: bool,
    picture: Option<egui::TextureId>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(CARD, Sense::click());
    let label = item.label();
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            chosen,
            label.clone(),
        )
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter_at(rect);
    let accent = theme::accent(ui.ctx()).colors();
    if response.hovered() {
        painter.rect_filled(rect, 6.0, theme::TEXT_WEAK.gamma_multiply(0.12));
    }
    let frame = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 4.0 + PICTURE / 2.0),
        vec2(PICTURE, PICTURE),
    );
    painter.rect_filled(frame, 4.0, ui.visuals().extreme_bg_color);
    if item.id.is_graph() {
        super::starters::draw_card_picture(&painter.with_clip_rect(frame), frame, picture);
    } else {
        match picture {
            Some(texture) => {
                painter.image(
                    texture,
                    frame,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => crate::shelf_cards::graph_mark(&painter, frame),
        }
        // Own boxes are marked apart from graphs: a small box in a corner.
        let corner = egui::Rect::from_min_size(frame.left_top() + vec2(4.0, 4.0), vec2(12.0, 9.0));
        painter.rect_filled(corner, 2.0, theme::PANEL);
        painter.rect_stroke(
            corner,
            2.0,
            egui::Stroke::new(1.0, theme::TEXT_STRONG),
            egui::StrokeKind::Inside,
        );
    }
    let edge = if chosen {
        egui::Stroke::new(2.0, accent.light)
    } else {
        egui::Stroke::new(1.0, theme::BORDER)
    };
    painter.rect_stroke(frame, 4.0, edge, egui::StrokeKind::Outside);
    let name = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, frame.bottom() + 4.0),
        egui::pos2(rect.right() - 2.0, frame.bottom() + 20.0),
    );
    let fitted = |text: &str, font: egui::FontId, colour: egui::Color32| {
        let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, colour);
        job.wrap = egui::text::TextWrapping::truncate_at_width(name.width());
        painter.layout_job(job)
    };
    let galley = fitted(
        &item.name,
        egui::FontId::proportional(12.0),
        if chosen {
            theme::TEXT_STRONG
        } else {
            theme::TEXT
        },
    );
    painter.galley(
        egui::pos2(name.center().x - galley.size().x / 2.0, name.top()),
        galley,
        theme::TEXT,
    );
    if !item.tags.is_empty() {
        let galley = fitted(
            &item.tags.join(", "),
            egui::FontId::proportional(10.0),
            theme::TEXT_WEAK,
        );
        painter.galley(
            egui::pos2(name.center().x - galley.size().x / 2.0, name.bottom() + 1.0),
            galley,
            theme::TEXT_WEAK,
        );
    }
    response
}
