//! Tests for the scene document: commands, undo and redo, and the tree.

use crate::*;

/// The standard 2 m cube.
fn cube() -> Object {
    Object::new(ObjectKind::Primitive(Primitive::Cube(Default::default())))
}

/// A document holding just a cube (created, so it can be undone), and its id.
fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Insert {
        object: cube(),
        place: Place {
            parent: None,
            index: 0,
        },
    })
    .unwrap();
    let id = doc.scene().children(None)[0];
    doc.clear_selection();
    (doc, id)
}

/// The cube's box.
fn cube_bounds() -> Bounds {
    Primitive::Cube(Default::default()).bounds()
}

/// A document with objects named `names`, top to bottom at the top level,
/// and their ids.
fn with_groups(names: &[&str]) -> (Document, Vec<ObjectId>) {
    let mut doc = Document::new();
    for (index, name) in names.iter().enumerate() {
        let mut object = Object::new(ObjectKind::Group);
        object.name = (*name).to_string();
        doc.execute(Command::Insert {
            object,
            place: Place {
                parent: None,
                index,
            },
        })
        .unwrap();
    }
    let ids = doc.scene().children(None).to_vec();
    (doc, ids)
}

/// The tree as text, e.g. "A[B C] D", for short, readable checks.
fn outline(doc: &Document) -> String {
    fn level(scene: &Scene, parent: Option<ObjectId>) -> String {
        scene
            .children(parent)
            .iter()
            .map(|id| {
                let name = scene.object(*id).unwrap().name.clone();
                if scene.children(Some(*id)).is_empty() {
                    name
                } else {
                    format!("{name}[{}]", level(scene, Some(*id)))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    level(doc.scene(), None)
}

/// Undoes everything and redoes it all, checking each state comes back
/// exactly (ids included).
fn check_round_trip(doc: &mut Document) {
    let mut states = vec![doc.scene().clone()];
    while doc.undo().is_some() {
        states.push(doc.scene().clone());
    }
    states.pop();
    while let Some(expected) = states.pop() {
        doc.redo().unwrap();
        assert_eq!(*doc.scene(), expected);
    }
    assert!(doc.redo().is_none());
}

// ---------------------------------------------------------------- create ---

#[test]
fn created_objects_go_on_top_named_and_selected() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(outline(&doc), "Group 3 Group 2 Group");
    let top = doc.scene().children(None)[0];
    assert_eq!(doc.selection(), [top]);
    assert_eq!(doc.undo_label(), Some("Create group"));
}

#[test]
fn ids_are_never_reused_and_come_back_the_same() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let first = doc.selection()[0];
    doc.execute(Command::Delete(vec![first])).unwrap();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let second = doc.selection()[0];
    assert_ne!(first, second, "a deleted object's id is not reused");
    doc.undo();
    doc.undo();
    assert!(
        doc.scene().contains(first),
        "undo brings back the very same id"
    );
}

#[test]
fn primitives_are_named_after_their_kind() {
    let mut doc = Document::new();
    for shape in Primitive::all() {
        doc.execute(Command::Create(ObjectKind::Primitive(shape)))
            .unwrap();
    }
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::all()[0])))
        .unwrap();
    assert_eq!(
        outline(&doc),
        "Cube 2 Cone Tube Disc Sphere Cylinder Plane Cube"
    );
    assert_eq!(doc.undo_label(), Some("Create cube"));
}

#[test]
fn inserting_needs_a_name_and_a_real_place() {
    let mut doc = Document::new();
    let mut nameless = Object::new(ObjectKind::Group);
    nameless.name = "  ".into();
    let at_top = Place {
        parent: None,
        index: 0,
    };
    assert_eq!(
        doc.execute(Command::Insert {
            object: nameless,
            place: at_top
        }),
        Err(SceneError::EmptyName)
    );
    let far = Place {
        parent: None,
        index: 5,
    };
    assert!(
        doc.execute(Command::Insert {
            object: Object::new(ObjectKind::Group),
            place: far
        })
        .is_err()
    );
    assert!(doc.scene().is_empty());
    assert_eq!(doc.undo_label(), None, "failures record nothing");
}

// ---------------------------------------------------------------- rename ---

