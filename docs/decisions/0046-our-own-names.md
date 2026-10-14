# 0046. Our own names for borrowed ones

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** the project rule "copy behaviour, never assets" (CLAUDE.md); planned with the project lead on 2026-10-13 (DESIGN.md decision log)

## Problem

Some names in the app, the manual and the code came from other programs:
- Cinema 4D's "Attribute Manager", "Object Manager" and "Live selection";
- 3ds Max's "Quick Align", for a tool still to come.

The project copies how good tools behave, never their assets, and product-specific names count as assets. Plain, descriptive names are also easier for someone new to 3D.

## Decision

| Was | Now |
|---|---|
| Attribute Manager | **Attributes panel** |
| Object Manager | **Objects panel** |
| Live selection | **Paint select** |
| Quick Align (planned) | **Snap to** |

- The app's tool label, the manual (page titles, links and text), the plans (DESIGN.md, README), crate READMEs and code comments use the new names.
- The panels' tabs already read "Attributes" and "Objects", so the window looks the same apart from the Paint select label.
- Decision records, dated lines in the DESIGN.md decision log, the history table and released changelog entries keep the names they were written with: they record what was said at the time.
- Phrases describing the *style* of the layout ("a Cinema 4D-style layout") stay; they say where the idea came from, not what our parts are called.

## Consequences

- No behaviour or file changes; a screenshot from before (`2026-10-10-object-manager.png`) keeps its file name.
- New features use our own names from the start; when a borrowed name slips in, it is renamed the same way.
