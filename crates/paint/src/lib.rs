//! Texture painting ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! texture sets by material, their layers, tiled images, and mixing the
//! layers into what a surface shows. Nothing here knows about scenes,
//! undo, files, the GPU or the interface.

pub mod composite;
pub mod image;
pub mod set;

pub use composite::Composite;
pub use image::{Image, TILE, TILE_BYTES, Tile};
pub use set::{Blend, Content, Layer, PaintStore, TextureSet};
