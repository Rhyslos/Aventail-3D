//! Square images split into tiles ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)).
//!
//! An image is `size` pixels across and down, four bytes a pixel (red,
//! green, blue, alpha; sRGB colour with straight alpha), split into
//! [`TILE`]-pixel square tiles. A tile nobody has painted is left out and
//! reads as fully transparent. Tiles are shared ([`Arc`]): copying an image
//! copies only pointers, and a change copies just the tiles it touches, so
//! undo and saving stay cheap even at 4K.
//!
//! Row 0 is the top of the texture, where V is 1, as in image files.

use std::sync::Arc;

/// Pixels across one tile.
pub const TILE: u32 = 64;

/// Bytes in one tile's pixels.
pub const TILE_BYTES: usize = (TILE * TILE * 4) as usize;

/// The smallest and largest image sizes (as the texture settings'
/// sizes, `scene::TextureSettings`).
pub const SIZES: std::ops::RangeInclusive<u32> = 16..=16384;

/// One tile's pixels: [`TILE`] rows of [`TILE`] pixels, four bytes each.
/// Pixels past the image's edge (in the last row and column of tiles of
/// an image whose size isn't a multiple of [`TILE`]) stay transparent.
pub type Tile = Arc<Vec<u8>>;

/// A square image in tiles.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    size: u32,
    tiles: Vec<Option<Tile>>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("size", &self.size)
            .field("painted_tiles", &self.painted_tiles())
            .finish()
    }
}

impl Image {
    /// A fully transparent image `size` pixels across, or `None` if the
    /// size is outside [`SIZES`].
    pub fn new(size: u32) -> Option<Self> {
        if !SIZES.contains(&size) {
            return None;
        }
        let across = tiles_across(size) as usize;
        Some(Self {
            size,
            tiles: vec![None; across * across],
        })
    }

    /// Pixels across (and down).
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Tiles across (and down).
    pub fn tiles_across(&self) -> u32 {
        tiles_across(self.size)
    }

    /// How many tiles the image has, painted or not.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Tile `index` (row by row from the top left), if painted.
    pub fn tile(&self, index: usize) -> Option<&Tile> {
        self.tiles.get(index).and_then(Option::as_ref)
    }

    /// Every tile, painted or not, row by row from the top left.
    pub fn tiles(&self) -> &[Option<Tile>] {
        &self.tiles
    }

    /// Puts `tile` at `index` (`None` clears it). A tile of the wrong
    /// length, or an index past the end, is ignored.
    pub fn set_tile(&mut self, index: usize, tile: Option<Tile>) {
        if tile.as_ref().is_some_and(|t| t.len() != TILE_BYTES) {
            return;
        }
        if let Some(slot) = self.tiles.get_mut(index) {
            *slot = tile;
        }
    }

    /// How many tiles have been painted.
    pub fn painted_tiles(&self) -> usize {
        self.tiles.iter().flatten().count()
    }

    /// Whether nothing is painted.
    pub fn is_empty(&self) -> bool {
        self.tiles.iter().all(Option::is_none)
    }

    /// Roughly how many bytes the painted tiles take.
    pub fn memory_size(&self) -> usize {
        self.painted_tiles() * TILE_BYTES + self.tiles.len() * size_of::<Option<Tile>>()
    }

