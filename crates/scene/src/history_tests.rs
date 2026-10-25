//! Tests for the undo history's memory budget.

use crate::*;
use std::sync::Arc;

fn create_group(doc: &mut Document) {
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
}

/// A plain-mesh object as large as a standard sphere.
fn big_mesh() -> ObjectKind {
    ObjectKind::Mesh(Arc::new(PrimitiveKind::Sphere.standard().mesh()))
}

/// Checks the running total matches the steps kept.
fn check_total(doc: &Document) {
    assert_eq!(doc.history_memory(), doc.history().measured());
}

#[test]
fn the_standard_budget_is_two_gigabytes() {
    assert_eq!(
        HistoryLimit::STANDARD,
        HistoryLimit::Bytes(2 * 1024 * 1024 * 1024)
    );
    let doc = Document::new();
    assert_eq!(doc.history_limit(), HistoryLimit::STANDARD);
    assert_eq!(doc.history_memory(), 0);
    assert_eq!(doc.forgotten_steps(), 0);
}

#[test]
fn memory_grows_with_steps_and_moves_between_undo_and_redo() {
    let mut doc = Document::new();
    create_group(&mut doc);
    let one = doc.history_memory();
    assert!(one > 0);
    create_group(&mut doc);
    assert!(doc.history_memory() > one);
    let two = doc.history_memory();
    doc.undo().unwrap();
    assert_eq!(
        doc.history_memory(),
        two,
        "undone steps are still kept, for redo"
    );
    assert_eq!(doc.redo_steps(), 1);
    // A new change forgets what could be redone, and its memory.
    doc.execute(Command::Rename {
        id: doc.scene().children(None)[0],
        name: "X".into(),
    })
    .unwrap();
    assert_eq!(doc.redo_steps(), 0);
    check_total(&doc);
}

#[test]
fn over_budget_the_oldest_steps_are_forgotten() {
    let mut doc = Document::new();
    create_group(&mut doc);
    let per_step = doc.history_memory();
    doc.set_history_limit(HistoryLimit::Bytes(per_step * 5));
    for _ in 0..20 {
        create_group(&mut doc);
        assert!(doc.history_memory() <= per_step * 5 + per_step / 2);
        check_total(&doc);
    }
    assert!(
        doc.undo_steps() <= 5 && doc.undo_steps() >= 4,
        "{}",
        doc.undo_steps()
    );
    assert_eq!(doc.forgotten_steps(), 21 - doc.undo_steps());
    // Undoing everything kept leaves the groups whose steps were forgotten.
    let kept = doc.undo_steps();
    while doc.undo().is_some() {}
    assert_eq!(doc.scene().len(), 21 - kept);
    // And all of it can be redone.
    while doc.redo().is_some() {}
    assert_eq!(doc.scene().len(), 21);
}

#[test]
fn the_newest_step_is_kept_however_large() {
    let mut doc = Document::new();
    doc.set_history_limit(HistoryLimit::Bytes(1));
    create_group(&mut doc);
    doc.execute(Command::Create(big_mesh())).unwrap();
    assert_eq!(doc.undo_steps(), 1, "only the newest");
    assert!(doc.history_memory() > 1);
    assert_eq!(doc.undo_label(), Some("Create mesh"));
    doc.undo().unwrap();
    assert_eq!(doc.scene().len(), 1, "the mesh is gone, the group stays");
}

#[test]
fn unlimited_keeps_every_step() {
    let mut doc = Document::new();
    doc.set_history_limit(HistoryLimit::Unlimited);
    for _ in 0..200 {
        create_group(&mut doc);
    }
    assert_eq!(doc.undo_steps(), 200);
    assert_eq!(doc.forgotten_steps(), 0);
}

#[test]
fn a_lower_limit_forgets_at_once_and_a_higher_one_cannot_bring_steps_back() {
    let mut doc = Document::new();
    for _ in 0..10 {
        create_group(&mut doc);
    }
    let per_step = doc.history_memory() / 10;
    doc.set_history_limit(HistoryLimit::Bytes(per_step * 3));
    assert!(doc.undo_steps() <= 3);
    let forgotten = doc.forgotten_steps();
    assert_eq!(forgotten, 10 - doc.undo_steps());
    doc.set_history_limit(HistoryLimit::Unlimited);
    assert_eq!(doc.forgotten_steps(), forgotten);
    assert!(doc.undo_steps() <= 3);
    check_total(&doc);
}

