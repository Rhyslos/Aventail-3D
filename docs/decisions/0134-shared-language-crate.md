# 0134. One language crate, used by every crate that makes text

- **Status:** Accepted
- **Date:** 2026-10-23
- **Builds on:** [0132](0132-one-language-layer.md) (one language layer), [0133](0133-key-audit.md) (Cmd for Ctrl on a Mac)

## Problem

Phase 9's gate asks that no text skips the English package. After [0132](0132-one-language-layer.md) every text the `ui` crate makes went through the language layer, but much of what a person reads is made lower down: undo labels such as "Delete 3 objects" (`scene`), the reasons an edit or a bake can't be done, the shapes' names and descriptions the palette shows (`mesh`), and why a project, OBJ or export couldn't be opened or written (`fileio`). Those crates can't use the layer while it lives in `ui`: lower layers never depend on higher ones, and `mesh` and `scene` must not depend on any interface crate.

## Options

1. **Move the layer into its own tiny crate at the bottom (chosen).** A crate with no dependencies at all, which every crate that makes text can use. The text stays where it's made, written the same way as in the interface.
2. Give each lower crate its own keys (an enum per message) and let `ui` word them. Every message would need an enum, a match in `ui` and a test that they stay in step; the lower crates' messages are many (about 500) and some are put together from parts.
3. Leave the lower crates' text in English. Phase 9's gate wouldn't be met, and a translation would show a mix of languages.

## Decision

- A new crate, `words` (layer 0, beside `mesh`; standard library only), holds the layer: `tr`, `fill`, `escape`, `Package`, `use_package`, `use_mac_keys`, and the `trf!` and `text!` macros. It has a generic name like every crate.
- `ui::text` re-exports it and `ui` re-exports the macros, so the interface still writes `crate::text::tr`, `crate::trf!` and `crate::text!`.
- `mesh`, `scene` and `fileio` put the text a person reads through it, and so do `app` (dialog titles, start-up steps, backup notes, the command line's help), `tools` (navigation schemes, view names, the view cube's labels) and `uv` (stretch explanations), made in the language in use where it's made: `tr("…")`, `trf!("…", values)` in place of `format!` and `write!`, and `text!("…")` for text kept in a constant or handed on as a `&'static str` and shown later through `tr` (an edit's reason, a part of a label). Functions such as `PrimitiveKind::name` and `Command::describe` give translated text, so their callers need do nothing.
- The English package moves with the layer, to `crates/words/lang/en.txt` (with `skipping.txt`). The check in `ui` (`text_checks`) reads the source of every crate that makes text (`ui`, `scene`, `mesh`, `fileio`, `app`, `tools`, `uv` and `config`; not `render`, whose texts are names for GPU debuggers, never shown), lists every text through the layer in the one package, and counts the text that skips it, file by file; it lists the texts themselves when it fails.
- Not interface text, and left alone: ids, file keys and lines written into files (OBJ lines, glTF names, byte strings), the name written into exported files for an unnamed object and numbered file names (so a game project's files keep their names whatever language the app is in), the app's name (`branding.rs`), thread names, log and Debug text, and tests. The check treats a single word in capitals (a code such as `VEC3` or a format such as `OBJ`) and byte strings as not text, and `NOT_TEXT` lists the rest with a comment each.
- `mesh` may depend on `words` and on nothing else of the app's; the layer check says so.

## Consequences

- Phase 9's gate condition "no text skips the English package" is met: `skipping.txt` is empty across every crate that makes text, and `en.txt` lists about 2,200 texts.
- English on screen reads exactly as before.
- Names the app gives new things (a cube's "Cube", "Edits 2", "Row 1", the material "Default") are made in the language in use, and kept as the user's names after that: a project made in one language keeps its names when opened in another.
- Some labels are still put together from parts ("Extrude" + "4" + "polygon" + "s"), as in the interface ([0132](0132-one-language-layer.md)); a language with other plural rules will need those reworked, or Fluent, with its first translation.
