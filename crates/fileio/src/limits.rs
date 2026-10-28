//! The limits rulebook: how much a file may ask of the app
//! ([decision 0045](../../../docs/decisions/0045-limits-for-files.md)).
//!
//! A project file can come from anywhere: a friend, a download, a damaged
//! disk. Without limits, a few kilobytes could ask for gigabytes of memory
//! or hours of work, and crash or freeze the app. Every file the app reads
//! is held to these limits *while it is read*, so nothing is allocated
//! before it has been checked. Hitting a limit gives a plain message
//! ("the file is damaged: …"), never a crash.
//!
//! | Limit | Value | Why |
//! |---|---|---|
//! | Unpacked body | [`MAX_UNPACKED`] | A tiny compressed file can unpack to gigabytes (a "zip bomb") |
//! | Memory for the file's contents | [`MAX_MEMORY`] | Lists can claim more items than the file holds bytes for |
//! | Objects | [`MAX_OBJECTS`] | |
//! | Recorded edits, in all | [`MAX_EDITS`] | Each edit is replayed when the shape is worked out |
//! | Polygons, estimated, in all | [`MAX_POLYGONS`] | Building shapes takes time and memory in proportion |
//!
//! The polygon limit counts plain meshes as stored and estimates
//! primitives from their settings. While shapes are worked out, the
//! scene's own guard stops any edit that would make a shape too large
//! (see `scene::stack`), so a file can't get round it with edits that
//! grow the shape.
//!
//! These limits are far above anything made in the app: the heavy-mesh
//! check's 3-million-polygon project unpacks to about 90 MB and needs
//! about 200 MB of memory to read.

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::{Serialize, Serializer};
use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use words::{text, tr};

/// The largest a project's body may be once unpacked: 512 MB.
pub const MAX_UNPACKED: u64 = 512 << 20;
/// The most memory a project's contents may take while being read: 2 GB.
pub const MAX_MEMORY: usize = 2 << 30;
/// The most objects a project may hold.
pub const MAX_OBJECTS: usize = 100_000;
/// The most recorded edits a project may hold, in all its objects.
pub const MAX_EDITS: usize = 100_000;
/// The most polygons a project's shapes may have before their edits, in
/// all, estimated from primitives' settings and counted for plain meshes.
pub const MAX_POLYGONS: usize = 30_000_000;

thread_local! {
    /// Memory left for the contents being read on this thread; `None`
    /// when no limited read is under way.
    static LEFT: Cell<Option<usize>> = const { Cell::new(None) };
    /// Whether the read under way ran out of budget.
    static HIT: Cell<bool> = const { Cell::new(false) };
}

/// The plain message for a read that ran out of budget (English: show it
/// with `tr`).
pub const TOO_MUCH: &str = text!(
    "it holds more than the app can open (a limit was reached; it may be damaged or made to \
     cause trouble)"
);

/// Whether the last limited read on this thread ran out of budget.
pub(crate) fn limit_hit() -> bool {
    HIT.with(Cell::get)
}

/// Runs `read` with [`MAX_MEMORY`] to spend on [`Bounded`] lists (or
/// less, inside a read already limited further, as tests do).
pub(crate) fn within_budget<T>(read: impl FnOnce() -> T) -> T {
    let budget = LEFT
        .with(Cell::get)
        .map_or(MAX_MEMORY, |left| left.min(MAX_MEMORY));
    within(budget, read)
}

/// Runs `read` with `budget` bytes to spend on [`Bounded`] lists (the
/// previous budget, if any, comes back afterwards).
pub(crate) fn within<T>(budget: usize, read: impl FnOnce() -> T) -> T {
    let before = LEFT.with(|l| l.replace(Some(budget)));
    HIT.with(|h| h.set(false));
    let out = read();
    LEFT.with(|l| l.set(before));
    out
}

/// Takes `bytes` from the budget; `false` if there isn't enough left.
pub(crate) fn spend(bytes: usize) -> bool {
    LEFT.with(|l| match l.get() {
        None => true,
        Some(left) if left >= bytes => {
            l.set(Some(left - bytes));
            true
        }
        Some(_) => {
            l.set(Some(0));
            HIT.with(|h| h.set(true));
            false
        }
    })
}

/// A list in a file model. It is saved exactly like a plain list, but
/// while it is read every item is paid for from the read's memory budget
/// (its size, at least one byte), and the read fails as soon as the budget
/// runs out: before the memory is taken.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bounded<T>(pub Vec<T>);

impl<T> Bounded<T> {
    /// An empty list.
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<T> Deref for Bounded<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> DerefMut for Bounded<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T> From<Vec<T>> for Bounded<T> {
    fn from(list: Vec<T>) -> Self {
        Self(list)
    }
}

impl<T> FromIterator<T> for Bounded<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        Self(items.into_iter().collect())
    }
}

impl<T> IntoIterator for Bounded<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Bounded<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: Serialize> Serialize for Bounded<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Bounded<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Items<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for Items<T> {
            type Value = Bounded<T>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(tr("a list"))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let each = size_of::<T>().max(1);
                let too_much = || serde::de::Error::custom(tr(TOO_MUCH));
                // Pay for what the list says it holds before making room.
                let claimed = seq.size_hint().unwrap_or(0);
                if !spend(claimed.saturating_mul(each)) {
                    return Err(too_much());
                }
                let mut items = Vec::with_capacity(claimed.min(1 << 16));
                while let Some(item) = seq.next_element()? {
                    if items.len() >= claimed && !spend(each) {
                        return Err(too_much());
                    }
                    items.push(item);
                }
                Ok(Bounded(items))
            }
        }

        deserializer.deserialize_seq(Items(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bounded_list_saves_like_a_plain_one() {
        let plain = vec![1u32, 2, 3];
        let bounded = Bounded(plain.clone());
        assert_eq!(
            postcard::to_stdvec(&plain).unwrap(),
            postcard::to_stdvec(&bounded).unwrap()
        );
        let back: Bounded<u32> =
            postcard::from_bytes(&postcard::to_stdvec(&plain).unwrap()).unwrap();
        assert_eq!(back, bounded);
    }

    #[test]
    fn reading_more_than_the_budget_fails_before_taking_the_memory() {
        let list = vec![0u64; 1000];
        let bytes = postcard::to_stdvec(&list).unwrap();
        let fits = within(8000, || postcard::from_bytes::<Bounded<u64>>(&bytes));
        assert!(fits.is_ok());
        let too_big = within(7999, || postcard::from_bytes::<Bounded<u64>>(&bytes));
        assert!(too_big.is_err());
        // A list claiming far more items than the bytes hold is refused at
        // once, not after allocating.
        let mut lying = postcard::to_stdvec(&u32::MAX).unwrap();
        lying.extend_from_slice(&[0; 8]);
        let read = within(1 << 20, || {
            postcard::from_bytes::<Bounded<Option<Vec<u8>>>>(&lying)
        });
        assert!(read.is_err());
        // Nested lists share one budget.
        let nested = vec![vec![0u8; 100]; 100];
        let bytes = postcard::to_stdvec(&nested).unwrap();
        let read = within(5000, || {
            postcard::from_bytes::<Bounded<Bounded<u8>>>(&bytes)
        });
        assert!(read.is_err());
    }
}