#[test]
fn renaming_trims_and_refuses_empty_names() {
    let (mut doc, ids) = with_groups(&["A"]);
    doc.execute(Command::Rename {
        id: ids[0],
        name: "  Wheel  ".into(),
    })
    .unwrap();
    assert_eq!(outline(&doc), "Wheel");
    assert_eq!(
        doc.execute(Command::Rename {
            id: ids[0],
            name: " ".into()
        }),
        Err(SceneError::EmptyName)
    );
    let steps = doc.undo_steps();
    doc.execute(Command::Rename {
        id: ids[0],
        name: "Wheel".into(),
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps, "the same name is not a step");
    doc.undo();
    assert_eq!(outline(&doc), "A");
}

#[test]
fn unused_names_count_up() {
    let (doc, _) = with_groups(&["Group", "Group 2"]);
    assert_eq!(doc.scene().unused_name("Group"), "Group 3");
    assert_eq!(doc.scene().unused_name("Cube"), "Cube");
}

// ------------------------------------------------------------ visibility ---

#[test]
fn hiding_a_parent_hides_what_is_inside() {
    let (mut doc, ids) = with_groups(&["A", "B"]);
    doc.execute(Command::Move {
        ids: vec![ids[1]],
        parent: Some(ids[0]),
        index: 0,
    })
    .unwrap();
    doc.execute(Command::SetVisible {
        ids: vec![ids[0]],
        visible: false,
    })
    .unwrap();
    assert!(!doc.scene().is_shown(ids[1]), "inside a hidden group");
    assert!(
        doc.scene().object(ids[1]).unwrap().visible,
        "its own switch is still on"
    );
    assert_eq!(doc.undo_label(), Some("Hide object"));
    doc.undo();
    assert!(doc.scene().is_shown(ids[1]));
}

#[test]
fn showing_what_is_shown_is_not_a_step() {
    let (mut doc, ids) = with_groups(&["A"]);
    let steps = doc.undo_steps();
    doc.execute(Command::SetVisible {
        ids: ids.clone(),
        visible: true,
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn hidden_objects_are_left_out_of_the_bounds() {
    let (mut doc, cube) = with_a_cube();
    assert_eq!(doc.scene().bounds(), Some(cube_bounds()));
    doc.execute(Command::SetVisible {
        ids: vec![cube],
        visible: false,
    })
    .unwrap();
    assert_eq!(doc.scene().bounds(), None);
    assert_eq!(doc.scene().bounds_of([cube]), None);
}

// ---------------------------------------------------------------- moving ---

#[test]
fn moving_into_another_object_nests_it() {
    let (mut doc, ids) = with_groups(&["A", "B", "C"]);
    doc.execute(Command::Move {
        ids: vec![ids[2], ids[0]],
        parent: Some(ids[1]),
        index: 0,
    })
    .unwrap();
    assert_eq!(outline(&doc), "B[A C]", "moved objects keep their order");
    assert_eq!(doc.scene().parent(ids[0]), Some(ids[1]));
    check_round_trip(&mut doc);
}

#[test]
fn moving_reorders_among_brothers_and_sisters() {
    let (mut doc, ids) = with_groups(&["A", "B", "C", "D"]);
    // Before D (index 3, counted before the move).
    doc.execute(Command::Move {
        ids: vec![ids[0]],
        parent: None,
        index: 3,
    })
    .unwrap();
    assert_eq!(outline(&doc), "B C A D");
    // To the end.
    doc.execute(Command::Move {
        ids: vec![ids[1]],
        parent: None,
        index: 4,
    })
    .unwrap();
    assert_eq!(outline(&doc), "C A D B");
    // To the top.
    doc.execute(Command::Move {
        ids: vec![ids[3]],
        parent: None,
        index: 0,
    })
    .unwrap();
    assert_eq!(outline(&doc), "D C A B");
    check_round_trip(&mut doc);
    assert_eq!(outline(&doc), "D C A B");
}

#[test]
fn moving_to_where_it_already_is_changes_nothing() {
    let (mut doc, ids) = with_groups(&["A", "B"]);
    let steps = doc.undo_steps();
    for index in [0, 1] {
        doc.execute(Command::Move {
            ids: vec![ids[0]],
            parent: None,
            index,
        })
        .unwrap();
    }
    assert_eq!(outline(&doc), "A B");
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn nothing_can_move_inside_itself() {
    let (mut doc, ids) = with_groups(&["A", "B"]);
    doc.execute(Command::Move {
        ids: vec![ids[1]],
        parent: Some(ids[0]),
        index: 0,
    })
    .unwrap();
    let before = doc.clone();
    for parent in [ids[0], ids[1]] {
        assert_eq!(
            doc.execute(Command::Move {
                ids: vec![ids[0]],
                parent: Some(parent),
                index: 0
            }),
            Err(SceneError::NestedInItself)
        );
    }
    assert_eq!(doc, before, "nothing changed");
}

#[test]
fn moving_a_parent_and_its_child_moves_the_child_along() {
    let (mut doc, ids) = with_groups(&["A", "B", "C"]);
    doc.execute(Command::Move {
        ids: vec![ids[1]],
        parent: Some(ids[0]),
        index: 0,
    })
    .unwrap();
    doc.execute(Command::Move {
        ids: vec![ids[0], ids[1]],
        parent: Some(ids[2]),
        index: 0,
    })
    .unwrap();
    assert_eq!(outline(&doc), "C[A[B]]");
    assert_eq!(doc.undo_label(), Some("Move object"));
}

#[test]
fn moving_unknown_objects_fails_cleanly() {
    let (mut doc, ids) = with_groups(&["A"]);
    let ghost = ObjectId(999);
    let before = doc.clone();
    assert_eq!(
        doc.execute(Command::Move {
            ids: vec![ids[0]],
            parent: Some(ghost),
            index: 0
        }),
        Err(SceneError::UnknownObject(ghost))
    );
    assert_eq!(
        doc.execute(Command::Move {
            ids: vec![ghost],
            parent: None,
            index: 0
        }),
        Err(SceneError::UnknownObject(ghost))
    );
    assert_eq!(doc, before);
}

// -------------------------------------------------------------- deleting ---

#[test]
fn deleting_takes_everything_inside_along_and_undo_restores_it() {
    let (mut doc, ids) = with_groups(&["A", "B", "C", "D"]);
    doc.execute(Command::Move {
        ids: vec![ids[1], ids[2]],
        parent: Some(ids[0]),
        index: 0,
    })
    .unwrap();
    doc.execute(Command::Move {
        ids: vec![ids[2]],
        parent: Some(ids[1]),
        index: 0,
    })
    .unwrap();
    assert_eq!(outline(&doc), "A[B[C]] D");
    doc.set_selection([ids[0], ids[3]]);
    doc.execute(Command::Delete(vec![ids[0], ids[2]])).unwrap();
    assert_eq!(outline(&doc), "D");
    assert_eq!(
        doc.undo_label(),
        Some("Delete object"),
        "C went with A: one object"
    );
    assert!(doc.selection().is_empty());
    doc.undo();
    assert_eq!(outline(&doc), "A[B[C]] D");
    doc.redo();
    check_round_trip(&mut doc);
}

#[test]
fn deleting_nothing_is_not_a_step() {
    let (mut doc, _) = with_groups(&["A"]);
    let steps = doc.undo_steps();
    doc.execute(Command::Delete(Vec::new())).unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn deleting_the_same_object_twice_in_one_go_deletes_it_once() {
    let (mut doc, ids) = with_groups(&["A", "B"]);
    doc.execute(Command::Delete(vec![ids[0], ids[0]])).unwrap();
    assert_eq!(outline(&doc), "B");
    doc.undo();
    assert_eq!(outline(&doc), "A B");
}

// -------------------------------------------------------------- grouping ---

#[test]
fn grouping_puts_objects_in_a_new_group_where_the_first_was() {
    let (mut doc, ids) = with_groups(&["A", "B", "C", "D"]);
    doc.execute(Command::Group(vec![ids[2], ids[1]])).unwrap();
    assert_eq!(outline(&doc), "A Group[B C] D");
    let group = doc.scene().children(None)[1];
    assert_eq!(doc.selection(), [group]);
    assert_eq!(doc.undo_label(), Some("Group 2 objects"));
    check_round_trip(&mut doc);
}

#[test]
fn grouping_objects_from_different_places_gathers_them() {
    let (mut doc, ids) = with_groups(&["A", "B", "C"]);
    doc.execute(Command::Move {
        ids: vec![ids[1]],
        parent: Some(ids[0]),
        index: 0,
    })
    .unwrap();
    doc.execute(Command::Group(vec![ids[1], ids[2]])).unwrap();
    assert_eq!(outline(&doc), "A[Group[B C]]");
}

#[test]
fn ungrouping_puts_the_contents_where_the_group_was() {
    let (mut doc, ids) = with_groups(&["A", "B", "C", "D"]);
    doc.execute(Command::Group(vec![ids[1], ids[2]])).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::Ungroup(vec![group])).unwrap();
    assert_eq!(outline(&doc), "A B C D");
    assert_eq!(doc.selection(), [ids[1], ids[2]]);
    assert!(!doc.scene().contains(group));
    check_round_trip(&mut doc);
}

#[test]
fn ungrouping_an_empty_group_just_removes_it() {
    let (mut doc, ids) = with_groups(&["A", "B"]);
    doc.execute(Command::Ungroup(vec![ids[0]])).unwrap();
    assert_eq!(outline(&doc), "B");
    assert!(doc.selection().is_empty());
}

#[test]
fn ungrouping_never_touches_objects_that_are_not_groups() {
    // Regression: Shift+G with the test cube selected deleted the cube.
    let (mut doc, cube) = with_a_cube();
    doc.set_selection([cube]);
    let before = doc.clone();
    doc.execute(Command::Ungroup(vec![cube])).unwrap();
    assert_eq!(doc, before, "nothing changed, no undo step");

    // Not even when something has been put inside the cube.
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::Move {
        ids: vec![group],
        parent: Some(cube),
        index: 0,
    })
    .unwrap();
    let before = doc.scene().clone();
    doc.execute(Command::Ungroup(vec![cube])).unwrap();
    assert_eq!(*doc.scene(), before);
}

#[test]
fn ungrouping_a_mixed_selection_only_ungroups_the_groups() {
    let (mut doc, cube) = with_a_cube();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let empty = doc.selection()[0];
    doc.execute(Command::Group(vec![cube])).unwrap();
    let holder = doc.selection()[0];
    assert_eq!(outline(&doc), "Group Group 2[Cube]");
    doc.execute(Command::Ungroup(vec![holder, cube, empty]))
        .unwrap();
    assert_eq!(outline(&doc), "Cube", "both groups gone, the cube kept");
    assert_eq!(doc.selection(), [cube]);
    assert_eq!(doc.undo_label(), Some("Ungroup 2 groups"));
}

#[test]
fn grouping_nothing_does_nothing() {
    let (mut doc, _) = with_groups(&["A"]);
    let before = doc.clone();
    doc.execute(Command::Group(Vec::new())).unwrap();
    assert_eq!(doc, before);
}

// ------------------------------------------------------- undo and redo ---

#[test]
fn undo_and_redo_report_what_they_did() {
    let mut doc = Document::new();
    assert_eq!(doc.undo(), None);
    assert_eq!(doc.redo(), None);
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(doc.undo().as_deref(), Some("Create group"));
    assert_eq!(doc.redo_label(), Some("Create group"));
    assert_eq!(doc.redo().as_deref(), Some("Create group"));
    assert_eq!(outline(&doc), "Group");
}

#[test]
fn the_revision_goes_up_with_every_change_and_only_then() {
    let mut doc = Document::new();
    assert_eq!(doc.revision(), 0);
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(doc.revision(), 1);
    doc.clear_selection();
    doc.execute(Command::Delete(Vec::new())).unwrap();
    assert_eq!(doc.revision(), 1, "selection and no-ops don't count");
    doc.undo();
    doc.redo();
    assert_eq!(doc.revision(), 3);
}

#[test]
fn a_new_change_forgets_what_could_be_redone() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    doc.undo();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(doc.redo_label(), None);
}

#[test]
fn undo_drops_vanished_objects_from_the_selection() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(doc.selection().len(), 1);
    doc.undo();
    assert!(doc.selection().is_empty());
}

#[test]
fn a_long_mixed_history_undoes_and_redoes_exactly() {
    let (mut doc, ids) = with_groups(&["A", "B", "C", "D", "E"]);
    let commands = [
        Command::Group(vec![ids[0], ids[1]]),
        Command::Move {
            ids: vec![ids[4]],
            parent: None,
            index: 0,
        },
        Command::SetVisible {
            ids: vec![ids[2], ids[3]],
            visible: false,
        },
        Command::Rename {
            id: ids[3],
            name: "Dee".into(),
        },
        Command::Move {
            ids: vec![ids[3]],
            parent: Some(ids[2]),
            index: 0,
        },
        Command::Delete(vec![ids[1]]),
        Command::Create(ObjectKind::Group),
        Command::Ungroup(vec![ids[2]]),
    ];
    let start = (outline(&doc), doc.scene().walk());
    let setup_steps = doc.undo_steps();
    for command in commands {
        doc.execute(command).unwrap();
    }
    let end = doc.scene().clone();
    check_round_trip(&mut doc);
    assert_eq!(*doc.scene(), end);
    while doc.undo_steps() > setup_steps {
        doc.undo();
    }
    // The same objects with the same ids (only the counter for new ids
    // has moved on: ids are never reused).
    assert_eq!((outline(&doc), doc.scene().walk()), start);
}

// ----------------------------------------------------------------- tree ---

#[test]
fn walking_lists_each_object_with_its_depth() {
    let (mut doc, ids) = with_groups(&["A", "B", "C"]);
    doc.execute(Command::Move {
        ids: vec![ids[1]],
        parent: Some(ids[0]),
        index: 0,
    })
    .unwrap();
    let walk = doc.scene().walk();
    assert_eq!(walk, [(ids[0], 0), (ids[1], 1), (ids[2], 0)]);
    assert!(doc.scene().is_inside(ids[1], ids[0]));
    assert!(!doc.scene().is_inside(ids[0], ids[1]));
    assert_eq!(doc.scene().subtree(ids[0]), [ids[0], ids[1]]);
    assert!(doc.scene().subtree(ObjectId(999)).is_empty());
    assert_eq!(
        doc.scene().place(ids[1]),
        Some(Place {
            parent: Some(ids[0]),
            index: 0
        })
    );
}

#[test]
fn world_bounds_follow_the_parents_transforms() {
    let mut doc = Document::new();
    let mut group = Object::new(ObjectKind::Group);
    group.transform.position = glam::Vec3::new(500.0, 0.0, 0.0);
    doc.execute(Command::Insert {
        object: group,
        place: Place {
            parent: None,
            index: 0,
        },
    })
    .unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::Insert {
        object: cube(),
        place: Place {
            parent: Some(group),
            index: 0,
        },
    })
    .unwrap();
    let bounds = doc.scene().bounds_of([group]).unwrap();
    assert_eq!(
        bounds.centre(),
        cube_bounds().centre() + glam::Vec3::new(500.0, 0.0, 0.0)
    );
    // A group alone has no shape.
    let mut empty = Document::new();
    empty.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(empty.scene().bounds(), None);
}

#[test]
fn errors_read_as_plain_sentences() {
    assert_eq!(SceneError::EmptyName.to_string(), "a name can't be empty");
    assert_eq!(
        SceneError::UnknownObject(ObjectId(4)).to_string(),
        "there is no object #4"
    );
}

#[test]
fn selection_ignores_unknown_objects_and_toggles() {
    let (mut doc, ids) = with_groups(&["A", "B"]);
    doc.set_selection([ids[1], ObjectId(999)]);
    assert_eq!(doc.selection(), [ids[1]]);
    doc.toggle_selected(ids[0]);
    assert_eq!(doc.selection(), [ids[0], ids[1]], "listed top to bottom");
    doc.toggle_selected(ids[1]);
    assert_eq!(doc.selection(), [ids[0]]);
    doc.toggle_selected(ObjectId(999));
    assert_eq!(doc.selection(), [ids[0]]);
    doc.clear_selection();
    assert!(doc.selection().is_empty());
}

// ------------------------------------------------------------------ fuzz ---

/// A small, repeatable random number source, so the fuzz test needs no
/// outside crate and fails the same way every time.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

/// Every object is listed exactly once, under the parent it names.
fn check_tree(scene: &Scene) {
    let walk = scene.walk();
    assert_eq!(walk.len(), scene.len(), "every object listed once");
    for (id, _) in &walk {
        let place = scene.place(*id).expect("listed objects have a place");
        assert_eq!(scene.children(place.parent)[place.index], *id);
    }
}

#[test]
fn random_commands_keep_the_tree_whole_and_undo_exactly() {
    for seed in 1..=20 {
        let mut rng = Lcg(seed);
        let mut doc = Document::new();
        let mut states = vec![doc.scene().clone()];
        for _ in 0..80 {
            let ids: Vec<ObjectId> = doc.scene().walk().into_iter().map(|(id, _)| id).collect();
            let pick = |rng: &mut Lcg| -> Vec<ObjectId> {
                (0..rng.next(3) + 1)
                    .filter_map(|_| ids.get(rng.next(ids.len())).copied())
                    .collect()
            };
            let command = match rng.next(9) {
                0 | 1 => Command::Create(ObjectKind::Group),
                7 => Command::Create(ObjectKind::Primitive(cube_of(10.0 + rng.next(50) as f32))),
                8 => Command::SetPrimitive {
                    ids: pick(&mut rng),
                    primitive: cube_of(rng.next(400) as f32),
                },
                2 => {
                    let picked = pick(&mut rng);
                    let parent = if rng.next(3) == 0 {
                        None
                    } else {
                        ids.get(rng.next(ids.len())).copied()
                    };
                    let index = rng.next(4);
                    Command::Move {
                        ids: picked,
                        parent,
                        index,
                    }
                }
                3 => Command::Delete(pick(&mut rng)),
                4 => Command::Group(pick(&mut rng)),
                5 => Command::Ungroup(pick(&mut rng)),
                _ => Command::SetVisible {
                    ids: pick(&mut rng),
                    visible: rng.next(2) == 0,
                },
            };
            let before = doc.clone();
            match doc.execute(command.clone()) {
                Ok(()) => {
                    if doc.undo_steps() > before.undo_steps() {
                        states.push(doc.scene().clone());
                    }
                }
                // Refused commands (moving into itself) change nothing.
                Err(_) => assert_eq!(doc, before, "{command:?}"),
            }
            check_tree(doc.scene());
        }
        let end = doc.scene().clone();
        // Undo everything, checking each earlier state comes back.
        states.pop();
        while let Some(expected) = states.pop() {
            doc.undo().unwrap();
            check_tree(doc.scene());
            assert_eq!(doc.scene().walk(), expected.walk(), "seed {seed}");
        }
        assert!(doc.scene().is_empty());
        // And redo it all.
        while doc.redo().is_some() {
            check_tree(doc.scene());
        }
        assert_eq!(*doc.scene(), end, "seed {seed}");
    }
}

// ------------------------------------------------------------- settings ---

fn cube_size(doc: &Document, id: ObjectId) -> glam::Vec3 {
    match doc.scene().object(id).unwrap().kind {
        ObjectKind::Primitive(Primitive::Cube(p)) => p.size,
        other => panic!("not a cube: {other:?}"),
    }
}

fn cube_of(size: f32) -> Primitive {
    Primitive::Cube(mesh::CubeParams {
        size: glam::Vec3::splat(size),
        ..Default::default()
    })
}

#[test]
fn primitive_settings_change_undoably() {
    let (mut doc, cube) = with_a_cube();
    let revision = doc.revision();
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: cube_of(50.0),
    })
    .unwrap();
    assert_eq!(cube_size(&doc, cube), glam::Vec3::splat(50.0));
    assert!(doc.revision() > revision, "redraw");
    assert_eq!(doc.undo_label(), Some("Change cube settings"));
    doc.undo();
    assert_eq!(cube_size(&doc, cube), glam::Vec3::splat(100.0));
}

