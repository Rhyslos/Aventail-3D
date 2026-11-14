//! Tests of the whole build on small made-up manuals.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A made-up repository with a `docs/manual` folder, removed afterwards.
struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(files: &[(&str, &[u8])]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "user-guide-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Repo { root }
    }

    fn manual(&self) -> PathBuf {
        self.root.join("docs/manual")
    }

    fn build(&self) -> Result<Guide, Vec<Problem>> {
        build(&self.manual())
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A tiny PNG header: 4 by 3 pixels (enough for the size to be read).
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\0\x04\0\0\0\x03rest";

const SUMMARY: &str = "# Test Manual\n\nRead [the concepts](concepts.md) first.\n\n\
    ### The short version\n1. Press **B**.\n\n\
    ## Basics\n- [Concepts: the ideas](concepts.md)\n- [Bevel: round edges](bevel.md)\n\n\
    ## Reference\n- [What's new](../../CHANGELOG.md)\n\n---\n\nFor contributors.\n";

const CONCEPTS: &str = "# Concepts\n\nPlain words.\n\n## Mesh basics\n\n**Edge**: a line. See [Bevel](bevel.md#settings).\n";

const BEVEL: &str = "# Bevel\n\n> Rounds edges off.\n\n\
    ![A bevelled cube](../screenshots/cube.png)\n\n\
    ## How to use it\n\n1. Press **2** for the Edges mode.\n2. Press **V** and drag. Hold **Shift** for finer steps; **Ctrl + Z** undoes.\n\n\
    ### Rounded or hollow\n\nPick the **X**, **Y** or **Z** axis. A [chamfer](concepts.md#mesh-basics) is flat; see [below](#settings).\n\n\
    ## Settings\n\n| Setting | Shortcut |\n|---|---|\n| Segments | **S** |\n\n\
    ## Settings\n\nAgain, with `Alt + V`. **Tip:** not a callout here.\n\n\
    **Tip:** keep bevels small.\n\nSee [decision 1](../decisions/0001-x.md) and [the web](https://example.com).\n";

fn good_files() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("docs/manual/SUMMARY.md", SUMMARY.as_bytes()),
        ("docs/manual/_template.md", b"# Template\n"),
        ("docs/manual/concepts.md", CONCEPTS.as_bytes()),
        ("docs/manual/bevel.md", BEVEL.as_bytes()),
        ("docs/screenshots/cube.png", PNG),
        ("docs/decisions/0001-x.md", b"# One\n"),
        ("CHANGELOG.md", b"# Changes\n"),
    ]
}

fn good() -> (Repo, Guide) {
    let repo = Repo::new(&good_files());
    let guide = repo.build().unwrap_or_else(|p| panic!("{p:#?}"));
    (repo, guide)
}

#[test]
fn chapters_pages_and_parts_are_numbered_sections() {
    let (_repo, guide) = good();
    let html = &guide.html;
    assert!(html.contains("<title>Test Manual</title>"));
    assert!(html.contains("<section class=\"chapter\" id=\"welcome\">"));
    assert!(html.contains("<section class=\"chapter\" id=\"chapter-basics\">\n<h2>Basics</h2>"));
    assert!(html.contains("<section class=\"topic\" id=\"bevel\">\n<h3>Bevel</h3>"));
    assert!(
        html.contains(
            "<section class=\"part\" id=\"bevel-how-to-use-it\">\n<h4>How to use it</h4>"
        )
    );
    assert!(html.contains("<h5 id=\"bevel-rounded-or-hollow\">Rounded or hollow</h5>"));
    // Concepts comes before Bevel, as in the contents.
    assert!(html.find("id=\"concepts\"") < html.find("id=\"bevel\""));
}

#[test]
fn a_group_with_only_outside_links_goes_to_the_footer() {
    let (_repo, guide) = good();
    assert!(!guide.html.contains("chapter-reference"));
    let footer = &guide.html[guide.html.find("<footer>").unwrap()..];
    assert!(footer.contains("/blob/main/CHANGELOG.md\">What&#39;s new</a>"));
}

