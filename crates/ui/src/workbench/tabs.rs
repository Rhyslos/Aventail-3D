//! Drawing each docked panel's contents (the viewport, Attributes,
//! Objects, Coordinates, Edit layers, the UV Editor and UV settings) as
//! egui_dock asks for them, with each panel's menu line.

use super::uv_menu::UV_EDITOR_UV_MENU;
use super::{automation, unwrapping};
use crate::attributes::{AttributesPanel, Shared};
use crate::coordinates::{CoordinatesView, Interaction};
use crate::drawing::SceneSummary;
use crate::objects::{self, DocRequest, ObjectManager};
use crate::palette::Mode;
use crate::panel::Panel;
use crate::panels::menu_line_with;
use crate::text::tr;
use crate::theme;
use crate::transforming::Tool;
use crate::units::Units;
use crate::viewport::{ViewportPanel, ViewportRenderer};
use egui::{Id, Ui, WidgetText};
use egui_dock::TabViewer;
use scene::Document;

/// Draws each panel's contents.
pub(super) struct PanelViewer<'a> {
    pub(super) viewport: &'a mut ViewportPanel,
    pub(super) renderer: &'a mut dyn ViewportRenderer,
    pub(super) summary: &'a SceneSummary,
    pub(super) document: &'a mut Document,
    pub(super) objects: &'a mut ObjectManager,
    pub(super) layers: &'a mut crate::layers::LayersPanel,
    /// What was picked on the modifiers palette this frame.
    pub(super) modifier_pick: &'a mut Option<crate::layers::ModifierPick>,
    pub(super) uv_editor: &'a mut crate::uv_editor::UvEditor,
    /// The Paint layers panel (decision 0181).
    pub(super) paint_layers: &'a mut crate::paint_layers::PaintLayersPanel,
    /// The brush bar (decision 0185).
    pub(super) brush_bar: &'a mut crate::brush_bar::BrushBar,
    /// The person's own brushes.
    pub(super) saved_brushes: &'a mut Vec<crate::brush_bar::BrushPreset>,
    /// The shelf's Colours tab, which tab shows, and the colours used
    /// lately (decision 0213).
    pub(super) colour_shelf: &'a mut crate::colour_shelf::ColourShelf,
    pub(super) shelf_tab: &'a mut crate::colour_shelf::ShelfTab,
    /// The projection brush's picture, for the Stencils tab.
    pub(super) stencil_picture: Option<egui::TextureId>,
    pub(super) used_colours: &'a [[u8; 3]],
    /// Where the viewport and the UV Editor were drawn this frame: the
    /// painting area (decision 0215).
    pub(super) paint_panes: &'a mut crate::paint_area::PaintPanes,
    /// The status bar's note, for what the panels couldn't do.
    pub(super) note: &'a mut Option<String>,
    pub(super) mode: Mode,
    pub(super) navigation: tools::NavigationSettings,
    pub(super) select_keys: crate::selecting::SelectKeys,
    pub(super) tool: Tool,
    pub(super) snap: crate::transforming::SnapSettings,
    pub(super) surface: crate::viewport::Surface,
    /// Whether the selection is drawn with high contrast (decision 0126).
    pub(super) high_contrast: bool,
    /// How pieces are grouped onto textures (the packing settings).
    pub(super) texture_groups: crate::uv_editor::checking::TextureGroups,
    /// What explaining choices in the UV Editor needs.
    pub(super) explain: crate::uv_editor::explain::ExplainContext,
    pub(super) viewport_hover: Option<(scene::ObjectId, scene::Element)>,
    pub(super) attributes: &'a mut AttributesPanel,
    pub(super) units: &'a mut Units,
    pub(super) requests: &'a mut Vec<DocRequest>,
    /// The UV Editor's UV menu button, if drawn (its menu is filled by the
    /// workbench).
    pub(super) uv_menu_button: &'a mut Option<egui::Response>,
    /// The automatic UV tools' settings, for the UV settings panel.
    pub(super) uv_settings: &'a mut unwrapping::UvSettings,
    /// What the Coordinates panel shows, and what was done in it.
    pub(super) coordinates: CoordinatesView,
    pub(super) coordinates_done: &'a mut Interaction,
    /// The view panel that carries the modifiers palette, if it is shown
    /// (decision 0176).
    pub(super) palette_host: Option<Panel>,
}

