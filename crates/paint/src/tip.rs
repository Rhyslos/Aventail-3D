//! Brush tips ([decision 0185](../../../docs/decisions/0185-brush-bar-and-tips.md)):
//! the shape a dab paints, for the stylised brush set. A tip gives each
//! texel a weight from where it lies in the dab (turned to follow the
//! stroke) and from the texel itself (so grain stays put on the texture,
//! like paper under chalk).

use glam::Vec2;

/// The shape of a brush's dabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Tip {
    /// A round dab: full inside its hard part, fading to its edge.
    #[default]
    Round,
    /// A round dab broken by grain that stays on the texture: chalk or a
    /// pencil on rough paper.
    Chalk,
    /// Separate bristle lines along the stroke.
    Bristle,
    /// Bristles that skip, leaving gaps: a dry brush dragged over a
    /// surface.
    DryBrush,
    /// A scatter of small specks inside the dab.
    Spatter,
}

impl Tip {
    /// Every tip, in the order the brush bar shows them.
    pub const ALL: [Tip; 5] = [
        Self::Round,
        Self::Chalk,
        Self::Bristle,
        Self::DryBrush,
        Self::Spatter,
    ];

    /// Its number in saved files.
    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0) as u8
    }

    /// The tip saved as `index`, if known.
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }

    /// Whether the dab turns to follow the stroke.
    pub fn follows_stroke(self) -> bool {
        matches!(self, Self::Bristle | Self::DryBrush)
    }

    /// The weight, 0 to 1, of a texel at `offset` from the dab's middle
    /// (in the same units as `radius`), for a dab turned to `angle`
    /// (radians, along the stroke), numbered `seed` in its stroke, at
    /// texel `texel` of the texture. `round` is the plain round falloff
    /// at that distance (the brush's hardness).
    pub fn weight(
        self,
        offset: Vec2,
        radius: f32,
        angle: f32,
        seed: u32,
        texel: (u32, u32),
        round: f32,
    ) -> f32 {
        if round <= 0.0 || radius <= 0.0 {
            return 0.0;
        }
        match self {
            Self::Round => round,
            Self::Chalk => {
                // Grain fixed to the texture: rough paper.
                let grain = grain(texel.0, texel.1);
                // Solid in the middle, breaking up toward the edge.
                if grain < 0.75 * (1.0 - round) {
                    0.0
                } else {
                    round.max(0.35)
                }
            }
            Self::Bristle | Self::DryBrush => {
                let (sin, cos) = angle.sin_cos();
                // Across the stroke, -1 to 1.
                let across = (-offset.x * sin + offset.y * cos) / radius;
                let bristles = 9.0;
                let place = (across * 0.5 + 0.5) * bristles;
                let which = place.floor() as i32;
                let within = place - place.floor();
                // Each bristle a soft line, with its own strength.
                let line = 1.0 - ((within - 0.5).abs() * 2.0).powi(2);
                let strength = 0.55 + 0.45 * hash(which as u32, 17);
                let mut w = round * line.max(0.0) * strength;
                if self == Self::DryBrush {
                    // Bristles skip where the surface's grain is high.
                    let skip = grain(texel.0, texel.1) * 0.7 + hash(which as u32, seed / 3) * 0.3;
                    if skip > 0.55 {
                        w = 0.0;
                    }
                }
                w
            }
            Self::Spatter => {
                let mut most: f32 = 0.0;
                for i in 0..10u32 {
                    let a = hash(seed, i * 2) * std::f32::consts::TAU;
                    let r = hash(seed, i * 2 + 1).sqrt() * radius * 0.85;
                    let speck = radius * (0.08 + 0.1 * hash(seed ^ 0x55, i));
                    let at = Vec2::new(a.cos(), a.sin()) * r;
                    let d = offset.distance(at);
                    if d < speck {
                        most = most.max(1.0 - (d / speck).powi(4));
                    }
                }
                most
            }
        }
    }
}

