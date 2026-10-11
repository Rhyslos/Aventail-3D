//! The interface: the dockable panel layout, menus, palettes, the
//! Attribute and Object Managers, and the theme.
//!
//! Layer 3 (front end). Changes the scene only by executing commands. See
//! this crate's README for its purpose and limits.
//!
//! The app's name is never written here: it arrives in [`AppInfo`] from
//! the app's branding.

mod attributes;
mod components;
mod drawing;
pub mod icons;
pub mod layout;
mod objects;
pub mod palette;
mod panel;
mod panels;
mod project;
mod selecting;
mod settings;
mod shape_picker;
pub mod theme;
mod undo_memory;
mod units;
mod viewport;
#[cfg(test)]
pub(crate) use viewport::chrome as viewport_chrome;
mod workbench;

pub use drawing::{ComponentOverlay, DrawItem, OverlayStyle, SceneDrawing, SceneSummary};
pub use egui_dock::DockState;
pub use objects::{
    DELETE_SHORTCUT, DESELECT_SHORTCUT, GROUP_SHORTCUT, MAKE_EDITABLE_SHORTCUT, REDO_SHORTCUT,
    REDO_SHORTCUT_2, UNDO_SHORTCUT, UNGROUP_SHORTCUT,
};
pub use palette::Mode;
pub use panel::Panel;
pub use project::{NEW_SHORTCUT, OPEN_SHORTCUT, RECENT_COUNT, SAVE_AS_SHORTCUT, SAVE_SHORTCUT};
pub use selecting::{
    BRUSH_MAX, BRUSH_MIN, BRUSH_RADIUS, BRUSH_STEP, INVERT_SHORTCUT, SELECT_ALL_SHORTCUT,
    SELECT_TOOL_KEY, SelectTool,
};
pub use settings::SETTINGS_SHORTCUT;
pub use shape_picker::SHAPE_LIST_KEY;
pub use undo_memory::{UndoMemory, format_bytes};
pub use units::Units;
pub use viewport::{
    DEFAULT_FRAME, FOUR_VIEWS_KEY, Frame, GLIDE_SECONDS, GridDetail, NoRenderer, PICKER_LABEL,
    PickAsk, PickReply, Picked, VIEW_COUNT, VIEW_KEYS, VIEWPORT_LABEL, ViewOptions, ViewportPanel,
    ViewportRenderer, ViewportView,
};
pub use workbench::{
    AppInfo, DISCARD_SHORTCUT, MODE_KEYS, Workbench, WorkbenchAction, navigation_hint,
};
