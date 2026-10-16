# Phase 8 plan: UV powerhouse and baking

The working plan for Phase 8 ([decision 0108](../decisions/0108-uv-powerhouse-and-baking.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-8--uv-powerhouse-and-baking)). It was written at the project lead's request when milestone 4 was finished, so a later session can carry on where this one stopped. It records:

- what is built;
- how the work is being done;
- the controls the project lead has already chosen;
- a plan for every milestone still to come.

Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan when its controls are asked about or its design changes.

## Where things stand

Each milestone is one pull request, stacked on the one before. None is merged yet: the project lead merges them in order, after the UV phase's PRs (#85 onward).

| # | Milestone | Branch | PR | Decision | File format |
|---|---|---|---|---|---|
| 1 | Pins, unfolding one way, unwrap methods | `uv/pins-and-methods` | #105 | [0109](../decisions/0109-pins-and-unwrap-methods.md) | 34 |
| 2 | Fully controllable automation | `uv/controllable-automation` | #106 | [0110](../decisions/0110-controllable-automation.md) | 35 |
| 3 | Seam preferences and stretch budgets | `uv/seam-preferences` | #107 | [0111](../decisions/0111-seam-preferences-and-stretch-budget.md) | 36 |
| 4 | Locked pieces and piece settings | `uv/locked-pieces` | #108 | [0112](../decisions/0112-locked-pieces-and-piece-settings.md) | 37 |
| – | The UV menu in groups (not a milestone) | `uv/menu-groups` | #109 | [0113](../decisions/0113-uv-menu-in-groups.md) | – |
| 18 | Layout snapshots | `uv/snapshots` | #110 | [0114](../decisions/0114-layout-snapshots.md) | 38 |
| 15 | Seeing and checking | `uv/seeing-and-checking` | see the PR list | [0115](../decisions/0115-seeing-and-checking.md) | – |

**Next:** the project lead asked for six milestones in least-work order, each stacked on the last: 18 (layout snapshots, built), 15 (seeing and checking, built), 23 ("Why is this here?") and 9 (precise placement). **Milestones 14 (lightmaps) and 20 (recipes) are skipped for now** at his request; don't build them until he asks. Branch the next one from the newest branch in the table. Next up: 23, then 9. The next decision is 0116 and the next file format 39.

## How the work is done (notes for the next session)

**Pull requests and docs:**

