# 0279. Find and minimap

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Phase 12's second extra (milestone 17, [decision 0269](0269-material-extras.md)): in a big material graph, a way to find a box by name and go to it, and a small map in a corner showing the whole graph. The plan names both but no keys or mouse actions.

## Options

- **Opening the search:** a key (Ctrl + F is the common one; it is unused on the canvas, but no key was asked of the project lead), or a field that is always on the canvas bar.
- **What is searched:** box names only; or also your own boxes' names, comment frames' titles and notes, the choice a box is set to ("Star", "Multiply") and the names of pieces on a Shape box's canvas. Number settings were left out: "0.35" is not something anyone searches for.
- **Where matching lives:** in the interface, or in `matgraph` beside the graph (testable without a window, and usable by a later library browser).
- **The minimap's extent:** the graph alone (the part on screen can fall off the map), or the graph and the part on screen together.
- **Dragging on the map:** the map keeps changing as the view moves (it slides under the pointer), or keeps its place until the drag ends.

## Decision

- **No key** (decision 0028; the project lead can choose one, Ctrl + F say, later). The **Find in the graph** field is always on the right of the canvas bar, with the count ("2 of 5"), **‹** and **›** for the previous and next match, and the bar's new **View** menu beside it. **Find a box in the graph…** in the command search puts the cursor in the field. A box's right-click menu gains **Find others like it** (the boxes of its kind). Inside the field, Enter goes to the next match and Esc empties it (both are the field's own keys, not new canvas keys); Esc is taken before the rest of the app sees it, as egui lets go of the field first.
- **Matching** is `matgraph::Graph::find(registry, query)` (`graph/find.rs`): every word typed must appear (any case, any order, part words too) in one of a box's name (a standard box's plain name, your own box's name through the project's registry, the id of a kind from a newer app), the choice each choice setting is set to (the standard one counts), a piece's name on its shape canvas, or a comment frame's title or a line of its note. Matches (`Found`: the box or frame, why it matched, its name, the words that matched) come in reading order, left to right as the graph flows then top to bottom, at most `MAX_FOUND` (200). Only the graph shown is searched (inside your own box, the boxes inside).
- **Going to a match** selects it (a box, or a frame for Attributes) and frames it on the canvas as Z does; nothing in the graph changes and no undo step is made. Typing goes to the first match; the list under the field (shown while typing, closed by a press elsewhere) goes to any; the matches are ringed in pale yellow on the canvas and coloured on the map. When the graph changes the same match stays chosen.
- **The minimap** sits in the canvas's bottom right corner, at most 200 × 136 points (and a quarter of the canvas each way), on canvases at least 320 × 220. It maps the graph (boxes, frames, reroute dots) with room round it, together with the part on screen, so the white rectangle showing that part is always on it. It shows frames in their colours, wires in their kinds' colours through their reroute dots (straight lines between the points the canvas's curves pass through), boxes as small grey blocks (selected ones in the accent, matches in yellow). It **hides itself while the whole graph fits on screen**; **View → Minimap** switches it off (it is on at first; not saved between runs, as the canvas's pan and zoom aren't).
- **Clicking or dragging on the map** centres the view on that place, keeping the zoom. While a drag goes on the map keeps its place (`Canvas::map_drag`), so it doesn't slide under the pointer. The map is an overlay: the canvas under it takes no press.
- **The View menu** also has **Frame the whole graph** and **Frame the selected boxes** (Z).
- **Not on the shape canvas:** its drawing is one texture square, which it shows whole; a map would add little.

## Consequences

Big graphs (the starters, your own) can be searched and crossed quickly, with nothing new to learn: a field, two arrows and a map. A key for finding can be added later without changing anything else. `Graph::find` is ready for the library browser (milestone 29) to search inside graphs. The canvas bar is fuller: on narrow panels the field is the first thing to be squeezed. No file format change.
