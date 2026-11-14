# 0297. The manual ships with the app as an HTML user guide

- **Status:** Accepted
- **Date:** 2026-11-14

## Problem

Help → User manual opened the manual's table of contents on GitHub: Markdown pages one at a time, no search across them, only online, and always the newest manual rather than the one for the installed version. The project lead's video editor, Kiria, ships a user guide that is one page in the browser, with numbered chapters, a contents sidebar that shows where you are, search with `/` and Ctrl K, a light and dark theme, and pictures that open large. The project lead asked for the same here, without changing how the manual is written.

## Options

1. **Write the guide by hand as HTML** (as Kiria's is). Looks exactly right, but means two manuals to keep in step, and the Markdown one, which every pull request updates, would win.
2. **An existing site generator** (mdBook and the like). Has its own look, which isn't Kiria's, needs its own install on CI, and would make a multi-page site rather than one page with one search.
3. **A small generator in the workspace** that reads the Markdown manual and writes one page in Kiria's layout. The Markdown stays the only source; one Markdown library (pulldown-cmark, MIT) is added.

## Decision

Option 3: `tools/user-guide`, a Rust program in the workspace, turns `docs/manual` into `user-guide/index.html` with a `pictures` folder beside it.

- **Shape:** `SUMMARY.md`'s title is the guide's title and the words before its first group are the welcome (with **The short version**); each `##` group is a numbered chapter, listing its pages; each page is a section, and each page's `##` headings are parts of it. A group with only links outside the manual (Reference) goes to the footer.
- **Links:** links between pages, and to headings (with GitHub's anchors, so a link that works on GitHub works in the guide) and to `<a id>` terms in Concepts, become anchors in the page. Links to other files in the repository go to GitHub. Pictures are copied, as they are (PNG), into `pictures/`.
- **Words:** a page's summary line (the quote under its title) is the lead; paragraphs starting with **Tip:**, **Note:** or **Careful:** and GitHub's `> [!NOTE]` quotes are callouts; numbered lists are steps; keys written in bold or as code are drawn as keyboard keys when they are keys for sure (combinations with Ctrl, Shift or Alt; Esc, Space, Enter…) or the words around say so ("press **2**", a Shortcut column), so **X** for an axis stays bold.
- **Look:** Kiria's layout, CSS and script, with the app's own colours (`crates/ui/src/theme.rs`: dark is the app's greys and red accent; light is the same family on paper), the app's logo, and its name from `branding.rs`. The guide's own `/` and Ctrl K search keys work only in the browser; the app gets no new keys.
- **Shipped:** the release packaging builds it into `<install folder>/user-guide/` (the setup program replaces the whole folder on an update). Help → User manual opens it in the web browser when it is there, else the manual online as before; Help → Words used in the app opens it at Concepts.
- **Kept current:** the guide is never committed. A test, and a **User guide** step in CI's documentation job, build it from the current manual and fail on any page in the contents that is missing, any manual page not in the contents, and any link, heading or picture that leads nowhere.

## Consequences

- Writers keep editing Markdown exactly as before; the page template and DESIGN.md §11 still hold. A broken link now fails CI (links to five headings that no longer existed were found and fixed when this was built).
- The installer grows by the pictures (about 11 MB).
- The guide shows the manual as it was when that version was built, offline. The online fallback is still the Markdown on GitHub.
- A heading or `<a id>` renamed in one page must be renamed in the links to it, or CI says which link broke.
