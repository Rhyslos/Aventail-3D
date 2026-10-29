//! The paint tools along the bottom of the painting area ([decision 0213](../../../docs/decisions/0213-paint-interface-overhaul.md), [0215](../../../docs/decisions/0215-one-painting-area.md),
//! [0224](../../../docs/decisions/0224-paint-tool-row.md)): in Paint mode, a floating row right after the
//! view's controls, with select, move, rotate and scale (adjustments while
//! texturing), then the paint brush, the eraser, projection, polygon fill,
//! smudge, blur, clone, the bucket and the colour picker. The keys 1 to 8
//! and P choose the same paint tools; Q, W, R and S the others.

use crate::attributes::{BrushSettings, PaintTool};
use crate::icons::{self, Icon};
use crate::text::tr;
use crate::theme;
use crate::transforming::Tool;
use egui::{Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};
use paint::BrushKind;

/// Accessible name of the row.
pub(crate) const TOOLS_LABEL: &str = crate::text!("Paint tools");

/// A button's side, as the viewport's other bottom buttons' height.
const BUTTON: f32 = 26.0;
/// The gap between buttons, and round the row inside its frame.
const PAD: f32 = 3.0;
/// The gap kept from the Shading and navigation buttons, and from the
/// view's controls.
const GAP: f32 = 8.0;
/// The gap between the transform tools and the paint tools.
const DIVIDE: f32 = 9.0;

/// What the row needs of the workbench besides the brush: the tool in
/// use (select, move, rotate, scale), whether the select arrow is on
/// instead of a paint tool, and whether a click picks a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hands {
    /// The workbench's tool.
    pub(crate) tool: Tool,
    /// The select arrow is on: clicks select instead of painting.
    pub(crate) selecting: bool,
    /// The next click picks a colour.
    pub(crate) picking: bool,
}

/// One tool: what it is called, its key, its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaintToolButton {
    /// Selecting objects (Q).
    Select,
    /// Moving them (W).
    Move,
    /// Rotating them (R).
    Rotate,
    /// Scaling them (S).
    Scale,
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
    /// Blur (7).
    Blur,
    /// The bucket (8): fills the whole layer.
    Bucket,
    /// The colour picker (P).
    Picker,
}

impl PaintToolButton {
    /// Every tool, in the row's order.
    pub(crate) const ALL: [PaintToolButton; 13] = [
        Self::Select,
        Self::Move,
        Self::Rotate,
        Self::Scale,
        Self::Brush,
        Self::Eraser,
        Self::Projection,
        Self::PolygonFill,
        Self::Smudge,
        Self::Blur,
        Self::Clone,
        Self::Bucket,
        Self::Picker,
    ];

