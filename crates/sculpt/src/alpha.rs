//! Alphas (decision 0308): greyscale pictures a sculpting brush stamps or
//! drags. White works fully, black not at all, and the greys in between
//! partly, so a dab of Draw with the Stones alpha raises stones instead of
//! a round lump.
//!
//! An [`Alpha`] keeps one byte a pixel, row by row from the top, at most
//! [`MAX_ALPHA_SIDE`] pixels across or down. The crate never reads image
//! files: whoever brings an alpha in (`fileio`) decodes it and hands over
//! the bytes. A set is made in code ([`Made`], [`made`]), so the app has
//! alphas to try from the start without any pictures to keep.
//!
//! A dab lays its alpha flat on the plane under the brush ([`DabAlpha`]),
//! the picture's top turned towards a way the interface gives (the view's
//! up, the stroke's way, or the way a stamp was dragged), and the live
//! sculpt scales each point's move by the alpha under it.

use glam::{Quat, Vec3};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use words::text;

/// The widest or tallest an alpha is kept, in pixels: bigger pictures are
/// made smaller when brought in.
pub const MAX_ALPHA_SIDE: u32 = 1024;

/// The most characters an alpha's name keeps.
pub const MAX_ALPHA_NAME: usize = 80;

/// The side of the alphas made in code, in pixels.
pub const MADE_SIDE: u32 = 256;

/// A greyscale picture a brush stamps or drags.
#[derive(Clone)]
pub struct Alpha {
    width: u32,
    height: u32,
    values: Arc<Vec<u8>>,
    name: String,
    key: u64,
    made: Option<Made>,
}

impl std::fmt::Debug for Alpha {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Alpha")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("name", &self.name)
            .field("made", &self.made)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Alpha {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.width == other.width
            && self.height == other.height
            && self.name == other.name
            && self.made == other.made
            && (Arc::ptr_eq(&self.values, &other.values) || self.values == other.values)
    }
}

impl Alpha {
    /// An alpha `width` × `height` pixels from one byte a pixel `values`
    /// (0 black … 255 white, row by row from the top) named `name` (cut
    /// to [`MAX_ALPHA_NAME`] characters); or why it can't be one.
    pub fn new(width: u32, height: u32, values: Vec<u8>, name: &str) -> Result<Self, &'static str> {
        if width == 0 || height == 0 || width > MAX_ALPHA_SIDE || height > MAX_ALPHA_SIDE {
            return Err(text!(
                "an alpha is bigger than the app keeps, or has no pixels"
            ));
        }
        if values.len() != width as usize * height as usize {
            return Err(text!("an alpha has the wrong number of pixels"));
        }
        if values.iter().all(|v| *v == 0) {
            return Err(text!(
                "an alpha is black everywhere, so a brush would do nothing with it"
            ));
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (width, height).hash(&mut hasher);
        values.hash(&mut hasher);
        Ok(Self {
            width,
            height,
            values: Arc::new(values),
            name: name.chars().take(MAX_ALPHA_NAME).collect(),
            key: hasher.finish(),
            made: None,
        })
    }

    /// Pixels across.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Pixels down.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Its pixels, one byte each, row by row from the top.
    pub fn values(&self) -> &[u8] {
        &self.values
    }

    /// Its name: the file it came from, or the made alpha's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// A number that changes when its pixels do, for keeping pictures of
    /// it.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// Which alpha made in code it is, if it is one.
    pub fn made(&self) -> Option<Made> {
        self.made
    }

    /// Roughly how many bytes it holds.
    pub fn memory_size(&self) -> usize {
        self.values.len() + self.name.len() + std::mem::size_of::<Self>()
    }

