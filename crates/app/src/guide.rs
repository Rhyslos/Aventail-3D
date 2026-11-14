//! Finding the user guide installed with the app (decision 0297).
//!
//! Releases carry the HTML user guide, built from the manual by
//! `tools/user-guide`, in a folder beside the program:
//! `<install folder>/user-guide/index.html`. Help → User manual opens it
//! in the web browser, so the manual is there offline and matches the
//! version installed; without it (a development build) the manual is read
//! online.

use std::path::{Path, PathBuf};

/// The guide's folder beside the program. The release packaging
/// (`installer/windows/build-setup.ps1`, the CI Build job) writes it under
/// this name.
pub const FOLDER: &str = "user-guide";

/// The installed guide's page, if this copy of the app has one.
pub fn installed() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|exe| beside(&exe))
}

/// The guide's page beside the program at `exe`, if it is there.
pub fn beside(exe: &Path) -> Option<PathBuf> {
    let page = exe.parent()?.join(FOLDER).join("index.html");
    page.is_file().then_some(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("guide-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_guide_beside_the_program_is_found() {
        let dir = folder("found");
        std::fs::create_dir_all(dir.join(FOLDER)).unwrap();
        std::fs::write(dir.join(FOLDER).join("index.html"), "<html>").unwrap();
        let found = beside(&dir.join("app.exe"));
        assert_eq!(found, Some(dir.join(FOLDER).join("index.html")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn without_the_guide_there_is_none() {
        let dir = folder("missing");
        assert_eq!(beside(&dir.join("app.exe")), None);
        // A folder of that name without its page is no guide either.
        std::fs::create_dir_all(dir.join(FOLDER)).unwrap();
        assert_eq!(beside(&dir.join("app.exe")), None);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(beside(Path::new("")), None);
    }
}
