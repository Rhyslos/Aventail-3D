//! The program's entry point: reads the command line, then opens the app
//! window (or prints the version or usage).
//!
//! On Windows it is a window program, so no console window opens behind
//! the app; `--version`, `--help` and errors print to the terminal it was
//! started from, if any (decision 0170).

#![cfg_attr(windows, windows_subsystem = "windows")]

use app::cli::{self, Command};
use std::io::Write;
use std::process::ExitCode;
use words::trf;

/// Writes text to standard output or standard error; `false` if it failed.
fn say(text: &str, to_error: bool) -> bool {
    let written = if to_error {
        std::io::stderr().lock().write_all(text.as_bytes())
    } else {
        std::io::stdout().lock().write_all(text.as_bytes())
    };
    written.is_ok()
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let printing = !matches!(
        cli::parse(std::env::args_os().skip(1)),
        Ok(Command::Run { .. })
    );
    if printing {
        app::platform::use_parent_terminal();
    }
    let program = args
        .next()
        .and_then(|p| {
            std::path::Path::new(&p)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| app::branding::APP_ID.to_string());

    match cli::parse(args) {
        Ok(Command::Version) => exit_with(say(&format!("{}\n", app::version_banner()), false)),
        Ok(Command::Help) => exit_with(say(&cli::usage(&program), false)),
        Ok(Command::Run { open }) => match app::window::run(app::app_info(), open) {
            Ok(()) => ExitCode::SUCCESS,
            Err(reason) => {
                let text = trf!(
                    "{} could not open its window: {}",
                    app::branding::APP_NAME,
                    reason
                );
                // Started from a terminal, it reads there; on Windows,
                // started from the Start menu, there is no terminal, so a
                // message box says it.
                app::platform::use_parent_terminal();
                say(&format!("{text}\n"), true);
                if cfg!(windows) {
                    rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title(app::branding::APP_NAME)
                        .set_description(&text)
                        .show();
                }
                ExitCode::FAILURE
            }
        },
        Err(reason) => {
            say(&format!("{reason}\n"), true);
            ExitCode::from(2)
        }
    }
}

/// Success if the text was written, failure otherwise.
fn exit_with(written: bool) -> ExitCode {
    if written {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
