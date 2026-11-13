//! Each starter graph's recipe.

use super::*;

/// The graph of starter `key`.
pub(in crate::starter_graphs) fn recipe(key: &str) -> Graph {
    match key {
        "bricks" => bricks(),
        "stone_tiles" => stone_tiles(),
        "cobbles" => cobbles(),
        "planks" => planks(),
        "stylised_wood" => stylised_wood(),
        "bark" => bark(),
        "fabric" => fabric(),
        "metal_plate" => metal_plate(),
        "ground" => ground(),
        "leaves" => super::leaves::leaves(),
        "trim_sheet" => trim_sheet(),
        other => panic!("no recipe for {other}"),
    }
}

fn bricks() -> Graph {
    let mut r = Recipe::new("Bricks");
    let bricks = r.add(
        "bricks",
        0,
        &[
            ("rows", whole(12)),
            ("across", whole(4)),
            ("gap", num(0.14)),
        ],
    );
    let noise = r.add(
        "noise",
        0,
        &[
            ("scale", whole(24)),
            ("detail", whole(5)),
            ("roughness", num(0.6)),
        ],
    );
    let grain = r.add("levels", 1, &[("low", num(0.3)), ("high", num(0.7))]);
    let warp = r.add("warp", 1, &[("amount", num(0.35)), ("scale", whole(12))]);
    let bevel = r.add(
        "bevel",
        2,
        &[
            ("width", num(1.0)),
            ("roundness", num(0.7)),
            ("height", num(0.7)),
        ],
    );
    let lift = r.add("blend", 3, &[("mode", choice(1)), ("amount", num(0.12))]);
    let rough = r.add("blend", 4, &[("mode", choice(1)), ("amount", num(0.12))]);
    let key = r.add("blend", 1, &[("amount", num(0.45))]);
    let mottle = r.add("blend", 2, &[("mode", choice(1)), ("amount", num(0.25))]);
    let edges = r.add("edges", 2, &[("width", num(0.8))]);
    let lit = r.add("blend", 3, &[("mode", choice(1)), ("amount", num(0.3))]);
    let colours = r.add(
        "gradient_map",
        5,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x6e675e),
                (0.15, 0xa59d8e),
                (0.42, 0x7a3426),
                (0.7, 0xa9533a),
                (1.0, 0xd88a5c),
            ]),
        )],
    );
    let material = r.add(
        "material_from_height",
        6,
        &[("depth", num(4.0)), ("roughness", num(0.85))],
    );
    let out = r.add("material_out", 7, &[("size_cm", num(90.0))]);
    // The shape.
    r.wire((bricks, "shape"), (warp, "in"));
    r.wire((warp, "out"), (bevel, "shape"));
    r.wire((bevel, "height"), (lift, "bottom"));
    r.wire((bricks, "random"), (lift, "top"));
    r.wire((lift, "out"), (rough, "bottom"));
    r.wire((noise, "height"), (grain, "in"));
    r.wire((grain, "out"), (rough, "top"));
    // The colour key: mortar black, each brick its own grey.
    r.wire((bricks, "random"), (key, "bottom"));
    r.wire((bricks, "shape"), (key, "top"));
    r.wire((key, "out"), (mottle, "bottom"));
    r.wire((grain, "out"), (mottle, "top"));
    r.wire((warp, "out"), (edges, "shape"));
    r.wire((mottle, "out"), (lit, "bottom"));
    r.wire((edges, "edges"), (lit, "top"));
    r.wire((lit, "out"), (colours, "in"));
    // The finish.
    r.wire((rough, "out"), (material, "height"));
    r.wire((colours, "colour"), (material, "colour"));
    r.wire((material, "material"), (out, "material"));
    r.show(bricks, &["rows", "across", "gap", "seed"]);
    r.show(colours, &["colours"]);
    r.show(material, &["depth"]);
    r.step(
        "1. Lay the bricks",
        MAKE,
        0,
        "Start here. Bricks lays the pattern: white bricks, black mortar. Rows and \
         Bricks per row set how many; Mortar sets the gaps. Noise, made stronger by \
         Levels, is a fine grain used twice later: a rough surface and blotchy colour.",
        &[bricks, noise, grain],
    );
    r.step(
        "2. Shape them",
        SHAPE,
        0,
        "Warp nudges the edges so the bricks look handmade. Bevel turns the flat \
         pattern into height with rounded edges. Then each brick gets its own small \
         lift (Random), and the grain roughens the surface.",
        &[warp, bevel, lift, rough],
    );
    r.step(
        "3. A grey for each part",
        COLOUR,
        1,
        "Before colouring, this builds a grey that says what each spot is: black for \
         mortar, a different grey for each brick, blotches from the grain, and lighter \
         edges (Edge finder), as a painter would pick out edges.",
        &[key, mottle, edges, lit],
    );
    r.step(
        "4. Colour",
        COLOUR,
        0,
        "Gradient map turns those greys into colours: the left end is mortar, the rest \
         brick reds. Click it and change the colours to make other bricks.",
        &[colours],
    );
    r.step(
        "5. Finish",
        FINISH,
        0,
        "Material from height makes the bumps catch the light, and adds the colour. \
         Material out says one repeat covers 90 cm, about four bricks across.",
        &[material, out],
    );
    r.finish()
}

