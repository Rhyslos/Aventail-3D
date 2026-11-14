//! Where the manual's links and pictures lead in the guide: a page or a
//! heading becomes an anchor in the one page, a picture a file in the
//! `pictures` folder beside it, anything else in the repository its page
//! on GitHub. A link that leads nowhere is a problem, so the guide never
//! goes stale silently.

use crate::Problem;
use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};

/// The picture types the guide carries along.
const PICTURES: [&str; 6] = ["png", "jpg", "jpeg", "gif", "svg", "webp"];

/// One page's anchors.
#[derive(Debug, Default)]
pub(crate) struct PageIds {
    /// The page's own anchor.
    pub(crate) id: String,
    /// Each heading's GitHub anchor and its anchor in the guide.
    pub(crate) anchors: HashMap<String, String>,
}

/// Everything links are resolved against, and what was found on the way.
pub(crate) struct Site<'a> {
    /// The manual's folder.
    pub(crate) manual: &'a Path,
    /// The repository's root folder.
    pub(crate) root: &'a Path,
    /// The repository's address, for links to files outside the manual.
    pub(crate) repository: &'a str,
    /// Every page by file stem; `SUMMARY` is the welcome.
    pub(crate) pages: &'a HashMap<String, PageIds>,
    /// The pictures to copy: name in `pictures/` and source file.
    pub(crate) pictures: BTreeMap<String, PathBuf>,
    /// What leads nowhere.
    pub(crate) problems: Vec<Problem>,
}

impl Site<'_> {
    fn problem(&mut self, stem: &str, message: String) {
        self.problems.push(Problem {
            file: format!("{stem}.md"),
            message,
        });
    }

    /// The guide's anchor for a heading of a page.
    pub(crate) fn anchor(&self, stem: &str, slug: &str) -> String {
        self.pages
            .get(stem)
            .and_then(|p| p.anchors.get(slug))
            .cloned()
            .unwrap_or_else(|| format!("{stem}-{slug}"))
    }

    /// Where a link written on page `stem` leads in the guide.
    pub(crate) fn link(&mut self, stem: &str, dest: &str) -> String {
        if dest.is_empty() || has_scheme(dest) {
            return dest.to_string();
        }
        let (path, fragment) = match dest.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (dest, None),
        };
        if path.is_empty() {
            return self.page_link(stem, stem, fragment, dest);
        }
        let Some(target) = normalise(&self.manual.join(path)) else {
            self.problem(
                stem,
                format!("the link `{dest}` leads outside the repository"),
            );
            return dest.to_string();
        };
        let in_manual = target.parent() == Some(self.manual);
        let is_markdown = target.extension().is_some_and(|e| e == "md");
        if in_manual && is_markdown {
            let page = target
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if self.pages.contains_key(&page) {
                return self.page_link(stem, &page, fragment, dest);
            }
        }
        if is_picture(&target) {
            return self.picture(stem, dest).0;
        }
        self.online(stem, &target, fragment, dest)
    }

    fn page_link(&mut self, stem: &str, page: &str, fragment: Option<&str>, dest: &str) -> String {
        let Some(ids) = self.pages.get(page) else {
            self.problem(
                stem,
                format!("the link `{dest}` leads to a page not in the guide"),
            );
            return dest.to_string();
        };
        match fragment {
            None | Some("") => format!("#{}", ids.id),
            Some(f) => match ids.anchors.get(f) {
                Some(id) => format!("#{id}"),
                None => {
                    let id = ids.id.clone();
                    self.problem(
                        stem,
                        format!("the link `{dest}` names a heading `#{f}` that isn't on that page"),
                    );
                    format!("#{id}")
                }
            },
        }
    }

    /// A file in the repository outside the manual: its page on GitHub.
    fn online(&mut self, stem: &str, target: &Path, fragment: Option<&str>, dest: &str) -> String {
        if !target.exists() {
            self.problem(
                stem,
                format!("the link `{dest}` leads to a file that doesn't exist"),
            );
            return dest.to_string();
        }
        let Ok(relative) = target.strip_prefix(self.root) else {
            self.problem(
                stem,
                format!("the link `{dest}` leads outside the repository"),
            );
            return dest.to_string();
        };
        let kind = if target.is_dir() { "tree" } else { "blob" };
        let path: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let mut url = format!(
            "{}/{kind}/main/{}",
            self.repository.trim_end_matches('/'),
            path.join("/")
        );
        if let Some(f) = fragment {
            url.push('#');
            url.push_str(f);
        }
        url
    }

    /// A picture's address in the guide and its size, if known. The
    /// picture is copied into `pictures/` when the guide is written.
    pub(crate) fn picture(&mut self, stem: &str, dest: &str) -> (String, Option<(u32, u32)>) {
        if has_scheme(dest) {
            return (dest.to_string(), None);
        }
        let path = dest.split(['#', '?']).next().unwrap_or_default();
        let Some(source) = normalise(&self.manual.join(path)) else {
            self.problem(
                stem,
                format!("the picture `{dest}` is outside the repository"),
            );
            return (dest.to_string(), None);
        };
        if !source.is_file() {
            self.problem(stem, format!("the picture `{dest}` doesn't exist"));
            return (dest.to_string(), None);
        }
        if !is_picture(&source) {
            self.problem(stem, format!("`{dest}` isn't a picture the guide can show"));
            return (dest.to_string(), None);
        }
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match self.pictures.get(&name) {
            Some(other) if *other != source => {
                self.problem(
                    stem,
                    format!("two different pictures are both called `{name}`; rename one"),
                );
            }
            _ => {
                self.pictures.insert(name.clone(), source.clone());
            }
        }
        (format!("pictures/{name}"), png_size(&source))
    }
}

fn has_scheme(dest: &str) -> bool {
    dest.contains("://") || dest.starts_with("mailto:") || dest.starts_with("data:")
}

fn is_picture(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| PICTURES.contains(&e.as_str()))
}

/// The path with `.` and `..` worked out, without asking the disk;
/// `None` when `..` climbs above its start.
pub(crate) fn normalise(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    Some(out)
}

/// A PNG's width and height, read from its header, so the page keeps
/// its place while pictures load.
fn png_size(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut head = [0u8; 24];
    std::fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    if &head[..8] != b"\x89PNG\r\n\x1a\n" || &head[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([head[16], head[17], head[18], head[19]]);
    let h = u32::from_be_bytes([head[20], head[21], head[22], head[23]]);
    (w > 0 && h > 0 && w < 100_000 && h < 100_000).then_some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_works_out_dots() {
        assert_eq!(
            normalise(Path::new("/a/docs/manual/../screenshots/x.png")),
            Some(PathBuf::from("/a/docs/screenshots/x.png"))
        );
        assert_eq!(normalise(Path::new("a/./b")), Some(PathBuf::from("a/b")));
        assert_eq!(normalise(Path::new("a/../..")), None);
    }

    #[test]
    fn schemes_are_left_alone() {
        assert!(has_scheme("https://example.com"));
        assert!(has_scheme("mailto:a@b.c"));
        assert!(!has_scheme("bevel.md#settings"));
    }

    #[test]
    fn picture_types() {
        assert!(is_picture(Path::new("a.PNG")));
        assert!(is_picture(Path::new("a.svg")));
        assert!(!is_picture(Path::new("a.md")));
        assert!(!is_picture(Path::new("a")));
    }
}
