//! The starter graphs ([decision 0264](../../../docs/decisions/0264-starter-graphs.md)):
//! ready-made material graphs to open and change instead of starting from
//! an empty canvas. Each is a graph file shipped with the app
//! (`crates/fileio/starters/*.matgraph`), read through the same checked
//! reader as any graph file (`crate::graph_file`), with a picture for its
//! card (`*.png`, made from the graph by [`card_picture`]).
//!
//! Opening one copies it into the project, where it is changed freely;
//! the starter itself never changes.
//!
//! The files are made from recipes in the tests (`starter_graphs/recipes.rs`):
//! run `UPDATE_STARTERS=1 cargo test -p fileio starter` to write them
//! again after changing a recipe; the tests fail while a file and its
//! recipe differ.

use crate::graph_file::{GraphFile, read_graph_file};
use matgraph::MaterialMaps;
use words::text;

/// How many pixels across a starter's card picture is.
pub const CARD_SIDE: u32 = 128;

/// The size a starter is worked out at for its card: twice the card, so
/// the card is smoothed.
pub const CARD_WORKED_OUT: u32 = 2 * CARD_SIDE;

/// One starter graph: its name, what it teaches, its graph file and its
/// card's picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Starter {
    /// Its key: the files' names.
    pub key: &'static str,
    /// Its name (English; show it through the language layer).
    pub name: &'static str,
    /// What it teaches, in a sentence (English; through the language
    /// layer).
    pub teaches: &'static str,
    /// Its graph file's text.
    pub file: &'static str,
    /// Its card's picture: a PNG file's bytes.
    pub card: &'static [u8],
}

macro_rules! starter {
    ($key:literal, $name:expr, $teaches:expr) => {
        Starter {
            key: $key,
            name: $name,
            teaches: $teaches,
            file: include_str!(concat!("../starters/", $key, ".matgraph")),
            card: include_bytes!(concat!("../starters/", $key, ".png")),
        }
    };
}

/// Every starter graph, in the order shown: the simplest first.
pub const STARTERS: [Starter; 9] = [
    starter!(
        "bricks",
        text!("Bricks"),
        text!(
            "A pattern box, Bevel to round its edges, and a Gradient map to colour it by its \
             height: the three steps most materials start with."
        )
    ),
    starter!(
        "stone_tiles",
        text!("Stone tiles"),
        text!("Warp makes neat tiles hand-cut, and Mix materials by height lays moss in the gaps.")
    ),
    starter!(
        "cobbles",
        text!("Cobbles"),
        text!(
            "Cells gives a cell for each stone; Bevel rounds them, Random gives each its own \
             colour, and sand fills the gaps."
        )
    ),
    starter!(
        "planks",
        text!("Planks"),
        text!(
            "Planks gives boards, their grain and a grey per board: three outputs coloured and \
             mixed into one floor."
        )
    ),
    starter!(
        "stylised_wood",
        text!("Stylised wood"),
        text!(
            "A hand-painted look: big shapes, strong grain lines and light painted on the \
             edges, as a game artist would paint it."
        )
    ),
    starter!(
        "bark",
        text!("Bark"),
        text!(
            "Planks turned upright and warped become bark plates; ridged Clouds adds the \
             cracks in them."
        )
    ),
    starter!(
        "fabric",
        text!("Fabric"),
        text!(
            "A woven pattern made only from smooth stripes: Gradient boxes, a chequer from two \
             sets of stripes, and Blend."
        )
    ),
    starter!(
        "metal_plate",
        text!("Metal plate"),
        text!(
            "Plates with rivets near their edges, a shiny metal finish, and worn, bright edges \
             from Edge finder."
        )
    ),
    starter!(
        "ground",
        text!("Ground"),
        text!(
            "Layers of noise make earth; pebbles from Cells sit on top, mixed in by height, \
             with grassy patches from Clouds."
        )
    ),
];

impl Starter {
    /// The starter with key `key`.
    pub fn find(key: &str) -> Option<&'static Starter> {
        STARTERS.iter().find(|s| s.key == key)
    }

    /// Its graph and own boxes, read and checked; or why it can't be
    /// read (only for a damaged build).
    pub fn load(&self) -> Result<GraphFile, String> {
        read_graph_file(self.file.as_bytes())
    }

    /// Its card's picture, read: `CARD_SIDE` square, four bytes a pixel
    /// (sRGB, alpha), the top row first.
    pub fn card_picture(&self) -> Result<crate::image_file::ReadImage, String> {
        crate::image_file::read_image(self.card)
    }
}

/// A material's card picture, `side` pixels across, from a material
/// worked out at a whole multiple of it (each card pixel the average
/// over its square): the colour lit from the top left by the normal,
/// darker where the occlusion is, with a little shine where it is smooth
/// or metal. Three bytes a pixel (sRGB), the top row first.
pub fn card_picture(material: &MaterialMaps, side: u32) -> Vec<u8> {
    let size = material.colour.size().max(1);
    let side = side.clamp(1, size);
    let step = (size / side).max(1);
    let light = glam::Vec3::new(-0.5, 0.6, 0.65).normalize();
    let half = (light + glam::Vec3::Z).normalize();
    let mut out = Vec::with_capacity((side * side * 3) as usize);
    for y in 0..side {
        for x in 0..side {
            let mut sum = glam::Vec3::ZERO;
            for dy in 0..step {
                for dx in 0..step {
                    let (px, py) = (i64::from(x * step + dx), i64::from(y * step + dy));
                    let colour = glam::Vec3::from(material.colour.at(px, py));
                    let normal =
                        glam::Vec3::from(material.normal.at(px, py)).normalize_or(glam::Vec3::Z);
                    let rough = material.roughness.at(px, py);
                    let metal = material.metal.at(px, py);
                    let open = material.occlusion.at(px, py);
                    let diffuse = normal.dot(light).max(0.0);
                    let power = 4.0 + (1.0 - rough) * 50.0;
                    let shine = normal.dot(half).max(0.0).powf(power) * (1.0 - rough) * 0.5;
                    let tint = glam::Vec3::ONE.lerp(colour, metal);
                    let lit =
                        colour * (0.35 + 0.75 * diffuse) * (0.6 + 0.4 * open) * (1.0 - 0.3 * metal)
                            + tint * shine;
                    sum += lit;
                }
            }
            let c = sum / (step * step) as f32;
            for v in [c.x, c.y, c.z] {
                out.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
    }
    out
}

#[cfg(test)]
mod recipes;
#[cfg(test)]
mod tests;
