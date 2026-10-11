//! The application: wires every crate together and owns the frame loop.
//!
//! Layer 4 (app). Holds no feature logic. The program's entry point is
//! `main.rs`; this library part holds what the entry point and its tests
//! share. See this crate's README for its purpose and limits.

pub mod branding;
pub mod cli;
pub mod gpu;
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
        family: branding::ORG_NAME.to_string(),
    }
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
        assert_eq!(info.family, branding::ORG_NAME);
    }
}
