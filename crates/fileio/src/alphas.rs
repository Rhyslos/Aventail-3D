//! Alphas of your own for the sculpting brushes (decision 0308): reading
//! a picture as an alpha, and the alpha library, a folder of greyscale
//! PNG files in the app's own data folder, the same in every project.
//!
//! A picture brought in is read as any image is (PNG, JPEG, TGA or BMP,
//! its size limited before its pixels are read), turned grey (how light
//! each pixel is, times how solid: see-through parts are black), made
//! smaller to fit within [`sculpt::alpha::MAX_ALPHA_SIDE`], and packed as
//! a greyscale PNG, which is what the library keeps. Its pixels are taken
//! from that PNG, so an alpha read again from the library is the same.

use crate::image_file::read_image;
use sculpt::Alpha;
use std::path::{Path, PathBuf};
use words::{tr, trf};

/// The most alphas the library lists: more files are left out, so a
/// folder filled by mistake can't make the shelf crawl.
pub const MAX_LIBRARY: usize = 200;

/// The biggest library file read, in bytes (a greyscale PNG of the
/// largest alpha is far smaller).
pub const MAX_ALPHA_FILE: u64 = 8 << 20;

/// An alpha read from a picture, and the greyscale PNG it is kept as.
#[derive(Debug, Clone)]
pub struct ReadAlpha {
    /// The alpha.
    pub alpha: Alpha,
    /// Its pixels packed as a greyscale PNG, for the library.
    pub packed: Vec<u8>,
}

/// Reads a picture as an alpha from the bytes of a PNG, JPEG, TGA or BMP
/// file, named `name`. Refused in plain words when it is damaged, too
/// big to read, or black everywhere.
pub fn read_alpha(bytes: &[u8], name: &str) -> Result<ReadAlpha, String> {
    let image = read_image(bytes)?;
    let grey: Vec<u8> = image
        .rgba
        .chunks_exact(4)
        .map(|p| {
            let light = 0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]);
            (light * f32::from(p[3]) / 255.0).round().clamp(0.0, 255.0) as u8
        })
        .collect();
    let mut picture = image::GrayImage::from_raw(image.width, image.height, grey)
        .ok_or_else(|| tr("it is damaged").to_string())?;
    let most = sculpt::alpha::MAX_ALPHA_SIDE;
    if image.width > most || image.height > most {
        let scale = f64::from(most) / f64::from(image.width.max(image.height));
        let w = ((f64::from(image.width) * scale).round() as u32).clamp(1, most);
        let h = ((f64::from(image.height) * scale).round() as u32).clamp(1, most);
        picture = image::imageops::resize(&picture, w, h, image::imageops::FilterType::Triangle);
    }
    let mut packed = Vec::new();
    picture
        .write_to(
            &mut std::io::Cursor::new(&mut packed),
            image::ImageFormat::Png,
        )
        .map_err(|e| trf!("it couldn't be packed ({})", e))?;
    let alpha = alpha_from_packed(&packed, name)?;
    Ok(ReadAlpha { alpha, packed })
}

/// The alpha whose greyscale PNG is `packed` (as kept in the library),
/// named `name`; refused when it is bigger than an alpha is kept.
pub fn alpha_from_packed(packed: &[u8], name: &str) -> Result<Alpha, String> {
    let image = read_image(packed)?;
    let most = sculpt::alpha::MAX_ALPHA_SIDE;
    if image.width > most || image.height > most {
        return Err(trf!(
            "it is bigger than an alpha is kept ({} pixels across or down at most)",
            most
        ));
    }
    // Kept grey: any of the three colours is the grey.
    let values = image.rgba.chunks_exact(4).map(|p| p[0]).collect();
    Alpha::new(image.width, image.height, values, name).map_err(|why| tr(why).to_string())
}

/// Reads the picture at `path` as an alpha ([`read_alpha`]), named after
/// the file (without its extension).
pub fn open_alpha(path: &Path) -> Result<ReadAlpha, String> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > crate::limits::MAX_UNPACKED {
        return Err(tr(crate::limits::TOO_MUCH).to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| trf!("it can't be read ({})", e))?;
    read_alpha(&bytes, &stem(path))
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The alphas in the library `folder`, by name, and for each file that
/// couldn't be read, its name and why. A missing folder is an empty
/// library.
pub fn library(folder: &Path) -> (Vec<Alpha>, Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return (Vec::new(), Vec::new());
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("png"))
        })
        .collect();
    paths.sort_by_key(|p| stem(p).to_lowercase());
    paths.truncate(MAX_LIBRARY);
    let (mut alphas, mut broken) = (Vec::new(), Vec::new());
    for path in paths {
        let name = stem(&path);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let read = if size > MAX_ALPHA_FILE {
            Err(tr(crate::limits::TOO_MUCH).to_string())
        } else {
            std::fs::read(&path)
                .map_err(|e| trf!("it can't be read ({})", e))
                .and_then(|bytes| alpha_from_packed(&bytes, &name))
        };
        match read {
            Ok(alpha) => alphas.push(alpha),
            Err(why) => broken.push((name, why)),
        }
    }
    (alphas, broken)
}

