# 0278. Bypass and reroute dots

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Phase 12's first extra (milestone 16, [decision 0269](0269-material-extras.md)): a way to switch a box off for a moment to see what it adds, and a way to lead long wires round boxes. The plan names the wire control (click a wire to add a bend point) but not a key for bypass.

## Options

- **Bypass:** a key (common choices: M in Blender's node editors, which mutes; Ctrl + B in Unreal) or controls on the box only. M is the shape canvas's mirror line and Ctrl + B is unused here, but neither was asked of the project lead.
- **What passes through:** only an input of exactly the output's kind; or, failing that, one that converts as a wire would (a shape as a height, greys as a colour, a material's height or colour).
- **Where dots are kept:** on each wire (a field of `Wire`, which is sorted and hashed as a key), or beside the wires, by the input each goes into (an input takes one wire).
- **Dots on a moved box:** left where they are, or moved with the boxes at the wire's ends.

## Decision

- **No key for bypass** (decision 0028: no key was asked for; the project lead can choose one later). A small **switch** on each box's title (right of its name, left of the "can't run" badge), **Bypass** / **Switch back on** in a new right-click menu on boxes (with **Remove**; on several selected boxes it does them all), and a **Bypassed** tick in Attributes beside the box's group. One undo step each (`GraphEdit::Bypass`).
- **A bypassed box doesn't run** (`matgraph::eval::bypass`). Each output passes on the first plugged input of the same kind, else the first that converts to it (`passed_through`). An output with none gives nothing; the boxes after it say why ("… is bypassed, so it only passes on what is plugged into it …", two wordings: nothing of that kind plugged in, or the box makes that itself) and offer `Fix::StopBypass`. A bypassed box with an empty needed input doesn't refuse itself; one fed by a box that can't run waits, as any box does. Material out (no outputs) can't be bypassed.
- **Its cache key** is made only from what it passes on (its kind, the size and its inputs' keys): not its settings, seed, drawing, the model or the time. So a bypassed moving box doesn't fill the cache while the preview plays (milestone 15's note), and switching bypass off and on finds both results kept. Its values are its inputs' pictures, shared, not stored again. A bypassed moving box isn't "animated" (`Graph::is_animated`, own boxes' `info`), and the export check lists bypassed boxes instead (`BakeNote::Bypassed`).
- **Drawn** dimmed, marked "Bypassed" on its picture, its switch lit in the accent (a quiet outline while on), with a wire through it from the input each output passes on.
- **Reroute dots** are kept beside the wires, by the input each wire goes into (`Graph::bends`, `GraphEdit::SetBends` sets a wire's whole list: adding, moving and removing are each one), at most `MAX_BENDS` (16) a wire. They go with their wire: when it is disconnected, replaced by a wire from another output, dissolved through or a box at either end removed. `MoveBoxes` moves the dots of wires with both ends moved.
- **Controls** as the plan says: click a wire to add a dot there (the stretch clicked decides its place in the order); drag a dot to move it (one undo step, tidied when let go); click a dot to select it (ringed) and **Delete** (or Backspace) removes it, before boxes. Pressing a wire and dragging still draws a selection box.
- **Drawn** as one curve per stretch between dots, each leaving and reaching its points level, so a bent wire still reads left to right; dots in the wire's colour.
- **The tidy layout** moves each dot with the boxes at its wire's ends, by how far across it is (a dot by the output moves with that box, one by the input with that one), then keeps it between the two plugs, never left of the dot before it: a wire never runs backwards. A tidy graph stays as it is.
- **Copied** with boxes when both ends of the wire are copied; kept inside own boxes when made and when opened back up.
- **File format 104:** `BoxV95::bypassed` (only when on) and `GraphV95::bends` (`v104::BendsV104`, only when any), read back through `Graph::from_parts`: dots of a wire not in the file dropped, too many or not real places refused as damage.

## Consequences

The graph can be compared with and without any box, at no cost while it plays, and big graphs stay readable. A key for bypass can be added later without changing anything else. Dots are kept by input: any later change letting an input take two wires would need them kept another way. Making your own box keeps the dots of the wires inside it; the wires crossing its edge lose theirs (they become new wires to its plugs).
