//! The floating brush bar ([decision 0213](../../../docs/decisions/0213-paint-interface-overhaul.md)):
//! in Paint mode, the brush settings changed all the time float over the
//! top of the viewport, in the style of the viewport's own buttons: the
//! colour square (its picker a pop-up), the brush's name, Size, Flow,
//! Hardness and Spacing, and switches for Mirror, the pen's pressure and
//! smoothing. Research into texture painters found this in all of them:
//! the few numbers you change every minute sit over the view, not in a
//! panel.

use crate::attributes::{AttributesPanel, BrushSettings, MirrorAxis, PaintTool};
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
/// Accessible name of the bar itself.
pub(crate) const BAR_LABEL: &str = crate::text!("Brush bar");

/// How far the bar sits from the view's left and from under its header.
const INSET: f32 = 8.0;
/// Room left at the top for the view's name, drawn there by the view.
pub(crate) const NAME_ROOM: f32 = 22.0;
/// The colour square's side.
const SQUARE: f32 = 20.0;
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
                                    0 => name_group(ui, attributes, palettes),
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

/// The colour square (for tools that paint a colour) and the tool's name.
fn name_group(ui: &mut Ui, attributes: &mut AttributesPanel, palettes: &[paint::palette::Palette]) {
    let fill = attributes.brush.tool == PaintTool::Fill;
    if fill || attributes.brush.brush.kind == BrushKind::Paint {
        colour_square(ui, attributes, palettes);
    }
    let name = if fill {
        tr("Polygon fill")
    } else {
        crate::attributes::kind_name(attributes.brush.brush.kind)
    };
    ui.label(RichText::new(name).color(theme::TEXT))
        .on_hover_text(tr(
            "The tool in use. Ready-made brushes are in the brush shelf under the views",
        ));
}

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

/// The brush's colour as a square; a click opens the colour picker under
/// it.
fn colour_square(
    ui: &mut Ui,
    attributes: &mut AttributesPanel,
    palettes: &[paint::palette::Palette],
) {
    let [r, g, b, _] = attributes.brush.brush.color;
    let (rect, response) = ui.allocate_exact_size(vec2(SQUARE, SQUARE), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tr(COLOUR_LABEL)));
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
                &mut attributes.picking,
                &mut attributes.palette_requests,
                false,
            );
            attributes.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        });
}

/// Size, Flow, Hardness and Spacing, as small drag fields.
fn numbers(ui: &mut Ui, settings: &mut BrushSettings) {
    weak(ui, tr("Size"));
    if settings.on_surface {
        let (low, high) = crate::attributes::BRUSH_SURFACE_RADIUS;
        let field = egui::DragValue::new(&mut settings.surface_radius)
            .range(low..=high)
            .speed(0.1)
            .max_decimals(2)
            .suffix(" cm");
        named(ui.add(field), tr("Brush size on the surface"))
    } else {
        let range = paint::brush::RADIUS_RANGE;
        let field = egui::DragValue::new(&mut settings.brush.radius)
            .range(*range.start()..=*range.end())
            .speed(0.5)
            .max_decimals(0)
            .suffix(" px");
        named(ui.add(field), tr("Brush size on screen"))
    }
    .on_hover_text(tr(
        "The brush's radius: drag or type. [ and ] change it, as does Ctrl + right drag \
         (sideways). Measured on screen or on the surface: see the Paint tab",
    ));
    let brush = &mut settings.brush;
    for (name, label, value, (low, high), help) in [
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
        percent(ui, value, (low, high), label).on_hover_text(help);
    }
    settings.brush = settings.brush.clamped();
}

/// The Mirror, Pen and Smoothing switches.
fn switches(ui: &mut Ui, settings: &mut BrushSettings) {
    let mirror = ui
        .add(egui::Button::selectable(
            settings.mirror_on,
            crate::trf!("Mirror {}", settings.mirror_axis.name()),
        ))
        .on_hover_text(tr(
            "Paint the other side of the object too, across its own axis through its pivot. \
             L turns it on and off; the arrow beside it chooses the axis",
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
                    .selectable_label(
                        settings.mirror_axis == axis,
                        crate::trf!("Across {}", axis.name()),
                    )
                    .clicked()
                {
                    settings.mirror_axis = axis;
                    settings.mirror_on = true;
                }
            }
        })
        .response
        .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, tr("Mirror axis")));
    let pen = ui
        .add(egui::Button::selectable(
            settings.size_pressure.on,
            tr("Pen"),
        ))
        .on_hover_text(tr(
            "A pen's pressure changes the brush's size: press harder for a bigger brush. \
             The curve and the least size are in the Paint tab",
        ));
    let pen = switch_named(pen, settings.size_pressure.on, PEN_SWITCH);
    if pen.clicked() {
        settings.size_pressure.on = !settings.size_pressure.on;
    }
    let smooth = ui
        .add(egui::Button::selectable(
            settings.smoothing_on,
            tr("Smoothing"),
        ))
        .on_hover_text(tr(
            "Smooth strokes: the brush trails the mouse on a string, for steady lines. \
             D turns it on and off; how much is in the Paint tab",
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

/// A share from 0 to 1 as a percentage drag field.
fn percent(ui: &mut Ui, value: &mut f32, (low, high): (f32, f32), label: &str) -> egui::Response {
    let mut shown = *value * 100.0;
    let response = ui.add(
        egui::DragValue::new(&mut shown)
            .range(low * 100.0..=high * 100.0)
            .speed(0.5)
            .max_decimals(0)
            .suffix("%"),
    );
    if response.changed() {
        *value = (shown / 100.0).clamp(low, high);
    }
    named(response, label)
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
