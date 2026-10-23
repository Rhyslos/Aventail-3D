//! The parts every window is built from, so they all look and behave the
//! same ([decision 0087](../../../docs/decisions/0087-one-look-for-every-window.md)):
//! a title bar with the window's name, anything it chooses at the top
//! (Export's Made for) and a close button; a body, or a side list of
//! sections beside one; and a footer with a short note on the left and
//! the buttons on the right, the main one in the accent colour.
//!
//! Inside, settings are rows with their name on the left and the control on
//! the right, under small section headings. Two to four choices are a
//! [`segmented`] row; five or more, or choices with a word of explanation
//! each, a [`drop_down`].
//!
//! A window is drawn with [`Dialog::show`], whose closure gets a
//! [`DialogUi`] and calls its parts in order: [`DialogUi::header`], then
//! [`DialogUi::body`] or [`DialogUi::sections`], then
//! [`DialogUi::footer`].

use crate::text::tr;
use crate::theme;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Rect, Response,
    RichText, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};

/// The window's title.
pub(crate) fn title_font(ctx: &egui::Context) -> FontId {
    theme::semibold(ctx, 15.0)
}

/// A section heading inside a window.
pub(crate) fn heading_font(ctx: &egui::Context) -> FontId {
    theme::semibold(ctx, 12.5)
}

/// Help and summaries.
pub(crate) fn help_font() -> FontId {
    FontId::proportional(11.5)
}

/// How wide the names of settings are, so the controls line up.
pub(crate) const NAME_WIDTH: f32 = 128.0;

/// How wide a side list of sections is.
pub(crate) const SIDE_WIDTH: f32 = 190.0;

/// The space around a window's contents.
const PADDING: i8 = 18;

/// The text colour of something switched on: a quiet green.
pub(crate) const ON: Color32 = Color32::from_rgb(0x6f, 0xcf, 0x97);

/// A window built from the shared parts.
pub(crate) struct Dialog {
    id: Id,
    title: String,
    width: f32,
    body_height: Option<f32>,
    modal: bool,
    /// Where it sits in the app's window: the middle unless chosen.
    anchor: (Align2, [f32; 2]),
}

/// What happened to a window this frame.
pub(crate) struct DialogOutcome {
    /// Whether its close button was pressed.
    pub(crate) closed: bool,
}

impl Dialog {
    /// A window called `title`, `width` points wide, in the middle of the
    /// app's window.
    pub(crate) fn new(title: impl Into<String>, width: f32) -> Self {
        let title = title.into();
        Self {
            id: Id::new(("dialog", title.clone())),
            title,
            width,
            body_height: None,
            modal: false,
            anchor: (Align2::CENTER_CENTER, [0.0, 0.0]),
        }
    }

    /// Its id, when its title changes (About names the app).
    pub(crate) fn id(mut self, id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Id::new(("dialog", id));
        self
    }

    /// Gives the body (and the side list) this height: a window with
    /// sections keeps its size while switching between them. Taller
    /// contents scroll.
    pub(crate) fn body_height(mut self, height: f32) -> Self {
        self.body_height = Some(height);
        self
    }

    /// Places it at `align` of the app's window, `offset` points from
    /// there, instead of the middle: a window whose changes show live in
    /// the viewport sits out of the way (Align, decision 0093).
    pub(crate) fn anchor(mut self, align: Align2, offset: [f32; 2]) -> Self {
        self.anchor = (align, offset);
        self
    }

    /// A question: the rest of the app is dimmed and can't be used until
    /// it is answered.
    pub(crate) fn modal(mut self) -> Self {
        self.modal = true;
        self
    }

