//! `.avtl` project files: saving a document and opening it again
//! (DESIGN.md §8, decisions [0007](../../../docs/decisions/0007-rhy-project-format.md),
//! [0010](../../../docs/decisions/0010-avtl-file-extension.md) and
//! [0024](../../../docs/decisions/0024-saving-projects.md)).
//!
//! A file is a 20-byte header followed by the body:
//!
//! | Bytes | What |
//! |---|---|
//! | 0–3 | The signature `AVTL`, so a project is recognised at once |
//! | 4–7 | The format version (little-endian), so old files can be upgraded and newer ones refused clearly |
//! | 8–11 | Flags; bit 0 says the body is zstd-compressed. Other bits are reserved and must be 0 |
//! | 12–19 | The body's length in bytes |
//! | 20– | The body: the file model of that version (`project/v1.rs` to `project/v24.rs`), encoded with postcard |
//!
//! Reading never trusts the file: lengths are checked before anything is
//! allocated, the unpacked body is capped, every list is paid for from a
//! memory budget while it is read, counts of objects, edits and polygons
//! are held to the limits rulebook ([`crate::limits`]), and every value
//! is checked before it becomes part of a scene. Problems come back as a
//! [`ProjectError`] in plain words; reading never panics.

mod v1;
mod v10;
mod v11;
mod v12;
mod v13;
mod v14;
mod v15;
mod v16;
mod v17;
mod v18;
mod v19;
mod v2;
mod v20;
mod v21;
mod v22;
mod v23;
mod v24;
mod v25;
mod v26;
mod v27;
mod v28;
mod v29;
mod v3;
mod v30;
mod v31;
mod v32;
mod v33;
mod v34;
mod v35;
mod v36;
mod v37;
mod v38;
mod v39;
mod v4;
mod v40;
mod v41;
mod v43;
mod v45;
mod v5;
mod v50;
mod v51;
mod v52;
mod v53;
mod v54;
mod v6;
mod v7;
mod v8;
mod v9;

use scene::Document;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use words::{text, tr, trf};

/// The signature every project file starts with.
pub const SIGNATURE: [u8; 4] = *b"AVTL";
/// The format version this app writes.
///
/// Version 42 (decision 0141) is version 41's model with one more kind of
/// recorded edit, an improved bevel (`v23::OperationV23::Improved`): a
/// version of its own, so an older app says it needs updating instead of
/// failing to read the edit.
///
/// Version 43 (decision 0142) is version 41's model with each object's
/// modifier stack (`v43::ProjectV43`). A new kind of modifier moves it up
/// by one again.
///
/// Version 44 (decision 0143) is version 43 with the Subdivide modifier
/// (`v43::KindV43::Subdivide`) and creases, one more kind of recorded
/// edit (`v37::OperationV37::SetCreases`).
///
/// Version 45 (decision 0144) is version 44 with lattice cages
/// (`v45::LatticeV45`) and the Lattice modifier (`v43::KindV43::Lattice`).
///
/// Version 46 (decision 0145) is version 45 with the Decimate modifier
/// (`v43::KindV43::Decimate`).
///
/// Version 47 (decision 0146) is version 46 with the Scatter modifier
/// (`v43::KindV43::Scatter`).
///
/// Version 48 (decision 0147) is version 47 with the Bend along curve and
/// Shrinkwrap modifiers (`v43::KindV43::BendAlongCurve` and `Shrinkwrap`).
///
/// Version 49 (decision 0148) is version 48 with the Weighted normals
/// modifier (`v43::KindV43::WeightedNormals`).
///
/// Version 50 (decision 0149) is version 49 with the Live bevel modifier
/// (`v43::KindV43::LiveBevel`) and bevel weights, one more kind of
/// recorded edit (`v37::OperationV37::SetBevelWeights`).
///
/// Version 51 (decision 0150) is version 50 with the Quadify modifier
/// (`v43::KindV43::Quadify`) and the one-off Quadify, one more kind of
/// recorded edit (`v37::OperationV37::Quadify`).
///
/// Version 52 (decision 0151) is version 51 with collision in parts
/// (*Wrapped in parts*) and its settings (`v52::CollisionPartsV52`, in
/// each object's development settings).
///
/// Version 53 (decision 0152) is version 52 with the Weld by distance
/// modifier (`v43::KindV43::WeldByDistance`) and the one-off Weld by
/// distance now, one more kind of recorded edit
/// (`v37::OperationV37::WeldByDistance`).
///
/// Version 54 (decision 0153) is version 53 with the Array modifier
/// (`v43::KindV43::Array`).
pub const FORMAT_VERSION: u32 = 54;
/// Flag: the body is compressed with zstd.
const FLAG_ZSTD: u32 = 1;
/// The flags this version knows.
const KNOWN_FLAGS: u32 = FLAG_ZSTD;
/// The header's size in bytes.
const HEADER_LEN: usize = 20;
/// The largest body, packed or unpacked, a file may have (the limits
/// rulebook, [`crate::limits`]). Anything larger is taken as damaged
/// rather than read into memory.
const MAX_BODY: u64 = crate::limits::MAX_UNPACKED;
/// How hard zstd works: a good balance of speed and size for scenes.
const ZSTD_LEVEL: i32 = 3;

