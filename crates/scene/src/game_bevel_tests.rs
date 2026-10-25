//! Tests for the game bevel (decision 0156): a Live bevel with soft
//! bevels and Weighted normals last, the object shaded by its marked
//! edges, as one undo step; and the shading it gives on a cube, a
//! cylinder, an arch and a booleaned shape.

use crate::*;
use glam::Vec3;
use mesh::custom_normals::has_custom_normals;

const KEY: u64 = 0x6a3e_be7e;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

fn game_bevel(doc: &mut Document, ids: &[ObjectId]) -> Result<GameBevelDone, SceneError> {
    doc.game_bevel(ids, "Game bevel", KEY)
}

/// How many recorded edits an object has.
fn edit_count(doc: &Document, id: ObjectId) -> usize {
    let object = doc.scene().object(id).unwrap();
    object.stack.layers().iter().map(|l| l.entries.len()).sum()
}

fn kinds(doc: &Document, id: ObjectId) -> Vec<&'static str> {
    doc.scene()
        .modifiers(id)
        .iter()
        .map(|m| m.kind.name())
        .collect()
}

/// A standard cube with a hole through it: a cylinder, narrowed and
/// lengthened, subtracted. The hole's rims are cut edges, which the
/// boolean marks hard after the object's edits (decision 0135).
fn holed_cube(doc: &mut Document) -> ObjectId {
    holed(doc, PrimitiveKind::Cube, PrimitiveKind::Cylinder)
}

/// A standard `kept` shape with a hole the shape of `cutter` through it.
fn holed(doc: &mut Document, kept: PrimitiveKind, cutter: PrimitiveKind) -> ObjectId {
    let id = create(doc, kept);
    let cutter = create(doc, cutter);
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

/// The test shapes, each in a document of its own: a cube, a cylinder,
/// an arch, a cube with a round hole and a cylinder with a square one.
fn shapes() -> Vec<(&'static str, Document, ObjectId)> {
    let mut out = Vec::new();
    for (name, kind) in [
        ("cube", PrimitiveKind::Cube),
        ("cylinder", PrimitiveKind::Cylinder),
        ("arch", PrimitiveKind::Arch),
    ] {
        let mut doc = Document::new();
        let id = create(&mut doc, kind);
        out.push((name, doc, id));
    }
    let mut doc = Document::new();
    let id = holed_cube(&mut doc);
    out.push(("holed cube", doc, id));
    let mut doc = Document::new();
    let id = holed(&mut doc, PrimitiveKind::Cylinder, PrimitiveKind::Cube);
    out.push(("holed cylinder", doc, id));
    out
}

/// A polygon's area divided by its outline's length: about half its
/// width for a long thin strip. A bevel's rows (1 cm wide) and corner
/// patches come under [`THIN`]; the shapes' own polygons don't.
fn fatness(mesh: &Mesh, face: FaceId) -> f32 {
    let corners = mesh.face(face).unwrap();
    let p: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
    let mut sum = Vec3::ZERO;
    let mut outline = 0.0;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        sum += a.cross(b);
        outline += a.distance(b);
    }
    sum.length() / 2.0 / outline.max(1e-6)
}

const THIN: f32 = 0.8;

/// How the final shape shades: the most a *flat* polygon's corner normals
/// lean from its facing (a polygon not on a bevel whose every neighbour
/// is on a bevel or lies flat with it: a cube's sides, a cylinder's caps,
/// an arch's front), how many such polygons there are, the most a
/// bevel's corner normals lean (round bevels lean towards the faces
/// beside them), and how many edges the shading breaks at.
struct Shading {
    flat_lean: f32,
    flat: usize,
    bevel_lean: f32,
    breaks: usize,
}

fn shading(mesh: &Mesh, hardness: Hardness) -> Shading {
    let (_, normals) = mesh::corner_shading_with(mesh, hardness);
    let (starts, _) = mesh.face_layout();
    let topology = mesh.topology();
    let mut out = Shading {
        flat_lean: 0.0,
        flat: 0,
        bevel_lean: 0.0,
        breaks: mesh::hardness::shading_hard_edges(mesh, hardness).len(),
    };
    for (index, pair) in starts.windows(2).enumerate() {
        let face = FaceId::from_raw(index as u32);
        let facing = mesh.face_normal(face);
        if facing.length_squared() < 0.5 {
            continue;
        }
        let lean = normals[pair[0] as usize..pair[1] as usize]
            .iter()
            .map(|n| n.angle_between(facing).to_degrees())
            .fold(0.0f32, f32::max);
        if fatness(mesh, face) < THIN {
            out.bevel_lean = out.bevel_lean.max(lean);
            continue;
        }
        let flat = topology.face_edges(face).iter().all(|e| {
            topology.edge_uses(*e).iter().all(|u| {
                u.face == face
                    || fatness(mesh, u.face) < THIN
                    || mesh.face_normal(u.face).angle_between(facing).to_degrees() < 1.0
            })
        });
        if flat {
            out.flat += 1;
            out.flat_lean = out.flat_lean.max(lean);
        }
    }
    out
}

