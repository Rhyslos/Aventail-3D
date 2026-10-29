//! How the viewport draws the scene ([decision 0230](../../../../docs/decisions/0230-viewport-looks.md)):
//! the backdrop's colours, the grid's colour and how the shapes are lit,
//! chosen in Settings → Viewport. **Default** is the standard: one smooth
//! grey gradient, a clearer grid and shapes lit from the camera, so they
//! read bright and clear. **Studio** is lighter, with a stronger key
//! light, a soft shine and soft shadows under objects. **Dusk** has a cool
//! blue-grey sky and a warm light with cool shade. **Classic** is the look
//! the viewport had before looks could be chosen. **Custom** lets the
//! person set each colour and light themselves.

use crate::text::tr;
use serde::{Deserialize, Serialize};

/// Which look the viewport uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LookChoice {
    /// One smooth grey gradient, shapes lit from the camera (the standard,
    /// named "Default").
    #[default]
    Standard,
    /// Lighter, with a key light, a shine and soft shadows.
    Studio,
    /// A cool blue-grey sky, a warm light and cool shade.
    Dusk,
    /// The look before looks could be chosen.
    Classic,
    /// The person's own colours and lights.
    Custom,
}

impl LookChoice {
    /// Every choice, in the order listed.
    pub const ALL: [LookChoice; 5] = [
        Self::Standard,
        Self::Studio,
        Self::Dusk,
        Self::Classic,
        Self::Custom,
    ];

    /// Its name in the list.
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => tr("Default"),
            Self::Studio => tr("Studio"),
            Self::Dusk => tr("Dusk"),
            Self::Classic => tr("Classic"),
            Self::Custom => tr("Custom"),
        }
    }

    /// What it looks like, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Standard => tr(
                "One smooth grey gradient from top to bottom, a clear grid, and shapes lit \
                 from the camera so their fronts read bright: the standard",
            ),
            Self::Dusk => tr(
                "A cool blue-grey sky over a darker ground, a warm light from above with cool \
                 blue shade, and soft shadows: calm, with some mood",
            ),
            Self::Studio => tr(
                "A lighter grey studio with a horizon, a stronger light from above, a soft \
                 shine and soft shadows under objects, so shapes show their form",
            ),
            Self::Classic => tr(
                "The darker, softly lit look the viewport had before: a dark grey sky over \
                 a darker ground",
            ),
            Self::Custom => tr("Your own colours and lights, set below"),
        }
    }

    /// The look to draw, `custom` being the person's own.
    pub fn look(self, custom: &SceneLook) -> SceneLook {
        match self {
            Self::Standard => SceneLook::STANDARD,
            Self::Studio => SceneLook::STUDIO,
            Self::Dusk => SceneLook::DUSK,
            Self::Classic => SceneLook::CLASSIC,
            Self::Custom => custom.clamped(),
        }
    }
}

/// The colours and lights of one look. Colours are sRGB.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneLook {
    /// The backdrop at the top of the view (the sky straight up).
    pub top: [u8; 3],
    /// The backdrop at the horizon.
    pub horizon: [u8; 3],
    /// The backdrop at the bottom of the view (the ground).
    pub bottom: [u8; 3],
    /// Whether there is a horizon line between sky and ground; without
    /// one the backdrop is one smooth gradient.
    pub horizon_line: bool,
    /// The grid's colour.
    pub grid: [u8; 3],
    /// How strongly the grid is drawn, 0 to 4 times the classic look's.
    pub grid_strength: f32,
    /// How strong the light from above is, 0 to 2.
    pub main_light: f32,
    /// How strong the light from the camera is, 0 to 2.
    pub camera_light: f32,
    /// How much the light shines on surfaces, 0 to 1.
    pub shine: f32,
    /// Whether objects cast soft shadows on the ground.
    pub shadows: bool,
    /// How warm the light is and how cool the shade, 0 (neutral) to 1.
    pub warmth: f32,
}

impl Default for SceneLook {
    fn default() -> Self {
        Self::STANDARD
    }
}

impl SceneLook {
    /// The standard look: one smooth grey gradient, a clear grid, shapes
    /// lit mostly from the camera.
    pub const STANDARD: SceneLook = SceneLook {
        top: [0x65, 0x67, 0x6b],
        horizon: [0x46, 0x47, 0x4b],
        bottom: [0x2d, 0x2e, 0x31],
        horizon_line: false,
        grid: [0xb4, 0xb6, 0xba],
        grid_strength: 2.2,
        main_light: 0.25,
        camera_light: 0.42,
        shine: 0.06,
        shadows: false,
        warmth: 0.0,
    };

    /// A lighter studio: a key light from above, a shine, soft shadows.
    pub const STUDIO: SceneLook = SceneLook {
        top: [0x78, 0x7b, 0x80],
        horizon: [0x5c, 0x5f, 0x63],
        bottom: [0x55, 0x58, 0x5c],
        horizon_line: true,
        grid: [0xff, 0xff, 0xff],
        grid_strength: 1.5,
        main_light: 0.8,
        camera_light: 0.08,
        shine: 0.3,
        shadows: true,
        warmth: 0.0,
    };

