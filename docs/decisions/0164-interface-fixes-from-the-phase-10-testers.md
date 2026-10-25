# 0164. Interface fixes from the Phase 10 testers

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-25
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (where a new modifier goes), [0148](0148-weighted-normals.md) (Weighted normals last), [0144](0144-lattice-cage.md) (lattice cages), [0155](0155-levels-of-detail.md) (levels of detail in exports), [0156](0156-game-bevel.md) (the game bevel), [0160](0160-kit-helpers.md) (Size to kit grid), [0163](0163-saving-and-exporting-in-the-background.md) (saving in the background)
- **Replaces part of:** [0142](0142-modifier-stack-and-remesh.md) and its followers, where most kinds went at the end of the list; [0161](0161-triangulate.md), where "nothing to cut" was an amber reason

## Problem

Two testers tried Phase 10 by hand. Most of what they found was small and is fixed without a choice to make (see the changelog), but five fixes change how something behaves:

1. On a Linux desktop without the file portal or zenity, **Save As, Open, Export…, Choose folder and the quit window's Save did nothing**, without a word. The file window library (rfd) answers only "a path" or "nothing", the same for a closed window and one that never opened.
2. A **Lattice or Scatter added after a game bevel went after its Weighted normals**, which then dropped the normals the game bevel is for.
3. **"Nothing to weld" read as a problem**, in amber, like "the shape isn't closed".
4. The Export window's **Levels of detail** switch started off even when the objects had levels.
5. **Size to kit grid** grew a shape out of its lattice cage, so a cutter could end up outside the bent shape.

## Decision

1. **A file window that gives nothing back within half a second never opened** (`app::project::asked`, `NO_WINDOW_WITHIN`): nobody sees a window and closes it that fast, while one that can't open comes back at once. Then the status bar says the system's file window didn't open (on Linux it needs the desktop's file portal or zenity), and a small window asks for the path instead (`ui::Workbench::file_window_missing`, `FilePurpose`): a text box starting with the suggested path, `~` for the home folder, a line saying why the path can't be used yet (no such folder, no such file, a folder where a file was wanted) or that a file will be replaced, and the button (Save, Open, Import, Export, Choose folder). Enter uses it, Esc or Cancel calls it off (and with it whatever waited for a save, such as closing). A save typed in gets the project's extension when it's left out, an export its format's.
2. **Every kind of modifier that changes the shape goes before Weighted normals** when added (`ModifierKind::goes_before`), as Displace already did; all but Weighted normals and Triangulate. Add lattice places its modifier the same way. The person can still move any modifier anywhere.
3. **A modifier with nothing to do says so in grey**, as "Nothing to do: …" (`scene::has_nothing_to_do`): Weld by distance with nothing close enough (or no open borders), Quadify with no triangles it may join, Triangulate with nothing to cut. Real problems stay amber ("Not worked out: …").
4. **Levels of detail starts on when anything going out has levels made, and off otherwise, until the person sets it**; from then on their choice is kept with the other remembered export settings (`ExportChoice::lod_chosen`). The window also says under its numbers how many objects take collision out and of which kinds, or that only Made for Godot writes it.
5. **Size to kit grid fits an untouched cage round the sized shape again**, in the same undo step (`Command::FitCage`, `Scene::cage_fits`). Untouched: its points at rest, not moved or put in a group, its size and place those a new cage would have, and used by no other shape. A cage the person changed is left as it is.

## Choices to check

Made for the project lead (decision 0028):

- **Half a second** tells "closed" from "couldn't open"; a window closed faster than that also gets the path box, which can just be cancelled.
- **The typed-path window** as the fallback (recommended): rather than only a note.
- **Shape-changing modifiers before Weighted normals**, not at the end.
- **Grey "Nothing to do"** for Weld, Quadify and Triangulate; amber for the rest.
- **Levels of detail follows the scene until set by hand,** then remembered.
- **Only untouched cages** are refitted by Size to kit grid.
- No new keys or mouse actions (Enter and Esc in the path window work as in the app's other windows).

## Consequences

- The manual's Saving and opening, Export, Modifiers, Weighted normals, Weld by distance, Quadify, Triangulate, Lattice and Modular kits pages say so.
- Tests: `ui/src/workbench/review_tests.rs` (the path window, the quit window's save through it, levels and collision in the Export window, calm notes), `ui/src/workbench/path_entry.rs` (paths checked), `app/src/project.rs` (no window told from closed), `scene/src/game_bevel_tests.rs` (the order after a game bevel), `scene/src/modifier/nothing_to_do.rs`, `scene/src/lattice_tests.rs` and `ui/src/workbench/kit_tests.rs` (cages fitted again).