#[test]
fn a_lower_limit_forgets_far_redo_steps_after_old_undo_steps() {
    let mut doc = Document::new();
    for _ in 0..4 {
        create_group(&mut doc);
    }
    for _ in 0..3 {
        doc.undo().unwrap();
    }
    assert_eq!((doc.undo_steps(), doc.redo_steps()), (1, 3));
    doc.set_history_limit(HistoryLimit::Bytes(1));
    assert_eq!(doc.undo_steps(), 1, "the newest undo step stays");
    assert_eq!(doc.redo_steps(), 0);
    assert_eq!(doc.forgotten_steps(), 3);
    check_total(&doc);
}

#[test]
fn a_long_drag_stays_one_small_step() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    let before = doc.history_memory();
    let mut cube = mesh::CubeParams::default();
    let mut sizes = Vec::new();
    for frame in 0..100 {
        cube.size.x = 101.0 + frame as f32;
        doc.execute_merging(
            Command::SetPrimitive {
                ids: vec![id],
                primitive: Primitive::Cube(cube),
            },
            7,
        )
        .unwrap();
        sizes.push(doc.history_memory());
        check_total(&doc);
    }
    assert_eq!(doc.undo_steps(), 2);
    assert!(
        sizes.windows(2).all(|w| w[0] == w[1]),
        "no growth per frame"
    );
    assert!(sizes[0] > before);
}

#[test]
fn random_work_keeps_the_running_total_exact() {
    let mut doc = Document::new();
    let mut seed: u64 = 99;
    let mut next = |below: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % below
    };
    doc.set_history_limit(HistoryLimit::Bytes(20_000));
    for _ in 0..400 {
        match next(6) {
            0 => {
                doc.undo();
            }
            1 => {
                doc.redo();
            }
            2 => doc.execute(Command::Create(big_mesh())).unwrap(),
            3 => {
                let ids: Vec<ObjectId> = doc.scene().children(None).to_vec();
                if let Some(id) = ids.first() {
                    doc.execute(Command::Delete(vec![*id])).unwrap();
                }
            }
            4 => doc.set_history_limit(HistoryLimit::Bytes(next(200_000) as usize + 1)),
            _ => create_group(&mut doc),
        }
        check_total(&doc);
        if let HistoryLimit::Bytes(limit) = doc.history_limit()
            && doc.history_memory() > limit
        {
            assert_eq!(
                doc.undo_steps() + doc.redo_steps(),
                1,
                "only the newest step"
            );
        }
    }
}

#[test]
fn a_step_of_several_commands_is_taken_back_whole_when_one_is_refused() {
    let mut doc = Document::new();
    create_group(&mut doc);
    let group = doc.selection()[0];
    let steps = doc.undo_steps();
    let before = doc.scene().clone();
    let refused = doc.execute_all_labelled(
        [
            Command::Rename {
                id: group,
                name: "Renamed".into(),
            },
            Command::Rename {
                id: group,
                name: "   ".into(),
            },
        ],
        "Rename twice",
        1 << 60,
    );
    assert!(refused.is_err());
    assert_eq!(doc.scene(), &before, "nothing is left half done");
    assert_eq!(doc.undo_steps(), steps);
    assert_eq!(doc.undo_label(), Some("Create group"));
    // The first one refused: the step before isn't taken back.
    let refused = doc.execute_all_labelled(
        [Command::Rename {
            id: group,
            name: String::new(),
        }],
        "Rename",
        (1 << 60) + 1,
    );
    assert!(refused.is_err());
    assert_eq!(doc.undo_steps(), steps);
    // All done: one step.
    doc.execute_all_labelled(
        [
            Command::Rename {
                id: group,
                name: "A".into(),
            },
            Command::Rename {
                id: group,
                name: "B".into(),
            },
        ],
        "Rename twice",
        (1 << 60) + 2,
    )
    .unwrap();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Rename twice"));
}
