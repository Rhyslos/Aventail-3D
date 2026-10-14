//! Objects: what the scene is made of.

use crate::stack::Stack;
use glam::{Mat4, Vec3};
use mesh::{Bounds, Mesh, Primitive};
use std::sync::Arc;

/// An object's identity. Stable for the object's whole life, never reused
/// for another object (even after it is deleted), and handed out in order,
/// so replaying the same edits gives the same ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub(crate) u64);

impl ObjectId {
    /// The id as a number, for labels, debugging and saving.
    pub fn get(self) -> u64 {
        self.0
    }

    /// The id with this number, as saved earlier. Only for rebuilding a
    /// saved scene ([`crate::Scene::from_objects`]); new objects get their
    /// ids from the scene.
    pub const fn from_raw(number: u64) -> Self {
        Self(number)
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// What kind of object something is. For an object with a shape, this is
/// its *base*: what its shape starts from, before the edits recorded in
/// its [`Stack`]. More kinds (imported meshes, and much later joints) join
/// this list; nothing assumes it is complete.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ObjectKind {
    /// An empty object that holds others: a way to organise the scene and
    /// move things together. Has no shape of its own.
    Group,
    /// A ready-made shape built from settings: a cube, cylinder and so on.
    /// The settings stay editable, even after edits are recorded on top.
    Primitive(Primitive),
    /// A plain mesh, with no settings: what "Make editable" turns a shape
    /// into. Shared, as meshes can be large and are never changed in place.
    Mesh(Arc<Mesh>),
    /// An instance: shows another object's final shape, wherever it is put
    /// (decision 0055). Changing the original changes every instance. It
    /// has no edits of its own; making it editable gives it its own copy.
    Instance(ObjectId),
}

impl ObjectKind {
    /// The kind's name, used for new objects ("Group", "Cube").
    pub fn name(&self) -> &'static str {
        match self {
            Self::Group => "Group",
            Self::Primitive(p) => p.name(),
            Self::Mesh(_) => "Mesh",
            Self::Instance(_) => "Instance",
        }
    }

    /// Whether this kind has a shape (and so can have edits recorded).
    pub fn has_shape(&self) -> bool {
        !matches!(self, Self::Group)
    }

    /// The base's box, in the object's own space, before any recorded
    /// edits, or `None` for a kind with no shape. For the final shape's
    /// box, ask the scene ([`crate::Scene::local_bounds`]).
    pub fn local_bounds(&self) -> Option<Bounds> {
        match self {
            Self::Group | Self::Instance(_) => None,
            Self::Primitive(p) => Some(p.bounds()),
            Self::Mesh(mesh) => mesh.bounds(),
        }
    }

    /// The base's shape, in the object's own space, before any recorded
    /// edits, or `None` for a kind with no shape. For the final shape, ask
    /// the scene ([`crate::Scene::evaluated`]).
    pub fn mesh(&self) -> Option<Mesh> {
        match self {
            Self::Group | Self::Instance(_) => None,
            Self::Primitive(p) => Some(p.mesh()),
            Self::Mesh(mesh) => Some((**mesh).clone()),
        }
    }
}

