//! The interface: the dockable panel layout, menus, palettes, the
//! Attribute and Object Managers, and the theme.
//!
//! Layer 3 (front end). Changes the scene only by executing commands. See
//! this crate's README for its purpose and limits.
//!
//! The app's name is never written here: it arrives in [`AppInfo`] from
//! the app's branding.

pub mod icons;
pub mod layout;
pub mod palette;
mod panel;
mod panels;
pub mod theme;
mod viewport;
#[cfg(test)]
pub(crate) use viewport::chrome as viewport_chrome;
mod workbench;

pub use egui_dock::DockState;
pub use palette::Mode;
pub use panel::Panel;
pub use viewport::{
    Frame, GLIDE_SECONDS, NoRenderer, PICKER_LABEL, PickAsk, PickReply, Picked, TEST_SCENE,
    VIEWPORT_LABEL, ViewOptions, ViewportPanel, ViewportRenderer, ViewportView,
};
pub use workbench::{AppInfo, QUIT_SHORTCUT, Workbench, WorkbenchAction, navigation_hint};
