# 0231. A steady frame rate, against flicker on variable-refresh screens

- **Status:** Accepted
- **Date:** 2026-10-30

## Problem

The project lead saw the whole screen flicker as the mouse moved on the Paint page. It happened on a 144 Hz monitor (AMD RX 7900 XT), but not when the window was moved to a 60 Hz one. Switching to another app and back sometimes stopped it.

The app draws a new picture only when something changes. A screen with a variable refresh rate (FreeSync, G-Sync) refreshes when the app sends a picture, so its pace jumps between nearly still and fast as the mouse moves. Some such screens change brightness when the pace jumps. The Paint page does the most work for each move: the brush is placed on the model and drawn in both views, and the painted texture is shown twice. So its pictures come the most unevenly. This is why the flicker was seen there.

## Options

- **Leave it to the driver:** the person turns FreeSync off for the app. That works, but nobody knows to.
- **Cap or smooth the pace in the app:** this needs timing the GPU's work, which egui and wgpu don't offer simply.
- **Draw steadily (chosen):** ask for the next frame at once, every frame. Presenting waits for the screen (vsync), so the pace is the screen's own, steady one.

## Decision

- **Settings → Viewport → Steady frame rate:**
  - **While painting** (standard): only on the Paint page.
  - **Always:** on every page.
  - **Off:** draw only when something changes, as before.
- **Only while the window is in front:** a window behind others never keeps drawing.
- **Restore standard** goes back to While painting.
- **Test harnesses switch it off**, as they run frames until the window rests; the tests that check it switch it back on.
- No keys or mouse actions are added ([0028](0028-ask-about-controls.md)).

## Consequences

- **The Paint page** draws at the screen's rate while the app is in front: a little more power and warmth. Heavy scenes cost more; Off is there for laptops on battery.
- **The real cause can't be tested by CI:** it needs a variable-refresh screen. The project lead confirms it by hand.
