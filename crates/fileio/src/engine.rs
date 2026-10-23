//! Exporting straight into a game project ([decision 0076](../../../docs/decisions/0076-engine-export-paths.md)):
//! finding which engine a folder belongs to, and writing a project's export
//! there, named after the project.
//!
//! A project file can come from anyone, and it names the folder, so writing
//! there is careful: the folder must already exist (nothing is created),
//! the file names are the project's own name with the format's extension,
//! and a file already there is only replaced if this app wrote it (its
//! first line, or a glTF's `generator`, names the app). Anything else is
//! left alone and the export says why.

use crate::export::{
    ExportError, ExportFiles, ExportFormat, ExportOptions, ExportPreset, ExportSummary,
    export_bytes,
};
use scene::Document;
use std::io::Read;
use std::path::{Path, PathBuf};
use words::{text, tr, trf};

/// The game engines a folder can belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    /// A Godot project: a folder with `project.godot`.
    Godot,
    /// A Unity project: a folder with `Assets` and `ProjectSettings`.
    Unity,
    /// An Unreal Engine project: a folder with a `.uproject` file.
    UnrealEngine,
    /// A Solis project: a folder with `project.solis` (decision 0122).
    Solis,
}

impl Engine {
    /// Its name.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Godot => text!("Godot"),
            Self::Unity => text!("Unity"),
            Self::UnrealEngine => text!("Unreal Engine"),
            Self::Solis => text!("Solis"),
        })
    }

    /// The export preset for it.
    pub fn preset(self) -> ExportPreset {
        match self {
            Self::Godot => ExportPreset::Godot,
            Self::Unity => ExportPreset::Unity,
            Self::UnrealEngine => ExportPreset::UnrealEngine,
            Self::Solis => ExportPreset::Solis,
        }
    }
}

/// The file that marks a Solis project's folder, one per game, as
/// `project.godot` marks Godot's.
pub const SOLIS_PROJECT_FILE: &str = "project.solis";

/// How far up from a folder [`detect`] looks for a game project.
const MAX_DEPTH: usize = 32;

/// The game project `folder` is in, if any: the engine, and the project's
/// own folder (`folder` or one above it).
pub fn detect(folder: &Path) -> Option<(Engine, PathBuf)> {
    for dir in folder.ancestors().take(MAX_DEPTH) {
        if dir.join("project.godot").is_file() {
            return Some((Engine::Godot, dir.to_path_buf()));
        }
        if dir.join(SOLIS_PROJECT_FILE).is_file() {
            return Some((Engine::Solis, dir.to_path_buf()));
        }
        if dir.join("Assets").is_dir() && dir.join("ProjectSettings").is_dir() {
            return Some((Engine::Unity, dir.to_path_buf()));
        }
        let unreal = std::fs::read_dir(dir).ok().is_some_and(|entries| {
            entries.flatten().take(10_000).any(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("uproject"))
            })
        });
        if unreal {
            return Some((Engine::UnrealEngine, dir.to_path_buf()));
        }
    }
    None
}

/// Why an export into a game project didn't happen, in plain words.
#[derive(Debug)]
pub enum TargetError {
    /// The folder isn't there (moved, renamed, or on a drive not plugged
    /// in).
    NoFolder(PathBuf),
    /// A file of that name is there, and this app didn't write it.
    NotOurs(PathBuf),
    /// The export itself failed.
    Export(ExportError),
}

impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFolder(path) => f.write_str(&trf!(
                "the game project folder {} isn't there; choose it again in the Export window",
                path.display()
            )),
            Self::NotOurs(path) => f.write_str(&trf!(
                "{} is already there and wasn't written by this app, so it was left alone; \
                 rename it, or the project",
                path.display()
            )),
            Self::Export(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for TargetError {}

/// The most of a file read to see who wrote it.
const LOOK_AT: u64 = 1 << 20;

/// Whether the file at `path` may be replaced by an export from the app
/// called `app_name`: it isn't there, or the app wrote it (an OBJ or MTL
/// file's first line, or a glTF's `asset.generator`, starts with the name).
pub fn may_replace(path: &Path, app_name: &str) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return true;
    };
    if !meta.is_file() {
        return false;
    }
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = Vec::new();
    if file.take(LOOK_AT).read_to_end(&mut head).is_err() {
        return false;
    }
    if head.starts_with(b"glTF") {
        return glb_generator(&head).is_some_and(|g| g.starts_with(app_name));
    }
    let first = head.split(|b| *b == b'\n').next().unwrap_or_default();
    String::from_utf8_lossy(first)
        .trim()
        .strip_prefix('#')
        .is_some_and(|rest| rest.trim_start().starts_with(app_name))
}

