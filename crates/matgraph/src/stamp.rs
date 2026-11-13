//! Small pictures of your own brought in on a Stamps box (Phase 12
//! milestone 22, [decision 0285](../../../docs/decisions/0285-stamp-images.md)):
//! a rivet, a leaf, a rune, scattered as stamps.
//!
//! A [`Stamp`] keeps its pixels as sRGB bytes with straight alpha, four a
//! pixel (red, green, blue, see-through), row by row from the top, at most
//! [`MAX_STAMP_SIDE`] pixels across or down: a stamp is small, and its
//! see-through parts are what give it its outline. [`Stamps`] is the list
//! a box holds, each with a **weight** (how often it is picked, 1 to
//! [`MAX_WEIGHT`]).
//!
//! As with photos ([`crate::Photo`]), the crate never reads image files
//! itself: whoever brings a stamp in (`fileio`) decodes it and hands over
//! the pixels with the bytes it will save (`packed`, a PNG, so the
//! see-through parts are kept exactly), so saving never packs a stamp
//! again and a stamp saved and opened again has the same pixels.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// The widest or tallest a stamp is kept, in pixels: bigger pictures are
/// made smaller when brought in (a stamp covers a small part of a
/// texture, so more would only make projects bigger).
pub const MAX_STAMP_SIDE: u32 = 512;

/// The most stamps one box holds.
pub const MAX_STAMPS: usize = 16;

/// The most bytes one stamp's packed form may take (4 MB).
pub const MAX_STAMP_PACKED: usize = 4 << 20;

/// The most bytes all of one box's stamps may take packed (8 MB), so a
/// graph file holding them stays within what is read.
pub const MAX_STAMPS_PACKED: usize = 8 << 20;

/// The highest weight a stamp can have: picked ten times as often as a
/// stamp of weight 1.
pub const MAX_WEIGHT: u32 = 10;

/// One small picture brought in on a Stamps box.
#[derive(Clone)]
pub struct Stamp {
    width: u32,
    height: u32,
    rgba: Arc<Vec<u8>>,
    packed: Arc<Vec<u8>>,
    name: String,
    key: u64,
}

impl std::fmt::Debug for Stamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stamp")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("packed", &self.packed.len())
            .field("name", &self.name)
            .field("key", &self.key)
            .finish()
    }
}

impl PartialEq for Stamp {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.key == other.key
            && self.name == other.name
            && (Arc::ptr_eq(&self.rgba, &other.rgba) || self.rgba == other.rgba)
    }
}

impl Stamp {
    /// A stamp `width` × `height` pixels from sRGB bytes with straight
    /// alpha `rgba` (four a pixel, row by row from the top), with the
    /// bytes it is saved as (`packed`, which may be empty in tests) and
    /// its file's `name` (cut to [`crate::photo::MAX_PHOTO_NAME`]); or why
    /// it can't be one, as a file's would be checked.
    pub fn new(
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        packed: Vec<u8>,
        name: &str,
    ) -> Result<Self, &'static str> {
        if width == 0 || height == 0 || width > MAX_STAMP_SIDE || height > MAX_STAMP_SIDE {
            return Err(words::text!(
                "a stamp is bigger than the app keeps, or has no pixels"
            ));
        }
        if rgba.len() != width as usize * height as usize * 4 {
            return Err(words::text!("a stamp has the wrong number of pixels"));
        }
        if packed.len() > MAX_STAMP_PACKED {
            return Err(words::text!("a stamp takes more room than the app keeps"));
        }
        if rgba.chunks_exact(4).all(|p| p[3] == 0) {
            return Err(words::text!(
                "a stamp is see-through everywhere, so there is nothing to stamp"
            ));
        }
        let mut hasher = DefaultHasher::new();
        (width, height).hash(&mut hasher);
        rgba.hash(&mut hasher);
        Ok(Self {
            width,
            height,
            rgba: Arc::new(rgba),
            packed: Arc::new(packed),
            name: crate::photo::cut(name, crate::photo::MAX_PHOTO_NAME),
            key: hasher.finish(),
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

    /// Its pixels: sRGB bytes with straight alpha, four a pixel, row by
    /// row from the top.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// The bytes it is saved as (a PNG, from whoever brought it in).
    pub fn packed(&self) -> &[u8] {
        &self.packed
    }

    /// Its file's name, for saying which stamp is which.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// A hash of its size and pixels.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// Whether any of it is see-through (a stamp with none covers its
    /// whole square).
    pub fn has_see_through(&self) -> bool {
        self.rgba.chunks_exact(4).any(|p| p[3] < 255)
    }

    /// Roughly how much memory it takes, in bytes.
    pub fn memory_size(&self) -> usize {
        96 + self.rgba.len() + self.packed.len() + self.name.len()
    }

    /// Whether `a` and `b` share their pixels in memory.
    pub fn shares(a: &Stamp, b: &Stamp) -> bool {
        Arc::ptr_eq(&a.rgba, &b.rgba)
    }
}

/// One stamp in a box's list, with how often it is picked.
#[derive(Debug, Clone, PartialEq)]
pub struct StampEntry {
    /// The picture.
    pub stamp: Arc<Stamp>,
    /// How often it is picked, from 1 to [`MAX_WEIGHT`]: a stamp of
    /// weight 2 is picked about twice as often as one of weight 1.
    pub weight: u32,
}

/// The stamps a Stamps box holds, in order (at least one).
#[derive(Debug, Clone, PartialEq)]
pub struct Stamps {
    entries: Vec<StampEntry>,
    key: u64,
}

impl Stamps {
    /// The list `entries`, weights brought into range; or why it can't be
    /// held (none, too many, or too big packed together).
    pub fn new(mut entries: Vec<StampEntry>) -> Result<Self, &'static str> {
        if entries.is_empty() {
            return Err(words::text!("a Stamps box's list has no stamps"));
        }
        if entries.len() > MAX_STAMPS {
            return Err(words::text!(
                "a Stamps box holds at most 16 stamps: take one off first"
            ));
        }
        let packed: usize = entries.iter().map(|e| e.stamp.packed().len()).sum();
        if packed > MAX_STAMPS_PACKED {
            return Err(words::text!(
                "a Stamps box's stamps together take more room than the app keeps (8 MB): \
                 use smaller pictures, or take one off first"
            ));
        }
        let mut hasher = DefaultHasher::new();
        for e in &mut entries {
            e.weight = e.weight.clamp(1, MAX_WEIGHT);
            (e.stamp.key(), e.weight).hash(&mut hasher);
        }
        Ok(Self {
            entries,
            key: hasher.finish(),
        })
    }

    /// The stamps in order.
    pub fn entries(&self) -> &[StampEntry] {
        &self.entries
    }

    /// How many stamps there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Never empty; for symmetry with [`Self::len`].
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// A hash of every stamp's pixels and weight, in order.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// The list with `stamp` added at the end, weight 1.
    pub fn with_added(&self, stamp: Stamp) -> Result<Self, &'static str> {
        let mut entries = self.entries.clone();
        entries.push(StampEntry {
            stamp: Arc::new(stamp),
            weight: 1,
        });
        Self::new(entries)
    }

