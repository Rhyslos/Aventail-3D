//! Checks on the interface's own text, read from the source (decision
//! 0126): no stray backslash at the end of a line inside a string, and no
//! "arrives in Phase…" placeholder left on screen.

use std::path::{Path, PathBuf};

/// Every Rust file under `src`, leaving out tests and this file.
fn sources() -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files
        .into_iter()
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            !name.contains("tests") && name != "text_checks.rs"
        })
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

/// The part of `text` before its `#[cfg(test)]` module, if any.
fn without_tests(text: &str) -> &str {
    text.find("#[cfg(test)]\nmod tests")
        .map_or(text, |at| &text[..at])
}

#[test]
fn no_string_ends_a_line_with_a_stray_backslash() {
    // `"… \\` followed by a line break shows a backslash and the next
    // line's spaces: the line was meant to continue with `\`.
    let mut found = Vec::new();
    for (path, text) in sources() {
        for (n, line) in without_tests(&text).lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if line.ends_with("\\\\") && !line.trim_end().ends_with("\"\\\\") {
                found.push(format!("{}:{}", path.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "stray backslashes: {found:#?}");
}

#[test]
fn no_arrives_later_placeholders_are_shown() {
    let mut found = Vec::new();
    for (path, text) in sources() {
        for (n, line) in without_tests(&text).lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let lower = line.to_lowercase();
            if (lower.contains("\"arrives in") || lower.contains(" arrives in phase"))
                && line.contains('"')
            {
                found.push(format!("{}:{}: {}", path.display(), n + 1, code));
            }
        }
    }
    assert!(found.is_empty(), "placeholders: {found:#?}");
}
