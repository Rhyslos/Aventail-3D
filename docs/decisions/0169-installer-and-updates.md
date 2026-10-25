# 0169. A Windows installer, and updates from GitHub

- **Status:** Accepted (the project lead's request and choices)
- **Date:** 2026-10-25
- **Brings forward part of:** Phase 13 (installers and updates, DESIGN.md §12)

## Problem

The project lead tested each build by downloading a zip from a pull request, unpacking it and running the program from there, past browser and Windows warnings each time. He asked for the app to be "actually installed on the system, and auto update from GitHub whenever there is a new version merged into main".

## Options

- **Updating:** download in the background, then ask (**Restart now** / **Later**, installing at the next close) (recommended); fully automatic, installing at the next close without asking; or only a note with a button.
- **Where:** just for the person (no admin password, quiet updates) (recommended), or for everyone on the PC (admin permission on every update).
- **How the installer is made:** Inno Setup (a free, widely used installer builder whose licence allows commercial use; a build tool, not a library in the app), WiX (MSI files; its toolset is under the MS-RL licence), or the app copying itself into place. Inno Setup: per-user installs, uninstall entries, file associations and quiet updates closing the running app are all built in.
- **How the app fetches:** a Rust HTTP library with TLS (several new crates and a TLS stack), or the system's own `curl`, part of Windows since Windows 10 (1803), with Windows' certificates. `curl`: nothing added to the app.

## Decision

The project lead chose **download, then ask** and **just for me**. So:

1. **Releases.** After every change to `main` whose CI passes, `.github/workflows/release.yml` builds Windows in release mode with `APP_BUILD` set to its run number, makes the setup program and publishes a GitHub release tagged `build-<number>`, titled *Aventail 3D build N*, with the setup program attached and the change's message as notes. The newest 10 releases are kept. It can be run by hand too.
2. **The setup program** (`installer/windows/setup.iss`, built by `installer/windows/build-setup.ps1`, which reads every name from `branding.rs` and `app/Cargo.toml`): installs for the person only (`PrivilegesRequired=lowest`, into `%LOCALAPPDATA%\Programs\<APP_NAME>`), with a Start menu entry, an optional desktop icon, an uninstall entry and `.avtl` files opening in the app. Quiet (`/VERYSILENT`), it closes a running copy and opens the app again afterwards unless given `/NOLAUNCH`. CI's Windows Build job makes it on every pull request too, so it is always checked, and uploads it as `app-windows-setup`.
3. **Build numbers.** A build made by the release workflow carries its number (`app::updates::build_number`), shown in the title and About box: *0.0.1 (build 412)*. A build made by hand has none and never updates itself.
4. **Looking** (`app::updates`). An installed copy (its uninstaller beside it) on Windows, with a build number and **Settings → Interface → Updates → Look for new versions** on (standard), asks GitHub for the latest release (`branding::RELEASES_API`) ten seconds after starting and every six hours, on another thread. A newer build's setup program is downloaded into the app's local folder (`updates`, beside `autosave`) as a `.part` file, kept only once it has the size GitHub gave, and older ones are removed. Settings shows what the last look found, in plain words (*no version has been published yet*, *no internet connection*).
5. **The bar** (`ui::workbench::updating`): *A new version is ready (build N).* **Restart now** · **Later** · **What's new** (the release page). Restart now closes the window as any close does (unsaved changes asked about), then runs the setup program quietly, which reopens the app. Later puts the bar away until a newer build, and the setup program runs quietly at the next close, without reopening. The workbench only shows and returns `WorkbenchAction::InstallUpdate`; the app does the rest after the window has closed.
6. **Fetching** is done by the system's `curl` (no window flashing up), so the app gains no network library. Only GitHub is asked; nothing about the person or their work is sent.

## Consequences

- Install once from the Releases page; every later build arrives by itself, after CI has passed.
- The setup program isn't code-signed: the browser and Windows warn the first time. The installed app downloads its own updates, which Windows doesn't mark as from the internet, so they install without a warning. Signing (for example through the SignPath Foundation, free for open-source projects) is for later.
- No icon of its own yet: shortcuts show Windows' plain program icon until the app has one.
- Linux and macOS keep their zips under GitHub's Actions tab; their installers come with Phase 13.
- A Restart now whose "Save changes?" is cancelled leaves the app open; the update installs, and the app reopens, at the next close.
- The release workflow can write to the repository (`contents: write`), only to publish releases; it runs only on main's own changes after CI.
