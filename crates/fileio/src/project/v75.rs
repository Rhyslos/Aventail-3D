//! Version 75 of the project file ([decision 0199](../../../../docs/decisions/0199-kit-finishing-touches.md)):
//! version 74's, with what each gradient runs across (`v72::GradientV72::span`:
//! each object, part or UV piece) and how warm and cool each
//! light-and-shade layer is (`v74::LightingV74::warmth`). Older files
//! read as each object and fully warm and cool, as those versions drew
//! them; the fields live with their structures, so this module only
//! records the version.
