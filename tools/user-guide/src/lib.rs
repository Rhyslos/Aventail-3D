//! Builds the HTML user guide from the Markdown manual (decision 0297).
//!
//! The manual in `docs/manual` stays the source: writers edit its
//! Markdown pages as before. This tool reads `SUMMARY.md` for the order
//! (each `##` group a numbered chapter, each page a section, each page's
//! headings parts of it) and writes one page, `index.html`, with a
//! `pictures` folder beside it: contents on the left, search, a theme
//! switch and reading progress, in the app's colours. It ships with the
//! app, and Help → User manual opens it.
//!
//! Every page in the contents must exist, every manual page must be in
//! the contents, and every link and picture must lead somewhere; anything
//! else is a [`Problem`], and no guide is made.

mod keys;
mod links;
mod page;
mod slug;
mod summary;
mod template;

// The app's name and addresses come from the app's own branding file, so
// a rename needs no change here (RENAMING.md). Only some of its values
// are used.
#[allow(dead_code, unreachable_pub)]
#[path = "../../../crates/app/src/branding.rs"]
mod branding;

use links::{PageIds, Site};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

/// Pages bigger than this are refused: the manual's longest is far below.
const MAX_PAGE_BYTES: u64 = 8 * 1024 * 1024;

/// Anchors the page itself uses, which no chapter or page may take.
const RESERVED: [&str; 16] = [
    "welcome",
    "main",
    "toc",
    "toc-button",
    "toc-read",
    "scrim",
    "crumb",
    "crumb-chapter",
    "crumb-section",
    "guide-search",
    "results",
    "theme-button",
    "progress",
    "to-top",
    "toast",
    "lightbox",
];

/// Something in the manual that would make the guide wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The manual file it is in, such as `bevel.md`.
    pub file: String,
    /// What is wrong, in plain words.
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// A built guide, ready to write.
#[derive(Debug)]
pub struct Guide {
    /// The whole page, `index.html`.
    pub html: String,
    /// The pictures it shows: their name in `pictures/` and their source.
    pub pictures: BTreeMap<String, PathBuf>,
}

/// The manual's folder, as this tool was built (`docs/manual`).
pub fn default_manual() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual")
}

/// One chapter while it is being put together.
struct Chapter {
    id: String,
    title: String,
    pages: Vec<(String, summary::Entry)>,
    online: Vec<summary::Entry>,
}

/// Builds the guide from the manual in `manual` (a repository's
/// `docs/manual`), or lists every problem found.
pub fn build(manual: &Path) -> Result<Guide, Vec<Problem>> {
    let manual = std::path::absolute(manual)
        .ok()
        .and_then(|p| links::normalise(&p))
        .unwrap_or_else(|| manual.to_path_buf());
    let root = manual
        .parent()
        .and_then(Path::parent)
        .unwrap_or(&manual)
        .to_path_buf();
    let mut problems = Vec::new();
    let summary_text = match read(&manual.join("SUMMARY.md")) {
        Ok(t) => t,
        Err(message) => {
            return Err(vec![Problem {
                file: "SUMMARY.md".into(),
                message,
            }]);
        }
    };
    let summary = summary::parse(&summary_text);
    if summary.title.is_empty() {
        problems.push(summary_problem("it has no `# Title` line"));
    }

    let (chapters, sources) = chapters(&manual, &summary, &mut problems);
    unlisted(&manual, &sources, &mut problems);

    // First every heading's anchor, so links can point at any page.
    let mut pages: HashMap<String, PageIds> = HashMap::new();
    let mut used: HashSet<String> = RESERVED.iter().map(|s| s.to_string()).collect();
    used.extend(chapters.iter().map(|c| c.id.clone()));
    used.extend(sources.keys().cloned());
    let mut order: Vec<&String> = sources.keys().collect();
    order.sort();
    let welcome = ("SUMMARY".to_string(), "welcome".to_string());
    for (stem, id, text) in std::iter::once((&welcome.0, welcome.1.clone(), &summary.preamble))
        .chain(order.into_iter().map(|s| (s, s.clone(), &sources[s])))
    {
        pages.insert(stem.clone(), page_ids(&id, text, &mut used));
    }

    let mut site = Site {
        manual: &manual,
        root: &root,
        repository: env!("CARGO_PKG_REPOSITORY"),
        pages: &pages,
        pictures: BTreeMap::new(),
        problems,
    };
    let welcome_html = page::render("SUMMARY", &summary.preamble, page::Kind::Welcome, &mut site);
    let mut drawn = Vec::new();
    for chapter in &chapters {
        let mut sections = Vec::new();
        for (stem, entry) in &chapter.pages {
            let rendered = page::render(stem, &sources[stem], page::Kind::Page, &mut site);
            if rendered.title.is_none() {
                site.problems.push(Problem {
                    file: format!("{stem}.md"),
                    message: "it has no `# Title` line".into(),
                });
            }
            sections.push(template::Section {
                id: stem.clone(),
                summary: entry.text.clone(),
                html: rendered.html,
            });
        }
        let online = chapter
            .online
            .iter()
            .map(|e| (e.text.clone(), site.link("SUMMARY", &e.target)))
            .collect();
        drawn.push(template::ChapterHtml {
            id: chapter.id.clone(),
            title: chapter.title.clone(),
            sections,
            online,
        });
    }
    if !site.problems.is_empty() {
        return Err(site.problems);
    }
    let html = template::page(&template::Parts {
        title: &summary.title,
        app_name: branding::APP_NAME,
        online_manual: branding::MANUAL_URL,
        welcome: &welcome_html.html,
        chapters: &drawn,
    });
    Ok(Guide {
        html,
        pictures: site.pictures,
    })
}

