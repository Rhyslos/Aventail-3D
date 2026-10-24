# 0138. Curves: drawn or ready-made, with a live sweep or lathe

- **Status:** Accepted
- **Date:** 2026-10-24
- **Builds on:** [0006](0006-base-plus-operation-stack.md) (tools make commands, never change the scene directly), [0028](0028-ask-about-controls.md) (ask about controls), [0043](0043-attribute-layers-and-the-builder.md) (shapes carry UVs and seams), [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel), [0049](0049-slit.md) (a click-to-place tool with Enter and Esc), [0126](0126-rework-and-first-use.md) (Bake into shape, the development file format), [0129](0129-more-shapes.md) and [0130](0130-a-second-shape-list.md) (shapes and the B and Alt + B lists), [0132](0132-one-language-layer.md)–[0134](0134-shared-language-crate.md) (every text through the language layer), [0135](0135-live-booleans.md) (how a live feature is added), [0137](0137-biggest-first.md) (Phase 10's order)
- **Replaces part of:** [0126](0126-rework-and-first-use.md) (one development file format: curves get version 41)

## Problem

Phase 10's second milestone: curves and splines, with sweep and lathe, for pipes, cables, railings, trims, vases and wheels. The project lead chose (2026-10-24):

1. **Drawing like a pen tool** (Illustrator, Figma): click to place a sharp point; press and drag to pull out handles for a smooth bend. Enter or a right click finishes, Esc cancels (as Slit). Clicking the first point closes the curve, and Backspace takes back the last point (both recommended as common in pen tools).
2. **Sweep profiles:** a built-in cross-section (circle, square or rectangle, with size and sides) for pipes and cables, or another curve for rails and trims.
3. **No new key:** **Draw curve** in the Create menu, the shape lists and command search, beside ready-made **Line, Circle, Arc and Spiral** with their settings in Attributes.
4. **Editing:** select the curve and press 1: its points and handles show as dots, moved with the usual gizmo, selected and deleted as a mesh's; a double-click switches a point between sharp and smooth.

Left to decide: what a curve is in the scene, where sweep and lathe live, how the profile follows the curve, what the solid's mesh looks like, the construction plane and snapping, what the Edges and Polygons modes do, where curves sit in the shape lists, exporting, saving and drawing.

## Options

For where the solid lives:
1. **Inside the curve object, as its setting (chosen):** a curve's kind holds its curve and what it makes (a line only, a sweep or a lathe), changed by commands, worked out whenever its shape is asked for, like a primitive's settings. Undo, saving and copies come free.
2. A separate "sweep" object naming a path and a profile: more flexible, but two objects for every pipe, and nothing gained for the common case.
3. An edit recorded on top of the curve: edits find their elements by id, and a curve's solid is new geometry whenever a point moves.

For following the curve:
1. **Rotation-minimising frames (chosen):** the double-reflection method of Wang, Jüttler, Zheng and Liu (2008). The cross-section turns as little as possible, never flipping at bends or spinning on a spiral, unlike Frenet frames, which flip where the curve straightens or bends the other way.
2. A fixed up direction: simple, but it fails wherever the curve runs straight up.

For the UVs:
1. **Pieces at their real sizes packed into the texture square (chosen),** as every primitive's are (decision 0095): U round the profile and V along the curve, at their real lengths against each other; each cap a piece of its own. Substance Painter bakes need UVs inside the square with no overlaps.
2. UVs at real-world size (a metre a UV unit), for tiling textures: better for long pipes in a game engine with a repeating texture, but outside the square. The texel density tools can scale the packed pieces up for that.

## Decision

**A curve object.** `ObjectKind::Curve(CurveObject)`: its curve (`CurveShape::Ready`, a ready-made curve with its settings, or `CurveShape::Points`) and its solid (`CurveSolid::Line`, `Sweep` or `Lathe`). The curve (`mesh::curve::Curve`) is a list of Bézier points, each with a place, an in and an out handle (offsets from the point) and a smooth switch, open or closed, in the object's own space. Point ids count up and are never reused, as a mesh's (`CurvePointId`; at most 4 096 points and ids below 65 536). Ready-made curves (line, circle, arc, spiral) lie on the ground round the pivot, as primitives do; editing their points turns them into points (one undo step). A drawn curve's pivot is the middle of its points.

**Changes go through commands:** `Command::EditCurve` (a `CurveEdit`: move parts by a matrix, delete parts, make points smooth or sharp, close or open), `Command::SetCurveSolid` and `Command::SetReadyCurve`, each a reversible `SetKind` edit, so undo works and the tools never touch the scene. A curve takes no recorded edits (refused with a reason: bake it into a shape first), and no edit layers. **Bake into shape** turns its solid into a plain mesh (a line only has nothing to bake). **Apply transforms** moves its points. Deleting the curve another one uses as its cross-section gives that one a round cross-section in the same undo step; copying a curve with its cross-section points the copy at the copy; a curve pasted with a cross-section that isn't there goes round.

**Sweep** (`mesh::curve::sweep`): the profile (a circle of `sides`, a square, a rectangle, or another curve laid flat on the plane it lies in, round its pivot: drawn in the top view, x is X and y is Y; in the front view X and Z; in the right view Y and Z) carried along the curve on rotation-minimising frames, starting upright (the world's Z made square to the curve's first direction). A closed curve spreads the small leftover turn evenly round, so the ends meet. **Twist** turns the profile evenly from start to end (rounded to whole turns on a closed curve). **Segments:** with **Adaptive** (on as standard), pieces for each quarter turn the curve bends (a straight stretch is one piece; a twist asks for more too); otherwise pieces for every stretch. At a sharp corner the profile is stretched across it by 1 / cos(half the angle), at most four times, so the solid keeps its width on both legs (a mitre). **Caps** close an open curve's ends with flat n-gons. An open profile makes an open strip (a trim), with no caps.

**Lathe** (`mesh::curve::lathe`): the curve spun round its own X, Y or Z axis (through the pivot), through an **angle** of 1° to 360°, in **segments round** (3 to 256), following the curve as a sweep does. Points on the axis become one point (a pole), so the surface closes there. With **caps**, an open curve's ends that stop short of the axis get a disc (or a pie slice for a part turn), a part turn gets a flat face at each side, and an open curve whose ends stand off the axis at the same height closes straight across between them.

**The solid** is quads (triangles at poles), facing outward and closed where caps close it; hard edges along the profile's sharp corners, round the caps, and round the curve's corners sharper than 30°; one material slot (the object's first material); seams where its UV pieces part. At most 2 million polygons: more detail is cut back evenly. Sizes, sides, segments and angles are brought within limits; nothing that isn't a number gets through.

**Drawing (Draw curve).** Picked from **Create**, the **Alt + B** list or command search; it puts the app in the Model mode (curves are objects). Points go on a construction plane chosen with the first point: the ground (the grid) in the perspective view; in a flat view, the plane facing it through the world's middle (its grid). With snapping on, a point snaps to a point, edge or polygon under the mouse, else to the grid's lines; handles follow the mouse freely. While drawing, the curve so far, its points and handles and the stretch the next click would add are drawn over the view. Nothing is recorded until the curve is finished: then it is one new object, one undo step, selected, and the tool goes back to Select. Finishing: Enter, a right click (not a drag), clicking the first point (with three points or more: closes it), or picking another tool or mode (keeps what was drawn). Esc cancels. Backspace takes back the last point. A curve needs two points.

**The Points mode.** The drawing gives a selected curve, in the Points mode, a mesh of loose points named by each part's code (`CurvePart::code`: the point id times three, plus 0 for the point, 1 its in handle, 2 its out handle), so the existing picking, box and brush selection, Ctrl adds, Shift removes, the selection's undo and the gizmo all work unchanged; the gizmo's change becomes `CurveEdit::Transform` on those parts. Moving a point brings its handles; moving one handle of a smooth point turns the other to stay in line (keeping its length). Delete and Backspace delete the selected points (a handle on its own goes back to nothing, making that side sharp); a curve keeps at least two points. A double-click on a point or handle switches the point between sharp (handles back to nothing) and smooth (handles in line, pointing from neighbour to neighbour, a third of the way to each). **The Edges and Polygons modes** have nothing selectable on a curve; switching to them with a curve selected says so in the status bar.

**Drawing curves in the viewport.** Every shown curve's line is drawn as overlay lines (the depth-tested, screen-sized lines the Points, Edges and Polygons modes use), in the accent colour when selected; in the Points mode, a selected curve's points and handles too. No new render path, so no new GPU test is needed. In the Model mode a click near a curve's line (6 screen points) selects it where no surface is under the mouse, and a box touching its line takes it.

**Lists.** Draw curve and the four ready-made curves close the **Alt + B** list (after the eleven other shapes), not the B list: B is for the everyday solids and stays short, and curves are a different kind of thing (lines that become solids through a sweep or a lathe), used less often. The Create menu lists them after a line.

**Ctrl + wheel** (decision 0048) is for a tool's count while the tool is in use. Draw curve has no count, and a sweep's or lathe's counts are settings of an object, not of a tool in use, so they are ordinary number fields in the Attributes panel.

**Exporting.** A curve with a solid exports as its solid (a mesh like any other); a curve that is only a line exports as nothing (game engines and Substance Painter have no use for a bare line). It can be a boolean's cutter or kept shape, and its solid can have a live mirror, as any shape's.

**Saving.** Development file format version 41: version 40's model, in which a curve object is an empty group in its place (name, transform, settings), plus each curve's points (with their ids and the next id) or ready-made settings and its solid. Curves get a version of their own, as booleans did: an older app would open them as empty groups without a word and lose them on saving. Older files open. A file's curves are checked (points real and within 10 km, ids unique, the object stored as an empty group, cross-sections that are curve objects other than themselves); settings are brought within limits.

## Consequences

- Projects with curves need this version of the app or later.
- A curve's solid can't be edited point by point: Bake into shape first. The UV Editor and edit layers leave curves alone.
- The interface keeps a curve's selected parts as point ids (their codes), so the selection machinery needs nothing new; the Delete key, the double-click and the gizmo check whether an object is a curve.
- Later milestones build on this: curve deform (bending a shape along a curve) uses the same frames, and scatter can place copies along curves.
- Not yet: adding a point in the middle of an existing curve, joining two curves, and curves in the UV Editor; they can join later.
