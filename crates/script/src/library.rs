//! Your script library on disk (plan, milestone 3, choice 12): a folder of
//! your own, the same in every project, holding your scripts and the
//! app's description files (`aventail/scene.lsc` and the rest), written
//! there so an editor outside the app (the Lust editor, say) knows every
//! call a script can make.
//!
//! The library is the app's own: scripts themselves reach files only
//! through the permissions you approve (decision 0307). Scripts come in
//! with [`Library::add`] (Install a script…, or a project's script copied
//! in), never over one of yours. Saving goes through
//! `fileio::project::safe_write`, so a script is never left half-written.

use crate::{Language, Names, ScriptFile};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use words::trf;

/// The most scripts listed from the library.
pub const MAX_LISTED: usize = 1_000;

/// The longest name a new script may have, in characters.
pub const MAX_NAME_CHARS: usize = 80;

/// The library's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    folder: PathBuf,
}

impl Library {
    /// The library kept in `folder` (made when first needed).
    pub fn new(folder: PathBuf) -> Self {
        Self { folder }
    }

    /// Its folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Makes the folder, and writes the description files into it when
    /// they're missing or differ (a newer app, or changed by hand).
    pub fn prepare(&self, names: &Names) -> Result<(), String> {
        std::fs::create_dir_all(&self.folder).map_err(|e| {
            trf!(
                "Your script library's folder ({}) couldn't be made ({}).",
                self.folder.display(),
                e
            )
        })?;
        for (path, text) in crate::descriptions(names) {
            let path = self.folder.join(path);
            if std::fs::read(&path).is_ok_and(|now| now == text.as_bytes()) {
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| unwritable(parent, &e))?;
            }
            fileio::project::safe_write(&path, text.as_bytes())
                .map_err(|e| unwritable(&path, &e))?;
        }
        Ok(())
    }

    /// The scripts in the library (its own folder only, not the
    /// description files' folder), by name ignoring case, at most
    /// [`MAX_LISTED`].
    pub fn scripts(&self) -> Result<Vec<PathBuf>, String> {
        let entries = std::fs::read_dir(&self.folder).map_err(|e| {
            trf!(
                "Your script library ({}) couldn't be read ({}).",
                self.folder.display(),
                e
            )
        })?;
        let mut found: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.path())
            .filter(|p| language_of(p).is_some())
            .take(MAX_LISTED)
            .collect();
        found.sort_by_key(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });
        Ok(found)
    }

    /// Makes a new script called `name` in `language`, from the starter,
    /// and gives back its path. Refuses a name a file can't have, or one
    /// already used.
    pub fn create(&self, name: &str, language: Language, names: &Names) -> Result<PathBuf, String> {
        let file = file_name(name, language)?;
        self.prepare(names)?;
        let path = self.folder.join(&file);
        if path.exists() {
            return Err(trf!(
                "There's already a script called {} in your library: choose another name.",
                file
            ));
        }
        save(&path, &starter(language, name.trim(), names))?;
        Ok(path)
    }

    /// Gives the script at `path` the name `name`, keeping its language,
    /// and gives back its new path. Refuses a name a file can't have, one
    /// already used, or a file outside the library.
    pub fn rename(&self, path: &Path, name: &str) -> Result<PathBuf, String> {
        let language = self.own(path)?;
        let file = file_name(name, language)?;
        let to = self.folder.join(&file);
        if to == path {
            return Ok(to);
        }
        let same_but_case = to
            .to_string_lossy()
            .eq_ignore_ascii_case(&path.to_string_lossy());
        if to.exists() && !same_but_case {
            return Err(trf!(
                "There's already a script called {} in your library: choose another name.",
                file
            ));
        }
        std::fs::rename(path, &to).map_err(|e| {
            trf!(
                "{} couldn't be renamed ({}).",
                path.file_name().unwrap_or_default().to_string_lossy(),
                e
            )
        })?;
        Ok(to)
    }

    /// Adds `file` to the library (Install a script…, or a project's
    /// script copied in; decision 0307) under its own name, or with a
    /// number added when that name is taken ("Bevel 2.lsc"), so nothing
    /// of yours is written over. Gives back its path. Refuses a file
    /// whose name isn't a script's or a file can't have, and one too big.
    pub fn add(&self, file: &ScriptFile, names: &Names) -> Result<PathBuf, String> {
        let language = file.language().ok_or_else(|| {
            trf!(
                "{} isn't a script: scripts end in .lsc (LustScript) or .lua (Lua).",
                file.name
            )
        })?;
        if file.text.len() as u64 > crate::MAX_SCRIPT_BYTES {
            return Err(trf!(
                "{} is too big for a script: a script may be at most {} KB.",
                file.name,
                crate::MAX_SCRIPT_BYTES >> 10
            ));
        }
        let stem = file
            .name
            .rsplit_once('.')
            .map_or(file.name.as_str(), |(stem, _)| stem);
        let base = file_name(stem, language)?;
        self.prepare(names)?;
        let mut path = self.folder.join(&base);
        let mut n = 2;
        while path.exists() || self.taken_but_case(&path) {
            if n > MAX_LISTED {
                return Err(trf!(
                    "Your library already has too many scripts called {}.",
                    stem
                ));
            }
            path = self
                .folder
                .join(file_name(&format!("{} {n}", stem.trim()), language)?);
            n += 1;
        }
        save(&path, &file.text)?;
        Ok(path)
    }

    /// Whether a script in the library has `path`'s name in other
    /// letters' case (the same file on Windows and macOS).
    fn taken_but_case(&self, path: &Path) -> bool {
        let wanted = path.file_name().map(|n| n.to_string_lossy().to_lowercase());
        self.scripts().is_ok_and(|found| {
            found
                .iter()
                .any(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()) == wanted)
        })
    }

    /// Deletes the script at `path` from the library.
    pub fn remove(&self, path: &Path) -> Result<(), String> {
        self.own(path)?;
        std::fs::remove_file(path).map_err(|e| {
            trf!(
                "{} couldn't be removed ({}).",
                path.file_name().unwrap_or_default().to_string_lossy(),
                e
            )
        })
    }

    /// The language of the script at `path`, when it's one of the
    /// library's own (directly in its folder).
    fn own(&self, path: &Path) -> Result<Language, String> {
        let language = language_of(path);
        match language {
            Some(language) if path.parent() == Some(self.folder.as_path()) && path.is_file() => {
                Ok(language)
            }
            _ => Err(trf!(
                "{} isn't one of the scripts in your library.",
                path.display()
            )),
        }
    }
}

