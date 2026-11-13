//! Tests for pasting objects copied in another project (decision 0268):
//! everything they need comes along (materials, paint, graphs, own boxes,
//! cutters, what is inside), nothing clashes with what the project has,
//! the paste undoes in one step, the project copied from is untouched,
//! and a fuzz run of random scenes and selections.

use crate::*;
use glam::Vec3;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, OwnBox};
use paint::{GraphFill, Image, Layer, TextureSet};
use std::sync::Arc;

fn cube(doc: &mut Document, at: Vec3) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: at,
            ..Transform::default()
        },
    )]))
    .unwrap();
    id
}

/// An own box called `name` holding a Levels box set to `low`.
fn own_box(name: &str, low: f32) -> OwnBox {
    let registry = matgraph::Registry::standard();
    let mut g = Graph::new("inside");
    g.apply(
        registry,
        &GraphEdit::AddBox {
            kind: "levels".into(),
            at: glam::Vec2::ZERO,
        },
    )
    .unwrap();
    g.apply(
        registry,
        &GraphEdit::SetSetting {
            node: BoxId(1),
            key: "low".into(),
            value: matgraph::SettingValue::Number(low),
        },
    )
    .unwrap();
    OwnBox {
        id: matgraph::own::fresh_id(name, |_| false),
        name: name.into(),
        about: String::new(),
        picture: None,
        inner: Arc::new(g),
        inputs: Vec::new(),
        outputs: Vec::new(),
        sliders: Vec::new(),
    }
}

/// Adds graph `name` to `doc`, holding a Material out box and a box of
/// `own` (if given); returns its id.
fn add_graph(doc: &mut Document, name: &str, own: Option<&OwnBox>) -> GraphId {
    let registry = doc.scene().graphs().registry().clone();
    let mut g = Graph::new(name);
    g.apply(
        &registry,
        &GraphEdit::AddBox {
            kind: "material_out".into(),
            at: glam::Vec2::ZERO,
        },
    )
    .unwrap();
    if let Some(own) = own {
        g.apply(
            &registry,
            &GraphEdit::AddBox {
                kind: own.id.clone(),
                at: glam::Vec2::new(-300.0, 0.0),
            },
        )
        .unwrap();
    }
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(g)).unwrap();
    id
}

/// A texture set with a paint layer (one pixel `rgba`) and, if given, a
/// graph fill running `graph`.
fn set_with(rgba: [u8; 4], graph: Option<GraphId>) -> TextureSet {
    let mut image = Image::new(64).unwrap();
    image.set_pixel(0, 0, rgba);
    let mut set = TextureSet::new(64).unwrap();
    set.layers.push(Layer::paint("Paint", image));
    if let Some(g) = graph {
        let mut fill = Layer::fill("Stone", [0, 0, 0, 255]);
        fill.graph = Some(GraphFill::of(g));
        set.layers.push(fill);
    }
    set
}

fn give_material(doc: &mut Document, id: ObjectId, name: &str) {
    doc.execute(Command::SetMaterials {
        id,
        materials: vec![Material {
            name: name.into(),
            color: [0.5, 0.4, 0.3],
        }],
    })
    .unwrap();
}

/// A project with a painted cube: material Stone, its texture set
/// painted and running graph Cobbles, which uses own box Rocky.
fn painted_project() -> (Document, ObjectId, OwnBox) {
    let mut doc = Document::new();
    let rocky = own_box("Rocky", 0.2);
    doc.execute(Command::SetOwnBox {
        id: rocky.id.clone(),
        own: Some(rocky.clone()),
    })
    .unwrap();
    let graph = add_graph(&mut doc, "Cobbles", Some(&rocky));
    let id = cube(&mut doc, Vec3::new(10.0, 0.0, 0.0));
    give_material(&mut doc, id, "Stone");
    doc.execute(Command::SetTextureSet {
        material: "Stone".into(),
        set: Some(set_with([9, 8, 7, 255], Some(graph))),
    })
    .unwrap();
    (doc, id, rocky)
}