fn stone_tiles() -> Graph {
    let mut r = Recipe::new("Stone tiles");
    let tiles = r.add(
        "tiles",
        0,
        &[
            ("across", whole(4)),
            ("down", whole(4)),
            ("gap", num(0.05)),
            ("shift", num(0.5)),
        ],
    );
    let noise = r.add("noise", 0, &[("scale", whole(8)), ("detail", whole(6))]);
    let warp = r.add("warp", 1, &[("amount", num(6.0)), ("scale", whole(5))]);
    let grain = r.add("levels", 1, &[("low", num(0.25)), ("high", num(0.75))]);
    let bevel = r.add(
        "bevel",
        2,
        &[
            ("width", num(5.0)),
            ("roundness", num(0.6)),
            ("height", num(0.55)),
        ],
    );
    let lift = r.add("blend", 3, &[("mode", choice(1)), ("amount", num(0.3))]);
    let rough = r.add("blend", 4, &[("mode", choice(1)), ("amount", num(0.25))]);
    let edges = r.add("edges", 2, &[("width", num(2.0))]);
    let key = r.add("blend", 5, &[("mode", choice(1)), ("amount", num(0.25))]);
    let colours = r.add(
        "gradient_map",
        6,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x2a2622),
                (0.3, 0x47423c),
                (0.55, 0x6d675d),
                (0.75, 0x958c7d),
                (0.9, 0xb3a994),
                (1.0, 0xd9d0bb),
            ]),
        )],
    );
    let stone = r.add(
        "material_from_height",
        7,
        &[("depth", num(5.0)), ("roughness", num(0.8))],
    );
    let clouds = r.add("clouds", 0, &[("scale", whole(4)), ("look", choice(1))]);
    let moss_colours = r.add(
        "gradient_map",
        1,
        &[(
            "colours",
            gradient(&[(0.0, 0x24361a), (0.5, 0x4c6b28), (1.0, 0x8fa53e)]),
        )],
    );
    let moss = r.add(
        "material_from_height",
        2,
        &[("depth", num(2.0)), ("roughness", num(0.95))],
    );
    let mix = r.add(
        "mix_materials",
        8,
        &[
            ("by", choice(1)),
            ("rise", num(-0.3)),
            ("softness", num(0.2)),
        ],
    );
    let out = r.add("material_out", 9, &[("size_cm", num(200.0))]);
    r.wire((tiles, "shape"), (warp, "in"));
    r.wire((noise, "height"), (grain, "in"));
    r.wire((warp, "out"), (bevel, "shape"));
    r.wire((bevel, "height"), (lift, "bottom"));
    r.wire((tiles, "random"), (lift, "top"));
    r.wire((lift, "out"), (rough, "bottom"));
    r.wire((grain, "out"), (rough, "top"));
    r.wire((warp, "out"), (edges, "shape"));
    r.wire((rough, "out"), (key, "bottom"));
    r.wire((edges, "edges"), (key, "top"));
    r.wire((key, "out"), (colours, "in"));
    r.wire((rough, "out"), (stone, "height"));
    r.wire((colours, "colour"), (stone, "colour"));
    r.wire((clouds, "height"), (moss_colours, "in"));
    r.wire((clouds, "height"), (moss, "height"));
    r.wire((moss_colours, "colour"), (moss, "colour"));
    r.wire((stone, "material"), (mix, "bottom"));
    r.wire((moss, "material"), (mix, "top"));
    r.wire((mix, "material"), (out, "material"));
    r.show(tiles, &["across", "down", "gap", "seed"]);
    r.show(warp, &["amount"]);
    r.show(mix, &["rise"]);
    r.show(colours, &["colours"]);
    r.step(
        "1. Lay the tiles",
        MAKE,
        0,
        "Start here. Tiles lays a neat grid; Shift moves every other row along. Noise is \
         the stone's rough surface.",
        &[tiles, noise],
    );
    r.step(
        "2. Cut the stones",
        SHAPE,
        0,
        "Warp wobbles the straight edges so the stones look cut by hand. Bevel rounds \
         them into height, Random lifts each stone a little, and the noise (made \
         stronger by Levels) roughens them.",
        &[warp, grain, bevel, lift, rough],
    );
    r.step(
        "3. Light edges",
        COLOUR,
        1,
        "Edge finder draws a line just inside each stone. Added to the height, it \
         makes the edges come out lighter when coloured: the way stone is painted by \
         hand.",
        &[edges, key],
    );
    r.step(
        "4. Colour the stone",
        COLOUR,
        0,
        "Gradient map colours by height: dark in the cracks, light on the tops. Try \
         warmer or cooler greys.",
        &[colours],
    );
    r.step(
        "5. Moss",
        DETAIL,
        2,
        "Billowy Clouds makes soft clumps; Gradient map colours them green. Material \
         from height makes the moss a material of its own.",
        &[clouds, moss_colours, moss],
    );
    r.step(
        "6. Finish",
        FINISH,
        0,
        "Mix materials by height lays the moss where it stands higher than the stone: \
         in the gaps. Rise moves the moss up or down. One repeat is 2 m across.",
        &[stone, mix, out],
    );
    r.finish()
}

