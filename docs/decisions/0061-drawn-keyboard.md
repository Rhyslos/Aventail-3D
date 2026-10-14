# 0061. The Keys window draws a keyboard

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0056](0056-shortcut-viewer.md) (the keyboard shortcut viewer)

## Problem

The Keys window (decision 0056) was a long list. It answered "what is the key for X?", but not "what does this key do?" or "which keys are still free?", and it didn't show where a key sits, which matters on keyboards other than the American one: the app reads letters by what is printed on them, so Z is in a different place on a German keyboard, and A on a French one.

The project lead asked for a drawn keyboard like the keybinds page of a game they showed: lit keys, colour marks, a legend, pointing at a key to see its jobs, a search, and the mouse actions too; plus a choice of keyboard layout, guessed from the system and remembered, and a check that every shortcut can be reached on every layout.

## Options

1. **Keep the list, add a layout note:** cheap, but doesn't answer "what does this key do?".
2. **A drawn keyboard with the list beside it:** answers both questions, at the cost of drawing keyboards for each layout.

## Decision

Option 2.

1. **The drawing:** the F row, the main block, and the Delete and arrow keys. Keys the app uses are lit; unused keys are dark. Each lit key has a small coloured mark for each situation it works in: **Anywhere** (blue), **Model mode** (purple), **Points, Edges, Polygons** (orange), **While dragging** (green), **Menus and questions** (yellow).
2. **Pointing at a key** lists its jobs in a panel beside the keyboard, each with its full keys and its situation's colour; **clicking** keeps them listed. Shift, Ctrl and Alt list what holding them does and, under **Held with**, every shortcut using them.
3. **Chips** in the situations' colours hide or show each situation's keys.
4. **The search** outlines the keys whose jobs match and lists the matches.
5. **Mouse actions** are listed under the keyboard, with the same colours.
6. **Layouts:** US, UK, Norwegian, Danish, Swedish / Finnish, German (QWERTZ) and French (AZERTY). The first time, the layout is guessed from the system language (read with the `sys-locale` crate, MIT or Apache-2.0); a layout picked in the window is remembered with the view options.
7. **What a key gives:** letters follow the printed letter, the number row always gives 1 to 0 (on AZERTY, the & key is 1), as egui reports them. Each drawn key records the key the app reads from it, and a test checks that every key the app uses exists on every layout.
8. **Size** (the project lead: the first version was far too small to read on an ultrawide): the window opens at 90% of the app's window, but no wider than 2.1 times its height, so very wide monitors don't stretch it. The key size is whatever fits the window (the keyboard and the jobs panel across, everything down), from 24 to 110 points, and the window's text and spacing scale with the keys. A line divides the keyboard from the jobs panel.
9. **Still never out of date:** every job comes from the same constants and command list the app acts on, as in 0056; only what holding Shift, Ctrl or Alt does, the situations, and the mouse actions are written out in the window's code.

## Consequences

- Free keys are visible at a glance, which helps when choosing keys for new tools (decision 0028).
- A new key needs its situation chosen when it's added to the window; tests catch a command key that is missing, or a key that a layout lacks.
- More layouts can be added in `crates/ui/src/keyboard.rs`, each with its printed labels.
- When custom keys arrive, clicking a drawn key is the natural place to change it.