    /// The alpha at `x`, `y`, each −1 to 1 across the square the picture
    /// is centred in (`y` up: 1 is the picture's top), 0 to 1, blended
    /// between pixels; 0 outside the picture.
    pub fn at(&self, x: f32, y: f32) -> f32 {
        if !(x.is_finite() && y.is_finite()) || x.abs() > 1.0 || y.abs() > 1.0 {
            return 0.0;
        }
        let (w, h) = (self.width as f32, self.height as f32);
        let side = w.max(h);
        // Pixel centres at whole numbers plus a half.
        let px = (x + 1.0) * 0.5 * side - (side - w) * 0.5 - 0.5;
        let py = (1.0 - y) * 0.5 * side - (side - h) * 0.5 - 0.5;
        if px < -0.5 || py < -0.5 || px > w - 0.5 || py > h - 0.5 {
            return 0.0;
        }
        let px = px.clamp(0.0, w - 1.0);
        let py = py.clamp(0.0, h - 1.0);
        let (x0, y0) = (px.floor() as u32, py.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (tx, ty) = (px - x0 as f32, py - y0 as f32);
        let v = |x: u32, y: u32| f32::from(self.values[(y * self.width + x) as usize]) / 255.0;
        let top = v(x0, y0) * (1.0 - tx) + v(x1, y0) * tx;
        let bottom = v(x0, y1) * (1.0 - tx) + v(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

/// An alpha laid on a dab: the picture, and which way its top points.
#[derive(Debug, Clone, Copy)]
pub struct DabAlpha<'a> {
    /// The picture.
    pub alpha: &'a Alpha,
    /// Which way the picture's top points, in the object's own space
    /// (laid on the plane under the brush): the view's up for a fixed
    /// alpha, the way a stamp was dragged.
    pub up: Vec3,
    /// The picture turned this far further round the plane's facing, in
    /// radians (anticlockwise seen from the front).
    pub angle: f32,
    /// Whether the picture's top follows the stroke's way instead of
    /// `up` (once the stroke has moved).
    pub follow: bool,
}

/// How a dab's alpha lies on the plane under the brush.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Laid<'a> {
    alpha: &'a Alpha,
    centre: Vec3,
    up: Vec3,
    right: Vec3,
    radius: f32,
}

impl<'a> Laid<'a> {
    /// `alpha` laid on a dab at `centre` of `radius` whose plane faces
    /// `normal`; `way` is the stroke's way since its last dab, if any.
    pub(crate) fn new(
        alpha: &DabAlpha<'a>,
        centre: Vec3,
        radius: f32,
        normal: Vec3,
        way: Option<Vec3>,
    ) -> Self {
        let on_plane = |v: Vec3| (v - normal * v.dot(normal)).try_normalize();
        let followed = way.filter(|_| alpha.follow).and_then(on_plane);
        let up = followed
            .or_else(|| on_plane(alpha.up))
            .unwrap_or_else(|| normal.any_orthonormal_vector());
        let turn = if alpha.angle.is_finite() {
            alpha.angle
        } else {
            0.0
        };
        let up = (Quat::from_axis_angle(normal, turn) * up).normalize_or(up);
        let right = up
            .cross(normal)
            .normalize_or(normal.any_orthonormal_vector());
        Self {
            alpha: alpha.alpha,
            centre,
            up,
            right,
            radius: radius.max(1e-12),
        }
    }

    /// The same alpha turned over left to right: a reflected dab's
    /// (symmetry), so the picture lands as the mirror image.
    pub(crate) fn mirrored(self) -> Self {
        Self {
            right: -self.right,
            ..self
        }
    }

    /// The alpha under the point `p`.
    pub(crate) fn at(&self, p: Vec3) -> f32 {
        let d = p - self.centre;
        self.alpha.at(
            d.dot(self.right) / self.radius,
            d.dot(self.up) / self.radius,
        )
    }
}

/// The alphas made in code: textures stylised sculpts use most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Made {
    /// Rounded stones packed together, with grooves between.
    Stones,
    /// Long grooves running up and down, like tree bark.
    Bark,
    /// Rows of overlapping rounded scales.
    Scales,
    /// A seam of short slanted stitches.
    Stitches,
    /// Round rivet heads in a grid.
    Rivets,
    /// Thin cracks branching across.
    Cracks,
    /// Rows of half-round ridges, like roof tiles or shell edges.
    Scallops,
}

impl Made {
    /// Every one, in the shelf's order.
    pub const ALL: [Self; 7] = [
        Self::Stones,
        Self::Bark,
        Self::Scales,
        Self::Stitches,
        Self::Rivets,
        Self::Cracks,
        Self::Scallops,
    ];

