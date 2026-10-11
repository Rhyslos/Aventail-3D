# 0026. One small unsafe exception: giving the window the keyboard on Windows

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

The project lead heard the Windows error sound ("ding") on almost every key press. The app still reacted to the keys, and the sound went away after switching to another app and back.

The cause:
1. eframe, the window library, opens the window hidden and shows it once the first picture is drawn, to avoid a white flash.
2. Windows makes the window active while it is still hidden, and a hidden window can't take the keyboard.
3. Once it appears, the window is active, but nothing holds the keyboard.
4. In that state Windows sends every key as a "system" key (the kind Alt combinations use). The app still sees it, but Windows plays its error sound because it finds no menu shortcut for the key.
5. Switching away and back activates the window properly, which hands it the keyboard.

Neither eframe nor winit (the layer underneath) offers a way to hand the window the keyboard. Their "focus" request only acts when the window is *not* already the active one, which here it is. Calling Windows directly is `unsafe` in Rust, and [decision 0005](0005-safety-lints.md) forbids `unsafe` without a decision record.

## Options

1. **Live with it** until eframe or winit fix it upstream. The sound is constant and annoying, so this was rejected.
2. **Start the window visible** to avoid the hidden phase. eframe hard-codes the hidden start, so this would mean forking it.
3. **Minimise and restore the window** after start, to force a proper activation. It flickers, and it relies on the same Windows behaviour indirectly.
4. **Ask Windows directly**, once per frame: "is our window active, with nothing holding the keyboard? Then give it the keyboard." This uses three plain Windows functions (`GetActiveWindow`, `GetFocus`, `SetFocus`). They only read state and move the keyboard to our own window, and touch no memory of ours.

## Decision

Option 4, in `crates/app/src/platform.rs`. The workspace lint for `unsafe` changes from `forbid` to `deny`, so that this one module can allow it. Everywhere else `unsafe` is still refused by the compiler, and any new exception needs its own decision record, as before.

- The code only exists on Windows (`cfg(windows)`); elsewhere the function does nothing.
- It runs at the start of each frame on the window's own thread, as the Windows functions require. It acts only in the broken state: our window is active and nothing holds the keyboard.
- It uses `windows-sys` 0.61 and `raw-window-handle` 0.6, the versions eframe and winit already use (both MIT/Apache-2.0), so nothing new is downloaded or built.
- CI's lint job runs on Linux, so the Windows test job also runs `cargo clippy -p app`, to lint this Windows-only code.

## Consequences

- No more error sound when using keys after start. This can only be confirmed on Windows, by the project lead; CI only checks that it builds and passes lint.
- `unsafe` is now technically possible behind an explicit `allow`. Review must refuse any `allow(unsafe_code)` that doesn't point to a decision record.
- If eframe or winit fix this upstream, this module can be deleted and the lint set back to `forbid`.
