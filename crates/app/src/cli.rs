//! Command-line options.

use std::ffi::OsString;

/// What the program was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the app window (the normal case).
    Run,
    /// Print the name and version, then exit.
    Version,
    /// Print the usage text, then exit.
    Help,
}

/// The usage text for `--help`. `program` is the executable's name.
pub fn usage(program: &str) -> String {
    format!(
        "Usage: {program} [option]\n\n\
         Options:\n  \
         --version   Show the name and version, then exit\n  \
         --help      Show this text, then exit\n\n\
         With no option, the app window opens.\n"
    )
}

/// Reads the command-line arguments (without the program name).
/// Unknown arguments are an error with a plain-language explanation.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut command = Command::Run;
    for arg in args {
        match arg.to_str() {
            Some("--version" | "-V") => command = Command::Version,
            Some("--help" | "-h") => command = Command::Help,
            Some(other) => {
                return Err(format!(
                    "Unknown option \"{other}\". Use --help to see the options."
                ));
            }
            None => {
                return Err("An option is not valid text. Use --help to see the options.".into());
            }
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
        assert_eq!(parse(args(&[])), Ok(Command::Run));
    }

    #[test]
    fn version_and_help_are_recognised() {
        assert_eq!(parse(args(&["--version"])), Ok(Command::Version));
        assert_eq!(parse(args(&["-V"])), Ok(Command::Version));
        assert_eq!(parse(args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(args(&["-h"])), Ok(Command::Help));
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