    /// Its name (English: the key of its translation).
    pub fn name(self) -> &'static str {
        match self {
            Self::Stones => text!("Stones"),
            Self::Bark => text!("Bark"),
            Self::Scales => text!("Scales"),
            Self::Stitches => text!("Stitches"),
            Self::Rivets => text!("Rivets"),
            Self::Cracks => text!("Cracks"),
            Self::Scallops => text!("Scallops"),
        }
    }

    /// What it's good for, in plain words (English: the key of its
    /// translation).
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Stones => {
                text!("Rounded stones with grooves between: cobbles, rocky ground, castle walls")
            }
            Self::Bark => text!("Long wavy grooves: tree bark, wood grain, stringy muscle"),
            Self::Scales => text!("Overlapping rounded scales: dragons, fish, armour"),
            Self::Stitches => {
                text!("A seam of slanted stitches: leather, cloth, patched creatures")
            }
            Self::Rivets => text!("Round rivet heads in rows: metal plates, armour, chests"),
            Self::Cracks => text!(
                "Thin branching cracks: hold Ctrl to dig them into stone, dry earth or old wood"
            ),
            Self::Scallops => text!("Rows of half-round ridges: roof tiles, shells, feathers"),
        }
    }

    /// Its value, 0 to 1, at `x`, `y` (each −1 to 1, `y` up).
    fn value(self, x: f32, y: f32) -> f32 {
        match self {
            Self::Stones => {
                let (f1, f2) = cells(x * 2.5, y * 2.5, 11);
                // Flat-topped stones that round off into the grooves.
                smoothstep(0.0, 0.35, f2 - f1) * (1.0 - 0.25 * f1)
            }
            Self::Bark => {
                let wave = (y * 3.0 + noise(x * 2.0, y * 1.5, 5) * 2.0).sin() * 0.12;
                let ridges = ((x + wave) * 6.0 * std::f32::consts::PI).sin();
                let grain = noise(x * 6.0, y * 1.2, 9) * 0.35;
                smoothstep(-0.35, 0.85, ridges + grain)
            }
            Self::Scales => {
                // Rows from the top down, each row under the one above,
                // so each scale shows as a rounded lump.
                let (across, down) = (0.4, 0.26);
                let row = ((1.0 - y) / down).floor() as i32;
                let mut best = 0.0f32;
                for r in (row - 1..=row + 2).rev() {
                    let shift = if r.rem_euclid(2) == 1 {
                        across * 0.5
                    } else {
                        0.0
                    };
                    let cy = 1.0 - r as f32 * down;
                    let cx = ((x - shift) / across).round() * across + shift;
                    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / (across * 0.62);
                    if d < 1.0 {
                        best = (1.0 - d * d).sqrt();
                    }
                }
                best
            }
            Self::Stitches => {
                // Slanted dashes across a seam running up the middle.
                let period = 0.25;
                let p = glam::Vec2::new(x, y - (y / period).round() * period);
                let (a, b) = (glam::Vec2::new(-0.3, -0.08), glam::Vec2::new(0.3, 0.08));
                let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
                let d = p.distance(a + (b - a) * t);
                1.0 - smoothstep(0.02, 0.07, d)
            }
            Self::Rivets => {
                let size = 0.5;
                let (cx, cy) = ((x / size).round() * size, (y / size).round() * size);
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / (size * 0.3);
                if d < 1.0 { (1.0 - d * d).sqrt() } else { 0.0 }
            }
            Self::Cracks => {
                let (f1, f2) = cells(x * 1.8, y * 1.8, 23);
                let wobble = noise(x * 8.0, y * 8.0, 3) * 0.04;
                1.0 - smoothstep(0.0, 0.07, f2 - f1 + wobble)
            }
            Self::Scallops => {
                let size = 0.5;
                let row = ((1.0 - y) / (size * 0.5)).floor();
                let shift = if (row as i32).rem_euclid(2) == 1 {
                    size * 0.5
                } else {
                    0.0
                };
                let cy = 1.0 - row * size * 0.5;
                let cx = ((x - shift) / size).round() * size + shift;
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / (size * 0.5);
                // A ridge rising towards each scallop's rounded rim.
                if d < 1.0 {
                    smoothstep(0.2, 0.95, d) * (1.0 - smoothstep(0.95, 1.0, d))
                } else {
                    0.0
                }
            }
        }
    }
}

