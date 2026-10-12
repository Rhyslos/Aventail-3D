# 0037. The tool menu (E) and one command list

- **Status:** Accepted
- **Date:** 2026-10-12
- **Follows:** [0019](0019-only-the-tool-keys-come-from-halberd.md), [0028](0028-ask-about-controls.md), [0008](0008-window-and-panels.md) (icons drawn in code)

## Problem

Phase 4 brings about fifteen modelling tools. The project lead had forgotten the tools and keys he used to know, and asked how Cinema 4D and 3ds Max deal with that. Neither expects anyone to remember every key. Both mix three layers:

| Layer | Cinema 4D | 3ds Max |
|---|---|---|
| Single keys for the most used tools | Yes | A few |
| A menu at the mouse | The M two-key menus, the V popup | The right-click quad menu |
| Search | Commander | Search |

A second problem showed up in the code. A command's name, icon, key and tooltip were written separately in each place that showed it: the palette, every menu, and the Object Manager. The tool menu and a later command search would have added more copies. That is how one feature ends up looking or being named differently in two places.

## What the project lead chose

Asked on 2026-10-12:

- **`E` opens the tool menu**, everywhere in the app. A tap opens it; holding isn't needed. The menu is **context sensitive**: object tools in the Model and Object modes (move, rotate and scale included), and only modelling tools in the Points, Edges and Polygons modes, since the top palette already has the transform tools.
- **Looks, chosen in Settings → Interface:** a **radial** ring (standard), or a **list** for those who prefer C4D's M menus (a third, **Segments**, was added after the first look; see below). Both open with `E` and hold the same tools in the same order.
- **Stepping:** tapping `E` again steps through the tools, as `B` does in the shape list, and **`Space` or `Enter`** uses the highlighted one.
- **The ring may hold more than eight tools**, as long as the icons are clean and understandable.
- **Keys for tools to come:** `D` extrude, `I` inset, `V` bevel. `D` matches Cinema 4D. The keys switch on when the tools are built.
- **Icons stay drawn in code**, in one place (`crates/ui/src/icons.rs`), not as image files.
- **One place for each command**, so nothing looks different across the app: the command list.

## Decision

**The command list** (`crates/ui/src/commands.rs`):
- **One entry per command:** its name, icon, key, plain-language explanation, and, for a command not built yet, when it arrives.
- **Everything reads from it:** the tool menu, the Mesh and Tools menus, the palette's tooltips, and the Object Manager's menus. The object commands' keys (`Ctrl + E`, `Alt + G`, `Shift + G`, `Delete`) are read from it too.
- **One function carries out a command** (`Workbench::run_command`), whether it comes from the menu or a key. It is also where the app says why a command can't be used now ("Nothing is selected.").
- **Tests check that it agrees with the keys the app uses.**

**The tool menu** (`crates/ui/src/tool_menu.rs`):
- **Opening and stepping:** `E` opens it at the mouse. Tapping `E` again steps forward, and `Shift + E` steps back; the arrow keys work too, mirroring the shape list. `Enter` or `Space` uses the highlighted tool, and `Esc` closes the menu.
- **The mouse:** pointing highlights a tool, and clicking uses it. In the radial look **the whole slice towards a tool is its target**, so a rough direction is enough. A click in the middle, or far from the ring, closes it.
- **Hold and flick:** holding `E`, pointing towards a tool and letting go uses it, like Blender's pie menus. A plain tap never waits for anything, because the menu only acts on letting go if the mouse pointed at a tool while `E` was held.
- **Every tool keeps its place.** The ring has fixed places, clockwise from the top. The object ring has 11; the modelling ring has 14, grouped as building (top right), shading (right), removing (bottom), and cutting and joining (left). A mode without a tool leaves a gap rather than moving the others, so a direction always means the same tool.
- **Every entry shows its key.** The middle of the ring shows the highlighted tool's name, key and explanation, or why it can't be used now.
- **Tools still to come are shown, dimmed,** with when they arrive, as everywhere else in the app (0008). The ring is complete from the start, and the project lead can judge it before the tools exist.
- **The UV mode has no tools yet.** Pressing `E` there says so in the status bar.
- **The shape list and the tool menu never show at once.** The menu's "Create a shape…" opens the shape list where the menu was.

**Controls added** (0028): `E`, `Shift + E`, the arrow keys, `Enter`, `Space` and `Esc` while the menu is open, plus pointing, clicking and hold-and-flick. All follow what the project lead chose, and the shape list's pattern where he said "as with the shape list". No key that was already in use changed.

## Added after the first look (2026-10-12)

After trying it, the project lead asked for three changes, all in the same pull request:

- **A bigger middle.** The space where the mouse points at nothing, and a click closes the menu, grew from 22 to 40 points across its radius.
- **A third look, Segments**, after a picture he shared: the ring as separate tiles with a little space between (6 points), nothing behind them, and the name and key outside each tile. The highlighted tile lights up, with an accent edge on its outside. Tiles of tools that can't be used now are darker but never see-through, so they still read over a light shape.
- **"Tap E to open"**, a switch in Settings → Interface, on as the standard. Off, a tap of `E` does nothing: the menu shows only while `E` is held, letting go over a tool uses it, and letting go anywhere else closes it. He may prefer it off himself. It stays on as the standard because tapping and stepping is the accessible way, for anyone who finds holding a key hard.

The settings are remembered together (`ToolMenuSettings`: the look and the switch).

## The final design (2026-10-13)

The project lead redesigned the menu and every tool's icon with Claude in a mockup page: an HTML page where he picked between drawn versions, with a live ring preview, over six rounds. The page saved his picks for Claude to read back. What was built from it **replaces the Radial and Segments looks above**:

**Icons**
- **A new icon for every tool**, the same everywhere (tool menu, palette and menus). All are drawn on the 24 × 24 grid with **one line width**; they differ only in tone (faint lines, light and strong fills, smooth shading) and, for Flip normals and Transform, the axis colours.
- **Converted point for point.** The picked drawings were converted from the mockup into the app's icon code, so the app draws exactly what was approved.

**Names**
- **Transform:** "Move, rotate and scale" (T).
- **Slit:** Knife, named after its icon.

**The ring**
- **Same size and places everywhere.** Every mode's ring has the same **14 places** at the same size, so no button ever moves or changes size between modes. Delete has the same place in the object and modelling rings, and empty places stay empty.
- **Icon only.** The buttons hold only their icons, centred. The middle names the tool pointed at, above a **cancel sign** (Delete's circle with a line): clicking there closes the menu.
- **Keys.** **Show keys around the ring** (a setting, on as standard) puts each key just outside its button. Off, the key shows in the middle with the name.

**Settings**
- **Four ring looks**, chosen in a new **Radial menu** section of Settings → Interface: **Continuous ring** (the standard: one band split by thin lines, the lit slice with an accent rim), **Inner edge**, **Outlined** and **Solid tiles**.
- **The List look stays** for those who prefer C4D's M menus.
- **Tap E to open** moved into the same section.
- **Tried and left out:**
  - turning icons with their buttons ("looks bad and isn't necessary");
  - labels inside the buttons, which pushed the icon off centre;
  - a 16-place ring, whose buttons were too narrow;
  - keycap and capsule buttons;
  - the earlier disc and Segments looks.

## Consequences

- **Adding a tool is one entry in the command list,** plus its work. The tool menu, the menus and the keys pick it up.
- **Command search** (planned, Shift + C or Ctrl + K) becomes one more view of the same list.
- **Room is limited.** The modelling ring is close to the most that fits comfortably (14 places). More tools would need a second ring or a "More…" entry, which would be decided when it happens.
