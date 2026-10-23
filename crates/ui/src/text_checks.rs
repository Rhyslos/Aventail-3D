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
            if (lower.contains("\"arrives in")
                || lower.contains(" arrives in phase")
                || lower.contains("arrives later"))
                && line.contains('"')
            {
                found.push(format!("{}:{}: {}", path.display(), n + 1, code));
            }
        }
    }
    assert!(found.is_empty(), "placeholders: {found:#?}");
}

/// Every string literal in `text` (outside its tests), with `\` line
/// continuations joined as Rust joins them.
fn literals(text: &str) -> Vec<String> {
    let code = without_tests(text);
    let mut out = Vec::new();
    let mut chars = code.chars().peekable();
    let mut in_comment = false;
    let mut previous = ' ';
    while let Some(c) = chars.next() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            previous = c;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            in_comment = true;
            continue;
        }
        // A character literal such as '"' isn't a string.
        if c == '"' && previous != '\'' {
            let mut s = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => match chars.next() {
                        Some('\n') => {
                            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                                chars.next();
                            }
                        }
                        Some('n') => s.push('\n'),
                        Some(other) => s.push(other),
                        None => break,
                    },
                    '"' => break,
                    other => s.push(other),
                }
            }
            out.push(s);
            previous = '"';
            continue;
        }
        previous = c;
    }
    out
}

/// The menus a path in the interface's text may start from: the menu bar,
/// the viewport's, the panels' and the UV Editor's menus, and Settings.
const MENUS: [&str; 22] = [
    "File",
    "Edit",
    "Create",
    "Modes",
    "Select",
    "Tools",
    "Mesh",
    "UV",
    "View",
    "Window",
    "Help",
    "Display",
    "Options",
    "Panel",
    "Cameras",
    "Texture",
    "Trim",
    "Snapshots",
    "Object",
    "Units",
    "Mode",
    "Settings",
];

#[test]
fn menu_paths_in_the_text_lead_somewhere() {
    // Decision 0126: a tooltip or note that says "UV → Pieces → Select
    // connected" must name entries that exist, so a renamed or moved entry
    // can't leave old directions behind. Each step after the menu must be
    // a label somewhere in the interface (the last may be followed by more
    // words).
    let all: Vec<(PathBuf, Vec<String>)> = sources()
        .into_iter()
        .map(|(p, t)| {
            let l = literals(&t);
            (p, l)
        })
        .collect();
    let labels: std::collections::BTreeSet<String> = all
        .iter()
        .flat_map(|(_, l)| l.iter().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty() && !s.contains('→'))
        .collect();
    let clean = |s: &str| s.trim().trim_matches('*').trim().to_string();
    let mut broken = Vec::new();
    let mut checked = 0;
    for (path, literals) in &all {
        for text in literals.iter().filter(|t| t.contains(" → ")) {
            for chain in chains(text) {
                checked += 1;
                let (menu, steps) = (&chain[0], &chain[1..]);
                for (k, step) in steps.iter().enumerate() {
                    let step = clean(step);
                    let ok = if k + 1 < steps.len() {
                        labels.contains(&step)
                    } else {
                        labels
                            .iter()
                            .filter(|l| l.chars().count() >= 3)
                            .any(|l| step.starts_with(l.as_str()))
                    };
                    if !ok {
                        broken.push(format!(
                            "{}: {menu} → … \"{step}\" in {text:?}",
                            path.display()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        checked > 20,
        "only {checked} menu paths found: the reader is off"
    );
    assert!(
        broken.is_empty(),
        "menu paths that lead nowhere:\n{}",
        broken.join("\n")
    );
}

/// The menu paths in `text`: each a menu from [`MENUS`] and the steps
/// after it ("UV", "Pieces", "Select connected and the rest of the
/// sentence"). A path ends at punctuation, so two paths in one sentence are
/// read apart.
fn chains(text: &str) -> Vec<Vec<String>> {
    const ARROW: &str = " → ";
    let ends = |c: char| matches!(c, '(' | ')' | '.' | ',' | ';' | ':' | '\n');
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(ARROW) {
        let before = rest[..at].trim_end().trim_end_matches('*');
        let menu = MENUS.iter().find(|m| {
            before.ends_with(**m)
                && before[..before.len() - m.len()]
                    .chars()
                    .last()
                    .is_none_or(|c| !c.is_alphanumeric())
        });
        let mut after = &rest[at + ARROW.len()..];
        let Some(menu) = menu else {
            rest = after;
            continue;
        };
        let mut chain = vec![(*menu).to_string()];
        loop {
            let next = after.find(ARROW);
            let piece = next.map_or(after, |n| &after[..n]);
            match piece.find(ends) {
                // The path ends inside this piece.
                Some(stop) => {
                    chain.push(piece[..stop].to_string());
                    rest = &after[stop..];
                    break;
                }
                None if next.is_some() => {
                    chain.push(piece.to_string());
                    after = &after[next.unwrap() + ARROW.len()..];
                }
                None => {
                    chain.push(piece.to_string());
                    rest = "";
                    break;
                }
            }
        }
        out.push(chain);
    }
    out
}

#[test]
fn menu_paths_are_read_apart_and_end_at_punctuation() {
    let found = chains(
        "Fix packs them apart (UV → Pack). Or switch on UV → Packing → Stack identical pieces, \
         and Box → Lasso isn't a menu.",
    );
    assert_eq!(
        found,
        vec![
            vec!["UV".to_string(), "Pack".to_string()],
            vec![
                "UV".to_string(),
                "Packing".to_string(),
                "Stack identical pieces".to_string()
            ],
        ]
    );
}
