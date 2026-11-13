//! Painted-in light and shade: the maths the texture painter's light and
//! shade helper ([decision 0196](../../../docs/decisions/0196-light-and-shade.md))
//! and the Directional light box share, so a material lit in a graph and
//! a model lit in the painter turn from light to shade the same way
//! (decision 0272).
//!
//! The painter's `paint::Lighting` keeps its settings and calls these.

/// Toward the sun, a unit long: `around` degrees round (0 from the right,
/// +X; 90 from the back, +Y; 180 from the left; 270 from the front) and
/// `height` degrees up (0 on the horizon, 90 straight overhead), Z up.
/// On a texture lying flat, +X is right and +Y is up the texture.
pub fn sun(around: f32, height: f32) -> [f32; 3] {
    let (a, h) = (around.to_radians(), height.to_radians());
    [a.cos() * h.cos(), a.sin() * h.cos(), h.sin()]
}

/// How lit (0 shade to 1 light) a surface facing `facing` (the dot of its
/// normal with the sun) is, before shadows, for a light `softness` soft
/// (0, a hard line as in toon shading, to 1, a long fade).
pub fn lit(facing: f32, softness: f32) -> f32 {
    // The turn is centred a little past side-on, so the light wraps
    // round as painters draw it; softness widens it.
    let w = 0.02 + softness * 0.6;
    let t = ((facing + 0.1 + w) / (2.0 * w)).clamp(0.0, 1.0);
    let edge = t * t * (3.0 - 2.0 * t);
    // Past the turn, a surface turned less toward the sun is a little
    // less lit, so a box's top, front and side read as three values. A
    // hard light (softness 0, toon) keeps two flat tones.
    let falloff = 1.0 - softness * 0.8 * (1.0 - facing.clamp(0.0, 1.0));
    edge * falloff
}

/// `rgb` (any scale, sRGB) moved toward a grey of its own lightness as
/// `warmth` falls from 1 (the colour as chosen) to 0 (a neutral grey;
/// decision 0199).
pub fn warmed(rgb: [f32; 3], warmth: f32) -> [f32; 3] {
    let [r, g, b] = rgb;
    let grey = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let k = warmth.clamp(0.0, 1.0);
    rgb.map(|v| grey + (v - grey) * k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_the_sun_is_light_and_away_is_shade() {
        assert_eq!(lit(1.0, 0.5), 1.0);
        assert_eq!(lit(-1.0, 0.5), 0.0);
        assert_eq!(lit(0.0, 0.0), 1.0);
        assert_eq!(lit(-0.2, 0.0), 0.0);
        for f in [-1.0, -0.3, 0.0, 0.4, 1.0] {
            for s in [0.0, 0.5, 1.0] {
                assert!((0.0..=1.0).contains(&lit(f, s)));
            }
        }
    }

    #[test]
    fn the_sun_points_where_its_angles_say() {
        assert!((sun(0.0, 90.0)[2] - 1.0).abs() < 1e-5);
        assert!((sun(0.0, 0.0)[0] - 1.0).abs() < 1e-5);
        assert!((sun(270.0, 0.0)[1] + 1.0).abs() < 1e-5);
    }

    #[test]
    fn no_warmth_is_a_grey_and_full_warmth_the_colour() {
        let c = [1.0, 0.5, 0.2];
        let same = warmed(c, 1.0);
        assert!((0..3).all(|i| (same[i] - c[i]).abs() < 1e-6));
        let g = warmed(c, 0.0);
        assert!((g[0] - g[1]).abs() < 1e-6 && (g[1] - g[2]).abs() < 1e-6);
    }
}
