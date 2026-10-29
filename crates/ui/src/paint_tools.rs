//! The paint tools along the bottom of the viewport ([decision 0213](../../../docs/decisions/0213-paint-interface-overhaul.md)):
//! in Paint mode, a floating row of buttons for the paint brush, the
//! eraser, projection, polygon fill, smudge and clone, in the look of the
//! viewport's own Shading and navigation buttons, between them. The keys
//! 1 to 6 still choose the same tools.

use crate::attributes::{BrushSettings, PaintTool};
use crate::icons::{self, Icon};
use crate::text::tr;
use crate::theme;
use egui::{Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};
use paint::BrushKind;

/// Accessible name of the row.
pub(crate) const TOOLS_LABEL: &str = crate::text!("Paint tools");

/// A button's side, as the viewport's other bottom buttons' height.
const BUTTON: f32 = 26.0;
/// The gap between buttons, and round the row inside its frame.
const PAD: f32 = 3.0;
/// The gap kept from the Shading and navigation buttons.
const GAP: f32 = 8.0;

/// One tool: what it is called, its key, its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaintToolButton {
    /// The paint brush (1).
    Brush,
    /// The eraser (2).
    Eraser,
    /// Projection (3).
    Projection,
    /// Polygon fill (4).
    PolygonFill,
    /// Smudge (5).
    Smudge,
    /// Clone (6).
    Clone,
}

impl PaintToolButton {
    /// Every tool, in key order.
    pub(crate) const ALL: [PaintToolButton; 6] = [
        Self::Brush,
        Self::Eraser,
        Self::Projection,
        Self::PolygonFill,
        Self::Smudge,
        Self::Clone,
    ];

    /// Its name, with its key.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Brush => tr("Paint brush (1)"),
            Self::Eraser => tr("Eraser (2)"),
            Self::Projection => tr("Projection (3)"),
            Self::PolygonFill => tr("Polygon fill (4)"),
            Self::Smudge => tr("Smudge (5)"),
            Self::Clone => tr("Clone (6)"),
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Brush => Icon::PaintMode,
            Self::Eraser => Icon::PaintEraser,
            Self::Projection => Icon::PaintProjection,
            Self::PolygonFill => Icon::PaintPolygonFill,
            Self::Smudge => Icon::PaintSmudge,
            Self::Clone => Icon::PaintClone,
        }
    }

    /// Whether it is the tool in use.
    pub(crate) fn is_on(self, settings: &BrushSettings) -> bool {
        let brush = settings.tool == PaintTool::Brush;
        match self {
            Self::PolygonFill => settings.tool == PaintTool::Fill,
            Self::Brush => brush && settings.brush.kind == BrushKind::Paint,
            Self::Eraser => brush && settings.brush.kind == BrushKind::Erase,
            Self::Projection => brush && settings.brush.kind == BrushKind::Projection,
            Self::Smudge => brush && settings.brush.kind == BrushKind::Smudge,
            Self::Clone => brush && settings.brush.kind == BrushKind::Clone,
        }
    }

    /// Makes it the tool in use, as its key does.
    pub(crate) fn choose(self, settings: &mut BrushSettings) {
        let kind = match self {
            Self::PolygonFill => {
                settings.tool = PaintTool::Fill;
                return;
            }
            Self::Brush => BrushKind::Paint,
            Self::Eraser => BrushKind::Erase,
            Self::Projection => BrushKind::Projection,
            Self::Smudge => BrushKind::Smudge,
            Self::Clone => BrushKind::Clone,
        };
        settings.tool = PaintTool::Brush;
        settings.brush.kind = kind;
    }
}

/// Where the row goes: between the Shading button and the navigation
/// button (`buttons`, where they were last drawn), level with them, when
/// there is room; else just above the Shading button. Without them yet,
/// at the bottom middle of `view`.
pub(crate) fn row_rect(view: Rect, buttons: Option<(Rect, Rect)>) -> Rect {
    let count = PaintToolButton::ALL.len() as f32;
    let size = vec2(count * BUTTON + (count + 1.0) * PAD, BUTTON + 2.0 * PAD);
    let Some((shading, navigation)) = buttons else {
        let at = pos2(view.center().x - size.x / 2.0, view.bottom() - 8.0 - size.y);
        return Rect::from_min_size(at, size);
    };
    let gap_left = shading.right() + GAP;
    let gap_right = navigation.left() - GAP;
    let middle_y = shading.center().y;
    if gap_right - gap_left >= size.x {
        let x = ((gap_left + gap_right) - size.x) / 2.0;
        Rect::from_center_size(pos2(x + size.x / 2.0, middle_y), size)
    } else {
        let bottom = shading.top() - GAP;
        Rect::from_min_size(pos2(shading.left(), bottom - size.y), size)
    }
}

