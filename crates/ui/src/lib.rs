//! The interface: the dockable panel layout, menus, palettes, the
//! Attributes and Objects panels, and the theme.
//!
//! Layer 3 (front end). Changes the scene only by executing commands. See
//! this crate's README for its purpose and limits.
//!
//! The app's name is never written here: it arrives in [`AppInfo`] from
//! the app's branding.

mod attributes;
mod autosave;
mod booleans;
pub mod brush_bar;
mod command_search;
pub mod commands;
mod components;
mod coordinates;
mod dialog;
mod drawing;
mod edge_values;
pub mod icons;
mod keyboard;
mod kit;
mod layers;
pub mod layout;
mod modifier_list;
mod modifiers;
mod objects;
mod paint_layers;
pub mod pen;
pub use pen::PenState;
mod paint_view;
pub mod palette;
mod panel;
mod panels;
mod project;
mod quad_menu;
mod selecting;
mod settings;
mod shape_picker;
pub mod splash;
pub mod text;
// The language layer's macros, as `crate::trf!` and `crate::text!`.
pub use words::{text, trf};
#[cfg(test)]
mod text_checks;
pub mod theme;
mod tool_menu;
mod transforming;
mod undo_memory;
mod units;
mod uv_editor;
mod viewport;
#[cfg(test)]
pub(crate) use viewport::chrome as viewport_chrome;
mod workbench;

pub use autosave::{AutosaveSettings, COPIES, MINUTES, RecoveryCopy, RecoveryOffer, ago};
pub use drawing::{
    ComponentOverlay, CurveDrawing, DrawItem, OverlayStyle, SceneDrawing, SceneSummary,
};
pub use egui_dock::DockState;
pub use keyboard::KeyboardLayout;
pub use objects::{
    DELETE_SHORTCUT, DESELECT_SHORTCUT, GROUP_SHORTCUT, MAKE_EDITABLE_SHORTCUT, REDO_SHORTCUT,
    REDO_SHORTCUT_2, UNDO_SHORTCUT, UNGROUP_SHORTCUT,
};
pub use paint_view::{PaintedTexture, texture_key};
pub use palette::Mode;
pub use panel::Panel;
pub use project::{NEW_SHORTCUT, OPEN_SHORTCUT, RECENT_COUNT, SAVE_AS_SHORTCUT, SAVE_SHORTCUT};
pub use selecting::{
    BRUSH_MAX, BRUSH_MIN, BRUSH_RADIUS, BRUSH_STEP, CONNECTED_SHORTCUT, INVERT_SHORTCUT,
    SELECT_ALL_SHORTCUT, SELECT_TOOL_KEY, SelectKeys, SelectTool,
};
pub use settings::SETTINGS_SHORTCUT;
pub use shape_picker::SHAPE_LIST_KEY;
pub use tool_menu::{TOOL_MENU_KEY, ToolMenuSettings, ToolMenuStyle};
pub use transforming::{AXES_KEY, Axes, LOCK_KEYS, SnapSettings, TOOL_KEYS, Tool};
pub use undo_memory::{UndoMemory, format_bytes};
pub use units::Units;
pub use viewport::{
    AUTO_HARD_ANGLE, DEFAULT_FRAME, FOUR_VIEWS_KEY, Frame, GLIDE_SECONDS, GRID_IMAGE_SIZE,
    GridDetail, NoRenderer, PICKER_LABEL, PaintPreview, PickAsk, PickReply, Picked, Shading,
    Surface, Symmetry, VIEW_COUNT, VIEW_KEYS, VIEWPORT_LABEL, ViewOptions, ViewportPanel,
    ViewportRenderer, ViewportView, checker_squares, grid_image,
};
pub use workbench::{
    AppInfo, DISCARD_SHORTCUT, DOUBLE_CLICK_SECONDS, ExportChoice, ExportRequest, ExportSettings,
    FilePurpose, FileWork, KitOutcome, KitRequest, MODE_KEYS, TargetOutcome, TargetRequest,
    UpdateOffer, Workbench, WorkbenchAction, navigation_hint,
};
pub use workbench::{UnwrapWay, UvSettings};
