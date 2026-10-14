# 0056. The keyboard shortcut viewer: Keys on the menu bar

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (the command list)

## Problem

Phase 4 brought many keys. They were spread over the manual pages, the menus and tooltips, with no one place to see them all.

## Decision

1. **Where** (the project lead's choice: "Layout button next to help in top menu"): a **Keys** button on the main menu bar, right after Help, opens a **Keyboard shortcuts** window; clicking it again (or the window's ×) closes it. No key of its own, so nothing is taken from the tools.
2. **What it shows:** every key and mouse action, grouped: File and editing, Modes, Selecting, Move rotate and scale, Modelling, Objects, The view. Keys are written as the system writes them (Ctrl on Windows and Linux, ⌘ on a Mac). A search field at the top keeps only the lines holding every word typed, in the action or the keys.
3. **Never out of date:** the keys are read from the same constants and command list the app acts on, and a test checks that every command with a key is listed with it. Mouse actions, which have no constants, are written out in the viewer's code.

## Consequences

- One place to check a key before choosing new ones, for the project lead and for each new tool.
- When custom keys arrive, the viewer is where they will be changed.
