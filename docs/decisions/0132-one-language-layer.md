# 0132. One language layer, English its one package

- **Status:** Accepted
- **Date:** 2026-10-23
- **Builds on:** [0117](0117-language-support.md) (language support planned), [0121](0121-rework-before-the-rest-of-phase-8.md) (the rework), [0126](0126-rework-and-first-use.md)

## Problem

Phase 9 asks for "English as the one language package: every text through one translation layer; other languages added later as packages". The interface's text was written straight into the code (about 2,500 texts in the `ui` crate), so a translation would have meant hunting through every file.

## Options

1. **English text as the key, through `tr()` (chosen).** `tr("Delete objects")`, `trf!("{} selected", n)` in place of `format!`, and `text!("…")` for text kept in constants. The code still reads in plain English; a language package maps each English text to its translation; missing translations fall back to English.
2. Named keys (`tr("menu.edit.delete_objects")`) with all English in a file: code no longer shows what the user sees, and every change needs two edits.
3. A full localisation library (Fluent): plural rules and grammar, but a bigger dependency than the app needs before a second language exists. It can come with the first translation if plurals need it.

## Decision

- The `ui` crate's `text` module holds the layer: `tr`, `trf!`, `text!`, `Package` (one `English = translation` per line) and `use_package`.
- `crates/ui/lang/en.txt` is the English package: every text that goes through the layer, made by a test (`UPDATE_LANG=1`).
- `crates/ui/lang/skipping.txt` counts, file by file, the text that doesn't go through the layer yet. A test fails if any file's count goes up, so new text always goes through the layer, and the count only goes down as text is moved.
- Text made outside the `ui` crate (undo labels and messages from `scene`, `mesh`, `fileio`) comes later: those crates can't use the interface's layer, and will need their own keys or a shared crate.

## Consequences

- Phase 9's gate ("no text skips the English package") is met when `skipping.txt` is empty and the other crates' text is through a layer too.
- Translations need no code changes: a package file and a setting.
- The `ui` crate's part is done (Phase 9 milestone 10): `skipping.txt` is empty and `en.txt` lists about 1,800 texts. Some sentences are still put together in code (a count with an "s" added for more than one, "it" or "them"), so a language with other plural rules will need those reworked, or Fluent (option 3), with its first translation.