    /// Its name, with its key.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Select => tr("Select (Q)"),
            Self::Move => tr("Move (W)"),
            Self::Rotate => tr("Rotate (R)"),
            Self::Scale => tr("Scale (S)"),
            Self::Brush => tr("Paint brush (1)"),
            Self::Eraser => tr("Eraser (2)"),
            Self::Projection => tr("Projection (3)"),
            Self::PolygonFill => tr("Polygon fill (4)"),
            Self::Smudge => tr("Smudge (5)"),
            Self::Clone => tr("Clone (6)"),
            Self::Blur => tr("Blur (7)"),
            Self::Bucket => tr("Fill the layer (8)"),
            Self::Picker => tr("Pick a colour (P)"),
        }
    }

    /// What it does, for its tooltip.
    pub(crate) fn explanation(self) -> &'static str {
        match self {
            Self::Select => tr("Click an object to choose what you paint; nothing is painted"),
            Self::Move | Self::Rotate | Self::Scale => {
                tr("Adjust the selected objects with the gizmo; nothing is painted")
            }
            Self::Bucket => tr(
                "Click the model to fill the chosen layer with the colour, everywhere on that \
                 material",
            ),
            Self::Picker => tr("Click the model to take the colour under the mouse"),
            Self::Blur => tr("Soften what is under the brush a little each pass"),
            _ => "",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Select => Icon::BoxSelection,
            Self::Move => Icon::Move,
            Self::Rotate => Icon::Rotate,
            Self::Scale => Icon::Scale,
            Self::Brush => Icon::PaintMode,
            Self::Eraser => Icon::PaintEraser,
            Self::Projection => Icon::PaintProjection,
            Self::PolygonFill => Icon::PaintPolygonFill,
            Self::Smudge => Icon::PaintSmudge,
            Self::Clone => Icon::PaintClone,
            Self::Blur => Icon::PaintBlur,
            Self::Bucket => Icon::PaintBucket,
            Self::Picker => Icon::PaintPicker,
        }
    }

    /// Whether it is the tool in use.
    pub(crate) fn is_on(self, settings: &BrushSettings, hands: Hands) -> bool {
        let painting = hands.tool == Tool::Select && !hands.selecting && !hands.picking;
        let brush = painting && settings.tool == PaintTool::Brush;
        let fill = painting && settings.tool == PaintTool::Fill;
        match self {
            Self::Select => hands.tool == Tool::Select && hands.selecting,
            Self::Move => hands.tool == Tool::Move,
            Self::Rotate => hands.tool == Tool::Rotate,
            Self::Scale => hands.tool == Tool::Scale,
            Self::Picker => hands.picking,
            Self::PolygonFill => fill && !settings.bucket,
            Self::Bucket => fill && settings.bucket,
            Self::Brush => brush && settings.brush.kind == BrushKind::Paint,
            Self::Eraser => brush && settings.brush.kind == BrushKind::Erase,
            Self::Projection => brush && settings.brush.kind == BrushKind::Projection,
            Self::Smudge => brush && settings.brush.kind == BrushKind::Smudge,
            Self::Clone => brush && settings.brush.kind == BrushKind::Clone,
            Self::Blur => brush && settings.brush.kind == BrushKind::Blur,
        }
    }

    /// Makes it the tool in use, as its key does. The workbench's tool and
    /// the select arrow are the caller's to set from `hands` afterwards.
    pub(crate) fn choose(self, settings: &mut BrushSettings, hands: &mut Hands) {
        hands.picking = false;
        hands.selecting = false;
        match self {
            Self::Select => {
                hands.tool = Tool::Select;
                hands.selecting = true;
                return;
            }
            Self::Move => {
                hands.tool = Tool::Move;
                return;
            }
            Self::Rotate => {
                hands.tool = Tool::Rotate;
                return;
            }
            Self::Scale => {
                hands.tool = Tool::Scale;
                return;
            }
            _ => hands.tool = Tool::Select,
        }
        let kind = match self {
            Self::PolygonFill => {
                settings.choose_fill();
                return;
            }
            Self::Bucket => {
                settings.choose_bucket();
                return;
            }
            Self::Picker => {
                hands.picking = true;
                return;
            }
            Self::Eraser => BrushKind::Erase,
            Self::Projection => BrushKind::Projection,
            Self::Smudge => BrushKind::Smudge,
            Self::Clone => BrushKind::Clone,
            Self::Blur => BrushKind::Blur,
            _ => BrushKind::Paint,
        };
        settings.tool = PaintTool::Brush;
        settings.brush.kind = kind;
    }
}

/// The row's size: the transform tools, a gap, the paint tools.
fn row_size() -> egui::Vec2 {
    let count = PaintToolButton::ALL.len() as f32;
    vec2(
        count * BUTTON + (count + 1.0) * PAD + DIVIDE,
        BUTTON + 2.0 * PAD,
    )
}

/// Where the row goes right after the view's controls (`after`, decision
/// 0224), level with them, when it fits in the painting `area` clear of
/// what else is along the bottom (`bottom`: the UV Editor's notes); else
/// just above them.
pub(crate) fn row_after(area: Rect, after: Rect, bottom: &[Rect]) -> Rect {
    let size = row_size();
    let beside = Rect::from_min_size(pos2(after.right() + GAP * 0.75, after.top()), size);
    let clear = bottom
        .iter()
        .all(|b| *b == after || !b.expand(GAP / 2.0).intersects(beside));
    if beside.right() <= area.right() - GAP && clear {
        beside
    } else {
        Rect::from_min_size(pos2(after.left(), after.top() - GAP * 0.75 - size.y), size)
    }
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

/// Draws the row over the painting `area`, above both views: right after
/// the view's controls (`after`) when they are drawn, otherwise in the
/// free stretch along the bottom. Switches the tool clicked.
pub(crate) fn show(
    ctx: &egui::Context,
    area: Rect,
    bottom: &[Rect],
    after: Option<Rect>,
    settings: &mut BrushSettings,
    hands: &mut Hands,
) {
    let row = match after {
        Some(after) => row_after(area, after, bottom),
        None => row_rect(area, bottom),
    };
    egui::Area::new(egui::Id::new("paint_tools"))
        .order(egui::Order::Middle)
        .fixed_pos(row.min)
        .show(ctx, |ui| draw_row(ui, row, settings, hands));
}

fn draw_row(ui: &mut Ui, row: Rect, settings: &mut BrushSettings, hands: &mut Hands) {
    let response = ui.allocate_rect(row, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(TOOLS_LABEL)));
    let accent = theme::accent(ui.ctx()).colors();
    crate::viewport::paint_group_frame(ui.painter(), row);
    let mut x = row.left() + PAD;
    for (i, tool) in PaintToolButton::ALL.into_iter().enumerate() {
        // A faint line between the transform tools and the paint tools.
        if tool == PaintToolButton::Brush {
            let line_x = x + DIVIDE / 2.0 - PAD / 2.0;
            ui.painter().vline(
                line_x,
                (row.top() + 7.0)..=(row.bottom() - 7.0),
                Stroke::new(1.0, theme::BORDER),
            );
            x += DIVIDE;
        }
        let rect = Rect::from_min_size(pos2(x, row.top() + PAD), vec2(BUTTON, BUTTON));
        x += BUTTON + PAD;
        let on = tool.is_on(settings, *hands);
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
        let tip = match tool.explanation() {
            "" => tool.label().to_string(),
            more => format!("{}\n{more}", tool.label()),
        };
        if button.on_hover_text(tip).clicked() {
            tool.choose(settings, hands);
        }
    }
}