fn summary_problem(message: &str) -> Problem {
    Problem {
        file: "SUMMARY.md".into(),
        message: message.into(),
    }
}

/// The chapters in the contents, and every page's Markdown by file stem.
fn chapters(
    manual: &Path,
    summary: &summary::Summary,
    problems: &mut Vec<Problem>,
) -> (Vec<Chapter>, HashMap<String, String>) {
    let mut sources = HashMap::new();
    let mut chapters = Vec::new();
    let mut ids = HashSet::new();
    for group in &summary.groups {
        let base = format!("chapter-{}", slug::github(&group.title));
        let mut id = base.clone();
        let mut n = 2;
        while !ids.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }
        let mut chapter = Chapter {
            id,
            title: group.title.clone(),
            pages: Vec::new(),
            online: Vec::new(),
        };
        for entry in &group.entries {
            let Some(stem) = page_stem(&entry.target) else {
                chapter.online.push(entry.clone());
                continue;
            };
            if stem == "SUMMARY" || stem.starts_with('_') {
                problems.push(summary_problem(&format!(
                    "`{}` isn't a page of the manual",
                    entry.target
                )));
                continue;
            }
            if sources.contains_key(&stem) {
                problems.push(summary_problem(&format!(
                    "`{}` is listed twice",
                    entry.target
                )));
                continue;
            }
            match read(&manual.join(&entry.target)) {
                Ok(text) => {
                    sources.insert(stem.clone(), text);
                    chapter.pages.push((stem, entry.clone()));
                }
                Err(message) => problems.push(summary_problem(&format!(
                    "the page `{}` listed under {}: {message}",
                    entry.target, group.title
                ))),
            }
        }
        chapters.push(chapter);
    }
    if chapters.iter().all(|c| c.pages.is_empty()) {
        problems.push(summary_problem("it lists no pages"));
    }
    // A group with no pages of the manual (only links elsewhere) is shown
    // at the foot of the guide instead of as an empty chapter.
    (chapters, sources)
}

/// A manual page's file stem, for a contents link such as `bevel.md`;
/// `None` for links outside the manual folder.
fn page_stem(target: &str) -> Option<String> {
    let name = target.split('#').next().unwrap_or_default();
    let stem = name.strip_suffix(".md")?;
    let ok = !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    ok.then(|| stem.to_string())
}

/// Every manual page not in the contents would be missing from the guide.
fn unlisted(manual: &Path, sources: &HashMap<String, String>, problems: &mut Vec<Problem>) {
    let Ok(entries) = std::fs::read_dir(manual) else {
        return;
    };
    let mut missing: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter_map(|name| name.strip_suffix(".md").map(str::to_string))
        .filter(|stem| stem != "SUMMARY" && !stem.starts_with('_'))
        .filter(|stem| !sources.contains_key(stem))
        .collect();
    missing.sort();
    for stem in missing {
        problems.push(Problem {
            file: format!("{stem}.md"),
            message: "it isn't listed in SUMMARY.md, so it would be missing from the guide".into(),
        });
    }
}

/// A page's anchor and its headings' anchors, each unused elsewhere.
fn page_ids(id: &str, markdown: &str, used: &mut HashSet<String>) -> PageIds {
    let mut slugs = slug::PageSlugs::default();
    let mut anchors = HashMap::new();
    for (_, text) in page::headings(markdown) {
        let slug = slugs.next(&text);
        let base = if slug.is_empty() {
            format!("{id}-part")
        } else {
            format!("{id}-{slug}")
        };
        let mut anchor = base.clone();
        let mut n = 2;
        while !used.insert(anchor.clone()) {
            anchor = format!("{base}-{n}");
            n += 1;
        }
        anchors.insert(slug, anchor);
    }
    for raw in page::raw_ids(markdown) {
        if anchors.contains_key(&raw) {
            continue;
        }
        let base = format!("{id}-{raw}");
        let mut anchor = base.clone();
        let mut n = 2;
        while !used.insert(anchor.clone()) {
            anchor = format!("{base}-{n}");
            n += 1;
        }
        anchors.insert(raw, anchor);
    }
    PageIds {
        id: id.to_string(),
        anchors,
    }
}

fn read(path: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|_| "it doesn't exist".to_string())?;
    if meta.len() > MAX_PAGE_BYTES {
        return Err("it is too big to be a manual page".into());
    }
    std::fs::read_to_string(path).map_err(|e| format!("it can't be read ({e})"))
}

/// Writes the guide into `out`: `index.html` and its `pictures` folder.
pub fn write(guide: &Guide, out: &Path) -> std::io::Result<()> {
    let pictures = out.join("pictures");
    std::fs::create_dir_all(&pictures)?;
    for (name, source) in &guide.pictures {
        std::fs::copy(source, pictures.join(name))?;
    }
    std::fs::write(out.join("index.html"), &guide.html)
}

/// Text made safe to put in HTML.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests;
