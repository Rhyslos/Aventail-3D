# fileio

**Layer 2 · Services**

Everything that touches files: `.rhy` projects, OBJ and glTF import and export, the settings file.

## Purpose

Reads and writes the outside world's files: the versioned `.rhy` project format (DESIGN.md §8, [decision 0007](../../docs/decisions/0007-rhy-project-format.md)), OBJ and glTF for Substance Painter and Godot (with deterministic triangulation and MikkTSpace tangents), and the settings file described by `config`.

## Must never

- Trust a file: every reader checks sizes and counts before allocating, rejects bad input with a plain-language error, and gets a fuzz test
- Leave a half-written file: write to a temporary file, keep the previous save as a backup, then swap it in
- Write usernames or local paths into exported files
- Know about the GPU or UI

## Depends on

`scene`, `mesh`, `uv`, `config` (planned).

## Status

Empty. Saving and loading arrive in Phase 2; export in Phase 5.
