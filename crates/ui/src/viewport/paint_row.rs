//! The Paint page's view controls ([decision 0224](../../../../docs/decisions/0224-paint-tool-row.md)):
//! along the bottom left of the view, in place of its menu line, dice,
//! shading and navigation buttons, four buttons that open lists:
//!
//! - **☰**: the view's menus (View, Cameras, Display, Options, Panel);
//! - **3D**: which camera the view looks through;
//! - **Lit**: the look (shaded or wireframe), the paint preview and the
//!   player view;
//! - **Show**: what is drawn over the surfaces (edges, the grid, cavity,
//!   hard edges, seams, normals, collision, triangles).
//!
//! The workbench draws the tools right after them ([`crate::paint_tools`]).

use super::{PaintPreview, Shading, Surface, ViewKind, ViewportPanel};
use crate::icons::{self, Icon};
use crate::text::tr;
use crate::theme;
use egui::{
    FontId, Frame, Margin, Popup, PopupCloseBehavior, Rect, RectAlign, Sense, Stroke, Ui,
    WidgetInfo, WidgetType, pos2, vec2,
};

/// Accessible name of the group.
pub(crate) const VIEW_CONTROLS_LABEL: &str = crate::text!("View controls");
/// Accessible names of its buttons.
pub(crate) const VIEW_MENUS_LABEL: &str = crate::text!("View menus");
pub(crate) const CAMERA_LABEL: &str = crate::text!("Camera");
pub(crate) const LOOK_LABEL: &str = crate::text!("Look");
pub(crate) const SHOW_LABEL: &str = crate::text!("Show");

/// The buttons' height, and the gap round them inside the group.
pub(crate) const BUTTON: f32 = 26.0;
pub(crate) const PAD: f32 = 3.0;
/// How far the group sits from the view's left and bottom edges.
const MARGIN: f32 = 10.0;
/// The lists' width.
const LIST_WIDTH: f32 = 220.0;

/// The group's outline and fill, shared with the tools after it so the two
/// read as one row.
pub(crate) fn group_frame(painter: &egui::Painter, rect: Rect) {
    painter.rect(
        rect,
        5.0,
        theme::PALETTE,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
}

/// One button of the group: hover and pressed looks, and its name.
fn button(ui: &mut Ui, rect: Rect, id: &str, name: &str) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(("paint_view", id)), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name));
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
    }
    response
}

/// The text on the 3D button: 3D for the perspective view, otherwise the
/// camera's name (Top, Front…).
fn camera_text(kind: ViewKind) -> String {
    if kind == ViewKind::Perspective {
        tr("3D").to_string()
    } else {
        kind.name().to_string()
    }
}

/// The text on the Lit button: the paint preview (Lit, Unlit, Toon…), or
/// Wireframe.
fn look_text(shading: Shading, surface: Surface, preview: PaintPreview) -> String {
    if shading == Shading::Wireframe {
        shading.label().to_string()
    } else if surface == Surface::Painted {
        preview.label().to_string()
    } else {
        shading.label().to_string()
    }
}

