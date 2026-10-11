//! The Coordinates panel: the selection's position, size and rotation,
//! which can be typed or dragged ([decision 0030](../../../docs/decisions/0030-transform-gizmos.md)).
//!
//! What it shows:
//!
//! - **One object (Model mode):** where its axis is in the world, its own
//!   size (along its own axes, scale included), and its heading, pitch
//!   and bank.
//! - **Several objects:** the middle of their axes, and the size of the
//!   box around them all. No rotation.
//! - **Points, edges or polygons:** the middle of the box around their
//!   points, and its size. No rotation.
//!
//! A change to a field is carried out by the workbench as a move, a scale
//! or a turn, one undo step per drag or typed value.

use crate::theme::{self, tint};
use crate::transforming::{Target, Targets};
use crate::units::Units;
use egui::{Align2, DragValue, FontId, RichText, Sense, Ui, WidgetInfo, WidgetType};
use glam::Vec3;
use scene::Document;

/// What the panel shows, worked out each frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct CoordinatesView {
    /// Where the selection is, in the world (centimetres).
    pub(crate) position: Option<Vec3>,
    /// Its size (centimetres).
    pub(crate) size: Option<Vec3>,
    /// Heading, pitch and bank, in degrees (one object only).
    pub(crate) rotation: Option<Vec3>,
}

impl CoordinatesView {
    /// The view of `target` (made with world axes), if anything is
    /// selected.
    pub(crate) fn of(target: Option<&Target>, document: &Document) -> Self {
        let Some(target) = target else {
            return Self::default();
        };
        match &target.targets {
            Targets::Objects(starts) if starts.len() == 1 => {
                let start = &starts[0];
                let own = document
                    .scene()
                    .local_bounds(start.id)
                    .map_or(Vec3::ZERO, |b| b.size());
                Self {
                    position: Some(target.pivot),
                    size: Some(own * start.transform.scale.abs()),
                    rotation: Some(Vec3::new(
                        start.transform.rotation.x.to_degrees(),
                        start.transform.rotation.y.to_degrees(),
                        start.transform.rotation.z.to_degrees(),
                    )),
                }
            }
            _ => Self {
                position: Some(target.pivot),
                size: Some(target.bounds.size()),
                rotation: None,
            },
        }
    }
}

/// A field of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Field {
    /// Position along X, Y or Z.
    Position(usize),
    /// Size along X, Y or Z.
    Size(usize),
    /// Heading, pitch or bank.
    Rotation(usize),
}

/// A field changed by the user this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Edit {
    pub(crate) field: Field,
    /// The new value: centimetres, or degrees for a rotation.
    pub(crate) value: f32,
}

/// What the user did in the panel this frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Interaction {
    /// A field's new value, if one changed.
    pub(crate) edit: Option<Edit>,
    /// The field being dragged or typed in, if any: while it is the same,
    /// changes are one undo step.
    pub(crate) active: Option<Field>,
    /// A new drag or typing began this frame.
    pub(crate) began: bool,
}

/// One group of fields: its title, its axis letters and colours, its
/// values, and which field each row is.
type Group<'a> = (
    &'a str,
    [(&'a str, egui::Color32); 3],
    Option<Vec3>,
    fn(usize) -> Field,
);

/// Width of one group (Position, Size or Rotation).
const GROUP_WIDTH: f32 = 200.0;
/// Height of a value field.
const FIELD_HEIGHT: f32 = 22.0;

/// Draws the panel.
pub(crate) fn show(ui: &mut Ui, units: Units, view: &CoordinatesView) -> Interaction {
    let mut out = Interaction::default();
    let position = [
        ("X", tint::AXIS_X),
        ("Y", tint::AXIS_Y),
        ("Z", tint::AXIS_Z),
    ];
    let rotation = [
        ("H", theme::TEXT_WEAK),
        ("P", theme::TEXT_WEAK),
        ("B", theme::TEXT_WEAK),
    ];
    let groups: [Group; 3] = [
        ("Position", position, view.position, Field::Position),
        ("Size", position, view.size, Field::Size),
        ("Rotation", rotation, view.rotation, Field::Rotation),
    ];
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        for (title, axes, values, field_of) in groups {
            ui.vertical(|ui| {
                ui.set_width(GROUP_WIDTH);
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(RichText::new(title).small().color(theme::TEXT_WEAK));
                for (index, (axis, color)) in axes.into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (letter, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, FIELD_HEIGHT), Sense::hover());
                        ui.painter().text(
                            letter.center(),
                            Align2::CENTER_CENTER,
                            axis,
                            FontId::proportional(13.0),
                            color,
                        );
                        let field = field_of(index);
                        let label = format!("{title} {axis}");
                        let value = values.map(|v| v[index]);
                        field_widget(ui, &label, field, value, units, &mut out);
                    });
                }
            });
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.vertical(|ui| {
                ui.add_space(18.0);
                ui.label(RichText::new("World").color(theme::TEXT_WEAK))
                    .on_hover_text(
                        "Positions are in the world. Type a value and press Enter, or drag \
                         the number sideways. Each change is one undo step.",
                    );
            });
        });
    });
    out
}

