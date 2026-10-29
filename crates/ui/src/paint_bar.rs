//! The floating brush bar ([decision 0213](../../../docs/decisions/0213-paint-interface-overhaul.md)):
//! in Paint mode, the brush settings changed all the time float over the
//! top of the viewport, in the style of the viewport's own buttons: the
//! colour square (its picker a pop-up), the brush's name, Size, Flow,
//! Hardness and Spacing, and switches for Mirror, the pen's pressure and
//! smoothing. Research into texture painters found this in all of them:
//! the few numbers you change every minute sit over the view, not in a
//! panel.
//!
//! Its look follows the mockup ([decision 0226](../../../docs/decisions/0226-brush-bar-look.md)): the colour and a
//! second colour behind it, the brush's tip as a drop-down of the ready-made
//! brushes, the numbers as bars filled to their value (drag sideways to
//! change, click to type), and the switches as small icons.

use crate::attributes::{AttributesPanel, BrushSettings, MirrorAxis, PaintTool, SizeSpace};
use crate::text::tr;
use crate::theme;
use egui::{
    Color32, Frame, Margin, Popup, PopupCloseBehavior, Pos2, Rect, RectAlign, RichText, Sense,
    Stroke, Ui, WidgetInfo, WidgetType, vec2,
};
use paint::BrushKind;

/// Accessible name of the colour square, which opens the colour picker.
pub(crate) const COLOUR_LABEL: &str = crate::text!("Brush colour");
/// Accessible names of the bar's switches.
pub(crate) const MIRROR_SWITCH: &str = crate::text!("Mirror painting switch");
pub(crate) const PEN_SWITCH: &str = crate::text!("Pen pressure switch");
pub(crate) const SMOOTHING_SWITCH: &str = crate::text!("Smoothing switch");
/// Accessible name of the second colour, behind the first.
pub(crate) const SECOND_COLOUR_LABEL: &str = crate::text!("Second colour");
/// Accessible name of the brush drop-down.
pub(crate) const BRUSH_LIST_LABEL: &str = crate::text!("Brush");
/// Accessible name of the bar itself.
pub(crate) const BAR_LABEL: &str = crate::text!("Brush bar");

/// How far the bar sits from the view's left and from under its header.
const INSET: f32 = 8.0;
/// Room left at the top for the view's name, drawn there by the view.
pub(crate) const NAME_ROOM: f32 = 22.0;
/// The gap above the bar on the Paint page, whose view has no menu line or
/// name (decision 0224).
pub(crate) const TOP_GAP: f32 = 8.0;
/// The colour square's side.
const SQUARE: f32 = 20.0;
/// How far the second colour sits behind and below the first.
const BEHIND: f32 = 6.0;
/// The width of the colour picker pop-up.
const PICKER_WIDTH: f32 = 290.0;

/// Draws the bar over the top left of the painting `area` (decision
/// 0215: the viewport and the UV Editor beside it), its top at `top`,
/// changing the brush in `attributes`.
pub(crate) fn show(
    ctx: &egui::Context,
    area: Rect,
    top: f32,
    attributes: &mut AttributesPanel,
    palettes: &[paint::palette::Palette],
    presets: &[crate::brush_bar::BrushPreset],
) {
    let at = Pos2::new(area.left() + INSET, top);
    egui::Area::new(egui::Id::new("paint_bar"))
        .order(egui::Order::Middle)
        .fixed_pos(at)
        .constrain_to(area)
        .show(ctx, |ui| {
            let width = (area.width() - 2.0 * INSET).max(SQUARE * 4.0);
            let fill = attributes.brush.tool == PaintTool::Fill;
            let shown: &[usize] = if fill { &[0] } else { &[0, 1, 2] };
            // A frame's width is only known once drawn: each group's width
            // from the frame before decides the rows (narrow views get two
            // or three).
            let ids = GROUPS.map(|name| ui.id().with(("paint_bar_group", name)));
            let widths: Vec<f32> = shown
                .iter()
                .map(|g| ui.ctx().data(|d| d.get_temp(ids[*g])).unwrap_or(0.0))
                .collect();
            let rows = rows_of(&widths, width, GAP);
            let response = ui
                .vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(GAP, GAP);
                    for row in rows {
                        ui.horizontal(|ui| {
                            for k in row {
                                let g = shown[k];
                                let drawn = group(ui, |ui| match g {
                                    0 => name_group(ui, attributes, palettes, presets),
                                    1 => numbers(ui, &mut attributes.brush),
                                    _ => switches(ui, &mut attributes.brush),
                                });
                                ui.ctx().data_mut(|d| d.insert_temp(ids[g], drawn));
                            }
                        });
                    }
                })
                .response;
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(BAR_LABEL)));
        });
}