#[test]
fn every_shape_shades_with_flat_faces_and_round_bevels() {
    for (name, mut doc, id) in shapes() {
        let done = game_bevel(&mut doc, &[id]).unwrap();
        assert_eq!(
            done,
            GameBevelDone {
                objects: 1,
                kept_bevels: 0,
                switched: 1
            }
        );
        let scene = doc.scene();
        assert_eq!(
            kinds(&doc, id),
            ["Live bevel", "Weighted normals"],
            "{name}"
        );
        assert!(scene.modifier_problems(id).is_empty(), "{name}");
        let hardness = scene.object(id).unwrap().hardness;
        assert_eq!(hardness, Hardness::Marked, "{name}");
        for purpose_shape in [scene.shown_shape(id), scene.exported_shape(id)] {
            let shape = purpose_shape.unwrap();
            assert!(has_custom_normals(&shape), "{name}");
            let s = shading(&shape, hardness);
            assert!(s.flat > 0, "{name}: no flat polygons found");
            assert!(
                s.flat_lean < 0.1,
                "{name}: flat faces lean {}°",
                s.flat_lean
            );
            assert!(s.bevel_lean > 10.0, "{name}: bevels lean {}°", s.bevel_lean);
            assert_eq!(
                s.breaks, 0,
                "{name}: the bevels shade round, with no creases"
            );
        }
    }
}

#[test]
fn without_soft_bevels_or_marks_the_bevels_crease() {
    // What the game bevel's other two steps are for. Smoothed by angle
    // (every new shape), a 2-row bevel's middle step is 45°, sharper than
    // 30°: each strip shades with a crease along it.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, ModifierKind::LiveBevel(game_bevel_options()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    let by_angle = Hardness::ByAngle(STANDARD_SMOOTHING_ANGLE);
    let s = shading(&doc.scene().shown_shape(id).unwrap(), by_angle);
    assert!(s.breaks >= 12, "{}", s.breaks);
    // On a holed cube shaded by its marks, with the cube's own edges
    // softened by a recorded edit but without Soft bevels: the hole's
    // rims, marked hard by the boolean, keep hard strip borders.
    let mut doc = Document::new();
    let id = holed_cube(&mut doc);
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    doc.execute(Command::Record {
        id,
        operation: Operation::SetHardEdges {
            edges: shape.topology().edges().collect(),
            hard: false,
            anchors: None,
        },
    })
    .unwrap();
    add(&mut doc, id, ModifierKind::LiveBevel(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    let s = shading(&doc.scene().shown_shape(id).unwrap(), Hardness::Marked);
    assert!(s.breaks > 0, "the cut rims stay hard");
    // Soft bevels on: none.
    let mut soft = doc.scene().modifiers(id)[0].clone();
    soft.kind = ModifierKind::LiveBevel(game_bevel_options());
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: soft,
    })
    .unwrap();
    let s = shading(&doc.scene().shown_shape(id).unwrap(), Hardness::Marked);
    assert_eq!(s.breaks, 0);
}

#[test]
fn it_is_one_undo_step() {
    let mut doc = Document::new();
    let a = create(&mut doc, PrimitiveKind::Cube);
    let b = create(&mut doc, PrimitiveKind::Cylinder);
    let before = doc.scene().clone();
    let steps = doc.undo_steps();
    let done = game_bevel(&mut doc, &[a, b]).unwrap();
    assert_eq!(done.objects, 2);
    assert_eq!(done.switched, 2);
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Game bevel"));
    // Its look kept as marks: a recorded edit on each.
    for id in [a, b] {
        assert_eq!(edit_count(&doc, id), 1);
    }
    doc.undo();
    for id in [a, b] {
        assert!(doc.scene().modifiers(id).is_empty());
        assert_eq!(
            doc.scene().object(id),
            before.object(id),
            "everything put back"
        );
    }
    doc.redo();
    assert_eq!(kinds(&doc, a), ["Live bevel", "Weighted normals"]);
    assert_eq!(kinds(&doc, b), ["Live bevel", "Weighted normals"]);
}

