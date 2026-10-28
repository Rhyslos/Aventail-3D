//! Texture painting ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! texture sets by material, their layers, tiled images, and mixing the
//! layers into what a surface shows. Nothing here knows about scenes,
//! undo, files, the GPU or the interface.

pub mod blend;
pub mod brush;
pub mod composite;
pub mod flat;
pub mod image;
pub mod set;
pub mod stack;
pub mod stroke;
pub mod surface;

pub use blend::Blend;
pub use brush::{Brush, BrushKind};
pub use composite::Composite;
pub use image::{Image, TILE, TILE_BYTES, Tile};
pub use set::{Content, DabRecord, Layer, PaintStore, StrokeRecord, TextureSet};
pub use stack::LayerPath;
pub use stroke::{Dab, DepthMap, StrokeCanvas, Target, View, paint_dab};
pub use surface::PaintSurface;
