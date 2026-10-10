//! The navigation controls picker: a small button in the viewport's
//! bottom-right corner naming the scheme in use, which opens a list of
//! schemes to choose from, with a few related switches.

use super::ViewportPanel;
use crate::icons::{self, Icon};
use crate::theme;
use egui::{
    FontId, Frame, Margin, Popup, PopupCloseBehavior, Rect, RectAlign, Sense, Stroke, Ui,
    WidgetInfo, WidgetType, pos2, vec2,
};
use tools::{NavigationSettings, Scheme};

/// Accessible name of the picker's button, and the title of its list.
pub const PICKER_LABEL: &str = "Navigation controls";

/// Width of the open list, in logical pixels.
const LIST_WIDTH: f32 = 344.0;

/// Draws the picker's button in the bottom-right corner of `rect`, and
/// its list when open.
pub(super) fn show(ui: &mut Ui, rect: Rect, panel: &mut ViewportPanel) {
    let scheme = panel.options.navigation.scheme;
    let font = FontId::proportional(12.0);
    let name = ui
        .painter()
        .layout_no_wrap(scheme.name().to_string(), font, theme::TEXT);
    let size = vec2(name.size().x + 46.0, 26.0);
    let button = Rect::from_min_size(rect.right_bottom() - size - vec2(10.0, 8.0), size);
    let response = ui.interact(button, ui.id().with("navigation_picker"), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, PICKER_LABEL));

    let painter = ui.painter();
    let fill = if response.hovered() {
        theme::HOVER
    } else {
        theme::PALETTE
    };
    painter.rect(
        button,
        5.0,
        fill,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    icons::paint(
        painter,
        Rect::from_center_size(button.left_center() + vec2(14.0, 0.0), vec2(14.0, 14.0)),
        Icon::Mouse,
        theme::TEXT,
    );
    painter.galley(
        pos2(
            button.left() + 26.0,
            button.center().y - name.size().y / 2.0,
        ),
        name,
        theme::TEXT,
    );
    // A small chevron pointing up: the list opens upwards.
    let tip = pos2(button.right() - 12.0, button.center().y - 2.0);
    painter.line(
        vec![tip + vec2(-3.5, 3.0), tip, tip + vec2(3.5, 3.0)],
        Stroke::new(1.5, theme::TEXT_WEAK),
    );
    let response = response.on_hover_text("Choose which mouse buttons and keys move the view");

    if panel.open_picker {
        Popup::open_id(ui.ctx(), Popup::default_response_id(&response));
        panel.open_picker = false;
    }
    Popup::from_toggle_button_response(&response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::TOP_END)
        .gap(6.0)
        .width(LIST_WIDTH)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(0, 10))
                .shadow(ui.visuals().popup_shadow),
        )
        .show(|ui| list(ui, &mut panel.options.navigation));
}

/// The open list: a title, every scheme, and the related switches.
fn list(ui: &mut Ui, settings: &mut NavigationSettings) {
    ui.set_width(LIST_WIDTH);
    ui.spacing_mut().item_spacing.y = 0.0;
    Frame::new()
        .inner_margin(Margin {
            left: 14,
            right: 14,
            top: 0,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                egui::RichText::new(PICKER_LABEL)
                    .color(theme::TEXT_STRONG)
                    .strong(),
            );
            ui.label(
                egui::RichText::new("How the mouse moves the view. Try them, keep your favourite.")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        });
    for scheme in Scheme::ALL {
        if scheme_row(ui, scheme, settings.scheme == scheme).clicked() {
            settings.scheme = scheme;
        }
    }
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);
    Frame::new()
        .inner_margin(Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.checkbox(
                &mut settings.alt_everywhere,
                "Alt + mouse works in every scheme",
            );
            ui.add_enabled(
                false,
                egui::Checkbox::new(
                    &mut false,
                    "Orbit and zoom around the point under the mouse",
                ),
            )
            .on_disabled_hover_text("Arrives in the next update");
            ui.checkbox(&mut settings.invert_orbit, "Invert orbit direction");
        });
}

/// One scheme in the list: a round marker, its name, and its controls.
fn scheme_row(ui: &mut Ui, scheme: Scheme, chosen: bool) -> egui::Response {
    let accent = theme::accent(ui.ctx()).colors();
    let wrap = LIST_WIDTH - 54.0;
    let name = ui.painter().layout_no_wrap(
        scheme.name().to_string(),
        FontId::proportional(12.5),
        theme::TEXT_STRONG,
    );
    let summary = ui.painter().layout(
        scheme.summary().to_string(),
        FontId::proportional(11.5),
        theme::TEXT_WEAK,
        wrap,
    );
    let height = 14.0 + name.size().y + 3.0 + summary.size().y;
    let (rect, response) = ui.allocate_exact_size(vec2(LIST_WIDTH, height), Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, chosen, scheme.name()));
    let painter = ui.painter();
    if chosen {
        painter.rect_filled(rect, 0.0, accent.soft);
    } else if response.hovered() {
        painter.rect_filled(rect, 0.0, theme::HOVER);
    }
    let marker = pos2(rect.left() + 21.0, rect.top() + 7.0 + name.size().y / 2.0);
    painter.circle_filled(marker, 7.0, theme::FIELD);
    if chosen {
        painter.circle_stroke(marker, 5.0, Stroke::new(4.0, accent.light));
    } else {
        painter.circle_stroke(marker, 6.25, Stroke::new(1.5, theme::TEXT_WEAK));
    }
    let text_left = rect.left() + 38.0;
    let name_height = name.size().y;
    painter.galley(pos2(text_left, rect.top() + 7.0), name, theme::TEXT_STRONG);
    painter.galley(
        pos2(text_left, rect.top() + 7.0 + name_height + 3.0),
        summary,
        theme::TEXT_WEAK,
    );
    response
}
