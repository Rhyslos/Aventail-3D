# 0172. The update in a window, checked on the splash screen

- **Status:** Accepted (the project lead's requests)
- **Date:** 2026-10-26
- **Changes part of:** [0169](0169-installer-and-updates.md) (the bar under the top palette, the look ten seconds after starting, the build number in the title)

## Problem

After the first update worked, the project lead asked for three changes:

1. The build number in the window's title ("will it always say build 3 at the top now? That's kind of annoying").
2. The offer to be "an actual popup", not a bar.
3. The app to "stay on the splash screen until the update checker is finished".

## Decision

1. **The title shows the name, the project and the version only**; the build number moves to **Help → About** and the splash screen (`AppInfo::build`, `AppInfo::full_version`).
2. **A window in the middle** (the app's usual dialog, modal) offers the update: *A new version is ready*, the build, what Restart now and Later do, **What's new in this build**, **Later** and **Restart now**; **Esc** is Later. It replaces the bar.
3. **The splash screen waits for the first look** (`Step::Updates`, the last start-up step): *Looking for a new version…*, then, when a newer build is found, *Downloading build N… 40%* with the bar filling (the `.part` file's size against GitHub's, `updates::progress`). The look starts at once and asks GitHub for at most 8 seconds (`ASK_TIMEOUT_SECONDS`); the splash screen waits at most 10 seconds for the answer and a minute for a download, then opens the app, which offers the update when the download is done. A copy that doesn't update itself (not installed, built by hand, the setting off) doesn't wait. While the app is open, it looks every six hours as before.

## Consequences

- Starting takes a moment longer on an installed copy; without internet, at most about 10 seconds longer.
- A new version is offered the moment the app opens, rather than after ten seconds of work.
- Reference picture `update_offer` shows the window.