    /// A cool blue-grey sky, a warm key light with cool shade, soft
    /// shadows.
    pub const DUSK: SceneLook = SceneLook {
        top: [0x4a, 0x54, 0x66],
        horizon: [0x36, 0x3c, 0x48],
        bottom: [0x2e, 0x33, 0x3b],
        horizon_line: true,
        grid: [0xa9, 0xc1, 0xea],
        grid_strength: 1.4,
        main_light: 0.75,
        camera_light: 0.08,
        shine: 0.25,
        shadows: true,
        warmth: 1.0,
    };

    /// The look before looks could be chosen.
    pub const CLASSIC: SceneLook = SceneLook {
        top: [0x4a, 0x4a, 0x4a],
        horizon: [0x3e, 0x3e, 0x3e],
        bottom: [0x38, 0x38, 0x38],
        horizon_line: true,
        grid: [0xff, 0xff, 0xff],
        grid_strength: 1.0,
        main_light: 0.5,
        camera_light: 0.12,
        shine: 0.0,
        shadows: false,
        warmth: 0.0,
    };

    /// The same look with its numbers in range (a hand-edited settings
    /// file can hold anything).
    #[must_use]
    pub fn clamped(&self) -> SceneLook {
        let num = |v: f32, low: f32, high: f32, fallback: f32| {
            if v.is_finite() {
                v.clamp(low, high)
            } else {
                fallback
            }
        };
        SceneLook {
            grid_strength: num(self.grid_strength, 0.0, 4.0, 1.0),
            main_light: num(self.main_light, 0.0, 2.0, 0.5),
            camera_light: num(self.camera_light, 0.0, 2.0, 0.12),
            shine: num(self.shine, 0.0, 1.0, 0.0),
            warmth: num(self.warmth, 0.0, 1.0, 0.0),
            ..*self
        }
    }

    /// The backdrop's colours as the renderer takes them: sRGB 0 to 1.
    pub fn backdrop(&self) -> [[f32; 3]; 3] {
        [self.top, self.horizon, self.bottom].map(unit)
    }

    /// The grid's colour, sRGB 0 to 1.
    pub fn grid_colour(&self) -> [f32; 3] {
        unit(self.grid)
    }
}

/// An sRGB colour as 0 to 1.
fn unit(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| v as f32 / 255.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_classic_look_is_the_one_before_looks() {
        use crate::theme::backdrop;
        let classic = SceneLook::CLASSIC;
        let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
        assert_eq!(classic.top, rgb(backdrop::SKY_TOP));
        assert_eq!(classic.horizon, rgb(backdrop::HORIZON));
        assert_eq!(classic.bottom, rgb(backdrop::GROUND));
        // The light the shader had before (render's FrameLook standard).
        assert_eq!((classic.main_light, classic.camera_light), (0.5, 0.12));
        assert_eq!(classic.grid_strength, 1.0);
        assert!(classic.horizon_line && !classic.shadows);
    }

    #[test]
    fn default_is_the_standard_and_each_choice_gives_its_look() {
        assert_eq!(LookChoice::default(), LookChoice::Standard);
        let mut custom = SceneLook::CLASSIC;
        custom.top = [1, 2, 3];
        assert_eq!(LookChoice::Standard.look(&custom), SceneLook::STANDARD);
        assert_eq!(LookChoice::Studio.look(&custom), SceneLook::STUDIO);
        assert_eq!(LookChoice::Dusk.look(&custom), SceneLook::DUSK);
        assert_eq!(LookChoice::Standard.label(), "Default");
        assert_eq!(LookChoice::Classic.look(&custom), SceneLook::CLASSIC);
        assert_eq!(LookChoice::Custom.look(&custom).top, [1, 2, 3]);
        for choice in LookChoice::ALL {
            assert!(!choice.label().is_empty() && !choice.explanation().is_empty());
        }
    }

    #[test]
    fn a_broken_custom_look_is_brought_into_range() {
        let broken = SceneLook {
            grid_strength: f32::NAN,
            main_light: 99.0,
            camera_light: -3.0,
            shine: f32::INFINITY,
            warmth: -1.0,
            ..SceneLook::STANDARD
        };
        let fixed = LookChoice::Custom.look(&broken);
        assert_eq!(fixed.grid_strength, 1.0);
        assert_eq!(fixed.main_light, 2.0);
        assert_eq!(fixed.camera_light, 0.0);
        assert_eq!(fixed.shine, 0.0);
        assert_eq!(fixed.warmth, 0.0);
    }

    #[test]
    fn a_look_survives_saving_and_settings_without_one_read_as_standard() {
        let look = SceneLook::STUDIO;
        let text = ron::to_string(&look).unwrap();
        let back: SceneLook = ron::from_str(&text).unwrap();
        assert_eq!(back, look);
        let partial: SceneLook = ron::from_str("(top: (9, 9, 9))").unwrap();
        assert_eq!(partial.top, [9, 9, 9]);
        assert_eq!(partial.bottom, SceneLook::STANDARD.bottom);
        for text in ["", "[]", "(shine: \"x\")", "()", "Standard", "Nope"] {
            let _ = ron::from_str::<SceneLook>(text);
            let _ = ron::from_str::<LookChoice>(text);
        }
    }
}
