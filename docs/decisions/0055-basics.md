# 0055. The basics: hide and isolate, copies and instances; file format version 12

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (the command list), [0052](0052-clean-up.md) (Delete in the element modes)

## Problem

Everyday modelling needs a few basics: getting things out of the way to see inside, and making copies, including copies that stay the same as their original (instances). Each needed controls and a precise meaning. Slide, split and join follow in the second part of this milestone.

## Decision

1. **Hide, isolate, show everything** (the project lead's keys: "Ctrl + H isolate, Shift + H hides, Alt + H shows everything").
   - **Shift + H** hides the selection; **Ctrl + H** isolates it (hides everything else); **Alt + H** shows everything again. The mode palette's **Show only the selection** button does Ctrl + H.
   - **In the Model mode** they work on objects, through each object's Visible switch: saved with the project, and undone like any change. Hidden objects are unselected first, so no gizmo sits on something you can't see. Isolating keeps the groups holding the selection shown.
   - **In the Points, Edges and Polygons modes** they work on polygons of the edited objects: the selected polygons, or those touching the selected points or edges. Hidden polygons are a view setting, like the camera: not drawn, not picked or selected (Select all takes only what is shown), not saved and not undone. Hiding unselects them, so no tool changes what can't be seen. Alt + H in the element modes shows the polygons; in the Model mode, the objects and the polygons.
   - The status bar says how many polygons are hidden. Opening another project shows everything again.
2. **Copies and instances** (the project lead's keys: "Ctrl + C and Ctrl + V for duplication, and Alt + C and Alt + V for instances", and the E menu).
   - **Ctrl + C** (or **Alt + C**) copies the selected objects; **Ctrl + V** pastes copies, **Alt + V** instances. What is copied is *which* objects: pasting copies them as they are at that moment, each placed just after its original in the Objects panel, in the same place in the scene.
   - **Duplicate** and **Instance** in the Model mode's tool menu (the empty places beside Make editable) and the Edit menu do both steps at once.
   - A copy takes everything: its settings, edits and what is inside it. It is named like the original, with a number.
   - An **instance** (`ObjectKind::Instance`) shows another object's final shape, wherever it is put and however it is turned or scaled. Changing the original changes every instance. An instance has no edits of its own (recording one is refused with a plain explanation); **Make editable** gives it its own copy of the shape. An instance of an instance shows the same original. Copying a group as instances makes a group of instances.
   - Deleting an original gives its instances their own copy of its shape first, in the same undo step, so they don't vanish.
   - The window only hears about Ctrl + V while the system clipboard holds text, so copying puts a short note there; pasting that note in the viewport pastes the objects, and other text is ignored. Text fields keep their own copy and paste.
   - Copy and paste work on objects only (in the Model mode). Copying points, edges and polygons may come later.
3. **File format version 12.** An instance is stored in the object list as a group (the kind version 1 knows) and listed beside it with the object it shows, so the object list keeps its shape. On opening, every instance must show an object that is there, with a shape of its own. Recorded edits are version 11's.

## Consequences

- Repeated parts (bolts, panels, wheels) can be modelled once and placed many times, and changed all at once.
- The app's own clipboard note replaces whatever text was on the system clipboard.
- Hidden polygons are forgotten when the app closes; hidden objects are kept.
