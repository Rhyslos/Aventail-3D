# config

**Layer 1 · Core**

App settings: preferences, hotkeys, navigation options and their defaults.

## Purpose

Describes every setting the app has, its default and its valid range, and turns a settings file's text into checked values (and back). Includes the configurable hotkeys and options such as orbiting around the point under the cursor. Reading and writing the actual file on disk happens in `fileio`.

## Must never

- Touch the disk, the GPU or UI
- Fail to start the app: a broken or missing setting falls back to its default, with a plain-language warning

## Depends on

None (no other app crates).

## Status

Empty. Settings arrive with the first features that need them.
