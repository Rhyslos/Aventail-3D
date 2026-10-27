//! The app window: opens it with eframe (winit + wgpu) and hands each
//! frame to the workbench from the `ui` crate. While the app starts, the
//! window shows a splash screen and does one step of starting up between
//! frames, saying which ([decision 0079](../../../docs/decisions/0079-splash-screen.md)).

use crate::backups::{self, Backups};
use crate::branding;
use crate::emergency::{Emergency, Snapshots};
use crate::file_work::{AfterWork, FileJobs};
use crate::gpu::GpuViewport;
use crate::project;
use crate::updates::{self, PendingInstall};
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use ui::{
    AppInfo, DockState, NoRenderer, Panel, ViewOptions, ViewportRenderer, Workbench,
    WorkbenchAction,
};
use words::{tr, trf};

#[path = "window_updating.rs"]
mod updating;
use updating::{SplashUpdate, Updating};

/// Key under which the panel layout is remembered between sessions.
const LAYOUT_KEY: &str = "panel_layout";
/// Key under which the UV Edit layout is remembered (decision 0096).
const UV_LAYOUT_KEY: &str = "uv_edit_layout";
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
/// Key under which the backup (autosave) settings are remembered.
const AUTOSAVE_KEY: &str = "autosave";
/// Key under which the tool menu's look (radial or list) is remembered.
const TOOL_MENU_KEY: &str = "tool_menu";
/// Where the UV tools' settings are kept.
const UV_SETTINGS_KEY: &str = "uv_tools";
/// Where the last export's settings are kept (decision 0122).
const EXPORT_KEY: &str = "export_settings";
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
    /// Backup copies in the background, and what a crash left behind.
    backups: Backups,
    /// The save or export being done on another thread (decision 0163).
    files: FileJobs,
    /// The snapshot the panic hook saves as an emergency copy.
    snapshots: Snapshots,
    /// Updates (decision 0169): looking for a newer build, and the one
    /// downloaded, if any.
    updating: Updating,
}

/// The shortest time the splash screen shows, so a quick start doesn't
/// just flash it.
const SPLASH_AT_LEAST: Duration = Duration::from_millis(600);

/// The settings remembered from the last session.
#[derive(Default)]
struct Remembered {
    layout: Option<DockState<Panel>>,
    uv_layout: Option<DockState<Panel>>,
    view: ViewOptions,
    accent: ui::theme::Accent,
    units: ui::Units,
    recent: Vec<PathBuf>,
    undo_memory: ui::UndoMemory,
    autosave: ui::AutosaveSettings,
    tool_menu: ui::ToolMenuSettings,
    uv: ui::UvSettings,
    export: ui::ExportSettings,
}

impl Remembered {
    /// The settings in `storage`, or the standard ones.
    fn read(storage: Option<&dyn eframe::Storage>) -> Self {
        Self {
            layout: storage.and_then(|s| eframe::get_value(s, LAYOUT_KEY)),
            uv_layout: storage.and_then(|s| eframe::get_value(s, UV_LAYOUT_KEY)),
            view: storage
                .and_then(|s| eframe::get_value(s, VIEW_KEY))
                .unwrap_or_default(),
            accent: storage
                .and_then(|s| eframe::get_value(s, ACCENT_KEY))
                .unwrap_or_default(),
            units: storage
                .and_then(|s| eframe::get_value(s, UNITS_KEY))
                .unwrap_or_default(),
            recent: storage
                .and_then(|s| eframe::get_value(s, RECENT_KEY))
                .unwrap_or_default(),
            undo_memory: storage
                .and_then(|s| eframe::get_value(s, UNDO_MEMORY_KEY))
                .unwrap_or_default(),
            autosave: storage
                .and_then(|s| eframe::get_value(s, AUTOSAVE_KEY))
                .unwrap_or_default(),
            tool_menu: storage
                .and_then(|s| eframe::get_value(s, TOOL_MENU_KEY))
                .unwrap_or_default(),
            uv: storage
                .and_then(|s| eframe::get_value(s, UV_SETTINGS_KEY))
                .unwrap_or_default(),
            export: storage
                .and_then(|s| eframe::get_value(s, EXPORT_KEY))
                .unwrap_or_default(),
        }
    }
}

