# 0268. Copy and paste between tabs

- **Status:** Accepted
- **Date:** 2026-11-08

## Problem

With scene tabs ([decision 0266](0266-scene-tabs.md)) several projects are open at once, and the project lead asked (step 4 of [docs/plans/scene-tabs.md](../plans/scene-tabs.md)) to copy objects in one tab and paste them in another, bringing their materials, paint, graphs and own boxes along, with ids remapped so nothing clashes, undoable in the receiving tab.

Until now copying ([decision 0055](0055-basics.md)) remembered only *which* objects were copied: pasting duplicated them, as they were then, just after their originals. That can't work in another project, whose ids mean other objects, and whose materials, texture sets, graphs and own boxes are its own:

- **Materials** belong together by name (decision 0074), and a material's texture set belongs to its name (decision 0177). The other project may use the same name for something else.
- **Graphs** have numbers (`GraphId`) given out by each project; graph fill layers name them.
- **Own boxes** have ids (`own:` and hex) made from what was being made; two projects can have the same id with different insides (one changed since, or made apart by chance).
- **Objects** name other objects: an instance its original, booleans their cutters, modifiers their cages, curves and shapes, sweeps their cross-sections, levels of detail made by hand their shapes.

## Options

1. **Paste by reading the other tab's project at paste time.** Breaks once that tab closes, and a sleeping tab would have to wake; and it pastes whatever the objects have become, not what was copied.
2. **A copy kept at copy time, in memory, shared by every tab.** Pasting works from any tab, after the tab copied from changes, sleeps or closes. Costs a little memory while something is copied (mostly shared: meshes, texture sets and graphs are shared `Arc`s, not copied again).
3. **The copy written to the system clipboard as text or a file.** Would also paste into a second running copy of the app, but means a new file format to keep, check and fuzz, and large paint through the clipboard. Nothing asks for it yet.

For clashes:

- **Always give new ids and names.** Simple, but pasting the same thing twice fills the project with copies of the same graph and own box.
- **Reuse what is the same, give new ids to what differs.** Chosen.

## Decision

Option 2, with reuse.

- **Copying** (Ctrl + C or Alt + C, as before) also keeps a `scene::Clipping`: the chosen objects with everything inside them, as they are; the objects they need that weren't chosen (cutters, cages, curves and shapes modifiers read, cross-sections), with what is inside those; the texture sets of their materials; the graphs those layers' graph fills run; and the own boxes those graphs use, with the own boxes inside them. An instance whose original isn't in the clipping becomes a copy of its original's shape (its kind, edits, booleans, modifiers and mirror), keeping its own name, place and materials. A chosen object inside a parent that isn't copied is placed where it was in the world.
- **The clipping is shared by every tab** (handed on with the settings, `Workbench::take_shared_from`), and held in memory only; nothing new is written to the system clipboard (it keeps its short note, so the window still hears Ctrl + V). It stays until the next copy, whatever happens to the tab it came from.
- **Pasting in the same project** works as before: copies (Ctrl + V) or instances (Alt + V) of the copied objects as they are now, each just after its original. Each project gets a number in the run (`Workbench::project`), new for every project opened or started, so a tab that has since opened another project pastes the clipping, as from another project.
- **Pasting in another project** (`Command::Paste`) is one undo step in that project:
  - **Own boxes** (`matgraph::own::bringing`): one the same as one the project has (inside, plugs, sliders, words and picture; its name may differ by a number, from an earlier paste) is used as it is; one whose id the project has for a different box gets a new id (and a numbered name if its name is taken too); otherwise it comes in as it is. Boxes holding others follow the ids given.
  - **Graphs:** the same graph the project has (after its own boxes are renamed; its name may differ by a number) is reused; otherwise it is added with the project's next id, and a numbered name when the name is taken ("Cobbles 2"). Graph fills are pointed at the ids given.
  - **Materials:** a name keeps its name when the project doesn't use it, or has the same texture set for it (or both none); else a numbered name the project has with the same set is reused, else it gets a new numbered name ("Stone 2"). The standard material counts as used while any of the project's shapes have no materials; an object on it whose paint must be renamed is given that material by name.
  - **Objects** get the project's next ids and their own names ("Cube 2" when "Cube" is taken). They go at the end of the top level, the chosen ones become the selection, and everything they name is pointed at the pasted copies.
  - Project settings (units, texture size, trim sheet, palettes, recipes, layout snapshots, export folder, Bake window settings) stay the receiving project's.
- **Alt + V in another project** can't make instances of objects in another project: it pastes plain copies, and the status bar says why.
- **Boxes copied on the Material canvas** (Ctrl + C) keep the own boxes among them (with those inside); pasted in another project the own boxes come along the same way. Ctrl + V and Alt + V both place the own boxes brought in (links can't reach into another project; Alt + V says so). In the same project, Ctrl + V and Alt + V work as before.
- **Refusals** are said plainly in the status bar: a project with no room for more graphs or own boxes, or a damaged box, and nothing is changed.

## Consequences

- Copying a lot of painted objects keeps their paint alive in memory until the next copy, even after their tab closes.
- Pasting the same thing twice into a project reuses what the first paste brought; two graphs or own boxes that are identical apart from a number in the name are treated as the same.
- The library of own boxes (decision 0263) still places the project's own version when a box with the same id is there; pasting differs on purpose: it brings exactly what was copied.
- No new saved data: nothing to round-trip or fuzz beyond the projects pasting makes, which save and open as any other.
