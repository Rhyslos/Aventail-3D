# 0117. Language support as Phase 9

- **Status:** Accepted
- **Date:** 2026-10-17
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (Phase 8, the UVs)

## Problem

The app is in English only. The project lead called language support a "huge thing we need to fix" and asked for it as the first task after the UVs. About 1 500 pieces of interface text are written straight into the `ui` crate today, and every milestone adds more, so the sooner they go through one place, the less there is to move.

## Options

- **When:**
  1. **Its own phase, Phase 9, right after Phase 8 (chosen, the project lead's request).** Advanced hard-surface becomes Phase 10, scripting Phase 11, rigging Phase 12; texturing still comes after advanced hard-surface.
  2. In the middle of Phase 8. It would interrupt the stacked UV pull requests.
- **How text is translated** (to confirm when the phase is planned):
  1. **Project Fluent (the `fluent` crates, Apache-2.0/MIT) (likely).** Made for interfaces: plurals, word order and numbers per language, with translators' files that are plain text.
  2. gettext-style catalogues. Weaker on plurals; most Rust libraries for it are GPL or LGPL, which the project can't use.
  3. Our own key-value tables. Simple, but plurals and word order would have to be invented again.
- **What else it needs:** a language setting that follows the computer at first and switches without a restart; numbers and units the local way (a decimal comma), with project files and exports unchanged; fonts for other alphabets (Noto, OFL) loaded only when needed; English as the fallback for anything not yet translated; and a check that catches new text skipping the translation layer.

## Decision

Language support is Phase 9, with the seven milestones in DESIGN.md §10 (groundwork, moving the text, the setting, numbers, fonts, the first translations, the rule for new text). Before planning it in detail, ask the project lead:

- which languages come first;
- whether the user manual is translated too, or only the app;
- who can check translations (native speakers among the testers).

## Consequences

- Phase numbers after 8 shift by one; DESIGN, README, ARCHITECTURE and the manual are updated. Older decision records keep the numbers they were written with.
- Until Phase 9, new interface text is still written in English in the code, as now; it is all moved over in Phase 9's second milestone.
- From Phase 9 on, every pull request adding text adds its English source text to the translation files.
