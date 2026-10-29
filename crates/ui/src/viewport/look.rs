//! The look row in the viewport's bottom-left corner, beside the axis
//! marker: a button naming the shading in use (Shaded, Wireframe), which
//! opens the list to choose from
//! ([decision 0063](../../../../docs/decisions/0063-wireframe.md)), with
//! the Edges switch ([decision 0220](../../../../docs/decisions/0220-edges-switch.md)).

use super::{PaintPreview, Shading, Surface, ViewportPanel};
use crate::text::tr;
use crate::theme;
use egui::{
    FontId, Frame, Margin, Popup, PopupCloseBehavior, Rect, RectAlign, Sense, Stroke, Ui,
    WidgetInfo, WidgetType, pos2, vec2,
};

/// Accessible name of the shading button, and the heading of its list and
/// of the Display menu's choices.
pub(crate) const SHADING_LABEL: &str = crate::text!("Shading");

/// How far the row starts from the view's left edge: past the axis
/// marker.
pub(super) const ROW_LEFT: f32 = 82.0;
/// The row's buttons' height, as the navigation picker's.
const HEIGHT: f32 = 26.0;
/// The shading list's width, fixed so it never changes size as the mouse
/// moves over it (a tooltip appearing used to make it narrower).
const LIST_WIDTH: f32 = 180.0;

/// Draws the shading button at the bottom left of `rect` (past the axis
/// marker), and its list when open.
pub(super) fn show(ui: &mut Ui, rect: Rect, panel: &mut ViewportPanel) -> Rect {
    let shading = panel.options.shading;
    let edges = panel.options.show_edges;
    let font = FontId::proportional(12.0);
    // A paint preview other than Lit is named too (decision 0198).
    let preview = panel.options.paint_preview;
    let text = if panel.options.surface == Surface::Painted && preview != PaintPreview::Lit {
        format!("{} · {}", shading.label(), preview.label())
    } else {
        shading.label().to_string()
    };
    let name = ui.painter().layout_no_wrap(text, font, theme::TEXT);
    let size = vec2(name.size().x + 44.0, HEIGHT);
    let button = Rect::from_min_size(
        pos2(rect.left() + ROW_LEFT, rect.bottom() - HEIGHT - 8.0),
        size,
    );
    let response = ui.interact(button, ui.id().with("shading_button"), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr(SHADING_LABEL)));
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
    swatch(
        painter,
        button.left_center() + vec2(14.0, 0.0),
        shading,
        edges,
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
    let tip = pos2(button.right() - 11.0, button.center().y - 2.0);
    painter.line(
        vec![tip + vec2(-3.5, 3.0), tip, tip + vec2(3.5, 3.0)],
        Stroke::new(1.5, theme::TEXT_WEAK),
    );
    let response = response.on_hover_text(tr(
        "How shapes are drawn: shaded or wireframe, with or without edges",
    ));
    Popup::from_toggle_button_response(&response)
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
                .shadow(ui.visuals().popup_shadow),
        )
        .show(|ui| {
            ui.set_width(LIST_WIDTH - 16.0);
            choices(ui, &mut panel.options);
            if panel.options.surface == Surface::Painted {
                ui.separator();
                preview_choices(ui, &mut panel.options);
            }
            ui.separator();
            player_choices(ui, &mut panel.options);
        });
    button
}

/// The player view's switch (decision 0200), in the shading button's list
/// and the Display menu.
pub(crate) const PLAYER_ENTRY: &str = crate::text!("See it as the player will");

/// The player view's switch, and its settings while on.
pub(crate) fn player_choices(ui: &mut Ui, options: &mut super::ViewOptions) {
    ui.checkbox(&mut options.player_view, tr(PLAYER_ENTRY))
        .on_hover_text(tr(
            "Show the model as small as a player sees it in the game, its pixels enlarged: \
             the texture blurs and loses detail as the game will show it. Painting still works",
        ));
    if !options.player_view {
        return;
    }
    let player = &mut options.player;
    egui::Grid::new("player_view_settings")
        .num_columns(2)
        .spacing(vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label(tr("Distance"))
                .on_hover_text(tr("How far the player's camera is from the model"));
            ui.add(
                egui::DragValue::new(&mut player.distance)
                    .range(0.5..=200.0)
                    .speed(0.1)
                    .suffix(" m"),
            );
            ui.end_row();
            let screen = ui.label(tr("Screen"));
            egui::ComboBox::from_id_salt("player_view_screen")
                .selected_text(format!("{}p", player.screen_height))
                .show_ui(ui, |ui| {
                    for height in super::SCREEN_HEIGHTS {
                        ui.selectable_value(
                            &mut player.screen_height,
                            height,
                            format!("{height}p"),
                        );
                    }
                })
                .response
                .labelled_by(screen.id)
                .on_hover_text(tr("The game's screen height in pixels"));
            ui.end_row();
            ui.label(tr("View angle")).on_hover_text(tr(
                "The game camera's field of view from top to bottom: about 60° for most \
                 third-person games",
            ));
            ui.add(
                egui::DragValue::new(&mut player.field_of_view)
                    .range(20.0..=120.0)
                    .speed(0.5)
                    .suffix("°"),
            );
            ui.end_row();
        });
}