/// One step of starting up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    /// Making the viewport's renderer on the window's GPU.
    Graphics,
    /// Making the workbench with the remembered settings.
    Workbench,
    /// Opening the project given on the command line.
    Open(PathBuf),
    /// Starting backup copies, and looking for what a crash left.
    Backups,
    /// Looking for a new version, which the splash screen waits for
    /// (decision 0172).
    Updates,
}

impl Step {
    /// What the splash screen says while it happens.
    pub(crate) fn doing(&self) -> String {
        match self {
            Self::Graphics => tr("Starting the graphics…").to_string(),
            Self::Workbench => tr("Getting the workbench ready…").to_string(),
            Self::Open(path) => trf!(
                "Opening {}…",
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned()
                )
            ),
            Self::Backups => tr("Checking for unsaved work from last time…").to_string(),
            Self::Updates => tr("Looking for a new version…").to_string(),
        }
    }
}

/// The steps of starting up, in order, with the project at `open` if any.
pub(crate) fn steps(open: Option<PathBuf>) -> Vec<Step> {
    let mut steps = vec![Step::Graphics, Step::Workbench];
    steps.extend(open.map(Step::Open));
    steps.push(Step::Backups);
    steps.push(Step::Updates);
    steps
}

/// The app while it starts: what is done so far.
struct Starting {
    info: AppInfo,
    remembered: Option<Remembered>,
    accent: ui::theme::Accent,
    steps: Vec<Step>,
    /// The next step.
    next: usize,
    /// Whether the splash screen naming the next step has been shown, so
    /// it is on screen while that step works.
    shown: bool,
    began: Instant,
    renderer: Option<Box<dyn ViewportRenderer>>,
    workbench: Option<Workbench>,
    backups: Option<Backups>,
    /// Shared with the panic hook (see [`crate::emergency`]).
    emergency: Emergency,
    /// Looking for a new version, started by the last step.
    updating: Updating,
}

impl Starting {
    /// Does `step`.
    fn run(&mut self, step: Step, frame: &eframe::Frame) {
        match step {
            Step::Graphics => {
                self.renderer = Some(match frame.wgpu_render_state() {
                    Some(state) => Box::new(GpuViewport::new(state)),
                    None => Box::new(NoRenderer {
                        reason: tr("the window has no GPU device").to_string(),
                    }),
                });
            }
            Step::Workbench => {
                let r = self.remembered.take().unwrap_or_default();
                let mut workbench = Workbench::new(self.info.clone(), r.layout)
                    .with_uv_layout(r.uv_layout)
                    .with_view_options(r.view)
                    .with_accent(r.accent)
                    .with_units(r.units)
                    .with_undo_memory(r.undo_memory)
                    .with_recent_projects(r.recent)
                    .with_autosave(r.autosave)
                    .with_tool_menu(r.tool_menu)
                    .with_uv_settings(r.uv)
                    .with_export_settings(r.export);
                // The Keys window draws the keyboard that goes with the
                // system language, until one is picked there.
                workbench.set_system_language(&sys_locale::get_locale().unwrap_or_default());
                self.workbench = Some(workbench);
            }
            Step::Open(path) => {
                if let Some(workbench) = self.workbench.as_mut() {
                    project::open_from(workbench, path);
                }
            }
            Step::Backups => {
                if let Some(workbench) = self.workbench.as_mut() {
                    let backups = Backups::start(backups::folder().as_deref(), workbench);
                    // A crash from now on saves an emergency copy here.
                    self.emergency
                        .set_folder(backups.folder().map(std::path::Path::to_path_buf));
                    self.backups = Some(backups);
                }
            }
            Step::Updates => {
                if let Some(workbench) = self.workbench.as_mut() {
                    self.updating.update(workbench);
                }
            }
        }
    }

    /// What the splash screen shows for updates, once every step is done
    /// (decisions 0172 and 0173).
    fn update_wait(&mut self) -> SplashUpdate {
        if self.next < self.steps.len() {
            return SplashUpdate::Done;
        }
        if let Some(workbench) = self.workbench.as_mut() {
            self.updating.update(workbench);
        }
        self.updating.splash()
    }