    /// Draws the window; `add` draws its parts.
    pub(crate) fn show(
        self,
        ctx: &egui::Context,
        add: impl FnOnce(&mut DialogUi<'_>),
    ) -> DialogOutcome {
        let frame = Frame::window(&ctx.global_style())
            .inner_margin(Margin::ZERO)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme::BORDER));
        let mut closed = false;
        let width = self.width;
        let title = self.title.clone();
        let body_height = self.body_height;
        let contents = |ui: &mut Ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut parts = DialogUi {
                ui,
                title: &title,
                body_height,
                closed: false,
            };
            add(&mut parts);
            closed = parts.closed;
        };
        if self.modal {
            egui::Modal::new(self.id).frame(frame).show(ctx, contents);
        } else {
            egui::Window::new(&self.title)
                .id(self.id)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(frame)
                .anchor(self.anchor.0, self.anchor.1)
                .show(ctx, contents);
        }
        DialogOutcome { closed }
    }
}

/// The parts of one window, drawn in order.
pub(crate) struct DialogUi<'u> {
    ui: &'u mut Ui,
    title: &'u str,
    body_height: Option<f32>,
    closed: bool,
}

/// One entry of a side list.
pub(crate) struct Section<'a, S> {
    /// The section.
    pub(crate) id: S,
    /// Its name.
    pub(crate) name: &'a str,
    /// A word on its right: a count, "on" or "off".
    pub(crate) badge: Option<Badge>,
    /// Whether a line goes above it, to set it apart from those above.
    pub(crate) apart: bool,
    /// What screen readers (and tests) call it, if not its name.
    pub(crate) label: Option<String>,
}

/// A word on the right of a side list entry.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Badge {
    /// "on", in green.
    On,
    /// "off".
    Off,
}

