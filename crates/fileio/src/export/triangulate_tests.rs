//! Tests for the Triangulate modifier going out (decision 0161): with
//! *Fixed*, the file is the same as the export's own triangulation gives;
//! the triangles go out even when the export doesn't cut polygons; and
//! Weighted normals before it go out unchanged.

use super::*;
use scene::{
    Command, ModifierKind, ObjectKind, Primitive, TriangulateMethod, TriangulateOptions,
    WeightedNormalsOptions,
};

/// A document with a standard cylinder (quads round it, two 16-sided
/// caps) and the modifiers `kinds`.
fn cylinder_with(kinds: &[ModifierKind]) -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cylinder(
        Default::default(),
    ))))
    .unwrap();
    let id = doc.selection()[0];
    for kind in kinds {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: kind.clone(),
        })
        .unwrap();
    }
    doc
}

fn fixed() -> ModifierKind {
    ModifierKind::Triangulate(TriangulateOptions {
        method: TriangulateMethod::Fixed,
        quads_only: false,
    })
}

/// Every exported triangle as its corners' places, normals and UVs
/// (rounded), each started at its smallest corner, sorted: the same set
/// whatever order the triangles and points come in.
fn triangles(mesh: &ExportMesh) -> Vec<Vec<[i64; 7]>> {
    let round = |x: f32| (x * 10_000.0).round() as i64;
    let mut out: Vec<Vec<[i64; 7]>> = mesh
        .polygons
        .iter()
        .map(|p| {
            let mut corners: Vec<[i64; 7]> = p
                .corners
                .iter()
                .map(|c| {
                    let at = mesh.points[c.point as usize];
                    let n = mesh.normals[c.normal as usize];
                    let uv = c.uv.map_or(Vec2::ZERO, |i| mesh.uvs[i as usize]);
                    [at.x, at.y, at.z, n.x, n.y, n.z, uv.x + uv.y * 7.0].map(round)
                })
                .collect();
            let low = (0..corners.len()).min_by_key(|i| corners[*i]).unwrap_or(0);
            corners.rotate_left(low);
            corners
        })
        .collect();
    out.sort();
    out
}

fn exported(doc: &Document, triangulate: bool) -> ExportMesh {
    let options = ExportOptions {
        triangulate,
        ..ExportOptions::default()
    };
    prepare(doc, &options).remove(0).world
}

#[test]
fn fixed_gives_exactly_what_the_export_cut_gives() {
    let plain = exported(&cylinder_with(&[]), true);
    let cut = exported(&cylinder_with(&[fixed()]), true);
    assert_eq!(plain.triangle_count(), cut.triangle_count());
    assert_eq!(triangles(&plain), triangles(&cut));
}

#[test]
fn the_triangles_go_out_even_without_the_exports_triangulate() {
    let doc = cylinder_with(&[ModifierKind::Triangulate(TriangulateOptions::default())]);
    let mesh = exported(&doc, false);
    assert!(mesh.polygons.iter().all(|p| p.corners.len() == 3));
    assert_eq!(mesh.triangle_count(), 16 * 2 + 14 * 2);
    // Quads only leaves the caps as they are.
    let doc = cylinder_with(&[ModifierKind::Triangulate(TriangulateOptions {
        quads_only: true,
        ..TriangulateOptions::default()
    })]);
    let mesh = exported(&doc, false);
    assert_eq!(
        mesh.polygons
            .iter()
            .filter(|p| p.corners.len() == 16)
            .count(),
        2
    );
}

#[test]
fn weighted_normals_before_it_go_out_unchanged() {
    let weighted = ModifierKind::WeightedNormals(WeightedNormalsOptions::default());
    let alone = exported(&cylinder_with(std::slice::from_ref(&weighted)), true);
    let then_cut = exported(&cylinder_with(&[weighted, fixed()]), true);
    assert_eq!(triangles(&alone), triangles(&then_cut));
}

#[test]
fn baked_it_gives_the_same_file() {
    let doc = cylinder_with(&[ModifierKind::Triangulate(TriangulateOptions::default())]);
    let mut baked = doc.clone();
    let id = baked.scene().walk()[0].0;
    baked.execute(Command::MakeEditable(vec![id])).unwrap();
    for triangulate in [false, true] {
        assert_eq!(exported(&doc, triangulate), exported(&baked, triangulate));
    }
}
