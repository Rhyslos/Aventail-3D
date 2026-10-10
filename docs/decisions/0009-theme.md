# 0009. The calm dark theme and the Inter font

- **Status:** Accepted; colours replaced by [0014](0014-studio-look-and-accent.md)
- **Date:** 2026-10-10

## Problem

The project lead remembers Cinema 4D's interface as "clean, pleasant, kind of calm", and approved mockups in that style. egui's default look is darker and higher in contrast, with a different font.

## Options

1. **Keep egui's default look:** no work, but not the calm feel the app is built around.
2. **A theme in one place** (`ui/src/theme.rs`): colours from the approved mockups, Inter for text, adjusted spacing, applied once at startup.

## Decision

Option 2:

- **Colours** from the mockups: neutral greys that differ only gently between areas (panels `#333333`, headers `#2a2a2a`, bars `#232323`, palettes `#2f2f2f`, fields `#262626`), a cool blue-grey for selections, and one warm accent (`#f2a33a`) for the active mode or tool.
- **Always dark**, whatever the operating system prefers.
- **Inter** (SIL Open Font License 1.1) for interface text, Regular and SemiBold, embedded in the program; egui's own fonts stay behind it as fallbacks for symbols and other scripts. Its license text is in `licenses/fonts/`.
- **Readability is tested:** normal and quiet text must reach WCAG contrast 4.5:1 on every background they appear on, and the accent 3:1 against the palettes. Neighbouring areas must stay within a gentle contrast (under 1.4:1), so the look stays calm.
- **The viewport fills its panel edge to edge**; the other panels add their own padding.

## Consequences

- Changing the look means changing one file, and the tests catch colours that become hard to read.
- Inter adds about 0.8 MB to the program.
- Later, the interface can offer a choice of themes or a light mode by building other `Style`s the same way.
