//! The application: wires every crate together and owns the frame loop.
//!
//! Layer 4 (app). Holds no feature logic. The program's entry point is
//! `main.rs`; this library part holds what the entry point and its tests
//! share. See this crate's README for its purpose and limits.

pub mod backups;
pub mod branding;
pub mod cli;
pub mod emergency;
pub mod file_work;
pub mod gpu;
pub mod platform;
pub mod project;
pub mod window;

/// The text shown by `--version`: the app's name and version.
pub fn version_banner() -> String {
    format!("{} {}", branding::APP_NAME, env!("CARGO_PKG_VERSION"))
}

/// The app's name and version, for the interface.
pub fn app_info() -> ui::AppInfo {
    ui::AppInfo {
        name: branding::APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        manual_url: branding::MANUAL_URL.to_string(),
    }
}

/// For tests that lock backup folders, or start a child process: one at a
/// time. Starting a child process briefly copies every open file of this
/// test program, a backup folder's lock among them, so a test checking
/// whether a folder is still locked could find it held by that copy and
/// miss the folder (it failed now and then on busy CI machines).
#[cfg(test)]
pub(crate) fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_contains_name_and_version() {
        let banner = version_banner();
        assert!(banner.starts_with(branding::APP_NAME));
        assert!(banner.ends_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn app_info_comes_from_branding() {
        let info = app_info();
        assert_eq!(info.name, branding::APP_NAME);
    }
}