/// One value field: a number to drag or type, or a dash when there is
/// nothing to show.
fn field_widget(
    ui: &mut Ui,
    label: &str,
    field: Field,
    value: Option<f32>,
    units: Units,
    out: &mut Interaction,
) {
    let width = ui.available_width();
    let size = egui::vec2(width, FIELD_HEIGHT);
    let Some(value) = value.filter(|v| v.is_finite()) else {
        let response = ui.add_enabled(false, egui::Button::new("—").min_size(size));
        response.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, false, label));
        response.on_disabled_hover_text(match field {
            Field::Rotation(_) => "Rotation shows for one object in the Model mode",
            _ => "Select something to see and type its coordinates",
        });
        return;
    };
    let response = match field {
        Field::Rotation(_) => {
            let mut degrees = f64::from(value);
            let response = ui.add_sized(
                size,
                DragValue::new(&mut degrees)
                    .speed(0.5)
                    .max_decimals(2)
                    .suffix("°"),
            );
            if response.changed() {
                out.edit = Some(Edit {
                    field,
                    value: degrees as f32,
                });
            }
            response
        }
        Field::Position(_) | Field::Size(_) => {
            let mut shown = units.from_cm(value);
            let mut drag = DragValue::new(&mut shown)
                .speed(units.drag_speed())
                .max_decimals(units.decimals())
                .custom_formatter(move |v, _| units.number(v))
                .suffix(format!(" {}", units.symbol()));
            if let Field::Size(_) = field {
                drag = drag.range(0.0..=f64::MAX);
            }
            let response = ui.add_sized(size, drag);
            if response.changed() {
                out.edit = Some(Edit {
                    field,
                    value: units.to_cm(shown),
                });
            }
            response
        }
    };
    response.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, label));
    if response.dragged() || response.has_focus() || response.changed() {
        out.active = Some(field);
    }
    if response.drag_started() || response.gained_focus() {
        out.began = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawing::DrawingCache;
    use crate::palette::Mode;
    use crate::transforming::Axes;
    use scene::{Command, ObjectKind, PrimitiveKind, Transform};

    #[test]
    fn one_object_shows_its_place_size_and_turn() {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let cube = doc.selection()[0];
        let placed = Transform {
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0),
            scale: Vec3::new(2.0, 1.0, 1.0),
        };
        doc.execute(Command::SetTransforms(vec![(cube, placed)]))
            .unwrap();
        let mut cache = DrawingCache::default();
        let drawing = cache.update(&doc, false).drawing.clone();
        let target = Target::of(&doc, &drawing, Mode::Model, Axes::World);
        let view = CoordinatesView::of(target.as_ref(), &doc);
        assert!(view.position.unwrap().abs_diff_eq(placed.position, 1e-3));
        assert!(
            view.size
                .unwrap()
                .abs_diff_eq(Vec3::new(200.0, 100.0, 100.0), 1e-2)
        );
        assert!((view.rotation.unwrap().x - 90.0).abs() < 1e-3);

        // Nothing selected: nothing to show.
        doc.clear_selection();
        let drawing = cache.update(&doc, false).drawing.clone();
        let target = Target::of(&doc, &drawing, Mode::Model, Axes::World);
        assert_eq!(
            CoordinatesView::of(target.as_ref(), &doc),
            CoordinatesView::default()
        );
    }
}
