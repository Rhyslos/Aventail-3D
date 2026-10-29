//! The paint tools along the bottom of the painting area ([decision 0213](../../../docs/decisions/0213-paint-interface-overhaul.md), [0215](../../../docs/decisions/0215-one-painting-area.md)):
//! in Paint mode, a floating row of buttons for the paint brush, the
//! eraser, projection, polygon fill, smudge and clone, in the look of the
//! viewport's own Shading and navigation buttons, level with them. The keys
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

/// The row's size.
fn row_size() -> egui::Vec2 {
    let count = PaintToolButton::ALL.len() as f32;
    vec2(count * BUTTON + (count + 1.0) * PAD, BUTTON + 2.0 * PAD)
}

/// Where the row goes in the painting `area` (decision 0215): along the
/// bottom, level with what is drawn there (`bottom`: the viewport's
/// Shading and navigation buttons, the UV Editor's notes), in the free
/// stretch nearest the area's middle, when one is wide enough; else just
/// above the leftmost of them. With nothing along the bottom yet, at the
/// bottom middle.
pub(crate) fn row_rect(area: Rect, bottom: &[Rect]) -> Rect {
    let size = row_size();
    let Some(first) = bottom.iter().min_by(|a, b| a.left().total_cmp(&b.left())) else {
        let at = pos2(area.center().x - size.x / 2.0, area.bottom() - 8.0 - size.y);
        return Rect::from_min_size(at, size);
    };
    let middle_y = first.center().y;
    let band = Rect::from_center_size(pos2(area.center().x, middle_y), vec2(area.width(), size.y));
    // The stretches along the bottom free of anything drawn there.
    let mut taken: Vec<(f32, f32)> = bottom
        .iter()
        .filter(|r| r.top() < band.bottom() && r.bottom() > band.top())
        .map(|r| (r.left() - GAP, r.right() + GAP))
        .collect();
    taken.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut free = Vec::new();
    let mut from = area.left() + GAP;
    for (l, r) in taken {
        if l > from {
            free.push((from, l));
        }
        from = from.max(r);
    }
    if area.right() - GAP > from {
        free.push((from, area.right() - GAP));
    }
    let centre = area.center().x;
    let best = free
        .into_iter()
        .filter(|(l, r)| r - l >= size.x)
        .map(|(l, r)| {
            let x = centre.clamp(l + size.x / 2.0, r - size.x / 2.0);
            (x, (x - centre).abs())
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
    match best {
        Some((x, _)) => Rect::from_center_size(pos2(x, middle_y), size),
        None => Rect::from_min_size(pos2(first.left(), first.top() - GAP - size.y), size),
    }
}

/// Draws the row over the painting `area`, above both views, and switches
/// the tool clicked.
pub(crate) fn show(ctx: &egui::Context, area: Rect, bottom: &[Rect], settings: &mut BrushSettings) {
    let row = row_rect(area, bottom);
    egui::Area::new(egui::Id::new("paint_tools"))
        .order(egui::Order::Middle)
        .fixed_pos(row.min)
        .show(ctx, |ui| draw_row(ui, row, settings));
}

fn draw_row(ui: &mut Ui, row: Rect, settings: &mut BrushSettings) {
    let response = ui.allocate_rect(row, Sense::hover());
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
        let row = row_rect(view, &[shading, navigation]);
        assert!(row.left() > shading.right() && row.right() < navigation.left());
        assert!((row.center().y - shading.center().y).abs() < 0.01, "level");
        assert!((row.center().x - 500.0).abs() < 0.01, "in the middle");
        // No room between them: above the Shading button.
        let close = Rect::from_min_size(pos2(200.0, 566.0), vec2(130.0, 26.0));
        let narrow = Rect::from_min_size(pos2(0.0, 0.0), vec2(340.0, 600.0));
        let row = row_rect(narrow, &[shading, close]);
        assert!(row.bottom() < shading.top());
        assert_eq!(row.left(), shading.left());
        // Before the buttons are known: the bottom middle.
        let row = row_rect(view, &[]);
        assert!(view.contains_rect(row));
        assert!((row.center().x - 500.0).abs() < 0.01);
    }

    #[test]
    fn across_two_views_the_row_takes_the_free_stretch_nearest_the_middle() {
        // The viewport (0 to 560) and the UV Editor (564 to 1100): Shading
        // and navigation in the viewport, the notes in the UV Editor.
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(1100.0, 600.0));
        let shading = Rect::from_min_size(pos2(82.0, 566.0), vec2(90.0, 26.0));
        let navigation = Rect::from_min_size(pos2(440.0, 566.0), vec2(112.0, 26.0));
        let pieces = Rect::from_min_size(pos2(574.0, 574.0), vec2(130.0, 16.0));
        let density = Rect::from_min_size(pos2(910.0, 574.0), vec2(180.0, 16.0));
        let bottom = [shading, navigation, pieces, density];
        let row = row_rect(area, &bottom);
        for b in bottom {
            assert!(!row.intersects(b), "{row:?} covers {b:?}");
        }
        assert!(area.contains_rect(row));
        // The middle (550) is taken; the nearest wide enough stretch is
        // between the navigation button and the pieces note? Too narrow
        // (552 to 574): it goes to the UV Editor's or the viewport's gap,
        // whichever reaches nearer the middle.
        assert!(row.right() <= navigation.left() - GAP || row.left() >= pieces.right() + GAP);
        // Wide open bottom: right in the middle, over both views.
        let row = row_rect(area, &[shading, density]);
        assert!((row.center().x - 550.0).abs() < 0.01);
    }
}
