//! Mirror painting ([decision 0187](../../../../docs/decisions/0187-mirror-painting.md)):
//! each dab is painted again on the other side of the object, reflected
//! across one of the object's own axes through its pivot.
//!
//! On the model, the other side is painted as if seen by the stroke's
//! camera reflected across the same plane. Seen that way, the reflected
//! dab lands on the same screen point as the dab itself, so the brush's
//! tip, projection's picture and hidden parts all work the same on both
//! sides, and a model that isn't quite symmetrical still gets the paint
//! on its surface. In the UV Editor, the dab is painted again flipped
//! left and right across the texture's middle.
//!
//! Smudge and clone don't mirror: both carry colour from one place, which
//! would lay the first side's colour on the other.

use crate::attributes::MirrorAxis;
use glam::{Mat4, Vec2};
use paint::{BrushKind, DepthMap, PaintSurface, View};

/// The other side of a stroke on the model.
#[derive(Debug)]
pub(super) struct ModelMirror {
    /// The reflection in the object's own space.
    pub(super) reflect: Mat4,
    /// The stroke's view reflected across the mirror's plane.
    pub(super) view: View,
    /// How deep the shape is, seen from the reflected view.
    pub(super) depth: DepthMap,
}

impl ModelMirror {
    /// The other side of a stroke painted on `surface` (placed by
    /// `model`) through `view`, reflected across the object's `axis`.
    pub(super) fn new(axis: MirrorAxis, surface: &PaintSurface, model: Mat4, view: &View) -> Self {
        let reflect = axis.reflection();
        let view = reflected_view(view, model * reflect * model.inverse());
        let depth = DepthMap::of(surface, model, &view);
        Self {
            reflect,
            view,
            depth,
        }
    }
}

/// `view` reflected by `world` (a reflection in world space, its own
/// inverse): what it sees at a point, the reflected view sees at the
/// point's reflection, on the same spot of the screen.
pub(super) fn reflected_view(view: &View, world: Mat4) -> View {
    View {
        view_projection: view.view_projection * world,
        size: view.size,
        eye: world.transform_point3(view.eye),
        forward: world.transform_vector3(view.forward).normalize_or_zero(),
        flat: view.flat,
    }
}

/// Whether a brush of `kind` mirrors.
pub(super) fn mirrors(kind: BrushKind) -> bool {
    !matches!(kind, BrushKind::Smudge | BrushKind::Clone)
}

/// A place on the texture flipped left and right across its middle.
pub(super) fn flipped_uv(uv: Vec2) -> Vec2 {
    Vec2::new(1.0 - uv.x, uv.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn the_reflected_view_sees_the_reflection_on_the_same_spot() {
        let camera = tools::Camera::default();
        let size = Vec2::new(300.0, 200.0);
        let view_projection = camera.view_projection(size);
        let view = View {
            view_projection,
            size,
            eye: camera.position,
            forward: camera.forward(),
            flat: false,
        };
        // An object moved and turned: its X axis isn't the world's.
        let model = Mat4::from_rotation_y(0.7) * Mat4::from_translation(Vec3::new(30.0, 0.0, 0.0));
        let reflect = MirrorAxis::X.reflection();
        let world = model * reflect * model.inverse();
        let mirrored = reflected_view(&view, world);
        let point = Vec3::new(40.0, 30.0, 20.0);
        let (a, depth_a) = view.screen(point).unwrap();
        let (b, depth_b) = mirrored.screen(world.transform_point3(point)).unwrap();
        assert!(a.distance(b) < 1e-3, "{a} vs {b}");
        assert!((depth_a - depth_b).abs() < 1e-5);
        // The reflected eye is as far from the reflected point.
        assert!(
            (view.eye.distance(point) - mirrored.eye.distance(world.transform_point3(point))).abs()
                < 1e-2
        );
        // Reflecting twice is where it began.
        assert!((world * world).abs_diff_eq(Mat4::IDENTITY, 1e-5));
    }

    #[test]
    fn uvs_flip_across_the_middle_and_smudge_and_clone_stay_put() {
        assert_eq!(flipped_uv(Vec2::new(0.2, 0.7)), Vec2::new(0.8, 0.7));
        assert_eq!(flipped_uv(Vec2::new(0.5, 0.1)), Vec2::new(0.5, 0.1));
        assert!(mirrors(BrushKind::Paint) && mirrors(BrushKind::Erase));
        assert!(mirrors(BrushKind::Blur) && mirrors(BrushKind::Projection));
        assert!(!mirrors(BrushKind::Smudge) && !mirrors(BrushKind::Clone));
    }

    #[test]
    fn each_axis_flips_its_own_coordinate() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(
            MirrorAxis::X.reflection().transform_point3(p),
            Vec3::new(-1.0, 2.0, 3.0)
        );
        assert_eq!(
            MirrorAxis::Y.reflection().transform_point3(p),
            Vec3::new(1.0, -2.0, 3.0)
        );
        assert_eq!(
            MirrorAxis::Z.reflection().transform_point3(p),
            Vec3::new(1.0, 2.0, -3.0)
        );
    }
}
