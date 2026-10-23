//! Command-line options.

use std::ffi::OsString;
use std::path::PathBuf;
use words::{tr, trf};

/// What the program was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the app window (the normal case), with this project open if
    /// one was given (as when a project file is double-clicked).
    Run {
        /// The project to open.
        open: Option<PathBuf>,
    },
    /// Print the name and version, then exit.
    Version,
    /// Print the usage text, then exit.
    Help,
}

/// The usage text for `--help`. `program` is the executable's name.
pub fn usage(program: &str) -> String {
    trf!(
        "Usage: {} [option | project]\n\n\
         Options:\n  \
         --version   Show the name and version, then exit\n  \
         --help      Show this text, then exit\n\n\
         With no option, the app window opens. Give a project file to open it\n\
         (put -- before a file name that starts with -).\n",
        program
    )
}

/// Reads the command-line arguments (without the program name).
/// Unknown arguments are an error with a plain-language explanation.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut command = Command::Run { open: None };
    // After `--`, everything is a file name, even one starting with `-`.
    let mut names_only = false;
    for arg in args {
        let text = if names_only { None } else { arg.to_str() };
        match text {
            Some("--") => names_only = true,
            Some("--version" | "-V") => command = Command::Version,
            Some("--help" | "-h") => command = Command::Help,
            Some(other) if other.starts_with('-') => {
                return Err(trf!(
                    "Unknown option \"{}\". Use --help to see the options. To open a file \
                     whose name starts with -, put -- before it.",
                    other
                ));
            }
            // Anything else is a project to open (any text, as file names
            // need not be valid text on every system).
            _ => match &mut command {
                Command::Run { open: open @ None } => *open = Some(PathBuf::from(arg)),
                Command::Run { open: Some(_) } => {
                    return Err(tr("Only one project can be opened at a time.").into());
                }
                _ => {}
            },
        }
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_arguments_opens_the_window() {
        assert_eq!(parse(args(&[])), Ok(Command::Run { open: None }));
    }

    #[test]
    fn version_and_help_are_recognised() {
        assert_eq!(parse(args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(args(&["-V"])), Ok(Command::Version));
        assert_eq!(parse(args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(args(&["-h"])), Ok(Command::Help));
    }

    #[test]
    fn a_project_file_is_opened() {
        assert_eq!(
            parse(args(&["/work/crate.avtl"])),
            Ok(Command::Run {
                open: Some(PathBuf::from("/work/crate.avtl"))
            })
        );
        let two = parse(args(&["a.avtl", "b.avtl"])).unwrap_err();
        assert!(two.contains("one project"));
    }

    #[test]
    fn a_file_name_starting_with_a_dash_opens_after_two_dashes() {
        assert_eq!(
            parse(args(&["--", "-crate.avtl"])),
            Ok(Command::Run {
                open: Some(PathBuf::from("-crate.avtl"))
            })
        );
        assert!(parse(args(&["-crate.avtl"])).unwrap_err().contains("--"));
        assert!(parse(args(&["--", "--help"])).is_ok_and(|c| c != Command::Help));
    }

    #[test]
    fn unknown_options_are_explained() {
        let err = parse(args(&["--fly"])).unwrap_err();
        assert!(err.contains("--fly") && err.contains("--help"));
    }

    #[test]
    fn usage_names_the_program_and_its_options() {
        let text = usage("aventail");
        assert!(text.contains("Usage: aventail"));
        assert!(text.contains("--version") && text.contains("--help"));
    }
}