    /// A list holding only `stamp`.
    pub fn one(stamp: Stamp) -> Result<Self, &'static str> {
        Self::new(vec![StampEntry {
            stamp: Arc::new(stamp),
            weight: 1,
        }])
    }

    /// The list with stamp `index`'s weight set to `weight` (brought into
    /// range); unchanged for a stamp it doesn't hold.
    pub fn with_weight(&self, index: usize, weight: u32) -> Self {
        let mut entries = self.entries.clone();
        if let Some(e) = entries.get_mut(index) {
            e.weight = weight;
        }
        Self::new(entries).unwrap_or_else(|_| self.clone())
    }

    /// The list without stamp `index`; `None` when nothing would be left.
    pub fn without(&self, index: usize) -> Option<Self> {
        let mut entries = self.entries.clone();
        if index < entries.len() {
            entries.remove(index);
        }
        Self::new(entries).ok()
    }

    /// Which stamp a copy given the random number `unit` (0 to under 1)
    /// gets: each stamp's share of the numbers is its weight's share of
    /// all the weights.
    pub fn pick(&self, unit: f32) -> usize {
        let total: u32 = self.entries.iter().map(|e| e.weight).sum();
        let at = (unit.clamp(0.0, 1.0) * total as f32) as u32;
        let mut sum = 0;
        for (i, e) in self.entries.iter().enumerate() {
            sum += e.weight;
            if at < sum {
                return i;
            }
        }
        self.entries.len() - 1
    }

    /// Roughly how much memory the list takes, in bytes.
    pub fn memory_size(&self) -> usize {
        64 + self
            .entries
            .iter()
            .map(|e| 16 + e.stamp.memory_size())
            .sum::<usize>()
    }

    /// What an undo step changing `a` into `b` holds on to besides what the
    /// project holds anyway: the stamps the two don't share (changing a
    /// weight shares them all, so holds on to almost nothing).
    pub fn memory_apart(a: Option<&Stamps>, b: Option<&Stamps>) -> usize {
        let a_entries = a.map_or(&[][..], |s| s.entries());
        let b_entries = b.map_or(&[][..], |s| s.entries());
        let shared = |e: &StampEntry, others: &[StampEntry]| {
            others.iter().any(|o| Stamp::shares(&e.stamp, &o.stamp))
        };
        let only = |mine: &[StampEntry], others: &[StampEntry]| -> usize {
            mine.iter()
                .filter(|e| !shared(e, others))
                .map(|e| e.stamp.memory_size())
                .sum()
        };
        let lists = [a, b].iter().flatten().count() * 64;
        lists + only(a_entries, b_entries) + only(b_entries, a_entries)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A stamp `w` × `h`: a round dot of `colour` on see-through.
    pub(crate) fn dot(w: u32, h: u32, colour: [u8; 3]) -> Stamp {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let r = cx.min(cy);
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let inside = (dx * dx + dy * dy).sqrt() < r * 0.9;
                if inside {
                    rgba.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
                } else {
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
        Stamp::new(w, h, rgba, Vec::new(), "dot.png").unwrap()
    }

    #[test]
    fn damaged_stamps_are_refused() {
        assert!(Stamp::new(0, 4, Vec::new(), Vec::new(), "x").is_err());
        let side = MAX_STAMP_SIDE + 1;
        let big = vec![255; (side * 4) as usize];
        assert!(Stamp::new(side, 1, big, Vec::new(), "x").is_err());
        assert!(Stamp::new(2, 2, vec![255; 15], Vec::new(), "x").is_err());
        assert!(Stamp::new(2, 2, vec![255; 16], vec![0; MAX_STAMP_PACKED + 1], "x").is_err());
        // See-through everywhere: nothing to stamp.
        let why = Stamp::new(2, 2, vec![0; 16], Vec::new(), "x");
        assert!(why.unwrap_err().contains("see-through everywhere"));
        assert!(Stamp::new(2, 2, vec![255; 16], Vec::new(), "x").is_ok());
    }

    #[test]
    fn the_key_follows_the_pixels_and_alpha_counts() {
        let a = dot(8, 8, [200, 10, 10]);
        assert_eq!(a, dot(8, 8, [200, 10, 10]));
        assert_ne!(a.key(), dot(8, 8, [10, 200, 10]).key());
        let mut rgba = a.rgba().to_vec();
        rgba[3] = 7;
        let b = Stamp::new(8, 8, rgba, Vec::new(), "dot.png").unwrap();
        assert_ne!(a.key(), b.key());
        assert!(a.has_see_through());
        let solid = Stamp::new(1, 1, vec![1, 2, 3, 255], Vec::new(), "s").unwrap();
        assert!(!solid.has_see_through());
    }

    #[test]
    fn a_list_keeps_its_limits() {
        assert!(Stamps::new(Vec::new()).is_err());
        let mut list = Stamps::one(dot(4, 4, [1, 2, 3])).unwrap();
        for _ in 1..MAX_STAMPS {
            list = list.with_added(dot(4, 4, [1, 2, 3])).unwrap();
        }
        assert_eq!(list.len(), MAX_STAMPS);
        let why = list.with_added(dot(4, 4, [1, 2, 3])).unwrap_err();
        assert!(why.contains("16"), "{why}");
        // Too big packed together.
        let packed = |n| Stamp::new(1, 1, vec![1, 1, 1, 255], vec![0; n], "p").unwrap();
        let two = Stamps::one(packed(MAX_STAMP_PACKED)).unwrap();
        let two = two.with_added(packed(MAX_STAMP_PACKED)).unwrap();
        assert!(two.with_added(packed(1)).is_err());
        // Weights kept from 1 to the most.
        let w = Stamps::one(dot(4, 4, [1, 2, 3])).unwrap();
        assert_eq!(w.with_weight(0, 0).entries()[0].weight, 1);
        assert_eq!(w.with_weight(0, 99).entries()[0].weight, MAX_WEIGHT);
        assert_eq!(w.with_weight(5, 3), w);
        assert_ne!(w.with_weight(0, 3).key(), w.key());
        // The last one can't be taken off: the box then holds none.
        assert!(w.without(0).is_none());
        let pair = w.with_added(dot(6, 6, [9, 9, 9])).unwrap();
        assert_eq!(pair.without(0).unwrap().entries()[0].stamp.width(), 6);
    }

    #[test]
    fn picking_follows_the_weights() {
        let list = Stamps::one(dot(4, 4, [1, 2, 3]))
            .unwrap()
            .with_added(dot(4, 4, [3, 2, 1]))
            .unwrap()
            .with_weight(1, 3);
        let mut counts = [0usize; 2];
        for i in 0..4000 {
            counts[list.pick(i as f32 / 4000.0)] += 1;
        }
        assert_eq!(counts, [1000, 3000]);
        assert_eq!(list.pick(0.0), 0);
        assert_eq!(list.pick(0.9999), 1);
        assert_eq!(list.pick(5.0), 1);
    }

    #[test]
    fn changing_a_weight_holds_on_to_no_more_pictures() {
        let list = Stamps::one(dot(64, 64, [1, 2, 3])).unwrap();
        let heavier = list.with_weight(0, 5);
        let apart = Stamps::memory_apart(Some(&list), Some(&heavier));
        let added = list.with_added(dot(64, 64, [4, 5, 6])).unwrap();
        let more = Stamps::memory_apart(Some(&list), Some(&added));
        assert!(apart < 1000, "{apart}");
        assert!(more > apart + 64 * 64 * 4 - 1, "{more} {apart}");
        assert!(Stamps::memory_apart(None, None) == 0);
    }
}