/// The language a script's path says, from its ending.
pub fn language_of(path: &Path) -> Option<Language> {
    ScriptFile {
        name: path.file_name()?.to_string_lossy().to_string(),
        text: String::new(),
    }
    .language()
}

/// The file name for a script called `name`: the name as typed, with its
/// language's ending. Refuses what a file name can't hold on any system.
pub fn file_name(name: &str, language: Language) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(words::tr("Give the script a name.").to_string());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(trf!(
            "That name is too long: a script's name may be at most {} characters.",
            MAX_NAME_CHARS
        ));
    }
    if let Some(c) = name
        .chars()
        .find(|c| c.is_control() || r#"/\:*?"<>|"#.contains(*c))
    {
        return Err(trf!(
            "A script's name can't hold {}: file names can't. Use letters, numbers, spaces and dashes.",
            if c.is_control() {
                words::tr("a control character").to_string()
            } else {
                format!("{c}")
            }
        ));
    }
    if name.starts_with('.') || name.ends_with('.') {
        return Err(words::tr("A script's name can't start or end with a dot.").to_string());
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"];
    let numbered = ["COM", "LPT"];
    if reserved.contains(&stem.as_str())
        || numbered.iter().any(|n| {
            stem.strip_prefix(n)
                .is_some_and(|d| d.len() == 1 && d.as_bytes()[0].is_ascii_digit())
        })
    {
        return Err(trf!(
            "{} is a name Windows keeps for itself: choose another.",
            name
        ));
    }
    let ending = match language {
        Language::LustScript => "lsc",
        Language::Lua => "lua",
    };
    Ok(format!("{name}.{ending}"))
}

/// A new script's text: a header with its name, and a few lines that
/// say the name of every selected shape, to start from.
pub fn starter(language: Language, name: &str, names: &Names) -> String {
    let api = crate::API_VERSION;
    let folder = &names.folder;
    let name = name.replace(['\n', '\r'], " ");
    match language {
        Language::LustScript => format!(
            r#"-- name: {name}
-- about: What it does, in a few words.
-- api: {api}

fetch "{folder}/scene":[Scene, Shape];
fetch "{folder}/script":Script;

-- Say the name of every selected shape.
loop (Shape shape in Scene:selected()) {{
    Script:say(shape:name());
}}
end
"#
        ),
        Language::Lua => format!(
            r#"-- name: {name}
-- about: What it does, in a few words.
-- api: {api}

-- Say the name of every selected shape.
for _, shape in ipairs({folder}.Scene:selected()) do
    print(shape:name())
end
"#
        ),
    }
}

/// Writes `text` to the script at `path`, whole or not at all.
pub fn save(path: &Path, text: &str) -> Result<(), String> {
    fileio::project::safe_write(path, text.as_bytes()).map_err(|e| unwritable(path, &e))
}

/// When a file last changed, and how long it is: a change made outside
/// the app shows as a different stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

/// The stamp of the file at `path`, if it's there.
pub fn stamp(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(Stamp {
        modified: meta.modified().ok(),
        len: meta.len(),
    })
}

fn unwritable(path: &Path, why: &std::io::Error) -> String {
    trf!("{} couldn't be written ({}).", path.display(), why)
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod tests;