/// The bar's groups: the colour and the tool's name, the numbers, the
/// switches.
const GROUPS: [&str; 3] = ["colour", "numbers", "switches"];
/// The gap between groups, across and between rows.
const GAP: f32 = 6.0;

/// Groups of `widths` in rows no wider than `width` (`gap` apart), in
/// order; a group wider than the whole width gets a row of its own.
fn rows_of(widths: &[f32], width: f32, gap: f32) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0.0;
    for (i, w) in widths.iter().enumerate() {
        match rows.last_mut() {
            Some(row) if used + gap + w <= width => {
                row.push(i);
                used += gap + w;
            }
            _ => {
                rows.push(vec![i]);
                used = *w;
            }
        }
    }
    rows
}

/// The colour squares (for tools that paint a colour) and the brush.
fn name_group(
    ui: &mut Ui,
    attributes: &mut AttributesPanel,
    palettes: &[paint::palette::Palette],
    presets: &[crate::brush_bar::BrushPreset],
) {
    let fill = attributes.brush.tool == PaintTool::Fill;
    if fill || attributes.brush.brush.kind == BrushKind::Paint {
        colour_squares(ui, attributes, palettes);
    }
    if fill {
        let name = if attributes.brush.bucket {
            tr("Fill the layer")
        } else {
            tr("Polygon fill")
        };
        ui.label(RichText::new(name).color(theme::TEXT))
            .on_hover_text(tr("The tool in use"));
    } else {
        brush_list(ui, &mut attributes.brush, presets);
    }
}

/// A soft round dab of `radius` at `centre`, as hard as `hardness`: the
/// brush's picture.
fn dab(painter: &egui::Painter, centre: Pos2, radius: f32, hardness: f32) {
    let steps = 8;
    for i in (0..steps).rev() {
        let t = (i + 1) as f32 / steps as f32;
        let edge = hardness.clamp(0.0, 1.0);
        let alpha = if t <= edge.max(0.05) {
            1.0
        } else {
            let s = 1.0 - (t - edge) / (1.0 - edge).max(1e-3);
            s * s * (3.0 - 2.0 * s)
        };
        painter.circle_filled(
            centre,
            radius * t,
            Color32::from_white_alpha((alpha * 255.0 / steps as f32 * 2.0).min(255.0) as u8),
        );
    }
}

