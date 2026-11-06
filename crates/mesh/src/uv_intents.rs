//! UV intents ([decision 0248](../../../docs/decisions/0248-uv-intents.md)):
//! what a piece was told to be (a straight strip, on a trim row, at a
//! texel density, this way up), kept on its polygons so it is made so
//! again after modelling changes the piece.
//!
//! Stored as three polygon layers: [`UV_INTENT`] (which), and
//! [`UV_INTENT_A`] and [`UV_INTENT_B`] (its values).

use crate::attributes::{AttributeElement, AttributeValues, UV_INTENT, UV_INTENT_A, UV_INTENT_B};
use crate::{Domain, FaceId, Mesh, Value};

/// What a piece was told to be.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum UvIntent {
    /// Nothing.
    #[default]
    None,
    /// Its quads in straight rows and columns (Straighten).
    Strip,
    /// At this texel density, in texture widths per unit of length (Set
    /// texel density).
    Density(f32),
    /// On the trim row from `bottom` to `top`, shares of the square's
    /// height (Put on trim row).
    TrimRow {
        /// The row's bottom.
        bottom: f32,
        /// The row's top.
        top: f32,
    },
    /// Turned so the world's up runs up the texture (Keep this way up).
    Upright,
}

impl UvIntent {
    /// Its number in the layer.
    fn code(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Strip => 1,
            Self::Density(_) => 2,
            Self::TrimRow { .. } => 3,
            Self::Upright => 4,
        }
    }

    /// Its values.
    fn values(self) -> (f32, f32) {
        match self {
            Self::Density(d) => (d, 0.0),
            Self::TrimRow { bottom, top } => (bottom, top),
            _ => (0.0, 0.0),
        }
    }

    /// The intent with this number and values, if they make one.
    fn from(code: i32, a: f32, b: f32) -> Self {
        let made = match code {
            1 => Self::Strip,
            2 => Self::Density(a),
            3 => Self::TrimRow { bottom: a, top: b },
            4 => Self::Upright,
            _ => Self::None,
        };
        if made.is_real() { made } else { Self::None }
    }

    /// Whether its values make sense: a density above 0, a row inside the
    /// square with some height.
    pub fn is_real(self) -> bool {
        match self {
            Self::Density(d) => d.is_finite() && d > 0.0,
            Self::TrimRow { bottom, top } => {
                bottom.is_finite() && top.is_finite() && bottom >= 0.0 && top > bottom && top <= 1.0
            }
            _ => true,
        }
    }
}

/// The intent of polygon `face`.
pub fn intent_of(mesh: &Mesh, face: FaceId) -> UvIntent {
    let element = AttributeElement::Face(face);
    let code = match mesh.attribute(UV_INTENT, element) {
        Some(Value::Int(i)) => i,
        _ => return UvIntent::None,
    };
    let float = |name| match mesh.attribute(name, element) {
        Some(Value::Float(x)) => x,
        _ => 0.0,
    };
    UvIntent::from(code, float(UV_INTENT_A), float(UV_INTENT_B))
}

/// The intent of the piece made of `faces`: its first polygon's (by id)
/// that has one.
pub fn piece_intent(mesh: &Mesh, faces: &[FaceId]) -> UvIntent {
    let mut sorted = faces.to_vec();
    sorted.sort();
    sorted
        .into_iter()
        .map(|f| intent_of(mesh, f))
        .find(|i| *i != UvIntent::None)
        .unwrap_or_default()
}

/// Gives `faces` the intent `intent` (`None` clears it). The layers are
/// added when first needed and removed when no polygon has an intent. An
/// intent that isn't real changes nothing.
pub fn set_intent(mesh: &mut Mesh, faces: &[FaceId], intent: UvIntent) -> crate::edit::Outcome {
    let present: Vec<FaceId> = faces.iter().copied().filter(|f| mesh.face(*f).is_some()).collect();
    let mut outcome = crate::edit::Outcome {
        matched: 0,
        missing: faces.len() - present.len(),
        skipped: present.len(),
    };
    if !intent.is_real() || (intent == UvIntent::None && mesh.attribute_values(UV_INTENT).is_none()) {
        return outcome;
    }
    for (name, default) in [
        (UV_INTENT, Value::Int(0)),
        (UV_INTENT_A, Value::Float(0.0)),
        (UV_INTENT_B, Value::Float(0.0)),
    ] {
        if mesh.attribute_values(name).is_none() && mesh.add_attribute(name, Domain::Face, default).is_err() {
            return outcome;
        }
    }
    let (a, b) = intent.values();
    let mut changed = 0;
    for face in &present {
        if intent_of(mesh, *face) == intent {
            continue;
        }
        let element = AttributeElement::Face(*face);
        let ok = mesh.set_attribute(UV_INTENT, element, Value::Int(intent.code())).is_ok()
            && mesh.set_attribute(UV_INTENT_A, element, Value::Float(a)).is_ok()
            && mesh.set_attribute(UV_INTENT_B, element, Value::Float(b)).is_ok();
        if ok {
            changed += 1;
        }
    }
    let used = matches!(
        mesh.attribute_values(UV_INTENT),
        Some((_, _, AttributeValues::Int(list))) if list.iter().any(|v| *v != 0)
    );
    if !used {
        for name in [UV_INTENT, UV_INTENT_A, UV_INTENT_B] {
            mesh.remove_attribute(name);
        }
    }
    outcome.matched = changed;
    outcome.skipped = present.len() - changed;
    outcome
}

/// Whether any polygon of `mesh` has an intent.
pub fn has_intents(mesh: &Mesh) -> bool {
    mesh.attribute_values(UV_INTENT).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CubeParams, Primitive};

    fn cube() -> Mesh {
        Primitive::Cube(CubeParams::default()).mesh()
    }

    #[test]
    fn intents_are_set_read_and_cleared() {
        let mut mesh = cube();
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).take(2).collect();
        let row = UvIntent::TrimRow {
            bottom: 0.25,
            top: 0.5,
        };
        assert_eq!(set_intent(&mut mesh, &faces, row).matched, 2);
        assert_eq!(intent_of(&mesh, faces[0]), row);
        assert_eq!(piece_intent(&mesh, &faces), row);
        // The same again changes nothing.
        assert_eq!(set_intent(&mut mesh, &faces, row).matched, 0);
        set_intent(&mut mesh, &faces, UvIntent::None);
        assert!(!has_intents(&mesh), "the layers go with the last intent");
    }

    #[test]
    fn intents_that_arent_real_change_nothing() {
        let mut mesh = cube();
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        for bad in [
            UvIntent::Density(0.0),
            UvIntent::Density(f32::NAN),
            UvIntent::TrimRow { bottom: 0.5, top: 0.5 },
            UvIntent::TrimRow { bottom: -0.1, top: 0.5 },
            UvIntent::TrimRow { bottom: 0.5, top: 1.5 },
        ] {
            assert_eq!(set_intent(&mut mesh, &faces, bad).matched, 0);
        }
        assert!(!has_intents(&mesh));
        // Clearing what isn't there adds nothing.
        set_intent(&mut mesh, &faces, UvIntent::None);
        assert!(!has_intents(&mesh));
    }

    #[test]
    fn missing_polygons_are_counted() {
        let mut mesh = cube();
        let out = set_intent(&mut mesh, &[FaceId::from_raw(999)], UvIntent::Strip);
        assert_eq!(out.missing, 1);
        assert_eq!(intent_of(&mesh, FaceId::from_raw(999)), UvIntent::None);
    }
}
