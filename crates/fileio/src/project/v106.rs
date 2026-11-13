//! Version 106 of the project file ([decision 0281](../../../../docs/decisions/0281-palette-map.md)):
//! version 105's, where a material graph's setting may name one of the
//! project's colour palettes (`v95::ValueV95::Palette`, the palette's
//! name; Palette map's Palette). The palettes themselves are stored as
//! since version 68 (`v68`). An older app can't read such a setting, so
//! the version goes up; files before version 106 hold none.
//!
//! A name is kept as written (brought within `matgraph::MAX_PALETTE_NAME`
//! characters when the graph is read). A setting naming a palette the
//! project doesn't have opens as it is: the box says why it can't run and
//! offers the project's palettes.
