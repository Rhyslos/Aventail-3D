//! The program's entry point.
//!
//! For now it only reports its name and version; the window arrives in a
//! later Phase 0 milestone.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut out = std::io::stdout().lock();
    let message = format!(
        "{}\nThe app window is not built yet. See the roadmap in README.md.\n",
        app::version_banner()
    );
    match out.write_all(message.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
