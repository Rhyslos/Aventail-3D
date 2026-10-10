# 0004. GPL 3.0 license, permissive dependencies

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Aventail 3D is meant to be a free alternative for everyone, and to stay that way. The license decides what others may do with the code; the project lead also wants to keep the option of selling it later.

## Options

1. **MIT or Apache 2.0:** maximally permissive; anyone, including a competitor, may close the code and sell their own version.
2. **GPL 3.0 or later:** anyone may use, change, share and sell it, but distributed modified versions must stay open under the same license. Used by Blender, Krita and GIMP.

## Decision

GPL-3.0-or-later for the app. Everything people create with it is theirs: the GPL covers the program, not its output.

To keep future options (paid builds, dual licensing) with the project lead:

- **Code contributions from others are not accepted for now.** Ideas, bug reports and feedback are. If that changes, contributors sign a Contributor License Agreement first.
- **Every dependency stays permissive** (MIT, Apache, BSD and similar; see `deny.toml`). A GPL dependency would rule out dual licensing.

## Consequences

- Nobody can take the app closed and sell it; forks stay open.
- Selling remains possible: paid builds (as Krita does), support, extras, or a commercial license, since the project lead holds the copyright.
- Copyright in AI-generated code is legally unsettled. The project lead's design decisions, the decision records and the git history document the human authorship; legal advice should be sought before any commercial launch.
- Already-released versions stay GPL forever, whatever happens to later ones.
