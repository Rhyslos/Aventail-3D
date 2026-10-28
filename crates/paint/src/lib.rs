//! Texture painting ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! texture sets by material, their layers, tiled images, and mixing the
//! layers into what a surface shows. Nothing here knows about scenes,
//! undo, files, the GPU or the interface.

pub mod blend;
pub mod brush;
pub mod composite;
pub mod fill;
pub mod flat;
pub mod follow;
pub mod gradient;
pub mod image;
pub mod lighting;
pub mod palette;
pub mod resize;
pub mod set;
pub mod smart;
pub mod stack;
pub mod stroke;
pub mod surface;
pub mod tip;

pub use blend::Blend;
pub use brush::{Brush, BrushKind};
pub use composite::Composite;
pub use fill::FillBy;
pub use gradient::{Gradient, GradientAxis, GradientSpan};
pub use image::{Image, TILE, TILE_BYTES, Tile};
pub use lighting::Lighting;
pub use set::{Content, DabRecord, FillRecord, Layer, Mask, PaintStore, StrokeRecord, TextureSet};
pub use smart::{SmartKind, SmartMask};
pub use stack::LayerPath;
pub use stroke::{
    Dab, DabJob, DepthMap, Source, Stencil, StrokeCanvas, Target, View, paint_dab, paint_dabs,
};
pub use surface::PaintSurface;
pub use tip::Tip;