/// Whether two scenes hold the same (the next ids to give out aside:
/// undoing never gives an id out twice).
fn same_content(a: &Scene, b: &Scene) -> bool {
    let objects = |s: &Scene| {
        s.walk()
            .into_iter()
            .map(|(i, d)| (i, d, s.object(i).cloned()))
            .collect::<Vec<_>>()
    };
    let graphs = |s: &Scene| {
        let g: Vec<_> = s.graphs().iter().map(|(i, g)| (i, g.clone())).collect();
        let o: Vec<_> = s.graphs().own_boxes().cloned().collect();
        (g, o)
    };
    objects(a) == objects(b) && a.paint() == b.paint() && graphs(a) == graphs(b)
}

fn paste(doc: &mut Document, clip: &Clipping) -> Result<(), SceneError> {
    doc.execute(Command::Paste(Arc::new(clip.clone())))
}

/// The graph a material's graph fill runs in `doc`.
fn fill_graph(doc: &Document, material: &str) -> Arc<Graph> {
    let set = doc.scene().paint().set(material).unwrap();
    let id = set
        .layers
        .iter()
        .find_map(|l| l.graph.as_ref().map(|g| g.graph))
        .unwrap();
    doc.scene().graphs().get(id).unwrap().clone()
}

#[test]
fn materials_paint_graphs_and_own_boxes_come_along() {
    let (source, id, rocky) = painted_project();
    let before = source.scene().clone();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    assert_eq!(source.scene(), &before, "copying changes nothing");
    assert_eq!(clip.chosen_count(), 1);
    assert_eq!(clip.sets().len(), 1);
    assert_eq!(clip.graphs().len(), 1);
    assert_eq!(clip.own_boxes().len(), 1);

    let mut target = Document::new();
    cube(&mut target, Vec3::ZERO);
    paste(&mut target, &clip).unwrap();
    let scene = target.scene();
    assert_eq!(scene.children(None).len(), 2);
    let pasted = target.selection();
    assert_eq!(pasted.len(), 1);
    let object = scene.object(pasted[0]).unwrap();
    assert_eq!(object.materials[0].name, "Stone");
    assert_eq!(object.transform.position, Vec3::new(10.0, 0.0, 0.0));
    let set = scene.paint().set("Stone").unwrap();
    assert_eq!(
        set.layers[0],
        source.scene().paint().set("Stone").unwrap().layers[0]
    );
    let graph = fill_graph(&target, "Stone");
    assert_eq!(graph.name(), "Cobbles");
    assert!(graph.boxes().any(|(_, b)| b.kind == rocky.id));
    assert_eq!(**scene.graphs().own(&rocky.id).unwrap(), rocky);
    assert_eq!(target.undo_steps(), 2);
    assert_eq!(target.undo_label(), Some("Paste object"));
    assert_eq!(source.scene(), &before, "pasting elsewhere changes nothing");
}

#[test]
fn a_paste_undoes_in_one_step_and_redoes() {
    let (source, id, _) = painted_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let mut target = Document::new();
    cube(&mut target, Vec3::ZERO);
    let before = target.scene().clone();
    paste(&mut target, &clip).unwrap();
    let after = target.scene().clone();
    target.undo().unwrap();
    assert!(same_content(target.scene(), &before));
    target.redo().unwrap();
    assert!(same_content(target.scene(), &after));
}

