//! Material layers ([decision 0210](../../../docs/decisions/0210-material-layers.md)):
//! a tiling material (brushed metal, wood, stone or a coat of paint)
//! made on the shape from settings, projected from three sides so it
//! needs no particular UVs. Like a gradient, the layer's pixels (and its
//! roughness and metal, with the realism channels) are made from the
//! settings by the `bake` crate, and it isn't painted into.

/// Which material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialKind {
    /// Brushed metal: fine streaks one way, shiny.
    #[default]
    Metal,
    /// Wood: grain along it and growth rings across it.
    Wood,
    /// Stone: blotches and fine speckles, rough.
    Stone,
    /// A coat of paint: one colour with a faint, even texture.
    Paint,
}

impl MaterialKind {
    /// Every kind, in the order shown.
    pub const ALL: [MaterialKind; 4] = [
        MaterialKind::Metal,
        MaterialKind::Wood,
        MaterialKind::Stone,
        MaterialKind::Paint,
    ];

    /// Its number, for saving.
    pub fn index(self) -> u8 {
        self as u8
    }

    /// The kind numbered `index`.
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }
}

/// The settings of a material layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialFill {
    /// Which material.
    pub kind: MaterialKind,
    /// Its main colour (sRGB).
    pub colour: [u8; 4],
    /// Its second colour: the streaks, the rings, the speckles (sRGB).
    pub second: [u8; 4],
    /// How big its pattern is: centimetres across one repeat, 1 to 1000.
    pub scale: f32,
    /// Its roughness, 0 to 1 (with the realism channels).
    pub roughness: f32,
    /// Whether it is metal, 0 to 1 (with the realism channels).
    pub metal: f32,
    /// How sharply the three sides' projections change over at corners,
    /// 0 (a long blend) to 1 (crisp).
    pub sharpness: f32,
    /// Which pattern: change it for a different one.
    pub seed: u32,
}

/// The repeats a material's scale may span, in centimetres.
pub const SCALE_RANGE: std::ops::RangeInclusive<f32> = 1.0..=1000.0;

impl MaterialFill {
    /// A material of `kind` with settings that suit it to start with.
    pub fn of(kind: MaterialKind) -> Self {
        let (colour, second, scale, roughness, metal) = match kind {
            MaterialKind::Metal => ([168, 170, 176, 255], [140, 142, 150, 255], 40.0, 0.35, 1.0),
            MaterialKind::Wood => ([150, 104, 62, 255], [104, 66, 36, 255], 60.0, 0.7, 0.0),
            MaterialKind::Stone => ([128, 124, 116, 255], [92, 88, 82, 255], 80.0, 0.9, 0.0),
            MaterialKind::Paint => ([70, 110, 150, 255], [62, 100, 140, 255], 30.0, 0.45, 0.0),
        };
        Self {
            kind,
            colour,
            second,
            scale,
            roughness,
            metal,
            sharpness: 0.6,
            seed: 1,
        }
    }

    /// Whether its numbers are real and in range.
    pub fn is_valid(&self) -> bool {
        self.scale.is_finite()
            && SCALE_RANGE.contains(&self.scale)
            && [self.roughness, self.metal, self.sharpness]
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_number_and_start_valid() {
        for kind in MaterialKind::ALL {
            assert_eq!(MaterialKind::from_index(kind.index()), Some(kind));
            assert!(MaterialFill::of(kind).is_valid());
        }
        assert_eq!(MaterialKind::from_index(9), None);
        let bad = MaterialFill {
            scale: 0.0,
            ..MaterialFill::of(MaterialKind::Wood)
        };
        assert!(!bad.is_valid());
        let nan = MaterialFill {
            roughness: f32::NAN,
            ..MaterialFill::of(MaterialKind::Wood)
        };
        assert!(!nan.is_valid());
    }
}