/// A sample stroke of `tip` with `hardness`, `width` × `height` pixels:
/// an S-curve across, in grey on transparent (RGBA, row 0 at the top),
/// for the brush bar's cards.
pub fn preview(tip: Tip, hardness: f32, width: u32, height: u32) -> Vec<u8> {
    let mut alpha = vec![0.0f32; (width * height) as usize];
    let (w, h) = (width as f32, height as f32);
    let radius = (h * 0.28).max(1.0);
    let point = |t: f32| {
        Vec2::new(
            radius + t * (w - 2.0 * radius),
            h * 0.5 - (t * std::f32::consts::TAU).sin() * h * 0.18,
        )
    };
    let brush = crate::Brush {
        hardness,
        ..crate::Brush::default()
    };
    let steps = ((w / (radius * 0.25)).ceil() as u32).clamp(4, 400);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let at = point(t);
        let ahead = point((t + 0.01).min(1.0)) - point((t - 0.01).max(0.0));
        let angle = ahead.y.atan2(ahead.x);
        let (x0, x1) = (
            (at.x - radius).floor().max(0.0) as u32,
            (at.x + radius).ceil().min(w - 1.0) as u32,
        );
        let (y0, y1) = (
            (at.y - radius).floor().max(0.0) as u32,
            (at.y + radius).ceil().min(h - 1.0) as u32,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let offset = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - at;
                let round = brush.falloff(offset.length(), radius);
                let weight = tip.weight(offset, radius, angle, i, (x, y), round);
                let a = &mut alpha[(y * width + x) as usize];
                *a = a.max(weight);
            }
        }
    }
    alpha
        .into_iter()
        .flat_map(|a| [230, 230, 230, (a.clamp(0.0, 1.0) * 255.0) as u8])
        .collect()
}

/// A steady pseudo-random number, 0 to 1, from two whole numbers.
fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// The texture's grain at a texel, 0 to 1: a little smooth noise, the
/// same wherever and whenever it is painted.
fn grain(x: u32, y: u32) -> f32 {
    let cell = |cx: u32, cy: u32| hash(cx, cy.wrapping_mul(7919));
    let (fx, fy) = (x as f32 / 2.0, y as f32 / 2.0);
    let (x0, y0) = (fx.floor() as u32, fy.floor() as u32);
    let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
    let top = cell(x0, y0) * (1.0 - tx) + cell(x0 + 1, y0) * tx;
    let bottom = cell(x0, y0 + 1) * (1.0 - tx) + cell(x0 + 1, y0 + 1) * tx;
    (top * (1.0 - ty) + bottom * ty) * 0.75 + hash(x, y) * 0.25
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(tip: Tip) -> (usize, usize) {
        // How many texels a dab of radius 20 touches, and in how many
        // places across its middle line (how broken it is).
        let mut touched = 0;
        for y in 0..40u32 {
            for x in 0..40u32 {
                let offset = Vec2::new(x as f32 - 20.0, y as f32 - 20.0);
                let round = (1.0 - offset.length() / 20.0).clamp(0.0, 1.0);
                if tip.weight(offset, 20.0, 0.0, 3, (x, y), round) > 0.05 {
                    touched += 1;
                }
            }
        }
        let mut runs = 0;
        let mut inside = false;
        for y in 0..40u32 {
            let offset = Vec2::new(0.0, y as f32 - 20.0);
            let round = (1.0 - offset.length() / 20.0).max(0.0);
            let on = tip.weight(offset, 20.0, 0.0, 3, (20, y), round) > 0.05;
            if on && !inside {
                runs += 1;
            }
            inside = on;
        }
        (touched, runs)
    }

    #[test]
    fn each_tip_has_its_own_shape() {
        let (round, round_runs) = coverage(Tip::Round);
        assert_eq!(round_runs, 1, "a round dab is whole");
        let (chalk, _) = coverage(Tip::Chalk);
        assert!(
            chalk < round && chalk > round / 4,
            "chalk is broken: {chalk} of {round}"
        );
        let (_, bristle_runs) = coverage(Tip::Bristle);
        assert!(
            bristle_runs >= 4,
            "bristles are separate lines: {bristle_runs}"
        );
        let (dry, _) = coverage(Tip::DryBrush);
        let (bristle, _) = coverage(Tip::Bristle);
        assert!(dry < bristle, "a dry brush skips: {dry} of {bristle}");
        let (spatter, _) = coverage(Tip::Spatter);
        assert!(spatter > 0 && spatter < round / 2, "specks: {spatter}");
    }

    #[test]
    fn previews_show_a_stroke_for_every_tip() {
        for tip in Tip::ALL {
            let image = preview(tip, 0.8, 96, 40);
            assert_eq!(image.len(), 96 * 40 * 4);
            let shown = image.chunks(4).filter(|p| p[3] > 0).count();
            assert!(shown > 40, "{tip:?}: {shown}");
        }
    }

    #[test]
    fn grain_stays_on_the_texture_and_tips_save_by_number() {
        assert_eq!(grain(10, 20), grain(10, 20));
        assert!((0.0..=1.0).contains(&grain(12345, 678)));
        for tip in Tip::ALL {
            assert_eq!(Tip::from_index(tip.index()), Some(tip));
        }
        assert_eq!(Tip::from_index(99), None);
        assert_eq!(Tip::Round.weight(Vec2::ZERO, 0.0, 0.0, 0, (0, 0), 1.0), 0.0);
    }
}
