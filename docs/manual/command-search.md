# Command search

> Press Space, type a few letters of what you want, and press Enter: any command, without hunting through the menus.

## When to use it

- You know what you want to do but not where it lives: *bevel*, *export*, *pivot*, *seams*.
- You've forgotten a command's key: the search shows it beside the name, so it also teaches you the keys.
- You want a command that has no key at all, such as **Apply mirror** or **Edit pivot**.

For the commands you use all the time, the [tool menu](tool-menu.md) (**E**) and the keys are quicker. The search is for everything else.

## How to use it

1. **Press Space** (not while typing in a field), or click the **magnifying glass** at the right end of the top palette, or choose **Help → Search for a command**. A box opens near the top of the window.
2. **Type part of the name**, such as *ext* for Extrude or *export*. A word from what a command does finds it too: *border* finds Inset. Capitals don't matter, and with several words every word must be there (*pull polygons*).
3. The matches are listed best first: names that start with what you typed, then names with a word starting with it, then the rest. Each shows where it lives (such as *Mesh* or *File*), what it does, and its key if it has one. At most 12 are listed, so type a little more if what you want isn't there.
4. **Press Enter** to run the highlighted one (the first, until you move), or **click** any of them. **↑** and **↓** move the highlight. **Esc** closes the box without running anything.

What it finds:

- every tool and command of the [tool menu](tool-menu.md), in every mode;
- the modes: *Model mode*, *Points mode* and the rest;
- the ready-made shapes: *Cube*, *Cylinder* and the rest;
- the other menu commands: New, Open, Save, Import, Export, Check for baking, Settings, Keys, the user manual and the glossary, Undo and Redo, the Select commands, Hide, Isolate and Show everything, the UV Editor, Show hard edges, Show seams, Show normals and Show collision, High contrast, [Edit pivot](pivots.md), [Apply mirror](symmetry.md) and Group.

A command that can't be used right now (say Bevel with nothing selected) is listed in grey, below the ones that can; hold the mouse over it to see why.

## Settings

None.

## Shortcuts

| Action | Shortcut |
|---|---|
| Open command search | `Space`, the search button on the top palette, or Help → Search for a command |
| Move the highlight | `↑` `↓` |
| Run the highlighted command | `Enter` (or click one) |
| Close without running anything | `Esc` |

## Tips and common problems

- **Problem:** Space typed a space instead of opening the search. **Why:** a text field (a name, a number) has the keyboard. **Fix:** click in the viewport first, or press Esc to leave the field.
- **Problem:** Space does nothing while the tool menu or the shape list is open. **Why:** one thing at a time. **Fix:** close it first (Esc).
- **Problem:** "Nothing found". **Why:** no command's name or description has every word you typed. **Fix:** try fewer words, or another word for the same thing (*delete*, *remove*).
- **Tip:** the shortcuts beside the names are the quickest way to learn the keys. The [Keys window](keyboard-shortcuts.md) shows them all on a drawn keyboard.

## Related

- [The tool menu (E)](tool-menu.md)
- [Keyboard shortcuts](keyboard-shortcuts.md)
- [Interface tour](interface-tour.md)
