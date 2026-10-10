//! Tests for the primitives: each builds a closed (or, for flat ones,
//! properly bounded) surface, faces outward, shades as intended, fits its
//! bounds, and survives silly settings.

use super::*;
use crate::polygon::EdgeKey;
use crate::shading::shaded_triangles;
use std::collections::BTreeMap;
use std::f32::consts::PI;

/// How many faces use each edge.
fn edge_use(mesh: &Mesh) -> BTreeMap<EdgeKey, usize> {
    let mut uses = BTreeMap::new();
    for (_, corners) in mesh.faces() {
        for i in 0..corners.len() {
            let key = EdgeKey::new(corners[i], corners[(i + 1) % corners.len()]);
            *uses.entry(key).or_insert(0) += 1;
        }
    }
    uses
}

/// Every edge is shared by exactly two faces: a closed, solid surface.
fn is_closed(mesh: &Mesh) -> bool {
    edge_use(mesh).values().all(|n| *n == 2)
}

/// The volume inside a closed mesh (positive when faces look outward).
fn volume(mesh: &Mesh) -> f32 {
    shaded_triangles(mesh)
        .iter()
        .map(|t| t.positions[0].dot(t.positions[1].cross(t.positions[2])) / 6.0)
        .sum()
}

fn closed_shapes() -> Vec<Primitive> {
    let mut shapes = vec![
        Primitive::Cube(CubeParams::default()),
        Primitive::Cube(CubeParams {
            size: Vec3::new(100.0, 300.0, 50.0),
            segments: [3, 1, 4],
        }),
        Primitive::Cylinder(CylinderParams::default()),
        Primitive::Cylinder(CylinderParams {
            height_segments: 3,
            sides: 5,
            ..CylinderParams::default()
        }),
        Primitive::Sphere(SphereParams::default()),
        Primitive::Sphere(SphereParams {
            sides: 3,
            rings: 2,
            radius: 10.0,
        }),
        Primitive::Tube(TubeParams::default()),
        Primitive::Tube(TubeParams {
            height_segments: 2,
            ..TubeParams::default()
        }),
        Primitive::Cone(ConeParams::default()),
        Primitive::Cone(ConeParams {
            top_radius: 40.0,
            height_segments: 2,
            ..ConeParams::default()
        }),
        Primitive::Cone(ConeParams {
            bottom_radius: 0.0,
            top_radius: 60.0,
            ..ConeParams::default()
        }),
    ];
    shapes.shrink_to_fit();
    shapes
}

#[test]
fn solid_primitives_are_closed_and_face_outward() {
    for shape in closed_shapes() {
        let mesh = shape.mesh();
        assert!(is_closed(&mesh), "{shape:?} has holes or overlapping faces");
        assert!(volume(&mesh) > 0.0, "{shape:?} faces inward");
    }
}

#[test]
fn volumes_match_the_shapes_they_approximate() {
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    assert!((volume(&cube) / 200.0f32.powi(3) - 1.0).abs() < 1e-4);
    let segmented = Primitive::Cube(CubeParams {
        segments: [4, 3, 2],
        ..CubeParams::default()
    })
    .mesh();
    assert!((volume(&segmented) / volume(&cube) - 1.0).abs() < 1e-4);
    // A 64-sided cylinder is close to a true one.
    let cylinder = Primitive::Cylinder(CylinderParams {
        sides: 64,
        ..CylinderParams::default()
    })
    .mesh();
    let exact = PI * 50.0 * 50.0 * 200.0;
    assert!((volume(&cylinder) / exact - 1.0).abs() < 0.01);
    let tube = Primitive::Tube(TubeParams {
        sides: 64,
        ..TubeParams::default()
    })
    .mesh();
    let exact = PI * (100.0f32.powi(2) - 70.0f32.powi(2)) * 100.0;
    assert!((volume(&tube) / exact - 1.0).abs() < 0.01);
}