fn cobbles() -> Graph {
    let mut r = Recipe::new("Cobbles");
    let cells = r.add(
        "cells",
        0,
        &[
            ("scale", whole(7)),
            ("jitter", num(0.85)),
            ("look", choice(2)),
        ],
    );
    let noise = r.add("noise", 0, &[("scale", whole(16)), ("detail", whole(6))]);
    let stones = r.add(
        "levels",
        1,
        &[
            ("low", num(0.03)),
            ("high", num(0.1)),
            ("invert", switch(true)),
        ],
    );
    let bevel = r.add(
        "bevel",
        2,
        &[
            ("width", num(5.0)),
            ("roundness", num(1.0)),
            ("height", num(0.8)),
        ],
    );
    let lift = r.add("blend", 3, &[("mode", choice(1)), ("amount", num(0.15))]);
    let rough = r.add("blend", 4, &[("mode", choice(1)), ("amount", num(0.1))]);
    let stone_colours = r.add(
        "gradient_map",
        1,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x6d6a66),
                (0.3, 0x8b7f70),
                (0.55, 0x707b85),
                (0.8, 0x9a907f),
                (1.0, 0xb4ab9b),
            ]),
        )],
    );
    let edge_dark = r.add(
        "levels",
        3,
        &[
            ("low", num(0.0)),
            ("high", num(0.6)),
            ("invert", switch(true)),
        ],
    );
    let shade = r.add(
        "hue_saturation",
        4,
        &[("lightness", num(-0.35)), ("saturation", num(0.9))],
    );
    let stone = r.add(
        "material_from_height",
        5,
        &[("depth", num(5.0)), ("roughness", num(0.75))],
    );
    let sand_noise = r.add("noise", 0, &[("scale", whole(48)), ("detail", whole(4))]);
    let sand_colours = r.add(
        "gradient_map",
        1,
        &[(
            "colours",
            gradient(&[(0.0, 0x6b5a43), (0.5, 0x8f7b5c), (1.0, 0xb59e78)]),
        )],
    );
    let sand = r.add(
        "material_from_height",
        2,
        &[("depth", num(1.0)), ("roughness", num(1.0))],
    );
    let mix = r.add(
        "mix_materials",
        6,
        &[
            ("by", choice(1)),
            ("rise", num(-0.4)),
            ("softness", num(0.15)),
        ],
    );
    let out = r.add("material_out", 7, &[("size_cm", num(100.0))]);
    r.wire((cells, "height"), (stones, "in"));
    r.wire((stones, "out"), (bevel, "shape"));
    r.wire((bevel, "height"), (lift, "bottom"));
    r.wire((cells, "random"), (lift, "top"));
    r.wire((lift, "out"), (rough, "bottom"));
    r.wire((noise, "height"), (rough, "top"));
    r.wire((cells, "random"), (stone_colours, "in"));
    r.wire((bevel, "height"), (edge_dark, "in"));
    r.wire((stone_colours, "colour"), (shade, "in"));
    r.wire((edge_dark, "out"), (shade, "where"));
    r.wire((rough, "out"), (stone, "height"));
    r.wire((shade, "out"), (stone, "colour"));
    r.wire((sand_noise, "height"), (sand_colours, "in"));
    r.wire((sand_noise, "height"), (sand, "height"));
    r.wire((sand_colours, "colour"), (sand, "colour"));
    r.wire((stone, "material"), (mix, "bottom"));
    r.wire((sand, "material"), (mix, "top"));
    r.wire((mix, "material"), (out, "material"));
    r.show(cells, &["scale", "jitter", "seed"]);
    r.show(stones, &["high"]);
    r.show(stone_colours, &["colours"]);
    r.show(mix, &["rise"]);
    r.step(
        "1. Stones",
        MAKE,
        0,
        "Start here. Cells with the Edges look draws the lines between irregular \
         cells: one cell for each stone. Scale sets how many; Jitter how uneven.",
        &[cells, noise],
    );
    r.step(
        "2. Shape them",
        SHAPE,
        0,
        "Levels, turned upside down, makes the cells white and the lines black: a \
         shape (Lightest widens the gaps). Bevel rounds each stone into a dome; Random \
         lifts some, Noise roughens.",
        &[stones, bevel, lift, rough],
    );
    r.step(
        "3. A colour per stone",
        COLOUR,
        1,
        "Gradient map colours Random, one grey per stone, so each stone gets its own \
         colour. Hue and saturation darkens their edges, only where Levels (the \
         bevel turned upside down) is white.",
        &[stone_colours, edge_dark, shade],
    );
    r.step(
        "4. Sand",
        DETAIL,
        2,
        "Fine Noise makes sand, coloured by a Gradient map, made into a material of its \
         own.",
        &[sand_noise, sand_colours, sand],
    );
    r.step(
        "5. Finish",
        FINISH,
        0,
        "Mix materials by height fills the gaps with sand. Rise sets how full they are. \
         One repeat is 1 m across.",
        &[stone, mix, out],
    );
    r.finish()
}

fn planks() -> Graph {
    let mut r = Recipe::new("Planks");
    let planks = r.add(
        "planks",
        0,
        &[
            ("rows", whole(6)),
            ("across", whole(2)),
            ("gap", num(0.05)),
            ("grain", whole(4)),
        ],
    );
    let bevel = r.add(
        "bevel",
        1,
        &[
            ("width", num(0.8)),
            ("roundness", num(0.5)),
            ("height", num(0.75)),
        ],
    );
    let grain = r.add("blend", 2, &[("mode", choice(1)), ("amount", num(0.12))]);
    let lift = r.add("blend", 3, &[("mode", choice(1)), ("amount", num(0.1))]);
    let key = r.add("blend", 1, &[("amount", num(0.45))]);
    let gaps = r.add("blend", 2, &[("mode", choice(3))]);
    let colours = r.add(
        "gradient_map",
        4,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x2a1a10),
                (0.12, 0x5a3a22),
                (0.4, 0x8a5a34),
                (0.7, 0xa8744a),
                (1.0, 0xc9965f),
            ]),
        )],
    );
    let material = r.add(
        "material_from_height",
        5,
        &[("depth", num(3.0)), ("roughness", num(0.6))],
    );
    let out = r.add("material_out", 6, &[("size_cm", num(120.0))]);
    r.wire((planks, "shape"), (bevel, "shape"));
    r.wire((bevel, "height"), (grain, "bottom"));
    r.wire((planks, "grain"), (grain, "top"));
    r.wire((grain, "out"), (lift, "bottom"));
    r.wire((planks, "random"), (lift, "top"));
    r.wire((planks, "random"), (key, "bottom"));
    r.wire((planks, "grain"), (key, "top"));
    r.wire((key, "out"), (gaps, "bottom"));
    r.wire((bevel, "height"), (gaps, "top"));
    r.wire((gaps, "out"), (colours, "in"));
    r.wire((lift, "out"), (material, "height"));
    r.wire((colours, "colour"), (material, "colour"));
    r.wire((material, "material"), (out, "material"));
    r.show(planks, &["rows", "across", "gap", "seed"]);
    r.show(colours, &["colours"]);
    r.show(material, &["roughness"]);
    r.step(
        "1. Boards",
        MAKE,
        0,
        "Start here. Planks lays the boards and gives three things: their shape, a \
         grey for each board (Random) and the grain.",
        &[planks],
    );
    r.step(
        "2. Shape",
        SHAPE,
        0,
        "Bevel rounds each board's edges into height. The grain is added as tiny \
         ridges, and Random lifts some boards a little, as real floors are uneven.",
        &[bevel, grain, lift],
    );
    r.step(
        "3. A grey for the colour",
        COLOUR,
        1,
        "Random and grain mixed give each board its own shade with streaks. \
         Multiplying by the bevel makes the gaps and edges dark.",
        &[key, gaps],
    );
    r.step(
        "4. Colour and finish",
        FINISH,
        0,
        "Gradient map turns the greys into wood colours; Material from height adds the \
         light. One repeat is 1.2 m across.",
        &[colours, material, out],
    );
    r.finish()
}