- One milestone per branch and pull request, each based on the previous milestone's branch.
- Every pull request starts with a **Controls** note and the **merge order**, and has a "How to test" list written for a non-programmer.
- It updates, in the same pull request:
  - the manual page(s), following `docs/manual/_template.md`, plus `SUMMARY.md` and `concepts.md`;
  - CHANGELOG;
  - DESIGN (the roadmap's *Built* line and the §13 log);
  - a decision record and its row in `docs/decisions/README.md`;
  - the crate READMEs;
  - the README tracker and history;
  - this plan.

**Code patterns** (see milestones 1 to 4 for examples):

- **Recorded edits.** A new kind of UV edit is a `scene::Operation` variant. Add it:
  - to each grouped match in `stack.rs`, `mirroring.rs` and `memory.rs`; a small script that adds `| Self::New { .. }` after a similar variant's line is quick and safe;
  - to `describe`, `numbers_are_real` and `apply_plain`;
  - to a new file format version: `fileio/src/project/vNN.rs`. It wraps the last version and stores only what older versions can't. Add a `rich_document` link in `project/tests.rs`, and a sample written with `touch tests/data/sample-vNN.avtl` then `WRITE_SAMPLE=1 cargo test -p fileio write_the_sample`.
- **Data on the shape.** Data that belongs to the shape (pins, locks, preferences, budgets, piece settings) is an attribute layer in `mesh::attributes` with read and write functions in `mesh::uvs`. The builder carries layers through modelling by itself.
- **Choices recorded with edits.** Every choice an automatic tool makes is recorded with its edit (`uv::UnwrapOptions`), so edits replay the same whatever the app's settings later (decision 0110). Project-wide settings that change evaluation travel in `scene::EvalContext`.
- **Tools that skip pieces.** Tools that work on many pieces skip locked ones: `uv::unwrap::chosen_pieces` already does.
- **Window tests.** They use egui_kittest. Submenus are found with `get_by_label_contains`. A fresh harness's cube isn't selected (`set_selection`).

**Checks:**

- Run cargo with `RUSTUP_TOOLCHAIN=stable`, `< /dev/null` and long timeouts.
- Before each PR: `cargo fmt`, clippy with `-D warnings`, `cargo doc` with `-D warnings`, `cargo test --workspace` and `cargo deny check`, plus the window smoke test on a 1440 × 900 Xvfb (`tools/smoke/window-smoke.sh`).
- Run the heavy-mesh check (`tools/heavy/heavy-check.sh`) at the end of the phase, and put its numbers in the last PR.

**Interface pressure:**

- **The E ring.** Its 15 UV places are full. New UV tools need a group (see decision 0109's Turn & flip) or a menu.
- **The UV menu.** It is grouped (decision 0113): Seams, Turn and flip, Lay out, Pieces, then Texture, Trim sheet and Check for baking, then Settings. Put new UV commands in the group they fit (a constant per group in `ui::workbench`); window tests open a group with `tests::uv_group`.

## Controls the project lead has chosen

These answers cover milestones still to come. Use them as given; ask only about what they leave open.

- **Stitch and cut** (milestone 8): **X** cuts and **C** stitches, in the UV Editor only (C differs from the viewport, where it bridges; the project lead knows). They are in the E ring's **Stitch & cut** group already, shown as arriving with milestone 8.
- **UV brushes and the seam brush** (milestones 6 and 10): pick from the menus, then drag to use. **Ctrl + drag** does the opposite (unmark seams, push instead of pull), **[** and **]** change the size, and **Esc** goes back to selecting.
- **Pivots** (milestone 9): a **Pivot** menu in the UV Editor. **Ctrl + click** places your own pivot point.
- **UV snapping** (milestone 9): the same as the viewport (the magnet button; holding Ctrl flips it), with a **Snap** menu in the UV Editor for its targets.
- **Projections** (milestone 7): **W**, **R** and **S** move, turn and scale the projection gizmo as usual. **Enter** or **Done** keeps the result; **Esc** cancels.
- **Lattice warp and measuring** (milestones 8 and 15): from the menus, then drag. No keys.
- **Layout snapshots, isolate and sync** (milestones 11 and 18): in the UV Editor's menus. No keys.
- **Pins** (built): the E menu and the UV menu, no key.
- **"Why is this here?"** (milestone 23): only while **View → Explain choices** is ticked in the UV Editor.

## The plan, milestone by milestone

Each entry gives the aim, a design, the main code, and what to ask the project lead first (**Ask**). Controls for anything not listed above must be asked about before building (decision 0028).

### 5. Straighten by flow, and the texture locked while moving points

- **Aim:**
  - Straighten works on any quad piece by following its edge flow, not only perfect grids.
  - Moving points can keep the texture stuck to the surface ("texture lock").
- **Design:**
  - *Straighten by flow:* pick the quads' loops (rows and columns by walking opposite edges, as `uv::unwrap::grid` already does). Where the piece isn't a full grid (holes, triangles, poles), straighten only the quad loops that run end to end and solve the rest around them with pins: the straightened corners become pinned wedges for `conformal_shapes`.
  - *Texture lock:* a switch in the viewport's Options (or the modelling Tools menu). While it's on, moving, rotating or scaling points re-solves the UVs of the polygons around them, with every corner not touched by the move pinned. Record it as part of the move: the move operation gains a `keep_texture` flag, so replays agree. (Mending already handles new polygons; this is about stretched ones.)
- **Code:** `uv::unwrap::straighten` (flow), a new `uv::follow` module for "move points, keep texture", `scene::Operation::MovePoints` / `TransformPoints` (a flag), file format 38.
- **Ask:** where the texture-lock switch goes, and whether it has a key. A recommendation: a palette toggle next to snapping, no key.

### 6. Seams: slider, brush, symmetric, loops

- **Aim:** quicker seam work.
- **Design:**
  - **Fewer seams ↔ less stretch slider:** a value from 0 to 1 that drives Suggest seams' stretch budget (decision 0111) and corner angle together, shown live as the slider moves (preview the seams before recording).
  - **Seam brush:** paint seams onto edges under the mouse. The controls are chosen (above). It records one `SetSeams` per stroke.
  - **Symmetric seams:** with symmetry on, seams are already mirrored (`SetSeams` is mirrored). Add **Mirror seams** (copy one side's seams to the other) for shapes without live symmetry, by matching mirrored edges with `mirroring::Partners`.
  - **Seams along loops and rings:** **Mark seam loop** and **Mark seam ring** use the existing loop/ring selection walkers on the selected edge.
- **Code:** `uv::seams` (slider mapping), a `workbench::seam_brush` tool (a `Tool` variant like the modelling tools), UV menu entries.
- **Ask:** where the slider lives (recommendation: UV → Seam suggestions, with a live preview while dragging), and whether loop and ring get keys.

### 7. Projections and world-aligned UVs

- **Aim:** planar, box, cylindrical and spherical projection with a gizmo on the model; project from view; world-aligned UVs for modular kits.
- **Design:**
  - A projection is an `Operation::ProjectUvs { faces, kind, transform, anchors }`. The transform is the projection gizmo's placement (where the box, cylinder or sphere sits), in object space.
  - Planar projects along the gizmo's axis. Box picks each polygon's axis by its normal. Cylinder and sphere unroll by angle, cut where the gizmo's seam line is (marking that as a seam).
  - The pieces are then scaled to the object's texel density, or kept at world size for **World-aligned** (texture widths per scene unit from the texture settings, so neighbouring kit pieces line up).
  - **Project from view** uses the camera's view direction as the planar axis.
  - The gizmo reuses the viewport gizmo: W, R and S move, turn and scale the projection shape. Enter or Done records; Esc cancels (controls chosen).
- **Code:** a new `uv::project` module, a `workbench::projecting` session (like the modelling tools' drag sessions), render overlay lines for the shape, file format 39.
- **Ask:** where the projections are offered. Recommendation: a **Project** submenu in the UV menu, and in the E ring by turning the Unwrap place into a group (Unwrap, Unfold across, Unfold up, Project), since the ring is full.

### 8. Editing pieces: stitch, cut, align, orient, lattice

- **Aim:** hand tools for pieces.
- **Design:**
  - **Stitch** (C in the UV Editor): for selected UV edges whose partner on the model lies elsewhere in the texture, move the smaller piece (fewer polygons) so the edge meets its partner (a similarity from the edge's two ends), then join the corners (shared UVs). Clear the seam mark on those edges. Records `TransformUvs` plus `SetSeams`.
  - **Cut** (X): mark the selected edges as seams and split the corners so the pieces can move apart (nudge each side's corners apart by a pixel). Records `SetSeams` plus a split operation.
  - **Align and distribute:** line up the selected pieces' left, middle, right, top or bottom, or spread them evenly. Records `TransformUvs` per piece.
  - **Orient by edge:** turn the piece so a selected edge lies straight across or up.
  - **Lattice warp:** a 3 × 3 (or 4 × 4) grid over the selection, dragged from the menu (controls chosen). The UVs are moved by bilinear (or Bézier) interpolation. Record the result as `PackUvs`-like moves per corner, which needs a new `SetUvs { corners, values }` operation.
  - Turn on the Stitch and Cut entries already in the ring's Stitch & cut group (`CommandId::UvStitch`, `UvCut`): remove their `arrives`.
- **Code:** a new `uv::stitch` module, `workbench::uv_pieces`, a lattice session in the UV Editor, file format 40 (for `SetUvs` / split).

### 9. Precise placement: snapping, exact numbers, pivots

- **Aim:** put pieces exactly where they belong.
- **Design:**
  - **Snapping**, the same as the viewport (chosen): targets are texture pixels, other pieces' corners and edges, trim rows, and a grid of your own size, chosen in a Snap menu in the UV Editor.
  - **Exact numbers:** with the UV Editor in charge, the Coordinates panel shows the selection's U, V, turn and size (in pixels or 0 to 1) and applies typed values as `TransformUvs`.
  - **Pivots:** a Pivot menu in the UV Editor offers the selection's middle, each piece's own middle, a corner of the selection's box, the texture's middle, or your own point (Ctrl + click places it, chosen). The gizmo and turn/flip use it.
- **Code:** `uv_editor::gizmo` (snapping and pivots), `coordinates` (a UV mode). No file format change.

### 10. UV brushes

- **Aim:** grab, relax and pinch UVs with a brush in the UV Editor (controls chosen).
- **Design:**
  - **Grab** moves corners within the radius with a smooth falloff.
  - **Relax** runs `uv::relax` steps weighted by the falloff.
  - **Pinch** pulls corners towards the middle.
  - Pins are respected. Each stroke records one operation with the final corner values (the `SetUvs` from milestone 8), so undo is one step per stroke.
- **Code:** a `uv_editor::brush` module, and a brush cursor drawn in the editor.

### 11. Selection, isolate and sync, copying UVs

- **Aim:** select UVs the smart ways, isolate pieces, choose whether the UV Editor follows the viewport, and copy UVs between matching pieces.
- **Design:**
  - Selection: **By overlap** (pieces lying over others, using `uv::check`'s grid), **Mirrored**, **Stretched** (by the stretch view's limits), **Similar** (same shape and size), **Grow and shrink in UV** (across shared UV edges only), and **Shortest path** in UV.
  - **Isolate** shows only the selected pieces in the UV Editor (a view setting, not an edit).
  - **Sync off** lets the UV Editor keep its own selection, separate from the viewport's. This needs a second selection in the document, or the editor keeping its own `ElementSelection`; the editor-held one is simpler.
  - **Copy UVs** from one piece to others with the same topology: match polygons by walking from a chosen corner, then record `SetUvs`.
- **Code:** `uv_editor` selection, `workbench::selection`, and a `uv::copy` module.

### 12. Packing

- **Aim:** packing groups, locked pieces, packing into a region, keeping the grain, per-object density, UDIM tiles.
- **Design:**
  - **Groups:** a polygon layer `uv_group` (a number). Pieces in one group are packed as one block, in order, with their own padding or region if set.
  - **Locked pieces** (decision 0112): their space is marked full on the packing grid before the rest are placed. `uv::pack::pack` takes pre-filled cells.
  - **Region:** pack into a rectangle of the square, drawn in the UV Editor.
  - **Grain:** pieces whose long side should stay horizontal (a polygon layer, or the piece's own settings) are turned only by 180°.
  - **Per-object density:** an object setting, used to scale before packing.
  - **UDIM:** a tile count across; pieces fill tiles 1001 and up in order. Exporters must keep UVs beyond 0 to 1.
- **Code:** `uv::pack` (pre-filled grid, groups, region), the Packing settings submenu, file format change for groups, region and UDIM settings.

### 13. Several UV sets per object

- **Aim:** more than one UV map per object, each with its own texture, for big models.
- **Design:**
  - Name the corner layers `uv`, `uv2`, and so on, with a list on the object of the names and which is active.
  - Every UV operation gains a `set` (`None` means the active set).
  - The UV Editor shows the active set. The Attributes panel lists the sets: add, rename, copy, delete, make active.
  - Export writes them all (glTF `TEXCOORD_n`; OBJ only the active set).
- **Code:** `mesh::uvs` taking a layer name everywhere (a large but mechanical change), `scene::Object` (set list), `fileio` (format change), and the exporters.

### 14. Lightmap UVs

- **Aim:** one button makes a second UV set laid out for Godot's baked lighting.
- **Design:**
  - Make a set `lightmap`.
  - Unwrap with Keep sizes and hard edges as seams, with every piece unique (no stacking or mirroring), then pack with generous padding at the lightmap's size (a setting, 512 by default).
  - Export it as `TEXCOORD_1` / UV2.
- **Code:** builds on milestone 13. **Ask:** where the button goes (UV menu recommended).

### 15. Seeing and checking (built)

Built as planned ([decision 0115](../decisions/0115-seeing-and-checking.md)): tints rather than outlines (red overlapping, also folded polygons; purple mirrored), a count top right, `uv::warnings` and `uv::measure`, View → Numbers on pieces and View → Measure (Esc stops). Milestone 23 can hook its explanations onto the warned pieces (`UvEditor::warnings`).

### 16. The numbered checker per object

- **Aim:** each object's numbered checker in its own colours, to tell objects apart.
- **Design:**
  - Each object gets a hue (from its id, or chosen in the Attributes panel).
  - The checker image is made per hue (cache a few). The GPU surface binds the object's image: `render` needs per-draw textures, or a texture array indexed by an object number.
- **Code:** `uv::grid::numbered_checker` takes a base colour, `render` (texture per draw item), and the Display menu (**Numbered checker per object**).

### 17. Seeing textures

- **Aim:**
  - a real texture behind the UVs and on the model;
  - UV layout export (PNG, SVG);
  - separate angle and size stretch views;
  - a mipmap preview.
- **Design:**
  - **Texture:** load an image (PNG/JPEG through a permissive crate such as `image` or `png`; check its licence), keep it per project (as a path), show it in the editor and as a surface choice.
  - **Layout export:** rasterize the UV edges to PNG at the texture size, or write SVG lines.
  - **Angle and size views:** split today's stretch view into two `SurfaceDraw` modes.
  - **Mipmap preview:** draw the texture at a chosen mip level.
- **Code:** `render`, `uv_editor`, `fileio` (image import and layout export).

### 18. Layout snapshots (built)

Built as planned ([decision 0114](../decisions/0114-layout-snapshots.md)): the UV Editor's Snapshots menu; `scene::snapshot`, `Command::SetSnapshots` and `Operation::RestoreUvs`; file format 38. A snapshot holds each object's `mesh::uvs::UvLayout` (polygons' corner UVs and seams), not one `SetUvs`/`SetSeams` pair. Later UV sets (milestone 13) can add the set to `UvLayout`.

### 19. Explained auto-unwrap

- **Aim:** one button runs seams, unwrap, straighten, density, pack and lightmap UVs, then lists every step to tweak or undo on its own.
- **Design:**
  - Run the existing tools in order, each recorded as its own edit in one edit layer named "Auto-unwrap" (decision 0090 layers), so each can be switched off or changed in the edit list.
  - A panel lists the steps with what each did ("12 seams, 5 pieces, packed at 92%").
- **Code:** `workbench::auto_unwrap`, and the layers API.

### 20. Unwrap recipes

- **Aim:** unwrap a whole kit at once with saved steps.
- **Design:**
  - A recipe is the auto-unwrap's step list with its settings, saved by name (in the app's settings, or in the project to share).
  - **Apply recipe** runs it on every selected object.
- **Code:** builds on milestone 19. **Ask:** recipes in the app or the project (recommendation: both, project first).

### 21. Player-view UVs (new)

- **Aim:**
  - seams hidden, and texture detail placed, by how the player sees the asset;
  - a texels-per-screen-pixel view.
- **Design:**
  - The user places a few "player cameras", or picks a preset (third person at 3 m, first person, top-down).
  - From those views, compute each polygon's visibility (rendered ID passes) and its screen size.
  - Suggest seams weights visibility by them instead of the fixed below-and-behind rule (`HideSeams::Player`).
  - Packing gives polygons texture in proportion to their screen size (per-piece density from milestone 4).
  - A surface view colours texels per screen pixel.
- **Code:** `render` (ID passes read back), `uv::seams` (a visibility input), and `uv::pack`.

### 22. UV intents (new)

- **Aim:** pieces keep what they were told to be after any change: a strip, on a trim row, a mirror, a density, this way up.
- **Design:**
  - An intent is a polygon layer (with a small parameter) set by the tool that made the layout: Straighten sets "strip", Put on trim row sets "trim row N", Set density sets "density", and so on.
  - Mending (and any re-unwrap) reads the intents and re-applies them to the piece after the edit, instead of a plain lay-out.
  - The UV Editor shows a piece's intents on hover; they can be cleared.
- **Code:** `uv::mend` (re-applying intents), `mesh::uvs` (an intent layer), and the tools that set intents.

### 23. "Why is this here?" (new)

- **Aim:** every automatic seam, size and packing choice explains itself and can be changed.
- **Design:**
  - The tools return reasons with their results: each suggested seam's reason (hard edge, sharp corner N°, needed to lie flat, stretch budget slit, preferred) and each packed piece's scale and place.
  - Keep the last run's reasons per object (not saved).
  - Hovering a seam or piece in the UV Editor or viewport shows the reason with a fix button (clear this seam and prefer another; never here).
- **Code:** `uv::seams::suggest_with` returning reasons, and the `uv_editor` hover.

### 24. Paint-aware detail and seam preview (new)

- **Aim:**
  - more texture on the edges and corners where wear gets painted;
  - bake seams shown before baking.
- **Design:**
  - Weight per-piece density by curvature (convex edges, where wear and edge masks go).
  - **Seam preview:** a surface view that renders the seams as a bake would show them (a thin line where the normal-map split falls), so the user can move seams before baking.
- **Code:** `uv::density` (weights) and `render` (a seam-preview surface).

### 25. The baker

- **Aim:** bake normal (matching Godot's tangent space), ambient occlusion, curvature, thickness, colour ID, position and height.
- **Design:**
  - A CPU ray-caster with a BVH over the high-poly shape, run on threads; a GPU path later.
  - Tangent space as Godot computes it: MikkTSpace. Check the licence of a Rust MikkTSpace port (`mikktspace` is zlib/Apache: fine).
  - Output PNGs at the texture size, with padding dilation (the padding from Pack).
  - A Bake window (decision 0087's window look) picks maps, size and samples.
- **Code:** a new `bake` crate (core layer: no UI or GPU), `fileio` (PNG writing), and a `ui` window. **Ask:** where Bake lives (File → Bake… recommended).

### 26. High to low with a cage

- **Aim:** _low/_high pairing, exploded bakes, floating details, and an automatic cage shown on the model, with a push slider and cage painting.
- **Design:**
  - Pair objects by name suffix.
  - **Explode:** move each pair apart for the bake only.
  - **Floating details** are high-poly pieces with no low-poly partner, projected.
  - The cage is the low-poly shape pushed out along averaged normals by the push distance, drawn as a see-through overlay. Painting adds per-point push.
- **Code:** `bake`, and a viewport overlay and paint tool for the cage.

### 27. Live bake preview and explained problems

- **Aim:**
  - see the bake on the model as settings change;
  - show missed rays, wrong parts and skew on the model, each explained with a fix.
- **Design:**
  - Bake at a low size progressively (tiles) and show the result as a surface.
  - Record per-texel problems (no hit, hit on the wrong pair, ray at a steep angle) as a mask image shown as a surface view, with the bake check's explain-and-fix pattern (decisions 0077 and 0105).
- **Code:** `bake` (progressive, problem mask), and `render`.

### 28. Bakes for the pipeline

- **Aim:**
  - bake to vertex colours;
  - bake a whole kit at once;
  - move stacked and mirrored pieces aside for the bake;
  - send results straight into a Godot material (a normal map and an ORM texture).
- **Design:**
  - **Vertex colours:** sample at points instead of texels.
  - **Kit bake:** loop over the kit-export set (decision 0078).
  - **Stacked and mirrored pieces:** offset them by one texture width during the bake, then back.
  - **ORM:** pack occlusion, roughness (a constant or a map) and metallic into one image.
  - Write the files beside the kit export, referenced from the exported glTF material.
- **Code:** `bake` and `fileio` (glTF material textures).
- **Phase end:** run the heavy-mesh check and put its numbers in this PR, then check the phase gate with the project lead: a modular kit is unwrapped, packed and baked in the app and looks right in Godot, and its UVs and intents survive later changes.
