//! Vertex colours ([decision 0205](../../../docs/decisions/0205-vertex-colour-painting.md)):
//! a colour kept on each polygon corner, so neighbouring polygons can
//! share a smooth colour (the same colour on all their corners at a
//! point) or meet with a crisp change (different colours). Kept as an
//! attribute layer, [`COLOUR`], of four numbers 0 to 1 (red, green, blue
//! as sRGB, and alpha); white where never painted.

use crate::attributes::{AttributeElement, Domain, Value};
use crate::edit::Outcome;
use crate::{FaceId, Mesh, VertexId};
use glam::Vec4;

/// The vertex colour layer's name.
pub const COLOUR: &str = "colour";

/// Whether `mesh` has vertex colours.
pub fn has_colours(mesh: &Mesh) -> bool {
    mesh.attributes()
        .any(|(name, domain, _)| name == COLOUR && domain == Domain::Corner)
}

/// The colour of each corner of `mesh`, in the order of its corner list,
/// as sRGB bytes; `None` when it has no vertex colours.
pub fn colours(mesh: &Mesh) -> Option<Vec<[u8; 4]>> {
    if !has_colours(mesh) {
        return None;
    }
    let (starts, _) = mesh.face_layout();
    let mut out = vec![[255u8; 4]; starts.last().copied().unwrap_or(0) as usize];
    for (f, pair) in starts.windows(2).enumerate() {
        let face = FaceId::from_raw(f as u32);
        if mesh.face(face).is_none() {
            continue;
        }
        for (i, k) in (pair[0] as usize..pair[1] as usize).enumerate() {
            if let Some(Value::Vec4(v)) = mesh.attribute(COLOUR, AttributeElement::Corner(face, i))
            {
                out[k] = bytes(v);
            }
        }
    }
    Some(out)
}

/// The colour of polygon `face`'s corner at point `point`, if it has one.
pub fn colour_at(mesh: &Mesh, face: FaceId, point: VertexId) -> Option<[u8; 4]> {
    let index = mesh.face(face)?.iter().position(|v| *v == point)?;
    match mesh.attribute(COLOUR, AttributeElement::Corner(face, index)) {
        Some(Value::Vec4(v)) => Some(bytes(v)),
        // Never painted: white.
        _ => Some([255; 4]),
    }
}

/// Sets the colours of corners, each named by its polygon and point:
/// those not in the mesh are left out (counted as missing).
pub fn set_colours(mesh: &mut Mesh, corners: &[(FaceId, VertexId, [u8; 4])]) -> Outcome {
    let mut outcome = Outcome::default();
    if corners.is_empty() {
        return outcome;
    }
    if !has_colours(mesh)
        && mesh
            .add_attribute(COLOUR, Domain::Corner, Value::Vec4(Vec4::ONE))
            .is_err()
    {
        outcome.skipped = corners.len();
        return outcome;
    }
    for (face, point, colour) in corners {
        let Some(index) = mesh
            .face(*face)
            .and_then(|l| l.iter().position(|v| v == point))
        else {
            outcome.missing += 1;
            continue;
        };
        let value = Value::Vec4(Vec4::from(colour.map(|c| f32::from(c) / 255.0)));
        if mesh
            .set_attribute(COLOUR, AttributeElement::Corner(*face, index), value)
            .is_ok()
        {
            outcome.matched += 1;
        } else {
            outcome.skipped += 1;
        }
    }
    outcome
}

/// A stored colour as bytes.
fn bytes(v: Vec4) -> [u8; 4] {
    v.to_array()
        .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrimitiveKind;

    #[test]
    fn corners_are_white_until_painted_and_keep_their_own_colours() {
        let mut cube = PrimitiveKind::Cube.standard().mesh();
        assert!(colours(&cube).is_none());
        let (face, list) = cube.faces().next().map(|(f, l)| (f, l.to_vec())).unwrap();
        let red = [255, 0, 0, 255];
        let outcome = set_colours(&mut cube, &[(face, list[0], red)]);
        assert_eq!((outcome.matched, outcome.missing), (1, 0));
        let all = colours(&cube).unwrap();
        assert_eq!(all[0], red);
        assert!(all[1..].iter().all(|c| *c == [255; 4]));
        assert_eq!(colour_at(&cube, face, list[0]), Some(red));
        // The same point on another polygon keeps its own colour.
        let other = cube
            .faces()
            .find(|(f, l)| *f != face && l.contains(&list[0]))
            .map(|(f, _)| f)
            .unwrap();
        assert_eq!(colour_at(&cube, other, list[0]), Some([255; 4]));
    }

    #[test]
    fn corners_not_in_the_mesh_are_counted_missing() {
        let mut cube = PrimitiveKind::Cube.standard().mesh();
        let (face, _) = cube.faces().next().map(|(f, l)| (f, l.to_vec())).unwrap();
        let outcome = set_colours(
            &mut cube,
            &[
                (FaceId::from_raw(999), VertexId::from_raw(0), [0; 4]),
                (face, VertexId::from_raw(999), [0; 4]),
            ],
        );
        assert_eq!((outcome.matched, outcome.missing), (0, 2));
        assert!(set_colours(&mut cube, &[]).matched == 0);
    }

    #[test]
    fn n_gons_and_open_shapes_take_colours_on_every_corner() {
        // A cylinder's caps are many-sided polygons; a plane is open.
        for kind in [PrimitiveKind::Cylinder, PrimitiveKind::Plane] {
            let mut mesh = kind.standard().mesh();
            let corners: Vec<(FaceId, VertexId, [u8; 4])> = mesh
                .faces()
                .flat_map(|(f, l)| {
                    l.iter()
                        .map(move |v| (f, *v, [10, 20, 30, 40]))
                        .collect::<Vec<_>>()
                })
                .collect();
            let outcome = set_colours(&mut mesh, &corners);
            assert_eq!(outcome.matched, corners.len(), "{kind:?}");
            assert!(
                colours(&mesh)
                    .unwrap()
                    .iter()
                    .all(|c| *c == [10, 20, 30, 40])
            );
        }
    }

    #[test]
    fn colours_out_of_range_are_kept_in_range_when_read() {
        assert_eq!(bytes(Vec4::new(-1.0, 2.0, 0.5, 1.0)), [0, 255, 128, 255]);
    }
}
