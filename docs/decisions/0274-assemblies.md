# 0274. Assemblies on the shape canvas: placed pieces, linked copies and groups

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 12 of Phase 12 ([the plan](../plans/phase-12.md)): "Many shape pieces on one canvas, each with its own colour, height and order. Pieces can be dragged, turned, sized, flipped and grouped. Plain copies, and linked copies: change one and every linked copy changes, the way instances work in the scene. For example, build a leaf once, then a twig out of linked leaves." Milestone 11 ([decision 0273](0273-shape-canvas.md)) moved, turned and sized pieces by changing their points, so a copy could never share its shape with another, and every piece came out white at full height.

## Options

**How a linked copy shares a shape.** (a) Keep a list of shapes in the drawing and have pieces point at one; (b) **each piece keeps its own shape, and pieces with the same link keep identical shapes, every change to one made to all** (chosen). (b) keeps a piece whole (the edge randomiser and veins in milestone 13 read one piece), saves simply, and a damaged file is easy to catch: `Drawing::from_parts` refuses linked pieces whose shapes differ.

**Where a piece sits.** Moving a linked copy must not move the others, so **a piece's place is kept apart from its shape** (`Piece::place`, a turn, size, flip and move as one `Change`). Moving, turning, sizing and flipping change the place only; editing points edits the shape, worked back through the place (`Drawing::with_path` takes the points where they are on the canvas), so dragging a point of a turned copy moves the same point on every copy, each turned its own way. `Drawing::drawn_paths` stays the one place a piece becomes what is filled, so the pictures and the canvas's outlines agree. A place that squashes a piece flat is refused ("size it up a little first").

**What a link is.** A number from the drawing's id counter (`LinkId`): a new piece's link is its own id, a linked copy takes its source's, and **Unlink** gives a piece a fresh one. Pasting linked onto another canvas keeps a link only when a piece with that link and the same shape is there; otherwise copies of one shape are linked to each other only. Linked copies share their shape and what they do (Add, Cut out, Line); each keeps its own place, name, colour, height and mirroring, and (milestone 13) will keep its own variation seed.

**Groups.** Nested groups were considered and left: one level covers a twig of leaves and keeps the pieces list readable. **A group's pieces always sit together in the order**: grouping gathers them where the topmost was (never inside another group); an ungrouped piece moves up and down past whole groups; a piece in a group moves only within it; **Group up / Group down** move the group as one. Clicking a grouped piece on the canvas selects its whole group (so dragging moves it as one); clicking a piece in the list selects just that piece, to change one leaf of a twig.

**Colour and height.** Each piece has a colour (white at first, so the new **Colours** output matches the Mask until colours are chosen) and a height (0 to 1). They are laid in order like the mask: each piece covers the ones under it by its coverage, and a cut-out takes colour and height away as it takes the shape away. The **Height** output is the shape puffed up as before, times the height of the piece on top there (the puff follows the whole shape's edge, not each piece's: one distance field rather than one per piece). The **Colours** output is each piece's colour over black.

**Flipping.** No key was named in the plan, and the keys other apps use for it (Shift + H and the like) already show and hide objects in the app. **Flip ↔ and Flip ↕ are buttons** in the pieces list, flipping the selected pieces about their middle; no new key.

## Decision

- **`shapes`:** `Piece` gains `place` (`Change`), `colour`, `height`, `link` (`LinkId`) and `group` (`GroupId`); `Piece::placed`. `Change::IDENTITY`, `then`, `inverse`, `is_usable`; `Path::is_finite`. `Drawing::with_path` (points on the canvas, every linked copy), `with_shape`, `with_kind` (every linked copy), `with_colour`, `with_height`, `with_change` (the place), `flipped`, `copied` / `with_pasted` (`PieceCopies`), `with_unlinked`, `linked_count`, `with_group`, `without_group`, `with_group_name`, `with_order` (past whole groups), `with_group_order`, `groups`, `group_pieces`, `middle_of` (`drawing/assembly.rs`); `Problem::Squashed`, `BadLink`, `BadGroup`; `raster::layers` (mask, colours and heights in one pass).
- **`matgraph`:** the Shape box gains a fourth output, **Colours** (`colours`, a colour), and its Height takes each piece's height; `drawn` returns `Drawn`. Gradient map joins its "usually comes next".
- **`fileio`:** **file format 101**: `v100::PieceV100` gains `place`, `colour`, `height`, `link` and `group`, and `DrawingV100` gains `groups` (`v101::GroupV101`); older files open with every piece where it was drawn, white, full height, unlinked and ungrouped.
- **`ui`:** on the shape canvas, **Ctrl + C** (or Alt + C) copies the selected pieces, **Ctrl + V** pastes copies of their own at the pointer, **Alt + V** pastes linked copies, as for objects and boxes (decision 0055); **Alt + G** groups and **Shift + G** ungroups, the app's group keys. The copied pieces paste on any Shape box's canvas and in any tab (decision 0268). The pieces list shows groups as headings with their pieces under them and marks linked copies; the selected pieces' rows have **Colour**, **Height**, **Flip ↔**, **Flip ↕**, **Group**, **Ungroup**, **Copy**, **Paste**, **Paste linked**; a chosen group's its name, **Group up**, **Group down**; one chosen piece's also "Linked with N other pieces" and **Unlink**. The canvas shows each piece in its colour, and the strip shows the four outputs.

## Consequences

- A leaf is drawn once and a twig built from linked copies of it; reshaping the leaf reshapes the twig, every copy staying where it was put.
- **Milestone 13 (vary copies)** can add a per-piece seed (a new `Piece` field, saved beside `colour`) and vary each copy's shape inside `drawn_paths`, after the link has made them share one shape; the edge setting belongs with the shape (shared by linked copies).
- A piece's place is applied before the mirror line, so a mirrored twig mirrors each copy where it is.
- Undo, saving, copying boxes and own boxes need nothing new: everything is still one `GraphEdit::SetDrawing`.
