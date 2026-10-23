//! Checks on the interface's own text, read from the source (decision
//! 0126): no stray backslash at the end of a line inside a string, and no
//! "arrives in Phase…" placeholder left on screen.

use std::path::{Path, PathBuf};

/// Every Rust file under this crate's `src`, leaving out tests and this
/// file.
fn sources() -> Vec<(PathBuf, String)> {
    sources_of(Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// The crates whose text a person reads (decision 0134): the interface's,
/// the undo labels, messages and reasons made below it, and the app's own
/// (its dialogs, start-up and command line). Not `render`: its texts are
/// names for GPU debuggers, never shown.
const CRATES_WITH_TEXT: [&str; 9] = [
    "ui", "scene", "mesh", "fileio", "app", "tools", "uv", "config", "booleans",
];

/// The `crates` folder.
fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every Rust file under a crate's `src`, leaving out tests and this file.
fn sources_of(krate: &Path) -> Vec<(PathBuf, String)> {
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
    walk(&krate.join("src"), &mut files);
    let read: Vec<(PathBuf, String)> = files
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect();
    // Files that are test modules of their own (`#[cfg(test)] mod
    // heavy_check;`), whatever their name.
    let mut test_files = Vec::new();
    for (path, text) in &read {
        let stem = path.file_stem().unwrap().to_string_lossy();
        let dir = if stem == "lib" || stem == "mod" {
            path.parent().unwrap().to_path_buf()
        } else {
            path.with_extension("")
        };
        for (at, _) in text.match_indices("#[cfg(test)]\nmod ") {
            let rest = &text[at + "#[cfg(test)]\nmod ".len()..];
            if let Some(name) = rest.split_once(';').map(|(n, _)| n)
                && !name.contains(char::is_whitespace)
            {
                test_files.push(dir.join(format!("{name}.rs")));
            }
        }
    }
    read.into_iter()
        .filter(|(p, _)| {
            let name = p.file_name().unwrap().to_string_lossy();
            !name.contains("tests") && name != "text_checks.rs" && !test_files.contains(p)
        })
        .collect()
}

/// `text` with its `#[cfg(test)]` modules blanked out (their lines kept
/// empty, so line numbers still match the file).
fn without_tests(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tests = false;
    let mut lines = text.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        if !in_tests
            && line.trim_end() == "#[cfg(test)]"
            && lines
                .peek()
                .is_some_and(|next| next.starts_with("mod ") && next.trim_end().ends_with('{'))
        {
            in_tests = true;
        }
        if in_tests {
            out.push_str(if line.ends_with('\n') { "\n" } else { "" });
            if line.trim_end() == "}" {
                in_tests = false;
            }
        } else {
            out.push_str(line);
        }
    }
    out
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

/// Every string literal in `text` (outside its tests) with the code just
/// before it, to tell interface text from ids, keys and messages for
/// programmers.
fn literals_in_context(text: &str) -> Vec<(String, String)> {
    let code = without_tests(text);
    let mut out = Vec::new();
    let bytes: Vec<char> = code.chars().collect();
    let mut i = 0;
    let mut in_comment = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if c == '/' && bytes.get(i + 1) == Some(&'/') {
            in_comment = true;
            i += 1;
            continue;
        }
        // A raw string r"…" or r#"…"#: skipped (none hold interface text).
        if c == 'r'
            && matches!(bytes.get(i + 1), Some('"') | Some('#'))
            && i > 0
            && !bytes[i - 1].is_alphanumeric()
        {
            let mut j = i + 1;
            let mut hashes = 0;
            while bytes.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if bytes.get(j) == Some(&'"') {
                j += 1;
                'raw: while j < bytes.len() {
                    if bytes[j] == '"' && (0..hashes).all(|k| bytes.get(j + 1 + k) == Some(&'#')) {
                        j += 1 + hashes;
                        break 'raw;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        if c == '"' && !(i > 0 && bytes[i - 1] == '\'') {
            // A byte string b"…" holds bytes for a file (b"JSON"), not
            // text: read past it like any other, but leave it out.
            let byte_string = i > 0
                && bytes[i - 1] == 'b'
                && !(i > 1 && (bytes[i - 2].is_alphanumeric() || bytes[i - 2] == '_'));
            let start = i.saturating_sub(40);
            let before: String = bytes[start..i].iter().collect();
            let mut s = String::new();
            i += 1;
            while i < bytes.len() {
                match bytes[i] {
                    '\\' => {
                        i += 1;
                        match bytes.get(i) {
                            Some('\n') => {
                                while bytes.get(i + 1).is_some_and(|c| c.is_whitespace()) {
                                    i += 1;
                                }
                            }
                            Some('n') => s.push('\n'),
                            Some(other) => s.push(*other),
                            None => {}
                        }
                    }
                    '"' => break,
                    other => s.push(other),
                }
                i += 1;
            }
            if !byte_string {
                out.push((s, before));
            }
        }
        i += 1;
    }
    out
}

/// Whether the code before a literal puts it through the language layer.
fn through_the_layer(before: &str) -> bool {
    let b = before.trim_end();
    ["tr(", "trf!(", "text!("].iter().any(|call| {
        // The call itself, not a longer name ending the same way
        // (`push_str(`, `from_str(`).
        b.strip_suffix(call).is_some_and(|rest| {
            !rest
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
    })
}

/// Whether a literal is interface text at all: it has letters and reads as
/// words (a space, or a capital first letter), and the code before it isn't
/// an id, a key, a file name or a message for programmers.
fn is_interface_text(literal: &str, before: &str) -> bool {
    // Fewer than two letters isn't words: a key's cap ("Q", "é 2", "F{}")
    // or an axis ("X"), the same in every language.
    let letters = literal.chars().filter(|c| c.is_alphabetic()).count();
    let words = letters >= 2
        && (literal.contains(' ') || literal.chars().next().is_some_and(char::is_uppercase));
    if !words {
        return false;
    }
    // One word in capitals (and digits), no spaces: a code or a format's
    // name, the same in every language ("OBJ", "VEC3", "TEXCOORD_0").
    if !literal.contains(' ')
        && literal
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return false;
    }
    const NOT_TEXT: [&str; 41] = [
        // A function serde calls for a missing field's value.
        "#[serde(default =",
        "Id::new(",
        "Id::new((",
        // A texture's name, for debugging.
        "load_texture(",
        // A grid's id, not shown.
        "Grid::new(",
        // Debug output, for programmers.
        "debug_struct(",
        "debug_tuple(",
        // An id made from a salt: `ui.id().with(("layer row", index))`.
        ".with((",
        // Comparisons, never shown: with an English key (a menu's title
        // from its constant), or with text made outside the interface
        // (an undo label from `scene`, a line of /proc).
        "==",
        "starts_with(",
        "ends_with(",
        "id_salt(",
        ".id(",
        "panic!(",
        "expect(",
        "unreachable!(",
        "assert!(",
        "assert_eq!(",
        "assert_ne!(",
        "debug_assert!(",
        "eprintln!(",
        "println!(",
        "#[path = ",
        "#[expect(",
        "#[allow(",
        "env!(",
        "include_str!(",
        "rename = ",
        "alias = ",
        "std::env::var(",
        "with_extension(",
        "reason = ",
        // A test left out, and why (`#[ignore = "…"]`).
        "#[ignore =",
        // A folder or file name in a path (a Unity project's "Assets").
        ".join(",
        // Lines written into an OBJ or MTL file, and read back from one.
        "writeln!(text,",
        "write!(text,",
        "strip_prefix(",
        // A name written into exported files when an object has none: the
        // same in every language, so a game project's files keep their
        // names whatever language the app is in (fileio).
        "UNNAMED_IN_FILES: &str =",
        // The app's name (app/src/branding.rs): the same in every
        // language; see RENAMING.md.
        "APP_NAME: &str =",
        "APP_SHORT_NAME: &str =",
        // A system message looked for in what the window library said.
        "contains(",
    ];
    let b = before.trim_end();
    !NOT_TEXT.iter().any(|n| b.ends_with(n))
}

/// The text a person reads, file by file, in every crate that makes it
/// ([`CRATES_WITH_TEXT`]): what goes through the language layer, and how
/// much doesn't yet (the texts themselves). Files are named from the
/// `crates` folder (`scene/src/command.rs`).
fn texts_by_file() -> (
    std::collections::BTreeSet<String>,
    std::collections::BTreeMap<String, Vec<String>>,
) {
    let mut through = std::collections::BTreeSet::new();
    let mut skipping = std::collections::BTreeMap::new();
    let all = CRATES_WITH_TEXT.iter().flat_map(|krate| {
        let dir = crates_dir().join(krate);
        sources_of(&dir)
            .into_iter()
            .map(move |(path, text)| (krate, dir.clone(), path, text))
    });
    for (krate, dir, path, text) in all {
        // The layer's own file in the interface: a re-export, no text.
        if *krate == "ui" && path.file_name().is_some_and(|n| n == "text.rs") {
            continue;
        }
        let name = format!(
            "{krate}/{}",
            path.strip_prefix(&dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        );
        let mut skipped = Vec::new();
        for (literal, before) in literals_in_context(&text) {
            if through_the_layer(&before) {
                through.insert(literal);
            } else if is_interface_text(&literal, &before) {
                skipped.push(literal);
            }
        }
        if !skipped.is_empty() {
            skipping.insert(name, skipped);
        }
    }
    (through, skipping)
}

#[test]
fn the_english_package_lists_every_text_and_no_more_text_skips_it() {
    // Decisions 0132 and 0134: every text a person reads goes through one
    // language layer (`tr`, `trf!`, `text!` from the `words` crate), in
    // the interface and in the crates below it. `words/lang/en.txt` lists
    // them all, for translators; `words/lang/skipping.txt` counts, file by
    // file, the text that doesn't go through it yet. That count may only
    // go down. After moving text into the layer, run with UPDATE_LANG=1 to
    // write both again.
    let lang = crates_dir().join("words").join("lang");
    let (through, skipping) = texts_by_file();
    let package: String = std::iter::once(
        "# The English package: every text the app shows, one per line.\n\
         # Made by a test (UPDATE_LANG=1); a translation repeats each line\n\
         # with \" = \" and the translation after it.\n"
            .to_string(),
    )
    .chain(
        through
            .iter()
            .map(|t| format!("{}\n", crate::text::escape(t))),
    )
    .collect();
    let counts: String = skipping
        .iter()
        .map(|(file, texts)| format!("{file} {}\n", texts.len()))
        .collect();
    if std::env::var_os("UPDATE_LANG").is_some() {
        std::fs::create_dir_all(&lang).unwrap();
        std::fs::write(lang.join("en.txt"), &package).unwrap();
        std::fs::write(lang.join("skipping.txt"), &counts).unwrap();
        return;
    }
    let kept = std::fs::read_to_string(lang.join("en.txt")).unwrap_or_default();
    assert!(
        kept == package,
        "words/lang/en.txt is out of date: run this test with UPDATE_LANG=1"
    );
    let allowed: std::collections::BTreeMap<String, usize> =
        std::fs::read_to_string(lang.join("skipping.txt"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let (file, n) = l.rsplit_once(' ')?;
                Some((file.to_string(), n.parse().ok()?))
            })
            .collect();
    let worse: Vec<String> = skipping
        .iter()
        .filter(|(file, texts)| texts.len() > allowed.get(*file).copied().unwrap_or(0))
        .map(|(file, texts)| {
            format!(
                "{file}: {} texts skip the language layer (allowed {}): {texts:#?}",
                texts.len(),
                allowed.get(file).copied().unwrap_or(0)
            )
        })
        .collect();
    assert!(
        worse.is_empty(),
        "put new text through tr(), trf!() or text!():\n{}",
        worse.join("\n")
    );
    let total: usize = skipping.values().map(Vec::len).sum();
    let allowed_total: usize = allowed.values().sum();
    assert!(
        total == allowed_total,
        "less text skips the layer now ({total}, was {allowed_total}): run with UPDATE_LANG=1"
    );
}
