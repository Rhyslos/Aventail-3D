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

/// Reads a reference image for the shape canvas (decision 0273) from the
/// bytes of a PNG, JPEG, TGA or BMP file named `name`: read as any image
/// is, then made smaller to fit within `shapes::reference::MAX_SIDE`, so
/// the project stays small.
pub fn read_reference(bytes: &[u8], name: &str) -> Result<shapes::Reference, String> {
    let image = read_image(bytes)?;
    shapes::Reference::fitted(image.width, image.height, &image.rgba, name)
        .ok_or_else(|| tr("it is damaged").to_string())
}

/// Reads the image file at `path` as a reference image for the shape
/// canvas ([`read_reference`]), named after the file.
pub fn open_reference(path: &Path) -> Result<shapes::Reference, String> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > crate::limits::MAX_UNPACKED {
        return Err(tr(crate::limits::TOO_MUCH).to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| trf!("it can't be read ({})", e))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    read_reference(&bytes, &name)
}

/// How well a photo brought in onto an Image box is packed (JPEG
/// quality, 1 to 100), best first: a photo packing to more than
/// [`PHOTO_PACKED_AIM`] bytes is packed again at the next.
const PHOTO_QUALITIES: [u8; 4] = [92, 85, 75, 60];

/// The most bytes a photo is packed into when it can be, so a graph file
/// holding one (JSON, at most 16 MB) has room for a few.
pub const PHOTO_PACKED_AIM: usize = 3 << 20;

/// Reads a photo or painting for an Image box (decision 0283) from the
/// bytes of a PNG, JPEG, TGA or BMP file named `name`: read as any image
/// is; see-through parts laid over mid grey (a material's colour has no
/// see-through parts); made smaller to fit within
/// [`matgraph::MAX_PHOTO_SIDE`] (the shape kept); packed as a JPEG (to
/// [`PHOTO_PACKED_AIM`] when it can be), and its pixels taken from that
/// packed picture, so it is the same after saving and opening again.
pub fn read_photo(bytes: &[u8], name: &str) -> Result<matgraph::Photo, String> {
    let image = read_image(bytes)?;
    let mut rgb = Vec::with_capacity(image.rgba.len() / 4 * 3);
    for p in image.rgba.chunks_exact(4) {
        let a = u32::from(p[3]);
        for c in &p[..3] {
            rgb.push(((u32::from(*c) * a + 128 * (255 - a) + 127) / 255) as u8);
        }
    }
    let mut picture = image::RgbImage::from_raw(image.width, image.height, rgb)
        .ok_or_else(|| tr("it is damaged").to_string())?;
    let most = matgraph::MAX_PHOTO_SIDE;
    if image.width > most || image.height > most {
        let scale = f64::from(most) / f64::from(image.width.max(image.height));
        let w = ((f64::from(image.width) * scale).round() as u32).clamp(1, most);
        let h = ((f64::from(image.height) * scale).round() as u32).clamp(1, most);
        picture = image::imageops::resize(&picture, w, h, image::imageops::FilterType::Triangle);
    }
    let mut packed = Vec::new();
    for quality in PHOTO_QUALITIES {
        packed = jpeg(&picture, quality)?;
        if packed.len() <= PHOTO_PACKED_AIM {
            break;
        }
    }
    photo_from_packed(packed, name)
}

/// `picture` packed as a JPEG at `quality`.
fn jpeg(picture: &image::RgbImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut packed = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut packed, quality)
        .encode_image(picture)
        .map_err(|e| trf!("it couldn't be packed ({})", e))?;
    Ok(packed)
}

/// The photo whose packed JPEG is `packed` (as brought in, or as read
/// from a project or graph file), named `name`: unpacked as any image is
/// read, and refused when it is bigger than an Image box keeps.
pub fn photo_from_packed(packed: Vec<u8>, name: &str) -> Result<matgraph::Photo, String> {
    let image = read_image(&packed)?;
    if image.width > matgraph::MAX_PHOTO_SIDE || image.height > matgraph::MAX_PHOTO_SIDE {
        return Err(tr("it is bigger than an Image box keeps").to_string());
    }
    let rgb: Vec<u8> = image
        .rgba
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    matgraph::Photo::new(image.width, image.height, rgb, packed, name).map_err(|why| tr(why).into())
}

