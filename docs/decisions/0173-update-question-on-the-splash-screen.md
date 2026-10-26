# 0173. The update question on the splash screen, and Skip this version

- **Status:** Accepted (the project lead's request)
- **Date:** 2026-10-26
- **Changes part of:** [0172](0172-update-popup-and-splash-check.md) (the splash screen downloading a new version before asking)

## Problem

The project lead described how starting should go: "When the update checker finishes, the app will either launch, or display a popup that there is an update available and prompt you to update the app, or open the app without updating. There should also be a checkbox option to 'skip version', that will not prompt the updater unless a new version releases again. There should also be a setting to stop the update checker all together."

## Decision

1. **The splash screen asks** when its look finds a newer build that isn't skipped (`Found::Newer`): *A new version is available*, with **Update now** (Enter), **Open without updating** (Esc), **What's new in this build** and **Skip this version** (`ui::splash::update_question`). Nothing downloads before the answer.
   - **Update now:** the splash screen downloads it (*Downloading build N… %*, no time limit: it was asked for), then *Installing build N…*: the window closes and the setup program installs it and opens the app again. If the download fails, the app opens as it was and the status bar says why.
   - **Open without updating:** the app opens; nothing installs. That build isn't offered again during this run (it is at the next start).
   - **Skip this version** (with Open without updating; Update now is greyed while it's ticked): remembered (`ViewOptions::skipped_update`); that build is never offered again, a newer one is. **Settings → Interface → Updates** names the skipped build, with **Ask about it again**.
2. **While the app runs**, later looks download a newer build at once and offer it in the window of 0172 (Restart now, Later), which gains **Skip this version** (with Later: not installed, `WorkbenchAction::SkipUpdate`).
3. **The looking can be stopped altogether** with the setting of 0169, **Look for new versions**; off, the splash screen doesn't look.
4. The updater (`app::updates::Updater`) reports the first look's newer build and waits for `download`; builds up to `set_quiet` (the skipped one, or the one declined this run) are reported as `Found::Skipped` and never downloaded.

## Consequences

- Nothing is downloaded at start unless the person chooses Update now.
- Declining (Open without updating) installs nothing at close, unlike Later in the running app's window, which installs at close.
- Reference picture `splash_update_question` shows the question.