fn stylised_wood() -> Graph {
    let mut r = Recipe::new("Stylised wood");
    let boards = r.add(
        "planks",
        0,
        &[
            ("rows", whole(3)),
            ("across", whole(1)),
            ("gap", num(0.035)),
        ],
    );
    let stripes = r.add(
        "gradient",
        0,
        &[("direction", choice(1)), ("repeats", whole(12))],
    );
    let wobble = r.add("warp", 1, &[("amount", num(4.0)), ("scale", whole(2))]);
    let bands = r.add("levels", 2, &[("low", num(0.4)), ("high", num(0.65))]);
    let bevel = r.add(
        "bevel",
        1,
        &[
            ("width", num(2.5)),
            ("edge", choice(1)),
            ("height", num(0.8)),
        ],
    );
    let height = r.add("blend", 3, &[("mode", choice(1)), ("amount", num(0.08))]);
    let edges = r.add("edges", 1, &[("width", num(1.2))]);
    let key = r.add("blend", 3, &[("amount", num(0.35))]);
    let lit = r.add("blend", 4, &[("mode", choice(4)), ("amount", num(0.9))]);
    let gaps = r.add("blend", 5, &[("mode", choice(3))]);
    let colours = r.add(
        "gradient_map",
        6,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x3b2314),
                (0.25, 0x7a4526),
                (0.5, 0xa5602f),
                (0.75, 0xcf8a45),
                (1.0, 0xf3c27d),
            ]),
        )],
    );
    let material = r.add(
        "material_from_height",
        7,
        &[
            ("depth", num(2.0)),
            ("roughness", num(0.7)),
            ("occlusion", num(0.5)),
        ],
    );
    let out = r.add("material_out", 8, &[("size_cm", num(100.0))]);
    r.wire((stripes, "height"), (wobble, "in"));
    r.wire((wobble, "out"), (bands, "in"));
    r.wire((boards, "shape"), (bevel, "shape"));
    r.wire((bevel, "height"), (height, "bottom"));
    r.wire((bands, "out"), (height, "top"));
    r.wire((boards, "shape"), (edges, "shape"));
    r.wire((bands, "out"), (key, "bottom"));
    r.wire((boards, "random"), (key, "top"));
    r.wire((key, "out"), (lit, "bottom"));
    r.wire((edges, "edges"), (lit, "top"));
    r.wire((lit, "out"), (gaps, "bottom"));
    r.wire((boards, "shape"), (gaps, "top"));
    r.wire((gaps, "out"), (colours, "in"));
    r.wire((height, "out"), (material, "height"));
    r.wire((colours, "colour"), (material, "colour"));
    r.wire((material, "material"), (out, "material"));
    r.show(boards, &["rows", "seed"]);
    r.show(stripes, &["repeats"]);
    r.show(wobble, &["amount"]);
    r.show(colours, &["colours"]);
    r.step(
        "1. Big shapes first",
        MAKE,
        0,
        "Start here. Hand-painted textures start big and simple: a few wide boards \
         (Planks), and stripes (Gradient) for the grain.",
        &[boards, stripes],
    );
    r.step(
        "2. Grain",
        SHAPE,
        0,
        "Warp bends the stripes into flowing grain; Levels makes the bands bolder, as \
         a painter would simplify them.",
        &[wobble, bands],
    );
    r.step(
        "3. Chunky edges",
        SHAPE,
        1,
        "A sharp Bevel gives each board a clear, chunky edge. The grain is added as a \
         slight ridge.",
        &[bevel, height],
    );
    r.step(
        "4. Paint in the light",
        COLOUR,
        2,
        "In hand-painted art the light is in the colour: Edge finder paints bright \
         board edges (Lighter keeps the lighter grey), Random gives each board its own \
         shade, and the gaps go dark.",
        &[edges, key, lit, gaps],
    );
    r.step(
        "5. Colour and finish",
        FINISH,
        0,
        "Gradient map picks warm, saturated browns (hand-painted wood is often more \
         orange than real wood). Occlusion is low, as the shade is already painted.",
        &[colours, material, out],
    );
    r.finish()
}