#[test]
fn the_contents_note_for_contributors_stays_out() {
    let (_repo, guide) = good();
    assert!(!guide.html.contains("For contributors"));
}

#[test]
fn links_become_anchors_in_the_page() {
    let (_repo, guide) = good();
    let html = &guide.html;
    assert!(html.contains("<a href=\"#concepts\">the concepts</a>"));
    assert!(html.contains("<a href=\"#bevel-settings\">Bevel</a>"));
    assert!(html.contains("<a href=\"#concepts-mesh-basics\">chamfer</a>"));
    assert!(html.contains("<a href=\"#bevel-settings\">below</a>"));
    assert!(html.contains("<a href=\"https://example.com\">the web</a>"));
    assert!(html.contains("/blob/main/docs/decisions/0001-x.md\">decision 1</a>"));
}

#[test]
fn repeated_headings_get_their_own_anchors() {
    let (_repo, guide) = good();
    assert!(guide.html.contains("id=\"bevel-settings\""));
    assert!(guide.html.contains("id=\"bevel-settings-1\""));
}

#[test]
fn pictures_become_figures_and_are_carried_along() {
    let (repo, guide) = good();
    assert!(guide.html.contains(
        "<figure><img loading=\"lazy\" src=\"pictures/cube.png\" width=\"4\" height=\"3\" \
         alt=\"A bevelled cube\"><figcaption>A bevelled cube</figcaption></figure>"
    ));
    assert_eq!(guide.pictures.len(), 1);
    let out = repo.root.join("out");
    write(&guide, &out).unwrap();
    assert!(out.join("index.html").is_file());
    assert_eq!(std::fs::read(out.join("pictures/cube.png")).unwrap(), PNG);
}

#[test]
fn the_summary_line_is_the_lead_and_tips_are_callouts() {
    let (_repo, guide) = good();
    assert!(
        guide
            .html
            .contains("<div class=\"lead\">\n<p>Rounds edges off.</p>\n</div>")
    );
    assert!(
        guide
            .html
            .contains("<div class=\"tip\"><p>keep bevels small.</p></div>")
    );
    // Only a paragraph that starts with it.
    assert!(
        guide
            .html
            .contains("<strong>Tip:</strong> not a callout here.")
    );
    assert!(guide.html.contains("<p class=\"lead\">Read"));
}

#[test]
fn keys_are_drawn_as_keys_and_axes_are_not() {
    let (_repo, guide) = good();
    let html = &guide.html;
    assert!(html.contains("Press <kbd>2</kbd> for the Edges mode"));
    assert!(html.contains("Press <kbd>V</kbd> and drag"));
    assert!(html.contains("Hold <kbd>Shift</kbd>"));
    assert!(html.contains("<kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes"));
    assert!(html.contains("<kbd>Alt</kbd>+<kbd>V</kbd>"));
    assert!(html.contains("<strong>X</strong>, <strong>Y</strong> or <strong>Z</strong>"));
    assert!(html.contains("<td><kbd>S</kbd></td>"), "a Shortcut column");
    assert!(html.contains("Press <kbd>B</kbd>."), "in the welcome too");
}

#[test]
fn steps_are_numbered_lists() {
    let (_repo, guide) = good();
    assert!(guide.html.contains("<ol class=\"steps\">"));
}

#[test]
fn the_name_comes_from_branding() {
    let (_repo, guide) = good();
    assert!(guide.html.contains(&format!(
        "<span class=\"word\">{}</span>",
        branding::APP_NAME
    )));
    assert!(guide.html.contains(branding::MANUAL_URL));
}

fn problems_with(change: impl FnOnce(&mut Vec<(&'static str, &'static [u8])>)) -> Vec<Problem> {
    let mut files = good_files();
    change(&mut files);
    let repo = Repo::new(&files);
    repo.build().expect_err("should fail")
}

fn set(files: &mut [(&'static str, &'static [u8])], path: &str, text: &'static str) {
    for f in files.iter_mut() {
        if f.0 == path {
            f.1 = text.as_bytes();
        }
    }
}

fn says(problems: &[Problem], file: &str, words: &str) -> bool {
    problems
        .iter()
        .any(|p| p.file == file && p.message.contains(words))
}

