# 0277. Animated boxes: Time, Scroll, Pulse and Flow, the preview's player, and "change in the game"

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 15 of Phase 12 ([the plan](../plans/phase-12.md)): materials that move (lava, water, glowing embers, conveyor belts). The project lead asked for them to move **in the app's own preview, with no engine shaders inside the app**, and moved the engine translators (Godot, Unity and Unreal, which would make them move in a game) to later, all three together. Until then animated materials export baked. Sliders a game should change while it runs are to be **marked "change in the game"** now, so the translators find them.

That raises: how time reaches the boxes without making graphs depend on a clock; what moment the painter and exports bake; how the preview plays without stalling the interface; and how each box loops or flows without visible jumps.

## Options

**How a box knows the time.** (a) A Time value kind wired into every moving box; (b) **the moment is part of the job** (chosen): `Job::time`, given by the evaluator to every box. The four moving boxes set `BoxInfo::animated` (the hook milestone 6 left), and only their cache keys include the moment, so everything not fed by a moving box is kept in the cache as time goes on. A **Time** box still exists, giving the time as flat greys (Loop, Wave) to drive any other box (a Blend's Where, Levels). Wires carry pictures only (shape, height, colour, material), so a flat grey is how a number travels.

**What moment is baked.** (a) Always 0; (b) the player's moment, whatever it is; (c) **a moment stored on the graph** (chosen): `Graph::moment` (seconds, 0 to an hour), set by **Bake this moment** under the player (`GraphEdit::SetMoment`, one undo step). The painter's graph fills, a shape given the material and every export work out graphs with no time set, so they all use the graph's moment and agree; the player's own time is the preview's and never an undo step. Option (b) would leave the painter (which works on another thread, from the saved graph) unable to agree with the preview.

**Playing without stalling.** The player (Play / Pause, Back to the start, a Time slider from 0 to 60 s, wrapping) moves its time by the interface clock (at most a quarter second a frame) and asks for a frame about every 33 ms while playing. Each frame sends the graph with the player's time to the pictures' thread (`PictureJob::time` → `Evaluator::set_time`), which takes only the newest job, at the small size used while dragging. The 3D preview lays the material at the dragging size too. A slow graph plays at fewer frames but at its real speed; the interface never waits.

**Looping and flowing without jumps.**
- **Scroll** slides by Speed (cm a second, through the real-world size) in Direction; the input wraps, so it tiles at every moment. **Loops** (on by default) rounds the distance per Loop length to whole textures across and down (at least one), so it is back at the start after each loop with no jump.
- **Pulse** multiplies a height, a colour, or a material's colour by Lowest to Highest, Rate times a second, in a Smooth (cosine), Sharp (a flash that fades) or Steps shape; with nothing plugged in it gives the strength as a flat grey.
- **Flow** drags each pixel along a flow map (red right, green up), downhill along a height's slope, or in Direction, for Stretch time seconds before starting again. Two copies half a stretch apart fade into each other, the second weighted |1 − 2·phase|, so each restarts only while invisible (the two-phase flow games' water shaders use).

**Marking "change in the game".** (a) A new kind of setting; (b) **a set of setting keys per box** (chosen), as "Shown in the painter" is: `GraphBox::in_game`, `GraphEdit::ChangeInGame`, `Graph::in_game`. Only numbers, lengths, whole numbers, colours and switches can be marked (`changes_in_game`): what a shader can take as a parameter; a choice, gradient or seed decides how the box works and is always baked (refused with why).

## Decision

- **matgraph:** `Job::with_time` / `time`; `Evaluator::set_time` / `time` (`None`: the graph's moment); `Given::time` (own boxes run their inside at the outer moment); animated boxes' keys hash the moment and the real-world size. `Graph::moment`, `is_animated`, `in_game`; `GraphParts::moment`; `GraphBox::in_game` (copied by `copy_edits`); `GraphEdit::SetMoment`, `ChangeInGame`; `MAX_MOMENT`, `usable_moment`, `changes_in_game`. `Group::Move` ("Moving", between Colour and Finish). Boxes `time`, `scroll`, `pulse`, `flow` (ids, plugs and settings as in the manual; shared `boxes::motion`). `BakeNote::Animated` names the moment; new `BakeNote::InGame` lists marked settings; `baking::seconds`.
- **fileio:** file format **103** (`project/v103.rs`): `GraphV95::moment` (none at 0) and `BoxV95::in_game` (none when empty). Older files open at 0 with nothing marked. Graph files and own box files carry both through the same model.
- **ui:** `material::player` (the player under the preview, Bake this moment), `material::game_rows` (Change in the game in Attributes; the graph's Attributes list the moment and marks); `PictureJob::time`; wide number sliders (a minimum of 0 or more, a maximum of 500 or more) are logarithmic.
- **No new keys or mouse actions:** buttons and a slider only (Space already opens the box search over the canvas).

## Consequences

- An animated material moves in the preview and on the boxes' pictures; everywhere else it is the baked moment, and the export check says so. When the translators come, they find the animated boxes (`BoxInfo::animated`) and marked sliders (`Graph::in_game`) on the graph, with nothing more to save.
- Scrolling, pulsing and flowing pictures stay seamless at every moment, and the same moment always gives the same pictures on any number of cores.
- A long animation in a heavy graph plays at few frames: the moving part and what follows it are worked out again each frame (everything before is cached). Moving those boxes to the GPU is the later step if needed.
- The player's slider runs one minute; a loop longer than that can still be baked at any moment up to an hour by setting the player there in steps (a later wish could widen it).
- Marks inside your own boxes are kept but not listed by the outer graph's export check.
