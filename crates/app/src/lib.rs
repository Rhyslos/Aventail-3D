//! The application: wires every crate together and owns the frame loop.
//!
//! Layer 4 (app). Holds no feature logic. The program's entry point is
//! `main.rs`; this library part holds what the entry point and its tests
//! share. See this crate's README for its purpose and limits.

pub mod branding;

/// The text shown by `--version` and at startup: the app's name and version.
pub fn version_banner() -> String {
    format!("{} {}", branding::APP_NAME, env!("CARGO_PKG_VERSION"))
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
}