#[test]
fn ids_and_names_taken_in_the_project_are_not_reused_for_something_else() {
    let (source, id, rocky) = painted_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    // The project has its own graph 1, its own Rocky (same id, another
    // inside) and its own Stone material, painted otherwise.
    let mut target = Document::new();
    let mut other_rocky = own_box("Rocky", 0.9);
    other_rocky.id.clone_from(&rocky.id);
    target
        .execute(Command::SetOwnBox {
            id: rocky.id.clone(),
            own: Some(other_rocky.clone()),
        })
        .unwrap();
    let theirs = add_graph(&mut target, "Cobbles", Some(&other_rocky));
    let mine = cube(&mut target, Vec3::ZERO);
    give_material(&mut target, mine, "Stone");
    let their_set = set_with([1, 1, 1, 255], None);
    target
        .execute(Command::SetTextureSet {
            material: "Stone".into(),
            set: Some(their_set.clone()),
        })
        .unwrap();
    let plan = clip.plan(target.scene()).unwrap();
    assert_eq!(plan.renamed_materials(), vec![("Stone", "Stone 2")]);
    paste(&mut target, &clip).unwrap();
    let scene = target.scene();
    // What the project had is as it was.
    assert_eq!(**scene.graphs().own(&rocky.id).unwrap(), other_rocky);
    assert_eq!(**scene.paint().set("Stone").unwrap(), their_set);
    assert_eq!(scene.graphs().get(theirs).unwrap().name(), "Cobbles");
    // What came in has ids and names of its own.
    let pasted = scene.object(target.selection()[0]).unwrap();
    assert_eq!(pasted.materials[0].name, "Stone 2");
    let graph = fill_graph(&target, "Stone 2");
    assert_eq!(graph.name(), "Cobbles 2");
    let kind = graph
        .boxes()
        .map(|(_, b)| b.kind.clone())
        .find(|k| matgraph::own::is_own(k))
        .unwrap();
    assert_ne!(kind, rocky.id);
    let brought = scene.graphs().own(&kind).unwrap();
    assert_eq!(brought.name, "Rocky 2");
    assert_eq!(brought.inner, rocky.inner);
    assert_eq!(scene.graphs().len(), 2);
    assert_eq!(scene.graphs().own_count(), 2);
}

#[test]
fn pasting_again_reuses_what_the_first_paste_brought() {
    let (source, id, _) = painted_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let mut target = Document::new();
    paste(&mut target, &clip).unwrap();
    paste(&mut target, &clip).unwrap();
    let scene = target.scene();
    assert_eq!(scene.children(None).len(), 2);
    assert_eq!(scene.graphs().len(), 1);
    assert_eq!(scene.graphs().own_count(), 1);
    assert_eq!(scene.paint().sets().count(), 1);
    for top in scene.children(None) {
        assert_eq!(scene.object(*top).unwrap().materials[0].name, "Stone");
    }
    // The second keeps a name of its own.
    let names: Vec<&str> = scene
        .children(None)
        .iter()
        .map(|id| scene.object(*id).unwrap().name.as_str())
        .collect();
    assert_eq!(names, ["Cube", "Cube 2"]);
}

#[test]
fn the_standard_material_s_paint_comes_along_under_a_name_of_its_own() {
    let mut source = Document::new();
    let id = cube(&mut source, Vec3::ZERO);
    let standard = Material::standard().name;
    source
        .execute(Command::SetTextureSet {
            material: standard.clone(),
            set: Some(set_with([5, 5, 5, 255], None)),
        })
        .unwrap();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    // Into an empty project: the same name.
    let mut empty = Document::new();
    paste(&mut empty, &clip).unwrap();
    let pasted = empty.scene().object(empty.selection()[0]).unwrap();
    assert!(pasted.materials.is_empty());
    assert!(empty.scene().paint().set(&standard).is_some());
    // Into a project with other paint on its standard material: its own.
    let mut painted = Document::new();
    painted
        .execute(Command::SetTextureSet {
            material: standard.clone(),
            set: Some(set_with([6, 6, 6, 255], None)),
        })
        .unwrap();
    paste(&mut painted, &clip).unwrap();
    let pasted = painted.scene().object(painted.selection()[0]).unwrap();
    let name = &pasted.materials[0].name;
    assert_ne!(name, &standard);
    assert_eq!(
        painted.scene().paint().set(name).unwrap(),
        source.scene().paint().set(&standard).unwrap()
    );
}