    /// The running app, once every step is done and the splash screen
    /// has shown long enough.
    fn finished(&mut self) -> Option<App> {
        if self.next < self.steps.len()
            || self.began.elapsed() < SPLASH_AT_LEAST
            || self.update_wait() != SplashUpdate::Done
        {
            return None;
        }
        Some(App {
            title: String::new(),
            closing: false,
            workbench: self.workbench.take()?,
            renderer: self.renderer.take()?,
            backups: self.backups.take()?,
            files: FileJobs::default(),
            snapshots: Snapshots::new(self.emergency.clone()),
            updating: std::mem::take(&mut self.updating),
        })
    }
}

/// The app window's contents: the splash screen while starting, then the
/// app.
enum Shell {
    Starting(Box<Starting>),
    Running(Box<App>),
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let Self::Starting(starting) = self else {
            if let Self::Running(app) = self {
                app.frame(ui, frame);
            }
            return;
        };
        // The step named on screen last frame is done now, while it shows.
        if starting.shown && starting.next < starting.steps.len() {
            let step = starting.steps[starting.next].clone();
            starting.run(step, frame);
            starting.next += 1;
            starting.shown = false;
        }
        if let Some(app) = starting.finished() {
            *self = Self::Running(Box::new(app));
            ui.ctx().request_repaint();
            return;
        }
        let total = starting.steps.len().max(1);
        let update = starting.update_wait();
        if starting.updating.install_now() {
            // Update now: downloaded, so the app closes and the setup
            // program installs it and opens the app again.
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let (doing, progress) = match &update {
            // Waiting for the update check: a download fills the bar.
            SplashUpdate::Waiting(text, share) => (text.clone(), share.unwrap_or(1.0)),
            SplashUpdate::Asking(release) => (trf!("Build {} is available.", release.build), 1.0),
            SplashUpdate::Done => (
                starting
                    .steps
                    .get(starting.next)
                    .map_or_else(|| tr("Ready").to_string(), Step::doing),
                starting.next as f32 / total as f32,
            ),
        };
        ui::splash::show(
            ui,
            &ui::splash::SplashView {
                info: &starting.info,
                accent: starting.accent,
                doing: &doing,
                progress,
            },
        );
        if let SplashUpdate::Asking(release) = update
            && let Some(answer) = ui::splash::update_question(
                ui.ctx(),
                release.build,
                &release.page,
                &mut starting.updating.skip_box,
            )
            && let Some(workbench) = starting.workbench.as_mut()
        {
            starting.updating.answer(answer, workbench);
        }
        starting.shown = true;
        ui.ctx().request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Nothing has changed while starting: the settings stay as they were.
        if let Self::Running(app) = self {
            app.save(storage);
        }
    }

    fn on_exit(&mut self) {
        match self {
            Self::Running(app) => app.on_exit(),
            Self::Starting(starting) => {
                starting.emergency.forget();
                if let Some(backups) = starting.backups.as_mut() {
                    backups.end();
                }
            }
        }
    }
}

