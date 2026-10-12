# 0036. Autosave: backup copies in the app's own folder, found again by file locks

- **Status:** Accepted
- **Date:** 2026-10-12
- **Follows:** [0033](0033-heavy-mesh-safeguard.md) (which added autosave to Phase 4, wishlist W12)

## Problem

A crash, a power cut or a frozen graphics driver should never cost more than a few minutes of work. The safe saving from Phase 3 protects the project file *while* it is written. It does nothing for work that was never saved, and nothing for an *Untitled* scene.

Three questions had to be answered:
1. Where do backup copies go?
2. How does the next start know the app crashed, rather than closed normally?
3. What happens then?

## What the project lead chose

Asked on 2026-10-12:

| Question | Choice |
|---|---|
| How often | **Changeable in Settings.** Every 5 minutes as standard, 1 to 30 allowed, and backups can be switched off. |
| Where | In the app's own data folder ("should it be in %Appdata%?"): yes. |
| How many | **The last 5** as standard, 1 to 20 allowed in Settings. |
| After a crash | **Ask first**, rather than restore silently. |

## Options considered

**Where:**
- *Beside the project* (`crate.avtl.autosave`). This was rejected for three reasons:
  - it clutters project folders and sync tools;
  - it fails for read-only folders;
  - it can't cover a scene that was never saved.
- *The app's own folder.* This was chosen. On Windows it is `%LOCALAPPDATA%\<app id>\autosave`, using **Local** rather than Roaming, because copies of heavy scenes are large and shouldn't travel between computers with a roaming profile. On macOS and Linux it is the same folder as the app's other data (for example `~/.local/share/<app id>/autosave`). Settings → Autosave shows the path.

**Detecting a crash:**
- *A "clean exit" flag in the settings file.* This was rejected. Two copies of the app running at once would confuse it, and a crash while the settings are written could lose it.
- *A lock file per running app.* This was chosen. Each run makes its own *session* folder and keeps a file in it locked. The operating system lets go of the lock when the process ends **in any way**, a crash or a kill included. The next start can therefore tell a crashed session (lock free, folder still there) from a running one (lock held). Two apps open at once never touch each other's copies. The lock uses `std::fs::File::try_lock`, from the standard library since Rust 1.89, so no new dependency is needed.

## Decision

**Writing copies**
- **Separate jobs.** The workbench (`ui`) decides *when* a copy is due and asks for one with `WorkbenchAction::Autosave`; it never touches files. The app (`app::backups`) writes the copy on another thread. The file work is in `fileio::autosave`.
- **The timer runs on its own clock, separate from saving.** A copy is due an interval after the last copy (or after the scene was opened), if anything changed since that copy, whether or not the project was saved in between. A quiet scene therefore never writes, a busy one writes once per interval, and the first change after a quiet spell longer than the interval is copied at once. While a copy is being written, no other is started. (First built counting from the first change since the last *save* or copy. The project lead found that each Ctrl + S restarted the countdown, and asked for the two to be separate, 2026-10-12.)
- **Copies are ordinary project files** (`copy-N.avtl`, the highest number the newest). Each is written beside its final name and then renamed, so a copy is never half-written. The oldest beyond the chosen number are deleted.
- **The project file itself is never touched** by autosave.

**Closing and crashing**
- **A normal close deletes the session folder.** Whatever was unsaved was either saved or deliberately thrown away in the "Save changes?" question.
- **After a crash**, the next start finds the left-behind folders and asks about each, newest first, in a "Recover unsaved work?" window. The window lists the copies with how long ago each was saved.
- **The three answers:**
  - **Restore** (or Enter) brings back the chosen copy as *unsaved* work. The project name is kept, so Ctrl + S saves over the original.
  - **Open as saved** opens the project as it was last saved. It is offered only when the project had a file.
  - **Discard** deletes the copies.

  Once answered, the left-behind folder is deleted.
- **Restored work is protected at once.** It is copied into the new session straight away, keeping its original time, so a second crash before the next interval still finds it.
- **Problems are shown in the status bar** ("Couldn't save a backup copy: …"). They never stop the user's work.

## Consequences

- Up to *copies × scene size* of disk space is used while the app runs, and nothing after a normal close. With heavy scenes and 20 copies this can be large, which is one more reason for Local AppData.
- A copy of a heavy scene takes a moment to write, but it happens on another thread. The scene snapshot is cheap because meshes share their parts ([0035](0035-heavy-meshes.md)).
- Backups before risky steps (wishlist W12) are not built yet. They come with the first tool that needs them, such as booleans in a later phase.
- A related bug was fixed along the way: opening or restoring a project mid-session could leave the viewport empty. The drawing cache restarted its version number at the one the GPU had just drawn, so nothing new was uploaded.