#[test]
fn an_unpainted_material_avoids_a_painted_one_of_the_same_name() {
    let mut source = Document::new();
    let id = cube(&mut source, Vec3::ZERO);
    give_material(&mut source, id, "Wood");
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let mut target = Document::new();
    target
        .execute(Command::SetTextureSet {
            material: "Wood".into(),
            set: Some(set_with([1, 2, 3, 255], None)),
        })
        .unwrap();
    paste(&mut target, &clip).unwrap();
    let pasted = target.scene().object(target.selection()[0]).unwrap();
    assert_eq!(pasted.materials[0].name, "Wood 2");
    // A project using Wood unpainted keeps the name.
    let mut plain = Document::new();
    let other = cube(&mut plain, Vec3::ZERO);
    give_material(&mut plain, other, "Wood");
    paste(&mut plain, &clip).unwrap();
    let pasted = plain.scene().object(plain.selection()[0]).unwrap();
    assert_eq!(pasted.materials[0].name, "Wood");
}

#[test]
fn what_is_inside_comes_along_and_the_top_keeps_its_place_in_the_world() {
    let mut source = Document::new();
    let a = cube(&mut source, Vec3::new(5.0, 0.0, 0.0));
    let b = cube(&mut source, Vec3::new(0.0, 7.0, 0.0));
    source.execute(Command::Group(vec![a, b])).unwrap();
    let group = source.selection()[0];
    source
        .execute(Command::SetTransforms(vec![(
            group,
            Transform {
                position: Vec3::new(100.0, 0.0, 0.0),
                ..Transform::default()
            },
        )]))
        .unwrap();
    // The whole group: its two cubes inside, in order.
    let clip = Clipping::capture(source.scene(), &[group, a]).unwrap();
    assert_eq!(clip.chosen_count(), 1);
    let mut target = Document::new();
    paste(&mut target, &clip).unwrap();
    let scene = target.scene();
    let top = scene.children(None)[0];
    assert_eq!(target.selection(), vec![top]);
    let inside = scene.children(Some(top));
    assert_eq!(inside.len(), 2);
    let names: Vec<_> = inside
        .iter()
        .map(|i| scene.object(*i).unwrap().name.clone())
        .collect();
    let source_names: Vec<_> = source
        .scene()
        .children(Some(group))
        .iter()
        .map(|i| source.scene().object(*i).unwrap().name.clone())
        .collect();
    assert_eq!(names, source_names);
    // Only the cube at 5 cm: it goes at the top, where it was in the
    // world (105 cm).
    let clip = Clipping::capture(source.scene(), &[a]).unwrap();
    let mut target = Document::new();
    paste(&mut target, &clip).unwrap();
    let id = target.selection()[0];
    let at = target.scene().world_matrix(id).w_axis.truncate();
    assert!((at - Vec3::new(105.0, 0.0, 0.0)).length() < 1e-3, "{at}");
}

#[test]
fn a_boolean_s_cutter_comes_along_hidden() {
    let mut source = Document::new();
    let kept = cube(&mut source, Vec3::ZERO);
    let cutter = cube(&mut source, Vec3::new(50.0, 0.0, 0.0));
    source
        .execute(Command::AddBoolean {
            kept,
            cutters: vec![cutter],
            kind: BooleanKind::Subtract,
        })
        .unwrap();
    let clip = Clipping::capture(source.scene(), &[kept]).unwrap();
    assert_eq!((clip.chosen_count(), clip.needed_count()), (1, 1));
    let mut target = Document::new();
    let mine = cube(&mut target, Vec3::ZERO);
    paste(&mut target, &clip).unwrap();
    let scene = target.scene();
    let pasted = target.selection()[0];
    let cutters = &scene.booleans(pasted)[0].cutters;
    assert_eq!(cutters.len(), 1);
    assert_ne!(cutters[0], mine);
    assert_eq!(
        scene.object(cutters[0]).unwrap().visible,
        source.scene().object(cutter).unwrap().visible
    );
    assert!(scene.cut_shape(pasted).unwrap().is_ok());
}