/// The brush: its picture, the ready-made brush it matches (or the tool's
/// name) and a drop-down of the ready-made and kept brushes.
fn brush_list(
    ui: &mut Ui,
    settings: &mut BrushSettings,
    presets: &[crate::brush_bar::BrushPreset],
) {
    let chosen = presets.iter().find(|p| p.matches(settings));
    let name = chosen.map_or_else(
        || crate::attributes::kind_name(settings.brush.kind).to_string(),
        |p| p.name.clone(),
    );
    let hardness = settings.brush.hardness;
    let response = ui
        .add(egui::Button::new(
            RichText::new(format!("      {name}  ⏷")).color(theme::TEXT),
        ))
        .on_hover_text(tr(
            "The brush: click for the ready-made brushes and those you kept. All of them \
             are in the shelf under the views too",
        ));
    response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, tr(BRUSH_LIST_LABEL)));
    dab(
        ui.painter(),
        response.rect.left_center() + vec2(14.0, 0.0),
        8.0,
        hardness,
    );
    Popup::from_toggle_button_response(&response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::BOTTOM_START)
        .gap(6.0)
        .width(LIST_WIDTH)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(6, 6))
                .shadow(ui.visuals().popup_shadow),
        )
        .show(|ui| {
            ui.set_width(LIST_WIDTH - 12.0);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for preset in presets {
                        let on = preset.matches(settings);
                        let row = ui.add(
                            egui::Button::selectable(on, format!("      {}", preset.name))
                                .min_size(vec2(LIST_WIDTH - 24.0, 24.0)),
                        );
                        dab(
                            ui.painter(),
                            row.rect.left_center() + vec2(14.0, 0.0),
                            7.0,
                            preset.hardness,
                        );
                        if row.clicked() {
                            preset.apply(settings);
                            ui.close();
                        }
                    }
                });
        });
}

/// The brush drop-down's width.
const LIST_WIDTH: f32 = 200.0;

/// One floating group, in the viewport buttons' look; returns its width.
fn group(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> f32 {
    Frame::new()
        .fill(theme::PALETTE)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(5.0)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                add(ui);
            });
        })
        .response
        .rect
        .width()
}

/// The brush's colour as a square, with a second colour behind it (as in
/// painting programs): a click on the first opens the colour picker under
/// it, a click on the second swaps them.
fn colour_squares(
    ui: &mut Ui,
    attributes: &mut AttributesPanel,
    palettes: &[paint::palette::Palette],
) {
    let (both, _) = ui.allocate_exact_size(vec2(SQUARE + BEHIND, SQUARE + BEHIND), Sense::hover());
    let rect = Rect::from_min_size(both.min, vec2(SQUARE, SQUARE));
    let back = Rect::from_min_size(both.min + vec2(BEHIND, BEHIND), vec2(SQUARE, SQUARE));
    // The second colour, behind: only its edges below and to the right
    // of the front square show, and those are what take the click.
    let below = Rect::from_min_max(egui::pos2(back.min.x, rect.max.y), back.max);
    let right = Rect::from_min_max(
        egui::pos2(rect.max.x, back.min.y),
        egui::pos2(back.max.x, rect.max.y),
    );
    let behind = ui.interact(below, ui.id().with("second_colour"), Sense::click());
    behind.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr(SECOND_COLOUR_LABEL)));
    let beside = ui.interact(right, ui.id().with("second_colour_right"), Sense::click());
    let behind = behind.union(beside);
    let [r2, g2, b2, _] = attributes.brush.second_color;
    ui.painter()
        .rect_filled(back, 3.0, Color32::from_rgb(r2, g2, b2));
    ui.painter().rect_stroke(
        back,
        3.0,
        Stroke::new(1.0, theme::TEXT_WEAK),
        egui::StrokeKind::Outside,
    );
    if behind
        .on_hover_text(tr("The second colour: click to swap it with the brush's"))
        .clicked()
    {
        attributes.brush.swap_colours();
    }
    let response = ui.interact(rect, ui.id().with("first_colour"), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr(COLOUR_LABEL)));
    let [r, g, b, _] = attributes.brush.brush.color;
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, Color32::from_rgb(r, g, b));
    let edge = if response.hovered() {
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    painter.rect_stroke(rect, 3.0, Stroke::new(1.5, edge), egui::StrokeKind::Outside);
    let response = response.on_hover_text(tr(
        "The brush's colour: click to choose another. P picks the colour under the mouse",
    ));
    let mut popup = Popup::from_toggle_button_response(&response);
    // Opened from the shelf (a double-click there): once the click is over,
    // or the pop-up would take it for a click outside and shut at once.
    let clicking = ui.input(|i| i.pointer.any_click() || i.pointer.any_released());
    if attributes.open_colour_picker && !clicking {
        attributes.open_colour_picker = false;
        popup = popup.open_memory(egui::SetOpenCommand::Bool(true));
    }
    popup
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::BOTTOM_START)
        .gap(8.0)
        .width(PICKER_WIDTH)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(10, 10))
                .shadow(ui.visuals().popup_shadow),
        )
        .show(|ui| {
            ui.set_width(PICKER_WIDTH - 20.0);
            let mut colour = [r, g, b];
            attributes.colour.show(
                ui,
                &mut colour,
                palettes,
                Some(&mut attributes.picking),
                &mut attributes.palette_requests,
                false,
            );
            attributes.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        });
}