fn bark() -> Graph {
    let mut r = Recipe::new("Bark");
    let plates = r.add(
        "planks",
        0,
        &[
            ("rows", whole(7)),
            ("across", whole(3)),
            ("gap", num(0.22)),
            ("grain", whole(5)),
        ],
    );
    let ridges = r.add(
        "clouds",
        0,
        &[
            ("scale", whole(6)),
            ("look", choice(2)),
            ("detail", whole(6)),
        ],
    );
    let upright = r.add("transform", 1, &[("turn", choice(1))]);
    let grain_up = r.add("transform", 1, &[("turn", choice(1))]);
    let warp = r.add("warp", 2, &[("amount", num(3.0)), ("scale", whole(4))]);
    let bevel = r.add(
        "bevel",
        3,
        &[
            ("width", num(2.5)),
            ("roundness", num(0.8)),
            ("height", num(0.75)),
        ],
    );
    let cracks = r.add("blend", 4, &[("mode", choice(3)), ("amount", num(0.35))]);
    let streaks = r.add("blend", 5, &[("mode", choice(1)), ("amount", num(0.12))]);
    let colours = r.add(
        "gradient_map",
        6,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x17110d),
                (0.3, 0x3d3027),
                (0.6, 0x655343),
                (0.85, 0x8a7864),
                (1.0, 0xa89880),
            ]),
        )],
    );
    let wood = r.add(
        "material_from_height",
        7,
        &[("depth", num(7.0)), ("roughness", num(0.9))],
    );
    let patches = r.add("clouds", 4, &[("scale", whole(3)), ("look", choice(1))]);
    let where_ = r.add("levels", 5, &[("low", num(0.55)), ("high", num(0.7))]);
    let lichen = r.add(
        "material_from_height",
        5,
        &[
            ("colour", colour(0x7d8a52)),
            ("depth", num(1.0)),
            ("roughness", num(0.95)),
        ],
    );
    let mix = r.add("mix_materials", 8, &[("amount", num(0.8))]);
    let out = r.add("material_out", 9, &[("size_cm", num(60.0))]);
    r.wire((plates, "shape"), (upright, "in"));
    r.wire((plates, "grain"), (grain_up, "in"));
    r.wire((upright, "out"), (warp, "in"));
    r.wire((warp, "out"), (bevel, "shape"));
    r.wire((bevel, "height"), (cracks, "bottom"));
    r.wire((ridges, "height"), (cracks, "top"));
    r.wire((cracks, "out"), (streaks, "bottom"));
    r.wire((grain_up, "out"), (streaks, "top"));
    r.wire((streaks, "out"), (colours, "in"));
    r.wire((streaks, "out"), (wood, "height"));
    r.wire((colours, "colour"), (wood, "colour"));
    r.wire((patches, "height"), (where_, "in"));
    r.wire((patches, "height"), (lichen, "height"));
    r.wire((wood, "material"), (mix, "bottom"));
    r.wire((lichen, "material"), (mix, "top"));
    r.wire((where_, "out"), (mix, "where"));
    r.wire((mix, "material"), (out, "material"));
    r.show(plates, &["rows", "across", "gap", "seed"]);
    r.show(warp, &["amount"]);
    r.show(where_, &["low"]);
    r.step(
        "1. Plates",
        MAKE,
        0,
        "Start here. Bark is made from Planks: boards with wide gaps become the \
         plates of bark. Ridged Clouds will make cracks.",
        &[plates, ridges],
    );
    r.step(
        "2. Stand them up",
        SHAPE,
        0,
        "Transform turns the boards and their grain a quarter, so they run up the \
         trunk. Warp makes the plates irregular.",
        &[upright, grain_up, warp],
    );
    r.step(
        "3. Deep furrows",
        SHAPE,
        0,
        "A wide, round Bevel makes deep furrows between the plates. The ridged clouds \
         multiply cracks into them, and the grain adds streaks.",
        &[bevel, cracks, streaks],
    );
    r.step(
        "4. Lichen",
        DETAIL,
        1,
        "Billowy Clouds makes patches; Levels picks only the biggest (Darkest sets how \
         much lichen).",
        &[patches, where_, lichen],
    );
    r.step(
        "5. Colour and finish",
        FINISH,
        0,
        "Dark in the furrows, lighter on the plates. Mix materials by shape lays the \
         lichen where Levels is white. One repeat is 60 cm.",
        &[colours, wood, mix, out],
    );
    r.finish()
}

fn fabric() -> Graph {
    let mut r = Recipe::new("Fabric");
    let ramp = |r: &mut Recipe, direction: u32| {
        r.add(
            "gradient",
            0,
            &[
                ("style", choice(1)),
                ("direction", choice(direction)),
                ("repeats", whole(8)),
            ],
        )
    };
    let across = ramp(&mut r, 0);
    let down = ramp(&mut r, 1);
    let cut_across = r.add("levels", 1, &[("low", num(0.49)), ("high", num(0.51))]);
    let cut_down = r.add("levels", 1, &[("low", num(0.49)), ("high", num(0.51))]);
    let one = r.add("blend", 2, &[("mode", choice(2))]);
    let other = r.add("blend", 2, &[("mode", choice(2))]);
    let chequer = r.add("blend", 3, &[("mode", choice(1))]);
    let thread_across = r.add(
        "gradient",
        3,
        &[("direction", choice(1)), ("repeats", whole(16))],
    );
    let thread_down = r.add("gradient", 3, &[("repeats", whole(16))]);
    let weave = r.add("blend", 4, &[]);
    let fibres = r.add("noise", 4, &[("scale", whole(64)), ("detail", whole(3))]);
    let fuzzy = r.add("blend", 5, &[("mode", choice(1)), ("amount", num(0.12))]);
    let soft = r.add("blur", 6, &[("radius", num(0.03))]);
    let colours = r.add(
        "gradient_map",
        7,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x1b2540),
                (0.45, 0x34508a),
                (0.8, 0x5576b8),
                (1.0, 0x8aa6dc),
            ]),
        )],
    );
    let material = r.add(
        "material_from_height",
        8,
        &[("depth", num(1.5)), ("roughness", num(0.95))],
    );
    let out = r.add("material_out", 9, &[("size_cm", num(6.0))]);
    r.wire((across, "height"), (cut_across, "in"));
    r.wire((down, "height"), (cut_down, "in"));
    r.wire((cut_across, "out"), (one, "bottom"));
    r.wire((cut_down, "out"), (one, "top"));
    r.wire((cut_down, "out"), (other, "bottom"));
    r.wire((cut_across, "out"), (other, "top"));
    r.wire((one, "out"), (chequer, "bottom"));
    r.wire((other, "out"), (chequer, "top"));
    r.wire((thread_across, "height"), (weave, "bottom"));
    r.wire((thread_down, "height"), (weave, "top"));
    r.wire((chequer, "out"), (weave, "where"));
    r.wire((weave, "out"), (fuzzy, "bottom"));
    r.wire((fibres, "height"), (fuzzy, "top"));
    r.wire((fuzzy, "out"), (soft, "in"));
    r.wire((soft, "out"), (colours, "in"));
    r.wire((soft, "out"), (material, "height"));
    r.wire((colours, "colour"), (material, "colour"));
    r.wire((material, "material"), (out, "material"));
    r.show(colours, &["colours"]);
    r.show(fuzzy, &["amount"]);
    r.show(material, &["depth"]);
    r.show(out, &["size_cm"]);
    r.step(
        "1. Stripes",
        MAKE,
        0,
        "Start here. Two Gradient boxes make stripes, one across and one down. Levels \
         makes their soft ramps into hard black and white stripes.",
        &[across, down, cut_across, cut_down],
    );
    r.step(
        "2. A chequer",
        SHAPE,
        0,
        "Each stripe taken from the other, and the two added: white where only one \
         set of stripes is white. That is a chequer, which says where each thread is \
         on top.",
        &[one, other, chequer],
    );
    r.step(
        "3. Weave the threads",
        SHAPE,
        1,
        "Two more Gradients are the round threads, one each way. Blend shows the \
         threads going down where the chequer is white: over, under, over, as \
         cloth is woven.",
        &[thread_across, thread_down, weave],
    );
    r.step(
        "4. Fibres",
        DETAIL,
        0,
        "Very fine Noise makes the threads fuzzy; Blur softens it all a touch.",
        &[fibres, fuzzy, soft],
    );
    r.step(
        "5. Colour and finish",
        FINISH,
        0,
        "Gradient map dyes the cloth. Real-world size is small: one repeat is only 6 cm, \
         so the threads are a sensible size on a model.",
        &[colours, material, out],
    );
    r.finish()
}

