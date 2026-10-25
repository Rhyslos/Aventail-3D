//! Telling a modifier that found nothing to do from one that failed. Both
//! pass their input on unchanged and say why ([`Scene::modifier_problems`]),
//! but "no two points are close enough to weld" is a calm fact about the
//! shape, not a problem to fix, so the Attributes panel shows it in grey
//! rather than in the warning colour.
//!
//! [`Scene::modifier_problems`]: crate::Scene::modifier_problems

use mesh::quadify::QuadifyProblem;
use mesh::triangulate::TriangulateProblem;
use mesh::weld_distance::WeldProblem;

/// Whether `why`, a reason a modifier gave for leaving its input as it
/// was, means only that there was nothing for it to do: Weld by distance
/// found no points close enough (or no open borders), Quadify no
/// triangles it may join, Triangulate no polygon to cut.
pub fn has_nothing_to_do(why: &str) -> bool {
    let calm = [
        WeldProblem::NothingClose.to_string(),
        WeldProblem::NoBorders.to_string(),
        QuadifyProblem::NoTriangles.to_string(),
        QuadifyProblem::NothingJoins.to_string(),
        TriangulateProblem::AllTriangles.to_string(),
        TriangulateProblem::NoQuads.to_string(),
        TriangulateProblem::Empty.to_string(),
    ];
    calm.iter().any(|c| c == why)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn a_weld_with_nothing_close_has_nothing_to_do() {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        for kind in [
            ModifierKind::WeldByDistance(Default::default()),
            ModifierKind::Quadify(Default::default()),
        ] {
            doc.execute(Command::AddModifier {
                ids: vec![id],
                kind,
            })
            .unwrap();
        }
        let problems = doc.scene().modifier_problems(id);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().all(|(_, why)| has_nothing_to_do(why)));
    }

    #[test]
    fn real_problems_are_not_calm() {
        assert!(!has_nothing_to_do("the shape is open"));
        assert!(!has_nothing_to_do(""));
        assert!(has_nothing_to_do(
            &TriangulateProblem::AllTriangles.to_string()
        ));
    }
}