    /// The pixel at column `x`, row `y`, or transparent black off the
    /// image or where nothing is painted.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.size || y >= self.size {
            return [0; 4];
        }
        let (index, offset) = self.place(x, y);
        match &self.tiles[index] {
            Some(tile) => [
                tile[offset],
                tile[offset + 1],
                tile[offset + 2],
                tile[offset + 3],
            ],
            None => [0; 4],
        }
    }

    /// Sets the pixel at column `x`, row `y` (off the image: ignored).
    /// The tile is copied first if it is shared.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.size || y >= self.size {
            return;
        }
        let (index, offset) = self.place(x, y);
        let tile = self.tiles[index].get_or_insert_with(|| Arc::new(vec![0; TILE_BYTES]));
        Arc::make_mut(tile)[offset..offset + 4].copy_from_slice(&rgba);
    }

    /// The tile index and the byte offset in it of pixel (`x`, `y`).
    fn place(&self, x: u32, y: u32) -> (usize, usize) {
        let across = self.tiles_across();
        let index = (y / TILE) * across + x / TILE;
        let offset = ((y % TILE) * TILE + x % TILE) * 4;
        (index as usize, offset as usize)
    }

    /// The pixel area tile `index` covers: its first column and row, and
    /// its width and height within the image.
    pub fn tile_area(&self, index: usize) -> (u32, u32, u32, u32) {
        let across = self.tiles_across() as usize;
        let x = (index % across) as u32 * TILE;
        let y = (index / across) as u32 * TILE;
        (x, y, TILE.min(self.size - x), TILE.min(self.size - y))
    }

    /// The whole image as rows of pixels, four bytes each, top row first.
    pub fn to_rgba(&self) -> Vec<u8> {
        let size = self.size as usize;
        let mut out = vec![0; size * size * 4];
        for (index, tile) in self.tiles.iter().enumerate() {
            let Some(tile) = tile else { continue };
            let (x, y, w, h) = self.tile_area(index);
            for row in 0..h as usize {
                let from = row * TILE as usize * 4;
                let to = ((y as usize + row) * size + x as usize) * 4;
                out[to..to + w as usize * 4].copy_from_slice(&tile[from..from + w as usize * 4]);
            }
        }
        out
    }

    /// An image `size` pixels across showing `rgba` (`width` × `height`
    /// pixels, four bytes each, top row first), stretched to fill it with
    /// smooth (bilinear) sampling. Tiles left fully transparent are left
    /// out. `None` if the size is outside [`SIZES`] or the pixels don't
    /// match the width and height.
    pub fn from_rgba(size: u32, width: u32, height: u32, rgba: &[u8]) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if width == 0 || height == 0 || rgba.len() != expected {
            return None;
        }
        let mut image = Self::new(size)?;
        let sample = |sx: f32, sy: f32| -> [u8; 4] {
            // Pixel centres: the source pixel (i, j) sits at (i + 0.5, j + 0.5).
            let fx = (sx - 0.5).clamp(0.0, (width - 1) as f32);
            let fy = (sy - 0.5).clamp(0.0, (height - 1) as f32);
            let (x0, y0) = (fx.floor() as u32, fy.floor() as u32);
            let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            let at = |x: u32, y: u32, c: usize| rgba[((y * width + x) * 4) as usize + c] as f32;
            let mut out = [0u8; 4];
            for (c, value) in out.iter_mut().enumerate() {
                let top = at(x0, y0, c) * (1.0 - tx) + at(x1, y0, c) * tx;
                let bottom = at(x0, y1, c) * (1.0 - tx) + at(x1, y1, c) * tx;
                *value = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
            }
            out
        };
        let (scale_x, scale_y) = (width as f32 / size as f32, height as f32 / size as f32);
        for index in 0..image.tiles.len() {
            let (x, y, w, h) = image.tile_area(index);
            let mut tile = vec![0u8; TILE_BYTES];
            let mut any = false;
            for row in 0..h {
                for column in 0..w {
                    let px = sample(
                        (x + column) as f32 * scale_x + scale_x * 0.5,
                        (y + row) as f32 * scale_y + scale_y * 0.5,
                    );
                    any |= px[3] != 0;
                    let offset = ((row * TILE + column) * 4) as usize;
                    tile[offset..offset + 4].copy_from_slice(&px);
                }
            }
            if any {
                image.tiles[index] = Some(Arc::new(tile));
            }
        }
        Some(image)
    }
}

