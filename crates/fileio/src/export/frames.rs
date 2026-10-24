//! Which of an object's two placings an export builds. Each is a whole
//! copy of the object's shape as polygons, normals and UVs, so building
//! only the one the file uses saves a good share of an export's time: glTF
//! keeps each object around its pivot (`local`), OBJ puts everything in
//! the world (`world`).

use super::{ExportFormat, ExportMesh, ExportedObject};

/// Which placings of each object an export builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frames {
    /// Only [`ExportedObject::world`] (OBJ).
    World,
    /// Only [`ExportedObject::local`] (glTF).
    Local,
    /// Both: for code that reads both (tests, mostly).
    Both,
}

impl Frames {
    /// Whether [`ExportedObject::world`] is built.
    pub fn world(self) -> bool {
        matches!(self, Self::World | Self::Both)
    }

    /// Whether [`ExportedObject::local`] is built.
    pub fn local(self) -> bool {
        matches!(self, Self::Local | Self::Both)
    }
}

impl ExportFormat {
    /// The placings this format writes.
    pub fn frames(self) -> Frames {
        match self {
            Self::Glb => Frames::Local,
            Self::Obj => Frames::World,
        }
    }
}

impl ExportedObject {
    /// Its shape as built: [`ExportedObject::world`], or
    /// [`ExportedObject::local`] when only that was. The two hold the same
    /// polygons and materials, placed differently, so either counts them.
    pub fn built(&self) -> &ExportMesh {
        if self.world.polygons.is_empty() && !self.local.polygons.is_empty() {
            &self.local
        } else {
            &self.world
        }
    }
}
