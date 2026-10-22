# 0124. Select connected in the UVs: whole pieces

- **Status:** Accepted
- **Date:** 2026-10-22
- **Replaces part of:** [0118](0118-select-connected-and-outside-warnings.md) (what Alt + A does in the UVs)

## Problem

Decision 0118 made Alt + A select everything joined to the selection on the model, across seams, in every mode. Testing in the UV Editor, the project lead expected it to select the UV piece (island) he was pointing at: on a model of one part, Alt + A selected every polygon of the object. Without a way to pick out one whole piece, two pieces lying over each other can't be separated: a click or box over them takes bits of both. He called this a big issue.

## Options

1. **Alt + A means whole pieces over the UV Editor and in the UV mode, and the whole part elsewhere (chosen).** Same key, and the place decides, as E and H already do over the editor.
2. A second key for pieces. One more key to learn, for what people expect the first to do.
3. Pieces everywhere. Loses the whole-part selection in the viewport, which is useful for modelling (a bolt inside a bigger shape).

## Decision

- `mesh::selection::connected_in_uvs`: the elements of the kind asked for in every UV island the selection touches (a polygon its own; a point or edge the islands of its polygons, so both sides on a seam). A mesh without UVs falls back to `connected`.
- Alt + A asks for it when the mouse is over the UV Editor or the mode is UV; otherwise `connected` as before. UV → Pieces → Select connected uses it too; Select → Select connected stays the whole part. Undo step "Select whole pieces".

## Consequences

- Separating overlapping pieces takes three steps: select a little of one, Alt + A over the editor, move.
- A point or edge on a seam brings the pieces on both sides, because a selected point isn't tied to one side of a seam. The manual says to select a polygon for exactly one piece.
- The newcomer checklist (0121): no new key, no new concept; the key's meaning follows where the mouse is, as other UV Editor keys do.
