//! The app's name and identity, in one place.
//!
//! Everything that shows or depends on the app's name (window titles, the
//! About box, settings folders, installer metadata) reads it from here, so
//! renaming the app means changing this file and following RENAMING.md.

/// The full name, shown in the title bar, About box and documentation.
pub const APP_NAME: &str = "Aventail 3D";

/// The everyday short name, for tight spaces such as menus and dialogs.
pub const APP_SHORT_NAME: &str = "Aventail";

/// The stable identifier used for the settings folder and log files.
///
/// Changing it makes the app start with fresh settings, so it should
/// survive a rename unless there is a strong reason (see RENAMING.md).
pub const APP_ID: &str = "aventail";

/// The project file extension, without the dot (decision 0010).
///
/// Short for Aventail. It stays the same even if the app is renamed, so old
/// projects keep opening and double-clicking keeps working (RENAMING.md).
pub const PROJECT_EXTENSION: &str = "avtl";

/// Where the user manual is read online (Help → User manual): its table
/// of contents in the repository.
pub const MANUAL_URL: &str =
    "https://github.com/Rhyslos/Aventail-3D/blob/main/docs/manual/SUMMARY.md";

/// Where new versions are published: the project's Releases page.
pub const RELEASES_URL: &str = "https://github.com/Rhyslos/Aventail-3D/releases";

/// Where an installed copy asks for the newest release (decision 0169):
/// GitHub's description of the Releases page's latest release.
pub const RELEASES_API: &str = "https://api.github.com/repos/Rhyslos/Aventail-3D/releases/latest";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_not_empty() {
        for value in [APP_NAME, APP_SHORT_NAME, APP_ID, PROJECT_EXTENSION] {
            assert!(!value.trim().is_empty());
        }
    }

    #[test]
    fn id_and_extension_are_safe_for_file_names() {
        for value in [APP_ID, PROJECT_EXTENSION] {
            assert!(
                value
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{value} must be lowercase letters, digits or dashes"
            );
        }
    }

    #[test]
    fn short_name_is_part_of_the_full_name() {
        assert!(APP_NAME.contains(APP_SHORT_NAME));
    }

    #[test]
    fn the_project_extension_is_settled() {
        // Changing it would stop existing projects from opening by
        // double-click. Change it only through a new decision record.
        assert_eq!(PROJECT_EXTENSION, "avtl");
    }
}