/// The `asset.generator` of a binary glTF file's start, if it has one.
fn glb_generator(bytes: &[u8]) -> Option<String> {
    let length = u32::from_le_bytes(bytes.get(12..16)?.try_into().ok()?) as usize;
    if bytes.get(16..20)? != b"JSON" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(bytes.get(20..20 + length)?).ok()?;
    Some(json.get("asset")?.get("generator")?.as_str()?.to_string())
}

/// Exports `document` into `folder` as `name` (the project's name) with
/// `preset`'s settings and `options`' choice of objects and shading.
/// `generator` names the app in the files (it starts with `app_name`).
/// Returns the file written and what it holds.
pub fn export_into(
    folder: &Path,
    name: &str,
    document: &Document,
    preset: ExportPreset,
    options: &ExportOptions,
    generator: &str,
    app_name: &str,
) -> Result<(PathBuf, ExportSummary), TargetError> {
    if !folder.is_dir() {
        return Err(TargetError::NoFolder(folder.to_path_buf()));
    }
    let settings = preset.settings();
    let options = ExportOptions {
        units: settings.units,
        up: settings.up,
        triangulate: settings.triangulate,
        godot_names: settings.godot_names,
        ..*options
    };
    let format: ExportFormat = settings.format;
    let file_name = format!("{name}.{}", format.extension());
    let files = export_bytes(document, format, &options, generator, &file_name)
        .map_err(TargetError::Export)?;
    let path = write_checked(folder, &file_name, &files, app_name)?;
    Ok((path, files.summary))
}

/// Writes `files` into `folder`, the main one as `file_name`, after
/// checking each may be replaced; nothing is written if one may not.
fn write_checked(
    folder: &Path,
    file_name: &str,
    files: &ExportFiles,
    app_name: &str,
) -> Result<PathBuf, TargetError> {
    let path = folder.join(file_name);
    let mut all = vec![(path.clone(), files.main.as_slice())];
    for (beside, bytes) in &files.beside {
        all.push((folder.join(beside), bytes.as_slice()));
    }
    if let Some((taken, _)) = all.iter().find(|(p, _)| !may_replace(p, app_name)) {
        return Err(TargetError::NotOurs(taken.clone()));
    }
    for (p, bytes) in all.iter().rev() {
        crate::project::safe_write(p, bytes)
            .map_err(|e| TargetError::Export(ExportError::Io(e)))?;
    }
    Ok(path)
}

/// A file name (without its extension) for an object called `name`:
/// characters some computers can't have in a file name become `_`, and
/// an empty one is "Object".
pub fn file_stem_for(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            let bad =
                c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|');
            if bad { '_' } else { c }
        })
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if cleaned.is_empty() {
        crate::UNNAMED_IN_FILES.to_string()
    } else {
        cleaned.chars().take(120).collect()
    }
}

/// How one piece of a kit export went: its name, and the file written
/// and what it held, or why not.
pub type PieceOutcome = (String, Result<(PathBuf, ExportSummary), TargetError>);