#[test]
fn an_instance_without_its_original_becomes_a_copy_of_its_shape() {
    let mut source = Document::new();
    let original = cube(&mut source, Vec3::ZERO);
    source
        .execute(Command::Duplicate {
            ids: vec![original],
            instances: true,
        })
        .unwrap();
    let instance = source.selection()[0];
    // Only the instance: a copy of the cube.
    let clip = Clipping::capture(source.scene(), &[instance]).unwrap();
    let mut target = Document::new();
    paste(&mut target, &clip).unwrap();
    let pasted = target.scene().object(target.selection()[0]).unwrap();
    assert!(matches!(pasted.kind, ObjectKind::Primitive(_)));
    // Both: the instance shows the pasted original.
    let clip = Clipping::capture(source.scene(), &[original, instance]).unwrap();
    let mut target = Document::new();
    paste(&mut target, &clip).unwrap();
    let scene = target.scene();
    let ids = target.selection();
    let shown = ids
        .iter()
        .find_map(|i| match scene.object(*i).unwrap().kind {
            ObjectKind::Instance(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert!(ids.contains(&shown));
}

#[test]
fn nothing_to_copy_gives_no_clipping() {
    let mut doc = Document::new();
    assert!(Clipping::capture(doc.scene(), &[]).is_none());
    let id = cube(&mut doc, Vec3::ZERO);
    doc.execute(Command::Delete(vec![id])).unwrap();
    assert!(Clipping::capture(doc.scene(), &[id]).is_none());
    // An empty clipping pastes nothing.
    let before = doc.scene().clone();
    paste(&mut doc, &Clipping::default()).unwrap();
    assert_eq!(doc.scene(), &before);
}

#[test]
fn a_paste_that_can_t_fit_is_refused_and_changes_nothing() {
    let (source, id, _) = painted_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let mut full = Document::new();
    let graphs =
        (1..=matgraph::MAX_GRAPHS as u64).map(|i| (GraphId(i), Graph::new(format!("G{i}"))));
    let store = matgraph::GraphStore::from_graphs(graphs, 1).unwrap();
    full = Document::from_scene(full.scene().clone().with_graphs(store), Vec::new());
    let before = full.scene().clone();
    let why = paste(&mut full, &clip).unwrap_err();
    assert!(why.to_string().contains("material graphs"), "{why}");
    assert_eq!(full.scene(), &before);
}

/// A small random number maker, so the fuzz run is the same every time.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n.max(1)
    }
}

/// A random project: cubes, groups, instances, booleans and materials,
/// some painted.
fn random_project(rng: &mut Lcg) -> Document {
    let mut doc = Document::new();
    for _ in 0..(3 + rng.next(8)) {
        let ids: Vec<ObjectId> = doc.scene().walk().into_iter().map(|(i, _)| i).collect();
        let pick = |rng: &mut Lcg| ids[rng.next(ids.len())];
        match rng.next(6) {
            _ if ids.is_empty() => {
                cube(&mut doc, Vec3::new(rng.next(100) as f32, 0.0, 0.0));
            }
            0 | 1 => {
                cube(&mut doc, Vec3::new(rng.next(100) as f32, 0.0, 0.0));
            }
            2 => {
                let _ = doc.execute(Command::Group(vec![pick(rng), pick(rng)]));
            }
            3 => {
                let _ = doc.execute(Command::Duplicate {
                    ids: vec![pick(rng)],
                    instances: rng.next(2) == 0,
                });
            }
            4 => {
                let _ = doc.execute(Command::AddBoolean {
                    kept: pick(rng),
                    cutters: vec![pick(rng)],
                    kind: BooleanKind::Subtract,
                });
            }
            _ => {
                let id = pick(rng);
                let name = ["Wood", "Stone", "Metal"][rng.next(3)];
                let _ = doc.execute(Command::SetMaterials {
                    id,
                    materials: vec![Material {
                        name: name.into(),
                        color: [0.2, 0.2, 0.2],
                    }],
                });
                if rng.next(2) == 0 {
                    let shade = rng.next(255) as u8;
                    let _ = doc.execute(Command::SetTextureSet {
                        material: name.into(),
                        set: Some(set_with([shade, 0, 0, 255], None)),
                    });
                }
            }
        }
    }
    doc
}

#[test]
fn random_projects_and_selections_paste_and_undo_cleanly() {
    let mut rng = Lcg(268);
    for round in 0..60 {
        let source = random_project(&mut rng);
        let mut target = random_project(&mut rng);
        let all: Vec<ObjectId> = source.scene().walk().into_iter().map(|(i, _)| i).collect();
        let chosen: Vec<ObjectId> = all.iter().copied().filter(|_| rng.next(3) == 0).collect();
        let source_before = source.scene().clone();
        let Some(clip) = Clipping::capture(source.scene(), &chosen) else {
            assert!(chosen.is_empty());
            continue;
        };
        let before = target.scene().clone();
        let count = before.walk().len();
        paste(&mut target, &clip).unwrap_or_else(|e| panic!("round {round}: {e}"));
        let scene = target.scene();
        assert_eq!(scene.walk().len(), count + clip.objects().len());
        // Every material pasted has the paint it came with.
        for id in target.selection() {
            let mut inside = vec![id];
            while let Some(i) = inside.pop() {
                inside.extend(scene.children(Some(i)));
                for m in &scene.object(i).unwrap().materials {
                    if let Some(set) = scene.paint().set(&m.name) {
                        assert!(set.is_valid());
                    }
                }
            }
        }
        // Instances show objects that are there.
        for (i, _) in scene.walk() {
            if let ObjectKind::Instance(s) = scene.object(i).unwrap().kind {
                assert!(scene.contains(s), "round {round}");
            }
        }
        // The project is still one that saves and opens again.
        let objects: Vec<_> = scene
            .walk()
            .into_iter()
            .map(|(i, _)| (i, scene.parent(i), scene.object(i).unwrap().clone()))
            .collect();
        assert!(Scene::from_objects(objects, 0).is_ok(), "round {round}");
        target.undo().unwrap();
        assert!(same_content(target.scene(), &before), "round {round}");
        assert_eq!(source.scene(), &source_before);
    }
}

#[test]
fn graphs_bring_their_variations_and_layers_keep_their_pick() {
    let (mut source, id, _) = painted_project();
    // The graph gets a variation, and the Stone layer picks it.
    let graph_id = source.scene().paint().set("Stone").unwrap().layers[1]
        .graph
        .as_ref()
        .unwrap()
        .graph;
    let registry = source.scene().graphs().registry().clone();
    let graph = source.scene().graphs().get(graph_id).unwrap().clone();
    let variation = matgraph::VariationId(graph.next_id());
    let look = graph.variation_now(&registry, "Mossy");
    source
        .execute(Command::EditGraph {
            id: graph_id,
            edit: GraphEdit::AddVariation(look),
        })
        .unwrap();
    let mut set = (**source.scene().paint().set("Stone").unwrap()).clone();
    set.layers[1].graph.as_mut().unwrap().variation = Some(variation);
    source
        .execute(Command::SetTextureSet {
            material: "Stone".into(),
            set: Some(set),
        })
        .unwrap();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    // Pasted where an own box of the same id differs: the graph is
    // rewritten for the renamed box, and keeps its variations.
    let mut target = Document::new();
    let clash = own_box("Rocky", 0.9);
    target
        .execute(Command::SetOwnBox {
            id: clash.id.clone(),
            own: Some(clash),
        })
        .unwrap();
    cube(&mut target, Vec3::ZERO);
    paste(&mut target, &clip).unwrap();
    let pasted = fill_graph(&target, "Stone");
    let names: Vec<&str> = pasted.variations().map(|(_, v)| v.name.as_str()).collect();
    assert_eq!(names, ["Mossy"]);
    let set = target.scene().paint().set("Stone").unwrap();
    let fill = set.layers[1].graph.as_ref().unwrap();
    assert_eq!(
        pasted.variation(fill.variation.unwrap()).unwrap().name,
        "Mossy"
    );
}

/// A palette called `name` of `colours`.
fn palette(name: &str, colours: &[[u8; 3]]) -> paint::palette::Palette {
    paint::palette::Palette {
        name: name.into(),
        colours: colours.to_vec(),
    }
}

/// `painted_project` with palettes Moss and Unused, and a Palette map
/// box in its graph set to Moss.
fn palette_project() -> (Document, ObjectId) {
    let (mut doc, id, _) = painted_project();
    doc.execute(Command::SetPalettes(vec![
        palette("Moss", &[[40, 90, 30], [120, 170, 60]]),
        palette("Unused", &[[1, 1, 1]]),
    ]))
    .unwrap();
    let graph = fill_graph(&doc, "Stone");
    let gid = doc
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| Arc::ptr_eq(g, &graph))
        .unwrap()
        .0;
    let map = BoxId(graph.next_id());
    for edit in [
        GraphEdit::AddBox {
            kind: "palette_map".into(),
            at: glam::Vec2::new(-600.0, 0.0),
        },
        GraphEdit::SetSetting {
            node: map,
            key: "palette".into(),
            value: matgraph::SettingValue::Palette("Moss".into()),
        },
    ] {
        doc.execute(Command::EditGraph { id: gid, edit }).unwrap();
    }
    (doc, id)
}

