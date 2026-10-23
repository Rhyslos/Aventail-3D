//! The interface's words: the language layer from the `words` crate
//! (decisions 0132 and 0134), kept here under its old path so the
//! interface's code reads `crate::text::tr`, `crate::trf!` and
//! `crate::text!` as before. See the `words` crate for how it works.

pub use words::{Package, escape, fill, tr, use_mac_keys, use_package};
