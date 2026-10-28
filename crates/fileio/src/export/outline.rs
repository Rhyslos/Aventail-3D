//! Outlines in exports ([decision 0201](../../../../docs/decisions/0201-outlines.md)):
//! each outlined object gets an *inverted hull* beside it, `name-outline`:
//! a copy of its shape with every point pushed out along the average
//! direction of the polygons round it, turned inside out, in the
//! outline's colour. Engines leave out back faces, so only the rim of the
//! shell shows round the object: an outline that works anywhere, with no
//! shader. For Godot, a ready-made outline shader goes beside the file
//! too.

use super::{ExportCorner, ExportMesh, ExportPolygon, ExportedObject};
use glam::Vec3;
use scene::{Collision, Material, Outline};

/// `shape` as an outline shell `width` thick (in the file's units):
/// pushed out, turned inside out, normals facing in, no UVs.
pub(crate) fn shell(shape: &ExportMesh, width: f32) -> ExportMesh {
    let mut sums = vec![Vec3::ZERO; shape.points.len()];
    for polygon in &shape.polygons {
        let points: Vec<Vec3> = polygon
            .corners
            .iter()
            .map(|c| {
                shape
                    .points
                    .get(c.point as usize)
                    .copied()
                    .unwrap_or(Vec3::ZERO)
            })
            .collect();
        let mut area = Vec3::ZERO;
        for k in 0..points.len() {
            area += points[k].cross(points[(k + 1) % points.len()]);
        }
        if !area.is_finite() {
            continue;
        }
        for c in &polygon.corners {
            if let Some(s) = sums.get_mut(c.point as usize) {
                *s += area;
            }
        }
    }
    let directions: Vec<Vec3> = sums.iter().map(|s| s.normalize_or_zero()).collect();
    let points = shape
        .points
        .iter()
        .zip(&directions)
        .map(|(p, d)| *p + *d * width)
        .collect();
    // One normal per point, facing in: the shell's inside is what shows.
    let normals = directions
        .iter()
        .map(|d| if *d == Vec3::ZERO { Vec3::Y } else { -*d })
        .collect();
    let polygons = shape
        .polygons
        .iter()
        .map(|polygon| ExportPolygon {
            corners: polygon
                .corners
                .iter()
                .rev()
                .map(|c| ExportCorner {
                    point: c.point,
                    normal: c.point,
                    uv: None,
                })
                .collect(),
            material: 0,
        })
        .collect();
    ExportMesh {
        points,
        normals,
        uvs: Vec::new(),
        polygons,
    }
}

/// The material of an outline of colour `colour`: named for it, so
/// outlines of one colour share it.
pub(crate) fn material(outline: &Outline) -> Material {
    let [r, g, b] = outline.colour;
    Material {
        name: format!("outline-{r:02x}{g:02x}{b:02x}"),
        color: [r, g, b].map(|c| f32::from(c) / 255.0),
    }
}

/// The outline shell of every outlined object in `objects`, each as an
/// object of its own beside it (`name-outline`), its width turned into
/// the file's units by `scale` (file units per centimetre).
pub(crate) fn outline_objects(objects: &[ExportedObject], scale: f32) -> Vec<ExportedObject> {
    objects
        .iter()
        .filter_map(|o| {
            let outline = o.outline?;
            let width = outline.width * scale;
            Some(ExportedObject {
                name: format!("{}-outline", o.name),
                world: shell(&o.world, width),
                local: shell(&o.local, width),
                materials: vec![material(&outline)],
                collision: Collision::None,
                collision_parts: Vec::new(),
                levels: Vec::new(),
                outline: None,
                ..o.clone()
            })
        })
        .collect()
}

/// A Godot outline shader (decision 0201), for a material's Next Pass:
/// draws the mesh again pushed out along its normals by `width` (metres),
/// back faces only, unlit in the outline's colour. `generator` names the
/// app on its first line, so a later export may replace it.
pub(crate) fn godot_shader(generator: &str, outline: &Outline) -> String {
    let [r, g, b] = outline.colour.map(|c| f32::from(c) / 255.0);
    let width = outline.width / 100.0;
    format!(
        "// {generator}: an outline shader. Set it as the Next Pass of a material\n\
         // (Material > Next Pass > New ShaderMaterial > Shader > Load).\n\
         shader_type spatial;\n\
         render_mode unshaded, cull_front;\n\
         \n\
         uniform float width = {width:.4};\n\
         uniform vec4 colour : source_color = vec4({r:.4}, {g:.4}, {b:.4}, 1.0);\n\
         \n\
         void vertex() {{\n\
         \tVERTEX += NORMAL * width;\n\
         }}\n\
         \n\
         void fragment() {{\n\
         \tALBEDO = colour.rgb;\n\
         }}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> ExportMesh {
        ExportMesh {
            points: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            uvs: Vec::new(),
            polygons: vec![ExportPolygon {
                corners: (0..4)
                    .map(|i| ExportCorner {
                        point: i,
                        normal: 0,
                        uv: None,
                    })
                    .collect(),
                material: 0,
            }],
        }
    }

    #[test]
    fn a_shell_is_pushed_out_and_turned_inside_out() {
        let s = shell(&square(), 0.5);
        assert_eq!(s.points[0], Vec3::new(0.0, 0.0, 0.5));
        assert_eq!(s.normals[0], -Vec3::Z);
        let corners: Vec<u32> = s.polygons[0].corners.iter().map(|c| c.point).collect();
        assert_eq!(corners, vec![3, 2, 1, 0]);
        assert!(s.uvs.is_empty());
    }

    #[test]
    fn outline_materials_are_named_by_colour() {
        let m = material(&Outline {
            width: 1.0,
            colour: [255, 0, 16],
        });
        assert_eq!(m.name, "outline-ff0010");
        assert_eq!(m.color, [1.0, 0.0, 16.0 / 255.0]);
    }

    #[test]
    fn the_godot_shader_names_the_app_and_its_width_in_metres() {
        let shader = godot_shader("App 1.0", &Outline::default());
        assert!(shader.starts_with("// App 1.0"));
        assert!(shader.contains("shader_type spatial;"));
        assert!(shader.contains("uniform float width = 0.0100;"));
    }
}