/// The tools down the left bar, under the modes, when Settings puts them
/// there: the mode buttons' size and look; the transform tools, a line,
/// then the paint tools.
pub(crate) fn column(ui: &mut Ui, settings: &mut BrushSettings, hands: &mut Hands) {
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add_space(4.0);
        let response = ui.label("");
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(COLUMN_LABEL)));
        for tool in PaintToolButton::ALL {
            if tool == PaintToolButton::Brush {
                crate::palette::divider_line(ui);
            }
            let state = if tool.is_on(settings, *hands) {
                crate::palette::ButtonState::Active
            } else {
                crate::palette::ButtonState::Normal
            };
            if crate::palette::icon_button(ui, tool.icon(), tool.label(), tool.label(), state)
                .clicked()
            {
                tool.choose(settings, hands);
            }
        }
    });
}

/// Accessible name of the left bar's tools.
pub(crate) const COLUMN_LABEL: &str = crate::text!("Paint tools in the left bar");

#[cfg(test)]
mod tests {
    use super::*;

    fn hands() -> Hands {
        Hands {
            tool: Tool::Select,
            selecting: false,
            picking: false,
        }
    }

    #[test]
    fn each_button_chooses_its_tool_and_shows_it_on() {
        let mut settings = BrushSettings::default();
        let mut hands = hands();
        for tool in PaintToolButton::ALL {
            tool.choose(&mut settings, &mut hands);
            for other in PaintToolButton::ALL {
                assert_eq!(
                    other.is_on(&settings, hands),
                    other == tool,
                    "{tool:?} {other:?}"
                );
            }
        }
    }

    #[test]
    fn a_paint_tool_puts_the_transform_tools_down_and_back() {
        let mut settings = BrushSettings::default();
        let mut hands = hands();
        PaintToolButton::Move.choose(&mut settings, &mut hands);
        assert_eq!(hands.tool, Tool::Move);
        PaintToolButton::Smudge.choose(&mut settings, &mut hands);
        assert_eq!(hands.tool, Tool::Select);
        assert!(!hands.selecting && !hands.picking);
        // The bucket and polygon fill are both fills, told apart.
        PaintToolButton::Bucket.choose(&mut settings, &mut hands);
        assert!(settings.tool == PaintTool::Fill && settings.bucket);
        PaintToolButton::PolygonFill.choose(&mut settings, &mut hands);
        assert!(settings.tool == PaintTool::Fill && !settings.bucket);
    }

    #[test]
    fn an_effect_brush_without_a_button_lights_none() {
        let mut settings = BrushSettings {
            tool: PaintTool::Brush,
            ..Default::default()
        };
        settings.brush.kind = BrushKind::HueShift;
        assert!(
            PaintToolButton::ALL
                .iter()
                .all(|t| !t.is_on(&settings, hands()))
        );
    }

    #[test]
    fn the_row_follows_the_view_controls_or_goes_above_them() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(1100.0, 600.0));
        let controls = Rect::from_min_size(pos2(10.0, 558.0), vec2(200.0, 32.0));
        let row = row_after(area, controls, &[controls]);
        assert!(row.left() > controls.right() && row.left() < controls.right() + GAP);
        assert_eq!(row.top(), controls.top(), "level");
        assert!(area.contains_rect(row));
        // A narrow area: above the controls.
        let narrow = Rect::from_min_size(pos2(0.0, 0.0), vec2(420.0, 600.0));
        let row = row_after(narrow, controls, &[controls]);
        assert!(row.bottom() < controls.top());
        assert_eq!(row.left(), controls.left());
        // A note in the way (the UV Editor's): above too.
        let note = Rect::from_min_size(pos2(400.0, 566.0), vec2(120.0, 16.0));
        let row = row_after(area, controls, &[controls, note]);
        assert!(row.bottom() < controls.top());
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
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(2200.0, 600.0));
        let shading = Rect::from_min_size(pos2(164.0, 566.0), vec2(180.0, 26.0));
        let navigation = Rect::from_min_size(pos2(880.0, 566.0), vec2(224.0, 26.0));
        let pieces = Rect::from_min_size(pos2(1148.0, 574.0), vec2(260.0, 16.0));
        let density = Rect::from_min_size(pos2(1820.0, 574.0), vec2(360.0, 16.0));
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
        assert!((row.center().x - 1100.0).abs() < 0.01);
    }
}