#[test]
fn the_palettes_a_pasted_graph_names_come_along_once() {
    let (source, id) = palette_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let names: Vec<&str> = clip.palettes().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Moss"], "only the palettes named come along");

    let mut target = Document::new();
    cube(&mut target, Vec3::ZERO);
    target
        .execute(Command::SetPalettes(vec![palette("Mine", &[[5, 5, 5]])]))
        .unwrap();
    let before = target.scene().clone();
    paste(&mut target, &clip).unwrap();
    let palettes: Vec<&str> = target
        .scene()
        .paint()
        .palettes()
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(palettes, ["Mine", "Moss"]);
    // The graphs see them at once.
    assert!(
        target
            .scene()
            .graphs()
            .registry()
            .palettes()
            .find("Moss")
            .is_some()
    );
    // One undo step takes it all back, palettes too.
    target.undo().unwrap();
    assert!(same_content(target.scene(), &before));
    assert_eq!(target.scene().paint().palettes().len(), 1);
    assert!(
        target
            .scene()
            .graphs()
            .registry()
            .palettes()
            .find("Moss")
            .is_none()
    );
    target.redo().unwrap();
    // Pasting again brings no second Moss.
    paste(&mut target, &clip).unwrap();
    assert_eq!(target.scene().paint().palettes().len(), 2);
}

#[test]
fn a_palette_of_the_same_name_in_the_project_is_kept() {
    let (source, id) = palette_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let mut target = Document::new();
    let theirs = palette("Moss", &[[0, 0, 255]]);
    target
        .execute(Command::SetPalettes(vec![theirs.clone()]))
        .unwrap();
    paste(&mut target, &clip).unwrap();
    assert_eq!(target.scene().paint().palettes(), [theirs]);
}

#[test]
fn a_paste_with_no_room_for_its_palettes_is_refused() {
    let (source, id) = palette_project();
    let clip = Clipping::capture(source.scene(), &[id]).unwrap();
    let mut target = Document::new();
    let full: Vec<_> = (0..paint::palette::MAX_PALETTES)
        .map(|i| palette(&format!("P{i}"), &[[0, 0, 0]]))
        .collect();
    target.execute(Command::SetPalettes(full)).unwrap();
    let before = target.scene().clone();
    let why = paste(&mut target, &clip).unwrap_err();
    assert!(why.to_string().contains("palettes"), "{why}");
    assert!(same_content(target.scene(), &before));
}
