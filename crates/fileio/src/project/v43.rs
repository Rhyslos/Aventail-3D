//! Version 43 of the project file's contents: version 41's development
//! model (also used by version 42) with each object's modifier stack
//! ([decision 0142](../../../../docs/decisions/0142-modifier-stack-and-remesh.md)).
//!
//! Modifiers get a version of their own, as booleans and curves did,
//! because an older app would skip them without a word: it would show
//! the shapes unmodified and lose the modifiers on saving. With a new
//! version, the older app says it needs updating instead.
//!
//! A new kind of modifier adds a variant to [`KindV43`] and moves
//! [`super::FORMAT_VERSION`] up by one (as version 42 added an edit to
//! version 41's model), so an older app says it needs updating rather
//! than failing to read the kind. Still CBOR, so settings can be added
//! to a kind with a standard value (`#[serde(default)]`) until the alpha.
//!
//! Version 44 ([decision 0143](../../../../docs/decisions/0143-subdivide-and-creases.md))
//! adds the Subdivide modifier here, and one more kind of recorded edit,
//! creases (`v37::OperationV37::SetCreases`): files before version 44
//! never hold either.

use super::v1::Problem;
use super::v41::ProjectV41;
use crate::limits::Bounded;
use scene::{Document, Modifier, ModifierKind, ObjectId, RemeshOptions, SubdivideOptions};
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// Version 43 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV43 {
    /// Version 41's model, as it was.
    pub(crate) project: ProjectV41,
    /// Each object's modifiers (objects not listed have none).
    #[serde(default = "Bounded::new")]
    pub(crate) modifiers: Bounded<ModifiersV43>,
}

/// One object's modifiers, first to last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ModifiersV43 {
    /// The object.
    pub(crate) id: u64,
    /// Its modifiers.
    pub(crate) list: Bounded<ModifierV43>,
}

/// One modifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ModifierV43 {
    /// What it does, with its settings.
    pub(crate) kind: KindV43,
    /// Whether it is on.
    #[serde(default = "on")]
    pub(crate) on: bool,
    /// Whether the viewport shows it.
    #[serde(default = "on")]
    pub(crate) in_viewport: bool,
}

fn on() -> bool {
    true
}

/// A modifier's kind and settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum KindV43 {
    /// Voxel remesh.
    Remesh {
        /// Voxel size, in centimetres.
        voxel_size: f32,
        /// Relax passes.
        smooth: u32,
    },
    /// Subdivision smoothing (version 44).
    Subdivide {
        /// Levels in the viewport.
        levels: u32,
        /// Levels for exports and baking.
        export_levels: u32,
        /// Whether hard edges stay sharp.
        hard_edges_sharp: bool,
    },
}

impl KindV43 {
    fn of(kind: &ModifierKind) -> Self {
        match kind {
            ModifierKind::Remesh(o) => Self::Remesh {
                voxel_size: o.voxel_size,
                smooth: o.smooth,
            },
            ModifierKind::Subdivide(o) => Self::Subdivide {
                levels: o.levels,
                export_levels: o.export_levels,
                hard_edges_sharp: o.hard_edges_sharp,
            },
        }
    }

    /// The kind, its settings brought within limits by the scene.
    fn kind(&self) -> ModifierKind {
        match self {
            Self::Remesh { voxel_size, smooth } => ModifierKind::Remesh(RemeshOptions {
                voxel_size: *voxel_size,
                smooth: *smooth,
            }),
            Self::Subdivide {
                levels,
                export_levels,
                hard_edges_sharp,
            } => ModifierKind::Subdivide(SubdivideOptions {
                levels: *levels,
                export_levels: *export_levels,
                hard_edges_sharp: *hard_edges_sharp,
            }),
        }
    }
}

impl ProjectV43 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let modifiers = scene
            .walk()
            .iter()
            .filter(|(id, _)| !scene.modifiers(*id).is_empty())
            .map(|(id, _)| ModifiersV43 {
                id: id.get(),
                list: scene
                    .modifiers(*id)
                    .iter()
                    .map(|m| ModifierV43 {
                        kind: KindV43::of(&m.kind),
                        on: m.on,
                        in_viewport: m.in_viewport,
                    })
                    .collect(),
            })
            .collect();
        Self {
            project: ProjectV41::from_document(document),
            modifiers,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut document = self.project.into_document()?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in self.modifiers {
            if !seen.insert(entry.id) {
                return Err(tr("an object's modifiers are listed twice").into());
            }
            let id = ObjectId::from_raw(entry.id);
            if !document.scene().contains(id) {
                return Err(tr("its modifiers name an object that isn't there").into());
            }
            let list: Vec<Modifier> = entry
                .list
                .0
                .into_iter()
                .map(|m| Modifier {
                    kind: m.kind.kind(),
                    on: m.on,
                    in_viewport: m.in_viewport,
                })
                .collect();
            document = document
                .with_modifiers(id, list)
                .map_err(|why| Problem::from(trf!("a modifier can't be used: {}", why)))?;
        }
        Ok(document)
    }
}

/// The CBOR bytes of `model`.
pub(crate) fn encode(model: &ProjectV43) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(model, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The model in CBOR `bytes`; anything after it is refused.
pub(crate) fn decode(bytes: &[u8]) -> Result<ProjectV43, String> {
    let mut reader = std::io::Cursor::new(bytes);
    let model = ciborium::from_reader(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() as usize != bytes.len() {
        return Err(tr("there is unexpected data after the scene").into());
    }
    Ok(model)
}
