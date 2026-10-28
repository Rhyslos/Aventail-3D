//! Version 66 of the project file ([decision 0181](../../../../docs/decisions/0181-paint-layers.md)):
//! version 65's, with the Paint layers panel's kinds of layer and their
//! settings, stored in `v64::LayerV64`: whether a layer is clipped to the
//! one below (`clipped`), a fill layer's colour (`fill`), a group's layers
//! (`children`) and whether they are listed (`expanded`), and blend modes
//! beyond Normal (`blend`, 0 to 7). Files before version 66 hold paint
//! layers in Normal only.
//!
//! An older app would mix the layers wrongly and drop fills and groups on
//! saving, so it gets a version of its own: the older app says it needs
//! updating instead.