/// Where an object sits relative to its parent (or the world, for an
/// object at the top of the tree): position in centimetres, rotation as
/// heading, pitch and bank in radians, and scale. Only this local transform
/// is stored; where an object is in the world is always worked out from
/// its parents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Position, in centimetres.
    pub position: Vec3,
    /// Heading (around Z), pitch (around X) and bank (around Y), radians.
    pub rotation: Vec3,
    /// Size factor along each axis.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// The transform as a matrix: scale, then bank, pitch and heading, then
    /// move into place.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_rotation_z(self.rotation.x)
            * Mat4::from_rotation_x(self.rotation.y)
            * Mat4::from_rotation_y(self.rotation.z)
            * Mat4::from_scale(self.scale)
    }

    /// The transform a matrix describes: the reverse of
    /// [`Self::matrix`]. A matrix that skews (scales along directions
    /// that aren't the object's own axes) can't be described exactly; the
    /// nearest transform without skew is given. `None` for a matrix that
    /// isn't usable (flattened to nothing, or not numbers).
    pub fn from_matrix(matrix: Mat4) -> Option<Self> {
        if !matrix.is_finite() || matrix.determinant().abs() < 1e-12 {
            return None;
        }
        let (scale, turn, position) = matrix.to_scale_rotation_translation();
        let (heading, pitch, bank) = turn.to_euler(glam::EulerRot::ZXY);
        let transform = Self {
            position,
            rotation: Vec3::new(heading, pitch, bank),
            scale,
        };
        transform.is_finite().then_some(transform)
    }

    /// Whether every value is a number.
    pub fn is_finite(&self) -> bool {
        self.position.is_finite() && self.rotation.is_finite() && self.scale.is_finite()
    }
}

/// One object in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// The name shown in the Objects panel. Never empty.
    pub name: String,
    /// What it is.
    pub kind: ObjectKind,
    /// Whether it is shown in the viewport. An object is only seen if its
    /// parents are shown too.
    pub visible: bool,
    /// Where it is, relative to its parent.
    pub transform: Transform,
    /// The edits recorded on top of its base, in layers. Always empty for
    /// a group. Changed only through commands.
    pub stack: Stack,
}

impl Object {
    /// A new, visible object of `kind`, named after it, in place.
    pub fn new(kind: ObjectKind) -> Self {
        Self {
            name: kind.name().to_string(),
            kind,
            visible: true,
            transform: Transform::default(),
            stack: Stack::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_objects_are_named_after_their_kind_and_shown() {
        let group = Object::new(ObjectKind::Group);
        assert_eq!(group.name, "Group");
        assert!(group.visible);
        assert_eq!(group.transform, Transform::default());
        assert_eq!(group.kind.local_bounds(), None);
        assert_eq!(group.kind.mesh(), None);
        let cube = Object::new(ObjectKind::Primitive(Primitive::Cube(Default::default())));
        assert_eq!(cube.name, "Cube");
        let bounds = cube.kind.local_bounds().unwrap();
        assert_eq!(bounds.size(), glam::Vec3::splat(100.0));
        assert_eq!(cube.kind.mesh().unwrap().face_count(), 6);
        assert!(cube.stack.is_empty());
        assert!(cube.kind.has_shape());
        assert!(!group.kind.has_shape());
    }

    #[test]
    fn a_mesh_base_is_its_own_shape() {
        let mesh = Arc::new(Primitive::Cube(Default::default()).mesh());
        let kind = ObjectKind::Mesh(Arc::clone(&mesh));
        assert_eq!(kind.name(), "Mesh");
        assert!(kind.has_shape());
        assert_eq!(kind.local_bounds(), mesh.bounds());
        assert_eq!(kind.mesh().as_ref(), Some(&*mesh));
        let empty = ObjectKind::Mesh(Arc::new(Mesh::new()));
        assert_eq!(empty.local_bounds(), None, "an empty mesh has no box");
    }

    #[test]
    fn the_default_transform_changes_nothing() {
        assert_eq!(Transform::default().matrix(), Mat4::IDENTITY);
    }

    #[test]
    fn transforms_scale_turn_then_move() {
        let t = Transform {
            position: Vec3::new(10.0, 0.0, 0.0),
            rotation: Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0),
            scale: Vec3::splat(2.0),
        };
        // X scaled to 2, turned a quarter around Z onto Y, moved along X.
        let p = t.matrix().transform_point3(Vec3::X);
        assert!((p - Vec3::new(10.0, 2.0, 0.0)).length() < 1e-5, "{p}");
    }

    #[test]
    fn ids_show_as_numbers() {
        assert_eq!(ObjectId(7).to_string(), "#7");
        assert_eq!(ObjectId(7).get(), 7);
    }
}
