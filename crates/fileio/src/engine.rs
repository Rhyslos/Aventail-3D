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
    ExportError, ExportFormat, ExportOptions, ExportPreset, ExportSummary, export_bytes,
};
use scene::Document;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The game engines a folder can belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    /// A Godot project: a folder with `project.godot`.
    Godot,
    /// A Unity project: a folder with `Assets` and `ProjectSettings`.
    Unity,
    /// An Unreal Engine project: a folder with a `.uproject` file.
    UnrealEngine,
}

impl Engine {
    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Godot => "Godot",
            Self::Unity => "Unity",
            Self::UnrealEngine => "Unreal Engine",
        }
    }

    /// The export preset for it.
    pub fn preset(self) -> ExportPreset {
        match self {
            Self::Godot => ExportPreset::Godot,
            Self::Unity => ExportPreset::Unity,
            Self::UnrealEngine => ExportPreset::UnrealEngine,
        }
    }
}

/// How far up from a folder [`detect`] looks for a game project.
const MAX_DEPTH: usize = 32;

/// The game project `folder` is in, if any: the engine, and the project's
/// own folder (`folder` or one above it).
pub fn detect(folder: &Path) -> Option<(Engine, PathBuf)> {
    for dir in folder.ancestors().take(MAX_DEPTH) {
        if dir.join("project.godot").is_file() {
            return Some((Engine::Godot, dir.to_path_buf()));
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
            Self::NoFolder(path) => write!(
                f,
                "the game project folder {} isn't there; choose it again in the Export window",
                path.display()
            ),
            Self::NotOurs(path) => write!(
                f,
                "{} is already there and wasn't written by this app, so it was left alone; \
                 rename it, or the project",
                path.display()
            ),
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
        ..*options
    };
    let format: ExportFormat = settings.format;
    let file_name = format!("{name}.{}", format.extension());
    let path = folder.join(&file_name);
    let files = export_bytes(document, format, &options, generator, &file_name)
        .map_err(TargetError::Export)?;
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
    Ok((path, files.summary))
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
}