/// Tiles across an image `size` pixels across.
pub fn tiles_across(size: u32) -> u32 {
    size.div_ceil(TILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_image_is_transparent_and_has_no_tiles() {
        let image = Image::new(256).unwrap();
        assert_eq!(image.tiles_across(), 4);
        assert_eq!(image.tile_count(), 16);
        assert!(image.is_empty());
        assert_eq!(image.pixel(10, 10), [0; 4]);
        assert_eq!(image.to_rgba().len(), 256 * 256 * 4);
    }

    #[test]
    fn sizes_outside_the_range_are_refused() {
        assert!(Image::new(8).is_none());
        assert!(Image::new(32768).is_none());
        assert!(Image::new(16).is_some());
    }

    #[test]
    fn setting_a_pixel_paints_only_its_tile() {
        let mut image = Image::new(256).unwrap();
        image.set_pixel(70, 3, [10, 20, 30, 255]);
        assert_eq!(image.pixel(70, 3), [10, 20, 30, 255]);
        assert_eq!(image.painted_tiles(), 1);
        assert!(image.tile(1).is_some());
        // Off the image: nothing happens.
        image.set_pixel(256, 0, [1, 1, 1, 1]);
        assert_eq!(image.pixel(256, 0), [0; 4]);
    }

    #[test]
    fn a_copy_shares_tiles_until_one_changes() {
        let mut image = Image::new(128).unwrap();
        image.set_pixel(1, 1, [255; 4]);
        let mut copy = image.clone();
        assert!(Arc::ptr_eq(image.tile(0).unwrap(), copy.tile(0).unwrap()));
        copy.set_pixel(2, 2, [9; 4]);
        assert!(!Arc::ptr_eq(image.tile(0).unwrap(), copy.tile(0).unwrap()));
        assert_eq!(image.pixel(2, 2), [0; 4], "the original is untouched");
    }

    #[test]
    fn a_size_not_a_multiple_of_a_tile_has_partial_edge_tiles() {
        let mut image = Image::new(100).unwrap();
        assert_eq!(image.tiles_across(), 2);
        assert_eq!(image.tile_area(3), (64, 64, 36, 36));
        image.set_pixel(99, 99, [1, 2, 3, 4]);
        let rgba = image.to_rgba();
        assert_eq!(&rgba[(99 * 100 + 99) * 4..][..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn importing_keeps_an_image_of_the_same_size_exactly() {
        let size = 64u32;
        let rgba: Vec<u8> = (0..size * size)
            .flat_map(|i| [(i % 256) as u8, (i / 256) as u8, 7, 255])
            .collect();
        let image = Image::from_rgba(size, size, size, &rgba).unwrap();
        assert_eq!(image.to_rgba(), rgba);
    }

    #[test]
    fn importing_stretches_a_small_image_smoothly() {
        // Two pixels side by side, black and white, made 16 across: a
        // ramp from black to white with the ends kept.
        let rgba = [0, 0, 0, 255, 255, 255, 255, 255];
        let image = Image::from_rgba(16, 2, 1, &rgba).unwrap();
        assert_eq!(image.pixel(0, 0)[0], 0);
        assert_eq!(image.pixel(15, 0)[0], 255);
        let middle = image.pixel(8, 5)[0];
        assert!(middle > 60 && middle < 200, "{middle}");
        // Alpha stays opaque, so every tile is kept.
        assert_eq!(image.painted_tiles(), 1);
    }

    #[test]
    fn importing_refuses_pixels_that_dont_match() {
        assert!(Image::from_rgba(64, 2, 2, &[0; 15]).is_none());
        assert!(Image::from_rgba(64, 0, 0, &[]).is_none());
        assert!(Image::from_rgba(4, 1, 1, &[0; 4]).is_none());
    }

    #[test]
    fn transparent_imports_leave_no_tiles() {
        let image = Image::from_rgba(128, 4, 4, &[0; 64]).unwrap();
        assert!(image.is_empty());
    }

    #[test]
    fn tiles_of_the_wrong_length_are_ignored() {
        let mut image = Image::new(64).unwrap();
        image.set_tile(0, Some(Arc::new(vec![0; 10])));
        assert!(image.is_empty());
        image.set_tile(5, Some(Arc::new(vec![0; TILE_BYTES])));
        assert!(image.is_empty());
    }
}
