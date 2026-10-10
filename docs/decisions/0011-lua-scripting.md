# 0011. Scripting in Lua (Luau, through mlua)

- **Status:** Accepted (confirmed by the project lead on 2026-10-10)
- **Date:** 2026-10-10

## Problem

The project lead wants people to be able to code their own functionality in the app: custom tools, batch jobs, generators, export helpers. The language must embed in a Rust app, have a permissive license (decision [0004](0004-gpl-license.md)), be safe to run when scripts are shared between strangers, and be pleasant for artists who are not programmers.

## Options

1. **Lua (Luau dialect, through `mlua`)**
   - Made to be embedded: tiny (about 1 MB), fast, nothing for users to install.
   - `mlua` and Luau are MIT. Luau (used by Roblox) adds a real sandbox and optional type hints.
   - The project lead knows Lua from Garry's Mod; Halberd plans Lua plugins, so one language covers the Rhylib family.
   - Less common among 3D artists than Python; arrays start at 1, which surprises some programmers.
2. **Python (through PyO3)**
   - The standard in 3D software: Blender, Maya, Houdini, Cinema 4D and 3ds Max all script in Python, so the largest number of artists already know it, and it has a huge library ecosystem.
   - Heavy to ship (an interpreter of tens of MB, or users must install the right version), and it **cannot be sandboxed**: a shared script can read or delete any file. Slower to start and to call into.
3. **JavaScript (QuickJS through `rquickjs`, or Boa in pure Rust)**
   - Very widely known, permissive licenses, embeddable and sandboxable.
   - Rare in 3D tools; the small engines are slower than Luau, and the fast one (V8) is very large.
4. **C#**
   - Familiar from Unity and Godot, but embedding the .NET runtime in a Rust app is heavy and awkward. Not a realistic fit.
5. **Rhai** (a scripting language made for Rust)
   - Easiest to embed and safe by design, but almost nobody knows it.

## Decision

**Lua, using the Luau dialect through `mlua`**, because it is the only option that is small, fast, safely sandboxable and already familiar to the project lead. Python's bigger audience doesn't outweigh that its scripts can't be made safe to share.

A middle path stays possible later: since every action is a named command taking plain data (DESIGN.md §4.7), a separate Python bridge could be added without changing the app's core, for people who want it and accept that Python scripts run unsandboxed.

## Consequences

- A new `script` crate (layer 2) wraps `mlua`. Scripts change the scene only through operations and commands, so they are undoable and appear in the Edit Layers stack.
- `mlua` builds Luau from C++ source, so the build needs a C++ compiler (already true for xatlas, decision pending in Phase 6). The workspace's no-`unsafe` rule applies to our code, not to dependencies.
- The scripting API is versioned so old scripts keep working as the app grows.
- Nothing is built until Phase 9; until then, the "doors to keep open" in DESIGN.md §4.7 apply.
