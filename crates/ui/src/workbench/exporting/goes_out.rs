//! What the Export window says about the objects that go out beyond the
//! numbers: whether any has levels of detail (the Levels of detail chip
//! starts on when one does), and their collision, by kind.

use crate::text::tr;
use fileio::export::ExportWhat;
use scene::{Collision, Document, ObjectId};

/// The objects an export of `what` takes, by the same rule as
/// `fileio::export::summary`: shown, with a shape, not a boolean's cutter,
/// and (for the selection) selected or inside a selected object.
fn exported(document: &Document, what: ExportWhat) -> Vec<ObjectId> {
    let scene = document.scene();
    let selection = document.selection();
    let cutters = scene.cutters();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| scene.is_shown(*id) && !cutters.contains(id))
        .filter(|id| scene.object(*id).is_some_and(|o| o.kind.has_shape()))
        .filter(|id| {
            what == ExportWhat::Everything
                || selection
                    .iter()
                    .any(|s| s == id || scene.is_inside(*id, *s))
        })
        .collect()
}

/// Whether any object an export of `what` takes has levels of detail made
/// (decision 0155).
pub(super) fn any_levels(document: &Document, what: ExportWhat) -> bool {
    let scene = document.scene();
    exported(document, what)
        .iter()
        .any(|id| scene.object(*id).is_some_and(|o| o.lods.on))
}

/// The line under the numbers about collision: how many objects take it
/// out and of which kinds, when the file carries it (`godot`: Made for
/// Godot, whose names tell Godot to make the shapes, decision 0078);
/// otherwise that it doesn't go out.
pub(super) fn collision_line(document: &Document, what: ExportWhat, godot: bool) -> String {
    let scene = document.scene();
    let mut by_kind: Vec<(Collision, usize)> = Vec::new();
    for id in exported(document, what) {
        let Some(kind) = scene.object(id).map(|o| o.collision) else {
            continue;
        };
        if kind == Collision::None {
            continue;
        }
        match by_kind.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, n)) => *n += 1,
            None => by_kind.push((kind, 1)),
        }
    }
    let objects: usize = by_kind.iter().map(|(_, n)| n).sum();
    if objects == 0 {
        return tr("No collision goes out: give an object some in Attributes → Collision.")
            .to_string();
    }
    let counted = match objects {
        1 => tr("1 object").to_string(),
        n => crate::trf!("{} objects", n),
    };
    if !godot {
        return crate::trf!(
            "Collision: {} have some, but it goes out only with Made for Godot, whose names \
             tell Godot to make the shapes.",
            counted
        );
    }
    by_kind.sort_by_key(|(kind, _)| Collision::ALL.iter().position(|k| k == kind));
    let kinds: Vec<String> = by_kind
        .iter()
        .map(|(kind, n)| crate::trf!("{} × {}", kind.name(), n))
        .collect();
    crate::trf!(
        "Collision goes out for {}: {}. Godot makes the shapes when it imports the file.",
        counted,
        kinds.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, ObjectKind, PrimitiveKind};

    fn shapes(n: usize) -> (Document, Vec<ObjectId>) {
        let mut doc = Document::new();
        let mut ids = Vec::new();
        for _ in 0..n {
            doc.execute(Command::Create(ObjectKind::Primitive(
                PrimitiveKind::Cube.standard(),
            )))
            .unwrap();
            ids.push(doc.selection()[0]);
        }
        (doc, ids)
    }

    fn collide(doc: &mut Document, id: ObjectId, collision: Collision) {
        doc.execute(Command::SetCollision {
            ids: vec![id],
            collision,
        })
        .unwrap();
    }

    #[test]
    fn collision_is_counted_by_kind() {
        let (mut doc, ids) = shapes(4);
        assert!(collision_line(&doc, ExportWhat::Everything, true).starts_with("No collision"));
        collide(&mut doc, ids[0], Collision::Box);
        collide(&mut doc, ids[1], Collision::Box);
        collide(&mut doc, ids[2], Collision::Parts);
        let line = collision_line(&doc, ExportWhat::Everything, true);
        assert_eq!(
            line,
            "Collision goes out for 3 objects: Wrapped in parts (convex) × 1, A box × 2. Godot \
             makes the shapes when it imports the file."
        );
        // Only the selection.
        doc.set_selection([ids[2]]);
        let line = collision_line(&doc, ExportWhat::Selected, true);
        assert!(
            line.starts_with("Collision goes out for 1 object:"),
            "{line}"
        );
        // Other files don't carry it.
        let line = collision_line(&doc, ExportWhat::Everything, false);
        assert!(line.contains("only with Made for Godot"), "{line}");
    }

    #[test]
    fn levels_are_found_on_what_goes_out() {
        let (mut doc, ids) = shapes(2);
        assert!(!any_levels(&doc, ExportWhat::Everything));
        doc.execute(Command::SetLods {
            ids: vec![ids[1]],
            lods: scene::Lods {
                on: true,
                ..scene::Lods::default()
            },
        })
        .unwrap();
        assert!(any_levels(&doc, ExportWhat::Everything));
        doc.set_selection([ids[0]]);
        assert!(!any_levels(&doc, ExportWhat::Selected));
    }
}
