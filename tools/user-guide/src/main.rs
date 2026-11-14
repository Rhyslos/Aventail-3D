//! Builds the HTML user guide from the Markdown manual (decision 0297).
//!
//! ```text
//! cargo run -p user-guide -- <folder>    write the guide into <folder>
//! cargo run -p user-guide -- --check     only check the manual
//!     --manual <folder>                  another manual than docs/manual
//! ```
//!
//! Every problem found (a page missing, a link leading nowhere) is listed
//! and the tool fails, so CI catches a manual that would make a broken
//! guide.

// A command-line tool: its messages are its output.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut check = false;
    let mut manual = user_guide::default_manual();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--manual" => match args.next() {
                Some(m) => manual = PathBuf::from(m),
                None => return usage("--manual needs a folder"),
            },
            "-h" | "--help" => return usage(""),
            _ if arg.starts_with('-') => return usage(&format!("unknown option {arg}")),
            _ if out.is_none() => out = Some(PathBuf::from(arg)),
            _ => return usage("only one output folder"),
        }
    }
    if !check && out.is_none() {
        return usage("give a folder to write the guide into, or --check");
    }
    let guide = match user_guide::build(&manual) {
        Ok(guide) => guide,
        Err(problems) => {
            eprintln!(
                "The manual in {} can't be made into the user guide:",
                manual.display()
            );
            for problem in &problems {
                eprintln!("  {problem}");
            }
            return ExitCode::FAILURE;
        }
    };
    if let Some(out) = out {
        if let Err(e) = user_guide::write(&guide, &out) {
            eprintln!("Couldn't write the guide into {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        println!(
            "Wrote the user guide to {} ({} KB, {} pictures).",
            out.join("index.html").display(),
            guide.html.len() / 1024,
            guide.pictures.len()
        );
    } else {
        println!("The manual makes a good user guide.");
    }
    ExitCode::SUCCESS
}

fn usage(problem: &str) -> ExitCode {
    if !problem.is_empty() {
        eprintln!("{problem}");
    }
    eprintln!("usage: user-guide [--manual <folder>] (<output folder> | --check)");
    if problem.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
