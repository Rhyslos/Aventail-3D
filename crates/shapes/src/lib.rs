//! Vector shapes for the shape canvas ([decision 0273](../../../docs/decisions/0273-shape-canvas.md),
//! the plan in `docs/plans/phase-12.md`): paths with straight and curved
//! sides, cut-outs, a mirror line, and turning them into a sharp mask at
//! any texture size.
//!
//! Layer 1 (core). No files, GPU or interface: everything here can be
//! tested without a window. See this crate's README for its purpose and
//! limits.
//!
//! - [`Path`] and [`Knot`] (`path`): points with two handles each, joined
//!   by straight or curved (cubic Bézier) sides, open or closed;
//!   measuring (area, inside, nearest place) through `kurbo`.
//! - [`Basic`] (`basic`): a circle, square, polygon or star as a path.
//! - [`Drawing`] and [`Piece`] (`drawing`): the pieces on one canvas in
//!   order (Add, Cut, Line), the mirror line and the reference image;
//!   every change checked, saying why it can't be made ([`Problem`]).
//!   Pieces are placed apart from their shapes, so linked copies share
//!   one shape; copies, links, groups and the order are in
//!   `drawing::assembly`.
//! - [`Edge`] (`edge`): the edge randomiser roughening an outline
//!   (jagged, serrated, wavy, torn); varied copies and veins are in
//!   `drawing::variety`.
//! - [`Reference`] (`reference`): the reference image under the canvas.
//! - [`raster`]: the filled shape and the lines at any size, wrapping
//!   round the texture's edges, filled by `tiny-skia`.
//! - [`Sheet`] (`sheet`): a drawing's leaves and twigs laid out apart on
//!   one texture (a leaf sheet, or atlas) by the UV packer, for alpha
//!   cards.

pub mod basic;
pub mod drawing;
pub mod edge;
pub mod path;
pub mod raster;
pub mod reference;
pub mod sheet;

pub use basic::Basic;
pub use drawing::{
    Drawing, DrawingParts, Group, GroupId, LinkId, Mirror, Piece, PieceCopies, PieceId, PieceKind,
    Problem, Variation, Vary, Vein,
};
pub use edge::{Edge, EdgeKind};
pub use path::{Change, Knot, Path};
pub use reference::Reference;
pub use sheet::{Sheet, SheetOptions};