/// The alpha made in code `kind`: made once, then shared.
pub fn made(kind: Made) -> Arc<Alpha> {
    static MADE: OnceLock<Vec<Arc<Alpha>>> = OnceLock::new();
    let all = MADE.get_or_init(|| Made::ALL.iter().map(|k| Arc::new(make(*k))).collect());
    let i = Made::ALL.iter().position(|k| *k == kind).unwrap_or(0);
    Arc::clone(&all[i])
}

fn make(kind: Made) -> Alpha {
    let side = MADE_SIDE;
    let mut values = Vec::with_capacity((side * side) as usize);
    for py in 0..side {
        for px in 0..side {
            let x = (px as f32 + 0.5) / side as f32 * 2.0 - 1.0;
            let y = 1.0 - (py as f32 + 0.5) / side as f32 * 2.0;
            let v = kind.value(x, y);
            values.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    let mut alpha = Alpha::new(side, side, values, kind.name())
        .unwrap_or_else(|_| unreachable!("a made alpha has pixels"));
    alpha.made = Some(kind);
    alpha
}

fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A steady pseudo-random number, 0 to 1, from two whole numbers and a
/// seed.
fn hash(a: i32, b: i32, seed: u32) -> f32 {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1)
        ^ (b as u32).wrapping_mul(0x85EB_CA77)
        ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth value noise, −1 to 1.
fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (x - x0, y - y0);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let (i, j) = (x0 as i32, y0 as i32);
    let c = |a: i32, b: i32| hash(a, b, seed);
    let top = c(i, j) + (c(i + 1, j) - c(i, j)) * sx;
    let bottom = c(i, j + 1) + (c(i + 1, j + 1) - c(i, j + 1)) * sx;
    (top + (bottom - top) * sy) * 2.0 - 1.0
}

/// The distances to the nearest and second nearest of a scatter of
/// points, one in each square of a grid (cell noise).
fn cells(x: f32, y: f32, seed: u32) -> (f32, f32) {
    let (i, j) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2) = (f32::MAX, f32::MAX);
    for b in j - 1..=j + 1 {
        for a in i - 1..=i + 1 {
            let px = a as f32 + 0.15 + 0.7 * hash(a, b, seed);
            let py = b as f32 + 0.15 + 0.7 * hash(a, b, seed ^ 0x5555);
            let d = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps() -> Alpha {
        // Two by two: black, white on top; grey, white below.
        Alpha::new(2, 2, vec![0, 255, 128, 255], "steps").unwrap()
    }

    #[test]
    fn an_alpha_reads_its_pixels_with_y_up() {
        let a = steps();
        // Pixel centres sit at ±0.5.
        assert!(a.at(-0.5, 0.5) < 1e-6, "top left is black");
        assert!((a.at(0.5, 0.5) - 1.0).abs() < 1e-6, "top right white");
        assert!((a.at(-0.5, -0.5) - 128.0 / 255.0).abs() < 1e-6);
        // Blended between pixels.
        let mid = a.at(0.0, 0.5);
        assert!((mid - 0.5).abs() < 1e-3, "{mid}");
        // Outside the picture, nothing.
        assert_eq!(a.at(1.01, 0.0), 0.0);
        assert_eq!(a.at(0.0, -1.5), 0.0);
        assert_eq!(a.at(f32::NAN, 0.0), 0.0);
        // The edges keep their pixels' values.
        assert!((a.at(1.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_wide_alpha_is_centred_in_its_square() {
        // Four across, two down: the top and bottom quarters are empty.
        let a = Alpha::new(4, 2, vec![255; 8], "wide").unwrap();
        assert!((a.at(0.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((a.at(0.0, 0.45) - 1.0).abs() < 1e-6);
        assert_eq!(a.at(0.0, 0.8), 0.0);
        assert_eq!(a.at(0.0, -0.8), 0.0);
        assert!((a.at(-0.99, 0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bad_alphas_are_refused() {
        assert!(Alpha::new(0, 4, vec![], "none").is_err());
        assert!(Alpha::new(MAX_ALPHA_SIDE + 1, 1, vec![1; 1025], "wide").is_err());
        assert!(Alpha::new(2, 2, vec![1; 3], "short").is_err());
        assert!(Alpha::new(2, 2, vec![0; 4], "black").is_err());
        let long = "x".repeat(500);
        assert_eq!(
            Alpha::new(1, 1, vec![9], &long).unwrap().name().len(),
            MAX_ALPHA_NAME
        );
    }

    #[test]
    fn every_made_alpha_has_a_picture_of_its_own() {
        let mut keys = std::collections::HashSet::new();
        for kind in Made::ALL {
            let a = made(kind);
            assert_eq!(a.made(), Some(kind));
            assert_eq!(a.name(), kind.name());
            assert!(!kind.explanation().is_empty());
            assert_eq!((a.width(), a.height()), (MADE_SIDE, MADE_SIDE));
            let lit = a.values().iter().filter(|v| **v > 128).count();
            let dark = a.values().iter().filter(|v| **v < 64).count();
            let all = a.values().len();
            // Each has both raised and empty parts: a texture, not a blob.
            assert!(lit > all / 50, "{kind:?} has too little white ({lit})");
            assert!(dark > all / 100, "{kind:?} has too little black ({dark})");
            assert!(keys.insert(a.key()), "{kind:?} is like another");
            // Made once, then shared.
            assert!(Arc::ptr_eq(&a, &made(kind)));
        }
    }

    #[test]
    fn a_laid_alpha_turns_with_its_up_and_angle() {
        let a = steps();
        let alpha = DabAlpha {
            alpha: &a,
            up: Vec3::Y,
            angle: 0.0,
            follow: false,
        };
        let laid = Laid::new(&alpha, Vec3::ZERO, 2.0, Vec3::Z, None);
        // The top right pixel lies up and right of the middle.
        assert!((laid.at(Vec3::new(1.0, 1.0, 0.0)) - 1.0).abs() < 1e-6);
        assert!(laid.at(Vec3::new(-1.0, 1.0, 0.0)) < 1e-6);
        // Turned a quarter anticlockwise: the top points left, so the
        // black top left pixel now lies down and left.
        let turned = Laid::new(
            &DabAlpha {
                angle: std::f32::consts::FRAC_PI_2,
                ..alpha
            },
            Vec3::ZERO,
            2.0,
            Vec3::Z,
            None,
        );
        assert!(turned.at(Vec3::new(-1.0, -1.0, 0.0)) < 1e-6);
        assert!((turned.at(Vec3::new(-1.0, 1.0, 0.0)) - 1.0).abs() < 1e-6);
        // Following the stroke: the top points the way it went.
        let follow = Laid::new(
            &DabAlpha {
                follow: true,
                ..alpha
            },
            Vec3::ZERO,
            2.0,
            Vec3::Z,
            Some(Vec3::X * 3.0),
        );
        assert!(follow.at(Vec3::new(1.0, 1.0, 0.0)) < 1e-6, "top left now");
        // An up along the facing itself still lays the picture somehow.
        let edge = Laid::new(
            &DabAlpha {
                up: Vec3::Z,
                ..alpha
            },
            Vec3::ZERO,
            2.0,
            Vec3::Z,
            None,
        );
        assert!(edge.at(Vec3::ZERO).is_finite());
    }
}
