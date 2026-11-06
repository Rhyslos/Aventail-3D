//! How the player sees a shape ([decision 0247](../../../docs/decisions/0247-player-view-uvs.md)):
//! from a few cameras placed as a game would place them (third person, first
//! person, top-down), which polygons are seen, and how many screen pixels a
//! metre of each covers at its nearest. The automatic UV tools use it to
//! hide seams where the player doesn't look and to give texture where the
//! player looks closest; the UV Editor shows texels per screen pixel.

use crate::Rays;
use glam::Vec3;
use mesh::{FaceId, Mesh};
use std::collections::BTreeMap;

/// How the player looks at the asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerKind {
    /// From behind a character, at eye height, a few metres off: the
    /// standard.
    #[default]
    ThirdPerson,
    /// Through the character's eyes, close up.
    FirstPerson,
    /// From high above at a slant, as in strategy games.
    TopDown,
}

impl PlayerKind {
    /// Every kind, in the order offered.
    pub const ALL: [Self; 3] = [Self::ThirdPerson, Self::FirstPerson, Self::TopDown];

    /// The standard distance from the asset, in metres.
    pub fn standard_distance(self) -> f32 {
        match self {
            Self::ThirdPerson => 3.0,
            Self::FirstPerson => 1.5,
            Self::TopDown => 12.0,
        }
    }

    /// The eye's height above the ground (for third and first person) or
    /// the slant from straight down (top-down, degrees).
    fn height(self) -> f32 {
        match self {
            Self::ThirdPerson => 1.7,
            Self::FirstPerson => 1.6,
            Self::TopDown => 0.0,
        }
    }
}

/// The player's cameras.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerView {
    /// How they look.
    pub kind: PlayerKind,
    /// How far from the asset's middle, in metres.
    pub distance: f32,
    /// The screen's height in pixels (1080 standard).
    pub screen_height: u32,
    /// The camera's field of view up and down, in degrees (60 standard).
    pub field_of_view: f32,
}

impl Default for PlayerView {
    fn default() -> Self {
        Self {
            kind: PlayerKind::ThirdPerson,
            distance: PlayerKind::ThirdPerson.standard_distance(),
            screen_height: 1080,
            field_of_view: 60.0,
        }
    }
}

/// The cameras placed round each asset.
const AROUND: usize = 8;
/// The limits of the distance, in metres.
pub const DISTANCE_RANGE: std::ops::RangeInclusive<f32> = 0.1..=1000.0;

/// What the player sees of one polygon.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Seen {
    /// The share of the cameras that see it (0 never, 1 from everywhere).
    pub seen: f32,
    /// How many screen pixels a metre of it covers from the nearest camera
    /// that sees it (0 when none does).
    pub pixels_per_metre: f32,
}

impl PlayerView {
    /// Where the cameras are for a shape whose box runs from `min` to
    /// `max` (Z up), all looking at its middle.
    pub fn cameras(&self, min: Vec3, max: Vec3) -> Vec<Vec3> {
        let middle = (min + max) / 2.0;
        let distance = self.distance.clamp(*DISTANCE_RANGE.start(), *DISTANCE_RANGE.end());
        // Never inside the shape.
        let reach = distance.max((max - min).length() * 0.6);
        (0..AROUND)
            .map(|i| {
                let turn = std::f32::consts::TAU * i as f32 / AROUND as f32;
                let (sin, cos) = turn.sin_cos();
                match self.kind {
                    PlayerKind::TopDown => {
                        let slant = 50f32.to_radians();
                        middle
                            + Vec3::new(cos * slant.sin(), sin * slant.sin(), slant.cos()) * reach
                    }
                    kind => {
                        let eye = min.z + kind.height();
                        let flat = (reach * reach - (eye - middle.z).powi(2)).max(0.0).sqrt();
                        Vec3::new(middle.x + cos * flat.max(reach * 0.3), middle.y + sin * flat.max(reach * 0.3), eye)
                    }
                }
            })
            .collect()
    }

    /// Screen pixels a metre covers `distance` metres from the camera.
    fn pixels_per_metre(&self, distance: f32) -> f32 {
        let half = (self.field_of_view.clamp(10.0, 170.0).to_radians() / 2.0).tan();
        self.screen_height.max(1) as f32 / (2.0 * distance.max(1e-3) * half)
    }
}