impl TabViewer for PanelViewer<'_> {
    type Tab = Panel;

    fn id(&mut self, tab: &mut Panel) -> Id {
        Id::new(("panel", *tab))
    }

    fn title(&mut self, tab: &mut Panel) -> WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Panel) {
        if *tab == Panel::UvEditor && self.palette_host == Some(Panel::UvEditor) {
            // The modifiers palette on the UV Editor's right, beside
            // Attributes (decision 0176), and the editor in the rest.
            let full = ui.max_rect();
            let rest = self.modifiers_palette(ui, full);
            let mut rest_ui =
                ui.new_child(egui::UiBuilder::new().max_rect(rest).layout(*ui.layout()));
            self.uv_editor_tab(&mut rest_ui);
            ui.advance_cursor_after_rect(full);
            return;
        }
        if *tab == Panel::Viewport {
            // Edge to edge, with no padding, and the modifiers palette on
            // its right when it is the view beside Attributes (decisions
            // 0091 and 0176).
            let full = ui.max_rect();
            let view = if self.palette_host == Some(Panel::Viewport) {
                self.modifiers_palette(ui, full)
            } else {
                full
            };
            let mut view_ui = ui.new_child(egui::UiBuilder::new().max_rect(view));
            self.viewport
                .show(&mut view_ui, self.renderer, self.summary);
            // Where the viewport is, for the brush bar and the paint tools
            // the workbench draws over the painting area (decisions 0213
            // and 0215).
            let (shading, navigation) = match self.viewport.bottom_buttons() {
                Some((a, b)) => (Some(a), Some(b)),
                None => (None, None),
            };
            self.paint_panes.viewport = Some(crate::paint_area::Pane {
                rect: view,
                // The Paint page's view has no menu line or name (decision
                // 0224): the bar sits at its top.
                bar_top: if self.viewport.painting() {
                    view.top() + crate::paint_bar::TOP_GAP
                } else {
                    view.top() + crate::viewport::HEADER_HEIGHT + crate::paint_bar::NAME_ROOM
                },
                bottom: [shading, navigation],
            });
            // A click in the viewport goes back to the objects' own
            // settings from a modifier's (decision 0168).
            let clicked_in_view = ui.input(|i| {
                i.pointer.primary_clicked()
                    && i.pointer.interact_pos().is_some_and(|p| view.contains(p))
            });
            if clicked_in_view {
                self.attributes.picked_modifier = None;
            }
            ui.advance_cursor_after_rect(full);
            return;
        }
        match tab {
            Panel::Attributes => menu_line_with(ui, |ui| {
                let mut shared = Shared {
                    clipboard: &mut self.objects.clipboard,
                    units: self.units,
                };
                self.attributes
                    .menus(ui, self.document, &mut shared, self.requests);
            }),
            Panel::Objects => menu_line_with(ui, |ui| {
                objects::menus(ui, self.document, self.objects, self.requests);
            }),
            Panel::EditLayers => menu_line_with(ui, |ui| {
                self.layers.menus(ui, self.document, self.requests);
            }),
            Panel::UvEditor => {
                self.uv_editor_tab(ui);
                return;
            }
            Panel::Viewport
            | Panel::Coordinates
            | Panel::UvSettings
            | Panel::Levels
            | Panel::PaintLayers
            | Panel::Brushes
            | Panel::Paint => {}
        }
        egui::Frame::new()
            .inner_margin(theme::panel_margin())
            .show(ui, |ui| match tab {
                Panel::Viewport => {}
                Panel::Attributes => {
                    self.attributes
                        .show(ui, self.document, *self.units, self.requests);
                }
                Panel::Objects => self.objects.show(
                    ui,
                    self.document,
                    &mut self.attributes.picked_modifier,
                    self.requests,
                ),
                Panel::EditLayers => {
                    self.layers.show(
                        ui,
                        self.document,
                        &mut self.attributes.opened,
                        self.requests,
                    );
                }
                Panel::UvEditor => {}
                Panel::PaintLayers => self.paint_layers.show(ui, self.document, self.note),
                Panel::Paint => self.attributes.paint_panel(ui, self.document),
                Panel::Brushes => {
                    use crate::colour_shelf::ShelfTab;
                    match *self.shelf_tab {
                        ShelfTab::Brushes => {}
                        ShelfTab::Colours => {
                            self.colours_tab(ui);
                            return;
                        }
                        ShelfTab::Materials | ShelfTab::SmartMasks | ShelfTab::Stencils => {
                            self.shelf_cards_tab(ui);
                            return;
                        }
                    }
                    let tab = &mut *self.shelf_tab;
                    let change = self.brush_bar.show(
                        ui,
                        &mut self.attributes.brush,
                        self.saved_brushes,
                        |ui| crate::colour_shelf::tabs(ui, tab),
                    );
                    match change {
                        Some(crate::brush_bar::SavedChange::Add(preset)) => {
                            *self.note = Some(crate::trf!("Kept the brush {}.", preset.name));
                            self.saved_brushes.push(preset);
                        }
                        Some(crate::brush_bar::SavedChange::Remove(i))
                            if i < self.saved_brushes.len() =>
                        {
                            self.saved_brushes.remove(i);
                        }
                        Some(crate::brush_bar::SavedChange::SaveFile) => {
                            self.paint_layers.preset_asked =
                                Some(crate::paint_layers::presets::PresetAsk::Save(
                                    crate::paint_layers::presets::PresetKind::Brushes,
                                ));
                        }
                        Some(crate::brush_bar::SavedChange::LoadFile) => {
                            self.paint_layers.preset_asked =
                                Some(crate::paint_layers::presets::PresetAsk::Load);
                        }
                        Some(crate::brush_bar::SavedChange::Remove(_)) | None => {}
                    }
                }
                Panel::Levels => {
                    self.attributes
                        .levels_panel(ui, self.document, *self.units, self.requests);
                }
                Panel::UvSettings => {
                    let mut view = automation::UvSettingsView {
                        uv_settings: self.uv_settings,
                        document: self.document,
                    };
                    view.panel(ui);
                }
                Panel::Coordinates => {
                    *self.coordinates_done =
                        crate::coordinates::show(ui, *self.units, &self.coordinates);
                }
            });
    }

    fn is_closeable(&self, _tab: &Panel) -> bool {
        false
    }

    fn on_tab_button(&mut self, tab: &mut Panel, response: &egui::Response) {
        // Name the tab for screen readers (and UI tests); egui_dock paints
        // the title without labelling the button.
        let title = tab.title();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    }

    fn allowed_in_windows(&self, _tab: &mut Panel) -> bool {
        // Panels dock inside the main window only: a floating window could
        // be closed and take its panel with it.
        false
    }

    fn clear_background(&self, tab: &Panel) -> bool {
        // The viewport and the UV Editor paint their own backgrounds.
        !matches!(tab, Panel::Viewport | Panel::UvEditor)
    }

    fn scroll_bars(&self, tab: &Panel) -> [bool; 2] {
        // The viewport and the UV Editor fill their panels exactly.
        if matches!(tab, Panel::Viewport | Panel::UvEditor) {
            [false, false]
        } else {
            [true, true]
        }
    }
}

