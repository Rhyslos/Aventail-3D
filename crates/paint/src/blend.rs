//! Blend modes ([decision 0181](../../../docs/decisions/0181-paint-layers.md)):
//! how a layer's colour mixes with the colour under it, as in the W3C
//! compositing rules paint programs follow.
//!
//! Each mode gives the colour where both the layer and what is under it
//! are fully there ([`Blend::mix`]); [`over`] then lays that on with the
//! layer's coverage, so where nothing is under the layer its own colour
//! shows, whatever the mode.

/// How a layer mixes onto the layers under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Blend {
    /// Over what is under it, by its own coverage.
    #[default]
    Normal,
    /// Darkens: the colours multiplied (white changes nothing).
    Multiply,
    /// Lightens: the opposite of Multiply (black changes nothing).
    Screen,
    /// Contrast: Multiply where what is under is dark, Screen where light.
    Overlay,
    /// Lightens: the colours added.
    Add,
    /// Darkens: the layer's colour taken away from what is under it.
    Subtract,
    /// The layer's hue and saturation with the lightness under it: tints
    /// keeping the shading.
    Colour,
    /// The layer's hue with the saturation and lightness under it.
    Hue,
}

impl Blend {
    /// Every mode, in the order the Paint layers panel lists them.
    pub const ALL: [Blend; 8] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Add,
        Self::Subtract,
        Self::Colour,
        Self::Hue,
    ];

    /// Its number in saved files.
    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|b| *b == self).unwrap_or(0) as u8
    }

    /// The mode saved as `index`, if known.
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }

    /// The colour where layer colour `s` lies fully over colour `d`
    /// (each 0 to 1).
    pub fn mix(self, s: [f32; 3], d: [f32; 3]) -> [f32; 3] {
        let each = |f: fn(f32, f32) -> f32| [f(s[0], d[0]), f(s[1], d[1]), f(s[2], d[2])];
        match self {
            Self::Normal => s,
            Self::Multiply => each(|s, d| s * d),
            Self::Screen => each(|s, d| s + d - s * d),
            Self::Overlay => each(|s, d| {
                if d <= 0.5 {
                    2.0 * s * d
                } else {
                    1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                }
            }),
            Self::Add => each(|s, d| (s + d).min(1.0)),
            Self::Subtract => each(|s, d| (d - s).max(0.0)),
            Self::Colour => set_lum(s, lum(d)),
            Self::Hue => set_lum(set_sat(s, sat(d)), lum(d)),
        }
    }
}

/// Lightness as the W3C rules weigh it.
fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 && l - n > 1e-6 {
        out = out.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 && x - l > 1e-6 {
        out = out.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    out.map(|v| v.clamp(0.0, 1.0))
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    if max - min <= 1e-6 {
        return [0.0; 3];
    }
    c.map(|v| (v - min) * s / (max - min))
}

/// Lays pixel `src` (straight alpha, 0 to 255) onto `dst` with `opacity`
/// (also scaling its coverage, as a clip does) in `blend` mode.
pub fn over(dst: &mut [u8], src: &[u8], opacity: f32, blend: Blend) {
    let sa = src[3] as f32 / 255.0 * opacity;
    if sa <= 0.0 {
        return;
    }
    // The common cases, quicker and the same as the sums below: nothing
    // under the layer (its own colour shows), and an opaque normal layer
    // (it covers what's under it).
    if dst[3] == 0 || (sa >= 1.0 && blend == Blend::Normal) {
        dst[..3].copy_from_slice(&src[..3]);
        dst[3] = (sa.min(1.0) * 255.0).round() as u8;
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    let s = [src[0], src[1], src[2]].map(|v| v as f32 / 255.0);
    let d = [dst[0], dst[1], dst[2]].map(|v| v as f32 / 255.0);
    let mixed = blend.mix(s, d);
    for c in 0..3 {
        // Where nothing is under the layer its own colour shows.
        let s = (1.0 - da) * s[c] + da * mixed[c];
        let value = (s * sa + d[c] * da * (1.0 - sa)) / out_a;
        dst[c] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay(under: [u8; 4], layer: [u8; 4], blend: Blend) -> [u8; 4] {
        let mut out = under;
        over(&mut out, &layer, 1.0, blend);
        out
    }

    #[test]
    fn each_mode_does_what_it_says() {
        let grey = [128, 128, 128, 255];
        let white = [255, 255, 255, 255];
        let black = [0, 0, 0, 255];
        assert_eq!(lay(grey, white, Blend::Multiply), grey);
        assert_eq!(lay(grey, black, Blend::Screen), grey);
        assert_eq!(lay(grey, white, Blend::Normal), white);
        assert_eq!(lay(grey, grey, Blend::Add)[0], 255);
        assert_eq!(lay(grey, grey, Blend::Subtract)[0], 0);
        // Overlay keeps black and white under it.
        assert_eq!(lay(black, grey, Blend::Overlay)[0], 0);
        assert_eq!(lay(white, grey, Blend::Overlay)[0], 255);
        // Colour tints a grey keeping its lightness.
        let red = [255, 0, 0, 255];
        let tinted = lay(grey, red, Blend::Colour);
        assert!(tinted[0] > tinted[1], "{tinted:?}");
        let l = |c: [u8; 4]| lum([c[0], c[1], c[2]].map(|v| v as f32 / 255.0));
        assert!((l(tinted) - l(grey)).abs() < 0.02);
        // Hue on a grey: no saturation to keep, so it stays grey.
        assert_eq!(lay(grey, red, Blend::Hue), grey);
    }

    #[test]
    fn nothing_under_shows_the_layer_as_it_is() {
        for blend in Blend::ALL {
            assert_eq!(
                lay([0, 0, 0, 0], [10, 200, 30, 255], blend),
                [10, 200, 30, 255],
                "{blend:?}"
            );
        }
    }

    #[test]
    fn modes_save_by_number_and_unknown_numbers_are_refused() {
        for blend in Blend::ALL {
            assert_eq!(Blend::from_index(blend.index()), Some(blend));
        }
        assert_eq!(Blend::from_index(200), None);
    }
}
