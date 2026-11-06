//! Baked maps as files ([decision 0250](../../../docs/decisions/0250-the-baker.md)):
//! a bake's pixels spread past the edges of the UV pieces (padding, so a
//! game's smaller copies of the texture don't pull in the empty space
//! round them), the rest filled with a neutral colour, as a PNG.

use crate::export::textures::{TextureFormat, encode, pad};

/// The PNG of `image` (a bake: texels off the pieces see-through), its
/// pieces spread `padding` texels outward and everything else `background`.
pub fn bake_png(image: &paint::Image, padding: u32, background: [u8; 3]) -> Option<Vec<u8>> {
    let size = image.size();
    let mut rgba = image.to_rgba();
    let mut covered: Vec<bool> = rgba.chunks_exact(4).map(|p| p[3] > 0).collect();
    pad(&mut rgba, &mut covered, size, padding);
    for (pixel, on) in rgba.chunks_exact_mut(4).zip(&covered) {
        if !*on {
            pixel[..3].copy_from_slice(&background);
        }
        pixel[3] = 255;
    }
    encode(&rgba, size, TextureFormat::Png)
}

/// An ORM texture (decision 0253), as Godot's ORM material reads it:
/// occlusion in red (from `occlusion`, a bake), roughness in green and
/// metal in blue (0 to 255, the same everywhere), padded like
/// [`bake_png`].
pub fn orm_png(occlusion: &paint::Image, roughness: u8, metal: u8, padding: u32) -> Option<Vec<u8>> {
    let size = occlusion.size();
    let mut rgba = occlusion.to_rgba();
    let mut covered: Vec<bool> = rgba.chunks_exact(4).map(|p| p[3] > 0).collect();
    pad(&mut rgba, &mut covered, size, padding);
    for (pixel, on) in rgba.chunks_exact_mut(4).zip(&covered) {
        let ao = if *on { pixel[0] } else { 255 };
        pixel.copy_from_slice(&[ao, roughness, metal, 255]);
    }
    encode(&rgba, size, TextureFormat::Png)
}

/// A Godot 4 material file (`.tres`, an `ORMMaterial3D`) using the normal
/// map and ORM texture at `normal` and `orm` (Godot paths: `res://…`, or
/// beside the material), either of which may be missing.
pub fn godot_material(normal: Option<&str>, orm: Option<&str>) -> String {
    let textures: Vec<(&str, &str)> = [("normal", normal), ("orm", orm)]
        .into_iter()
        .filter_map(|(k, p)| Some((k, p?)))
        .collect();
    let mut out = format!(
        "[gd_resource type=\"ORMMaterial3D\" load_steps={} format=3]\n\n",
        textures.len() + 1
    );
    for (i, (_, path)) in textures.iter().enumerate() {
        let path = path.replace('\\', "/").replace('"', "");
        out.push_str(&format!(
            "[ext_resource type=\"Texture2D\" path=\"{path}\" id=\"{}\"]\n",
            i + 1
        ));
    }
    out.push_str("\n[resource]\n");
    for (i, (kind, _)) in textures.iter().enumerate() {
        match *kind {
            "normal" => out.push_str(&format!(
                "normal_enabled = true\nnormal_texture = ExtResource(\"{}\")\n",
                i + 1
            )),
            _ => out.push_str(&format!("orm_texture = ExtResource(\"{}\")\n", i + 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_are_padded_and_the_rest_filled() {
        let mut image = paint::Image::new(16).unwrap();
        image.set_pixel(8, 8, [200, 10, 10, 255]);
        let png = bake_png(&image, 2, [128, 128, 255]).unwrap();
        let read = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(read.get_pixel(8, 8).0, [200, 10, 10, 255]);
        // Two texels out: the piece's colour.
        assert_eq!(read.get_pixel(10, 8).0, [200, 10, 10, 255]);
        // Further: the background, opaque.
        assert_eq!(read.get_pixel(0, 0).0, [128, 128, 255, 255]);
    }

    #[test]
    fn the_orm_texture_packs_occlusion_roughness_and_metal() {
        let mut image = paint::Image::new(8).unwrap();
        image.set_pixel(4, 4, [90, 90, 90, 255]);
        let png = orm_png(&image, 200, 0, 0).unwrap();
        let read = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(read.get_pixel(4, 4).0, [90, 200, 0, 255]);
        // Off the pieces: open.
        assert_eq!(read.get_pixel(0, 0).0, [255, 200, 0, 255]);
    }

    #[test]
    fn the_godot_material_names_its_textures() {
        let both = godot_material(Some("res://kit/crate_normal.png"), Some("crate_orm.png"));
        assert!(both.starts_with("[gd_resource type=\"ORMMaterial3D\" load_steps=3 format=3]"));
        assert!(both.contains("path=\"res://kit/crate_normal.png\" id=\"1\""));
        assert!(both.contains("normal_texture = ExtResource(\"1\")"));
        assert!(both.contains("orm_texture = ExtResource(\"2\")"));
        let none = godot_material(None, None);
        assert!(none.contains("load_steps=1") && !none.contains("ext_resource"));
        // Quotes can't break the file.
        assert!(!godot_material(Some("a\"b"), None).contains("a\"b"));
    }

    #[test]
    fn an_empty_bake_is_all_background() {
        let image = paint::Image::new(16).unwrap();
        let png = bake_png(&image, 4, [0, 0, 0]).unwrap();
        let read = image::load_from_memory(&png).unwrap().to_rgba8();
        assert!(read.pixels().all(|p| p.0 == [0, 0, 0, 255]));
    }
}