#[test]
fn a_missing_page_is_a_problem() {
    let p = problems_with(|f| f.retain(|(path, _)| *path != "docs/manual/bevel.md"));
    assert!(says(&p, "SUMMARY.md", "bevel.md"), "{p:?}");
}

#[test]
fn a_page_not_in_the_contents_is_a_problem() {
    let p = problems_with(|f| f.push(("docs/manual/orphan.md", b"# Orphan\n")));
    assert!(says(&p, "orphan.md", "isn't listed"), "{p:?}");
}

#[test]
fn a_link_to_a_missing_page_is_a_problem() {
    let p = problems_with(|f| set(f, "docs/manual/concepts.md", "# C\n\n[x](nowhere.md)\n"));
    assert!(says(&p, "concepts.md", "nowhere.md"), "{p:?}");
}

#[test]
fn a_link_to_a_missing_heading_is_a_problem() {
    let p = problems_with(|f| set(f, "docs/manual/concepts.md", "# C\n\n[x](bevel.md#nope)\n"));
    assert!(says(&p, "concepts.md", "#nope"), "{p:?}");
    let p = problems_with(|f| set(f, "docs/manual/concepts.md", "# C\n\n[x](#nope)\n"));
    assert!(says(&p, "concepts.md", "#nope"), "{p:?}");
}

#[test]
fn a_missing_picture_is_a_problem() {
    let p = problems_with(|f| f.retain(|(path, _)| !path.ends_with(".png")));
    assert!(says(&p, "bevel.md", "cube.png"), "{p:?}");
}

#[test]
fn links_outside_the_repository_are_problems() {
    let p = problems_with(|f| {
        set(
            f,
            "docs/manual/concepts.md",
            "# C\n\n[x](../../../../../../../../../../../x.md)\n",
        )
    });
    assert!(says(&p, "concepts.md", "outside"), "{p:?}");
}

#[test]
fn a_missing_file_outside_the_manual_is_a_problem() {
    let p = problems_with(|f| f.retain(|(path, _)| *path != "CHANGELOG.md"));
    assert!(says(&p, "SUMMARY.md", "CHANGELOG.md"), "{p:?}");
}

#[test]
fn a_page_listed_twice_is_a_problem() {
    let p = problems_with(|f| {
        set(
            f,
            "docs/manual/SUMMARY.md",
            "# M\n## A\n- [C](concepts.md)\n- [B](bevel.md)\n- [C again](concepts.md)\n",
        )
    });
    assert!(says(&p, "SUMMARY.md", "listed twice"), "{p:?}");
}

#[test]
fn a_page_without_a_title_is_a_problem() {
    let p = problems_with(|f| set(f, "docs/manual/concepts.md", "No title.\n"));
    assert!(says(&p, "concepts.md", "Title"), "{p:?}");
}

#[test]
fn no_contents_is_a_problem() {
    let p = problems_with(|f| f.retain(|(path, _)| !path.ends_with("SUMMARY.md")));
    assert!(says(&p, "SUMMARY.md", "doesn't exist"), "{p:?}");
    let p = problems_with(|f| set(f, "docs/manual/SUMMARY.md", "Nothing here.\n"));
    assert!(says(&p, "SUMMARY.md", "no `# Title`"), "{p:?}");
    assert!(says(&p, "SUMMARY.md", "no pages"), "{p:?}");
}

#[test]
fn every_problem_is_listed_at_once() {
    let p = problems_with(|f| {
        set(
            f,
            "docs/manual/concepts.md",
            "# C\n\n[a](a.md) [b](b.md) ![c](c.png)\n\n## Mesh basics\n",
        )
    });
    assert_eq!(p.len(), 3, "{p:?}");
}