/// How a number is shown and typed in its bar: times `scale`, with
/// `suffix` and `decimals`.
#[derive(Clone, Copy)]
struct Shown {
    scale: f32,
    suffix: &'static str,
    decimals: usize,
    /// Whether the bar fills by the square root of its share, so small
    /// values (a small brush) get most of its width.
    eased: bool,
}

/// The width of a number's bar.
const FIELD_WIDTH: f32 = 84.0;
/// Its height.
const FIELD_HEIGHT: f32 = 20.0;

/// A number as a bar filled to its share of `range`, its value on the
/// right (decision 0226): drag sideways to change it (Shift for finer
/// steps), click to type it. Named `label` for screen readers and tests.
fn fill_field(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    (low, high): (f32, f32),
    shown: Shown,
) -> egui::Response {
    let id = ui.id().with(("fill_field", label));
    let typing = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    if typing {
        let mut typed = *value * shown.scale;
        let response = ui.add_sized(
            vec2(FIELD_WIDTH, FIELD_HEIGHT),
            egui::DragValue::new(&mut typed)
                .range(low * shown.scale..=high * shown.scale)
                .speed(0.0)
                .max_decimals(shown.decimals)
                .suffix(shown.suffix),
        );
        if ui
            .ctx()
            .data(|d| d.get_temp::<bool>(id.with("focus")))
            .unwrap_or(true)
        {
            response.request_focus();
            ui.ctx()
                .data_mut(|d| d.insert_temp(id.with("focus"), false));
        }
        if response.changed() {
            *value = (typed / shown.scale).clamp(low, high);
        }
        if response.lost_focus() || !response.has_focus() && response.clicked_elsewhere() {
            ui.ctx().data_mut(|d| {
                d.insert_temp(id, false);
                d.insert_temp(id.with("focus"), true);
            });
        }
        return named(response, label);
    }
    let (rect, response) =
        ui.allocate_exact_size(vec2(FIELD_WIDTH, FIELD_HEIGHT), Sense::click_and_drag());
    let span = (high - low).max(1e-6);
    let share = |v: f32| {
        let t = ((v - low) / span).clamp(0.0, 1.0);
        if shown.eased { t.sqrt() } else { t }
    };
    let unshare = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        low + span * if shown.eased { t * t } else { t }
    };
    if response.dragged() {
        let fine = if ui.input(|i| i.modifiers.shift) {
            0.1
        } else {
            1.0
        };
        let t = share(*value) + response.drag_delta().x / FIELD_WIDTH * fine;
        *value = unshare(t);
    }
    if response.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(id, true));
    }
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    let filled = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.left() + rect.width() * share(*value), rect.bottom()),
    );
    painter.rect_filled(filled, 3.0, theme::FIELD_FILL);
    let edge = if response.hovered() || response.dragged() {
        theme::TEXT_WEAK
    } else {
        theme::BORDER
    };
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    let text = format!(
        "{:.*}{}",
        shown.decimals,
        *value * shown.scale,
        shown.suffix
    );
    painter.text(
        rect.right_center() - vec2(6.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        text,
        egui::FontId::proportional(12.0),
        theme::TEXT_STRONG,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    named(response, label)
}

/// Size, Flow, Hardness and Spacing, as bars filled to their value.
fn numbers(ui: &mut Ui, settings: &mut BrushSettings) {
    weak(ui, tr("Size"));
    let size_help = tr(
        "The brush's radius: drag sideways or click to type. [ and ] change it, as does \
         Ctrl + right drag (sideways). Measured on screen, on the object or on the texture: \
         Size space in the Paint tab",
    );
    match settings.size_space {
        SizeSpace::Object => {
            let (low, high) = crate::attributes::brush_rows::SURFACE_RADIUS;
            let shown = Shown {
                scale: 1.0,
                suffix: " cm",
                decimals: 1,
                eased: true,
            };
            fill_field(
                ui,
                tr("Brush size on the surface"),
                &mut settings.surface_radius,
                (low, high),
                shown,
            )
        }
        SizeSpace::Texture => {
            let (low, high) = crate::attributes::brush_rows::TEXTURE_RADIUS;
            let shown = Shown {
                scale: 1.0,
                suffix: tr(" texels"),
                decimals: 0,
                eased: true,
            };
            fill_field(
                ui,
                tr("Brush size on the texture"),
                &mut settings.texture_radius,
                (low, high),
                shown,
            )
        }
        SizeSpace::Viewport => {
            let range = paint::brush::RADIUS_RANGE;
            let shown = Shown {
                scale: 1.0,
                suffix: " px",
                decimals: 0,
                eased: true,
            };
            fill_field(
                ui,
                tr("Brush size on screen"),
                &mut settings.brush.radius,
                (*range.start(), *range.end()),
                shown,
            )
        }
    }
    .on_hover_text(size_help);
    let percent = Shown {
        scale: 100.0,
        suffix: "%",
        decimals: 0,
        eased: false,
    };
    let brush = &mut settings.brush;
    let eased = Shown {
        eased: true,
        ..percent
    };
    for (name, label, value, range, help) in [
        (
            tr("Flow"),
            tr("Brush flow"),
            &mut brush.flow,
            (0.0, 1.0),
            tr("How much paint one stroke lays on. Ctrl + left drag (sideways) changes it"),
        ),
        (
            tr("Hardness"),
            tr("Brush hardness"),
            &mut brush.hardness,
            (0.0, 1.0),
            tr(
                "100% has a crisp edge, 0% fades from the middle. Ctrl + right drag (up and \
                down) changes it",
            ),
        ),
        (
            tr("Spacing"),
            tr("Brush spacing"),
            &mut brush.spacing,
            (0.01, 4.0),
            tr(
                "The gap between the brush's dabs, as a share of its width: small for smooth \
                lines",
            ),
        ),
    ] {
        weak(ui, name);
        // Spacing reaches 400%, but is mostly small: the bar eases.
        let shown = if range.1 > 1.0 { eased } else { percent };
        fill_field(ui, label, value, range, shown).on_hover_text(help);
    }
    settings.brush = settings.brush.clamped();
}

/// A switch as a small icon (decision 0226), with `text` after it when
/// given: lit in the accent colour when on.
fn icon_switch(
    ui: &mut Ui,
    icon: crate::icons::Icon,
    text: Option<&str>,
    on: bool,
) -> egui::Response {
    let galley = text.map(|t| {
        ui.painter().layout_no_wrap(
            t.to_string(),
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        )
    });
    let width = 24.0 + galley.as_ref().map_or(0.0, |g| g.size().x + 4.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 22.0), Sense::click());
    let accent = theme::accent(ui.ctx()).colors();
    let fill = if on {
        accent.fill
    } else if response.hovered() {
        theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    let ink = if on { Color32::WHITE } else { theme::TEXT };
    let icon_rect = Rect::from_center_size(rect.left_center() + vec2(12.0, 0.0), vec2(16.0, 16.0));
    crate::icons::paint(ui.painter(), icon_rect, icon, ink);
    if let Some(galley) = galley {
        ui.painter().galley_with_override_text_color(
            egui::pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0),
            galley,
            ink,
        );
    }
    response
}

