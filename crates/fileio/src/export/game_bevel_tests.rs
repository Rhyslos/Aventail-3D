//! Tests for the game bevel going out (decision 0156): a cube, a
//! cylinder, an arch and a holed cube, game-bevelled, read back from a
//! `.glb` the way an engine reads them: the flat faces' normals point
//! straight out, the bevels' lean, and nothing is missing or bad.

use super::normals_tests::{glb, indices, parts, vec3s};
use super::*;
use glam::Vec3;
use scene::{BooleanKind, Command, ObjectId, ObjectKind, PrimitiveKind, Transform};

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// A document holding one game-bevelled shape, `name`d: a cube, a
/// cylinder, an arch, a cube with a round hole cut through it (a boolean)
/// or a cylinder with a square one.
/// With `weighted` false, its Weighted normals is switched off.
pub(super) fn game_bevelled(name: &str, weighted: bool) -> Document {
    let mut doc = Document::new();
    let id = match name {
        "cube" => create(&mut doc, PrimitiveKind::Cube),
        "cylinder" => create(&mut doc, PrimitiveKind::Cylinder),
        "arch" => create(&mut doc, PrimitiveKind::Arch),
        _ => {
            let (kept, cutter) = if name == "holed cube" {
                (PrimitiveKind::Cube, PrimitiveKind::Cylinder)
            } else {
                (PrimitiveKind::Cylinder, PrimitiveKind::Cube)
            };
            let id = create(&mut doc, kept);
            let cutter = create(&mut doc, cutter);
            doc.execute(Command::SetTransforms(vec![(
                cutter,
                Transform {
                    position: Vec3::new(0.0, 0.0, -25.0),
                    scale: Vec3::new(0.5, 0.5, 1.5),
                    ..Transform::default()
                },
            )]))
            .unwrap();
            doc.execute(Command::AddBoolean {
                kept: id,
                cutters: vec![cutter],
                kind: BooleanKind::Subtract,
            })
            .unwrap();
            id
        }
    };
    doc.game_bevel(&[id], "Game bevel", 1).unwrap();
    if !weighted {
        let mut off = doc.scene().modifiers(id)[1].clone();
        off.on = false;
        doc.execute(Command::ChangeModifier {
            id,
            index: 1,
            modifier: off,
        })
        .unwrap();
    }
    doc
}

/// The game-bevelled object (not a cutter).
fn bevelled_id(doc: &Document) -> ObjectId {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| !doc.scene().modifiers(*id).is_empty())
        .unwrap()
}

pub(super) const SHAPES: [&str; 5] = ["cube", "cylinder", "arch", "holed cube", "holed cylinder"];

/// A polygon's (or triangle's) area and outline length.
fn measure(points: &[Vec3]) -> (f32, f32) {
    let mut sum = Vec3::ZERO;
    let mut outline = 0.0;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        sum += a.cross(b);
        outline += a.distance(b);
    }
    (sum.length() / 2.0, outline)
}

/// The share of the shape's area that is *flat*: polygons not on a bevel
/// (area ÷ outline at least 0.8 cm) whose every neighbour is on a bevel
/// or lies flat with them, as the scene's game bevel tests count them.
fn flat_share(mesh: &Mesh) -> f32 {
    let topology = mesh.topology();
    let points = |f: FaceId| -> Vec<Vec3> {
        mesh.face(f)
            .unwrap()
            .iter()
            .map(|v| mesh.position(*v).unwrap())
            .collect()
    };
    let thin = |f: FaceId| {
        let (area, outline) = measure(&points(f));
        area / outline.max(1e-6) < 0.8
    };
    let (mut flat, mut total) = (0.0, 0.0);
    for (face, _) in mesh.faces() {
        let (area, _) = measure(&points(face));
        total += area;
        let facing = mesh.face_normal(face);
        if thin(face) {
            continue;
        }
        let alone = topology.face_edges(face).iter().all(|e| {
            topology.edge_uses(*e).iter().all(|u| {
                u.face == face
                    || thin(u.face)
                    || mesh.face_normal(u.face).angle_between(facing).to_degrees() < 1.0
            })
        });
        if alone {
            flat += area;
        }
    }
    flat / total
}

/// From a `.glb`: the share of its triangles' area whose three normals
/// all point within 0.1° of the triangle's facing (shaded flat), and the
/// most any normal leans from its triangle's facing.
fn glb_flat_share(glb: &[u8]) -> (f32, f32) {
    let (json, bin) = parts(glb);
    let (mut flat, mut total, mut most) = (0.0, 0.0, 0.0f32);
    for primitive in json["meshes"][0]["primitives"].as_array().unwrap() {
        let attributes = &primitive["attributes"];
        let at = |name: &str| attributes[name].as_u64().unwrap() as usize;
        let positions = vec3s(&json, &bin, at("POSITION"));
        let normals = vec3s(&json, &bin, at("NORMAL"));
        // Every tangent a unit vector, as glTF asks (a bevel's corner can
        // have no UV area, decision 0156).
        let tangents = super::normals_tests::vec4s(&json, &bin, at("TANGENT"));
        for t in &tangents {
            assert!(
                (t.truncate().length() - 1.0).abs() < 1e-3,
                "a unit tangent: {t}"
            );
            assert!(t.w.abs() == 1.0);
        }
        let list = indices(&json, &bin, primitive["indices"].as_u64().unwrap() as usize);
        for t in list.chunks(3) {
            let p = [positions[t[0]], positions[t[1]], positions[t[2]]];
            let (area, _) = measure(&p);
            total += area;
            if area < 1e-9 {
                continue;
            }
            let facing = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
            let lean = t
                .iter()
                .map(|i| {
                    let n = normals[*i];
                    assert!((n.length() - 1.0).abs() < 1e-4, "a unit normal");
                    n.angle_between(facing).to_degrees()
                })
                .fold(0.0f32, f32::max);
            most = most.max(lean);
            if lean < 0.1 {
                flat += area;
            }
        }
    }
    (flat / total, most)
}

#[test]
fn a_glb_carries_flat_faces_and_round_bevels() {
    for name in SHAPES {
        let doc = game_bevelled(name, true);
        let id = bevelled_id(&doc);
        let shape = doc.scene().exported_shape(id).unwrap();
        let expected = flat_share(&shape);
        assert!(expected > 0.1, "{name}: {expected}");
        let (share, most) = glb_flat_share(&glb(&doc));
        assert!(
            share >= expected * 0.999,
            "{name}: {share} of the area is flat, {expected} should be"
        );
        assert!(most > 10.0, "{name}: the bevels lean ({most}°)");
        // No normal points away from its triangle: nothing folds (a
        // corner fanned by the boolean once kept its point and folded
        // round it, 111°, decision 0156). A saddle corner's patch, where
        // a hole's inner edge meets its rim, leans up to about 65°.
        assert!(most < 80.0, "{name}: a normal leans {most}°");
        // Without Weighted normals the flat faces lean towards the bevels.
        let (plain, _) = glb_flat_share(&glb(&game_bevelled(name, false)));
        assert!(
            plain < expected * 0.5,
            "{name}: {plain} flat without Weighted normals"
        );
    }
}

#[test]
fn the_baked_shape_exports_the_same() {
    for name in SHAPES {
        let doc = game_bevelled(name, true);
        let live = glb(&doc);
        let mut baked = doc;
        let id = bevelled_id(&baked);
        baked.execute(Command::MakeEditable(vec![id])).unwrap();
        assert_eq!(glb(&baked), live, "{name}");
    }
}
