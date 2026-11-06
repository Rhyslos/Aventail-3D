# 0241. Several UV sets per object

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 13. An object had one UV layout, so a lightmap for Godot (milestone 14) or a second detail texture had nowhere to go.

## Decision

- **The active set is always the `uv` corner layer** (`mesh::uv_sets`), so every UV tool, painting and mending work on it unchanged, instead of every UV function taking a layer name (the plan's "large but mechanical change"). Other sets are corner layers named `uvset:` and their name; the active set's name is kept by an empty polygon layer `uvname:` and its name (none: `UVMap`). Making a set active swaps its values with `uv`'s. Names: up to 32 letters, digits, spaces, `-` and `_`; up to 8 sets.
- **`Operation::UvSet { change }`** (`SetChange`: add as a copy of the active set, copy, rename, delete, make active), recorded on every selected object as one undo step; **file format 89** stores it (`v89::UvSetEditV89`). The sets themselves are layers, saved as every layer is, and carried through edits as corner layers are.
- **UV → UV sets** lists the first selected object's sets (radio: the one worked on) with a name field, Add, Rename and Delete. No keys. Changed from the plan: the UV menu rather than the Attributes panel, beside the other UV tools.
- **Export**: glTF gets the active set as `TEXCOORD_0` and one more as `TEXCOORD_1` (`mesh::uv_sets::second`: the set called `lightmap` first, else the first other by name) — Godot reads only UV and UV2. OBJ holds the active set only.
- Changed from the plan: no texture per set yet; painting uses the set worked on.

## Consequences

- Switching sets changes what painting lays on; the manual warns.
- More than two sets are kept and edited but only two are exported to glTF.
