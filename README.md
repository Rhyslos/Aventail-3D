# Aventail 3D

A free, open-source 3D modelling and UV unwrapping tool for **hard-surface game assets**, written in Rust.

Model it, unwrap it, export it to Godot and Substance Painter, all in one calm, tool-rich app.

> **Status: early development.** There's nothing to download yet. The plan, design and roadmap are in place, and the first builds are on their way.

## Why

Aventail 3D is built for people who loved the modelling workflows of classic 3D packages like Cinema 4D and 3ds Max, but don't have access to them anymore and never got along with the alternatives. It takes inspiration from how those tools *felt* to use: a clean, calm viewport, deep modelling tools and a powerful UV unwrapper, and builds its own thing from there.

It's not affiliated with, endorsed by or based on code from Maxon or Autodesk.

## What it's for

- **Hard-surface modelling** for game assets: props, weapons, vehicles, environment pieces
- **UV unwrapping**: seams, unwrapping, packing, texel density
- **Clean export** to glTF and OBJ for Godot and Substance Painter
- **Never losing work**: effectively unlimited undo, and base object settings that stay editable after you've modelled on top of them

Rendering, sculpting and texturing are left to the tools that already do them well. Rigging and animation are planned for much later.

## Roadmap

See [DESIGN.md](DESIGN.md) for the full design and the phase-by-phase roadmap. In short:

0. App skeleton and layout
1. Viewport and navigation
2. Objects, primitives, undo, saving (`.rhy` project files)
3. Editable meshes and selection
4. Core modelling tools
5. Export pipeline
6. UV tools
7. Edit Layers stack and other workflow tools
8. Advanced hard-surface tools

## Documentation

The user manual lives in [docs/manual](docs/manual/SUMMARY.md) and grows with every feature. New to 3D, or rusty? Start with [Concepts](docs/manual/concepts.md). What's changed recently is in the [changelog](docs/CHANGELOG.md).

## Contributing

Ideas, bug reports and feedback are very welcome through Issues. Code contributions aren't being accepted at the moment.

## License

Aventail 3D is free software, licensed under the [GNU General Public License v3.0 or later](LICENSE). Anything you make with it is yours.

Part of the **Rhylib** family of tools.
