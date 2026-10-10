# 0016. Keyboard and mouse controls follow Halberd

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Every tool, mode and command will need keys. Copying another 3D program's keys means learning a second set: the project lead already uses Halberd, the Rhylib map editor, every day. They asked that the app copies Halberd's controls, except viewport navigation, where the navigation schemes are still being tried ([0015](0015-navigation-schemes.md)).

## Options

1. **Keys from other 3D programs:** familiar to people coming from them, but a different set from Halberd's.
2. **Halberd's keys and mouse use**, the same in both Rhylib tools, extended in the same spirit where the app needs more.

## Decision

Option 2. As of Halberd's main branch on 2026-10-10:

| Keys | Action | When |
|---|---|---|
| `Ctrl + N` / `Ctrl + O` | New project / open a project | Anywhere |
| `Ctrl + S` / `Ctrl + Shift + S` | Save / save as… | Anywhere |
| `Ctrl + Q` | Quit | Anywhere |
| `Ctrl + Z` | Undo | Anywhere |
| `Ctrl + Y`, `Ctrl + Shift + Z` | Redo | Anywhere |
| `Delete`, `Backspace` | Delete the selection | Anywhere |
| `Q` | Select tool | Viewport |
| `W` | Move gizmo (again: off) | Viewport |
| `R` | Rotate gizmo (again: off) | Viewport |
| `S` | Scale gizmo (again: off) | Viewport |
| `T` | All gizmos at once (again: off) | Viewport |
| `Esc` | Deselect, or cancel a drag | Viewport |
| `H` / `F` | Frame all / frame selected (new here; Halberd has none, and these keys are free there) | Viewport, not while typing |
| `F1` / `F2` / `F3` / `F4` | Show the perspective / top / right / front view on its own (new here; free in Halberd) | Anywhere, not while typing |
| `F5` | Show all four views (new here; free in Halberd) | Anywhere, not while typing |
| `B` | Create tool; again: open or close the shape list (Halberd's Draw tool, for primitives here) | Viewport |
| `Ctrl + W` | Pick single objects inside groups (on/off) (Halberd: brushes inside entities) | Viewport |
| `R` / `Shift` / `Alt` / `Esc` / `Q` | While creating a shape: quarter turn / equal sides / equal sides around the start / cancel / cancel and pick Select | While creating |
| `↑` `↓`, `Enter` or `Space`, `Esc` | Shape list: previous / next, pick, close | Shape list open |
| `W` `A` `S` `D`, `Space`, `C`, `Shift`, wheel | Fly forward, left, back, right, up, down, faster; wheel sets fly speed | Fly camera, holding right mouse |
| `Shift + W`, `Shift + S` | Move gizmo, scale gizmo | Holding right mouse |
| `Enter` / `N` / `Esc` | Save / don't save / cancel | "Save changes?" showing |
| `Esc` | Close the message | Message box showing |

Mouse, outside the navigation schemes: **left click** selects (`Ctrl` + click adds or removes), **left drag** creates a shape (Create tool) or drags a gizmo handle. A **middle click** (pressing the wheel without dragging) in the viewport switches between one view and four; a middle drag still does what the navigation scheme says.

- **One list of every binding** with when it applies, and a **Keybinds** viewer (a drawn keyboard: hover a key to see what it does, search for an action to light up its keys), as in Halberd. Changing bindings comes with a Settings page later, editing the same list.
- **Navigation stays with the schemes** from 0015 until the project lead picks a standard. Halberd's camera modes (orbit and a right-mouse fly camera) are kept in mind for that choice.
- **Where Halberd has nothing yet** (modelling tools, modes, views, frame selected), new keys are chosen so they never clash with the table above, and are copied back to Halberd's style of naming (plain words, a "when" for each).
- When Halberd's controls change, this app follows; check its `halberd-ui/src/keymap.rs` before adding keys.

## Consequences

- Moving between the two Rhylib tools needs no relearning.
- Some keys differ from other 3D programs (for example `W`, `R`, `S` for the gizmos, and `S` is not "frame selected"). The manual says so where it matters.
- Earlier plans in DESIGN.md §5.3 and §5.4 that named other keys (frame selected on `S`, views on `F1` to `F5`, a modelling menu on a letter key) are checked against this table when they're built; Halberd wins any clash. The views kept `F1` to `F5` (2026-10-10), so the in-app help planned for `F1` in §11 will need another key when it's built.