/// The Mirror, Pen and Smoothing switches, as icons.
fn switches(ui: &mut Ui, settings: &mut BrushSettings) {
    let mirror = icon_switch(
        ui,
        crate::icons::Icon::Symmetry,
        Some(settings.mirror_axis.name()),
        settings.mirror_on,
    )
    .on_hover_text(tr(
        "Mirror painting: paint the other side of the object too, across its own axis \
         through its pivot. L turns it on and off; the arrow beside it chooses the axis",
    ));
    let mirror = switch_named(mirror, settings.mirror_on, MIRROR_SWITCH);
    if mirror.clicked() {
        settings.mirror_on = !settings.mirror_on;
    }
    egui::ComboBox::from_id_salt("paint_bar_mirror_axis")
        .selected_text("")
        .width(14.0)
        .show_ui(ui, |ui| {
            for axis in MirrorAxis::ALL {
                if ui
                    .selectable_label(settings.mirror_axis == axis, axis.described())
                    .clicked()
                {
                    settings.mirror_axis = axis;
                    settings.mirror_on = true;
                }
            }
        })
        .response
        .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, tr("Mirror axis")));
    let pen = icon_switch(
        ui,
        crate::icons::Icon::PaintPen,
        None,
        settings.size_pressure.on,
    )
    .on_hover_text(tr(
        "Pen pressure: press harder for a bigger brush. The curve and the least size are \
         in the Paint tab",
    ));
    let pen = switch_named(pen, settings.size_pressure.on, PEN_SWITCH);
    if pen.clicked() {
        settings.size_pressure.on = !settings.size_pressure.on;
    }
    let smooth = icon_switch(
        ui,
        crate::icons::Icon::PaintSmoothing,
        None,
        settings.smoothing_on,
    )
    .on_hover_text(tr(
        "Smoothing: the brush trails the mouse on a string, for steady lines. D turns it on \
         and off; how much is in the Paint tab",
    ));
    let smooth = switch_named(smooth, settings.smoothing_on, SMOOTHING_SWITCH);
    if smooth.clicked() {
        settings.smoothing_on = !settings.smoothing_on;
    }
}