impl<'u> DialogUi<'u> {
    /// The title bar: the window's name, `extra` beside it, and a close
    /// button on the right (without `closable`, none: a question has to
    /// be answered).
    pub(crate) fn header(&mut self, closable: bool, extra: impl FnOnce(&mut Ui)) {
        let title = self.title;
        let mut close = false;
        Frame::new()
            .fill(theme::HEADER)
            .inner_margin(Margin::symmetric(PADDING, 12))
            .corner_radius(CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            })
            .show(self.ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.horizontal(|ui| {
                    ui.set_min_height(28.0);
                    ui.label(
                        RichText::new(title)
                            .font(title_font(ui.ctx()))
                            .color(theme::TEXT_STRONG),
                    );
                    ui.add_space(10.0);
                    extra(ui);
                    if closable {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            close = close_button(ui).clicked();
                        });
                    }
                });
            });
        line(self.ui);
        if close {
            self.closed = true;
        }
    }

    /// The body, without a side list.
    pub(crate) fn body<R>(&mut self, add: impl FnOnce(&mut Ui) -> R) -> R {
        let height = self.body_height;
        Frame::new()
            .inner_margin(Margin::symmetric(PADDING, 14))
            .show(self.ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 6.0;
                match height {
                    Some(height) => {
                        egui::ScrollArea::vertical()
                            .max_height(height)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.set_min_height(height);
                                add(ui)
                            })
                            .inner
                    }
                    None => add(ui),
                }
            })
            .inner
    }

    /// A side list of `sections` with `chosen` marked (a click chooses
    /// another), `note` at its foot, and the chosen section's settings
    /// beside it, drawn by `add`.
    pub(crate) fn sections<S: PartialEq + Copy, R>(
        &mut self,
        chosen: &mut S,
        sections: &[Section<'_, S>],
        note: Option<&str>,
        add: impl FnOnce(&mut Ui, S) -> R,
    ) -> R {
        let height = self.body_height.unwrap_or(360.0);
        let ui = &mut *self.ui;
        let top = ui.cursor().min;
        let full = ui.available_width();
        // The side list.
        let side = Rect::from_min_size(top, vec2(SIDE_WIDTH, height + 28.0));
        let mut side_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(side.shrink2(vec2(8.0, 10.0)))
                .layout(Layout::top_down_justified(Align::Min)),
        );
        side_ui.spacing_mut().item_spacing.y = 2.0;
        for section in sections {
            if section.apart {
                side_ui.add_space(6.0);
                let y = side_ui.cursor().min.y;
                side_ui.painter().hline(
                    side_ui.max_rect().x_range(),
                    y,
                    Stroke::new(1.0, theme::BORDER),
                );
                side_ui.add_space(8.0);
            }
            if side_entry(&mut side_ui, section, *chosen == section.id).clicked() {
                *chosen = section.id;
            }
        }
        if let Some(note) = note {
            side_ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(note)
                            .font(help_font())
                            .color(theme::TEXT_WEAK),
                    )
                    .wrap(),
                );
            });
        }
        ui.painter().vline(
            side.right(),
            side.y_range(),
            Stroke::new(1.0, theme::BORDER),
        );
        // The chosen section.
        let body = Rect::from_min_max(
            pos2(side.right() + 1.0, top.y),
            pos2(top.x + full, side.bottom()),
        );
        let mut body_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body.shrink2(vec2(PADDING as f32, 14.0)))
                .layout(Layout::top_down(Align::Min)),
        );
        body_ui.spacing_mut().item_spacing.y = 6.0;
        let chosen = *chosen;
        let inner = egui::ScrollArea::vertical()
            .id_salt("dialog section")
            .max_height(height)
            .auto_shrink([false, false])
            .show(&mut body_ui, |ui| {
                // Room for the scroll bar on the right.
                Frame::new()
                    .inner_margin(Margin {
                        right: 12,
                        ..Margin::ZERO
                    })
                    .show(ui, |ui| add(ui, chosen))
                    .inner
            })
            .inner;
        ui.allocate_rect(
            Rect::from_min_size(top, vec2(full, side.height())),
            Sense::hover(),
        );
        inner
    }

    /// The footer: `note` on the left, quiet, and the buttons `add` puts
    /// in from the right (the main one first, see [`main_button`]).
    pub(crate) fn footer<R>(&mut self, note: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
        self.footer_with(
            |ui| {
                if let Some(note) = note {
                    ui.label(
                        RichText::new(note)
                            .font(help_font())
                            .color(theme::TEXT_WEAK),
                    );
                }
            },
            add,
        )
    }

    /// The footer, with `left` drawing its left side (a button such as
    /// Restore standard, or a note) and `add` the buttons from the right.
    pub(crate) fn footer_with<R>(
        &mut self,
        left: impl FnOnce(&mut Ui),
        add: impl FnOnce(&mut Ui) -> R,
    ) -> R {
        line(self.ui);
        Frame::new()
            .fill(theme::HEADER)
            .inner_margin(Margin::symmetric(PADDING, 12))
            .corner_radius(CornerRadius {
                nw: 0,
                ne: 0,
                sw: 8,
                se: 8,
            })
            .show(self.ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.set_min_height(28.0);
                    left(ui);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        add(ui)
                    })
                    .inner
                })
                .inner
            })
            .inner
    }
}

/// A line across the window between its parts.
fn line(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, theme::BORDER),
    );
}

/// The × in the title bar.
fn close_button(ui: &mut Ui) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr("Close window")));
    let color = if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    let c = rect.center();
    let r = 5.0;
    let stroke = Stroke::new(1.5, color);
    ui.painter()
        .line_segment([c + vec2(-r, -r), c + vec2(r, r)], stroke);
    ui.painter()
        .line_segment([c + vec2(-r, r), c + vec2(r, -r)], stroke);
    response.on_hover_text(tr("Close (Esc)"))
}

/// One entry of a side list.
fn side_entry<S>(ui: &mut Ui, section: &Section<'_, S>, chosen: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::SelectableLabel,
            true,
            chosen,
            section.label.as_deref().unwrap_or(section.name),
        )
    });
    let painter = ui.painter();
    if chosen {
        let accent = theme::accent(ui.ctx()).colors();
        painter.rect_filled(rect, 5.0, accent.soft);
    } else if response.hovered() {
        painter.rect_filled(rect, 5.0, theme::HOVER.gamma_multiply(0.6));
    }
    let text = if chosen || response.hovered() {
        theme::TEXT_STRONG
    } else {
        theme::TEXT
    };
    let font = if chosen {
        theme::semibold(ui.ctx(), 12.5)
    } else {
        FontId::proportional(12.5)
    };
    painter.text(
        rect.left_center() + vec2(10.0, 0.0),
        Align2::LEFT_CENTER,
        section.name,
        font,
        text,
    );
    if let Some(badge) = &section.badge {
        let (word, color) = match badge {
            Badge::On => (tr("on"), ON),
            Badge::Off => (tr("off"), theme::TEXT_WEAK),
        };
        painter.text(
            rect.right_center() - vec2(10.0, 0.0),
            Align2::RIGHT_CENTER,
            word,
            FontId::proportional(11.0),
            color,
        );
    }
    response
}