fn metal_plate() -> Graph {
    let mut r = Recipe::new("Metal plate");
    let plates = r.add(
        "tiles",
        0,
        &[
            ("across", whole(2)),
            ("down", whole(2)),
            ("gap", num(0.012)),
        ],
    );
    let dots = r.add(
        "basic_shape",
        0,
        &[
            ("size", num(0.5)),
            ("repeats", whole(12)),
            ("soft", num(0.1)),
        ],
    );
    let smudges = r.add("clouds", 0, &[("scale", whole(4)), ("detail", whole(8))]);
    let bevel = r.add(
        "bevel",
        1,
        &[
            ("width", num(2.0)),
            ("roundness", num(0.6)),
            ("height", num(0.6)),
        ],
    );
    let domes = r.add("bevel", 1, &[("width", num(2.0)), ("roundness", num(1.0))]);
    let near_edge = r.add("edges", 1, &[("width", num(6.0))]);
    let band = r.add("levels", 2, &[("low", num(0.02)), ("high", num(0.06))]);
    let rivets = r.add("blend", 3, &[("mode", choice(3))]);
    let height = r.add("blend", 4, &[("mode", choice(4)), ("amount", num(0.85))]);
    let worn = r.add("edges", 2, &[("width", num(1.0))]);
    let tone = r.add("blend", 5, &[("mode", choice(1)), ("amount", num(0.15))]);
    let key = r.add("blend", 6, &[("mode", choice(1)), ("amount", num(0.45))]);
    let marked = r.add("blend", 7, &[("mode", choice(2)), ("amount", num(0.2))]);
    let scratches = r.add(
        "scratches",
        5,
        &[
            ("spacing", num(9.0)),
            ("length", num(14.0)),
            ("width", num(0.1)),
            ("direction_vary", num(180.0)),
            ("strength_vary", num(0.85)),
            ("depth", num(0.04)),
        ],
    );
    let scratched = r.add("blend", 8, &[("mode", choice(1)), ("amount", num(0.2))]);
    let colours = r.add(
        "gradient_map",
        9,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x2a2e33),
                (0.45, 0x5f6871),
                (0.7, 0x87919b),
                (0.88, 0xb5bec6),
                (1.0, 0xf2f5f7),
            ]),
        )],
    );
    let metal = r.add(
        "material_from_height",
        10,
        &[
            ("depth", num(3.0)),
            ("roughness", num(0.35)),
            ("metal", num(1.0)),
        ],
    );
    let out = r.add("material_out", 11, &[("size_cm", num(100.0))]);
    r.wire((plates, "shape"), (bevel, "shape"));
    r.wire((dots, "shape"), (domes, "shape"));
    r.wire((plates, "shape"), (near_edge, "shape"));
    r.wire((near_edge, "edges"), (band, "in"));
    r.wire((domes, "height"), (rivets, "bottom"));
    r.wire((band, "out"), (rivets, "top"));
    r.wire((bevel, "height"), (height, "bottom"));
    r.wire((rivets, "out"), (height, "top"));
    r.wire((plates, "shape"), (worn, "shape"));
    r.wire((height, "out"), (tone, "bottom"));
    r.wire((plates, "random"), (tone, "top"));
    r.wire((tone, "out"), (key, "bottom"));
    r.wire((worn, "edges"), (key, "top"));
    r.wire((key, "out"), (marked, "bottom"));
    r.wire((smudges, "height"), (marked, "top"));
    r.wire((height, "out"), (scratches, "in"));
    r.wire((marked, "out"), (scratched, "bottom"));
    r.wire((scratches, "scratches"), (scratched, "top"));
    r.wire((scratched, "out"), (colours, "in"));
    r.wire((scratches, "out"), (metal, "height"));
    r.wire((colours, "colour"), (metal, "colour"));
    r.wire((metal, "material"), (out, "material"));
    r.show(plates, &["across", "down"]);
    r.show(dots, &["repeats", "size"]);
    r.show(colours, &["colours"]);
    r.show(metal, &["roughness"]);
    r.step(
        "1. Plates and dots",
        MAKE,
        0,
        "Start here. Tiles makes the plates, with thin seams. Basic shape makes a grid \
         of small circles: the rivets to be. Clouds will be smudges.",
        &[plates, dots, smudges],
    );
    r.step(
        "2. Rivets near the edges",
        SHAPE,
        0,
        "Bevel puffs the circles into domes and rounds the plates. Edge finder and \
         Levels make a band along each plate's edge; multiplied, only the rivets in \
         the band stay. Lighter puts them on the plates.",
        &[bevel, domes, near_edge, band, rivets, height],
    );
    r.step(
        "3. Worn edges and scratches",
        COLOUR,
        1,
        "Painted metal shows bright, worn edges. Tiles' Random gives each plate its own \
         tone, Edge finder adds the bright edges, and the clouds take a little away as \
         smudges, so it isn't too clean. Scratches scores fine lines every way into the \
         height, and adds them, bright, to the colour.",
        &[worn, tone, key, marked, scratches, scratched],
    );
    r.step(
        "4. Colour and finish",
        FINISH,
        0,
        "Gradient map gives steel greys. Metal is set to 1 and Roughness low, so it \
         shines in the realistic preview. One repeat is 1 m.",
        &[colours, metal, out],
    );
    r.finish()
}

