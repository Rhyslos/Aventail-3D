//! Value fields with step arrows: drag the number, click and type one, or
//! click the small arrows beside it to nudge it up or down a step (Shift
//! for ten steps), for fine tuning.

use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{DragValue, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

/// Width of the number part of a field.
pub(super) const NUMBER_WIDTH: f32 = 110.0;
/// Height of a field.
const HEIGHT: f32 = 20.0;

/// What happened to a field this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Touched {
    /// The value changed.
    pub(crate) changed: bool,
    /// A new drag, typing or click began: a new undo step.
    pub(crate) began: bool,
}

/// A length field (stored in centimetres, shown in `units`), with step
/// arrows. Kept between `min` and `max` centimetres.
pub(crate) fn length(
    ui: &mut Ui,
    cm: &mut f32,
    (min, max): (f32, f32),
    units: Units,
    label: &str,
) -> Touched {
    let mut shown = units.from_cm(*cm);
    let range = units.from_cm(min)..=units.from_cm(max);
    let mut touched = Touched::default();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let response = ui.add_sized(
            vec2(NUMBER_WIDTH, HEIGHT),
            DragValue::new(&mut shown)
                .range(range.clone())
                .speed(units.drag_speed())
                .max_decimals(units.decimals())
                .custom_formatter(move |value, _| units.number(value))
                .suffix(format!(" {}", units.symbol())),
        );
        touched = touched_by(&response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, label));
        let steps = arrows(ui, label);
        if steps != 0 {
            shown = (shown + units.step() * f64::from(steps)).clamp(*range.start(), *range.end());
            touched = Touched {
                changed: true,
                began: true,
            };
        }
    });
    if touched.changed {
        *cm = units.to_cm(shown);
    }
    touched
}

/// An angle field in degrees with step arrows (a degree a step), kept
/// between `low` and `high`.
pub(super) fn degrees(
    ui: &mut Ui,
    value: &mut f32,
    (low, high): (f32, f32),
    label: &str,
) -> Touched {
    let mut shown = f64::from(*value);
    let mut touched = Touched::default();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let response = ui.add_sized(
            vec2(NUMBER_WIDTH, HEIGHT),
            DragValue::new(&mut shown)
                .range(f64::from(low)..=f64::from(high))
                .speed(0.5)
                .max_decimals(1)
                .suffix("°"),
        );
        touched = touched_by(&response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, label));
        let steps = arrows(ui, label);
        if steps != 0 {
            shown = (shown + f64::from(steps)).clamp(f64::from(low), f64::from(high));
            touched = Touched {
                changed: true,
                began: true,
            };
        }
    });
    if touched.changed {
        *value = shown as f32;
    }
    touched
}

/// A share shown as a percentage with step arrows (a percent a step),
/// kept between `low` and `high` (as shares: 1 is 100%).
pub(super) fn percent(
    ui: &mut Ui,
    value: &mut f32,
    (low, high): (f32, f32),
    label: &str,
) -> Touched {
    let mut shown = f64::from(*value) * 100.0;
    let (low, high) = (f64::from(low) * 100.0, f64::from(high) * 100.0);
    let mut touched = Touched::default();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let response = ui.add_sized(
            vec2(NUMBER_WIDTH, HEIGHT),
            DragValue::new(&mut shown)
                .range(low..=high)
                .speed(0.5)
                .max_decimals(1)
                .suffix("%"),
        );
        touched = touched_by(&response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, label));
        let steps = arrows(ui, label);
        if steps != 0 {
            shown = (shown + f64::from(steps)).clamp(low, high);
            touched = Touched {
                changed: true,
                began: true,
            };
        }
    });
    if touched.changed {
        *value = (shown / 100.0) as f32;
    }
    touched
}

/// A whole-number field with step arrows, kept between `low` and `high`.
pub(super) fn count(ui: &mut Ui, value: &mut u32, (low, high): (u32, u32), label: &str) -> Touched {
    let mut touched = Touched::default();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let response = ui.add_sized(
            vec2(NUMBER_WIDTH, HEIGHT),
            DragValue::new(value).range(low..=high).speed(0.05),
        );
        touched = touched_by(&response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, label));
        let steps = arrows(ui, label);
        if steps != 0 {
            let moved = i64::from(*value) + i64::from(steps);
            *value = u32::try_from(moved.clamp(i64::from(low), i64::from(high))).unwrap_or(low);
            touched = Touched {
                changed: true,
                began: true,
            };
        }
    });
    touched
}

fn touched_by(response: &egui::Response) -> Touched {
    Touched {
        changed: response.changed(),
        began: response.drag_started() || response.gained_focus() || response.clicked(),
    }
}

/// Two small arrows stacked beside a field: up adds a step, down takes
/// one away (ten with Shift held). Returns the steps clicked this frame.
fn arrows(ui: &mut Ui, label: &str) -> i32 {
    let (rect, _) = ui.allocate_exact_size(vec2(14.0, HEIGHT), Sense::hover());
    let half = rect.height() / 2.0;
    let up = Rect::from_min_size(rect.min, vec2(rect.width(), half));
    let down = Rect::from_min_size(
        pos2(rect.min.x, rect.min.y + half),
        vec2(rect.width(), half),
    );
    let shift = ui.input(|i| i.modifiers.shift);
    let size = if shift { 10 } else { 1 };
    let mut steps = 0;
    for (area, sign, name) in [(up, 1, tr("Increase")), (down, -1, tr("Decrease"))] {
        let response = ui.interact(area, ui.id().with((label, name)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Button, true, crate::trf!("{} {}", name, label))
        });
        let color = if response.hovered() {
            theme::TEXT_STRONG
        } else {
            theme::TEXT_WEAK
        };
        let c = area.center();
        let (tip, base) = if sign > 0 { (-2.5, 2.0) } else { (2.5, -2.0) };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                c + vec2(-3.5, base),
                c + vec2(3.5, base),
                c + vec2(0.0, tip),
            ],
            color,
            Stroke::NONE,
        ));
        let clicked = response
            .on_hover_text(crate::trf!("{} by one step (Shift: ten steps)", name))
            .clicked();
        if clicked {
            steps += sign * size;
        }
    }
    steps
}
