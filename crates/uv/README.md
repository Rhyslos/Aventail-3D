# uv

**Layer 1 · Core**

UV mapping: seams, unwrapping, relaxing, straightening and packing.

## Purpose

Turns a mesh's surface into flat UV islands and arranges them in the texture square: seams from hard edges, conformal unwrapping (LSCM), relax, straighten, packing with padding, texel density. See DESIGN.md §7.

## Must never

- Know about scenes, undo, files, the GPU or UI
- Change the mesh's shape: it only works out UV coordinates and seams
- Hang or allocate without bound on degenerate input (zero-area faces, islands with no area)

## Depends on

`mesh`, `glam`.

## Status

- **Seams** (`seams`, [decision 0098](../../docs/decisions/0098-seams.md)): `pieces` (the polygons left joined once seams are cut), `is_disk` (whether a piece lies flat in one go: its Euler characteristic once cut open is 1), and `suggest` (seams for a whole shape: the hard edges given and corners sharper than `CORNER_ANGLE`, then each piece cut to a disk: a piece with no border opened along the least seen path from its lowest point to its highest, then a spanning tree of its polygons keeping the most visible edges joined, the rest cut and trimmed back; "seen" is higher facing up or to the front, across flat stretches and on outward corners). Tests in `seams/tests.rs`: disks and non-disks, a cube's twelve edges, a cylinder's one line, a sphere's bottom-to-top cut at the back, every primitive, random starting cuts, empty and stray input.

- **Unwrapping** (`unwrap`, [decision 0099](../../docs/decisions/0099-unwrap-relax-straighten.md)): `unwrap` (least squares conformal maps through `solve`'s sparse conjugate gradients; pieces that can't lie flat cut open with `seams::disk_cuts` and the cut marked; strips of square quads laid out as straight grids; each piece turned to its smallest box, a quarter turn to match how it lay, sized to the object's texel density, put where it was; pieces over `SOLVE_UP_TO` wedges projected flat), `straighten` (the grid layout alone) and `relax` (springs towards real side lengths, the chosen polygons' corners only). `chart` holds a piece's wedges (a point once per side of a cut). Tests in `unwrap/tests.rs`: a flat grid stays an even grid where it was, a box's sides keep their real sizes, a cylinder's side comes out straight, a ball without folds and a closed one cut open, quarter turns kept, only the asked pieces change, straightening and relaxing, and odd input; an ignored benchmark (`unwrap_benchmark`) times growing balls.

- **Texel density** (`density`, [decision 0100](../../docs/decisions/0100-texture-size-and-texel-density.md)): `areas` (UV and real areas of polygons), `density` (texture widths per unit of length) and `set_density` (each piece holding the polygons scaled about its middle to a density). Tests in the module.

- **Packing** (`pack`, [decision 0101](../../docs/decisions/0101-packing.md)): `pack` places pieces by shape on a grid of cells (`Grid`: rows of bits; each piece drawn, its inside and every cell its sides cross, grown by half the padding), lowest then leftmost, turned to its smallest box and tried a quarter turn round, at the largest common scale (found by halving); `PackOptions` (padding, rotate, stack, cells); identical pieces stacked (a half turn allowed). `islands_of`, `piece_of`, `place`. Tests in `pack/tests.rs`: padding kept and the square filled, a small piece nesting in a frame's hole, stacking, turning, nothing to pack, exact placing; an ignored benchmark (`pack_benchmark`).

The basics every part of the app reads (UVs per corner, islands, seam marks, moving UVs) are in `mesh::uvs`, and the primitives' ready-made UVs in `mesh::primitives`.