/// Why a project couldn't be saved or opened, in plain words.
#[derive(Debug)]
pub enum ProjectError {
    /// The file isn't a project file (it doesn't start with `AVTL`).
    NotAProject,
    /// The file was saved by a newer version of the app, in a format this
    /// version doesn't know.
    TooNew {
        /// The file's format version.
        version: u32,
    },
    /// The file is a project but parts of it are damaged or missing.
    Damaged(String),
    /// The file says it is larger than any project could be.
    TooLarge,
    /// Saving was stopped because the saved project wouldn't open again
    /// (or an edit couldn't be stored): writing it would have lost work.
    /// The previous save is untouched.
    WouldNotReopen(String),
    /// The file couldn't be read or written.
    Io {
        /// What was being done ("read", "write"), in English: shown
        /// through `tr`.
        action: &'static str,
        /// The file.
        path: PathBuf,
        /// What the system said.
        source: std::io::Error,
    },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAProject => f.write_str(tr("it isn't a project file")),
            Self::TooNew { version } => f.write_str(&trf!(
                "it was saved by a newer version of the app (file format {}; this version \
                 reads up to {}). Update the app to open it",
                version,
                FORMAT_VERSION
            )),
            Self::Damaged(why) => f.write_str(&trf!("the file is damaged: {}", why)),
            Self::TooLarge => {
                f.write_str(tr("the file is damaged: it claims to be impossibly large"))
            }
            Self::WouldNotReopen(why) => f.write_str(&trf!(
                "it wasn't saved, because the saved copy wouldn't open again ({}). Your \
                 previous save is untouched; please report this so it can be fixed",
                why
            )),
            Self::Io {
                action,
                path,
                source,
            } => f.write_str(&trf!(
                "couldn't {} {}: {}",
                tr(action),
                path.display(),
                source
            )),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The project file's bytes for `document`, checked by reading them back
/// with the same loader that opens projects: bytes that wouldn't open
/// again are never handed out, so a save can't silently lose a project.
pub fn to_bytes(document: &Document) -> Result<Vec<u8>, ProjectError> {
    let bytes = encode(document)?;
    match from_bytes(&bytes) {
        Ok(_) => Ok(bytes),
        Err(why) => Err(ProjectError::WouldNotReopen(why.to_string())),
    }
}

/// How many recorded edits each object holds.
fn edit_counts(document: &Document) -> usize {
    let scene = document.scene();
    scene
        .walk()
        .iter()
        .filter_map(|(id, _)| scene.object(*id))
        .map(|o| o.stack.operation_count())
        .sum()
}

/// The project file's bytes for `document`, unchecked.
fn encode(document: &Document) -> Result<Vec<u8>, ProjectError> {
    let model = v43::ProjectV43::from_document(document);
    let stored: usize = model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .objects
        .iter()
        .flat_map(|o| o.layers.iter())
        .map(|l| l.operations.len())
        .sum();
    if stored != edit_counts(document) {
        return Err(ProjectError::WouldNotReopen(
            tr("an edit of a kind this version can't store yet").into(),
        ));
    }
    let encoded = v43::encode(&model)
        .map_err(|e| ProjectError::Damaged(trf!("the scene couldn't be encoded ({})", e)))?;
    let body = zstd::bulk::compress(&encoded, ZSTD_LEVEL)
        .map_err(|e| ProjectError::Damaged(trf!("the scene couldn't be compressed ({})", e)))?;
    let mut bytes = Vec::with_capacity(HEADER_LEN + body.len());
    bytes.extend_from_slice(&SIGNATURE);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&FLAG_ZSTD.to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}

/// Reads a little-endian number from `bytes` at `at`.
fn number<const N: usize>(bytes: &[u8], at: usize) -> Option<[u8; N]> {
    bytes.get(at..at + N)?.try_into().ok()
}

/// The document a project file's bytes hold.
pub fn from_bytes(bytes: &[u8]) -> Result<Document, ProjectError> {
    if bytes.len() < SIGNATURE.len() || bytes[..SIGNATURE.len()] != SIGNATURE {
        return Err(ProjectError::NotAProject);
    }
    let header = |what| ProjectError::Damaged(trf!("its header is cut short ({})", tr(what)));
    let version = u32::from_le_bytes(number(bytes, 4).ok_or_else(|| header(text!("version")))?);
    let flags = u32::from_le_bytes(number(bytes, 8).ok_or_else(|| header(text!("flags")))?);
    let length = u64::from_le_bytes(number(bytes, 12).ok_or_else(|| header(text!("length")))?);
    match version {
        0 => return Err(ProjectError::Damaged(tr("its format version is 0").into())),
        1..=FORMAT_VERSION => {}
        newer => return Err(ProjectError::TooNew { version: newer }),
    }
    if flags & !KNOWN_FLAGS != 0 {
        // Flags this version doesn't know mean a newer writer.
        return Err(ProjectError::TooNew { version });
    }
    if length > MAX_BODY {
        return Err(ProjectError::TooLarge);
    }
    let body = &bytes[HEADER_LEN.min(bytes.len())..];
    if body.len() as u64 != length {
        return Err(ProjectError::Damaged(trf!(
            "it should hold {} bytes of scene but holds {}; it may be cut short",
            length,
            body.len()
        )));
    }
    let unpacked;
    let encoded = if flags & FLAG_ZSTD != 0 {
        unpacked = unpack(body)?;
        unpacked.as_slice()
    } else {
        body
    };
    // Older versions are read with their own model (decision 0031).
    match version {
        1 => decode::<v1::OperationV1>(encoded),
        2 => decode::<v2::OperationV2>(encoded),
        3 => decode::<v3::OperationV3>(encoded),
        4 => decode::<v4::OperationV4>(encoded),
        5 => decode_entries::<v5::EntryV5>(encoded),
        6 => decode_entries::<v6::EntryV6>(encoded),
        7 => decode_entries::<v7::EntryV7>(encoded),
        8 => decode_entries::<v8::EntryV8>(encoded),
        9 => decode_entries::<v9::EntryV9>(encoded),
        10 => decode_entries::<v10::EntryV10>(encoded),
        11 => decode_entries::<v11::EntryV11>(encoded),
        12 => decode_instances::<v12::EntryV12>(encoded),
        13 => decode_instances::<v13::EntryV13>(encoded),
        14 => decode_instances::<v14::EntryV14>(encoded),
        15 => decode_instances::<v15::EntryV15>(encoded),
        16 => decode_places::<v15::EntryV15>(encoded),
        17 => decode_places::<v17::EntryV17>(encoded),
        18 => decode_materials(encoded),
        19 => decode_targets(encoded),
        20 => decode_collisions::<v18::EntryV18>(encoded),
        21 => decode_collisions::<v21::EntryV21>(encoded),
        22 => decode_collisions::<v22::EntryV22>(encoded),
        23 => decode_collisions::<v23::EntryV23>(encoded),
        24 => decode_collisions::<v24::EntryV24>(encoded),
        25 => decode_mirrors::<v24::EntryV24>(encoded),
        26 => decode_mirrors::<v26::EntryV26>(encoded),
        27 => decode_names::<v26::EntryV26>(encoded),
        28 => decode_names::<v28::EntryV28>(encoded),
        29 => decode_names::<v29::EntryV29>(encoded),
        30 => decode_names::<v30::EntryV30>(encoded),
        31 => decode_texture::<v31::EntryV31>(encoded),
        32 => decode_texture::<v32::EntryV32>(encoded),
        33 => decode_trim::<v33::EntryV33>(encoded),
        34 => decode_trim::<v34::EntryV34>(encoded),
        35 => decode_mending::<v35::EntryV35>(encoded),
        36 => decode_mending::<v36::EntryV36>(encoded),
        37 => decode_mending::<v37::EntryV37>(encoded),
        38 => decode_snapshots::<v38::EntryV38>(encoded),
        39 => decode_development(encoded),
        40 => decode_booleans(encoded),
        41 | 42 => decode_curves(encoded),
        _ => decode_modifiers(encoded),
    }
}

/// The document a body of version 39 (the development format, decision
/// 0126) holds.
fn decode_development(encoded: &[u8]) -> Result<Document, ProjectError> {
    let model = crate::limits::within_budget(|| v39::decode(encoded))
        .map_err(|e| ProjectError::Damaged(trf!("its scene can't be read ({})", e)))?;
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 40 (the development format with
/// booleans, decision 0135) holds.
fn decode_booleans(encoded: &[u8]) -> Result<Document, ProjectError> {
    let model = crate::limits::within_budget(|| v40::decode(encoded))
        .map_err(|e| ProjectError::Damaged(trf!("its scene can't be read ({})", e)))?;
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 41 (the development format with
/// curves, decision 0138) or 42 (with improved bevels, decision 0141)
/// holds.
fn decode_curves(encoded: &[u8]) -> Result<Document, ProjectError> {
    let model = crate::limits::within_budget(|| v41::decode(encoded))
        .map_err(|e| ProjectError::Damaged(trf!("its scene can't be read ({})", e)))?;
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 43 (the development format with the
/// modifier stack, decision 0142) or later holds.
fn decode_modifiers(encoded: &[u8]) -> Result<Document, ProjectError> {
    let model = crate::limits::within_budget(|| v43::decode(encoded))
        .map_err(|e| ProjectError::Damaged(trf!("its scene can't be read ({})", e)))?;
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 18 or later holds.
fn decode_materials(encoded: &[u8]) -> Result<Document, ProjectError> {
    let (model, rest): (v18::ProjectV18, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 16 or later holds, its recorded edits
/// read as `E`.
fn decode_places<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v16::ProjectWithPlaces<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 19 or later holds.
fn decode_targets(encoded: &[u8]) -> Result<Document, ProjectError> {
    let (model, rest): (v19::ProjectV19, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 20 or later holds, its recorded edits
/// read as `E`.
fn decode_collisions<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v20::ProjectV20<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 25 or later holds, its recorded edits
/// read as `E`.
fn decode_mirrors<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v25::ProjectV25<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 27 or later holds, its recorded edits
/// read as `E`.
fn decode_names<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v27::ProjectV27<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// Reads version 31 and later: version 27's model with edits stored as
/// `E`, and the texture settings.
fn decode_texture<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v31::ProjectV31<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// Reads version 33 and later: version 31's model with edits stored as
/// `E`, and the trim sheet.
fn decode_trim<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v33::ProjectV33<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// Reads version 35 and later: version 33's model with edits stored as
/// `E`, and the mending settings.
fn decode_mending<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v35::ProjectV35<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// Reads version 38 and later: version 35's model with edits stored as
/// `E`, and the layout snapshots.
fn decode_snapshots<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v38::ProjectV38<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .project
        .places
        .project
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// Why a body couldn't be read: a limit, or damage.
fn unreadable(error: postcard::Error) -> ProjectError {
    if crate::limits::limit_hit() {
        ProjectError::Damaged(tr(crate::limits::TOO_MUCH).into())
    } else {
        ProjectError::Damaged(trf!("the scene can't be read ({})", error))
    }
}

/// The document a body of version 5 or later holds, its recorded edits
/// read as `E`.
fn decode_entries<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v5::ProjectWith<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document a body of version 12 or later holds (with instances), its
/// recorded edits read as `E`.
fn decode_instances<E>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    E: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v12::ProjectWithInstances<E>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model
        .project
        .check_limits()
        .map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// The document an unpacked body holds, read with the model for
/// operations of type `Op`.
fn decode<Op>(encoded: &[u8]) -> Result<Document, ProjectError>
where
    Op: v1::FileOperation + serde::de::DeserializeOwned,
{
    let (model, rest): (v1::ProjectFile<Op>, &[u8]) =
        crate::limits::within_budget(|| postcard::take_from_bytes(encoded)).map_err(unreadable)?;
    if !rest.is_empty() {
        return Err(ProjectError::Damaged(
            tr("there is unexpected data after the scene").into(),
        ));
    }
    model.check_limits().map_err(ProjectError::Damaged)?;
    model.into_document().map_err(ProjectError::Damaged)
}

/// Unpacks a zstd body, refusing to unpack more than [`MAX_BODY`] bytes.
/// A body that says up front it is larger is refused without unpacking.
fn unpack(body: &[u8]) -> Result<Vec<u8>, ProjectError> {
    if let Ok(Some(size)) = zstd::zstd_safe::get_frame_content_size(body)
        && size > MAX_BODY
    {
        return Err(ProjectError::TooLarge);
    }
    let decoder = zstd::stream::read::Decoder::new(body)
        .map_err(|e| ProjectError::Damaged(trf!("the scene can't be unpacked ({})", e)))?;
    let mut out = Vec::new();
    decoder
        .take(MAX_BODY + 1)
        .read_to_end(&mut out)
        .map_err(|e| ProjectError::Damaged(trf!("the scene can't be unpacked ({})", e)))?;
    if out.len() as u64 > MAX_BODY {
        return Err(ProjectError::TooLarge);
    }
    Ok(out)
}

/// The backup kept of the previous save: `name.avtl.bak`.
pub fn backup_path(path: &Path) -> PathBuf {
    with_suffix(path, ".bak")
}

/// `path` with `suffix` added to the file name.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// Saves `document` to `path`, safely:
///
/// 1. The project's bytes are checked to open again (see [`to_bytes`]).
///    If they wouldn't, nothing is replaced; the bytes go to
///    `name.avtl.unreadable` for a bug report, and saving fails.
/// 2. They are written to a new, uniquely named temporary file beside
///    the project (never one that already exists, so a planted link
///    can't redirect the write) and read back to check they arrived
///    whole.
/// 3. The previous save, if any, becomes `name.avtl.bak` the same way.
/// 4. The temporary file replaces the project in one step.
///
/// So a crash, a full disk or a bug never leaves a half-written or
/// unreadable project: the old file stays as it was until the new one is
/// complete and known to open.
pub fn save(path: &Path, document: &Document) -> Result<(), ProjectError> {
    let io = |action, path: &Path| {
        let path = path.to_path_buf();
        move |source| ProjectError::Io {
            action,
            path,
            source,
        }
    };
    let bytes = match to_bytes(document) {
        Ok(bytes) => bytes,
        Err(error @ ProjectError::WouldNotReopen(_)) => {
            if let Ok(raw) = encode(document) {
                let _ = safe_write(&with_suffix(path, ".unreadable"), &raw);
            }
            return Err(error);
        }
        Err(other) => return Err(other),
    };
    if path.exists() {
        let previous = std::fs::read(path).map_err(io(text!("keep a backup of"), path))?;
        safe_write(&backup_path(path), &previous).map_err(io(text!("keep a backup of"), path))?;
    }
    safe_write(path, &bytes).map_err(io(text!("write"), path))
}

/// Writes `bytes` to `path` without ever writing through a file or link
/// already at a temporary name: a new, uniquely named temporary file
/// beside it is created (refusing one that exists), written, synced, read
/// back, and then renamed over `path` in one step. Renaming replaces a
/// link at `path` itself rather than the file it points to. Every save,
/// backup and copy goes through here.
pub fn safe_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = unique_temporary(path)?;
    let result = (|| {
        {
            let mut file = std::fs::File::options()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            std::io::Write::write_all(&mut file, bytes)?;
            file.sync_all()?;
        }
        if std::fs::read(&temporary)? != bytes {
            return Err(std::io::Error::other(tr(
                "the saved file didn't read back the same; the disk may be failing",
            )));
        }
        std::fs::rename(&temporary, path)?;
        sync_folder(path);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// A temporary file name beside `path` that doesn't exist yet.
fn unique_temporary(path: &Path) -> std::io::Result<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    for _ in 0..100 {
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let candidate = with_suffix(path, &format!(".{}-{nanos:x}-{n}.tmp", std::process::id()));
        if std::fs::symlink_metadata(&candidate).is_err() {
            return Ok(candidate);
        }
    }
    Err(std::io::Error::other(tr(
        "couldn't find a free temporary file name",
    )))
}

/// Makes a rename in `path`'s folder reach the disk, where the system
/// allows it (Linux and macOS; Windows has no such step).
fn sync_folder(path: &Path) {
    #[cfg(unix)]
    if let Some(folder) = path.parent()
        && let Ok(dir) = std::fs::File::open(if folder.as_os_str().is_empty() {
            Path::new(".")
        } else {
            folder
        })
    {
        let _ = dir.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Opens the project at `path`.
pub fn open(path: &Path) -> Result<Document, ProjectError> {
    let size = std::fs::metadata(path)
        .map_err(|source| ProjectError::Io {
            action: text!("read"),
            path: path.to_path_buf(),
            source,
        })?
        .len();
    if size > MAX_BODY + HEADER_LEN as u64 {
        return Err(ProjectError::TooLarge);
    }
    let bytes = std::fs::read(path).map_err(|source| ProjectError::Io {
        action: text!("read"),
        path: path.to_path_buf(),
        source,
    })?;
    from_bytes(&bytes)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod v40_tests;
#[cfg(test)]
mod v41_tests;
#[cfg(test)]
mod v42_tests;
#[cfg(test)]
mod v43_tests;
#[cfg(test)]
mod v44_tests;
#[cfg(test)]
mod v45_tests;
#[cfg(test)]
mod v46_tests;
#[cfg(test)]
mod v47_tests;
#[cfg(test)]
mod v48_tests;
#[cfg(test)]
mod v49_tests;
#[cfg(test)]
mod v50_tests;
#[cfg(test)]
mod v51_tests;
#[cfg(test)]
mod v52_tests;
#[cfg(test)]
mod v53_tests;
#[cfg(test)]
mod v54_tests;