#[test]
fn odd_markdown_is_drawn_safely() {
    let mut files = good_files();
    set(
        &mut files,
        "docs/manual/concepts.md",
        "# C <b>bold</b>\n\n> [!WARNING]\n> Careful.\n\n## \n\n## !!!\n\n<div>raw</div>\n\n\
         ```\ncode **B**\n```\n\n- **Tip:** in a list\n\n3. starts at three\n\n\
         Text with \"quotes\" & <angles>.\n\n## Mesh basics\n",
    );
    let repo = Repo::new(&files);
    let guide = repo.build().unwrap_or_else(|p| panic!("{p:#?}"));
    let html = &guide.html;
    assert!(html.contains("<div class=\"warn\">"));
    assert!(html.contains("id=\"concepts-part\""));
    assert!(html.contains("<ol class=\"steps\" start=\"3\" style=\"counter-reset: step 2\">"));
    assert!(
        html.contains("\"quotes\" &amp; <angles>"),
        "text is escaped, inline HTML kept"
    );
    assert!(html.contains("code **B**"), "code blocks are left alone");
}

/// The theme's colours, as the app draws them, are the guide's dark
/// colours, so the guide keeps looking like the app.
#[test]
fn the_dark_colours_are_the_apps() {
    let theme = include_str!("../../../crates/ui/src/theme.rs");
    let css = include_str!("guide.css");
    let hex = |name: &str| -> String {
        let line = theme
            .lines()
            .find(|l| l.contains(&format!("pub const {name}: Color32 = Color32::from_rgb(")))
            .unwrap_or_else(|| panic!("{name} not in theme.rs"));
        let inside = &line[line.find("from_rgb(").unwrap() + 9..line.rfind(')').unwrap()];
        let parts: Vec<String> = inside
            .split(',')
            .map(|p| p.trim().trim_start_matches("0x").to_lowercase())
            .collect();
        format!("#{}", parts.join(""))
    };
    for (name, token) in [
        ("BACKGROUND", "--bg"),
        ("PANEL", "--surface"),
        ("TEXT", "--text2"),
        ("TEXT_STRONG", "--text"),
        ("TEXT_WEAK", "--muted"),
    ] {
        let want = format!("{token}: {};", hex(name));
        assert!(css.contains(&want), "{want}");
    }
    let red = theme
        .lines()
        .find(|l| l.contains("Self::Red => (["))
        .expect("the red accent");
    let colours: Vec<String> = red
        .split('[')
        .skip(1)
        .map(|c| {
            let c = &c[..c.find(']').unwrap()];
            let parts: Vec<String> = c
                .split(',')
                .map(|p| p.trim().trim_start_matches("0x").to_lowercase())
                .collect();
            format!("#{}", parts.join(""))
        })
        .collect();
    assert!(
        css.contains(&format!("--accent: {};", colours[1])),
        "dark accent"
    );
    assert!(
        css.contains(&format!("--accent: {};", colours[0])),
        "light accent"
    );
}

/// Changed and cut-off Markdown never makes the tool panic: it builds, or
/// lists problems.
#[test]
fn fuzz_changed_pages_never_panic() {
    let pieces: [&str; 24] = [
        "# ",
        "## ",
        "### ",
        "> ",
        "- ",
        "1. ",
        "**",
        "*",
        "`",
        "[",
        "](",
        ")",
        "![",
        "\n",
        "\n\n",
        "| a | b |\n|---|---|\n",
        "Ctrl + ",
        "press ",
        "bevel.md#",
        "#",
        "<b>",
        "Tip:",
        "concepts.md",
        "../screenshots/cube.png",
    ];
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let repo = Repo::new(&good_files());
    let page = repo.manual().join("bevel.md");
    for round in 0..300 {
        let mut text = String::new();
        if round % 3 == 0 {
            // A real page, changed in a few places.
            let mut bytes = BEVEL.as_bytes().to_vec();
            for _ in 0..4 {
                let at = (next() as usize) % bytes.len();
                bytes[at] = (next() >> 40) as u8;
            }
            bytes.truncate((next() as usize) % bytes.len() + 1);
            text = String::from_utf8_lossy(&bytes).into_owned();
        } else {
            for _ in 0..(next() % 60) {
                text.push_str(pieces[(next() as usize) % pieces.len()]);
                if next() % 3 == 0 {
                    text.push_str("word ");
                }
            }
        }
        std::fs::write(&page, &text).unwrap();
        let _ = repo.build();
        let _ = summary::parse(&text);
    }
}
