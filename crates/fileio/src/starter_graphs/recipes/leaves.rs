//! The Leaves starter (milestone 14, decision 0276; left over from
//! milestone 8): a leaf drawn once on the shape canvas, with a serrated
//! edge, a stem and veins, grouped; linked copies of it, each varied; laid
//! out on a leaf sheet and cut out as an alpha card.

use super::*;
use shapes::{Change, Drawing, Edge, EdgeKind, Knot, Path, PieceId, PieceKind, Vary, Vein};

/// How many leaves are on the sheet.
const LEAVES: usize = 6;

/// One leaf drawn upright in the middle of the canvas: its outline (two
/// curved sides from the stem to the tip), its stem, and a midrib with
/// three pairs of side veins.
fn leaf() -> (Drawing, Vec<PieceId>) {
    let v = Vec2::new;
    let outline = Path::new(
        vec![
            // The tip: a point, its sides leaving it curved.
            Knot {
                at: v(0.5, 0.1),
                inn: v(-0.03, 0.1),
                out: v(0.03, 0.1),
            },
            Knot::smooth(v(0.665, 0.48), v(0.0, 0.16)),
            // The base, rounding into the stem.
            Knot {
                at: v(0.5, 0.86),
                inn: v(0.1, -0.01),
                out: v(-0.1, -0.01),
            },
            Knot::smooth(v(0.335, 0.48), v(0.0, -0.16)),
        ],
        true,
    );
    let stem = Path::polygon([v(0.49, 0.84), v(0.51, 0.84), v(0.512, 0.97), v(0.488, 0.97)]);
    let line = |a: Vec2, b: Vec2| Path::new(vec![Knot::corner(a), Knot::corner(b)], false);
    let mut d = Drawing::new()
        .with_piece("Leaf", outline, PieceKind::Add)
        .and_then(|d| d.with_piece("Stem", stem, PieceKind::Add))
        .and_then(|d| d.with_piece("Midrib", line(v(0.5, 0.84), v(0.5, 0.16)), PieceKind::Line))
        .expect("the leaf draws");
    for (i, y) in [0.68, 0.53, 0.38].into_iter().enumerate() {
        for side in [-1.0, 1.0] {
            let name = format!("Vein {}", 2 * i + usize::from(side > 0.0) + 1);
            let reach = 0.12 - 0.02 * i as f32;
            let tip = v(0.5 + side * reach, y - 0.11);
            d = d
                .with_piece(&name, line(v(0.5, y), tip), PieceKind::Line)
                .expect("a vein draws");
        }
    }
    let ids: Vec<PieceId> = d.pieces().iter().map(|p| p.id).collect();
    let (leaf, stem) = (ids[0], ids[1]);
    let d = d
        .with_colour(leaf, rgb(0x5f9a3c))
        .and_then(|d| d.with_colour(stem, rgb(0x6f7d32)))
        .and_then(|d| {
            d.with_edge(
                leaf,
                Edge {
                    kind: EdgeKind::Serrated,
                    amount: 0.45,
                    size: 0.013,
                    seed: 7,
                },
            )
        })
        .and_then(|d| {
            d.with_vary(
                leaf,
                Vary {
                    on: true,
                    size: 0.25,
                    angle: 0.35,
                    colour: 0.35,
                },
            )
        })
        .expect("the leaf's look");
    let d = ids[2..].iter().fold(d, |d, id| {
        let width = if *id == ids[2] { 0.009 } else { 0.005 };
        d.with_vein(
            *id,
            Vein {
                width,
                ..Vein::STANDARD
            },
        )
        .expect("a vein's look")
    });
    let (d, _) = d.with_group(&ids, "Leaf").expect("grouped");
    (d, ids)
}

/// The leaf made small, and linked copies of it set apart on the canvas
/// (apart, so each is a card of its own on the sheet).
fn leaves_drawing() -> Drawing {
    let (d, ids) = leaf();
    let d = d
        .with_change(&ids, &Change::scaled(Vec2::new(0.5, 0.5), Vec2::splat(0.3)))
        .and_then(|d| d.with_change(&ids, &Change::moved(Vec2::new(-0.33, -0.25))))
        .expect("made small");
    let copies = d.copied(&ids);
    let mut d = d;
    for k in 1..LEAVES {
        let by = Vec2::new((k % 3) as f32 * 0.33, (k / 3) as f32 * 0.5);
        d = d
            .with_pasted(&copies, true, &Change::moved(by))
            .expect("a linked copy")
            .0;
    }
    d
}

pub(super) fn leaves() -> Graph {
    let mut r = Recipe::new("Leaves");
    let shape = r.add(
        "drawn_shape",
        0,
        &[
            ("layout", choice(1)),
            ("gap", num(0.03)),
            ("puff", num(1.2)),
            ("outline", num(0.3)),
        ],
    );
    r.draw(shape, leaves_drawing());
    let blotches = r.add(
        "clouds",
        0,
        &[("scale", whole(10)), ("detail", whole(4))],
    );
    let mottle = r.add(
        "hue_saturation",
        1,
        &[("hue", num(-8.0)), ("lightness", num(-0.12))],
    );
    let material = r.add(
        "material_from_height",
        2,
        &[("depth", num(1.5)), ("roughness", num(0.55))],
    );
    let card = r.add(
        "opacity",
        3,
        &[("translucency", num(0.6)), ("thick", num(0.6))],
    );
    let out = r.add("material_out", 4, &[("size_cm", num(40.0))]);
    r.wire((shape, "colours"), (mottle, "in"));
    r.wire((blotches, "height"), (mottle, "where"));
    r.wire((shape, "height"), (material, "height"));
    r.wire((mottle, "out"), (material, "colour"));
    r.wire((material, "material"), (card, "material"));
    r.wire((shape, "mask"), (card, "shape"));
    r.wire((card, "material_out"), (out, "material"));
    r.show(shape, &["gap", "puff"]);
    r.show(material, &["depth"]);
    r.show(card, &["translucency"]);
    r.step(
        "1. Draw the leaves",
        MAKE,
        0,
        "Double-click Shape to open its canvas. One leaf is drawn (outline, stem, \
         veins as lines), grouped, with a serrated edge. The others are linked \
         copies: change one and all change, but Vary copies gives each its own size, \
         turn, colour and teeth. Layout: Leaf sheet lays them out apart.",
        &[shape, blotches],
    );
    r.step(
        "2. Colour",
        COLOUR,
        0,
        "The leaves' own colours, darker along the veins, with blotches from \
         Clouds: a little darker and yellower here and there.",
        &[mottle],
    );
    r.step(
        "3. Material",
        FINISH,
        0,
        "Material from height makes the bumps: rounded leaves with sunken veins.",
        &[material],
    );
    r.step(
        "4. Cut out",
        FINISH,
        0,
        "Opacity cuts the card out by the Mask: the game shows only the leaves. \
         Translucency lets light through them from behind. Export textures… makes \
         an alpha card for the engine chosen.",
        &[card, out],
    );
    r.finish()
}
