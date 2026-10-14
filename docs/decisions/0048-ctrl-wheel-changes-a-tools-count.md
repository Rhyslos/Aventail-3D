# 0048. Ctrl + wheel changes a tool's count

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0047](0047-loop-cut.md) (Loop cut's Ctrl + wheel), [0029](0029-selection.md) (Ctrl + wheel grows the selection), [0028](0028-ask-about-controls.md)

## Problem

After trying Ctrl + wheel for Loop cut's number of cuts, the project lead asked for it to be the standard for every tool from now on, starting with Bevel.

## Decision

1. **The rule.** While a tool with a count is in use, **Ctrl + wheel up adds one, down takes one away** (never below 1 or above the tool's maximum). The counts are:
   - Bevel: Segments;
   - Loop cut: Cuts;
   - Extrude: its bend's Segments ([0050](0050-pipe-bends.md));
   - tools to come: their own count, such as Bridge's segments or Slit's cuts where they have one.
2. **What it changes.** Exactly what the arrows beside that count in the Attributes panel change:
   - **Before** using the tool: its setting for the next use.
   - **While dragging**: the edit being made, at once, still one undo step.
   - **After**, while the panel shows the tool's edit as the Last edit: that edit, one undo step per notch, and the setting for the next use.
3. **The plain wheel always zooms.** Ctrl + wheel still grows and shrinks the selection with every tool that has no count. With a counted tool, grow and shrink stay in the Select menu. The Paint select brush keeps Ctrl + wheel for its size.
4. In code: `Tool::has_wheel_count` lists the tools. The viewport hands their notches over (`take_tool_steps`), and `Workbench::wheel_count` applies them. A new tool with a count adds itself to both.

## Consequences

- Every count can be changed without leaving the viewport, and it works the same way for every tool. The Attributes panel still shows and types it.
- With Bevel or Loop cut in use, Ctrl + wheel no longer grows the selection. That rarely matters while cutting, and the Select menu does it.
- Each new tool's manual page lists Ctrl + wheel when it has a count.
