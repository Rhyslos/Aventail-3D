//! Painted textures on the GPU ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! one texture per texture set, with every smaller copy for far away, and
//! a bind group per texture so each material slot of a painted surface
//! can be drawn with its own.
//!
//! A texture keeps a copy of its pixels (and its smaller copies) in the
//! computer's memory, so a brush stroke uploads only the area it changed
//! and the smaller copies of just that area.

use std::collections::HashMap;

/// One painted texture.
struct PaintTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: u32,
    /// Each level's pixels (RGBA, sRGB), the full size first.
    levels: Vec<Vec<u8>>,
    /// How many of the largest levels the graphics card can't take (a
    /// texture bigger than its largest image, decision 0191): the card
    /// gets the levels from this one down.
    skip: u32,
    /// Its bind group with the placements buffer, once there is one.
    bind_group: Option<wgpu::BindGroup>,
}

/// Every painted texture by key, and a see-through stand-in for slots
/// without one.
pub(crate) struct PaintTextures {
    textures: HashMap<u64, PaintTexture>,
    blank: PaintTexture,
}

impl std::fmt::Debug for PaintTextures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintTextures")
            .field("textures", &self.textures.len())
            .finish()
    }
}

impl PaintTextures {
    pub(crate) fn new(device: &wgpu::Device, queue: Option<&wgpu::Queue>) -> Self {
        Self {
            textures: HashMap::new(),
            blank: PaintTexture::new(device, queue, 1, &[0; 4]),
        }
    }

    /// Sets texture `key` to `rgba` (`size` square), made new if its size
    /// changed. The bind group is made when the placements buffer is
    /// known.
    pub(crate) fn set(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: u64,
        size: u32,
        rgba: &[u8],
    ) {
        if size == 0 || rgba.len() != (size as usize).pow(2) * 4 {
            return;
        }
        self.textures
            .insert(key, PaintTexture::new(device, Some(queue), size, rgba));
    }

    /// Replaces the area of texture `key` from column `x`, row `y`,
    /// `width` × `height` pixels, with `rgba`, and its smaller copies.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(
        &mut self,
        queue: &wgpu::Queue,
        key: u64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        if let Some(texture) = self.textures.get_mut(&key) {
            texture.update(queue, x, y, width, height, rgba);
        }
    }

    pub(crate) fn remove(&mut self, key: u64) {
        self.textures.remove(&key);
    }

    pub(crate) fn keys(&self) -> Vec<u64> {
        self.textures.keys().copied().collect()
    }

    /// Makes the bind groups that are missing (or all of them, with
    /// `again`, after the placements buffer was replaced).
    pub(crate) fn bind(
        &mut self,
        again: bool,
        make: impl Fn(&wgpu::TextureView) -> wgpu::BindGroup,
    ) {
        for texture in self.textures.values_mut().chain([&mut self.blank]) {
            if again || texture.bind_group.is_none() {
                texture.bind_group = Some(make(&texture.view));
            }
        }
    }

    /// The bind group to draw a slot painted with texture `key` (0, or
    /// one not uploaded: the see-through stand-in).
    pub(crate) fn group(&self, key: u64) -> Option<&wgpu::BindGroup> {
        self.textures
            .get(&key)
            .filter(|_| key != 0)
            .and_then(|t| t.bind_group.as_ref())
            .or(self.blank.bind_group.as_ref())
    }
}

impl PaintTexture {
    fn new(device: &wgpu::Device, queue: Option<&wgpu::Queue>, size: u32, rgba: &[u8]) -> Self {
        let count = 32 - size.leading_zeros();
        let skip = levels_to_skip(size, device.limits().max_texture_dimension_2d);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("painted texture"),
            size: wgpu::Extent3d {
                width: (size >> skip).max(1),
                height: (size >> skip).max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: count - skip,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut levels = vec![rgba.to_vec()];
        let mut side = size;
        while side > 1 {
            let next = halved(levels.last().map_or(&[][..], Vec::as_slice), side);
            side = (side / 2).max(1);
            levels.push(next);
        }
        let out = Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture,
            size,
            levels,
            bind_group: None,
            skip,
        };
        if let Some(queue) = queue {
            for level in 0..out.levels.len() as u32 {
                let side = out.side(level);
                out.write(queue, level, 0, 0, side, side);
            }
        }
        out
    }

    /// Pixels across level `level`.
    fn side(&self, level: u32) -> u32 {
        (self.size >> level).max(1)
    }