/// A section heading inside a window, with a little space above it.
pub(crate) fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(text)
            .font(heading_font(ui.ctx()))
            .color(theme::TEXT_STRONG),
    );
}

/// A section heading with a word beside it ("On", "Off").
pub(crate) fn heading_with_pill(ui: &mut Ui, text: &str, on: bool) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(text)
                .font(heading_font(ui.ctx()))
                .color(theme::TEXT_STRONG),
        );
        pill(ui, on);
    });
}

/// "On" in green or "Off" in grey, in a small rounded box.
pub(crate) fn pill(ui: &mut Ui, on: bool) -> Response {
    let (word, text, fill) = if on {
        (tr("On"), ON, Color32::from_rgb(0x1f, 0x3a, 0x2c))
    } else {
        (tr("Off"), theme::TEXT_WEAK, theme::FIELD)
    };
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(8, 1))
        .show(ui, |ui| {
            ui.label(
                RichText::new(word)
                    .font(FontId::proportional(11.0))
                    .color(text),
            )
        })
        .inner
}

/// Help or a summary under a heading or a setting: quiet, smaller text.
pub(crate) fn help(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.add(
        egui::Label::new(
            RichText::new(text.into())
                .font(help_font())
                .color(theme::TEXT_WEAK),
        )
        .wrap(),
    )
}

/// A line between groups of settings.
pub(crate) fn divider(ui: &mut Ui) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, Color32::from_rgb(0x30, 0x30, 0x30)),
    );
    ui.add_space(6.0);
}

/// A setting: `name` on the left, in a column as wide as every other
/// setting's name, and the control `add` draws on the right.
pub(crate) fn row<R>(ui: &mut Ui, name: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        ui.allocate_ui_with_layout(
            vec2(NAME_WIDTH, 28.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.set_min_width(NAME_WIDTH);
                ui.label(RichText::new(name).color(theme::TEXT_WEAK));
            },
        );
        // The control's own column, so long text wraps inside it.
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width());
            ui.horizontal_wrapped(|ui| add(ui)).inner
        })
        .inner
    })
    .inner
}

/// A row of two to four choices, side by side in one box; the chosen one
/// is lit. Gives back whether the choice changed. `choices` are each a
/// value, its name and what it means (shown when pointed at).
pub(crate) fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    choices: &[(T, &str, &str)],
) -> bool {
    let mut changed = false;
    Frame::new()
        .fill(theme::FIELD)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (choice, name, meaning) in choices {
                    let chosen = *value == *choice;
                    let response = segment(ui, name, chosen);
                    let response = if meaning.is_empty() {
                        response
                    } else {
                        response.on_hover_text(*meaning)
                    };
                    if response.clicked() && !chosen {
                        *value = *choice;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// One choice of a [`segmented`] row.
fn segment(ui: &mut Ui, name: &str, chosen: bool) -> Response {
    let font = FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_string(), font.clone(), theme::TEXT);
    let size = vec2(galley.size().x + 24.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, enabled, chosen, name));
    let painter = ui.painter();
    if chosen {
        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x3d, 0x3d, 0x3d));
    } else if response.hovered() && enabled {
        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x2c, 0x2c, 0x2c));
    }
    let color = if !enabled {
        theme::TEXT_WEAK.gamma_multiply(0.6)
    } else if chosen || response.hovered() {
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    painter.text(rect.center(), Align2::CENTER_CENTER, name, font, color);
    response
}