impl App {
    /// One frame of the running app.
    fn frame(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        crate::platform::keep_keyboard_focus(frame);
        let ctx = ui.ctx().clone();
        // The window's close button: wait for a save or export under way,
        // and ask about unsaved changes first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing {
            if self.files.busy() {
                self.files.close_when_done(&mut self.workbench, false);
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else if !self.workbench.request_close() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        let actions = self.workbench.show(ui, self.renderer.as_mut());
        // What the app did (opened, restored, ...) shows on the next frame.
        if !actions.is_empty() {
            ctx.request_repaint();
        }
        for action in actions {
            match action {
                WorkbenchAction::Quit if self.files.busy() => {
                    self.files.close_when_done(&mut self.workbench, true);
                }
                WorkbenchAction::Quit => {
                    self.closing = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                WorkbenchAction::Save { path } => {
                    if let Some(path) = project::where_to_save(&mut self.workbench, path) {
                        self.files.save(&mut self.workbench, path);
                    }
                }
                WorkbenchAction::Open { path } => project::open(&mut self.workbench, path),
                WorkbenchAction::Autosave { document, keep } => {
                    self.backups.write(*document, keep, &mut self.workbench);
                }
                WorkbenchAction::Restore {
                    folder,
                    copy,
                    project,
                } => {
                    let keep = self.workbench.autosave_settings().clamped().copies as usize;
                    self.backups
                        .restore(&folder, &copy, project, keep, &mut self.workbench);
                }
                WorkbenchAction::DiscardBackups { folder } => self.backups.discard(&folder),
                WorkbenchAction::Export(request) => {
                    if let Some(path) = project::export_path(&mut self.workbench, &request) {
                        self.files.export(&mut self.workbench, request, path);
                    }
                }
                WorkbenchAction::Import { path } => project::import(&mut self.workbench, path),
                WorkbenchAction::ChooseExportFolder { start } => {
                    project::choose_export_folder(&mut self.workbench, start);
                }
                WorkbenchAction::ExportKit(request) => {
                    if let Some(folder) = project::kit_folder(&mut self.workbench, &request) {
                        self.files.export_kit(&mut self.workbench, request, folder);
                    }
                }
                WorkbenchAction::ExportToTarget(request) => {
                    self.files.export_to_game(&mut self.workbench, request);
                }
                WorkbenchAction::SkipUpdate => self.updating.skipped(),
                WorkbenchAction::InstallUpdate { now } => {
                    if let Some(installer) = self.updating.ready.clone() {
                        updates::set_pending(PendingInstall {
                            installer,
                            reopen: now,
                        });
                        if now {
                            // Closing asks about unsaved changes as usual.
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
            }
        }
        if self.updating.update(&mut self.workbench) {
            // Hand on what the looking finds without waiting for input.
            ctx.request_repaint_after(Duration::from_secs(5));
        }
        self.backups.update(&mut self.workbench);
        match self.files.update(&mut self.workbench) {
            AfterWork::Stay => {}
            // The close button was pressed meanwhile: try again, so
            // changes made during the save are asked about.
            AfterWork::AskToClose => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            AfterWork::Close => {
                self.closing = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.files.busy() {
            // Look again shortly for the work to be done.
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        // The emergency copy's snapshot, taken between frames, when the
        // document is whole.
        let document = self.workbench.document();
        let unsaved = self.workbench.has_unsaved_changes();
        if let Some(wait) = self.snapshots.update(document, unsaved, Instant::now()) {
            ctx.request_repaint_after(wait);
        }
        let title = self.workbench.title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, LAYOUT_KEY, self.workbench.layout());
        eframe::set_value(storage, UV_LAYOUT_KEY, self.workbench.uv_layout());
        eframe::set_value(storage, VIEW_KEY, &self.workbench.view_options());
        eframe::set_value(storage, ACCENT_KEY, &self.workbench.accent());
        eframe::set_value(storage, UNITS_KEY, &self.workbench.new_project_units());
        eframe::set_value(storage, UNDO_MEMORY_KEY, &self.workbench.undo_memory());
        eframe::set_value(storage, RECENT_KEY, &self.workbench.recent_projects());
        eframe::set_value(storage, AUTOSAVE_KEY, &self.workbench.autosave_settings());
        eframe::set_value(storage, TOOL_MENU_KEY, &self.workbench.tool_menu_settings());
        eframe::set_value(storage, UV_SETTINGS_KEY, &self.workbench.uv_settings());
        eframe::set_value(storage, EXPORT_KEY, &self.workbench.export_settings());
    }

    /// A normal close: this run's backup copies are no longer needed.
    fn on_exit(&mut self) {
        // A save or export under way is finished first.
        self.files.finish(&mut self.workbench);
        self.snapshots.emergency().forget();
        self.backups.end();
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
            .with_min_inner_size(MIN_SIZE)
            .with_icon(window_icon()),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // From here on, a crash saves the work as an emergency copy (once
    // backups have started and there is something unsaved).
    let emergency = Emergency::new();
    emergency.install();

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
                let remembered = Remembered::read(cc.storage);
                let accent = remembered.accent;
                Ok(Box::new(Shell::Starting(Box::new(Starting {
                    info,
                    remembered: Some(remembered),
                    accent,
                    steps: steps(open),
                    next: 0,
                    shown: false,
                    began: Instant::now(),
                    renderer: None,
                    workbench: None,
                    backups: None,
                    emergency,
                    updating: Updating::default(),
                }))))
            }),
        )
    }));
    // An update agreed to (Restart now or Later) installs now that the
    // window has closed (decision 0169). If it can't start, there is no
    // window left to say so; the next start finds the downloaded build
    // whole and offers it again.
    if let Some(pending) = updates::take_pending() {
        let _ = pending.run();
    }
    match outcome {
        Ok(result) => result.map_err(|err| err.to_string()),
        Err(payload) => Err(explain_panic(payload.as_ref())),
    }
}

/// The app's logo for the window's title bar and taskbar button
/// (decision 0171), from `assets/icon-256.png`; an empty icon (the
/// system's own) if it couldn't be read, which a test makes sure never
/// happens.
fn window_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon-256.png"))
        .unwrap_or_default()
}

/// Turns a caught panic into a plain-language reason, adding an install
/// hint when the cause is a missing Linux system library.
fn explain_panic(payload: &(dyn std::any::Any + Send)) -> String {
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or(tr("the windowing system stopped unexpectedly"));
    if message.contains(".so") && message.contains("could not be loaded") {
        trf!(
            "{} On Linux, install the missing library with your package manager \
             (for example libxkbcommon-x11-0 and mesa-vulkan-drivers on Ubuntu).",
            message
        )
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_up_says_what_it_does_in_order() {
        let plain: Vec<String> = steps(None).iter().map(Step::doing).collect();
        assert_eq!(
            plain,
            [
                "Starting the graphics…",
                "Getting the workbench ready…",
                "Checking for unsaved work from last time…",
                "Looking for a new version…",
            ]
        );
        let opening = steps(Some(PathBuf::from("/work/crate.avtl")));
        assert_eq!(opening.len(), 5);
        assert_eq!(opening[2].doing(), "Opening crate.avtl…");
        // The project opens after the workbench is made, and before the
        // backups look for what a crash left (which may offer them back).
        assert_eq!(opening[1], Step::Workbench);
        assert_eq!(opening[3], Step::Backups);
        // The update check is last: the splash screen waits for it.
        assert_eq!(opening[4], Step::Updates);
    }

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
    fn the_window_icon_is_the_logo() {
        let icon = window_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // The middle of the logo is its red cube's edge or the dark disc,
        // never see-through; the corners are see-through.
        let alpha = |x: usize, y: usize| icon.rgba[(y * 256 + x) * 4 + 3];
        assert_eq!(alpha(128, 128), 255);
        assert_eq!(alpha(0, 0), 0);
    }

    #[test]
    fn the_layout_key_does_not_name_the_app() {
        assert!(!LAYOUT_KEY.contains(branding::APP_ID));
        assert!(!VIEW_KEY.contains(branding::APP_ID));
        assert!(!ACCENT_KEY.contains(branding::APP_ID));
        assert_ne!(LAYOUT_KEY, VIEW_KEY);
        assert!(!UV_LAYOUT_KEY.contains(branding::APP_ID));
        assert_ne!(LAYOUT_KEY, UV_LAYOUT_KEY);
        assert_ne!(VIEW_KEY, ACCENT_KEY);
        assert!(!RECENT_KEY.contains(branding::APP_ID));
        assert!(!AUTOSAVE_KEY.contains(branding::APP_ID));
        assert_ne!(AUTOSAVE_KEY, UNDO_MEMORY_KEY);
        assert!(!TOOL_MENU_KEY.contains(branding::APP_ID));
        assert_ne!(TOOL_MENU_KEY, AUTOSAVE_KEY);
        assert!(!UV_SETTINGS_KEY.contains(branding::APP_ID));
        assert_ne!(UV_SETTINGS_KEY, UV_LAYOUT_KEY);
        assert!(!EXPORT_KEY.contains(branding::APP_ID));
        assert_ne!(EXPORT_KEY, UV_SETTINGS_KEY);
    }
}
