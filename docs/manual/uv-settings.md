# Settings of the automatic UV tools

> Every choice the automatic UV tools make is a setting you can change, and the same model and settings always give exactly the same result.

![The UV menu, grouped into Seams, Turn and flip, Lay out and Pieces, then Texture, Trim sheet and Check for baking, then Settings. Settings is open, listing Seam suggestions, Unwrapping, Relaxing, Packing and Mending, with Unwrapping open: the four methods, Straighten strips when unwrapping, Strip corners within 25° of square, and Unwrap as seams change.](../screenshots/2026-10-16-uv-settings.png)

## When to use it

The app does a lot of UV work by itself:

- [Suggest seams](seams.md) picks where to cut;
- [Unwrap](unwrapping.md) lays pieces flat, and cuts closed pieces open;
- [Relax](unwrapping.md) evens UVs out;
- [Pack](packing.md) arranges pieces in the square;
- [mending](uvs-and-modelling.md) lays out again the UVs a modelling edit breaks.

Each of them follows rules: how sharp a corner must be to get a seam, where cuts hide, how square a strip must be. In many programs these rules are hidden, and some tools give a different answer each time you run them. Here every rule is a setting, and nothing is ever random. Run a tool twice on the same model with the same settings and you get exactly the same UVs. That means you can try a setting, undo, and try another, knowing that only your change made the difference.

Change these settings when an automatic result is close to what you want but not quite: seams on the wrong side, a strip that didn't straighten, edits mending too much or too little.

## How to use it

1. Open **UV → Settings** (on the main menu bar, or under the **UV** button at the start of the [UV Editor](uv-editor.md)'s menu line): it holds five submenus, one for each automatic tool. Or, in the **UV Edit** layout, click the **UV settings** tab beside Attributes: the same five, as sections you can fold open, so they stay in view while you work.
2. Point at one to see its settings, and change them. Each is explained when you point at it.
3. Run the tool again (or Ctrl + Z and run it again) to see the difference.

Edits you've already made keep the settings they were made with, so changing a setting never changes your earlier work. The one exception is **Mending**, which belongs to the project and applies to every edit in it.

A piece can also have settings of its own (method, straightening, texel density) that win over these, or be locked so no automatic tool changes it: see [Locking pieces and piece settings](locked-pieces.md).

## Settings

| Submenu | Settings | Where explained |
|---|---|---|
| **Seam suggestions** | **Cut corners sharper than** (66°), **Cut along hard edges** (on), **Stretch budget for every piece** (off; 2×), **Hide cuts** (underneath and at the back; underneath; at the back; anywhere, the shortest cuts) | [Seams](seams.md#settings) |
| **Unwrapping** | **Method** (keep shapes, keep shapes smoother, keep sizes, automatic), **Straighten strips when unwrapping** (on), **Strip corners within** (25° of square), **Unwrap as seams change** (on) | [Unwrap, Relax and Straighten](unwrapping.md#settings) |
| **Relaxing** | **Strength** (60 steps) | [Unwrap, Relax and Straighten](unwrapping.md#settings) |
| **Packing** | **Padding** (8 px), **Turn pieces to fit**, **Stack identical pieces**, **A texture per material**, **A texture per object**, **Fit** (fast, standard, best) | [Pack](packing.md#settings) |
| **Mending** | **Mend UVs after each edit** (on), **Mend when stretched** (3×), **Mend when sized off** (3×) | [UVs while you keep modelling](uvs-and-modelling.md#settings) |

The first four are remembered by the app between sessions. Mending is saved with the project, so anyone opening it sees the same UVs; changing it is one undo step.

## Shortcuts

None: the settings are in **UV → Settings**.

## Tips and common problems

- **Problem:** I changed a setting and nothing happened. **Why:** settings apply the next time a tool runs; earlier edits keep theirs. **Fix:** run the tool again (Ctrl + Z first to replace the last run).
- **Problem:** Suggest seams puts cuts on the side I see most. **Why:** "the back" is the object's +Y, away from the Front view. **Fix:** choose another **Hide cuts**, or turn the object so its back faces away from the Front view.
- **Problem:** after changing Mending, UVs changed all over the project. **Why:** mending applies to every edit, so every shape is worked out again. **Fix:** that's the point. Ctrl + Z puts it back if you don't like it.
- **Tip:** for a prop seen from all round (a ball, a barrel on a table), **Hide cuts → Anywhere (shortest cuts)** gives the least cutting.

## Related

- [Seams](seams.md)
- [Unwrap, Relax and Straighten](unwrapping.md)
- [Pack](packing.md)
- [UVs while you keep modelling](uvs-and-modelling.md)
- [Pinning UVs](pins.md)
- [Seam preferences and stretch budgets](seam-preferences.md)