/// Draws the group at the bottom left of `body` and opens its lists.
/// Returns where it is, for the tools to follow it.
pub(super) fn show(ui: &mut Ui, body: Rect, panel: &mut ViewportPanel) -> Rect {
    let font = FontId::proportional(12.0);
    let shadow = ui.visuals().popup_shadow;
    let words = [
        camera_text(panel.active_kind()),
        look_text(
            panel.options.shading,
            panel.options.surface,
            panel.options.paint_preview,
        ),
        tr(SHOW_LABEL).to_string(),
    ];
    let widths: Vec<f32> = words
        .iter()
        .map(|w| {
            ui.painter()
                .layout_no_wrap(w.clone(), font.clone(), theme::TEXT)
                .size()
                .x
                + 26.0
        })
        .collect();
    let width = PAD + BUTTON + widths.iter().map(|w| w + PAD).sum::<f32>() + PAD;
    let group = Rect::from_min_size(
        pos2(
            body.left() + MARGIN,
            body.bottom() - MARGIN - BUTTON - 2.0 * PAD,
        ),
        vec2(width, BUTTON + 2.0 * PAD),
    );
    let response = ui.allocate_rect(group, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(VIEW_CONTROLS_LABEL)));
    group_frame(ui.painter(), group);
    // ☰: the view's menus.
    let mut x = group.left() + PAD;
    let menu_rect = Rect::from_min_size(pos2(x, group.top() + PAD), vec2(BUTTON, BUTTON));
    let menu = button(ui, menu_rect, "menus", tr(VIEW_MENUS_LABEL));
    icons::paint(
        ui.painter(),
        Rect::from_center_size(menu_rect.center(), vec2(15.0, 15.0)),
        Icon::Menu,
        theme::TEXT,
    );
    let menu = menu.on_hover_text(tr(
        "The view's menus: View, Cameras, Display, Options and Panel",
    ));
    list(&menu, shadow, |ui| {
        super::chrome::menus(ui, panel);
        // The navigation picker's list, which has no button of its own on
        // this page.
        crate::dialog::menu(ui, tr(super::PICKER_LABEL), |ui| {
            super::picker::list(ui, &mut panel.options.navigation);
        });
    });
    panel.open_picker = false;
    x += BUTTON + PAD;
    // 3D, Lit and Show: their words and a small upward chevron.
    let ids = ["camera", "look", "show"];
    let names = [tr(CAMERA_LABEL), tr(LOOK_LABEL), tr(SHOW_LABEL)];
    let tips = [
        tr(
            "Which camera the view looks through: 3D (perspective), or flat from the top, front or side",
        ),
        tr("How the model is drawn: shaded or wireframe, and how the painting is lit"),
        tr("What is drawn over the model: its edges, the grid, cavity, hard edges, seams and more"),
    ];
    for i in 0..3 {
        let rect = Rect::from_min_size(pos2(x, group.top() + PAD), vec2(widths[i], BUTTON));
        let response = button(ui, rect, ids[i], names[i]);
        let text = ui
            .painter()
            .layout_no_wrap(words[i].clone(), font.clone(), theme::TEXT);
        ui.painter().galley(
            pos2(rect.left() + 9.0, rect.center().y - text.size().y / 2.0),
            text,
            theme::TEXT,
        );
        let tip = pos2(rect.right() - 10.0, rect.center().y - 2.0);
        ui.painter().line(
            vec![tip + vec2(-3.0, 2.5), tip, tip + vec2(3.0, 2.5)],
            Stroke::new(1.3, theme::TEXT_WEAK),
        );
        let response = response.on_hover_text(tips[i]);
        match i {
            0 => list(&response, shadow, |ui| {
                super::chrome::camera_entries(ui, panel);
                ui.separator();
                super::chrome::frame_entries(ui, panel);
            }),
            1 => list(&response, shadow, |ui| {
                super::look::shading_choices(ui, &mut panel.options.shading);
                if panel.options.surface == Surface::Painted {
                    ui.separator();
                    super::look::preview_choices(ui, &mut panel.options);
                }
                ui.separator();
                super::look::player_choices(ui, &mut panel.options);
            }),
            _ => list(&response, shadow, |ui| {
                super::look::edges_switch(ui, &mut panel.options);
                super::look::paint_grid_switch(ui, &mut panel.options);
                super::chrome::show_entries(ui, &mut panel.options);
            }),
        }
        x += widths[i] + PAD;
    }
    group
}

/// A list opening upwards from `response`'s button, closed by a click
/// elsewhere.
fn list(response: &egui::Response, shadow: egui::epaint::Shadow, add: impl FnOnce(&mut Ui)) {
    Popup::from_toggle_button_response(response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::TOP_START)
        .gap(6.0)
        .width(LIST_WIDTH)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(8, 8))
                .shadow(shadow),
        )
        .show(|ui| {
            ui.set_width(LIST_WIDTH - 16.0);
            add(ui);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buttons_name_the_camera_and_the_look() {
        assert_eq!(camera_text(ViewKind::Perspective), "3D");
        assert_eq!(camera_text(ViewKind::Top), ViewKind::Top.name());
        assert_eq!(
            look_text(Shading::Shaded, Surface::Painted, PaintPreview::Toon),
            PaintPreview::Toon.label()
        );
        assert_eq!(
            look_text(Shading::Wireframe, Surface::Painted, PaintPreview::Lit),
            Shading::Wireframe.label()
        );
        assert_eq!(
            look_text(Shading::Shaded, Surface::Plain, PaintPreview::Lit),
            Shading::Shaded.label()
        );
    }
}
