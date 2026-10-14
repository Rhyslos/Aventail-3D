# Whole-app review, 13 October 2026

At the project lead's request, the whole app was reviewed before more tools are built. The review covered:
- how it is built, and whether that holds up against the future plans (edit list, modifiers, splines, instances, materials, UVs, texturing, engine export, scripting, animation);
- stability (crashes, freezes, lost work);
- security (hostile files, the file system, dependencies, CI);
- speed.

Four independent reviews ran in parallel, each reading the code and testing their claims with throwaway programs outside the repository. The key claims were then checked again by hand. The code reviewed is `main` plus the Bevel branch (PR #31).

**In one paragraph:** the foundations are good. Edits are pure and repeatable, data is shared cheaply between copies, every command can be undone, files are checked on reading and saved safely, the crates are kept in their layers, and CI is well hardened. But there are three real problems:
1. **Two ways to lose a whole project.** A session can save a file that the app then refuses to open again.
2. **Every edit replays the object's whole history, every frame.** It gets slow quickly, and on heavy meshes it freezes. This blocks the planned edit list.
3. **The mesh has no place for anything but positions, faces and hard edges.** UVs, materials, seams and vertex colours will need it, and every tool written before it exists will have to be rewritten.

All three are cheapest to fix now, before more tools are built.

---

## 1. Lost work (fix first)

| # | Finding | Severity | Status |
|---|---|---|---|
| L1 | **Bevel can record a "hard edge" from a point to itself** (two polygons back to back). After Make editable, the project saves fine but can't be opened ("a hard edge joins points 3 and 3"). The whole project is lost, and so is every backup copy written after that. `Mesh::set_hard` accepts `a == b`; the loader rejects it. | Critical | Reproduced |
| L2 | **A NaN (not-a-number) can be recorded in an edit and saved.** Type 0 in a Size field of the Coordinates panel (the object's scale becomes 0), then move a point with the gizmo: the move is recorded as NaN. Saving works, but opening fails. `Command::Record` doesn't check that an edit's numbers are real. | Critical | Traced in code; scene, save and open chain reproduced |
| L3 | **Saving never checks that the file will open again.** Any gap between what edits can produce and what the loader accepts becomes silent loss of the whole file. This is the root cause behind L1 and L2. | High | Confirmed |
| L4 | **Saving silently leaves out edits the file writer doesn't know.** A new kind of edit that isn't added to the file format just disappears from saves and backups, and nothing catches it. | Medium | Confirmed |
| L5 | **Loading is all-or-nothing.** One bad value anywhere means the whole project won't open, even when it's only shading (a hard edge). | Medium | Confirmed |

**Fixes:**
- `set_hard` ignores a point joined to itself, and points that don't exist.
- `Record` and `ReplaceOperation` refuse numbers that aren't real.
- The gizmo skips objects that can't be inverted (scale 0).
- Every save (and backup copy) is read back with the real loader before it replaces the old file. If it fails, the old file is kept, the new bytes go to `name.avtl.unreadable`, and the user is told.
- Writing the file format must cover every kind of edit (an exhaustive match), with a test that saves and reopens one of each.
- Loading becomes lenient for harmless problems: a bad hard edge is dropped with a note instead of refusing the project.

## 2. Speed (fix before the edit list and modifiers)

All of these were measured on a 2-core virtual machine; a desktop is roughly 2–3× faster.

| # | Finding | Measured |
|---|---|---|
| P1 | **Every drag frame replays the object's whole edit history from the base.** All edits live in one layer, and the cache works per layer. This applies to gizmo moves as well as extrude, inset and bevel. | 3k-polygon prop: 25–37 ms per frame after 200 edits. 20k: about 280 ms. 3M heavy shape: about 0.75 s extra per frame for each extrude in the history. |
| P2 | **One extrude in the history switches off the heavy-mesh speed-ups.** After it, a point move no longer shares structure with the frame before, so topology, GPU upload and picking are all rebuilt. | 3M: a move frame went from about 0 ms to about 0.8 s or more. |
| P3 | **Bevel slows down with the square of the selection.** Two loops compare every strip with every other. | Bevel all edges of a 32k-polygon sphere: 65 s. All points: 311 s. Extrude on the same mesh: 36 ms. |
| P4 | **Finding edits again after a settings change scans the whole mesh for every element**, and its tolerance is too loose on dense meshes. | 3M: about 21 ms per point, so 1,000 points take about 21 s. On dense meshes, one anchored polygon found 12 points. |
| P5 | **Undo memory is counted wrongly in both directions.** Shared meshes are counted several times, while the topology kept with each mesh (about 4× the mesh) isn't counted at all. | 3M, Make editable: the history reports 432 MB but holds about 1.2 GB or more. |
| P6 | **Waits on the interface thread when a drag ends** (picking index, topology): up to about 0.8 s on 3M. | Measured |

**Fixes:**
- **Cache the shape after each edit, not only after each layer**, under a memory budget. A drag then only re-applies the edit being dragged, and the heavy-mesh speed-ups come back. This is the biggest single win, and the Phase 6 edit list needs it.
- Fix the two bevel loops (a map instead of a scan).
- A spatial grid for finding edits again, with a tolerance based on the element's own size.
- Count shared memory once, and count topology.
- Make drag-end work non-blocking.

Later: chunk the face list like the points, and update topology in part, so structural edits on heavy meshes cost about as much as what they touch.

## 3. How it's built, against the future plans

| # | Finding | Hurts | Severity |
|---|---|---|---|
| A1 | **The mesh has no attribute layers** (per point, edge, polygon, polygon corner). It only knows positions, faces and hard edges, and every tool carries hard edges over by hand (Bevel has about 15 such lines). | Material slots (Phase 5), UVs and seams (Phase 8), vertex colours, creases, skin weights. Every tool written before this exists must be rewritten. | High |
| A2 | **Changing an earlier edit can make later edits lose their points.** The "find it again by position" system can't tell "the points were renumbered" from "the points were moved by an edit below". In a test, extruding twice, moving the middle ring and then changing the first extrude lost the move completely, although the right points still existed. | The edit list (Phase 6), UVs that survive modelling (Phase 8) | High |
| A3 | **New element numbers depend on everything before them.** Change a bevel's segments and every element made after it is renumbered, so later edits fall back on the weak "find by position" path. This is the classic "naming problem" of history-based modellers. | Same as A2 | High |
| A4 | **Edits have no identity or switches.** They are addressed by their position in the list, and have no on/off, pin or freeze flags. | The edit list, scripting, the per-edit cache | High (cheap now) |
| A5 | **There is no way to delete a polygon** in the mesh structure. | Delete, Dissolve, Weld, Bridge, booleans, split into an object (Phase 4) | Medium |
| A6 | **The file format won't scale.** One block, no sections, edit types nested one version inside the next (V4 inside V3 inside V2…), a 4 GB cap, everything in memory at once. | Materials, many more edits, textures and paint layers (hundreds of MB) | Medium (before Phase 5) |
| A7 | **An object's shape can't depend on another object, and can only be a mesh.** | Instances, booleans, sweep along a spline, splines themselves, high/low bake pairs | Medium (before Phase 7) |
| A8 | **Shapes are worked out on the interface thread, under one lock for every object.** | Subdivision, booleans, long histories, baking: freezes instead of "slower" | Medium (before Phase 7) |
| A9 | Undo isn't designed for painting (it would need small tile changes); texturing will need its own crate. | Texturing | Low (design later) |

**Holding up well:**
- Edits are pure and repeatable, and report what they couldn't do instead of failing.
- Copies share their parts, so a per-edit cache is cheap to add.
- Edit and object types are open for extension, and evaluation is ready for time (animation).
- Commands are plain data made of small reversible steps, a good base for scripting.
- The file model is separate from the app, and saves are safe with a `.bak` copy.
- Layering is enforced, and background jobs never touch the scene.

**Fixes, in short:**
- **A1, A5:** typed attribute layers and a small "mesh builder" that tools use to make elements with a declared origin ("copy this corner", "between these two corners", "like that polygon"). Hard edges move onto it first, along with polygon deletion.
- **A4:** edits become entries with an id and on/off and pin flags.
- **A2, A3:**
  - When the shape's structure upstream is unchanged, element numbers are trusted.
  - Elements carry a stable "where I came from" name (which edit made me, and from what).
  - Searching by position is the last resort, done with a grid.
- **A6:** a sectioned file with a table of contents, a flat list of edit types, and textures as their own chunks.
- **A7, A8:** evaluation that can see other objects, a geometry type that can be a curve, instance objects, and evaluation on a worker thread with the last good shape shown meanwhile.

## 4. Security

No memory-safety issue and no way to run code was found. The single `unsafe` block (the Windows keyboard fix) is sound. `cargo deny` passes: 450 crates, all licences allowed, no known vulnerabilities. CI is well hardened: actions are pinned to exact versions, permissions are minimal, and no workflow runs untrusted pull requests with secrets.

The problems are with **hostile project files** (someone sends you a booby-trapped `.avtl`):

| # | Finding | Severity | Status |
|---|---|---|---|
| S1 | **A tiny file can make the app run out of memory and crash.** A 4.6 KB file asked for 4 GB and aborted the app. Nothing limits how many objects, points, polygons or edits a file may hold. | Medium | Reproduced |
| S2 | **A file can freeze opening for a very long time.** One huge polygon is checked in a way that grows with the square of its size. And replaying recorded edits has no budget: an 83-byte file took 11 s, and a few KB could take hours. | Medium | Reproduced |
| S3 | **Saving follows symbolic links planted beside the project.** A project folder from a zip or git could make Save overwrite another of your files with project data. | Medium | Confirmed in code |
| S4 | An object number of the maximum value makes the counter wrap around; a start-up race between two copies of the app; backup folder permissions; very long backup numbers; file names starting with `-`. | Low | Mixed |
| S5 | After a crash caused by S1 or S2, recovery offers the same file again, and restoring it crashes again. | Low | Suspected |

**Fixes:**
- **One "limits" rulebook** for every file the app reads (counts, sizes, estimated work), checked while reading, with a plain message when a limit is hit.
- A lower body limit.
- Evaluation off the interface thread, with a way to cancel.
- **One safe-write helper:** a uniquely named temporary file that never follows links, then a rename. Every save, backup, export and settings write uses it.
- Fuzz tests that open random files under time and memory limits.

**Rules to adopt now for later features:**
- **glTF import** only reads files next to the model (or embedded), never absolute paths, never the internet. Image sizes are limited.
- **Engine export:**
  - only writes inside the folder the user picked, with cleaned-up file names, and only over files it made itself;
  - the folder is stored in local settings, never in the shared project;
  - it never re-exports when you merely *open* someone else's project.
- **Lua scripting:**
  - sandboxed, with no file or OS access unless the user grants it per script;
  - with memory and time limits;
  - **never runs scripts that come inside a project file automatically.**
- **Privacy:** project files contain no paths, usernames or timestamps today, and the app has no telemetry and no network access. Keep it that way. Crash reports, if they ever come, are opt-in.
- **Before a public release:** a separate signed release build (Windows and macOS signing), published checksums, and a 7-day Dependabot delay on brand-new releases.

## 5. Other stability findings

- **The graphics card's buffer limit** (256 MB) isn't checked. A huge flat-shaded mesh (about 2.8M quads or more) could crash the renderer. Graphics device loss isn't handled either. *Suspected.*
- **Cancelling a drag wipes the redo list**, because the first frame already records a step. *Confirmed in code.*
- After a save on Linux the folder isn't synced, so a power cut right after a save could keep the previous file. That file is still intact. *Low.*

---

## Suggested plan

A **Foundations** step before the next tools (Loop cut, Slit), in this order:

1. **Never lose a project** (L1–L5, S3): small and urgent.
2. **Fast edits** (P1, P3, P4, P5, P6): the per-edit cache and the bevel and find-again fixes.
3. **Room to grow** (A1, A4, A5, then A2 and A3): attribute layers, a mesh builder, polygon deletion, edit entries with ids and switches, and stable element naming.
4. **Hostile files** (S1, S2, S4, S5): one limits rulebook and fuzz tests.

The file format sections (A6) come before Phase 5. Evaluation that sees other objects and runs off the interface thread (A7, A8) comes before Phase 7.

## Progress

- **1. Never lose a project:** done ([decision 0041](../decisions/0041-saves-that-always-reopen.md)).
- **2. Fast edits:** done ([decision 0042](../decisions/0042-fast-edits.md)). Measured before and after, release build: a drag after 200 edits on a 3k-polygon prop 25.9 ms → 0.07 ms per frame; dragging an extrude on a 3M-polygon shape with five extrudes in its history 3.5 s → 35 ms per frame; bevelling every edge of a 32k-polygon sphere 65 s → 1.9 s and every point 298 s → 0.1 s; finding 1,000 points again on 3M polygons 19 s → 0.15 s. P5 counts the topology; shared meshes are still counted once per step that holds them (an over-count, the safe direction).
- **3. Room to grow:** first part done ([decision 0043](../decisions/0043-attribute-layers-and-the-builder.md): A1 attribute layers and the builder, A5 deleting polygons). Second part done ([decision 0044](../decisions/0044-edits-that-hold.md)): A4 edit ids and switches and A2 ids trusted while the structure is unchanged, with attribute layers saved in file format version 5. Stretching new rows along (asked for by the project lead) was tried and dropped; see the DESIGN.md decision log. A3 (stable element names) is left for a smaller step before the Phase 6 edit list.
- **4. Hostile files:** done ([decision 0045](../decisions/0045-limits-for-files.md)): S1 and S2 (limits while reading, a memory budget, a size guard for shapes), S4 (private backup folders, the start-up race, file names starting with `-`; the object id overflow was fixed in step 1) and S5 (recovery can't loop). Evaluation on another thread (A8) stays planned for before Phase 7.