/// A drop-down list: a box `width` wide showing `chosen` (and a quiet
/// `hint` on its right, such as the file type); a click opens a list that
/// `add` fills with [`drop_down_entry`]s. `label` names it for screen
/// readers.
pub(crate) fn drop_down(
    ui: &mut Ui,
    label: &str,
    chosen: &str,
    hint: &str,
    width: f32,
    add: impl FnOnce(&mut Ui),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::ComboBox,
            ui.is_enabled(),
            crate::trf!("{}: {}", label, chosen),
        )
    });
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let painter = ui.painter();
    let border = if response.hovered() || open {
        Color32::from_rgb(0x4a, 0x4a, 0x4a)
    } else {
        theme::BORDER
    };
    painter.rect(
        rect,
        5.0,
        theme::FIELD,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    painter.text(
        rect.left_center() + vec2(10.0, 0.0),
        Align2::LEFT_CENTER,
        chosen,
        FontId::proportional(12.5),
        theme::TEXT_STRONG,
    );
    // A small chevron, and the hint before it.
    let c = rect.right_center() - vec2(14.0, 0.0);
    let stroke = Stroke::new(1.3, theme::TEXT_WEAK);
    painter.line_segment([c + vec2(-3.5, -1.5), c + vec2(0.0, 2.0)], stroke);
    painter.line_segment([c + vec2(0.0, 2.0), c + vec2(3.5, -1.5)], stroke);
    if !hint.is_empty() {
        painter.text(
            c - vec2(12.0, 0.0),
            Align2::RIGHT_CENTER,
            hint,
            FontId::proportional(11.0),
            theme::TEXT_WEAK,
        );
    }
    egui::Popup::menu(&response).width(width).show(|ui| {
        ui.set_min_width(width - 12.0);
        add(ui);
    });
    response
}

/// One entry of a [`drop_down`]'s list: `name`, with `hint` on its right.
pub(crate) fn drop_down_entry(ui: &mut Ui, chosen: bool, name: &str, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, chosen, name));
    let painter = ui.painter();
    if chosen {
        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x3d, 0x3d, 0x3d));
    } else if response.hovered() {
        painter.rect_filled(rect, 4.0, theme::HOVER);
    }
    painter.text(
        rect.left_center() + vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(12.5),
        if chosen || response.hovered() {
            theme::TEXT_STRONG
        } else {
            theme::TEXT
        },
    );
    painter.text(
        rect.right_center() - vec2(8.0, 0.0),
        Align2::RIGHT_CENTER,
        hint,
        FontId::proportional(11.0),
        theme::TEXT_WEAK,
    );
    response
}

/// A switch in a small rounded box, for things a file includes.
pub(crate) fn chip(ui: &mut Ui, on: &mut bool, name: &str) -> Response {
    Frame::new()
        .fill(Color32::from_rgb(0x2c, 0x2c, 0x2c))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(13))
        .inner_margin(Margin::symmetric(8, 1))
        .show(ui, |ui| ui.checkbox(on, name))
        .inner
}

/// A value fixed by a choice made elsewhere (a preset), with a small lock
/// before it.
pub(crate) fn locked(ui: &mut Ui, text: &str) -> Response {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(12.0, 14.0), Sense::hover());
        let painter = ui.painter();
        let body = Rect::from_center_size(rect.center() + vec2(0.0, 2.0), vec2(9.0, 7.0));
        painter.rect_filled(body, 1.5, theme::TEXT_WEAK);
        painter.circle_stroke(
            pos2(body.center().x, body.top()),
            3.0,
            Stroke::new(1.3, theme::TEXT_WEAK),
        );
        painter.rect_filled(
            Rect::from_min_size(pos2(body.left(), body.top() - 0.5), vec2(9.0, 1.5)),
            0.0,
            theme::TEXT_WEAK,
        );
        ui.label(RichText::new(text).color(theme::TEXT))
    })
    .inner
}