/// Exports each of `pieces` (with the objects inside it) into `folder` as
/// a file of its own, named after it, its pivot at the file's origin
/// ([decision 0078](../../../docs/decisions/0078-kit-export.md)), in
/// `format` with `options`. Each file is written as [`export_into`]
/// writes: only replacing the app's own files. Pieces of the same name
/// get a number.
pub fn export_kit(
    folder: &Path,
    document: &Document,
    pieces: &[scene::ObjectId],
    format: ExportFormat,
    options: &ExportOptions,
    generator: &str,
    app_name: &str,
) -> Result<Vec<PieceOutcome>, TargetError> {
    if !folder.is_dir() {
        return Err(TargetError::NoFolder(folder.to_path_buf()));
    }
    let options = format.options(options);
    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for root in pieces {
        let Some(object) = document.scene().object(*root) else {
            continue;
        };
        let name = object.name.clone();
        let base = file_stem_for(&name);
        let mut stem = base.clone();
        let mut n = 2;
        while used.iter().any(|u| u.eq_ignore_ascii_case(&stem)) {
            // A file name, the same in every language.
            stem = format!("{} {}", base, n);
            n += 1;
        }
        used.push(stem.clone());
        let file_name = format!("{stem}.{}", format.extension());
        let objects = crate::export::prepare_piece(document, *root, &options);
        let result = crate::export::write_files(&objects, format, &options, generator, &file_name)
            .map_err(TargetError::Export)
            .and_then(|files| {
                write_checked(folder, &file_name, &files, app_name).map(|p| (p, files.summary))
            });
        out.push((name, result));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, ObjectKind, PrimitiveKind};

    /// A new, empty folder for one test.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("engine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cube() -> Document {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        doc
    }

    #[test]
    fn engines_are_found_from_inside_their_projects() {
        let root = folder("detect");
        assert_eq!(detect(&root), None);
        let godot = root.join("game");
        std::fs::create_dir_all(godot.join("models/props")).unwrap();
        std::fs::write(godot.join("project.godot"), "").unwrap();
        assert_eq!(
            detect(&godot.join("models/props")),
            Some((Engine::Godot, godot.clone()))
        );
        let unity = root.join("unity");
        std::fs::create_dir_all(unity.join("Assets/Models")).unwrap();
        std::fs::create_dir_all(unity.join("ProjectSettings")).unwrap();
        assert_eq!(
            detect(&unity.join("Assets/Models")).map(|d| d.0),
            Some(Engine::Unity)
        );
        let unreal = root.join("unreal");
        std::fs::create_dir_all(unreal.join("Content")).unwrap();
        std::fs::write(unreal.join("Shooter.uproject"), "{}").unwrap();
        assert_eq!(
            detect(&unreal.join("Content")).map(|d| d.0),
            Some(Engine::UnrealEngine)
        );
        assert_eq!(Engine::UnrealEngine.preset(), ExportPreset::UnrealEngine);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn solis_projects_are_found_by_their_project_file() {
        let root = folder("solis");
        let game = root.join("game");
        std::fs::create_dir_all(game.join("models/props")).unwrap();
        assert_eq!(detect(&game.join("models/props")), None);
        // A folder named like the file is not a project.
        std::fs::create_dir_all(root.join("fake/project.solis")).unwrap();
        assert_eq!(detect(&root.join("fake")), None);
        // A scene file is not a project file either.
        std::fs::write(game.join("level.solscene"), "").unwrap();
        assert_eq!(detect(&game), None);
        std::fs::write(game.join(SOLIS_PROJECT_FILE), "").unwrap();
        assert_eq!(
            detect(&game.join("models/props")),
            Some((Engine::Solis, game.clone()))
        );
        assert_eq!(Engine::Solis.name(), "Solis");
        assert_eq!(Engine::Solis.preset(), ExportPreset::Solis);
        // The nearest project wins: a Godot project inside a Solis one.
        let inner = game.join("tools/godot-viewer");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("project.godot"), "").unwrap();
        assert_eq!(detect(&inner).map(|d| d.0), Some(Engine::Godot));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn exports_go_in_named_after_the_project_and_replace_only_their_own() {
        let dir = folder("into");
        let doc = cube();
        let options = ExportOptions::default();
        let (path, summary) = export_into(
            &dir,
            "crate",
            &doc,
            ExportPreset::Godot,
            &options,
            "Test App 1.0",
            "Test App",
        )
        .unwrap();
        assert_eq!(path, dir.join("crate.glb"));
        assert_eq!(summary.objects, 1);
        assert!(may_replace(&path, "Test App"));
        assert!(!may_replace(&path, "Other App"));
        // Again: it replaces its own file.
        export_into(
            &dir,
            "crate",
            &doc,
            ExportPreset::Godot,
            &options,
            "Test App 1.1",
            "Test App",
        )
        .unwrap();
        // Someone else's file of that name is left alone.
        std::fs::write(dir.join("barrel.glb"), b"glTF something else").unwrap();
        let refused = export_into(
            &dir,
            "barrel",
            &doc,
            ExportPreset::Godot,
            &options,
            "Test App 1.0",
            "Test App",
        );
        assert!(matches!(refused, Err(TargetError::NotOurs(_))));
        assert_eq!(
            std::fs::read(dir.join("barrel.glb")).unwrap(),
            b"glTF something else"
        );
        // An OBJ's .mtl is checked too.
        std::fs::write(dir.join("box.mtl"), "newmtl Theirs\n").unwrap();
        let refused = export_into(
            &dir,
            "box",
            &doc,
            ExportPreset::SubstancePainter,
            &options,
            "Test App 1.0",
            "Test App",
        );
        assert!(matches!(refused, Err(TargetError::NotOurs(p)) if p.ends_with("box.mtl")));
        assert!(!dir.join("box.obj").exists(), "nothing written");
        // A missing folder isn't made.
        let gone = dir.join("missing");
        let refused = export_into(
            &gone,
            "crate",
            &doc,
            ExportPreset::Godot,
            &options,
            "Test App 1.0",
            "Test App",
        );
        assert!(matches!(refused, Err(TargetError::NoFolder(_))));
        assert!(!gone.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_files_naming_the_app_may_be_replaced() {
        let dir = folder("replace");
        let obj = dir.join("a.obj");
        assert!(may_replace(&obj, "Test App"), "not there yet");
        std::fs::write(&obj, "# Test App 1.0\nv 0 0 0\n").unwrap();
        assert!(may_replace(&obj, "Test App"));
        std::fs::write(&obj, "# Blender 4.2\nv 0 0 0\n").unwrap();
        assert!(!may_replace(&obj, "Test App"));
        std::fs::write(&obj, "").unwrap();
        assert!(!may_replace(&obj, "Test App"));
        // A folder of that name isn't a file to replace.
        std::fs::create_dir_all(dir.join("b.glb")).unwrap();
        assert!(!may_replace(&dir.join("b.glb"), "Test App"));
        // A damaged glTF is left alone.
        std::fs::write(dir.join("c.glb"), b"glTF\x02\0\0\0\xff\xff\xff\xffJSON").unwrap();
        assert!(!may_replace(&dir.join("c.glb"), "Test App"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_names_hold_only_what_every_computer_allows() {
        assert_eq!(file_stem_for("Wall: corner/left"), "Wall_ corner_left");
        assert_eq!(file_stem_for("  ..  "), "Object");
        assert_eq!(file_stem_for("Æblekasse"), "Æblekasse");
        assert_eq!(file_stem_for(&"x".repeat(500)).len(), 120);
    }

    #[test]
    fn each_piece_goes_to_its_own_file_at_the_origin() {
        let dir = folder("kit");
        let mut doc = Document::new();
        for (name, x) in [("Wall", 0.0), ("Wall", 400.0), ("Door/frame", 800.0)] {
            doc.execute(Command::Create(ObjectKind::Primitive(
                PrimitiveKind::Cube.standard(),
            )))
            .unwrap();
            let id = doc.selection()[0];
            doc.execute(Command::Rename {
                id,
                name: name.into(),
            })
            .unwrap();
            doc.execute(Command::SetTransforms(vec![(
                id,
                scene::Transform {
                    position: glam::Vec3::new(x, 0.0, 0.0),
                    ..scene::Transform::default()
                },
            )]))
            .unwrap();
        }
        let pieces: Vec<scene::ObjectId> = doc.scene().children(None).to_vec();
        let results = export_kit(
            &dir,
            &doc,
            &pieces,
            ExportFormat::Glb,
            &ExportOptions::default(),
            "Test App 1.0",
            "Test App",
        )
        .unwrap();
        assert_eq!(results.len(), 3);
        let mut files: Vec<String> = results
            .iter()
            .map(|(_, r)| {
                let (path, _) = r.as_ref().unwrap();
                path.file_name().unwrap().to_string_lossy().into_owned()
            })
            .collect();
        files.sort();
        assert!(files.iter().any(|f| f == "Door_frame.glb"), "{files:?}");
        let walls = files.iter().filter(|f| f.starts_with("Wall")).count();
        assert_eq!(walls, 2, "{files:?}");
        // Each piece's pivot is at the file's origin: no node matrix.
        for (_, result) in &results {
            let bytes = std::fs::read(&result.as_ref().unwrap().0).unwrap();
            let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
            assert!(
                json["nodes"][0].get("matrix").is_none(),
                "{}",
                json["nodes"]
            );
        }
        // Someone else's file is left alone; the rest still go.
        let theirs = dir.join("Door_frame.glb");
        std::fs::write(&theirs, "theirs").unwrap();
        let again = export_kit(
            &dir,
            &doc,
            &pieces,
            ExportFormat::Glb,
            &ExportOptions::default(),
            "Test App 1.0",
            "Test App",
        )
        .unwrap();
        assert_eq!(again.iter().filter(|(_, r)| r.is_ok()).count(), 2);
        assert_eq!(std::fs::read_to_string(&theirs).unwrap(), "theirs");
        assert!(matches!(
            export_kit(
                &dir.join("missing"),
                &doc,
                &pieces,
                ExportFormat::Glb,
                &ExportOptions::default(),
                "T",
                "T"
            ),
            Err(TargetError::NoFolder(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
