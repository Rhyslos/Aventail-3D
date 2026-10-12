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
- **Two looks, chosen in Settings → Interface:** a **radial** ring (standard), or a **list** for those who prefer C4D's M menus. Both open with `E` and hold the same tools in the same order.
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

## Consequences

- **Adding a tool is one entry in the command list,** plus its work. The tool menu, the menus and the keys pick it up.
- **Command search** (planned, Shift + C or Ctrl + K) becomes one more view of the same list.
- **Room is limited.** The modelling ring is close to the most that fits comfortably (14 places). More tools would need a second ring or a "More…" entry, which would be decided when it happens.
