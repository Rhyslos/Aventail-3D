//! Reading images painted in other programs, to use as paint layers
//! ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! PNG, JPEG, TGA and BMP, through the `image` crate. Files are hostile:
//! their size is limited before their pixels are read.

use std::path::Path;
use words::{tr, trf};

/// The widest or tallest image read, in pixels.
pub const MAX_SIDE: u32 = 16384;

/// The extensions of the image files read.
pub const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "tga", "bmp"];

/// An image read from a file: rows of pixels, four bytes each (red,
/// green, blue, alpha; sRGB colour with straight alpha), the top row
/// first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadImage {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// The pixels.
    pub rgba: Vec<u8>,
}

/// Reads the image file at `path`. The error says in plain words why
/// it couldn't be read.
pub fn open_image(path: &Path) -> Result<ReadImage, String> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > crate::limits::MAX_UNPACKED {
        return Err(tr(crate::limits::TOO_MUCH).to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| trf!("it can't be read ({})", e))?;
    read_image(&bytes)
}

/// Reads an image from the bytes of a PNG, JPEG, TGA or BMP file.
pub fn read_image(bytes: &[u8]) -> Result<ReadImage, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
    reader = reader
        .with_guessed_format()
        .map_err(|e| trf!("it can't be read ({})", e))?;
    if reader.format().is_none() {
        // TGA has no mark at its start: try it last.
        reader.set_format(image::ImageFormat::Tga);
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(u64::from(MAX_SIDE) * u64::from(MAX_SIDE) * 4);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| match e {
        image::ImageError::Limits(_) => trf!(
            "it is larger than the app reads ({} pixels across or down at most)",
            MAX_SIDE
        ),
        image::ImageError::Unsupported(_) => {
            tr("it isn't an image the app reads (PNG, JPEG, TGA or BMP)").to_string()
        }
        other => trf!("it is damaged ({})", other),
    })?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return Err(tr("it has no pixels").to_string());
    }
    Ok(ReadImage {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG of `width` × `height` pixels of one colour, made with the
    /// same crate.
    fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba(rgba));
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn a_png_reads_as_rgba() {
        let read = read_image(&png(3, 2, [10, 20, 30, 40])).unwrap();
        assert_eq!((read.width, read.height), (3, 2));
        assert_eq!(&read.rgba[..4], &[10, 20, 30, 40]);
        assert_eq!(read.rgba.len(), 3 * 2 * 4);
    }

    #[test]
    fn a_tga_reads_without_a_mark_at_its_start() {
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 100, 0, 255]));
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Tga,
            )
            .unwrap();
        let read = read_image(&bytes).unwrap();
        assert_eq!(&read.rgba[..4], &[200, 100, 0, 255]);
    }

    #[test]
    fn junk_and_empty_files_are_refused_in_plain_words() {
        assert!(read_image(&[]).is_err());
        assert!(read_image(b"not an image at all").is_err());
        let mut cut = png(64, 64, [1, 2, 3, 255]);
        cut.truncate(cut.len() / 2);
        assert!(read_image(&cut).is_err());
    }

    #[test]
    fn an_image_claiming_to_be_huge_is_refused_before_reading() {
        // A PNG header saying 100 000 × 100 000 pixels.
        let mut bytes = png(1, 1, [0; 4]);
        bytes[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert!(read_image(&bytes).is_err());
    }

    #[test]
    fn reading_random_bytes_never_crashes() {
        // A small fuzz: every one-byte change of a good PNG reads or is
        // refused, never crashes.
        let good = png(8, 8, [9, 9, 9, 255]);
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xA5;
            let _ = read_image(&bad);
        }
        let mut state = 12345u32;
        for _ in 0..200 {
            let bytes: Vec<u8> = (0..64)
                .map(|_| {
                    state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
                    (state >> 16) as u8
                })
                .collect();
            let _ = read_image(&bytes);
        }
    }
}