#[test]
fn settings_are_brought_within_limits_when_set() {
    let (mut doc, cube) = with_a_cube();
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: cube_of(-5.0),
    })
    .unwrap();
    assert_eq!(
        cube_size(&doc, cube),
        glam::Vec3::splat(mesh::primitives::MIN_SIZE)
    );
}

#[test]
fn settings_of_one_kind_cant_go_to_another() {
    let (mut doc, cube) = with_a_cube();
    let group = {
        doc.execute(Command::Create(ObjectKind::Group)).unwrap();
        doc.selection()[0]
    };
    let before = doc.clone();
    let sphere = Primitive::Sphere(Default::default());
    assert_eq!(
        doc.execute(Command::SetPrimitive {
            ids: vec![cube],
            primitive: sphere
        }),
        Err(SceneError::WrongKind("Sphere"))
    );
    assert!(
        doc.execute(Command::SetPrimitive {
            ids: vec![group],
            primitive: cube_of(10.0)
        })
        .is_err()
    );
    assert_eq!(doc, before);
    assert_eq!(
        SceneError::WrongKind("Sphere").to_string(),
        "Sphere settings can only be given to a sphere"
    );
}

#[test]
fn one_drag_of_a_value_is_one_undo_step() {
    let (mut doc, cube) = with_a_cube();
    let steps = doc.undo_steps();
    for size in [210.0, 220.0, 230.0, 240.0] {
        doc.execute_merging(
            Command::SetPrimitive {
                ids: vec![cube],
                primitive: cube_of(size),
            },
            7,
        )
        .unwrap();
    }
    assert_eq!(doc.undo_steps(), steps + 1, "four changes, one step");
    // A new drag (a new key) is a new step.
    doc.execute_merging(
        Command::SetPrimitive {
            ids: vec![cube],
            primitive: cube_of(300.0),
        },
        8,
    )
    .unwrap();
    assert_eq!(doc.undo_steps(), steps + 2);
    doc.undo();
    assert_eq!(cube_size(&doc, cube), glam::Vec3::splat(240.0));
    doc.undo();
    assert_eq!(
        cube_size(&doc, cube),
        glam::Vec3::splat(100.0),
        "back to the start"
    );
    // Redo replays the whole drag at once.
    doc.redo();
    assert_eq!(cube_size(&doc, cube), glam::Vec3::splat(240.0));
}

