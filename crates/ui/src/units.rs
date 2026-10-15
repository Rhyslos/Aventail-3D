//! The unit lengths are shown and typed in: metres or centimetres. The
//! scene always stores centimetres; this only changes how they read.
//!
//! Each project keeps its own units, saved with it (`scene::Units`,
//! decision 0068); this is the interface's side of them, with the user's
//! choice for new projects remembered between sessions.

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
            Self::Metres => "Metres",
            Self::Centimetres => "Centimetres",
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
        let text = format!("{value:.*}", self.decimals());
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            text
        }
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
    fn units_survive_saving() {
        let text = ron::to_string(&Units::Centimetres).unwrap();
        assert_eq!(ron::from_str::<Units>(&text).unwrap(), Units::Centimetres);
    }
}
