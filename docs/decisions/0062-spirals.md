# 0062. Spirals: bends past one turn climb clear of themselves; file format version 15

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0050](0050-pipe-bends.md) (pipe bends), [0059](0059-symmetry-for-every-edit.md) (symmetry for every edit)

## Problem

A bend stopped at 180°. Springs, coils, screw threads and spiral handrails need a bend that keeps going round. Past one full turn, a plain bend runs into itself: something must move each loop out of the way.

The project lead asked: "Won't the radius or angle decide if it goes like a spring? Otherwise it would just overlap". They chose **automatic climbing plus a number field**, for those who want fine control.

## Options

1. **Only a "Rise per turn" field:** the user works out how far the loops must climb; at 0 the loops overlap.
2. **Automatic only:** the loops climb just clear of each other; no control over how open the coil is.
3. **Automatic plus a field** (chosen): past one turn the loops climb just clear of each other, and **Rise per turn** adds more.

## Decision

1. **Angle:** with a Bend radius, the Angle goes up to **3600°** (ten turns). Without a radius it still stops at 150°, because a sharp corner can't spiral.
2. **Climbing:**
   - Past 360°, every row climbs along the turn's axis, evenly as it goes round.
   - Each full turn climbs the pipe's width along that axis plus a 5% gap (`SPIRAL_GAP`), so loops never touch.
   - **Rise per turn** (0 or more, in the scene's units) adds to that.
   - At 360° or less, the automatic climb is 0. Rise per turn alone then tilts the bend into a ramp, and with no rise, **nothing changes from before**.
3. **Segments:**
   - Past one turn, Segments counts the rows in **each full turn**, so a spiral is as smooth as one loop of it. It is continuous at 360°.
   - A whole spiral has at most 256 rows (`MAX_BEND_ROWS`), so editing it stays quick: about 0.1 s in a release build at the cap.
4. **Mirrored spirals:** a mirror turns a right-handed spiral into a left-handed one. The mirrored copy made with Symmetry on gets `Bend::mirrored`, which makes it climb the other way along its own axis, so it is the true mirror image. The flag isn't shown in the panel.
5. **The panel:**
   - A **Rise per turn** row under Segments. It is greyed out without a Bend radius, because it does nothing then.
   - The Last edit reads "Extrude 1 polygon with a spiral".
6. **File format 15:**
   - Bent extrudes are stored with their whole bend: angle, direction, radius, segments, rise and the mirrored flag. They can be stored on their own or as a mirrored edit.
   - Older files open with no rise and not mirrored, so they look the same as before.

## Consequences

- Springs and coils are one extrude, editable afterwards like any bend.
- The straight part after a spiral goes on from the last loop, the way the end faces.
- The pipe's rings stay square to the way the turn goes round, not tilted along the climb. For tightly climbing spirals the walls shear slightly; for springs this is invisible. A true tilted ring can come with the advanced sweep tools later.
- A Bend radius below the pipe's own radius still folds the inside of each turn, as with a single elbow.
