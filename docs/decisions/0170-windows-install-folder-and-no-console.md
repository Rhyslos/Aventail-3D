# 0170. Choosing the install folder, and no console window

- **Status:** Accepted (the project lead's request)
- **Date:** 2026-10-25
- **Builds on:** [0169](0169-installer-and-updates.md) (the Windows installer)

## Problem

After installing build 1, the project lead asked for two changes:

1. The setup program installed straight into the C: drive without asking: "the installer needs to be specified where to install".
2. A console window (the terminal) opened with the app and had to stay open while it ran.

## Decision

1. **The setup program asks for the folder on the first install** (Inno Setup's `DisableDirPage=auto`), suggesting the person's own Programs folder; any folder they may write to, on any drive, will do. It still needs no admin password. Updates and reinstalls go back into the folder chosen then (`UsePreviousAppDir`), without asking. The Ready page shows where it will go before installing.
2. **On Windows the app is a window program** (`windows_subsystem = "windows"`), so no console window opens with it. `--version`, `--help` and command-line mistakes still print when it is started from a terminal: it borrows that terminal (`app::platform::use_parent_terminal`, `AttachConsole`). If the window can't open, the reason also shows in a message box, as there may be no terminal to read it in.

## Consequences

- Only the program goes into the chosen folder. Settings, the panel layout, backup copies and downloaded updates stay in the person's own app-data folder on the system drive, where Windows keeps every program's.
- Started from a terminal, the prompt comes back at once, before anything the app prints; that is how window programs behave on Windows.
