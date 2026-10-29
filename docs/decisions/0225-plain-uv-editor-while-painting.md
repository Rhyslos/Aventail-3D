# 0225. A plain UV Editor while painting

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The mockup's Paint page ([0223](0223-paint-page-like-the-mockup.md)) shows the UV Editor as a dark square with its pieces and a small name at its top ("UV · Clay pot"). The app's had its tab, its menu line (UV, Texture, Trim, Snapshots, View), 0 and 1 beside the square, and notes along the bottom (how many pieces, the texture size and texel density), which also kept the tool row ([0224](0224-paint-tool-row.md)) from fitting on one line.

## Decision

While painting, the UV Editor has no tab, no menu line, no 0 and 1 and no notes. At its top left, under where the brush bar may reach, it writes **UV ·** and what it shows: the object's name, the first and how many more, "no UVs", "too many polygons to draw" or "select an object to see its UVs". Its menus stay in the main **UV** menu. Everything else (the warnings at its top right, the stretch view's note, painting, navigating) is as before. Outside Paint mode nothing changes.

## Consequences

- The tool row fits on one line beside the view controls whenever the painting area is wide enough.
- The texture size and texel density aren't shown while painting; the UV Edit layout and the UV menu's Texture entries show them.