    /// Writes the area of level `level` from its copy here to the GPU
    /// (nothing for a level the card doesn't hold).
    fn write(&self, queue: &wgpu::Queue, level: u32, x: u32, y: u32, width: u32, height: u32) {
        if level < self.skip {
            return;
        }
        let side = self.side(level);
        let pixels = &self.levels[level as usize];
        let start = ((y * side + x) * 4) as usize;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: level - self.skip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &pixels[start..],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(side * 4),
                rows_per_image: Some(side),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn update(
        &mut self,
        queue: &wgpu::Queue,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        if width == 0
            || height == 0
            || x.saturating_add(width) > self.size
            || y.saturating_add(height) > self.size
            || rgba.len() != (width * height * 4) as usize
        {
            return;
        }
        let size = self.size as usize;
        for row in 0..height as usize {
            let to = ((y as usize + row) * size + x as usize) * 4;
            let from = row * width as usize * 4;
            self.levels[0][to..to + width as usize * 4]
                .copy_from_slice(&rgba[from..from + width as usize * 4]);
        }
        self.write(queue, 0, x, y, width, height);
        // The smaller copies of just that area.
        let (mut x0, mut y0, mut x1, mut y1) = (x, y, x + width, y + height);
        for level in 1..self.levels.len() {
            let (from_side, side) = (self.side(level as u32 - 1), self.side(level as u32));
            (x0, y0) = (x0 / 2, y0 / 2);
            (x1, y1) = (x1.div_ceil(2).min(side), y1.div_ceil(2).min(side));
            let (before, rest) = self.levels.split_at_mut(level);
            let (source, target) = (&before[level - 1], &mut rest[0]);
            for ty in y0..y1 {
                for tx in x0..x1 {
                    let mut sum = [0u32; 4];
                    let mut n = 0;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (sx, sy) = (tx * 2 + dx, ty * 2 + dy);
                        if sx < from_side && sy < from_side {
                            let at = ((sy * from_side + sx) * 4) as usize;
                            for (total, value) in sum.iter_mut().zip(&source[at..at + 4]) {
                                *total += u32::from(*value);
                            }
                            n += 1;
                        }
                    }
                    let at = ((ty * side + tx) * 4) as usize;
                    for (value, total) in target[at..at + 4].iter_mut().zip(sum) {
                        *value = (total / n.max(1)) as u8;
                    }
                }
            }
            self.write(queue, level as u32, x0, y0, x1 - x0, y1 - y0);
        }
    }
}

/// How many of the largest levels of a texture `size` pixels across to
/// leave off a graphics card whose largest image is `most` pixels across.
fn levels_to_skip(size: u32, most: u32) -> u32 {
    let count = 32 - size.leading_zeros();
    let mut skip = 0;
    while (size >> skip) > most.max(1) && skip + 1 < count {
        skip += 1;
    }
    skip
}

/// Each 2 × 2 block of an image `side` pixels square averaged into one
/// pixel, as the GPU sizes smaller copies (half, rounded down; an odd last
/// row or column folded into the block before it is left out).
fn halved(rgba: &[u8], side: u32) -> Vec<u8> {
    let half = (side / 2).max(1);
    let mut out = vec![0u8; (half * half * 4) as usize];
    for y in 0..half {
        for x in 0..half {
            let mut sum = [0u32; 4];
            let mut n = 0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = (x * 2 + dx, y * 2 + dy);
                if sx < side && sy < side {
                    let at = ((sy * side + sx) * 4) as usize;
                    let pixel = rgba.get(at..at + 4).unwrap_or(&[0; 4]);
                    for (total, value) in sum.iter_mut().zip(pixel) {
                        *total += u32::from(*value);
                    }
                    n += 1;
                }
            }
            let at = ((y * half + x) * 4) as usize;
            for (value, total) in out[at..at + 4].iter_mut().zip(sum) {
                *value = (total / n.max(1)) as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textures_too_big_for_the_card_leave_off_their_largest_levels() {
        assert_eq!(levels_to_skip(4096, 8192), 0);
        assert_eq!(levels_to_skip(4096, 4096), 0);
        assert_eq!(levels_to_skip(8192, 4096), 1);
        assert_eq!(levels_to_skip(16384, 2048), 3);
        // Never every level, however small the card says it is.
        assert_eq!(levels_to_skip(4, 0), 2);
        assert_eq!(levels_to_skip(1, 0), 0);
    }

    #[test]
    fn halving_averages_blocks_and_keeps_odd_edges() {
        let rgba = [
            0, 0, 0, 0, 100, 100, 100, 100, 200, 200, 200, 200, 0, 0, 0, 0,
        ];
        assert_eq!(halved(&rgba, 2), vec![75, 75, 75, 75]);
        let three = vec![30u8; 3 * 3 * 4];
        assert_eq!(halved(&three, 3), vec![30u8; 4]);
    }
}