/// Heading of the paint previews (decision 0198), in the shading button's
/// list and the Display menu, shown while the surface is Painted.
pub(crate) const PREVIEW_LABEL: &str = crate::text!("Paint preview");

/// The paint previews, under a heading.
pub(crate) fn preview_choices(ui: &mut Ui, options: &mut super::ViewOptions) {
    ui.label(egui::RichText::new(tr(PREVIEW_LABEL)).color(theme::TEXT_WEAK));
    let preview = &mut options.paint_preview;
    for choice in PaintPreview::ALL {
        if ui
            .add(
                egui::Button::selectable(*preview == choice, choice.label())
                    .wrap_mode(egui::TextWrapMode::Extend),
            )
            .on_hover_text(choice.explanation())
            .clicked()
        {
            *preview = choice;
            ui.close();
        }
    }
    // The realistic preview's sun, turned round the model (decision 0207).
    if *preview == PaintPreview::Realistic {
        ui.horizontal(|ui| {
            ui.label(tr(LIGHT_TURN_LABEL));
            ui.add(
                egui::Slider::new(&mut options.light_turn, 0.0..=360.0)
                    .suffix("°")
                    .step_by(1.0),
            )
            .on_hover_text(tr(
                "Turn the sun round the model, to see how the roughness and metal catch \
                 the light from every side",
            ));
        });
    }
}

/// The realistic preview's light slider's name (decision 0207).
pub(crate) const LIGHT_TURN_LABEL: &str = crate::text!("Turn the light");

/// The Edges switch's name (decision 0220), in the shading button's list
/// and the Display menu.
pub(crate) const EDGES_LABEL: &str = crate::text!("Edges");

/// The shading choices, under a heading, then the Edges switch: in the
/// button's list and the Display menu.
pub(crate) fn choices(ui: &mut Ui, options: &mut super::ViewOptions) {
    ui.label(egui::RichText::new(tr(SHADING_LABEL)).color(theme::TEXT_WEAK));
    let shading = &mut options.shading;
    for look in Shading::ALL {
        if ui
            .add(
                egui::Button::selectable(*shading == look, look.label())
                    .wrap_mode(egui::TextWrapMode::Extend),
            )
            .on_hover_text(look.explanation())
            .clicked()
        {
            *shading = look;
            ui.close();
        }
    }
    // Wireframe is all edges: the switch has nothing to add there.
    let wireframe = *shading == Shading::Wireframe;
    ui.add_enabled(
        !wireframe,
        egui::Checkbox::new(&mut options.show_edges, tr(EDGES_LABEL)),
    )
    .on_hover_text(tr(
        "Draw every edge over the surfaces, to see how the shape is built. The Points, \
         Edges and Polygons modes tick it while you use them",
    ))
    .on_disabled_hover_text(tr("Wireframe always shows every edge"));
}

/// A small square showing the look: filled for shaded, filled and crossed
/// with edges on, an outline crossed for wireframe.
fn swatch(painter: &egui::Painter, centre: egui::Pos2, shading: Shading, edges: bool) {
    let square = Rect::from_center_size(centre, vec2(11.0, 11.0));
    let line = Stroke::new(1.2, theme::TEXT);
    if shading.shows_faces() {
        painter.rect_filled(square, 2.0, theme::TEXT_WEAK);
    }
    painter.rect_stroke(square, 2.0, line, egui::StrokeKind::Inside);
    if shading == Shading::Wireframe || edges {
        let wire = Stroke::new(1.0, theme::TEXT_STRONG);
        painter.line_segment([square.left_top(), square.right_bottom()], wire);
        painter.line_segment([square.center_top(), square.center_bottom()], wire);
        painter.line_segment([square.left_center(), square.right_center()], wire);
    }
}
