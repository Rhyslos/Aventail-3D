# user-guide

A development tool, not part of the app: it turns the Markdown manual in `docs/manual` into the HTML user guide that ships with the app ([decision 0297](../../docs/decisions/0297-html-user-guide.md)). Help → User manual opens the copy installed beside the program.

## Purpose

- **Builds the guide:** `cargo run -p user-guide -- <folder>` writes `<folder>/index.html` (one page: contents, search with `/` and Ctrl K, Auto, Light and Dark, reading progress, pictures that open large) and `<folder>/pictures/` (the manual's pictures, copied as they are). The release packaging runs it into `user-guide/` beside the program (`installer/windows/build-setup.ps1`, and the CI Build job's zips).
- **Checks the manual:** `cargo run -p user-guide -- --check` builds it without writing, and fails listing every problem: a page in `SUMMARY.md` that doesn't exist or is listed twice, a manual page missing from `SUMMARY.md`, a page without a `#` title, a link to a page, heading (GitHub's anchors) or `<a id>` that isn't there, a picture or repository file that doesn't exist. CI runs this, and the test `tests/manual.rs` does the same.

`--manual <folder>` reads another manual (a copy of `docs/manual`; links outside it are checked against the folder two levels up).

## How the manual becomes the guide

| Markdown | Guide |
| --- | --- |
| `SUMMARY.md`'s `#` title and the words before its first `##` | The guide's title and the welcome (its first paragraph the lead) |
| A `##` group in `SUMMARY.md` | A numbered chapter, listing its pages with their summaries |
| A group with only links outside the manual (Reference) | Links in the footer |
| A page | A section of its chapter, in the contents |
| The `>` quote under a page's title | The page's lead |
| A page's `##` headings | Parts of the page, each found by search on its own |
| `[text](page.md#heading)`, `[text](#heading)`, `<a id="term">` | Links inside the one page (anchors `page-heading`) |
| A link to another file in the repository | That file on GitHub |
| A paragraph that is only `![words](../screenshots/x.png)` | A figure, the words as its caption, the picture in `pictures/` |
| A paragraph starting **Tip:**, **Note:** or **Careful:**, or `> [!NOTE]` and the like | A callout |
| A numbered list | Steps with numbered circles |
| **Ctrl + Z**, `Esc`; **2** after "press" or in a Shortcut column | Keyboard keys (**X**, **Y**, **Z** as axes stay bold) |
| Below a `---` line in `SUMMARY.md` | Left out (notes for contributors) |

The look and behaviour are `src/guide.css` and `src/guide.js`, from the project lead's video editor's user guide (theirs to reuse), with the app's colours: dark is `crates/ui/src/theme.rs`'s greys and red accent (a test checks they match), light the same family on paper. The name and online manual come from `crates/app/src/branding.rs`, the logo from `assets/logo.svg`.

## Files

- `src/main.rs`: the command line.
- `src/lib.rs`: `build` (contents, anchors for every page, then each page drawn) and `write`.
- `src/summary.rs`: reading `SUMMARY.md`.
- `src/page.rs`: one page from Markdown to HTML (pulldown-cmark's events, changed on the way).
- `src/links.rs`: where links and pictures lead, and what leads nowhere.
- `src/keys.rs`: telling keys from other bold words.
- `src/slug.rs`: GitHub's heading anchors.
- `src/template.rs`: the page around the chapters; `src/guide.css` and `src/guide.js` its look and behaviour.
- `src/tests.rs`: builds of small made-up manuals (normal, edge, every kind of problem, a fuzz test); `tests/manual.rs`: the real manual.

## Must never

- Ship inside the app's program (only the guide it writes does)
- Become a second source for the manual: change the Markdown, never the built guide
- Name the app itself: the name comes from `branding.rs`