/// Names a switch for screen readers (and tests), with whether it's on.
fn switch_named(response: egui::Response, on: bool, label: &'static str) -> egui::Response {
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, tr(label)));
    response
}

/// A name in the bar, quieter than the values.
fn weak(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(theme::TEXT_WEAK));
}

/// Names a field for screen readers (and tests).
fn named(response: egui::Response, label: &str) -> egui::Response {
    let label = label.to_string();
    response.widget_info(move || WidgetInfo::labeled(WidgetType::DragValue, true, &label));
    response
}

/// Where the bar would be drawn in `view`: for a quick check that a
/// point is under it.
#[cfg(test)]
pub(crate) fn origin(view: Rect, header: f32) -> Pos2 {
    Pos2::new(view.left() + INSET, view.top() + header + NAME_ROOM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_fill_rows_in_order() {
        // All fit: one row.
        assert_eq!(
            rows_of(&[100.0, 300.0, 150.0], 600.0, 6.0),
            vec![vec![0, 1, 2]]
        );
        // The last moves down.
        assert_eq!(
            rows_of(&[100.0, 300.0, 150.0], 450.0, 6.0),
            vec![vec![0, 1], vec![2]]
        );
        // Too narrow for any two: one each, even one wider than the view.
        assert_eq!(
            rows_of(&[100.0, 300.0, 150.0], 120.0, 6.0),
            vec![vec![0], vec![1], vec![2]]
        );
        // Widths not known yet (the first frame): all on one row.
        assert_eq!(rows_of(&[0.0, 0.0, 0.0], 300.0, 6.0), vec![vec![0, 1, 2]]);
        assert!(rows_of(&[], 300.0, 6.0).is_empty());
    }

    #[test]
    fn the_bar_starts_under_the_view_name() {
        let view = Rect::from_min_size(Pos2::new(10.0, 20.0), vec2(800.0, 600.0));
        let at = origin(view, 26.0);
        assert_eq!(at, Pos2::new(18.0, 68.0));
        assert!(view.contains(at));
    }
}
