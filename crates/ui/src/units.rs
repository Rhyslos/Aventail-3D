//! The unit lengths are shown and typed in: metres or centimetres. The
//! scene always stores centimetres; this only changes how they read.
//!
//! Each project keeps its own units, saved with it (`scene::Units`,
//! decision 0068); this is the interface's side of them, with the user's
//! choice for new projects remembered between sessions.

use crate::text::tr;
use serde::{Deserialize, Serialize};

/// How lengths are shown and typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Units {
    /// Metres: 1 m, 2.5 m. Easier for whole objects.
    #[default]
    Metres,
    /// Centimetres: 100 cm, 250 cm. Handy for small details.
    Centimetres,
}

impl Units {
    /// Both, in the order menus list them.
    pub const ALL: [Units; 2] = [Self::Metres, Self::Centimetres];

    /// The unit's name, for menus.
    pub fn name(self) -> &'static str {
        match self {
            Self::Metres => tr("Metres"),
            Self::Centimetres => tr("Centimetres"),
        }
    }

    /// The unit's symbol, after a number.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Metres => "m",
            Self::Centimetres => "cm",
        }
    }

    /// How many centimetres one unit is.
    fn centimetres(self) -> f64 {
        match self {
            Self::Metres => 100.0,
            Self::Centimetres => 1.0,
        }
    }

    /// A length in centimetres, in this unit.
    pub fn from_cm(self, cm: f32) -> f64 {
        f64::from(cm) / self.centimetres()
    }

    /// A length in this unit, in centimetres.
    pub fn to_cm(self, value: f64) -> f32 {
        (value * self.centimetres()) as f32
    }

    /// How far a value moves per pixel dragged: half a centimetre.
    pub fn drag_speed(self) -> f64 {
        0.5 / self.centimetres()
    }

    /// How far one click on a step arrow moves a value: one centimetre,
    /// for fine tuning.
    pub fn step(self) -> f64 {
        1.0 / self.centimetres()
    }

    /// Decimals shown: down to a millimetre either way.
    pub fn decimals(self) -> usize {
        match self {
            Self::Metres => 3,
            Self::Centimetres => 1,
        }
    }

    /// A length as text, without trailing zeros: "1.5 m", "150 cm".
    pub fn format(self, cm: f32) -> String {
        format!("{} {}", self.number(self.from_cm(cm)), self.symbol())
    }

    /// A value already in this unit, as a number without trailing zeros
    /// ("1", "1.25"), for value fields.
    pub fn number(self, value: f64) -> String {
        let text = format!(
            "{:.*}",
            self.decimals(),
            no_negative_zero(value, self.decimals())
        );
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            text
        }
    }
}

/// `value`, but 0 where it would show as "-0" at `decimals` places (a
/// turn of -0.0001° is shown as 0, not "-0.0").
fn no_negative_zero(value: f64, decimals: usize) -> f64 {
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(6));
    if (value * scale).round() == 0.0 {
        0.0
    } else {
        value
    }
}

/// An angle in degrees as the value fields show it, without the sign:
/// at least one decimal and at most two ("0.0", "12.5", "33.33"), and
/// never "-0.0".
pub(crate) fn degrees_text(value: f64) -> String {
    let text = format!("{:.2}", no_negative_zero(value, 2));
    let trimmed = text.trim_end_matches('0');
    if trimmed.ends_with('.') {
        format!("{trimmed}0")
    } else {
        trimmed.to_string()
    }
}

impl From<scene::Units> for Units {
    fn from(units: scene::Units) -> Self {
        match units {
            scene::Units::Metres => Self::Metres,
            scene::Units::Centimetres => Self::Centimetres,
        }
    }
}

impl From<Units> for scene::Units {
    fn from(units: Units) -> Self {
        match units {
            Units::Metres => Self::Metres,
            Units::Centimetres => Self::Centimetres,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_match_the_projects_both_ways() {
        for units in Units::ALL {
            assert_eq!(Units::from(scene::Units::from(units)), units);
        }
        assert_eq!(Units::from(scene::Units::default()), Units::default());
    }

    #[test]
    fn metres_are_the_standard() {
        assert_eq!(Units::default(), Units::Metres);
    }

    #[test]
    fn lengths_convert_both_ways() {
        assert_eq!(Units::Metres.from_cm(250.0), 2.5);
        assert_eq!(Units::Metres.to_cm(2.5), 250.0);
        assert_eq!(Units::Centimetres.from_cm(250.0), 250.0);
        assert_eq!(Units::Centimetres.to_cm(12.5), 12.5);
    }

    #[test]
    fn a_step_and_a_drag_move_the_same_real_distance_in_either_unit() {
        for units in Units::ALL {
            assert!((units.to_cm(units.step()) - 1.0).abs() < 1e-6, "{units:?}");
            assert!((units.to_cm(units.drag_speed()) - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn lengths_read_without_trailing_zeros() {
        assert_eq!(Units::Metres.format(100.0), "1 m");
        assert_eq!(Units::Metres.format(150.0), "1.5 m");
        assert_eq!(Units::Metres.format(0.1), "0.001 m");
        assert_eq!(Units::Metres.format(0.0), "0 m");
        assert_eq!(Units::Centimetres.format(100.0), "100 cm");
        assert_eq!(Units::Centimetres.format(12.25), "12.2 cm");
        assert_eq!(Units::Metres.number(1.0), "1");
        assert_eq!(Units::Metres.number(1.25), "1.25");
        assert_eq!(Units::Metres.number(-0.5), "-0.5");
    }

    #[test]
    fn nothing_reads_minus_zero() {
        // A turn or a place a hair below zero, from rounding.
        assert_eq!(Units::Metres.number(-0.000_01), "0");
        assert_eq!(Units::Centimetres.number(-0.01), "0");
        assert_eq!(degrees_text(-0.0), "0.0");
        assert_eq!(degrees_text(-0.000_3), "0.0");
        assert_eq!(degrees_text(-1e-12), "0.0");
        assert_eq!(degrees_text(0.0), "0.0");
        assert_eq!(degrees_text(30.0), "30.0");
        assert_eq!(degrees_text(12.5), "12.5");
        assert_eq!(degrees_text(-33.333), "-33.33");
        assert_eq!(degrees_text(-0.01), "-0.01");
    }

    #[test]
    fn units_survive_saving() {
        let text = ron::to_string(&Units::Centimetres).unwrap();
        assert_eq!(ron::from_str::<Units>(&text).unwrap(), Units::Centimetres);
    }
}
