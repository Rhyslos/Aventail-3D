//! The splash screen ([decision 0079](../../../docs/decisions/0079-splash-screen.md)):
//! what the window shows while the app starts. The app's name and
//! version, a progress bar, and what it is doing right now ("Starting the
//! graphics…"), so a slow start never looks like a hang.
//!
//! The app decides the steps and does the work between frames; this only
//! draws.

use crate::AppInfo;
use crate::icons::{Icon, paint};
use crate::theme::{self, Accent};
use egui::{Align2, FontId, Rect, Sense, Ui, pos2, vec2};

/// What the splash screen shows this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SplashView<'a> {
    /// The app's name and version.
    pub info: &'a AppInfo,
    /// The accent colour picked in Settings.
    pub accent: Accent,
    /// What the app is doing ("Starting the graphics…").
    pub doing: &'a str,
    /// How far along it is, from 0 to 1.
    pub progress: f32,
}

/// The size of the icon above the name.
const ICON: f32 = 72.0;
/// The width of the progress bar.
const BAR_WIDTH: f32 = 260.0;

/// Draws the splash screen over the whole of `ui`.
pub fn show(ui: &mut Ui, view: &SplashView<'_>) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::BACKGROUND);
    let colors = view.accent.colors();
    let middle = rect.center();
    // The icon, the name, the version, the bar and the words, centred.
    let icon = Rect::from_center_size(middle - vec2(0.0, 90.0), vec2(ICON, ICON));
    paint(&painter, icon, Icon::Cube, colors.light);
    painter.text(
        middle - vec2(0.0, 22.0),
        Align2::CENTER_CENTER,
        &view.info.name,
        FontId::proportional(32.0),
        theme::TEXT_STRONG,
    );
    painter.text(
        middle + vec2(0.0, 12.0),
        Align2::CENTER_CENTER,
        format!("Version {}", view.info.version),
        FontId::proportional(14.0),
        theme::TEXT_WEAK,
    );
    let bar = Rect::from_center_size(middle + vec2(0.0, 48.0), vec2(BAR_WIDTH, 4.0));
    painter.rect_filled(bar, 2.0, theme::PANEL);
    let done = Rect::from_min_size(
        bar.min,
        vec2(bar.width() * view.progress.clamp(0.0, 1.0), bar.height()),
    );
    painter.rect_filled(done, 2.0, colors.fill);
    painter.text(
        pos2(middle.x, bar.max.y + 22.0),
        Align2::CENTER_CENTER,
        view.doing,
        FontId::proportional(13.0),
        theme::TEXT_WEAK,
    );
    // One label for screen readers (and tests): the name, version and
    // what is happening.
    let response = ui.allocate_rect(rect, Sense::hover());
    let text = format!("{} {}: {}", view.info.name, view.info.version, view.doing);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn the_splash_says_what_is_starting() {
        let info = AppInfo {
            name: "Test App".into(),
            version: "1.2.3".into(),
            manual_url: String::new(),
        };
        let harness = Harness::builder().with_size([800.0, 600.0]).build_ui(|ui| {
            show(
                ui,
                &SplashView {
                    info: &info,
                    accent: Accent::default(),
                    doing: "Starting the graphics…",
                    progress: 0.25,
                },
            );
        });
        assert!(
            harness
                .query_by_label("Test App 1.2.3: Starting the graphics…")
                .is_some()
        );
    }
}
