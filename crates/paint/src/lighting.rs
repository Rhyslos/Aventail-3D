//! The light-and-shadow helper ([decision 0196](../../../docs/decisions/0196-light-and-shade.md)):
//! a layer that paints soft light and shade from a sun direction, warm
//! where the sun reaches and cool in the shade, as a starting layer for
//! hand painting. It keeps its settings and is made again from them on
//! the shape (by the `bake` crate).

/// The light-and-shade layer's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lighting {
    /// Which way the sun is around the object, in degrees: 0 is from
    /// the right (+X), 90 from the back (+Y), 180 from the left, 270
    /// from the front.
    pub around: f32,
    /// How high the sun is, in degrees: 0 on the horizon, 90 straight
    /// overhead.
    pub height: f32,
    /// The colour where the sun reaches (sRGB, with alpha).
    pub light: [u8; 4],
    /// The colour in the shade.
    pub shade: [u8; 4],
    /// How soft the turn from light to shade is, 0 (a hard line, as in
    /// toon shading) to 1 (a long fade).
    pub softness: f32,
    /// Whether parts of the shape cast shadows on the rest.
    pub shadows: bool,
    /// How warm the light and how cool the shade, 0 (both neutral greys
    /// of the same lightness) to 1 (the colours as chosen; decision 0199).
    pub warmth: f32,
}

impl Default for Lighting {
    /// A warm sun from the front left, high up; cool blue shade; soft;
    /// with shadows.
    fn default() -> Self {
        Self {
            around: 240.0,
            height: 50.0,
            light: [255, 222, 170, 255],
            shade: [60, 80, 140, 255],
            softness: 0.5,
            shadows: true,
            warmth: 1.0,
        }
    }
}

impl Lighting {
    /// Whether its numbers are real and in range.
    pub fn is_valid(&self) -> bool {
        self.around.is_finite()
            && (0.0..=360.0).contains(&self.around)
            && self.height.is_finite()
            && (0.0..=90.0).contains(&self.height)
            && self.softness.is_finite()
            && (0.0..=1.0).contains(&self.softness)
            && self.warmth.is_finite()
            && (0.0..=1.0).contains(&self.warmth)
    }

    /// Toward the sun, unit length, in the object's axes (Z up).
    pub fn sun(&self) -> [f32; 3] {
        let (a, h) = (self.around.to_radians(), self.height.to_radians());
        [a.cos() * h.cos(), a.sin() * h.cos(), h.sin()]
    }

    /// How lit (0 shade to 1 light) a surface facing `facing` (the dot
    /// of its normal with the sun) is, before shadows.
    pub fn lit(&self, facing: f32) -> f32 {
        // The turn is centred a little past side-on, so the light wraps
        // round as painters draw it; softness widens it.
        let w = 0.02 + self.softness * 0.6;
        let t = ((facing + 0.1 + w) / (2.0 * w)).clamp(0.0, 1.0);
        let edge = t * t * (3.0 - 2.0 * t);
        // Past the turn, a surface turned less toward the sun is a little
        // less lit, so a box's top, front and side read as three values (a
        // review found the top and front came out the same). A hard light
        // (softness 0, toon) keeps two flat tones.
        let falloff = 1.0 - self.softness * 0.8 * (1.0 - facing.clamp(0.0, 1.0));
        edge * falloff
    }

    /// The colour at `amount` lit, 0 shade to 1 light.
    pub fn at(&self, amount: f32) -> [u8; 4] {
        let k = amount.clamp(0.0, 1.0);
        let (shade, light) = (self.warmed(self.shade), self.warmed(self.light));
        let mut out = [0u8; 4];
        for (c, o) in out.iter_mut().enumerate() {
            let (a, b) = (shade[c], light[c]);
            *o = (a + (b - a) * k).round().clamp(0.0, 255.0) as u8;
        }
        out
    }
}

impl Lighting {
    /// `colour` moved toward a grey of its own lightness as warmth falls.
    fn warmed(&self, colour: [u8; 4]) -> [f32; 4] {
        let [r, g, b, a] = colour.map(f32::from);
        let grey = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let k = self.warmth.clamp(0.0, 1.0);
        let mix = |v: f32| grey + (v - grey) * k;
        [mix(r), mix(g), mix(b), a]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_points_where_its_angles_say() {
        let up = Lighting {
            height: 90.0,
            ..Lighting::default()
        };
        assert!((up.sun()[2] - 1.0).abs() < 1e-5);
        let right = Lighting {
            around: 0.0,
            height: 0.0,
            ..Lighting::default()
        };
        assert!((right.sun()[0] - 1.0).abs() < 1e-5);
        let front = Lighting {
            around: 270.0,
            height: 0.0,
            ..Lighting::default()
        };
        assert!((front.sun()[1] + 1.0).abs() < 1e-5);
    }

    #[test]
    fn facing_the_sun_is_light_and_away_is_shade() {
        let l = Lighting::default();
        assert_eq!(l.lit(1.0), 1.0);
        assert_eq!(l.lit(-1.0), 0.0);
        let mid = l.lit(-0.1);
        assert!((mid - 0.5).abs() < 1e-4, "{mid}");
        // Hard: a sharp step.
        let hard = Lighting { softness: 0.0, ..l };
        assert_eq!(hard.lit(0.0), 1.0);
        assert_eq!(hard.lit(-0.2), 0.0);
        assert_eq!(l.at(1.0), l.light);
        assert_eq!(l.at(0.0), l.shade);
    }

    #[test]
    fn no_warmth_makes_neutral_greys_of_the_same_lightness() {
        let l = Lighting {
            warmth: 0.0,
            ..Lighting::default()
        };
        let light = l.at(1.0);
        assert!(light[0] == light[1] && light[1] == light[2], "{light:?}");
        let [r, g, b, _] = Lighting::default().light.map(f32::from);
        let lightness = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        assert!((f32::from(light[0]) - lightness).abs() <= 1.0);
        let half = Lighting { warmth: 0.5, ..l }.at(0.0);
        let full = Lighting::default().at(0.0);
        assert!(half[2] < full[2] && half[2] > l.at(0.0)[2], "{half:?}");
    }

    #[test]
    fn numbers_out_of_range_are_refused() {
        let l = Lighting::default();
        assert!(l.is_valid());
        for bad in [
            Lighting { around: 400.0, ..l },
            Lighting { height: -1.0, ..l },
            Lighting {
                softness: f32::NAN,
                ..l
            },
            Lighting { warmth: 1.5, ..l },
        ] {
            assert!(!bad.is_valid());
        }
    }

    #[test]
    fn a_soft_light_gives_top_front_and_side_three_values() {
        // The hand-painted start's sun and a box's faces (decision 0196,
        // amended after the first painter review).
        let l = Lighting {
            around: 250.0,
            height: 55.0,
            ..Lighting::default()
        };
        let sun = l.sun();
        let facing = |n: [f32; 3]| n[0] * sun[0] + n[1] * sun[1] + n[2] * sun[2];
        let top = l.lit(facing([0.0, 0.0, 1.0]));
        let front = l.lit(facing([0.0, -1.0, 0.0]));
        let side = l.lit(facing([-1.0, 0.0, 0.0]));
        assert!(top > front + 0.04 && front > side + 0.04, "{top} {front} {side}");
        // A hard (toon) light: two flat tones, full where lit.
        let hard = Lighting { softness: 0.0, ..l };
        assert_eq!(hard.lit(facing([0.0, -1.0, 0.0])), 1.0);
        assert_eq!(hard.lit(facing([0.0, 0.0, 1.0])), 1.0);
    }
}