impl PanelViewer<'_> {
    /// The brush shelf's Colours tab (decision 0213): a click paints with
    /// a colour, a double-click opens the colour picker too.
    fn colours_tab(&mut self, ui: &mut Ui) {
        let attributes = &mut *self.attributes;
        let [r, g, b, _] = attributes.brush.brush.color;
        let mut colour = [r, g, b];
        let click = self.colour_shelf.show(
            ui,
            &mut colour,
            self.used_colours,
            self.document.scene().paint().palettes(),
            &mut attributes.palette_requests,
            |ui| crate::colour_shelf::tabs(ui, self.shelf_tab),
        );
        attributes.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        if click.open_picker {
            // The picker is on the colour square, which only tools that
            // paint a colour have.
            let brush = &mut attributes.brush;
            if brush.tool != crate::attributes::PaintTool::Fill
                && brush.brush.kind != paint::BrushKind::Paint
            {
                brush.tool = crate::attributes::PaintTool::Brush;
                brush.brush.kind = paint::BrushKind::Paint;
            }
            attributes.open_colour_picker = true;
        }
    }

    /// The shelf's Materials, Smart masks and Stencils tabs (decision
    /// 0229).
    fn shelf_cards_tab(&mut self, ui: &mut Ui) {
        use crate::colour_shelf::ShelfTab;
        use crate::shelf_cards::{self, StencilClick};
        let tab = &mut *self.shelf_tab;
        match *tab {
            ShelfTab::Materials => {
                if let Some(ask) =
                    shelf_cards::materials(ui, |ui| crate::colour_shelf::tabs(ui, tab))
                {
                    self.paint_layers
                        .apply_shelf(ui.ctx(), self.document, ask, self.note);
                }
            }
            ShelfTab::SmartMasks => {
                let realism = self.attributes.realism;
                if let Some(ask) =
                    shelf_cards::smart_masks(ui, realism, |ui| crate::colour_shelf::tabs(ui, tab))
                {
                    self.paint_layers
                        .apply_shelf(ui.ctx(), self.document, ask, self.note);
                }
            }
            ShelfTab::Stencils => {
                let attributes = &mut *self.attributes;
                let name = attributes.stencil_name.clone();
                let picture = name.as_deref().map(|n| (n, self.stencil_picture));
                let projecting = attributes.brush.tool == crate::attributes::PaintTool::Brush
                    && attributes.brush.brush.kind == paint::BrushKind::Projection;
                match shelf_cards::stencils(ui, picture, projecting, |ui| {
                    crate::colour_shelf::tabs(ui, tab)
                }) {
                    Some(StencilClick::Choose) => attributes.choose_stencil = true,
                    Some(StencilClick::Use) => {
                        attributes.brush.tool = crate::attributes::PaintTool::Brush;
                        attributes.brush.brush.kind = paint::BrushKind::Projection;
                    }
                    None => {}
                }
            }
            ShelfTab::Brushes | ShelfTab::Colours => {}
        }
    }

    /// Draws the modifiers palette down the right of `full` (decisions
    /// 0091 and 0176) and returns what is left for the panel.
    fn modifiers_palette(&mut self, ui: &mut Ui, full: egui::Rect) -> egui::Rect {
        let split = full.right() - crate::layers::MODIFIERS_WIDTH;
        let strip = egui::Rect::from_min_max(egui::pos2(split, full.top()), full.max);
        let mut strip_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("modifiers palette")
                .max_rect(strip)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        strip_ui.painter().rect_filled(strip, 0.0, theme::PALETTE);
        strip_ui.painter().vline(
            strip.left() + 0.5,
            strip.y_range(),
            egui::Stroke::new(1.0, theme::BORDER),
        );
        if let Some(pick) = crate::layers::modifiers_palette(&mut strip_ui, self.document) {
            *self.modifier_pick = Some(pick);
        }
        egui::Rect::from_min_max(full.min, egui::pos2(split, full.bottom()))
    }

    /// The UV Editor panel: its menu line, then the editor edge to edge.
    fn uv_editor_tab(&mut self, ui: &mut Ui) {
        // Plain while painting (decision 0225): no menu line (the UV
        // menu at the top has it all).
        let painting = self.mode == crate::palette::Mode::Paint;
        if !painting {
            self.uv_editor_menus(ui);
        }
        self.uv_editor_view(ui);
    }

    /// The UV Editor's menu line: UV, Texture, Trim, Snapshots, View.
    fn uv_editor_menus(&mut self, ui: &mut Ui) {
        menu_line_with(ui, |ui| {
            // The main UV menu, here too (issue #115).
            let uv = ui
                .add(egui::Button::new(tr(UV_EDITOR_UV_MENU)).frame_when_inactive(false))
                .on_hover_text(tr(
                    "Every UV command and setting, as in the UV menu at the top",
                ));
            *self.uv_menu_button = Some(uv);
            crate::dialog::menu(ui, tr("Texture"), |ui| {
                crate::uv_editor::texture_menu(ui, self.document);
            });
            crate::dialog::menu(ui, tr("Trim"), |ui| {
                crate::uv_editor::trim_menu(ui, self.document);
            });
            // Stays open while snapshots are taken and put back, so
            // what each did can be read in it.
            egui::containers::menu::MenuButton::new(tr(tr(
                crate::uv_editor::snapshots::SNAPSHOTS_MENU,
            )))
            .config(
                crate::dialog::menu_config()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
            )
            .ui(ui, |ui| {
                ui.set_min_width(crate::dialog::MENU_WIDTH);
                crate::uv_editor::snapshots::snapshots_menu(ui, self.document, self.uv_editor);
            });
            crate::dialog::menu(ui, tr("View"), |ui| {
                for what in [
                    crate::uv_editor::UvFrame::Square,
                    crate::uv_editor::UvFrame::Selected,
                ] {
                    if ui
                        .add(egui::Button::new(what.label()).shortcut_text(what.key().name()))
                        .clicked()
                    {
                        self.uv_editor.frame(what);
                        ui.close();
                    }
                }
                ui.separator();
                let mut numbers = self.uv_editor.piece_numbers();
                let mut explain = self.uv_editor.explaining();
                if ui
                    .checkbox(&mut explain, tr(crate::uv_editor::explain::EXPLAIN_CHOICES))
                    .on_hover_text(tr(
                        "Point at a seam or a piece to see why it's there and how to \
                         change it",
                    ))
                    .changed()
                {
                    self.uv_editor.set_explaining(explain);
                }
                if ui
                    .checkbox(&mut numbers, tr(crate::uv_editor::checking::PIECE_NUMBERS))
                    .on_hover_text(tr(
                        "Write on each piece its size in texture pixels, its texel \
                         density and, if it's uneven, how stretched it is",
                    ))
                    .changed()
                {
                    self.uv_editor.set_piece_numbers(numbers);
                }
                let measuring = self.uv_editor.measuring();
                if ui
                    .add(egui::Button::selectable(
                        measuring,
                        tr(crate::uv_editor::checking::MEASURE),
                    ))
                    .on_hover_text(tr(
                        "Then drag in the editor: the line's length in texture pixels \
                         and on the surface, in centimetres. Esc stops.",
                    ))
                    .clicked()
                {
                    self.uv_editor.set_measuring(!measuring);
                    ui.close();
                }
            });
        });
    }

    /// The UV Editor itself, edge to edge like the viewport.
    fn uv_editor_view(&mut self, ui: &mut Ui) {
        let cx = crate::uv_editor::UvContext {
            mode: self.mode,
            navigation: &self.navigation,
            select_keys: self.select_keys,
            tool: self.tool,
            snap: self.snap,
            stretch: self.surface == crate::viewport::Surface::Stretch,
            outside_hover: self.viewport_hover,
            grid: (self.surface == crate::viewport::Surface::NumberedChecker)
                .then(theme::grid_colors),
            painted: (self.surface == crate::viewport::Surface::Painted)
                .then_some(self.summary.drawing.textures.as_slice()),
            texture_groups: self.texture_groups,
            explain: self.explain,
            high_contrast: self.high_contrast,
            channel: self.attributes.channel_shown,
        };
        self.uv_editor.show(ui, self.document, &cx);
        if let Some(rect) = self.uv_editor.drawn() {
            self.paint_panes.uv_editor = Some(crate::paint_area::Pane {
                rect,
                bar_top: rect.top() + 6.0,
                bottom: self.uv_editor.bottom_notes(),
            });
        }
    }
}
