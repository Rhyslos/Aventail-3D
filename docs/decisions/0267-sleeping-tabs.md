# 0267. Sleeping tabs

- **Status:** Accepted
- **Date:** 2026-11-08

## Problem

With scene tabs ([decision 0266](0266-scene-tabs.md)) every open project keeps everything ready to draw: its meshes and painted textures on the graphics card, the four views' images, and on the processor's side the shapes after their modifiers, the paint's mixes and the maps made for graph fills. A few heavy projects open at once can fill the graphics card. The project lead asked (step 3 of [docs/plans/scene-tabs.md](../plans/scene-tabs.md)) for a tab that is left to sleep **straight away, provided switching back doesn't stutter**: put away what is costly, keep everything needed to wake exactly as left, measure waking on a heavy scene, and sleep after a short delay or in the background instead if waking would stutter visibly.

## What waking costs (measured)

`cargo test --release -p render --test gpu tab_wake_check -- --ignored --nocapture` (the GPU tests' software driver, llvmpipe, on the build machine: 2 processor cores; the processor's numbers are the ones that carry over to a real computer) and the heavy-mesh check's new row:

| Waking | Time |
|---|---|
| Making a renderer again (shaders, pipelines) | 70–90 ms |
| 40 objects of 5,000 polygons (200,000 in all): turning them into GPU geometry, on the window's thread | 91 ms |
| … and uploading them | 13 ms |
| One object of 3,000,000 polygons: its GPU geometry, in the background | 2,400 ms |
| … and uploading it, on the window's thread | 170 ms |
| The interface's side for that object (the drawing worked out again; heavy-mesh check, release build) | 0.4 ms |

What sleeping frees: about 18 MB on the graphics card for the light scene, 240 MB for the heavy one, plus each view's images (77 MB for a 1280 × 800 view with smoothing; up to four views) and the paint's textures and mixes.

So waking even a middling project holds the window for about a tenth of a second (six frames), and a heavy one shows its shape only after a couple of seconds, with a hitch of ten frames as it is uploaded. That is a visible stutter: waking straight away for every tab left would make quick back-and-forth switching feel worse than without sleeping.

## Options

1. **Sleep straight away** for every tab left: least memory, but the stutter above on every switch back to a big project.
2. **Sleep after a short delay** (the standard), straight away as a setting: switching back and forth is instant; tabs left a while free their memory and wake with the measured pause.
3. **Keep the GPU geometry on the processor's side while asleep**, so waking only uploads: saves the background work, but the memory moves rather than goes, and the upload hitch stays.
4. **Wake in the background before the tab is used** (on hovering it, say): guesswork, and the work is done for tabs never used.

## Decision

Option 2.

- **When:** Settings → Interface → Projects → **Tabs sleep**: **Straight away**, **After 2 minutes** (the standard), **After 10 minutes**, **After 30 minutes**, **Never** (`ViewOptions::tab_sleep`, a drop-down list; five choices don't fit side by side in the window). The tab in use never sleeps.
- **Low memory:** when less than a tenth of the computer's memory, or less than 768 MB, is free for programs (Windows: `GlobalMemoryStatusEx`; Linux: `/proc/meminfo`'s `MemAvailable`; elsewhere unknown, so never low), every tab left sleeps at once, whatever the delay, except with **Never**. The app looks at most every ten seconds, and only while a tab could sleep. The graphics card's own memory can't be asked about portably, so it isn't watched.
- **What is put away:** the tab's renderer lets go of its meshes, outline shells, painted textures, checker image, the views' images (taken off egui's list) and the Material preview's (`ViewportRenderer::sleep`); shapes being turned into GPU geometry are left to finish and thrown away. The workbench lets go of the scene drawing's summary and the shapes after cuts, modifiers and levels of detail (their threads finish and are thrown away), the paint's mixes, the graph fills' maps and evaluator (a fill under way is cancelled, and made again on waking as its layer still isn't made from the current graph), the Material mode's pictures (its thread stops) and its preview pictures, and a bake preview being made (cancelled; made again on waking).
- **What is kept:** the document (scene, undo history, selection), the views and camera, the mode and tool, the panel layouts, the file and its unsaved changes, the backups' timer, the picking index, and the maps baked by hand with Bake maps (slow to make and not saved). The renderer's shaders and pipelines are kept too: they are small, and making them again cost a tenth of a second, so waking only uploads the scene. (This does what sharing the pipelines between tabs would do for waking; tabs still make their own when they open.)
- **Never asleep while busy:** a tab whose save or export, Bake maps, File → Bake or Export textures is under way, or whose closing is being asked about, stays awake and is looked at again every two seconds; it sleeps once that is done.
- **Waking** is using the tab again (clicking it, or the app bringing it to the front to ask about closing or quitting): the next frame works out what was put away again from the document. Heavy shapes are made ready in the background, with the view's "preparing" note, as when a project is opened.
- **The moon:** a sleeping tab shows a crescent moon before its name (after the unsaved dot, if it has one). Pointing at it (or the tab) says why it sleeps ("You left it more than 2 minutes ago", "The computer was running low on memory"), about how much memory was freed, and that nothing is lost: its unsaved changes are kept safe (and backed up as usual), with its undo history, selection, view and mode.
- **Backups and emergency copies** carry on for a sleeping tab: they are made from the document, which is kept, and leaving a tab already backed up its changes.

## Consequences

- Switching between a few tabs is as quick as before; tabs left for two minutes give back their graphics card memory and most of what they worked out.
- Coming back to a heavy project left a while waits for its shapes as opening it does (a couple of seconds for millions of polygons), with a short hitch as they are uploaded.
- Settings has one more choice; people who switch often between heavy projects can choose a longer delay or Never, and people short of memory Straight away.
- `windows-sys` gains its `Win32_System_SystemInformation` feature (MIT or Apache-2.0, as before) for the memory question; the one `unsafe` call sits in `app::platform`'s Windows part with the others decision 0026 allows.