/// Keeps `read` in the library `folder` (made if missing) under its name,
/// or its name with a number when that is taken. Returns the alpha named
/// as kept.
pub fn keep(folder: &Path, read: &ReadAlpha) -> Result<Alpha, String> {
    std::fs::create_dir_all(folder).map_err(|e| trf!("the alpha library can't be made ({})", e))?;
    let base: String = read
        .alpha
        .name()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let base = if base.trim().is_empty() {
        "alpha".to_string()
    } else {
        base.trim().to_string()
    };
    let mut name = base.clone();
    let mut n = 2;
    while folder.join(format!("{name}.png")).exists() {
        name = format!("{base} {n}");
        n += 1;
    }
    std::fs::write(folder.join(format!("{name}.png")), &read.packed)
        .map_err(|e| trf!("it couldn't be kept in the alpha library ({})", e))?;
    alpha_from_packed(&read.packed, &name)
}

/// Takes the alpha `name` out of the library `folder` (deleting its file).
pub fn forget(folder: &Path, name: &str) -> Result<(), String> {
    // Only a plain name in the folder, never a path elsewhere.
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return Err(tr("it isn't in the alpha library").to_string());
    }
    std::fs::remove_file(folder.join(format!("{name}.png")))
        .map_err(|e| trf!("it couldn't be taken out of the alpha library ({})", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(width, height, |x, y| image::Rgba(pixel(x, y)));
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("alpha-library-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_picture_reads_as_grey_with_see_through_parts_black() {
        let bytes = png(4, 2, |x, _| match x {
            0 => [255, 255, 255, 255],
            1 => [0, 0, 0, 255],
            2 => [255, 255, 255, 0],
            _ => [255, 0, 0, 255],
        });
        let read = read_alpha(&bytes, "test").unwrap();
        let v = read.alpha.values();
        assert_eq!((read.alpha.width(), read.alpha.height()), (4, 2));
        assert_eq!(&v[..4], &[255, 0, 0, 54]);
        assert_eq!(read.alpha.name(), "test");
        // Read again from what is kept: the same.
        let again = alpha_from_packed(&read.packed, "test").unwrap();
        assert_eq!(again, read.alpha);
    }

    #[test]
    fn a_big_picture_is_made_smaller_and_keeps_its_shape() {
        let most = sculpt::alpha::MAX_ALPHA_SIDE;
        let bytes = png(most * 2, most, |x, _| {
            let v = if x < most { 255 } else { 0 };
            [v, v, v, 255]
        });
        let read = read_alpha(&bytes, "big").unwrap();
        assert_eq!((read.alpha.width(), read.alpha.height()), (most, most / 2));
        assert!(read.alpha.at(-0.5, 0.0) > 0.99);
        assert!(read.alpha.at(0.5, 0.0) < 0.01);
    }

    #[test]
    fn broken_huge_and_black_pictures_are_refused_in_plain_words() {
        assert!(read_alpha(b"not a picture", "x").is_err());
        let mut cut = png(64, 64, |_, _| [9, 9, 9, 255]);
        cut.truncate(cut.len() / 2);
        assert!(read_alpha(&cut, "x").is_err());
        // A header claiming 100 000 pixels across: refused before reading.
        let mut huge = png(1, 1, |_, _| [255; 4]);
        huge[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        huge[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert!(read_alpha(&huge, "x").is_err());
        let black = read_alpha(&png(8, 8, |_, _| [0, 0, 0, 255]), "x").unwrap_err();
        assert!(black.contains("black everywhere"), "{black}");
        // A kept file bigger than an alpha is refused, not shrunk.
        let side = sculpt::alpha::MAX_ALPHA_SIDE + 1;
        assert!(alpha_from_packed(&png(side, 1, |_, _| [255; 4]), "x").is_err());
    }

    #[test]
    fn reading_random_bytes_as_an_alpha_never_crashes() {
        // A small fuzz: flipped and cut bytes of a good PNG read or are
        // refused, never crash.
        let good = png(16, 16, |x, y| [(x * 16) as u8, (y * 16) as u8, 7, 255]);
        let mut seed = 0x1234_5678u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for _ in 0..600 {
            let mut bad = good.clone();
            for _ in 0..(next() % 4 + 1) {
                let i = next() as usize % bad.len();
                bad[i] = next() as u8;
            }
            if next() % 5 == 0 {
                bad.truncate(next() as usize % bad.len());
            }
            if let Ok(read) = read_alpha(&bad, "fuzz") {
                assert!(read.alpha.width() <= sculpt::alpha::MAX_ALPHA_SIDE);
            }
            let _ = alpha_from_packed(&bad, "fuzz");
        }
    }

    #[test]
    fn the_library_keeps_lists_and_forgets_alphas() {
        let dir = folder("keep");
        assert!(library(&dir).0.is_empty(), "a missing folder is empty");
        let read = read_alpha(&png(8, 8, |x, _| [(x * 30) as u8; 4]), "stone/rock?").unwrap();
        let kept = keep(&dir, &read).unwrap();
        assert_eq!(kept.name(), "stone_rock_");
        let twice = keep(&dir, &read).unwrap();
        assert_eq!(twice.name(), "stone_rock_ 2");
        // A damaged file beside them is reported, not fatal.
        std::fs::write(dir.join("broken.png"), b"junk").unwrap();
        std::fs::write(dir.join("notes.txt"), b"not an alpha").unwrap();
        let (alphas, broken) = library(&dir);
        let names: Vec<_> = alphas.iter().map(Alpha::name).collect();
        assert_eq!(names, vec!["stone_rock_", "stone_rock_ 2"]);
        assert_eq!(alphas[0], kept);
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].0, "broken");
        forget(&dir, "stone_rock_").unwrap();
        assert!(forget(&dir, "../elsewhere").is_err());
        assert!(forget(&dir, "missing").is_err());
        assert_eq!(library(&dir).0.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