/// A path or place, in a box with a folder before it.
pub(crate) fn place(ui: &mut Ui, text: &str) -> Response {
    Frame::new()
        .fill(theme::FIELD)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(14.0, 12.0), Sense::hover());
                let stroke = Stroke::new(1.2, theme::TEXT_WEAK);
                let r = rect.shrink2(vec2(1.0, 1.5));
                ui.painter().rect_stroke(
                    Rect::from_min_max(pos2(r.left(), r.top() + 2.0), r.max),
                    1.5,
                    stroke,
                    StrokeKind::Middle,
                );
                ui.painter().line_segment(
                    [pos2(r.left(), r.top() + 2.0), pos2(r.left() + 4.0, r.top())],
                    stroke,
                );
                ui.add(egui::Label::new(RichText::new(text).color(theme::TEXT)).truncate())
            })
            .inner
        })
        .inner
}

/// Numbers side by side, each in a box with a word under it ("3
/// objects").
pub(crate) fn tiles(ui: &mut Ui, numbers: &[(String, &str)]) {
    let gap = 8.0;
    let count = numbers.len().max(1) as f32;
    let width = ((ui.available_width() - gap * (count - 1.0)) / count).floor();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (value, word) in numbers {
            Frame::new()
                .fill(theme::FIELD)
                .corner_radius(CornerRadius::same(5))
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(width - 24.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(value)
                                .font(theme::semibold(ui.ctx(), 17.0))
                                .color(theme::TEXT_STRONG),
                        );
                        ui.label(
                            RichText::new(*word)
                                .font(help_font())
                                .color(theme::TEXT_WEAK),
                        );
                    });
                });
        }
    });
}

/// The window's main button, in the accent colour: Export…, Save. Put it
/// in a footer first, so it sits on the right.
pub(crate) fn main_button(ui: &mut Ui, text: &str) -> Response {
    let accent = theme::accent(ui.ctx()).colors();
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .font(theme::semibold(ui.ctx(), 12.5))
                .color(Color32::WHITE),
        )
        .fill(accent.fill)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(5))
        .min_size(vec2(84.0, 28.0)),
    )
}

/// Any other button in a footer: Cancel, Close.
pub(crate) fn plain_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(text)
            .fill(Color32::from_rgb(0x2e, 0x2e, 0x2e))
            .stroke(Stroke::new(1.0, Color32::from_rgb(0x3a, 0x3a, 0x3a)))
            .corner_radius(CornerRadius::same(5))
            .min_size(vec2(72.0, 28.0)),
    )
}

/// Whether Enter or Esc was pressed for a window's buttons this frame
/// (not while typing in a field): `(enter, escape)`. Both are taken, so
/// nothing else acts on them too.
pub(crate) fn enter_or_escape(ctx: &egui::Context) -> (bool, bool) {
    if ctx.egui_wants_keyboard_input() || egui::Popup::is_any_open(ctx) {
        return (false, false);
    }
    ctx.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
        )
    })
}

/// How the app's menus look: roomier rows than egui's own, with the same
/// clear background.
pub(crate) fn menu_style(style: &mut egui::Style) {
    egui::containers::menu::menu_style(style);
    style.spacing.button_padding = vec2(8.0, 4.0);
    style.spacing.item_spacing.y = 1.0;
}

/// The menu bars' setting: their menus look as [`menu_style`] says.
pub(crate) fn menu_config() -> egui::containers::menu::MenuConfig {
    egui::containers::menu::MenuConfig::new().style(menu_style)
}

/// The narrowest a menu is, so shortcuts sit well apart from the names.
pub(crate) const MENU_WIDTH: f32 = 210.0;

/// A menu on a menu bar (or a submenu): `ui.menu_button`, at least
/// [`MENU_WIDTH`] wide.
pub(crate) fn menu<R>(
    ui: &mut Ui,
    title: impl Into<egui::WidgetText>,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    ui.menu_button(title, |ui| {
        ui.set_min_width(MENU_WIDTH);
        add(ui)
    })
}