/// Reads the image file at `path` as a photo for an Image box
/// ([`read_photo`]), named after the file.
pub fn open_photo(path: &Path) -> Result<matgraph::Photo, String> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > crate::limits::MAX_UNPACKED {
        return Err(tr(crate::limits::TOO_MUCH).to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| trf!("it can't be read ({})", e))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    read_photo(&bytes, &name)
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
    fn a_reference_image_is_made_small_enough_to_keep() {
        let side = shapes::reference::MAX_SIDE;
        let big = read_reference(&png(side * 2, 8, [50, 60, 70, 255]), "big.png").unwrap();
        assert_eq!((big.width(), big.height()), (side, 4));
        assert_eq!(&big.rgba()[..4], &[50, 60, 70, 255]);
        assert_eq!(big.name, "big.png");
        let small = read_reference(&png(3, 2, [1, 2, 3, 4]), "small.png").unwrap();
        assert_eq!((small.width(), small.height()), (3, 2));
        assert!(read_reference(b"no", "x").is_err());
        let dir = std::env::temp_dir().join(format!("reference-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("leaf.png");
        std::fs::write(&path, png(4, 4, [9, 9, 9, 255])).unwrap();
        assert_eq!(open_reference(&path).unwrap().name, "leaf.png");
        assert!(open_reference(&dir.join("missing.png")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reading_random_bytes_as_a_reference_never_crashes() {
        // A fuzz run of the reference image reader: flipped bytes in a good
        // PNG, and random bytes, read or are refused, never crash, and
        // what is read always fits.
        let good = png(16, 8, [9, 90, 200, 255]);
        let mut state = 4242u32;
        let mut next = move || {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
            state >> 8
        };
        for _ in 0..400 {
            let mut bad = good.clone();
            for _ in 0..(next() % 4 + 1) {
                let at = next() as usize % bad.len();
                bad[at] ^= 1 << (next() % 8);
            }
            if let Ok(r) = read_reference(&bad, "fuzz") {
                assert!(r.width() <= shapes::reference::MAX_SIDE);
                assert_eq!(r.rgba().len(), (r.width() * r.height() * 4) as usize);
            }
        }
        for len in 0..80 {
            let bytes: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            let _ = read_reference(&bytes, "random");
        }
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

    /// A photo-like PNG: smooth colour changes with a little grain.
    fn photo_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(width, height, |x, y| {
            let grain = ((x * 7919 + y * 104_729) % 13) as u8;
            image::Rgba([
                (x * 200 / width.max(1)) as u8 + grain,
                (y * 200 / height.max(1)) as u8 + grain,
                120 + grain,
                255,
            ])
        });
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
    fn a_photo_is_kept_packed_and_the_same_after_unpacking() {
        let photo = read_photo(&photo_png(300, 200), "wall.png").unwrap();
        assert_eq!((photo.width(), photo.height()), (300, 200));
        assert_eq!(photo.name(), "wall.png");
        assert_eq!(&photo.packed()[..2], &[0xFF, 0xD8], "a JPEG");
        let again = photo_from_packed(photo.packed().to_vec(), "wall.png").unwrap();
        assert_eq!(again, photo);
        assert_eq!(again.key(), photo.key());
        // Close to the picture brought in.
        let first = photo.rgb();
        assert!(first[0] < 12, "{}", first[0]);
    }

    #[test]
    fn a_big_photo_is_made_smaller_keeping_its_shape() {
        let most = matgraph::MAX_PHOTO_SIDE;
        let photo = read_photo(&photo_png(most * 2, 64), "long.png").unwrap();
        assert_eq!((photo.width(), photo.height()), (most, 32));
        assert!(photo.packed().len() <= PHOTO_PACKED_AIM);
    }

    #[test]
    fn see_through_parts_are_laid_over_mid_grey() {
        let photo = read_photo(&png(4, 4, [255, 0, 0, 0]), "clear.png").unwrap();
        let p = &photo.rgb()[..3];
        for c in p {
            assert!((i32::from(*c) - 128).abs() < 10, "{p:?}");
        }
    }

    #[test]
    fn unreadable_photos_are_refused_in_plain_words() {
        assert!(read_photo(b"not a picture", "x").is_err());
        assert!(read_photo(&[], "x").is_err());
        assert!(photo_from_packed(Vec::new(), "x").is_err());
        // A packed picture bigger than an Image box keeps.
        let big = image::RgbImage::new(matgraph::MAX_PHOTO_SIDE + 1, 1);
        let packed = jpeg(&big, 50).unwrap();
        let why = photo_from_packed(packed, "x").unwrap_err();
        assert!(why.contains("bigger"), "{why}");
        let dir = std::env::temp_dir().join(format!("photo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bark.png");
        std::fs::write(&path, photo_png(8, 8)).unwrap();
        assert_eq!(open_photo(&path).unwrap().name(), "bark.png");
        assert!(open_photo(&dir.join("missing.png")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reading_random_bytes_as_a_photo_never_crashes() {
        // A fuzz run of the photo reader and of unpacking a stored photo:
        // flipped bytes in a good PNG and a good JPEG, and random bytes,
        // read or are refused, never crash, and what is read always fits.
        let good_png = photo_png(16, 12);
        let good_jpeg = read_photo(&good_png, "g").unwrap().packed().to_vec();
        let mut state = 777u32;
        let mut next = move || {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
            state >> 8
        };
        for good in [&good_png, &good_jpeg] {
            for _ in 0..300 {
                let mut bad = good.clone();
                for _ in 0..(next() % 4 + 1) {
                    let at = next() as usize % bad.len();
                    bad[at] ^= 1 << (next() % 8);
                }
                let reads = [
                    read_photo(&bad, "fuzz"),
                    photo_from_packed(bad.clone(), "fuzz"),
                ];
                for p in reads.iter().flatten() {
                    assert!(p.width() <= matgraph::MAX_PHOTO_SIDE);
                    assert_eq!(p.rgb().len(), (p.width() * p.height() * 3) as usize);
                }
            }
        }
        for len in 0..80 {
            let bytes: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            let _ = read_photo(&bytes, "random");
            let _ = photo_from_packed(bytes, "random");
        }
    }
}
