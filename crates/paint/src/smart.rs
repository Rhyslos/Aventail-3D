//! Smart masks ([decision 0195](../../../docs/decisions/0195-smart-masks.md)):
//! a layer's mask made from the shape instead of painted, so the layer
//! shows on its edges, in its cavities, where it faces up, or above a
//! height, broken up with noise. A smart mask keeps its settings and is
//! made again from them on the shape (by the `bake` crate).

/// What a smart mask picks out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmartKind {
    /// Outer edges: where wear and chips go.
    #[default]
    Edges,
    /// Inner edges and creases: where dirt and grime gather.
    Cavities,
    /// Surfaces facing up: where dust, snow and moss settle.
    FacingUp,
    /// Above a height on the object: a tide line, or light toward the top.
    Height,
}

impl SmartKind {
    /// Every kind, in the order shown.
    pub const ALL: [SmartKind; 4] = [
        SmartKind::Edges,
        SmartKind::Cavities,
        SmartKind::FacingUp,
        SmartKind::Height,
    ];

    /// Its number, for saving.
    pub fn index(self) -> u8 {
        match self {
            SmartKind::Edges => 0,
            SmartKind::Cavities => 1,
            SmartKind::FacingUp => 2,
            SmartKind::Height => 3,
        }
    }

    /// The kind numbered `index`.
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }
}

/// A smart mask's settings. Every number is 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmartMask {
    /// What it picks out.
    pub kind: SmartKind,
    /// How much it takes in: for edges and cavities, how wide the band
    /// along them (a share of a tenth of the shape's size); for facing
    /// up, how far from straight up still counts; for height, the level
    /// along the object's height above which it shows.
    pub amount: f32,
    /// How soft its border is: 0 hard, 1 a long fade.
    pub softness: f32,
    /// How strongly noise breaks its border up.
    pub noise: f32,
    /// How big the noise's blotches are, as a share of the shape's size.
    pub noise_size: f32,
    /// Which noise: change it for a different pattern.
    pub seed: u32,
}

impl SmartMask {
    /// A mask of `kind` with settings that suit it to start with.
    pub fn of(kind: SmartKind) -> Self {
        let amount = match kind {
            SmartKind::Edges | SmartKind::Cavities => 0.25,
            SmartKind::FacingUp => 0.4,
            SmartKind::Height => 0.5,
        };
        Self {
            kind,
            amount,
            softness: 0.3,
            noise: 0.3,
            noise_size: 0.08,
            seed: 1,
        }
    }

    /// Whether its numbers are real and in range.
    pub fn is_valid(&self) -> bool {
        [self.amount, self.softness, self.noise, self.noise_size]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_number_and_settings_check() {
        for kind in SmartKind::ALL {
            assert_eq!(SmartKind::from_index(kind.index()), Some(kind));
            assert!(SmartMask::of(kind).is_valid());
        }
        assert_eq!(SmartKind::from_index(9), None);
        let bad = SmartMask {
            noise: 2.0,
            ..SmartMask::of(SmartKind::Edges)
        };
        assert!(!bad.is_valid());
    }
}
