//! The layout buttons on the right of the menu bar (decision 0265): one
//! bar of bigger buttons, each with a picture and the layout's name, the
//! one in use filled with the accent colour. The pictures are the modes'
//! own icons (each layout goes with a mode), drawn in code like every
//! other icon in the app.
//!
//! The small text tabs the app had before are kept as a setting
//! (`LayoutSwitch::SmallTabs`) so the project lead can compare the two;
//! they are drawn in `layouts.rs`.

use super::layouts::LayoutChoice;
use crate::icons::{self, Icon};
use crate::theme;
use egui::{
    Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    WidgetType, vec2,
};

/// How tall each button is, in points.
const BUTTON_HEIGHT: f32 = 24.0;
/// The space round the buttons inside their bar.
const BAR_MARGIN: f32 = 2.0;
/// How tall the whole bar is: the menu bar grows to fit it.
pub(super) const BAR_HEIGHT: f32 = BUTTON_HEIGHT + 2.0 * BAR_MARGIN;
/// The picture's size.
const ICON_SIZE: f32 = 16.0;
/// The space between the picture and the name.
const ICON_GAP: f32 = 6.0;
/// The space on each side of a button's contents.
const PADDING: f32 = 10.0;
/// The space between two buttons.
const GAP: f32 = 2.0;
/// The name's size.
const TEXT_SIZE: f32 = 13.0;

/// The picture on a layout's button: the icon of the mode it works in.
pub(super) fn icon_of(choice: LayoutChoice) -> Icon {
    match choice {
        LayoutChoice::Standard => Icon::Cube,
        LayoutChoice::UvEdit => Icon::Uv,
        LayoutChoice::Paint => Icon::PaintMode,
        LayoutChoice::Material => Icon::MaterialMode,
    }
}

/// One button: its name, the layout (or, for one not built yet, when it
/// arrives) and what it is for.
pub(super) type Entry<'a> = &'a (&'a str, Result<LayoutChoice, &'a str>, &'a str);

/// Draws the bar of layout buttons, left to right, with `current` filled
/// in the accent colour. Returns the layout clicked, if another.
pub(super) fn layout_buttons(
    ui: &mut Ui,
    entries: &[Entry<'_>],
    current: LayoutChoice,
) -> Option<LayoutChoice> {
    let font = FontId::new(TEXT_SIZE, egui::FontFamily::Proportional);
    let widths: Vec<f32> = entries
        .iter()
        .map(|(name, _, _)| {
            let text = ui
                .painter()
                .layout_no_wrap(name.to_string(), font.clone(), theme::TEXT)
                .size()
                .x;
            PADDING + ICON_SIZE + ICON_GAP + text + PADDING
        })
        .collect();
    let gaps = GAP * entries.len().saturating_sub(1) as f32;
    let width = widths.iter().sum::<f32>() + gaps + 2.0 * BAR_MARGIN;
    let (bar, _) = ui.allocate_exact_size(vec2(width, BAR_HEIGHT), Sense::hover());
    ui.painter().rect(
        bar,
        CornerRadius::same(5),
        theme::FIELD,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
    let mut picked = None;
    let mut left = bar.left() + BAR_MARGIN;
    for (entry, width) in entries.iter().zip(widths) {
        let rect = Rect::from_min_size(
            egui::pos2(left, bar.top() + BAR_MARGIN),
            vec2(width, BUTTON_HEIGHT),
        );
        left += width + GAP;
        if let Some(choice) = button(ui, rect, entry, current, &font) {
            picked = Some(choice);
        }
    }
    picked
}

/// One layout's button in `rect`. Returns its layout if clicked while
/// another is in use.
fn button(
    ui: &mut Ui,
    rect: Rect,
    (name, layout, help): Entry<'_>,
    current: LayoutChoice,
    font: &FontId,
) -> Option<LayoutChoice> {
    let id = egui::Id::new("layout_button").with(*name);
    let choice = match layout {
        Ok(choice) => *choice,
        Err(when) => {
            // Not built yet: greyed out, saying when it arrives.
            let response = ui.interact(rect, id, Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, false, *name));
            let faded = theme::TEXT_WEAK.gamma_multiply(0.5);
            contents(ui, rect, None, name, font, faded);
            response.on_hover_text(*when);
            return None;
        }
    };
    let in_use = choice == current;
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, in_use, *name));
    let accent = theme::accent(ui.ctx()).colors();
    let painter = ui.painter();
    let (fill, color) = if in_use {
        (accent.fill, theme::TEXT_STRONG)
    } else if response.hovered() {
        (theme::HOVER, theme::TEXT_STRONG)
    } else {
        (Color32::TRANSPARENT, theme::TEXT)
    };
    painter.rect_filled(rect, CornerRadius::same(4), fill);
    contents(ui, rect, Some(choice), name, font, color);
    let clicked = response.clicked();
    if in_use {
        response.on_hover_text(crate::trf!("{} (in use)", help));
        None
    } else {
        response.on_hover_text(*help);
        clicked.then_some(choice)
    }
}

/// A button's picture and name, side by side in the middle of `rect`.
fn contents(
    ui: &Ui,
    rect: Rect,
    choice: Option<LayoutChoice>,
    name: &str,
    font: &FontId,
    color: Color32,
) {
    let painter = ui.painter();
    let icon = Rect::from_center_size(
        egui::pos2(rect.left() + PADDING + ICON_SIZE / 2.0, rect.center().y),
        vec2(ICON_SIZE, ICON_SIZE),
    );
    if let Some(choice) = choice {
        icons::paint(painter, icon, icon_of(choice), color);
    }
    painter.text(
        egui::pos2(icon.right() + ICON_GAP, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        font.clone(),
        color,
    );
}
