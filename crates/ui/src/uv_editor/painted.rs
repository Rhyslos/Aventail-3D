//! The painted texture behind the UVs ([decision 0177](../../../../docs/decisions/0177-texture-painter-plan.md)):
//! while the viewport shows the Painted surface, the editor shows the
//! texture set of the material its first shape's first polygon uses,
//! updated tile by tile as it changes.

use super::UvEditor;
use crate::paint_view::PaintedTexture;
use scene::Document;
use std::sync::Arc;

/// A painted texture as shown here: which one, the image last given to
/// egui, and egui's handle.
#[derive(Clone)]
pub(super) struct PaintedShown {
    key: u64,
    image: paint::Image,
    handle: egui::TextureHandle,
    /// Texels of the image per texel shown: more than 1 for a texture
    /// bigger than the graphics card takes in one piece (decision 0191).
    step: u32,
}

impl std::fmt::Debug for PaintedShown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintedShown")
            .field("key", &self.key)
            .finish()
    }
}

impl UvEditor {
    /// Whether a painted texture is shown behind the UVs.
    #[cfg(test)]
    pub(crate) fn shows_painted(&self) -> bool {
        self.painted.is_some()
    }

    /// The texture set's image to show behind the UVs, from `textures`,
    /// sent to egui (only the tiles that changed since last time).
    pub(super) fn painted_texture(
        &mut self,
        ctx: &egui::Context,
        doc: &Document,
        textures: &[PaintedTexture],
    ) -> Option<egui::TextureId> {
        let material = self.shown_material(doc)?;
        let texture = textures.iter().find(|t| t.material == material)?;
        let image = &texture.image;
        // A texture bigger than the graphics card takes is shown smaller.
        let most = ctx.input(|i| i.max_texture_side).max(64) as u32;
        let mut step = 1;
        while image.size() / step > most && step < paint::TILE {
            step *= 2;
        }
        if let Some(shown) = &mut self.painted
            && shown.key == texture.key
            && shown.image.size() == image.size()
            && shown.step == step
        {
            for (index, (old, new)) in shown.image.tiles().iter().zip(image.tiles()).enumerate() {
                let same = match (old, new) {
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    (None, None) => true,
                    _ => false,
                };
                if !same {
                    let (x, y, w, h) = image.tile_area(index);
                    let pixels = if step == 1 {
                        tile_image(new.as_deref(), w, h)
                    } else {
                        reduced(image, (x, y, w, h), step)
                    };
                    shown.handle.set_partial(
                        [(x / step) as usize, (y / step) as usize],
                        pixels,
                        egui::TextureOptions::LINEAR,
                    );
                }
            }
            shown.image = image.clone();
            return Some(shown.handle.id());
        }
        let size = image.size() as usize;
        let whole = if step == 1 {
            egui::ColorImage::from_rgba_unmultiplied([size, size], &image.to_rgba())
        } else {
            reduced(image, (0, 0, image.size(), image.size()), step)
        };
        let handle = ctx.load_texture("painted texture", whole, egui::TextureOptions::LINEAR);
        let id = handle.id();
        self.painted = Some(PaintedShown {
            key: texture.key,
            image: image.clone(),
            handle,
            step,
        });
        Some(id)
    }

    /// The name of the material the first shown shape's first polygon
    /// uses.
    fn shown_material(&self, doc: &Document) -> Option<String> {
        let scene = doc.scene();
        let id = *super::shown_objects(doc).first()?;
        let shape = scene.evaluated(id)?;
        let face = shape.mesh.faces().next()?.0;
        let slot = mesh::material::slot_of(&shape.mesh, face) as usize;
        let list = scene.material_list(id);
        list.get(slot).or(list.first()).map(|m| m.name.clone())
    }
}

/// The area `x`, `y`, `w` × `h` of `image` shown `step` times smaller:
/// each texel shown the average of `step` × `step` (by coverage).
fn reduced(
    image: &paint::Image,
    (x, y, w, h): (u32, u32, u32, u32),
    step: u32,
) -> egui::ColorImage {
    let (cw, ch) = (w.div_ceil(step), h.div_ceil(step));
    let mut rgba = Vec::with_capacity((cw * ch * 4) as usize);
    for sy in 0..ch {
        for sx in 0..cw {
            let mut sum = [0u32; 4];
            let mut count = 0;
            for dy in 0..step {
                for dx in 0..step {
                    let (px, py) = (x + sx * step + dx, y + sy * step + dy);
                    if px >= x + w || py >= y + h {
                        continue;
                    }
                    let p = image.pixel(px, py);
                    let a = u32::from(p[3]);
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]) * a;
                    }
                    sum[3] += a;
                    count += 1;
                }
            }
            let a = sum[3].max(1);
            rgba.extend_from_slice(&[
                (sum[0] / a) as u8,
                (sum[1] / a) as u8,
                (sum[2] / a) as u8,
                (sum[3] / count.max(1)) as u8,
            ]);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([cw as usize, ch as usize], &rgba)
}

/// A tile's `w` × `h` pixels (rows of [`paint::TILE`] pixels) as an egui
/// image, transparent for a tile nobody painted.
fn tile_image(tile: Option<&Vec<u8>>, w: u32, h: u32) -> egui::ColorImage {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    if let Some(tile) = tile {
        for row in 0..h as usize {
            let from = row * paint::TILE as usize * 4;
            let to = row * w as usize * 4;
            rgba[to..to + w as usize * 4].copy_from_slice(&tile[from..from + w as usize * 4]);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba)
}
