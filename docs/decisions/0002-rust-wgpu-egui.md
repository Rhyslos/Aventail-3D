# 0002. Rust, wgpu and egui

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

The app must feel smooth with game-asset meshes (up to about a million triangles), must never lose work to a crash, and should run on Windows, Linux and macOS.

## Options

1. **C++ with Dear ImGui:** fastest and most mature, but crash-prone memory bugs and complex cross-platform builds.
2. **TypeScript with Three.js, in Tauri or Electron:** fastest to iterate, but weaker performance for heavy mesh editing and higher memory use.
3. **Rust with wgpu and egui:** C++-level speed with memory safety, one-command builds on every platform, one GPU API over Vulkan, DirectX 12 and Metal. The project lead already builds Halberd this way.

## Decision

Rust for everything, wgpu for the viewport, egui with egui_dock for the dockable C4D-style panels.

## Consequences

- Memory-safety bugs, the classic cause of modeller crashes, are ruled out by the compiler in safe code.
- egui does not look like Cinema 4D out of the box; getting the calm look right is theming work in `ui`.
- Rust is slower to iterate in than a web stack, which matters most for UI tuning; the "how to test" list in each pull request keeps feel feedback quick.
- macOS builds come for free from CI but are untested by the project lead.