#[test]
fn every_primitive_fits_its_bounds_and_stands_on_the_ground() {
    for shape in Primitive::all().into_iter().chain(closed_shapes()) {
        let mesh = shape.mesh();
        let bounds = shape.bounds();
        let actual = mesh.bounds().expect("not empty");
        assert!(actual.min.z.abs() < 1e-3, "{shape:?} stands on the ground");
        for (_, p) in mesh.vertices() {
            assert!(
                bounds.contains(p) || (p - p.clamp(bounds.min, bounds.max)).length() < 1e-3,
                "{shape:?}: {p} outside {bounds:?}"
            );
        }
        // Centred on the pivot across the ground (round shapes with an odd
        // number of sides lean a little towards their first point).
        let size = actual.size();
        assert!(actual.centre().x.abs() <= size.x * 0.25 + 1e-3, "{shape:?}");
        assert!(actual.centre().y.abs() <= size.y * 0.25 + 1e-3, "{shape:?}");
    }
}

#[test]
fn flat_primitives_lie_on_the_ground_facing_up() {
    for shape in [
        Primitive::Plane(PlaneParams::default()),
        Primitive::Plane(PlaneParams {
            segments: [3, 5],
            ..PlaneParams::default()
        }),
        Primitive::Disc(DiscParams::default()),
        Primitive::Disc(DiscParams {
            inner_radius: 40.0,
            ..DiscParams::default()
        }),
    ] {
        let mesh = shape.mesh();
        for (face, _) in mesh.faces() {
            assert!(
                (mesh.face_normal(face) - Vec3::Z).length() < 1e-5,
                "{shape:?}"
            );
        }
        for (_, p) in mesh.vertices() {
            assert_eq!(p.z, 0.0);
        }
        // Open at the edges only: no edge is used more than twice.
        assert!(edge_use(&mesh).values().all(|n| *n <= 2));
    }
}

#[test]
fn face_and_point_counts_follow_the_segments() {
    let count = |p: Primitive| {
        let m = p.mesh();
        (m.vertex_count(), m.face_count())
    };
    assert_eq!(count(Primitive::Cube(CubeParams::default())), (8, 6));
    assert_eq!(
        count(Primitive::Cube(CubeParams {
            segments: [2, 2, 2],
            ..CubeParams::default()
        })),
        (26, 24)
    );
    assert_eq!(count(Primitive::Plane(PlaneParams::default())), (4, 1));
    // 16 sides, one band, two n-gon caps.
    assert_eq!(
        count(Primitive::Cylinder(CylinderParams::default())),
        (32, 18)
    );
    // 24 around, 12 rings: two poles and 11 rings of 24 points.
    assert_eq!(
        count(Primitive::Sphere(SphereParams::default())),
        (2 + 11 * 24, 24 * 12)
    );
    assert_eq!(count(Primitive::Disc(DiscParams::default())), (24, 1));
    // A pointed cone: a ring, the tip, 16 triangles and the base.
    assert_eq!(count(Primitive::Cone(ConeParams::default())), (17, 17));
    assert_eq!(count(Primitive::Tube(TubeParams::default())), (96, 96));
}

#[test]
fn caps_are_single_faces_with_hard_rims() {
    let mesh = Primitive::Cylinder(CylinderParams::default()).mesh();
    let caps = mesh.faces().filter(|(_, c)| c.len() == 16).count();
    assert_eq!(caps, 2);
    assert_eq!(mesh.hard_edge_count(), 32, "the two rims");
    let open = Primitive::Cylinder(CylinderParams {
        caps: false,
        ..CylinderParams::default()
    })
    .mesh();
    assert_eq!(open.face_count(), 16);
    assert_eq!(open.hard_edge_count(), 0);
}

#[test]
fn a_cube_has_exactly_its_twelve_edges_hard_however_it_is_cut() {
    for segments in [[1, 1, 1], [3, 2, 4]] {
        let mesh = Primitive::Cube(CubeParams {
            segments,
            ..CubeParams::default()
        })
        .mesh();
        let along_edges = 4 * (segments[0] + segments[1] + segments[2]) as usize;
        assert_eq!(mesh.hard_edge_count(), along_edges, "{segments:?}");
        assert!(is_closed(&mesh));
    }
}