/// Draws the row in `view` and switches the tool clicked.
pub(crate) fn show(
    ui: &mut Ui,
    view: Rect,
    buttons: Option<(Rect, Rect)>,
    settings: &mut BrushSettings,
) {
    let row = row_rect(view, buttons);
    let response = ui.interact(row, ui.id().with("paint_tools"), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(TOOLS_LABEL)));
    let accent = theme::accent(ui.ctx()).colors();
    let painter = ui.painter();
    painter.rect(
        row,
        5.0,
        theme::PALETTE,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    for (i, tool) in PaintToolButton::ALL.into_iter().enumerate() {
        let min = pos2(
            row.left() + PAD + i as f32 * (BUTTON + PAD),
            row.top() + PAD,
        );
        let rect = Rect::from_min_size(min, vec2(BUTTON, BUTTON));
        let on = tool.is_on(settings);
        let button = ui.interact(rect, ui.id().with(("paint_tool", i)), Sense::click());
        button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, tool.label()));
        let fill = if on {
            accent.fill
        } else if button.hovered() {
            theme::HOVER
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 4.0, fill);
        let ink = if on {
            egui::Color32::WHITE
        } else {
            theme::TEXT
        };
        icons::paint(ui.painter(), rect.shrink(4.0), tool.icon(), ink);
        if button.on_hover_text(tool.label()).clicked() {
            tool.choose(settings);
        }
    }
}

/// The tools down the left bar, under the modes, when Settings puts them
/// there: the mode buttons' size and look.
pub(crate) fn column(ui: &mut Ui, settings: &mut BrushSettings) {
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add_space(4.0);
        crate::palette::divider_line(ui);
        let response = ui.label("");
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(COLUMN_LABEL)));
        for tool in PaintToolButton::ALL {
            let state = if tool.is_on(settings) {
                crate::palette::ButtonState::Active
            } else {
                crate::palette::ButtonState::Normal
            };
            if crate::palette::icon_button(ui, tool.icon(), tool.label(), tool.label(), state)
                .clicked()
            {
                tool.choose(settings);
            }
        }
    });
}

/// Accessible name of the left bar's tools.
pub(crate) const COLUMN_LABEL: &str = crate::text!("Paint tools in the left bar");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_button_chooses_its_tool_and_shows_it_on() {
        let mut settings = BrushSettings::default();
        for tool in PaintToolButton::ALL {
            tool.choose(&mut settings);
            for other in PaintToolButton::ALL {
                assert_eq!(other.is_on(&settings), other == tool, "{tool:?} {other:?}");
            }
        }
    }

    #[test]
    fn an_effect_brush_without_a_button_lights_none() {
        let mut settings = BrushSettings {
            tool: PaintTool::Brush,
            ..Default::default()
        };
        settings.brush.kind = BrushKind::Blur;
        assert!(PaintToolButton::ALL.iter().all(|t| !t.is_on(&settings)));
    }

    #[test]
    fn the_row_sits_between_the_bottom_buttons_or_above_them() {
        let view = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0));
        let shading = Rect::from_min_size(pos2(82.0, 566.0), vec2(90.0, 26.0));
        let navigation = Rect::from_min_size(pos2(860.0, 566.0), vec2(130.0, 26.0));
        let row = row_rect(view, Some((shading, navigation)));
        assert!(row.left() > shading.right() && row.right() < navigation.left());
        assert!((row.center().y - shading.center().y).abs() < 0.01, "level");
        // No room between them: above the Shading button.
        let close = Rect::from_min_size(pos2(200.0, 566.0), vec2(130.0, 26.0));
        let row = row_rect(view, Some((shading, close)));
        assert!(row.bottom() < shading.top());
        assert_eq!(row.left(), shading.left());
        // Before the buttons are known: the bottom middle.
        let row = row_rect(view, None);
        assert!(view.contains_rect(row));
        assert!((row.center().x - 500.0).abs() < 0.01);
    }
}
