//! The window's layouts: the standard one and the UV Edit one, the tabs
//! that switch between them on the right of the menu bar (beside the
//! project's name), and keeping the docked panels fitted as the window
//! changes size.

use super::Workbench;
use crate::layout::{default_layout, fit_coordinates};
use crate::palette::Mode;
use crate::panel::Panel;
use crate::text::tr;
use crate::theme;
use crate::transforming::Tool;
use egui::Ui;
use egui_dock::DockState;

/// The layouts the tabs on the right of the menu bar switch between
/// (decision 0096): each keeps its own arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutChoice {
    /// The standard arrangement.
    #[default]
    Standard,
    /// The viewport and the UV Editor side by side, in the UV mode.
    UvEdit,
}

/// The layout tabs, left to right: a layout's name, the layout (or, for
/// one not built yet, when it arrives) and what it is for.
fn layouts() -> [(
    &'static str,
    Result<LayoutChoice, &'static str>,
    &'static str,
); 2] {
    [
        (
            tr("Standard"),
            Ok(LayoutChoice::Standard),
            tr("The standard layout"),
        ),
        (
            tr("UV Edit"),
            Ok(LayoutChoice::UvEdit),
            tr("The viewport and the UV Editor side by side, in the UV mode"),
        ),
    ]
}

impl Workbench {
    /// The standard panel layout, for saving between sessions.
    pub fn layout(&self) -> &DockState<Panel> {
        match self.layout_choice {
            LayoutChoice::Standard => &self.dock,
            LayoutChoice::UvEdit => &self.other_dock,
        }
    }

    /// The UV Edit layout, for saving between sessions (decision 0096).
    pub fn uv_layout(&self) -> &DockState<Panel> {
        match self.layout_choice {
            LayoutChoice::Standard => &self.other_dock,
            LayoutChoice::UvEdit => &self.dock,
        }
    }

    /// Gives the workbench the UV Edit layout saved last session, if any.
    pub fn with_uv_layout(mut self, saved: Option<DockState<Panel>>) -> Self {
        let restored = crate::layout::restore_uv_layout(saved);
        match self.layout_choice {
            LayoutChoice::Standard => self.other_dock = restored,
            LayoutChoice::UvEdit => self.dock = restored,
        }
        self
    }

    /// The layout in use.
    pub fn layout_choice(&self) -> LayoutChoice {
        self.layout_choice
    }

    /// Switches to `choice`, each layout keeping its own arrangement. The
    /// UV Edit layout works in the UV mode (unless editing points, edges
    /// or polygons); leaving it leaves the UV mode.
    pub fn switch_layout(&mut self, choice: LayoutChoice) {
        if choice == self.layout_choice {
            return;
        }
        std::mem::swap(&mut self.dock, &mut self.other_dock);
        self.layout_choice = choice;
        self.fit_pending = true;
        match choice {
            LayoutChoice::UvEdit if matches!(self.mode, Mode::Model | Mode::Object) => {
                if self.tool.has_settings() {
                    self.set_tool(Tool::Select);
                }
                self.mode = Mode::Uv;
            }
            LayoutChoice::Standard if self.mode == Mode::Uv => self.mode = Mode::Model,
            _ => {}
        }
    }

    /// Shows the UV Editor in this layout, or puts it away (Window menu).
    pub fn toggle_uv_editor(&mut self) {
        let shown = crate::layout::toggle_uv_editor(&mut self.dock);
        self.note = Some(if shown {
            tr("UV Editor shown beside the viewport. Window → UV Editor puts it away.").into()
        } else {
            tr("UV Editor put away. Window → UV Editor brings it back.").into()
        });
    }

    /// Puts every panel back in its standard place (View → Reset panel layout).
    pub fn reset_layout(&mut self) {
        self.dock = match self.layout_choice {
            LayoutChoice::Standard => default_layout(),
            LayoutChoice::UvEdit => crate::layout::uv_edit_layout(),
        };
        self.fit_pending = true;
        self.note = Some(tr("Panel layout reset to the standard arrangement.").into());
    }

