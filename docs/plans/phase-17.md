# Phase 17 plan: sculpting, for stylised creators

The working plan for Phase 17 ([decision 0300](../decisions/0300-sculpting-plan.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-17--sculpting)). The project lead asked on 2026-11-17 for the sculpting milestones to be added. The only thing already settled was the aim: sculpting **focused on stylised creators** ([decision 0216](../decisions/0216-sculpting-in-alpha-out.md)). The project lead couldn't answer questions that day, so Claude made every choice below, picking the option most 3D programs use that is safe and clashes with nothing. **Each choice is listed with its alternatives under [Choices to review](#choices-to-review)**, to go through together before the first milestone is built.

Update this plan as milestones are built: tick them off here and in the README tracker, and change a milestone's plan when its design changes.

## Choices to review

Each item: the question in plain words, what was chosen, the alternatives, and what changing it later would cost. Until they are reviewed, these are the recommended choices ([decision 0028](../decisions/0028-ask-about-controls.md)); every milestone's pull request says so at the top.

1. **How big is the phase?** Chosen: **two parts, Phase 17a (sculpting the forms) and Phase 17b (detail and the game-asset finish), six milestones each**, each part ending with a gate you try by hand. Alternatives: one phase of twelve (breaks the rule of two to six milestones per phase); three parts of four (more gates, slower). *Cost to change later:* none; it only regroups the same milestones.
2. **When is it built?** **Changed by the project lead on 2026-11-17: now**, straight after and beside scripting (Phase 14), before rigging and animation (Phases 15 and 16), since sculpting needs nothing from them; it builds on the edit layers, the baker, the modifiers and the painter's brushes, which all exist. First chosen: where the roadmap had it, after Phases 15 and 16. *Cost:* none; the plan doesn't depend on its place.
3. **Where does sculpting happen?** Chosen: **a Sculpt mode**, a button in the left palette between Polygons and UV (model, then sculpt, then unwrap, then paint), and a **Sculpt layout** button on the menu bar beside Standard, UV Edit, Paint and Material. Like Paint mode, it has its own brush bar, shelf and keys. Alternatives: sculpt brushes as tools in the Polygons mode (no mode switch, but the keys and the panels clash with modelling); a separate sculpting window. *Cost:* small before milestone 1, large after (the layout, keys and manual are built around a mode).
4. **How is sculpted detail kept?** Chosen: **a Sculpt layer in the object's Edit Layers stack.** It keeps the *result* of sculpting (how far each point has moved, in chunks of points), not every stroke as an edit to replay. Each stroke is one undo step that keeps only the chunks it touched, like a paint stroke keeps only its tiles. Alternatives: record every stroke as an operation and replay them (thousands of strokes over millions of points would make opening a project or changing a lower layer take minutes); sculpt only on a shape with no stack (simplest, but breaks "base settings stay editable", wish W4). *Cost:* very high after milestone 1: it sets the file format, undo and the stack.
5. **How are the moves measured?** Chosen: **along the surface** ("tangent space": out along the way the surface faces, and sideways along it). Changing the shape under the sculpt (a primitive's size, a lower edit layer, a lower detail level) carries the detail along with it, the way 3D programs' detail levels do. Alternative: in the object's own space (simpler; detail stays where it was in space when the shape under it changes, so it tears). *Cost:* high after milestone 1 (the file format and every brush).
6. **How does the surface get more polygons where you need them?** Chosen: **Remesh for sculpting** (an even surface at a size you choose, merging overlapping pieces into one; milestone 17a.5) and **detail levels** (the shape subdivided up to six times, each level sculptable, the lower levels kept; milestone 17b.1). **Not dynamic topology** (polygons added under the brush as you sculpt). Why: stylised work is mostly big clean forms built from blocked-out pieces, which remeshing serves well; detail levels give clean quads to bake from and to make a low-poly from; dynamic topology makes uneven triangles, can't keep detail levels, and is the hardest of the three to make fast and crash-free on millions of polygons. Alternatives: dynamic topology as well (Blender, Nomad Sculpt, ZBrush's Sculptris Pro); detail levels only; remesh only. *Cost:* adding dynamic topology later is one extra milestone of its own; it would wipe an object's detail levels when used, as it does in other programs.
7. **What happens to booleans, the live mirror and modifiers on a shape you sculpt?** They come after the edit layers, so after the sculpt. Chosen: **on entering Sculpt mode, the app asks once per object**: *Bake them in* (recommended: one undo step, then you sculpt exactly what you see) or *Sculpt under them* (they're hidden while you sculpt and come back when you leave). Alternatives: always bake them in; always hide them; sculpt through them (sculpting the mirrored side or a modifier's result and sending the change back down: complicated, and impossible for most modifiers). *Cost:* small.
8. **Brush keys.** Chosen, in Sculpt mode, number keys like Paint mode's: **1** Draw, **2** Clay strips, **3** Inflate, **4** Crease, **5** Flatten, **6** Grab, **7** Snake hook, **8** Trim (planar), **9** Mask (press again: box mask, then lasso mask). **Hold Shift** to smooth with any brush. Every brush is also on the shelf and in the **E** ring. Alternatives: Blender's letters (X draw, C clay strips, I inflate, Shift + C crease, Shift + T flatten, G grab, K snake hook, M mask); ZBrush's B then two letters. *Cost:* small before it's built: one table.
9. **Building up or digging in.** Chosen: **hold Ctrl while stroking to invert** the brush (dig instead of build; Inflate deflates; Mask unmasks), as in Blender and Nomad Sculpt. Note: in Paint mode, Ctrl + left drag sets the flow; sculpting uses it for inverting instead. Alternatives: Alt (ZBrush's; clashes with Alt + mouse navigation, which is on in every scheme); a button only. *Cost:* small.
10. **Brush size and strength.** Chosen: **[** and **]** for size, and **Ctrl + right drag**: sideways for size, up and down for strength, the same gesture as Paint mode's (where up and down is hardness). Alternatives: Blender's F and Shift + F, then move the mouse; ZBrush's Space menu (Space is command search here). *Cost:* small.
11. **Symmetry.** Chosen: **L** turns it on and off, as mirror painting does; **X** (left and right) as standard, across the object's own plane through its pivot, as mirror painting and the live mirror. Several axes at once, and **radial** symmetry (a count of copies round an axis, for flowers, wheels, crowns), in the Sculpt tab. Alternatives: X (ZBrush's key; X is Slit while modelling); buttons only. *Cost:* small.
12. **Masks.** Chosen: the **Mask** brush (**9**), **Ctrl + I** inverts the mask (as it inverts a selection), **Alt + M** clears it (Blender's key; free here), **Ctrl + wheel** grows and shrinks it (as it grows a selection), and Mask from cavity, from a sculpt group and from the edges in the Sculpt menu. With part of a shape masked, **W, R, S** and **T** move, turn and scale the rest with the gizmo, fading through partly masked points. Alternatives: hold Ctrl to paint a mask (ZBrush's; Ctrl is invert here); Ctrl + wheel for detail levels instead (decision 0048 gives Ctrl + wheel to a tool's count, and detail levels are a count of sorts). *Cost:* small.
13. **Changing detail level.** Chosen: **Page Up** and **Page Down** (and the Detail level arrows in the Sculpt tab). Alternatives: D and Shift + D (ZBrush's; D is smoothing in Paint mode, kept so here); Ctrl + wheel (taken by the mask, item 12). *Cost:* small.
14. **Remeshing while sculpting.** Chosen: **Ctrl + R** remeshes for sculpting (Blender's key; free here); the voxel size is in the brush bar, shown on the model as a small grid while you change it. It **bakes** the shape into a new even mesh, one undo step; it is not the Remesh modifier, which stays live and would undo your sculpting each time it ran again. Alternatives: R and drag for the size (R is rotate, a Halberd key); a button only. *Cost:* small.
15. **What are "sculpt groups"?** Chosen: our own plain name, **sculpt groups**, for parts of a sculpt's surface marked with a soft colour (other programs call them face sets or polygroups; the app doesn't copy names). **Shift + H** hides the group under the pointer, **Ctrl + H** shows only that group, **Alt + H** shows everything again: the same keys as hide and isolate elsewhere. Alternatives: other programs' names; separate keys. *Cost:* small (a name and three keys).
16. **Kinds of stroke.** Chosen: **Freehand** (standard), **Line** (press, drag, let go: a straight stroke), and **Drag a stamp** (press where it goes, drag to size and turn one dab: how alphas are stamped). Paint mode's Shift + click for a straight line doesn't come along, because Shift is smoothing here. Alternative: keep Shift + click for lines and put smoothing on a key. *Cost:* small.
17. **How sculpts look.** Chosen: Sculpt mode shows **matcaps** (a lit material picture wrapped on the shape, so forms read clearly): our own, drawn in code (Clay, Red wax, Pale stone, and the app's lit look), with **cavity** shading on and edges off. Alternatives: images from other programs (not allowed: copy behaviour, never assets); only the app's lit look. *Cost:* small.
18. **Alphas.** Chosen: alphas (greyscale pictures a brush stamps or drags) live on the shelf's **Alphas** tab: a set made in code (stones, bark, scales, stitches, rivets, cracks, scallops) and your own PNG pictures, shared with the painter's tips and the material graph's stamps. Alternative: a separate sculpting library. *Cost:* small.
19. **Stylised hard-surface helpers.** Chosen: **Trim** (flattens to the plane under the stroke's start, for chiselled rock, wood and stone), **Polish** (smooths the surface while keeping its edges crisp), **Line flatten** (a line across the view flattens everything on one side onto it, cutting nothing), and **Box cut** and **Lasso cut** (real cuts with the app's booleans, then a remesh). Alternative: the first three only, as nothing is cut. *Cost:* moderate; the cuts are their own part of milestone 17b.2 and could be left out.
20. **From a sculpt to a game model.** Chosen: **Make a low-poly** (a new `_low` object from the sculpt with Decimate's method and a quad tidy, the sculpt renamed `_high`, ready for the baker's name matching), **modelling on the sculpt** (Snap to surface: every modelling tool's points land on the sculpt's surface, the sculpt drawn see-through), and **Automatic quads** last (a quad remesher that lays even quads along the shape's flow), which can be dropped if it proves too hard. Alternatives: decimate only; leave retopology to other programs. *Cost:* Automatic quads is the riskiest milestone of the phase, built last on purpose.
21. **Baking the sculpt.** Chosen: the Bake window gains **From its sculpt**: an object bakes its own sculpted detail onto its shape without it (or a chosen detail level), with no second object or cage to line up, beside the `_low` and `_high` matching that already exists. Alternative: always two objects. *Cost:* small.
22. **Pen pressure.** Chosen: pressure changes **both size and strength**, with curves, as in Paint mode (Wacom through Wintab, other pens through Windows). Alternatives: strength only; size only. *Cost:* trivial (a setting).
23. **How fast, and on what?** Chosen: **brushes run on the processor, on all its cores**, in chunks of points; only the chunks a stroke touches are redrawn, re-indexed and uploaded. Targets on the 3-million-polygon heavy shape are under [Performance plan](#performance-plan); the heavy-mesh check gains sculpting steps. Alternatives: brushes on the graphics card (faster for huge brushes, much harder to keep exact, testable and undoable); a lower bar (1 million polygons). *Cost:* high to switch later.
24. **Colour while sculpting?** Chosen: **no**: the texture painter and vertex colour painting already exist, and sculpt groups give the bake's ID map. Alternative: paint colour on points while sculpting (ZBrush's polypaint, Nomad's painting). *Cost:* additive later (one milestone).
25. **Leaving Sculpt mode.** Chosen: as Paint mode: **Esc** leaves (or cancels a stroke in progress first), the palette button or another layout leaves, and **Tab** stays in the mode. Alternatives: Tab leaves, as in the modelling modes. *Cost:* small.

Added while building milestone 17a.1 ([decision 0304](../decisions/0304-sculpt-mode-and-layers.md)), also to review:

26. **Where a Sculpt layer's data lives.** Chosen: **in the layer's one edit** (`Operation::Sculpt`, a shared handle), so undo, saving, copying objects and the stack's cache work as for every edit. Alternative: a store beside the paint (`Scene::sculpts`, the edit naming a sculpt by id), as this plan first drew it: its own undo, saving and cache handling for nothing gained. *Cost:* moderate (the file format and the command).
27. **What a sculpt keeps to fit itself again.** Chosen: **where the shape's points were when sculpted** (about 12 bytes a point, shared by every stroke, packed in the file), so a primitive given more segments keeps its sculpting by position. Alternative: only the point ids (smaller; a renumbered shape loses its sculpting). *Cost:* moderate (the file format).
28. **Keys 2 to 9 before their brushes exist.** Chosen: **they do nothing in Sculpt mode** (a note says the brushes are coming), rather than switching to the Edges or Polygons mode as elsewhere. Alternative: leave them switching modes until their brushes arrive. *Cost:* trivial.
29. **A press off the sculpted shape.** Chosen: **nothing is sculpted, and a note says to pick another shape in the Objects panel** (the brush owns the left button, as in Paint mode). Alternative: a press on another shape selects it to sculpt (as Blender's Alt + click). *Cost:* small; the next milestone can add it.
30. **Matcaps drawn how.** Chosen: **worked out in the shader** from which way the surface faces on screen (Clay, Red wax, Pale stone, and Lit, the app's usual look), no pictures to load or keep. Alternative: small pictures made at start-up, sampled (needed only for matcaps from files, which aren't planned). *Cost:* small.

Added while building milestone 17a.2 ([decision 0306](../decisions/0306-sculpt-brushes-for-forms.md)), also to review:

31. **What Ctrl does to Grab, Elastic grab and Snake hook.** Chosen: **nothing**: a grab has no opposite, so holding Ctrl by habit changes nothing. Alternatives: Ctrl makes Grab move only along the surface's facing (in and out), or only along the surface (sideways). *Cost:* small.
32. **Picking a brush in the floating brush bar.** Chosen: **a drop-down list** (thirteen buttons would cover a third of the view); the Sculpt tab keeps a button per brush, and the shelf a card per brush. Alternative: buttons for the seven brushes with keys, the rest in a "More" list. *Cost:* trivial.
33. **The tool menu (E) in Sculpt mode.** Chosen: **every brush in its own place**, in the shelf's order: Draw at the top, the brushes that build up at the right, those that level at the bottom, the grabbing ones at the left, Smooth beside Draw; two places kept empty for the Mask and Trim brushes. Alternative: groups (Clay strips and Clay; Flatten, Scrape and Fill; the three grabs) so every brush still to come fits, at the cost of a second click. *Cost:* small (one table), but a place once learnt is muscle memory.
34. **How far Elastic grab reaches, and its rubber.** Chosen: **four times the brush's size**, fading to nothing over the outer half, with rubber that **keeps its volume** (the published method's Poisson's ratio of one half), worked out step by step as the mouse moves. Alternatives: the whole shape (the method's own reach: slow on millions of polygons); a springier rubber that lets the volume change. *Cost:* small (two numbers).
35. **The brushes' pictures.** Chosen: **drawings in code** of the surface seen from the side, dashed before and solid after, with an arrow where the brush pulls, the same on the shelf's cards (in the paint shelf's card and scrolling row) and in the tool menu. Alternative: small renders of a sculpted ball (pictures to make, keep and remake when a brush changes). *Cost:* small.

Added while building milestone 17a.3 ([decision 0308](../decisions/0308-sculpt-pens-strokes-alphas.md)), also to review:

36. **Where your own alphas are kept.** Chosen: **an alpha library of your own**, an `alphas` folder in the app's data folder (beside your scripts), the same in every project; the shelf's **Add…** card copies a picture there as a greyscale PNG, and right-click takes one out. Alternatives: in each project (a file format change, and an alpha added in one project is missing from the next); only while the app is open. *Cost:* small.
37. **Sharing alphas with the painter's tips and the graph's stamps.** Chosen: **not yet**: the painter's tips are drawn in code, not pictures, and a graph's stamps are colour pictures kept in one box, so a shared store would change the painter and the graph in a sculpting milestone. Alphas use the same image reader and limits, and sharing comes with the first painter tip made from a picture. Alternative: one shared picture store now. *Cost:* moderate whenever it is built (a store, its files and undo).
38. **How an alpha changes a dab.** Chosen: **it multiplies each point's move** by the alpha under it, laid flat on the plane the brush faces, the same for every brush (the grabbing ones take no alpha). Alternative: each brush using the alpha its own way. *Cost:* small.
39. **Which way an alpha's top points.** Chosen: **up the view**, turned by an **Angle**; **Follow the stroke** points it along the stroke instead; a stamp's top points along its drag. Alternatives: always along the stroke; a random turn per dab (a jitter setting). *Cost:* trivial.
40. **Pen pressure in a Line or a stamp.** Chosen: **the pressure at the press**, kept for the whole line or stamp (it is laid again at every move, and would otherwise change under you). Alternative: the pressure now. *Cost:* trivial.
41. **Strokes and smoothing for the grabbing brushes.** Chosen: **Grab, Elastic grab and Snake hook always stroke freehand, without smoothing or an alpha**: they hold the surface under the mouse. Alternative: Line and smoothing for Snake hook too (a straight horn). *Cost:* small.

## What the project lead asked for

- **Sculpting focused on stylised creators** (2026-10-29, decision 0216): the big, clean, readable forms of hand-painted games, chiselled rocks and planks, chunky characters and creatures, mushrooms, trees, props.
- **The sculpting milestones added** (2026-11-17), with every choice recorded for review.
- **Brushes that feel like the paint brushes:** pen pressure, falloff, the shelf, symmetry.

## What a stylised artist can do at the end

After **Phase 17a**:

- Block out a creature, a tree stump or a rock from the app's primitives (or any model), **remesh** them into one even surface, and sculpt it.
- Sculpt with a calm set of brushes: **Draw, Clay strips, Clay, Inflate, Crease, Pinch, Flatten, Scrape, Fill, Smooth, Grab, Elastic grab, Snake hook**, with pen pressure, falloff, alphas and three kinds of stroke.
- Sculpt one side and see both (**symmetry**, radial too), **mask** areas to protect them, and move, turn or scale the unmasked part with the gizmo.
- Undo every stroke, save and reopen, and keep the shape's base settings and edit layers under the sculpt.
- Stay smooth on a sculpt of 3 million polygons.

After **Phase 17b**:

- Add **detail levels** and move between them; a big change on a low level carries the fine detail along.
- Use the stylised hard-surface helpers: **Trim**, **Polish**, **Line flatten**, **Box cut** and **Lasso cut**.
- Split a sculpt into **sculpt groups** to hide, isolate, mask and polish by.
- Turn the sculpt into a game model: **Make a low-poly**, model on the sculpt with **Snap to surface**, or let **Automatic quads** lay out a quad mesh; unwrap it (the existing tools); **bake** the sculpt's detail onto it (normal, occlusion, curvature, ID from the sculpt groups); paint it in the app; export it to Godot.

## Research behind the choices

How stylised game artists sculpt, from ZBrush, Blender, Nomad Sculpt and 3D-Coat workflows and tutorials (sources at the end):

- **The usual order:** block out the silhouette with simple shapes; merge them into one surface with a voxel remesh (ZBrush's DynaMesh, Blender's voxel remesh, Nomad's voxel remesher); sculpt the big forms; then polish and add detail; then make a low-poly (retopology or decimation) and bake the sculpt onto it.
- **The brushes they use most:** clay build-up and clay strips for forms, move (grab) for the silhouette, smooth all the time (Shift in every program), inflate and crease or pinch for folds and sharp edges, flatten, trim and polish for the planar, chiselled look of stylised rocks, wood and armour, and the mask for separating parts.
- **The three ways to get polygons:** dynamic topology (polygons added under the brush: great for loose sketching, but uneven triangles that must be remeshed after); voxel remesh (an even surface, merging pieces, losing the detail levels); detail levels (subdivision levels, the classic way for detail and the cleanest for baking). Nomad's manual and Blender's community both describe dynamic topology as a block-out tool to follow with a remesh, and detail levels as the most capable way for finished detail.
- **What beginners trip over:** brushes too strong, too many brushes, not knowing when to remesh, losing symmetry, and a sculpt too heavy to bake. The plan answers with a small numbered set, plain "What does this do?" notes on every brush, a warning when the remesh size would make a shape too heavy, and the finish built in.

## Architecture

### Crates

- **`sculpt`** (new, core layer 1, no UI or GPU; depends on `mesh`, `glam` and `words`): the sculpt layer's data (point moves, the mask and sculpt groups, in chunks), the brushes as plain functions (a dab in, the changed chunks out), symmetry, detail levels, Remesh for sculpting (through `mesh::remesh`), the trims and cuts (through `booleans`), and the finish's helpers. Everything testable without a window.
- **`scene`** holds the sculpt layers (`Scene::sculpts`, keyed by a sculpt ID that a stack entry names) and the commands that change them, so undo, saving, the stack and scripting work as for everything else.
- **`fileio`** saves them (a file format bump in milestone 17a.1; chunks compressed with zstd, untouched chunks skipped; limits through `crate::limits`; a fuzz test for the reader).
- **`render`** draws sculpts: matcaps, cavity, the mask and sculpt groups as tints, and uploads only changed chunks.
- **`ui`** holds Sculpt mode, its layout, the brush bar (shared with Paint), the shelf's Brushes and Alphas tabs, the Sculpt tab in Attributes and the Sculpt menu.
- **`bake`** gains *From its sculpt* (milestone 17b.5).

The `layer-check` keeps the direction: `sculpt` knows nothing of the scene, the scene nothing of the GPU.

### The data model

An object's stack today is *base → layer 1 → layer 2 → …*, each layer a list of recorded edits ([decision 0006](../decisions/0006-base-plus-operation-stack.md)). A **Sculpt layer** is a layer with one entry, `Operation::Sculpt { sculpt }`, which holds the sculpt itself (choice 26; this plan first drew it beside the paint, as below, by id):

```text
Object
 ├─ Base
 └─ Stack: [Layer]
      Layer "Edits"          (ordinary edits)
      Layer "Sculpt 1"       → Operation::Sculpt { sculpt: 7 }
      Layer "Edits 2"        (edits made after sculpting)
Scene::sculpts: { 7 → Sculpt }

Sculpt
 ├─ made_for: the shape it was sculpted on (point count, point IDs, a few anchors)
 ├─ levels: [Level]          level 0 = the shape coming in; 1..=6 subdivided
 │    Level { moves: chunks of Vec3 (along the surface), 16 384 points each, Arc-shared }
 ├─ mask:   chunks of u8 per point of the finest level (0 free … 255 masked)
 ├─ groups: a u16 per polygon of the finest level (0 = none)
 └─ shown_level, settings (symmetry, remesh size …)
```

- **Evaluating a Sculpt layer:** take the shape coming in; add level 0's moves; subdivide once and add level 1's moves; and so on up to the level shown (or, for export and baking, the level chosen there). Each move is measured along the surface: out along the point's normal, and sideways along a direction fixed by its first edge. So a change below (a primitive's size, a lower layer, a lower detail level) carries the sculpted detail with it.
- **Sculpting a level** changes only that level's moves. Higher levels keep theirs, and since they are measured along the surface, they ride on the change.
- **When the shape coming in changes its points** (a lower edit adds polygons), the sculpt finds its place again by position, like edits do ([decision 0032](../decisions/0032-edits-find-their-points-again.md)): each new point takes the moves of the nearest place on the old surface, and the layer shows a warning badge ("The sculpt was fitted to a changed shape"). Nothing is thrown away; undo brings the old shape back exactly.
- **Remesh for sculpting** collapses the stack up to and including the sculpt into a new even base, and starts a fresh Sculpt layer on it: one undo step, the old base and stack kept in the step (shared, not copied).
- **Undo:** a stroke is one `scene::Command` holding the chunks it touched, before and after (`Arc`), exactly as a paint stroke holds its tiles. Memory is counted in `scene::memory`.
- **Saving:** the sculpt's chunks, compressed, in the project. A level whose moves are all zero (a fresh subdivision) is saved as nothing.
- **Scripting (Phase 14):** sculpting commands are named commands taking plain data, like every other, so a script could smooth a sculpt or remesh it. Single strokes are not replayable edits (item 4 above).

### While a stroke runs

1. The interface thread holds a **live sculpt**: the shape coming into the Sculpt layer (cached, as every layer's input is), the shown level's positions, normals and spatial index, all in chunks.
2. Each dab: find the points in reach through the spatial index (the BVH that picking uses, [decision 0035](../decisions/0035-heavy-meshes.md)), work out their moves in parallel across the processor's cores (`mesh::parallel`), and mark their chunks changed. Symmetry repeats the dab reflected (and turned, for radial).
3. Once per frame: recompute the normals of the changed chunks and one ring around them, refit the spatial index's boxes around them (no rebuild), and upload the changed chunks' vertices to the GPU.
4. On release: the stroke becomes a command (its chunks before and after). Nothing above the sculpt is re-evaluated until you leave Sculpt mode or stop for a moment (choice 7: what's above is baked in or hidden while sculpting).

### Drawing sculpts

- **Matcaps** (choice 17): a fragment shader looks the surface normal up in a small matcap picture generated in code when the app starts. Cavity shading darkens creases from the curvature.
- **The mask** shows as a darker grey over the matcap, **sculpt groups** as soft colours (a switch in the Sculpt tab).
- **Chunks:** the sculpt's vertex buffer is split into chunks of 16 384 points matching the sculpt's, so a stroke uploads only what it touched (`queue.write_buffer` per chunk). Edges are off as standard in Sculpt mode; when switched on, dense wireframes fade as they already do.
- The brush draws a circle on the surface, turned to face along it, with an inner ring for its falloff, and the reflected circle where symmetry sends the dab (as mirror painting's dashed circle).

## Performance plan

Sculpting is where heavy meshes stop being a safeguard and become ordinary use: a stylised sculpt is easily a million polygons, and sometimes three. The targets are measured on the heavy test shape (Help → Add a heavy test shape, 3 million polygons) on ordinary hardware, as the rest of [DESIGN.md §4.8](../../DESIGN.md#48-performance-and-robustness-w11):

| What | Game-sized sculpt (up to 500 000 polygons) | 3 million polygons |
|---|---|---|
| A stroke with a brush covering 20 000 points | 60 fps | 60 fps |
| A stroke with a brush covering 200 000 points | 60 fps | no visible stutter (30 fps or better) |
| Smooth, Grab and Snake hook | as above | as above |
| Undo or redo a stroke | instant | under 50 ms |
| Memory per stroke | the touched chunks, twice | the touched chunks, twice; never the whole mesh |
| Remesh for sculpting to 1 million polygons | under 2 s, in the background | under 5 s, in the background, with progress |
| Changing detail level | instant | under 1 s (the levels are kept) |
| Masking, hiding a group | instant | under 50 ms |
| Saving and opening | instant | a few seconds, never a freeze |
| Memory held by the sculpt | small | about 50 MB for moves, mask and groups, plus the mesh |

How:

- **Chunks of 16 384 points** everywhere (moves, mask, normals, GPU buffer, undo), so every cost scales with what a stroke touches, never with the whole shape.
- **All cores** for dabs and normals through `mesh::parallel` (the same threads as the heavy-mesh work; no new dependency).
- **Refit, don't rebuild** the spatial index while sculpting; rebuild in the background after a remesh or a level change.
- **Spacing:** dabs every tenth of the brush's size along the stroke (a setting), merged per frame, so a fast stroke doesn't pile up work.
- **The heavy-mesh check gains sculpting steps** in milestone 17a.6: a stroke with a small and a big brush, Smooth, Grab, undo, mask, remesh, save and open. They run at the end of each part (17a and 17b) and when the project lead asks ([decision 0033](../decisions/0033-heavy-mesh-safeguard.md)). Every milestone on a performance path adds its own small benchmark.

## Brushes

All brushes share the brush bar's settings, as the paint brushes do: **Size** (on screen, or in centimetres on the surface: Paint's *Size space*), **Strength**, **Falloff** (a curve, with presets: smooth, sharp, flat, spike), **Spacing**, the **alpha**, **pen** (pressure to size and strength, with curves) and **smoothing** (the lazy mouse, **D**). Each brush has a "What does this do?" note on the shelf, and keeps its own size and strength. **Ctrl** inverts while stroking; **Shift** smooths.

| Brush | Key | What it does | Milestone |
|---|---|---|---|
| **Draw** | 1 | Pushes the surface out (Ctrl: in) along its average facing | 17a.1 |
| **Smooth** | hold Shift | Evens the surface out: bumps go, the shape's flow stays | 17a.1 |
| **Clay strips** | 2 | Lays square-edged strips of clay on a plane under the brush, filling hollows first: the main form-builder | 17a.2 |
| **Clay** | shelf | The same, round and softer | 17a.2 |
| **Inflate** | 3 | Swells the surface out along each point's own facing (Ctrl: deflates) | 17a.2 |
| **Crease** | 4 | Cuts a sharp groove and pinches its sides in (Ctrl: a sharp ridge) | 17a.2 |
| **Pinch** | shelf | Pulls points towards the brush's middle, sharpening an edge | 17a.2 |
| **Flatten** | 5 | Flattens towards the average plane under the brush | 17a.2 |
| **Scrape** / **Fill** | shelf | Flatten only the bumps / only the hollows | 17a.2 |
| **Grab** | 6 | Moves a lump of the surface with the mouse, for the silhouette | 17a.2 |
| **Elastic grab** | shelf | Grab that bends the shape around the lump like rubber, for big pose and proportion changes (regularised Kelvinlets, a published method) | 17a.2 |
| **Snake hook** | 7 | Pulls out horns, tentacles, tufts and branches along the stroke | 17a.2 |
| **Mask** | 9 | Paints a mask; press 9 again for the box mask, then the lasso mask | 17a.4 |
| **Trim** | 8 | Flattens to the plane under where the stroke started, cutting chiselled facets | 17b.2 |
| **Polish** | shelf | Smooths and flattens while keeping edges crisp | 17b.2 |
| **Line flatten** | shelf | Draw a line across the view: everything on one side is flattened onto it | 17b.2 |
| **Box cut** / **Lasso cut** | shelf | Cut the shape away along a box or a lasso drawn on the view (a boolean, then a remesh) | 17b.2 |
| **Group** | shelf | Paints a new sculpt group | 17b.3 |
| **Relax** | shelf | Slides points along the surface to even out stretched polygons, without changing the shape | 17b.1 |

## Topology approach

(Choice 6.) **Remesh for sculpting** (17a.5) uses the app's own voxel remesher (surface nets on a distance grid, [decision 0142](../decisions/0142-modifier-stack-and-remesh.md)), made fast enough for millions of polygons with a sparse grid and all cores. Its settings: **Voxel size** (the brush bar; shown as a grid on the model while changed; a warning above 3 million polygons), **Keep detail** (the new surface is fitted to the old one's sculpted detail, on as standard), **Keep sculpt groups** and **Fix holes** (open shapes are closed first). Overlapping pieces become one surface: block out with primitives, select them, Ctrl + R.

**Detail levels** (17b.1) use the app's Subdivide (Catmull–Clark, with creases, [decision 0143](../decisions/0143-subdivide-and-creases.md)): **Add a level** (up to six; a warning when the next would pass 3 million polygons), **Page Up** and **Page Down** to move between levels, **Delete higher levels**, **Delete lower levels** (the finest becomes level 0), and **Rebuild lower levels** (works out lower levels for a shape that came from a remesh). Export and baking choose a level.

## Masking

- The mask is a value per point (0 free to 255 fully masked), blended through the brush's falloff, shown darker. Masked points don't move under any brush, including the gizmo.
- **Mask brush (9):** paint (Ctrl: unmask), **box** and **lasso** (9 again, as Q cycles the select tools). **Ctrl + I** invert, **Alt + M** clear, **Ctrl + wheel** grow and shrink (by one ring of points), and **Sharpen** and **Soften** mask in the Sculpt menu.
- **Mask from:** cavity (the creases), the edges (the boundary of an open shape), a sculpt group, the facing (up, down or towards the view).
- **Make a shape from the mask:** the masked part becomes a new object with a thickness (armour plates, belts, clothing from the body; the app's Shell). This is the stylised artist's "extract".
- With a mask, **W**, **R**, **S** and **T** show the gizmo at the unmasked part's middle; dragging moves, turns or scales it, partly masked points following partly.

## Symmetry

- **L** on and off; **X**, **Y** and **Z** axes in the Sculpt tab, several at once (X as standard), across the object's own planes through its pivot, as mirror painting ([decision 0187](../decisions/0187-mirror-painting.md)) and the live mirror.
- **Radial:** a count (2 to 64) of copies round a chosen axis.
- **Make symmetrical:** copies one side's sculpting onto the other (X+ to X−, and the other way), for a sculpt that drifted. Needs the shape to be symmetrical in position; it says so in plain words when it isn't.
- Each symmetric dab is part of the same stroke: one undo step.

## The finish: from a sculpt to a game model

1. **Make a low-poly** (17b.4): a new object `name_low` from the sculpt, by Decimate's method ([decision 0145](../decisions/0145-decimate.md)) to a triangle budget, then Quadify ([decision 0150](../decisions/0150-quadify.md)) to tidy it into quads where it can, keeping sculpt group borders. The sculpt is renamed `name_high`, so the baker's name matching ([decision 0251](../decisions/0251-high-to-low.md)) pairs them with nothing more to do.
2. **Model on the sculpt** (17b.4): **Snap to surface** in the snapping list. Every modelling tool's points (moves, extrudes, new polygons, loop cuts) land on the chosen target's surface, the target drawn see-through behind the shape you build. Shrinkwrap ([decision 0147](../decisions/0147-curve-deform-and-shrinkwrap.md)) keeps the result on the surface later.
3. **Automatic quads** (17b.6): a quad remesher that follows the shape's flow (field-aligned, after the published Instant Meshes method; our own code), to a target polygon count, guided by sculpt groups and creases. Built last, and dropped if it can't be made robust.
4. **Bake** (17b.5): the existing baker, plus **From its sculpt** (choice 21), and an **ID map from sculpt groups**. "Bake check" ([its page](../manual/bake-check.md)) warns when the low-poly misses parts of the sculpt.
5. **Unwrap and paint** with the existing tools; **export** with the existing pipeline.

## The manual's needs

New pages, each in the shape of `docs/manual/_template.md`, for someone rusty at 3D:

- **Sculpting** (17a.1): what sculpting is and when to sculpt rather than model; the Sculpt mode and layout; your first sculpt; Sculpt layers and undo.
- **Sculpt brushes** (17a.2, grown in 17a.3 and 17b.2): every brush with a picture, what it's for, and the keys; strokes, alphas and pens (17a.3).
- **Symmetry and masks while sculpting** (17a.4).
- **Remesh for sculpting** (17a.5), linked from the Remesh modifier's page explaining the difference.
- **Detail levels** (17b.1), **Stylised hard edges** (17b.2), **Sculpt groups** (17b.3).
- **From a sculpt to a game model** (17b.4 to 17b.6): low-poly, modelling on the sculpt, automatic quads, baking.
- **Concepts** gains: sculpting, brush strength and falloff, alpha, matcap, voxel remesh, detail levels, mask (sculpting), sculpt group, dynamic topology (and why the app doesn't use it), retopology, high-poly and low-poly.
- **Keyboard shortcuts:** a new colour for Sculpt mode's keys in the Keys window, and its rows.
- **Heavy shapes:** sculpting on millions of polygons and what to expect.

## Milestones

Each milestone is one pull request, with its own decision record, tests, manual pages, CHANGELOG lines and test steps for the project lead. Controls are the recommended choices above until reviewed. "Heavy numbers" are measured on the 3-million-polygon shape.

### Phase 17a: sculpting the forms

**17a.1 Sculpt mode, Sculpt layers, Draw and Smooth.**
- The `sculpt` crate; `Operation::Sculpt` and `Scene::sculpts`; the Sculpt layer in the Edit Layers panel; the file format bump with the sculpt's chunks; the live sculpt and chunked drawing; matcaps and cavity.
- Sculpt mode and the Sculpt layout (the brush bar on top, the shelf below, Objects and Attributes with a Sculpt tab on the right); the question about booleans, mirror and modifiers (choice 7).
- **Draw** and **Smooth**, with size, strength, falloff and spacing; undo per stroke.
- *Controls:* the Sculpt palette and layout buttons; **1** Draw; **hold Shift** smooth; **Ctrl** while stroking inverts; **[ ]** size; **Ctrl + right drag** size and strength; **Esc** leaves.
- *Tests:* a dab moves only points in reach, by the falloff (normal); a dab at the shape's edge and on a one-polygon shape (edge); a sculpt naming a missing sculpt ID gives a warning, not a crash (failure); save and open gives the same points exactly (round trip); fuzzed sculpt chunks in a project file (fuzz); a lower layer changing its points fits the sculpt again with a badge (regression of the stack's promise); undo and redo restore exact positions; a benchmark for a dab on 1 million points.
- *Heavy numbers:* a stroke with 20 000 and 200 000 points in reach; undo; save and open with a sculpt.

**17a.2 The brush set for forms.**
- **Clay strips, Clay, Inflate, Crease, Pinch, Flatten, Scrape, Fill, Grab, Elastic grab, Snake hook**; the shelf's **Brushes** tab with pictures and "What does this do?"; the **E** ring of brushes.
- *Controls:* **2** Clay strips, **3** Inflate, **4** Crease, **5** Flatten, **6** Grab, **7** Snake hook; the rest on the shelf and in **E**.
- *Tests:* per brush, its effect on a flat grid against a known result (normal), with Ctrl (inverted), at zero strength (nothing), with a brush bigger than the shape (edge); Grab and Snake hook follow the mouse in every view, including flat views (drags follow the pull, [decision 0039](../decisions/0039-drags-follow-the-pull.md)); Elastic grab keeps volume within a tolerance; a benchmark per brush.
- *Heavy numbers:* Grab, Snake hook and Clay strips with a big brush.

**17a.3 Pens, strokes and alphas.**
- Pen pressure to size and strength with curves (the painter's pen code); smoothing (the lazy mouse); **Freehand**, **Line** and **Drag a stamp**; the shelf's **Alphas** tab (the set made in code, your own PNGs, shared with the painter's tips and the graph's stamps); alpha angle (follow the stroke or fixed).
- *Controls:* **D** smoothing on and off; Drag a stamp: press, drag to size and turn.
- *Tests:* pressure curves map as drawn (normal, edge at 0 and 1); a broken or huge PNG alpha is refused with a plain message (failure, fuzz the reader); Line strokes are straight in object space; a stamp's result matches its alpha (normal).
- *Heavy numbers:* a stamp of a 1024-pixel alpha with 200 000 points in reach.

**17a.4 Symmetry and masks.**
- Symmetry: **L**, axes, radial, Make symmetrical, the reflected brush circle.
- Masks: the Mask brush, box and lasso, invert, clear, grow and shrink, sharpen and soften, mask from cavity, edges and facing; the gizmo moving the unmasked part; Make a shape from the mask.
- *Controls:* **L**; **9** Mask (again: box, lasso); **Ctrl + I**; **Alt + M**; **Ctrl + wheel**; **W, R, S, T** with a mask.
- *Tests:* a mirrored stroke on a symmetric shape gives a symmetric result (normal); a shape that isn't symmetric still mirrors in space, and Make symmetrical explains why it can't (failure); radial counts 2 and 64 (edge); a fully masked shape doesn't move under any brush or the gizmo (normal); grow then shrink returns the mask (round trip); undo of mask changes.
- *Heavy numbers:* a symmetric stroke (twice the dabs); invert and grow the mask; the gizmo moving the unmasked half.

**17a.5 Remesh for sculpting.**
- **Ctrl + R** and the Sculpt menu's Remesh; Voxel size with the grid preview and the heavy warning; Keep detail; Keep sculpt groups (ready for 17b.3); Fix holes; several selected shapes merged into one; in the background with progress; one undo step.
- The remesher made faster: a sparse grid, all cores.
- *Controls:* **Ctrl + R**; Voxel size in the brush bar.
- *Tests:* overlapping primitives become one closed surface (normal); a tiny voxel size is refused before it runs out of memory, with the polygon count it would make (failure); an open shape is closed first (edge); Keep detail keeps a sculpted bump within a voxel (normal); undo returns the old stack exactly (round trip); a benchmark for remeshing to 1 million polygons.
- *Heavy numbers:* remeshing a 3-million-polygon sculpt to 1 million, and to 3 million.

**17a.6 Speed: 3 million polygons stay smooth.**
- Measure every brush, mask and remesh against the targets; fix what misses them; the heavy-mesh check's sculpting steps; memory per stroke counted in the undo budget.
- *Tests:* the heavy-mesh check (on request and at the gate); benchmarks for dab, normals, refit and upload; a long session (1 000 strokes) stays within the undo memory budget.
- *Heavy numbers:* the whole table under Performance plan, in the pull request.

#### What milestone 1 built

([decision 0304](../decisions/0304-sculpt-mode-and-layers.md), pull request in the README's history.)

- **`sculpt` crate:** `Sculpt` (structure, places, `Moves` in chunks of 16 384 shared between copies; `apply`, `fitted_to` by position through a grid, `memory_apart`), `frames::frame` (area-weighted facing and a direction towards the first neighbour), `brush` (Draw, Smooth, four falloffs, `BrushSettings` per brush), `tree::FaceTree` (Morton-ordered boxes round polygons, `refit` round moved ones with marks, `ray_hit`, `near`), `LiveSculpt` (`dab` on every core above 16 384 points, `finish` setting the shape to exactly what evaluating gives, `cancel`, `follow` chunk by chunk). Levels, mask and groups aren't in the data yet: each later milestone adds its own to the file format.
- **Scene:** `Operation::Sculpt`, `Layer::sculpt`/`is_sculpt`, `Stack::sculpt_layer` and `next_sculpt_name` ("Sculpt", "Sculpt 2"), `recording_layer` skipping a Sculpt layer on top, `Command::Sculpt`, Merge down refused, `Scene::sculpt_target` and `prime_sculpt` (the cache takes the stroke's shape), undo memory counting only chunks one side has.
- **Files:** format 112 (`fileio::project::v112`), a fuzz test and the 112 sample.
- **Drawing:** `SurfaceDraw::Matcap` (shader mode 6), cavity on and edges off in Sculpt mode, the live shape shown during a stroke through `DrawingCache::show_live` (only moved points uploaded).
- **Interface:** `Mode::Sculpt` (palette between Polygons and UV), `LayoutChoice::Sculpt` and `layout::sculpt_layout` (saved between sessions), `Panel::Sculpt`, the shelf's brush cards, the floating brush bar (`sculpt_tab`), `workbench::sculpting` (the live sculpt, a heavy shape made ready on another thread, strokes with spacing and a time budget per frame, the question about booleans, mirror and modifiers), the keys list's Sculpt mode chip. The brush circle is still drawn on screen; size is on screen only; pressure arrives with 17a.3.
- **Heavy numbers:** `heavy_sculpt_check` in `tools/heavy/heavy-check.sh` (strokes with about 20 000 and 200 000 points in reach on the heavy test shape, undo, save and open with a sculpt). On two cores: ready to sculpt in about 1.1 s in the background; a 20 000-point stroke 5 ms a frame (8 ms at most); a 200 000-point stroke 12 ms a frame on average with one frame of 71 ms (its first move); Smooth 4 ms (24 ms at most); undo of a big stroke 98 ms, redo 49 ms; saving 2.9 s and opening 2.0 s; 81 MB of undo memory over three strokes. **Left for 17a.6:** that first frame and undo (target 50 ms; keeping the stroke's earlier places with its step would make undo instant).

#### What milestone 2 built

([decision 0306](../decisions/0306-sculpt-brushes-for-forms.md), pull request in the README's history.)

- **`sculpt` crate:** `BrushKind` has all thirteen brushes (`grabs` for Grab, Elastic grab and Snake hook), each with `BrushSettings::standard` (the grabbing ones at full strength, Crease with Sharp). `forms` holds what a dab does to a point: the plane under the brush (`Plane`, through the points in reach weighted by the brush, facing their area-weighted facing), `clay` (up to `CLAY_RATE` of the radius above the plane, only points below it, a quarter of the way a dab; Clay strips measured to a square turned with the stroke, its reach √2 wider), `inflate` (each point's own facing), `crease`, `pinch`, `flatten` (`Side` for Scrape and Fill; Ctrl swaps them), `grab`, `kelvinlet` and `elastic`. `LiveSculpt` keeps what a grabbing brush holds (`held`): Grab moves the points it took by the whole move since the press (exact, not added up), Elastic grab bends the points within four radii step by step from where they are (so the volume stays within 2% on a ball), Snake hook gathers afresh at the place it holds each dab. `place` sets only points whose place changes. No new file format.
- **Interface:** `SculptSettings::brushes` (one per brush), `sculpt_tab::brush_key` (2 Clay strips to 7 Snake hook), the brush bar's brush as a drop-down, the shelf's cards (`shelf_cards::card` and `row`, now shared) with `Icon::SculptBrush` pictures (`icons/sculpting.rs`), `CommandId::SculptBrush` and `commands::SCULPT_RING` (the E ring in Sculpt mode, command search under Sculpt; outside Sculpt mode they say where they work). `workbench::sculpting`: a grabbing brush's stroke takes hold where it starts, then `sculpt_grab_to` puts what it holds under the mouse on a plane through it facing the view (Grab once per mouse move; Elastic grab and Snake hook every spacing, within the frame's time, always ending where the mouse is). Keys 8 and 9 still kept.
- **Tests:** per brush on a flat grid against known results, with Ctrl, at no strength and bigger than the shape (and evaluating exactly to the live shape); hollows filled first; a ball inflating outwards; Grab following the mouse in a perspective and a flat view; Snake hook pulling past its reach; Elastic grab's volume and a Kelvinlet's divergence; every brush one undo step on the sphere; the tool menu, keys and shelf. `brush_benchmark` in the `sculpt` crate; the heavy sculpting check gains Clay strips, Grab and Snake hook with the big brush.
- **Heavy numbers** (the heavy sculpting check, 3 million polygons, two cores, measured while another build used the same two cores, so every number, Draw's too, is about two to three times milestone 1's): with about 200 000 points in reach, Clay strips 40 ms a frame on average (216 ms at most), Grab 47 ms (88 ms; taking hold 22 ms), Snake hook 29 ms (141 ms), beside Draw's 33 ms and Smooth's 30 ms on the same run; finishing each stroke 100 to 250 ms. Grab and Clay strips miss the 30-fps target on a busy machine; making them, and every brush, meet it is 17a.6's work (moving the points and refitting the tree are still on one core).
- **Left for later:** Elastic grab works on every point within four brush sizes each frame (about 180 ms a dab with a million points in reach on two cores): the slowest brush, for 17a.6. Pen pressure, alphas and the brush circle on the surface come with 17a.3 and 17a.4.

#### What milestone 3 built

([decision 0308](../decisions/0308-sculpt-pens-strokes-alphas.md), pull request in the README's history.)

- **`sculpt` crate:** `alpha` (`Alpha`: one byte a pixel, at most 1024 across, `at` blending between pixels with `y` up, centred in a square, black outside; `Made`: Stones, Bark, Scales, Stitches, Rivets, Cracks, Scallops, 256 pixels made once by `made`; `DabAlpha`: the picture, its up, an angle, following the stroke). `LiveSculpt::dab_with` lays the alpha on the plane the brush's polygons face and multiplies each point's move by it; the grabbing brushes take none. No new file format.
- **`fileio::alphas`:** a picture read as an alpha (grey from how light times how solid, made at most 1024, packed as a greyscale PNG and read back from it); the alpha library (a folder of greyscale PNGs: `library`, `keep`, `forget`).
- **Interface:** `SculptSettings` gains `stroke` (`StrokeKind`), the alpha and its angle and following, your own alphas, `size_pressure` and `strength_pressure` (`pressed`), smoothing (`smoothing_reach`); the brush bar's Stroke and Alpha lists; the Sculpt tab's Stroke and alpha, and Pen and smoothing; the shelf's Brushes and Alphas tabs. `workbench::sculpting`: pressure from the tablet or the system's touch (Line and stamps keep the press's), the lazy mouse for freehand strokes of the non-grabbing brushes, `lay_line` and `lay_stamp` (taken back and laid again at each move), `bring_in_alpha` and `forget_alpha`, **D**. `AppInfo::alpha_library` (`alphas` in the app's data folder), `WorkbenchAction::ChooseAlpha` and the app's file window.
- **Tests:** a stamp matching its alpha exactly, black parts staying still, the alpha turned by its up, angle and the stroke's way, grabbing brushes taking none, a stroke with an alpha evaluating exactly; reading pictures as alphas (grey, see-through, made smaller, refusals, a fuzz run) and the library's round trip; in the window: a Line going straight past a detour, Esc taking a line back, a stamp sized and turned by its drag, an alpha's black half staying still, a lighter press sculpting less, D, smoothing trailing, the Alphas tab, your own alphas kept and taken out, Grab ignoring the stroke kind; the pressure curves at 0, ½ and 1. `alpha_benchmark` in the `sculpt` crate; the heavy sculpting check gains a stamp of a 1024-pixel alpha.
- **Heavy numbers:** HEAVY_NUMBERS
- **Left for later:** the brush circle still drawn on screen (not on the surface), and under the mouse while a stamp is dragged; alphas shared with the painter's tips and the graph's stamps (choice 37); a jitter for the alpha's turn.

**Gate 17a:** the project lead blocks out a stylised creature or tree stump from primitives, remeshes it into one surface, sculpts its forms with symmetry and masks, undoes freely, saves and reopens it, and sculpting the heavy test shape feels smooth.

### Phase 17b: detail and the game-asset finish

**17b.1 Detail levels.**
- Add a level, Page Up and Page Down, Delete higher and lower levels, Rebuild lower levels; export and bake choose a level; the **Relax** brush.
- *Controls:* **Page Up**, **Page Down**; the Detail level arrows in the Sculpt tab.
- *Tests:* sculpting on level 1 moves the detail on level 3 with it (normal); adding a level past the polygon warning asks first (edge); Delete lower levels keeps the shape within a tolerance (round trip); save and open with four levels (round trip); a broken level count in a file is refused (fuzz).
- *Heavy numbers:* changing levels with 3 million polygons on the top level; a stroke on level 2 under it.

**17b.2 Stylised hard edges.**
- **Trim**, **Polish**, **Line flatten**, **Box cut** and **Lasso cut** (the cuts through the app's booleans, then Remesh for sculpting).
- *Controls:* **8** Trim; the rest on the shelf and in **E**; a cut is drawn with a left drag, Esc cancels.
- *Tests:* Trim makes a flat facet on a sphere (normal); Polish keeps a cube's edges within a tolerance while smoothing its faces (normal); Line flatten on a line that misses the shape does nothing (edge); a cut that would leave nothing is refused (failure); a cut is one undo step (round trip).
- *Heavy numbers:* Trim and Polish with a big brush; a Box cut on 3 million polygons (in the background).

**17b.3 Sculpt groups.**
- The Group brush; groups from the mask, from pieces (after a remesh), from materials, by angle; hide, isolate, show; mask from a group; Polish and Smooth keep group borders (a switch); groups kept through remesh and detail levels.
- *Controls:* **Shift + H**, **Ctrl + H**, **Alt + H** on the group under the pointer; the rest in the Sculpt menu.
- *Tests:* groups by angle on a cube give six (normal); hiding every group leaves nothing to sculpt and a note (edge); groups survive a remesh (regression); save and open (round trip); a group number past the limit in a file is refused (fuzz).
- *Heavy numbers:* hide and isolate a group; groups by angle.

**17b.4 Low-poly and modelling on the sculpt.**
- **Make a low-poly** (to a triangle budget, the `_low` and `_high` names), **Snap to surface** for every modelling tool, the target drawn see-through.
- *Controls:* none new beyond the snapping list's entry and the menu command (Space finds them).
- *Tests:* the low-poly stays within a distance of the sculpt and within the budget (normal); a budget of 12 triangles still makes a closed shape (edge); snapping onto a shape with nothing under the mouse leaves the point where it was (edge); undo (round trip).
- *Heavy numbers:* making a low-poly of a 3-million-polygon sculpt (in the background, with progress); snapping while dragging.

**17b.5 Baking the sculpt.**
- **From its sculpt** in the Bake window (a detail level as the high, the shape under the sculpt or a chosen level as the low); the ID map from sculpt groups; bake checks for sculpts.
- *Tests:* a bake from its sculpt matches a two-object bake of the same shapes within a tolerance (normal); a sculpt with no detail bakes a flat normal map (edge); a low-poly with no UVs is explained before baking (failure); the settings saved with the project (round trip).
- *Heavy numbers:* baking a 3-million-polygon sculpt to a 2K normal map.

**17b.6 Automatic quads.**
- A quad remesher to a target count, following the shape's flow, guided by sculpt groups and creases, in the background; the result a new object, the sculpt untouched. Dropped from the phase (and the choice recorded) if it can't be made robust.
- *Tests:* a sphere and a cube give quads only, near the target count (normal); a shape with holes and thin parts gives a result or a plain refusal, never a crash (failure, fuzzed shapes); a benchmark at 1 million polygons in.
- *Heavy numbers:* remeshing a 3-million-polygon sculpt to 5 000 quads.

**Gate 17b:** the project lead sculpts a stylised prop (a treasure chest, a rock with chiselled facets, a mushroom) with detail levels and crisp planar edges, makes its low-poly, bakes the sculpt onto it, paints it in the app, and sees it right in Godot.

## Sources

- [Stylized character art: sculpting tips for beginners](https://80.lv/articles/stylized-character-art-sculpting-tips-for-beginners) (80.lv): blockout, DynaMesh, ClayBuildup, hPolish and Move, then baking.
- [Nomad Sculpt manual: Topology](https://nomadsculpt.com/manual/topology.html): detail levels, the voxel remesher, dynamic topology, decimation and quad remeshing, and how each wipes the others' levels.
- [SubD, Multires, Voxel Remesh and Dyntopo: which to use?](https://blenderartists.org/t/subd-multires-voxel-remesh-and-dyntopo-which-to-use/1225303) (Blender Artists): dynamic topology for blocking out, then remeshing; detail levels as the classic way.
- [ZBrush tip: a clean, controllable ClayBuildup workflow](https://novedge.com/blogs/design-news/zbrush-tip-clean-controllable-claybuildup-workflow) and [Creating stylised characters with ZBrush](https://80.lv/articles/creating-stylized-characters-with-zbrush-marmoset-toolbag).
- [Blender manual: sculpting tools](https://docs.blender.org/manual/en/2.82/sculpt_paint/sculpting/tools.html) and [Blender sculpting shortcuts](https://www.versluis.com/2024/03/blender-scultping-shortcuts/), for the keys other programs use.
- Published methods the brushes and remesher follow (our own code, no copied code): de Goes and James, *Regularized Kelvinlets* (2017), for Elastic grab; Jakob, Tarini, Panozzo and Sorkine-Hornung, *Instant Field-Aligned Meshes* (2015), for Automatic quads.
