//! Locked pieces and pieces' own settings (decision 0112): no automatic
//! tool changes a locked piece, and a piece's own settings win over the
//! app's on every unwrap.

use crate::unwrap::{PieceSettings, UnwrapOptions, unwrap_with};
use crate::{UnwrapMethod, relax, straighten};
use glam::Vec2;
use mesh::uvs::{Uvs, set_locked, set_piece_values};
use mesh::{CubeParams, CylinderParams, FaceId, Mesh, Primitive};

fn corner_uvs(mesh: &Mesh) -> Vec<Vec2> {
    let count = mesh.face_layout().1.len();
    Uvs::of(mesh).unwrap().into_values(count)
}

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// A cube whose UVs are turned a little, so every tool would change them.
fn turned_cube() -> Mesh {
    let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
    let turned: Vec<Vec2> = corner_uvs(&mesh)
        .iter()
        .map(|v| glam::Mat2::from_angle(0.3) * *v)
        .collect();
    mesh::uvs::set_uvs(&mut mesh, turned);
    mesh
}

/// A tool run on a shape's polygons.
type Tool = fn(&mut Mesh, &[FaceId]);

#[test]
fn no_tool_changes_a_locked_piece() {
    let tools: [(&str, Tool); 5] = [
        ("unwrap", |m, f| {
            unwrap_with(m, f, UnwrapOptions::plain(true));
        }),
        ("relax", |m, f| {
            relax(m, f, 60);
        }),
        ("straighten", |m, f| {
            straighten(m, f);
        }),
        ("density", |m, f| {
            crate::density::set_density(m, f, 0.5);
        }),
        ("trim", |m, f| {
            crate::trim::place(m, f, 0.25, 0.5);
        }),
    ];
    for (name, tool) in tools {
        let mut mesh = turned_cube();
        let faces = all_faces(&mesh);
        set_locked(&mut mesh, &faces, true);
        let before = corner_uvs(&mesh);
        tool(&mut mesh, &faces);
        assert_eq!(corner_uvs(&mesh), before, "{name}");
        // Unlocked, the same tool changes them.
        set_locked(&mut mesh, &faces, false);
        tool(&mut mesh, &faces);
        assert_ne!(corner_uvs(&mesh), before, "{name} unlocked");
    }
}

#[test]
fn only_the_locked_piece_is_kept() {
    let mut mesh = turned_cube();
    let faces = all_faces(&mesh);
    // A cube's sides are pieces of their own: lock one.
    set_locked(&mut mesh, &faces[..1], true);
    let uvs = Uvs::of(&mesh).unwrap();
    let side = |mesh: &Mesh| {
        let u = Uvs::of(mesh).unwrap();
        (0..4)
            .map(|k| u.corner(mesh, faces[0], k).unwrap())
            .collect::<Vec<_>>()
    };
    let kept = side(&mesh);
    let other = (0..4)
        .map(|k| uvs.corner(&mesh, faces[1], k).unwrap())
        .collect::<Vec<_>>();
    unwrap_with(&mut mesh, &faces, UnwrapOptions::plain(true));
    assert_eq!(side(&mesh), kept);
    let u = Uvs::of(&mesh).unwrap();
    let now: Vec<Vec2> = (0..4)
        .map(|k| u.corner(&mesh, faces[1], k).unwrap())
        .collect();
    assert_ne!(now, other);
}

#[test]
fn a_pieces_own_settings_win_over_the_apps() {
    // A cylinder's side is a strip: straightened by the app's setting,
    // but not when the piece says no.
    let cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    let faces = all_faces(&cylinder);
    let side: Vec<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| cylinder.face(*f).unwrap().len() == 4)
        .collect();
    let mut plain = cylinder.clone();
    unwrap_with(&mut plain, &faces, UnwrapOptions::plain(true));
    let mut own = cylinder.clone();
    let settings = PieceSettings {
        strips: Some(false),
        ..PieceSettings::default()
    };
    set_piece_values(&mut own, &side, settings.values());
    assert_eq!(PieceSettings::of(&own, &side), settings);
    unwrap_with(&mut own, &faces, UnwrapOptions::plain(true));
    assert_ne!(corner_uvs(&own), corner_uvs(&plain));
    // Its own density sizes it.
    let mut dense = cylinder.clone();
    let settings = PieceSettings {
        density: Some(0.05),
        ..PieceSettings::default()
    };
    set_piece_values(&mut dense, &side, settings.values());
    unwrap_with(&mut dense, &faces, UnwrapOptions::plain(true));
    let got = crate::density::density(&dense, &side).unwrap();
    assert!((got - 0.05).abs() < 0.05 * 0.02, "{got}");
    // The method is carried over the app's options.
    let options = PieceSettings {
        method: Some(UnwrapMethod::SizeKeeping),
        ..PieceSettings::default()
    }
    .over(UnwrapOptions::plain(true));
    assert_eq!(options.method, UnwrapMethod::SizeKeeping);
    assert!(options.strips);
    // Stored values round trip; nonsense isn't real.
    for s in [
        PieceSettings::default(),
        PieceSettings {
            method: Some(UnwrapMethod::Automatic),
            strips: Some(true),
            density: Some(1.0),
        },
    ] {
        assert_eq!(PieceSettings::from_values(s.values()), s);
    }
    assert!(
        !PieceSettings {
            density: Some(f32::NAN),
            ..PieceSettings::default()
        }
        .is_real()
    );
}
