# 0055. The basics: hide and isolate, copies and instances, slide, split off and join; file format versions 12 and 13

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (the command list), [0052](0052-clean-up.md) (Delete in the element modes)

## Problem

Everyday modelling needs a few basics: getting things out of the way to see inside, and making copies, including copies that stay the same as their original (instances). Each needed controls and a precise meaning. So did sliding points along the surface, splitting polygons off into an object, and joining objects. The milestone came in two pull requests: parts 1 to 3 first, then 4 to 6.

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

4. **Slide** (the project lead: "Alt + Move drag").
   - In the Points, Edges and Polygons modes, **Alt + left drag on the Move gizmo** (or Transform's) slides the selected points along their edges instead of moving them freely, so they stay on the surface. Alt + left drag anywhere else still orbits.
   - Each point follows the edge to a neighbour that points most nearly the way of the drag (neighbours that aren't moving come first, so a selected loop slides along the edges crossing it), as far as the drag reaches along it, stopping at the neighbour. A point with no edge going that way stays.
   - Recorded as `Operation::SlidePoints` (each point, its neighbour and how far along, 0 to 1), with anchors for the points and then their neighbours, found again by position. Symmetry doesn't apply to a slide.
5. **Split off** (Mesh menu, Polygons mode, no key): the selected polygons of one object go into a new plain-mesh object, placed just after it, in the same place; they are deleted from the original as a recorded edit. One undo step (`Command::SplitOff`). It moves them rather than copying them; to keep them on the original too, duplicate the object first.
6. **Join objects** (the Model mode's tool menu and the Mesh menu, no key): the selected shapes go into the first of them, as one plain mesh, each part where it was (`Command::Join`). Its own edits are baked in, the others are deleted, and anything inside them takes their place. Hard edges and other values come along; a mirrored part is turned so it still faces out.
7. **File format version 13.** Version 12's file, with slides among the edits: an amount that is a real number (brought within 0 to 1), and a point never sliding towards itself.

## Consequences

- Repeated parts (bolts, panels, wheels) can be modelled once and placed many times, and changed all at once.
- The app's own clipboard note replaces whatever text was on the system clipboard.
- Hidden polygons are forgotten when the app closes; hidden objects are kept.
- Splitting off and joining are the way to reorganise a model into the parts a game engine or texturing tool needs.
- Slide follows edges only. Dragging points freely over the surface (another wish) can come later.