#[test]
fn round_sides_shade_smoothly_and_flat_sides_crisply() {
    // On a cylinder's side, corner normals point straight out from the
    // axis (smooth); on a cube, every corner shares its face's normal.
    let cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    for t in shaded_triangles(&cylinder) {
        let face = cylinder.face_normal(t.face);
        if face.z.abs() > 0.5 {
            continue; // a cap: flat
        }
        for (p, n) in t.positions.iter().zip(t.normals) {
            let radial = Vec3::new(p.x, p.y, 0.0).normalize();
            assert!((n - radial).length() < 1e-4, "smooth side");
        }
    }
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    for t in shaded_triangles(&cube) {
        for n in t.normals {
            assert!((n - cube.face_normal(t.face)).length() < 1e-5, "crisp box");
        }
    }
    let sphere = Primitive::Sphere(SphereParams::default()).mesh();
    let centre = Vec3::new(0.0, 0.0, 100.0);
    for t in shaded_triangles(&sphere) {
        for (p, n) in t.positions.iter().zip(t.normals) {
            let error = (n - (*p - centre).normalize()).length();
            assert!(error < 0.05, "ball: {p} off by {error}");
        }
    }
}

#[test]
fn silly_settings_are_brought_within_limits() {
    let silly = [
        Primitive::Cube(CubeParams {
            size: Vec3::new(f32::NAN, -5.0, 1e12),
            segments: [0, 100_000, 3],
        }),
        Primitive::Cylinder(CylinderParams {
            radius: 0.0,
            height: f32::INFINITY,
            sides: 0,
            height_segments: 0,
            caps: true,
        }),
        Primitive::Sphere(SphereParams {
            radius: -1.0,
            sides: 1,
            rings: 0,
        }),
        Primitive::Disc(DiscParams {
            radius: 10.0,
            inner_radius: 50.0,
            sides: 2,
        }),
        Primitive::Tube(TubeParams {
            outer_radius: 10.0,
            inner_radius: 20.0,
            height: 0.0,
            sides: 9999,
            height_segments: 0,
        }),
        Primitive::Cone(ConeParams {
            bottom_radius: 0.0,
            top_radius: 0.0,
            height: 1.0,
            sides: 3,
            height_segments: 1,
            caps: true,
        }),
        Primitive::Plane(PlaneParams {
            width: f32::NAN,
            depth: 0.0,
            segments: [0, 0],
        }),
    ];
    for shape in silly {
        let clamped = shape.clamped();
        assert_eq!(clamped.clamped(), clamped, "clamping twice changes nothing");
        let mesh = shape.mesh();
        assert!(mesh.face_count() > 0, "{shape:?} still builds");
        let b = mesh.bounds().unwrap();
        assert!(b.min.is_finite() && b.max.is_finite());
        assert!(b.size().max_element() <= MAX_SIZE * 2.0);
        if !matches!(shape, Primitive::Plane(_) | Primitive::Disc(_)) {
            assert!(is_closed(&mesh), "{shape:?}");
            assert!(volume(&mesh) > 0.0, "{shape:?}");
        }
    }
}

#[test]
fn the_same_settings_build_the_same_mesh() {
    for shape in Primitive::all() {
        assert_eq!(shape.mesh(), shape.mesh());
    }
}

#[test]
fn the_largest_settings_stay_buildable() {
    // Capped segment counts keep the biggest cube to a few hundred
    // thousand points.
    let big = Primitive::Cube(CubeParams {
        segments: [MAX_SEGMENTS; 3],
        ..CubeParams::default()
    })
    .mesh();
    assert_eq!(big.face_count(), 6 * (MAX_SEGMENTS * MAX_SEGMENTS) as usize);
}

#[test]
fn every_kind_has_a_name() {
    let names: Vec<_> = Primitive::all().iter().map(Primitive::name).collect();
    assert_eq!(
        names,
        [
            "Cube", "Plane", "Cylinder", "Sphere", "Disc", "Tube", "Cone"
        ]
    );
}