#[test]
fn its_settings_are_ordinary_modifiers_changed_afterwards() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    game_bevel(&mut doc, &[id]).unwrap();
    let list = doc.scene().modifiers(id).to_vec();
    assert_eq!(
        list[0].kind,
        ModifierKind::LiveBevel(LiveBevelOptions {
            soften: true,
            ..LiveBevelOptions::default()
        })
    );
    let ModifierKind::LiveBevel(o) = list[0].kind else {
        panic!("a live bevel")
    };
    assert_eq!((o.angle, o.width, o.segments), (30.0, 2.0, 2));
    assert_eq!(
        list[1].kind,
        ModifierKind::WeightedNormals(WeightedNormalsOptions::default())
    );
    // A wider bevel in three rows: still flat faces and round bevels.
    let mut wider = list[0].clone();
    wider.kind = ModifierKind::LiveBevel(LiveBevelOptions {
        width: 6.0,
        segments: 3,
        ..o
    });
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: wider,
    })
    .unwrap();
    let s = shading(&doc.scene().shown_shape(id).unwrap(), Hardness::Marked);
    assert!(s.flat_lean < 0.1 && s.breaks == 0);
}

#[test]
fn a_live_bevel_and_weighted_normals_already_there_are_kept() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(WeightedNormalsOptions {
            strength: 0.5,
            ..Default::default()
        }),
    );
    add(
        &mut doc,
        id,
        ModifierKind::LiveBevel(LiveBevelOptions {
            width: 5.0,
            ..Default::default()
        }),
    );
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    // Weighted normals switched off, and not last (a new Subdivide goes
    // before it: moved up past it by hand).
    doc.execute(Command::MoveModifier { id, from: 2, to: 1 })
        .unwrap();
    let mut off = doc.scene().modifiers(id)[1].clone();
    off.on = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 1,
        modifier: off,
    })
    .unwrap();
    assert_eq!(
        kinds(&doc, id),
        ["Live bevel", "Weighted normals", "Subdivide"]
    );
    let done = game_bevel(&mut doc, &[id]).unwrap();
    assert_eq!(done.kept_bevels, 1);
    assert_eq!(done.switched, 0, "it already shaded by its marks");
    assert_eq!(
        kinds(&doc, id),
        ["Live bevel", "Subdivide", "Weighted normals"]
    );
    let list = doc.scene().modifiers(id);
    assert_eq!(
        list[0].kind,
        ModifierKind::LiveBevel(LiveBevelOptions {
            width: 5.0,
            soften: true,
            ..Default::default()
        }),
        "its own width kept"
    );
    assert!(list[2].on);
    assert_eq!(
        list[2].kind,
        ModifierKind::WeightedNormals(WeightedNormalsOptions {
            strength: 0.5,
            ..Default::default()
        })
    );
    assert_eq!(edit_count(&doc, id), 0);
}

#[test]
fn a_live_bevel_switched_off_is_switched_on() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, ModifierKind::LiveBevel(Default::default()));
    let mut off = doc.scene().modifiers(id)[0].clone();
    off.on = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: off,
    })
    .unwrap();
    game_bevel(&mut doc, &[id]).unwrap();
    assert!(doc.scene().modifiers(id)[0].on);
}

#[test]
fn running_it_again_changes_nothing() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    game_bevel(&mut doc, &[id]).unwrap();
    let scene = doc.scene().clone();
    let steps = doc.undo_steps();
    doc.game_bevel(&[id], "Game bevel", KEY + 1).unwrap();
    assert_eq!(doc.undo_steps(), steps);
    assert_eq!(doc.scene().object(id), scene.object(id));
}