    /// Gives Coordinates exactly the height it needs (decision 0126's slim
    /// band): on a new layout's first frame; again whenever the window's
    /// size changes, until the user drags the band to a size of their own;
    /// and once at start for a saved band far taller than needed.
    pub(super) fn keep_coordinates_fitted(&mut self, ctx: &egui::Context) {
        let now = crate::layout::coordinates_height(&mut self.dock);
        let fitted = (self.coordinates_fitted)
            && now.is_some_and(|h| (h - crate::layout::COORDINATES_HEIGHT).abs() > 0.5);
        let too_tall = self.coordinates_checked.is_none()
            && now.is_some_and(|h| h > crate::layout::COORDINATES_HEIGHT * 1.6);
        if now.is_some() && self.coordinates_checked.is_none() {
            self.coordinates_checked = now;
        }
        // Still the fitted size the user hasn't dragged away from?
        let dragged = self.coordinates_fitted
            && self
                .coordinates_last
                .zip(now)
                .is_some_and(|(last, h)| (last - h).abs() > 0.5 && !self.window_resized(ctx));
        if dragged {
            self.coordinates_fitted = false;
        }
        if (self.fit_pending || too_tall || (fitted && !dragged)) && fit_coordinates(&mut self.dock)
        {
            self.fit_pending = false;
            self.coordinates_fitted = true;
            ctx.request_repaint();
        }
        self.coordinates_last = crate::layout::coordinates_height(&mut self.dock);
        self.last_screen = Some(ctx.content_rect().size());
    }

    /// Whether the window's size changed since last frame.
    fn window_resized(&self, ctx: &egui::Context) -> bool {
        self.last_screen
            .is_some_and(|size| size != ctx.content_rect().size())
    }
}

/// The layout tabs on the right of the menu bar. Only the standard layout
/// exists so far; the others are shown greyed out with when they arrive.
pub(super) fn layout_tabs(
    ui: &mut Ui,
    project: &str,
    unsaved: bool,
    current: LayoutChoice,
) -> Option<LayoutChoice> {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.add_space(6.0);
        let all = layouts();
        let tabs = all.iter().rev().collect::<Vec<_>>();
        let picked = layout_tab_labels(ui, &tabs, current);
        ui.add_space(12.0);
        project_badge(ui, project, unsaved);
        picked
    })
    .inner
}

/// The project's name in the menu bar, with a badge in the accent colour
/// while there are unsaved changes: easier to see than a mark in the
/// window's title, which the system draws.
fn project_badge(ui: &mut Ui, project: &str, unsaved: bool) {
    ui.spacing_mut().item_spacing.x = 6.0;
    if unsaved {
        let accent = theme::accent(ui.ctx()).colors();
        let text = egui::RichText::new(tr("Unsaved"))
            .small()
            .strong()
            .color(theme::TEXT_STRONG);
        let galley = egui::WidgetText::from(text).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        );
        let size = galley.size() + egui::vec2(12.0, 4.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().rect_filled(rect, size.y / 2.0, accent.fill);
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            theme::TEXT_STRONG,
        );
        response
            .on_hover_text(crate::trf!(
                "Changes since the last save. {} saves.",
                ui.ctx().format_shortcut(&crate::SAVE_SHORTCUT)
            ))
            .widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Label, true, tr("Unsaved changes"))
            });
    }
    ui.add(
        egui::Label::new(egui::RichText::new(project).color(theme::TEXT_WEAK)).selectable(false),
    )
    .on_hover_text(tr("The project open now"));
}

/// The layout tabs themselves, right to left. Returns the one clicked.
fn layout_tab_labels(
    ui: &mut Ui,
    tabs: &[&(&str, Result<LayoutChoice, &str>, &str)],
    current: LayoutChoice,
) -> Option<LayoutChoice> {
    let mut picked = None;
    for (name, layout, help) in tabs.iter().copied() {
        match layout {
            Ok(choice) if *choice == current => {
                let accent = theme::accent(ui.ctx()).colors();
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(*name).color(theme::TEXT_STRONG))
                        .selectable(false)
                        .sense(egui::Sense::click()),
                );
                let rect = response.rect;
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom() + 3.0,
                    egui::Stroke::new(2.0, accent.light),
                );
                response
                    .on_hover_text(crate::trf!("{} (in use)", help))
                    .widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::Button, true, true, *name)
                    });
            }
            Ok(choice) => {
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(*name).color(theme::TEXT_WEAK))
                        .selectable(false)
                        .sense(egui::Sense::click()),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::Button, true, false, *name)
                });
                if response.on_hover_text(*help).clicked() {
                    picked = Some(*choice);
                }
            }
            Err(when) => {
                ui.add_enabled(false, egui::Label::new(*name).selectable(false))
                    .on_disabled_hover_text(*when);
            }
        }
    }
    picked
}
