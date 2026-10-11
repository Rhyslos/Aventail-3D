//! The app window: opens it with eframe (winit + wgpu) and hands each
//! frame to the workbench from the `ui` crate.

use crate::branding;
use crate::gpu::GpuViewport;
use crate::project;
use eframe::egui;
use std::path::PathBuf;
use ui::{
    AppInfo, DockState, NoRenderer, Panel, ViewOptions, ViewportRenderer, Workbench,
    WorkbenchAction,
};

/// Key under which the panel layout is remembered between sessions.
const LAYOUT_KEY: &str = "panel_layout";
/// Key under which the viewport's display choices (such as grid detail)
/// are remembered between sessions.
const VIEW_KEY: &str = "view_options";
/// Key under which the accent colour is remembered between sessions.
const ACCENT_KEY: &str = "accent";
/// Key under which the choice of metres or centimetres is remembered.
const UNITS_KEY: &str = "units";
/// Key under which the undo memory limit is remembered.
const UNDO_MEMORY_KEY: &str = "undo_memory";
/// Key under which the recently used projects are remembered.
const RECENT_KEY: &str = "recent_projects";
/// Window size on first launch, in logical pixels.
const FIRST_SIZE: [f32; 2] = [1440.0, 900.0];
/// The smallest the window may get while staying usable.
const MIN_SIZE: [f32; 2] = [960.0, 600.0];

/// The running app.
struct App {
    workbench: Workbench,
    /// Draws the 3D viewport on the window's GPU.
    renderer: Box<dyn ViewportRenderer>,
    /// The window title last set, to change it only when it changes.
    title: String,
    /// Closing has been agreed (unsaved changes dealt with).
    closing: bool,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        crate::platform::keep_keyboard_focus(frame);
        let ctx = ui.ctx().clone();
        // The window's close button: ask about unsaved changes first.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.closing
            && !self.workbench.request_close()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        for action in self.workbench.show(ui, self.renderer.as_mut()) {
            match action {
                WorkbenchAction::Quit => {
                    self.closing = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                WorkbenchAction::Save { path } => project::save(&mut self.workbench, path),
                WorkbenchAction::Open { path } => project::open(&mut self.workbench, path),
            }
        }
        let title = self.workbench.title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, LAYOUT_KEY, self.workbench.layout());
        eframe::set_value(storage, VIEW_KEY, &self.workbench.view_options());
        eframe::set_value(storage, ACCENT_KEY, &self.workbench.accent());
        eframe::set_value(storage, UNITS_KEY, &self.workbench.units());
        eframe::set_value(storage, UNDO_MEMORY_KEY, &self.workbench.undo_memory());
        eframe::set_value(storage, RECENT_KEY, &self.workbench.recent_projects());
    }
}

/// Opens the app window and runs until it is closed, with the project at
/// `open` open, if given.
///
/// Returns a plain-language reason if the window could not be opened (for
/// example, no graphics driver that supports Vulkan, DirectX 12 or Metal).
pub fn run(info: AppInfo, open: Option<PathBuf>) -> Result<(), String> {
    let title = format!("{} {}", info.name, info.version);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id(branding::APP_ID)
            .with_inner_size(FIRST_SIZE)
            .with_min_inner_size(MIN_SIZE),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // Some windowing libraries stop with a panic instead of an error when a
    // system library is missing (for example libxkbcommon-x11 on Linux).
    // Catch that and turn it into an explanation, so the app never just
    // vanishes.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // eframe keeps its storage (window size, panel layout) in a folder
        // named after this, so it uses the stable APP_ID, not the display
        // name: a rename keeps everyone's layout.
        eframe::run_native(
            branding::APP_ID,
            options,
            Box::new(move |cc| {
                ui::theme::apply(&cc.egui_ctx);
                let saved: Option<DockState<Panel>> =
                    cc.storage.and_then(|s| eframe::get_value(s, LAYOUT_KEY));
                let view: ViewOptions = cc
                    .storage
                    .and_then(|s| eframe::get_value(s, VIEW_KEY))
                    .unwrap_or_default();
                let accent: ui::theme::Accent = cc
                    .storage
                    .and_then(|s| eframe::get_value(s, ACCENT_KEY))
                    .unwrap_or_default();
                let units: ui::Units = cc
                    .storage
                    .and_then(|s| eframe::get_value(s, UNITS_KEY))
                    .unwrap_or_default();
                let recent: Vec<PathBuf> = cc
                    .storage
                    .and_then(|s| eframe::get_value(s, RECENT_KEY))
                    .unwrap_or_default();
                let undo_memory: ui::UndoMemory = cc
                    .storage
                    .and_then(|s| eframe::get_value(s, UNDO_MEMORY_KEY))
                    .unwrap_or_default();
                let renderer: Box<dyn ViewportRenderer> = match cc.wgpu_render_state.as_ref() {
                    Some(state) => Box::new(GpuViewport::new(state)),
                    None => Box::new(NoRenderer {
                        reason: "the window has no GPU device".to_string(),
                    }),
                };
                let mut workbench = Workbench::new(info, saved)
                    .with_view_options(view)
                    .with_accent(accent)
                    .with_units(units)
                    .with_undo_memory(undo_memory)
                    .with_recent_projects(recent);
                if let Some(path) = open {
                    project::open_from(&mut workbench, path);
                }
                Ok(Box::new(App {
                    title: String::new(),
                    closing: false,
                    workbench,
                    renderer,
                }))
            }),
        )
    }));
    match outcome {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(payload) => Err(explain_panic(payload.as_ref())),
    }
}

/// Turns a caught panic into a plain-language reason, adding an install
/// hint when the cause is a missing Linux system library.
fn explain_panic(payload: &(dyn std::any::Any + Send)) -> String {
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("the windowing system stopped unexpectedly");
    if message.contains(".so") && message.contains("could not be loaded") {
        format!(
            "{message} On Linux, install the missing library with your package manager \
             (for example libxkbcommon-x11-0 and mesa-vulkan-drivers on Ubuntu)."
        )
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_linux_library_gets_an_install_hint() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(String::from(
            "Library libxkbcommon-x11.so could not be loaded.",
        ));
        let text = explain_panic(payload.as_ref());
        assert!(text.starts_with("Library libxkbcommon-x11.so could not be loaded."));
        assert!(text.contains("package manager"));
    }

    #[test]
    fn other_panics_are_passed_through() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("surface lost");
        assert_eq!(explain_panic(payload.as_ref()), "surface lost");
    }

    #[test]
    fn unknown_panic_payloads_get_a_generic_reason() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42_u32);
        assert!(explain_panic(payload.as_ref()).contains("stopped unexpectedly"));
    }

    #[test]
    fn the_layout_key_does_not_name_the_app() {
        assert!(!LAYOUT_KEY.contains(branding::APP_ID));
        assert!(!VIEW_KEY.contains(branding::APP_ID));
        assert!(!ACCENT_KEY.contains(branding::APP_ID));
        assert_ne!(LAYOUT_KEY, VIEW_KEY);
        assert_ne!(VIEW_KEY, ACCENT_KEY);
        assert!(!RECENT_KEY.contains(branding::APP_ID));
    }
}