#[test]
fn groups_are_skipped_and_instances_refused() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    let other = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::Group(vec![other])).unwrap();
    let group = doc.selection()[0];
    // A group alone: nothing to do.
    assert_eq!(game_bevel(&mut doc, &[group]), Err(SceneError::NoShape));
    // With a shape: the shape gets it, the group's own child doesn't.
    let done = game_bevel(&mut doc, &[group, id]).unwrap();
    assert_eq!(done.objects, 1);
    assert!(doc.scene().modifiers(other).is_empty());
    doc.execute(Command::Duplicate {
        ids: vec![other],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    let steps = doc.undo_steps();
    assert_eq!(
        doc.game_bevel(&[other, instance], "Game bevel", KEY + 1),
        Err(SceneError::Instance)
    );
    assert_eq!(doc.undo_steps(), steps);
    assert!(doc.scene().modifiers(other).is_empty());
    assert_eq!(
        game_bevel(&mut doc, &[ObjectId::from_raw(9_999)]),
        Err(SceneError::UnknownObject(ObjectId::from_raw(9_999)))
    );
}

#[test]
fn a_refusal_part_way_takes_back_what_was_done() {
    let mut doc = Document::new();
    let first = create(&mut doc, PrimitiveKind::Cube);
    let full = create(&mut doc, PrimitiveKind::Cube);
    // The second has room for one more modifier, not two.
    for _ in 0..MAX_MODIFIERS - 1 {
        add(&mut doc, full, ModifierKind::Subdivide(Default::default()));
    }
    let before = doc.scene().clone();
    let steps = doc.undo_steps();
    let result = doc.game_bevel(&[first, full], "Game bevel", KEY + 2);
    assert_eq!(result, Err(SceneError::Modifier(ModifierRefusal::TooMany)));
    assert_eq!(doc.undo_steps(), steps);
    for id in [first, full] {
        assert_eq!(doc.scene().object(id), before.object(id));
    }
}

#[test]
fn a_curve_shades_by_its_marks_without_a_recorded_edit() {
    let mut doc = Document::new();
    let curve = Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
        ],
        false,
    )
    .unwrap();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
        CurveShape::Points(curve),
    ))))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetCurveSolid {
        ids: vec![id],
        solid: CurveSolid::Sweep(Sweep {
            profile: SweepProfile::Square,
            ..Sweep::default()
        }),
    })
    .unwrap();
    let done = game_bevel(&mut doc, &[id]).unwrap();
    assert_eq!(done.switched, 1);
    assert_eq!(doc.scene().object(id).unwrap().hardness, Hardness::Marked);
    assert_eq!(kinds(&doc, id), ["Live bevel", "Weighted normals"]);
    let shape = doc.scene().shown_shape(id).unwrap();
    assert!(shape.vertices().all(|(_, p)| p.is_finite()));
}

#[test]
fn bake_into_shape_keeps_it() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cylinder);
    game_bevel(&mut doc, &[id]).unwrap();
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    assert_eq!(**baked, *expected);
    let s = shading(baked, Hardness::Marked);
    assert!(s.flat_lean < 0.1 && s.breaks == 0);
}

#[test]
fn seams_stay_where_the_shape_had_them() {
    // The game bevel marks no seams of its own: the bevel moves each seam
    // to its strip's far border, so every piece of the UV layout stays
    // whole and apart.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    let plain = doc.scene().shown_shape(id).unwrap();
    let pieces = |m: &Mesh| mesh::uvs::seams(m).len();
    game_bevel(&mut doc, &[id]).unwrap();
    let shape = doc.scene().shown_shape(id).unwrap();
    assert!(mesh::uvs::has_uvs(&shape));
    assert!(pieces(&shape) >= pieces(&plain));
}

#[test]
fn shape_changing_modifiers_added_after_it_go_above_its_weighted_normals() {
    // A tester added a Lattice and a Scatter after Game bevel: they went
    // below Weighted normals, which then dropped its normals.
    for kind in ModifierKind::standard_kinds() {
        let finishing = matches!(
            kind,
            ModifierKind::WeightedNormals(_) | ModifierKind::Triangulate(_)
        );
        assert_eq!(
            kind.goes_before(&ModifierKind::WeightedNormals(Default::default())),
            !finishing,
            "{}",
            kind.name()
        );
    }
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    game_bevel(&mut doc, &[id]).unwrap();
    doc.execute(Command::AddLattice {
        ids: vec![id],
        divisions: [2, 2, 2],
    })
    .unwrap();
    add(&mut doc, id, ModifierKind::Array(Default::default()));
    add(&mut doc, id, ModifierKind::Triangulate(Default::default()));
    assert_eq!(
        kinds(&doc, id),
        [
            "Live bevel",
            "Lattice",
            "Array",
            "Weighted normals",
            "Triangulate"
        ]
    );
    // Nothing after the normals changes the shape: they reach the shape.
    assert_eq!(doc.scene().normals_dropped_by(id, 3), None);
}