/// What the player sees of each of `mesh`'s polygons, by id.
pub fn player_view(mesh: &Mesh, view: &PlayerView) -> BTreeMap<FaceId, Seen> {
    let mut triangles = Vec::new();
    let mut middles = Vec::new();
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (face, corners) in mesh.faces() {
        let points: Vec<Vec3> = corners.iter().filter_map(|v| mesh.position(*v)).collect();
        if points.len() < 3 || points.len() != corners.len() {
            continue;
        }
        for p in &points {
            min = min.min(*p);
            max = max.max(*p);
        }
        mesh::triangulate::face_triangles(mesh, corners, |[a, b, c]| {
            triangles.push([points[a], points[b], points[c]]);
        });
        let middle = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        middles.push((face, middle, mesh.face_normal(face)));
    }
    if middles.is_empty() {
        return BTreeMap::new();
    }
    let rays = Rays::of_triangles(triangles);
    let cameras = view.cameras(min, max);
    let size = (max - min).length().max(1e-6);
    middles
        .into_iter()
        .map(|(face, middle, normal)| {
            let mut seen = 0;
            let mut nearest = f32::MAX;
            for camera in &cameras {
                let to = *camera - middle;
                let distance = to.length();
                if distance <= 1e-6 || normal.dot(to) <= 0.0 {
                    continue;
                }
                // From just off the polygon toward the camera: anything in
                // the way hides it.
                let direction = to / distance;
                let from = middle + direction * (size * 1e-4);
                if rays.hit(from, direction, distance).is_none() {
                    seen += 1;
                    nearest = nearest.min(distance);
                }
            }
            let pixels = if seen > 0 {
                view.pixels_per_metre(nearest)
            } else {
                0.0
            };
            (
                face,
                Seen {
                    seen: seen as f32 / cameras.len() as f32,
                    pixels_per_metre: pixels,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::primitives::{CubeParams, Primitive};

    fn cube() -> Mesh {
        Primitive::Cube(CubeParams::default()).mesh()
    }

    #[test]
    fn a_cube_from_third_person_shows_its_sides_and_top_but_not_its_bottom() {
        let mesh = cube();
        let seen = player_view(&mesh, &PlayerView::default());
        assert_eq!(seen.len(), 6);
        for (face, s) in &seen {
            let n = mesh.face_normal(*face);
            if n.z < -0.5 {
                assert_eq!(s.seen, 0.0, "the bottom is never seen");
                assert_eq!(s.pixels_per_metre, 0.0);
            } else {
                assert!(s.seen > 0.0, "{n:?} {s:?}");
                assert!(s.pixels_per_metre > 0.0);
            }
        }
    }

    #[test]
    fn closer_cameras_give_more_pixels() {
        let mesh = cube();
        let near = player_view(
            &mesh,
            &PlayerView {
                distance: 2.0,
                ..Default::default()
            },
        );
        let far = player_view(
            &mesh,
            &PlayerView {
                distance: 20.0,
                ..Default::default()
            },
        );
        let side = near
            .iter()
            .find(|(_, s)| s.seen > 0.0)
            .map(|(f, _)| *f)
            .unwrap();
        assert!(near[&side].pixels_per_metre > far[&side].pixels_per_metre * 3.0);
    }

    #[test]
    fn a_hidden_polygon_isnt_seen() {
        // A small cube inside a big one: none of it is seen.
        let mut mesh = Primitive::Cube(CubeParams {
            size: Vec3::splat(4.0),
            segments: [1, 1, 1],
        })
        .mesh();
        let inner = cube();
        let before = mesh.face_count();
        mesh::parts::append(&mut mesh, &inner, glam::Mat4::IDENTITY);
        let seen = player_view(&mesh, &PlayerView::default());
        let hidden = seen.iter().skip(before).filter(|(_, s)| s.seen == 0.0).count();
        assert_eq!(hidden, inner.face_count());
    }

    #[test]
    fn top_down_sees_the_top_most() {
        let mesh = cube();
        let seen = player_view(
            &mesh,
            &PlayerView {
                kind: PlayerKind::TopDown,
                distance: PlayerKind::TopDown.standard_distance(),
                ..Default::default()
            },
        );
        let top = seen
            .iter()
            .find(|(f, _)| mesh.face_normal(**f).z > 0.5)
            .unwrap()
            .1;
        assert_eq!(top.seen, 1.0);
    }

    #[test]
    fn an_empty_or_broken_shape_gives_nothing() {
        assert!(player_view(&Mesh::new(), &PlayerView::default()).is_empty());
        let odd = PlayerView {
            distance: f32::NAN,
            field_of_view: 0.0,
            screen_height: 0,
            ..Default::default()
        };
        let seen = player_view(&cube(), &odd);
        assert!(seen.values().all(|s| s.seen.is_finite() && s.pixels_per_metre.is_finite()));
    }
}