fn ground() -> Graph {
    let mut r = Recipe::new("Ground");
    let earth = r.add("noise", 0, &[("scale", whole(6)), ("detail", whole(7))]);
    let pebbles = r.add(
        "scatter",
        0,
        &[
            ("spacing", num(20.0)),
            ("size", num(17.0)),
            ("size_vary", num(0.5)),
            ("height_vary", num(0.3)),
            ("apart", switch(true)),
        ],
    );
    let patches = r.add("clouds", 0, &[("scale", whole(3)), ("look", choice(1))]);
    let earth_colours = r.add(
        "gradient_map",
        1,
        &[(
            "colours",
            gradient(&[
                (0.0, 0x2e2117),
                (0.45, 0x4f3a28),
                (0.75, 0x6d5338),
                (1.0, 0x8a6e4c),
            ]),
        )],
    );
    let dirt = r.add(
        "material_from_height",
        2,
        &[("depth", num(3.0)), ("roughness", num(0.95))],
    );
    let uneven = r.add("warp", 1, &[("amount", num(2.5)), ("scale", whole(12))]);
    let pebble_colours = r.add(
        "gradient_map",
        1,
        &[(
            "colours",
            gradient(&[(0.0, 0x4a433b), (0.5, 0x786d5f), (1.0, 0x9c917f)]),
        )],
    );
    let stones = r.add(
        "material_from_height",
        3,
        &[("depth", num(4.0)), ("roughness", num(0.8))],
    );
    let on_top = r.add(
        "mix_materials",
        4,
        &[
            ("by", choice(1)),
            ("rise", num(-0.25)),
            ("softness", num(0.1)),
        ],
    );
    let grass_where = r.add("levels", 1, &[("low", num(0.5)), ("high", num(0.65))]);
    let grass_colours = r.add(
        "gradient_map",
        1,
        &[(
            "colours",
            gradient(&[(0.0, 0x2c3d1a), (0.5, 0x4e6b2a), (1.0, 0x7c9a3c)]),
        )],
    );
    let grass = r.add(
        "material_from_height",
        2,
        &[("depth", num(2.0)), ("roughness", num(0.95))],
    );
    let grassy = r.add(
        "mix_materials",
        5,
        &[("amount", num(0.85)), ("softness", num(0.2))],
    );
    let out = r.add("material_out", 6, &[("size_cm", num(200.0))]);
    r.wire((earth, "height"), (earth_colours, "in"));
    r.wire((earth, "height"), (dirt, "height"));
    r.wire((earth_colours, "colour"), (dirt, "colour"));
    r.wire((pebbles, "out"), (uneven, "in"));
    r.wire((pebbles, "random"), (pebble_colours, "in"));
    r.wire((uneven, "out"), (stones, "height"));
    r.wire((pebble_colours, "colour"), (stones, "colour"));
    r.wire((dirt, "material"), (on_top, "bottom"));
    r.wire((stones, "material"), (on_top, "top"));
    r.wire((patches, "height"), (grass_where, "in"));
    r.wire((patches, "height"), (grass_colours, "in"));
    r.wire((patches, "height"), (grass, "height"));
    r.wire((grass_colours, "colour"), (grass, "colour"));
    r.wire((on_top, "material"), (grassy, "bottom"));
    r.wire((grass, "material"), (grassy, "top"));
    r.wire((grass_where, "out"), (grassy, "where"));
    r.wire((grassy, "material"), (out, "material"));
    r.show(pebbles, &["spacing", "size", "seed"]);
    r.show(grass_where, &["low"]);
    r.show(earth_colours, &["colours"]);
    r.step(
        "1. Earth",
        MAKE,
        0,
        "Start here. Noise with lots of Detail makes lumpy earth; Gradient map colours \
         it, dark in the dips.",
        &[earth, earth_colours, dirt],
    );
    r.step(
        "2. Pebbles",
        DETAIL,
        1,
        "Scatter places round pebbles of different sizes, kept apart (Spacing: fewer \
         or more; Size: bigger or smaller); its Random gives each its own colour. Warp \
         makes them less round.",
        &[pebbles, pebble_colours, uneven, stones],
    );
    r.step(
        "3. Grass",
        DETAIL,
        2,
        "Billowy Clouds makes patches. Levels picks where the grass grows (Darkest: \
         less or more grass).",
        &[patches, grass_where, grass_colours, grass],
    );
    r.step(
        "4. Layer them",
        FINISH,
        0,
        "Mix materials by height puts the pebbles on the earth where they stand \
         higher. Then by shape, the grass patches go on top. One repeat is 2 m.",
        &[on_top, grassy, out],
    );
    r.finish()
}

