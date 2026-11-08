# 0265. Layout buttons on the menu bar, with the small tabs as a setting

- **Status:** Accepted
- **Date:** 2026-11-08

## Problem

The four layouts (Standard, UV Edit, Paint, Material) were switched with small text tabs on the right of the menu bar, the one in use underlined. They were easy to miss, and with scene tabs coming (several projects in one window) the menu bar is about to hold more, so the layouts need to read clearly as their own group. The project lead looked at mockups (the Design artifact with options A to F) on 2026-11-08.

## Options

1. **A, layout buttons:** one bar of bigger buttons on the right of the menu bar, each with a picture and the layout's name, the one in use filled with the accent colour; the menu bar a little taller. Clear and easy to hit; takes a little more room.
2. **Keep the small tabs:** the slimmest menu bar, but easy to overlook.
3. Other placements in the mockups (a separate row, a drop-down): more room or more clicks for something used often.

## Decision

Option A, chosen by the project lead, with the small tabs kept as a setting so the two can be compared:

- The layout buttons are the standard. Each picture is the icon of the mode the layout works in (cube, UV square, brush, wired boxes), drawn in code like every icon in the app; nothing is copied from other software. The menu bar grows 6 pixels (its items stay centred).
- **Settings → Interface → Layouts**: **Buttons with pictures** (standard) or **Small tabs**, saved with the view options (`ViewOptions::layout_switch`; options saved before read as the buttons).
- **Help → Try the small layout tabs** (or **Try the layout buttons**) flips the same setting in one click. **This entry is temporary**, at the project lead's request ("just for now so I can test it"): take it out once the choice is settled, with its manual lines (Interface tour, Settings) and changelog note.

## Consequences

- The layouts stand out as one group, ready for the scene tabs to sit between the menus and them (the next pull request).
- Everything below the menu bar moves down 6 pixels with the buttons: the window smoke test's fixed click positions moved with it, and the GPU reference pictures were made again.
- Two looks to keep working until the choice is settled; tests click every layout in both.
