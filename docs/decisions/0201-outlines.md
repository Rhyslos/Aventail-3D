# 0201. Outlines

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-second milestone ([0177](0177-texture-painter-plan.md)), and one the project lead called "almost a priority" with the toon preview (DESIGN.md, texturing choices). Cartoon games draw an ink line round objects. DESIGN.md asks for one switch per object: a live outline in the viewport; on export an inverted-hull outline as real geometry that works in every engine, and a ready-made Godot shader; with the width able to vary by area.

## Decision

- **Per object** (`scene::Outline` on `Object::outline`: width in centimetres, 0.05 to 50, standard 1; colour, standard a warm near-black). Set in the **Attributes** panel's Basic grid: **Outline** (a switch), then **Width** and **Colour** while on. `Command::SetOutline` sets, changes or removes it; a drag is one undo step; a width out of range is clamped, one that isn't a number refused.
- **In the viewport:** an *inverted hull*. The renderer keeps a shell per outlined mesh (`render::OutlineGeometry`: each point once, pushed out along the area-weighted average of its polygons' directions, so hard edges leave no gaps), uploaded when the shape changes. The outline pipeline draws it after the faces, pushed out by the width in the world, and leaves out its outside faces in the shader (so mirrored placements work too). It is drawn wherever the faces are.
- **On export:** the same shell as an object beside each outlined one, `Name-outline`, turned inside out (polygons reversed, normals in), in a material named for its colour (`outline-rrggbb`), in glTF and OBJ. Engines cull back faces, so only its rim shows: no shader needed. The export window's counts include it.
- **For Godot:** a glTF export with Godot's names also writes `name-outline.gdshader` beside it: unshaded, front faces culled, pushed out along the normals by the width (metres), for a material's Next Pass. Its first line names the app, so a later export may replace it (`engine::may_replace` reads `//` comments too).
- **No keys:** a switch in Attributes.
- **Saving:** project format 76 stores the outline with the object's development settings.

## Consequences

- Cartoon outlines in one click, the same in the viewport and every engine.
- An outlined object costs about twice its triangles in the game. The manual says so.
- **Width varying by area** is left for later: it needs per-point data (vertex colours, milestone 26, or a painted mask), so it comes with vertex colour painting. A single width per object is what most props need.
- The shell isn't a child node in the glTF; it sits beside the object at the same pivot. Engine-specific setups (a Unity or Unreal shader) can follow with the live engine preview.
