//! The program's entry point: reads the command line, then opens the app
//! window (or prints the version or usage).

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
                say(
                    &trf!(
                        "{} could not open its window: {}\n",
                        app::branding::APP_NAME,
                        reason
                    ),
                    true,
                );
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