/// Bolts, a painted panel, planks and a copper pipe stacked as the four
/// strips of a trim sheet (decision 0289).
fn trim_sheet() -> Graph {
    let mut r = Recipe::new("Trim sheet");
    // Strip 1: bolts, one round dome a repeat, shrunk into the strip.
    let dot = r.add("basic_shape", 0, &[("size", num(0.6)), ("soft", num(0.05))]);
    let dome = r.add("bevel", 1, &[("width", num(30.0)), ("roundness", num(1.0))]);
    let steel = r.add(
        "gradient_map",
        2,
        &[(
            "colours",
            gradient(&[(0.0, 0x34383d), (0.6, 0x7d868f), (1.0, 0xd7dde2)]),
        )],
    );
    let bolts = r.add(
        "material_from_height",
        3,
        &[
            ("depth", num(4.0)),
            ("roughness", num(0.35)),
            ("metal", num(1.0)),
        ],
    );
    // Strip 2: a painted panel, cut at its own size.
    let panels = r.add(
        "tiles",
        0,
        &[("across", whole(3)), ("down", whole(1)), ("gap", num(0.01))],
    );
    let panel_edge = r.add("bevel", 1, &[("width", num(2.0)), ("roundness", num(0.5))]);
    let paint = r.add(
        "gradient_map",
        2,
        &[(
            "colours",
            gradient(&[(0.0, 0x1f2a24), (0.5, 0x4d6b57), (1.0, 0x7c9b83)]),
        )],
    );
    let panel = r.add(
        "material_from_height",
        3,
        &[("depth", num(2.0)), ("roughness", num(0.55))],
    );
    // Strip 3: planks, one row of boards.
    let boards = r.add(
        "planks",
        0,
        &[
            ("rows", whole(4)),
            ("across", whole(2)),
            ("gap", num(0.05)),
            ("grain", whole(4)),
        ],
    );
    let board_edge = r.add("bevel", 1, &[("width", num(0.8)), ("roundness", num(0.5))]);
    let wood = r.add(
        "gradient_map",
        2,
        &[(
            "colours",
            gradient(&[(0.0, 0x2a1a10), (0.4, 0x8a5a34), (1.0, 0xc9965f)]),
        )],
    );
    let planks = r.add(
        "material_from_height",
        3,
        &[("depth", num(3.0)), ("roughness", num(0.6))],
    );
    // Strip 4: a copper pipe, round across, shrunk into the strip.
    let round = r.add("gradient", 0, &[("direction", choice(1))]);
    let pipe = r.add(
        "material_from_height",
        3,
        &[
            ("depth", num(6.0)),
            ("colour", colour(0xb87333)),
            ("roughness", num(0.3)),
            ("metal", num(1.0)),
        ],
    );
    // The sheet.
    let sheet = r.add(
        "trim_sheet",
        4,
        &[
            ("height2", whole(3)),
            ("height3", whole(2)),
            ("height4", whole(2)),
            ("fit1", choice(1)),
            ("fit4", choice(1)),
            ("gap", num(1.5)),
            ("edge_width", num(1.5)),
        ],
    );
    let out = r.add("material_out", 5, &[("size_cm", num(100.0))]);
    r.wire((dot, "shape"), (dome, "shape"));
    r.wire((dome, "height"), (steel, "in"));
    r.wire((dome, "height"), (bolts, "height"));
    r.wire((steel, "colour"), (bolts, "colour"));
    r.wire((panels, "shape"), (panel_edge, "shape"));
    r.wire((panel_edge, "height"), (paint, "in"));
    r.wire((panel_edge, "height"), (panel, "height"));
    r.wire((paint, "colour"), (panel, "colour"));
    r.wire((boards, "shape"), (board_edge, "shape"));
    r.wire((board_edge, "height"), (wood, "in"));
    r.wire((board_edge, "height"), (planks, "height"));
    r.wire((wood, "colour"), (planks, "colour"));
    r.wire((round, "height"), (pipe, "height"));
    r.wire((bolts, "material"), (sheet, "strip1"));
    r.wire((panel, "material"), (sheet, "strip2"));
    r.wire((planks, "material"), (sheet, "strip3"));
    r.wire((pipe, "material"), (sheet, "strip4"));
    r.wire((sheet, "material_out"), (out, "material"));
    r.show(sheet, &["gap", "depth"]);
    r.show(paint, &["colours"]);
    r.step(
        "1. Bolts",
        MAKE,
        0,
        "Start here. One round dome, made steel. In the sheet it is shrunk to fit its \
         strip, so it repeats as a row of bolts.",
        &[dot, dome, steel, bolts],
    );
    r.step(
        "2. Painted panel",
        SHAPE,
        1,
        "Tall panels with thin seams, painted green. The sheet cuts a band from them at \
         their own size, so they match the strips around them.",
        &[panels, panel_edge, paint, panel],
    );
    r.step(
        "3. Planks",
        DETAIL,
        2,
        "Four rows of boards: a strip a quarter of the sheet tall holds exactly one row.",
        &[boards, board_edge, wood, planks],
    );
    r.step(
        "4. Pipe",
        COLOUR,
        3,
        "Gradient running down, light in the middle, is a pipe's round shape; copper \
         metal. Shrunk to fit, the whole shape fills its strip.",
        &[round, pipe],
    );
    r.step(
        "5. The trim sheet",
        FINISH,
        0,
        "Each Strip becomes a band of the texture, from the top. They lie on the \
         project's trim rows (the UV Editor's Trim menu) or, with none, share the height \
         by their Heights (1, 3, 2, 2 here). Gap and bevelled edges make each read as a \
         separate piece. On a model, put strips on the rows with Put on trim row.",
        &[sheet, out],
    );
    r.finish()
}
