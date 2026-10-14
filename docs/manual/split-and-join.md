# Split off and join

> Move polygons into an object of their own, or join several objects into one.

How a model is cut into objects matters: each object can be hidden, moved, textured and exported on its own. **Split off** takes part of a shape out into a new object. **Join objects** does the opposite, making several shapes one.

## When to use it

- **Split off:** a part you modelled as one shape should be its own object: a lid, a door, a panel you'll want to texture or animate separately.
- **Join:** parts that belong together should be one object, before welding or bridging between them (those work inside one object), or before export.

## How to use it

### Split off

1. Select the object, press **3** for the Polygons mode, and select the polygons.
2. Pick **Mesh → Split off**.

The polygons go into a new object, just below the original in the Objects panel, in exactly the same place, and it becomes the selection. They're deleted from the original, which keeps everything else, including its settings. One **Ctrl + Z** puts it all back.

To copy polygons out instead of moving them, duplicate the object first (**Ctrl + C**, **Ctrl + V**), then split off from the copy.

### Join objects

1. In the **Model** mode, select the shapes, the one to keep first.
2. Press **E** and pick **Join objects** (or use **Mesh → Join objects**).

They become one plain mesh in the first object you selected, each part staying where it was. The others are deleted (anything inside them takes their place in the Objects panel). Hard edges come along.

## Settings

None.

## Shortcuts

| Action | Shortcut |
|---|---|
| Split off | Mesh menu (Polygons mode) |
| Join objects | Tool menu `E` (Model mode), or the Mesh menu |

## Tips and common problems

- **Tip:** after joining, use [Weld](connect-bridge-weld.md) or [Bridge](connect-bridge-weld.md) to connect the parts: they only work within one object.
- **Problem:** Split off says to select polygons on one object at a time. **Why:** each split makes one new object from one shape. **Fix:** split each object separately.
- **Problem:** after joining, the shape's settings (size, segments) are gone. **Why:** a joined shape is a plain mesh. **Fix:** Ctrl + Z, if you still need them.
- **Problem:** Join says to select two or more shapes. **Why:** groups have no shape of their own. **Fix:** select the shapes inside them.

## Related

- [Copies and instances](copies-and-instances.md)
- [The Objects panel](objects.md)
- [Clean-up](clean-up.md): Delete leaves a hole instead.