#[test]
fn something_in_between_starts_a_new_step() {
    let (mut doc, cube) = with_a_cube();
    doc.execute_merging(
        Command::SetPrimitive {
            ids: vec![cube],
            primitive: cube_of(10.0),
        },
        1,
    )
    .unwrap();
    doc.execute(Command::Rename {
        id: cube,
        name: "Box".into(),
    })
    .unwrap();
    doc.execute_merging(
        Command::SetPrimitive {
            ids: vec![cube],
            primitive: cube_of(20.0),
        },
        1,
    )
    .unwrap();
    doc.undo();
    assert_eq!(cube_size(&doc, cube), glam::Vec3::splat(10.0));
    // An undone step never merges again.
    doc.undo();
    doc.execute_merging(
        Command::SetPrimitive {
            ids: vec![cube],
            primitive: cube_of(30.0),
        },
        1,
    )
    .unwrap();
    doc.undo();
    assert_eq!(cube_size(&doc, cube), glam::Vec3::splat(10.0));
}

#[test]
fn dragging_back_to_the_start_leaves_nothing_to_undo() {
    let (mut doc, cube) = with_a_cube();
    let steps = doc.undo_steps();
    doc.execute_merging(
        Command::SetPrimitive {
            ids: vec![cube],
            primitive: cube_of(250.0),
        },
        3,
    )
    .unwrap();
    doc.execute_merging(
        Command::SetPrimitive {
            ids: vec![cube],
            primitive: cube_of(100.0),
        },
        3,
    )
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn pasting_settings_to_several_primitives_is_one_step() {
    let (mut doc, a) = with_a_cube();
    doc.execute(Command::Insert {
        object: cube(),
        place: Place {
            parent: None,
            index: 0,
        },
    })
    .unwrap();
    let b = doc.selection()[0];
    let steps = doc.undo_steps();
    doc.execute(Command::SetPrimitive {
        ids: vec![a, b],
        primitive: cube_of(80.0),
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(cube_size(&doc, a), cube_size(&doc, b));
    doc.undo();
    assert_eq!(cube_size(&doc, a), glam::Vec3::splat(100.0));
    assert_eq!(cube_size(&doc, b), glam::Vec3::splat(100.0));
}
