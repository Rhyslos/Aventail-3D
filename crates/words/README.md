# words

**Layer 0 · Geometry** (the bottom layer, beside `mesh`)

The app's words through one language layer: English as the key, other languages as packages.

## Purpose

Every text a person reads goes through here, wherever it is made: the interface (`ui`), and the undo labels, messages and reasons made in `scene`, `mesh` and `fileio`. The English text stays in the code where it's used; a language package maps each English text to its translation, and missing translations fall back to English. See decisions [0132](../../docs/decisions/0132-one-language-layer.md) and [0134](../../docs/decisions/0134-shared-language-crate.md).

- `tr("Delete objects")` for plain text.
- `trf!("{} polygons selected", count)` in place of `format!`: the template is translated, then the values are put in. Takes `{}`, `{:.N}` (a number with N decimals) and `{{` `}}` (braces) only; anything else (`{:?}`, widths, signs, named values) is formatted first and passed as a value.
- `text!("Delete objects")` for text kept in a constant, shown later with `tr(CONSTANT)`.
- `Package::read` reads a package (one `English = translation` per line, with `\n`, `\=` and `\\` escapes), `use_package` puts it in use, `escape` writes a package line's English part.
- `use_mac_keys` makes every text say "Cmd" for "Ctrl" on a Mac ([decision 0133](../../docs/decisions/0133-key-audit.md)).

The English package is `lang/en.txt`: every text through the layer, in all four crates, one per line. `lang/skipping.txt` counts, file by file, the text that doesn't go through it yet (empty: none does). Both are written by a test in the `ui` crate (`text_checks`, run with `UPDATE_LANG=1`), which reads the four crates' source; it fails when a new text skips the layer.

## Must never

- Depend on anything outside the standard library, or on another app crate: every crate that makes text uses this one
- Know about the GPU, UI, files or the scene: packages arrive as text, read by whoever loads them
- Panic on a bad package: lines it can't read are skipped

## Depends on

Nothing: the standard library only.

## Status

The layer ([decision 0132](../../docs/decisions/0132-one-language-layer.md)) moved here from the `ui` crate's `text` module ([decision 0134](../../docs/decisions/0134-shared-language-crate.md)), so `scene`, `mesh` and `fileio` can use it too. `ui::text` re-exports it, and `ui` re-exports the macros, so the interface still writes `crate::text::tr`, `crate::trf!` and `crate::text!`. English is the only package so far.
