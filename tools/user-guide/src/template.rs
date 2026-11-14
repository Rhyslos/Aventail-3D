//! The guide's one page around the drawn chapters: the top bar, the
//! contents, the welcome, the footer, and the look and behaviour
//! (`guide.css`, `guide.js`).

use crate::escape;

/// The app's logo (decision 0171), shown in the top bar, the title and
/// the browser tab.
const LOGO: &[u8] = include_bytes!("../../../assets/logo.svg");
const CSS: &str = include_str!("guide.css");
const SCRIPT: &str = include_str!("guide.js");

/// A drawn manual page.
pub(crate) struct Section {
    /// Its anchor (the file stem).
    pub(crate) id: String,
    /// Its line in the contents, such as "Bevel: chamfer or round edges".
    pub(crate) summary: String,
    /// Its HTML.
    pub(crate) html: String,
}

/// A drawn chapter.
pub(crate) struct ChapterHtml {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) sections: Vec<Section>,
    /// Links elsewhere listed in the group: words and address.
    pub(crate) online: Vec<(String, String)>,
}

/// Everything the page is made of.
pub(crate) struct Parts<'a> {
    pub(crate) title: &'a str,
    pub(crate) app_name: &'a str,
    pub(crate) online_manual: &'a str,
    pub(crate) welcome: &'a str,
    pub(crate) chapters: &'a [ChapterHtml],
}

/// The whole page.
pub(crate) fn page(parts: &Parts<'_>) -> String {
    let logo = format!("data:image/svg+xml;base64,{}", base64(LOGO));
    let title = escape(parts.title);
    let name = escape(parts.app_name);
    let mut html = String::with_capacity(2 << 20);
    html.push_str(
        &HEAD
            .replace("{title}", &title)
            .replace("{name}", &name)
            .replace("{logo}", &logo),
    );
    html.push_str(&CSS.replace("{logo}", &logo));
    html.push_str(&TOP.replace("{name}", &name));

    html.push_str("<section class=\"chapter\" id=\"welcome\">\n");
    html.push_str(&format!(
        "<h1><span class=\"logo\" aria-hidden=\"true\"></span>{title}</h1>\n"
    ));
    // How to find things goes right under the lead, as the first thing read.
    let after_lead = parts
        .welcome
        .starts_with("<p class=\"lead\">")
        .then(|| parts.welcome.find("</p>\n").map(|at| at + 5))
        .flatten()
        .unwrap_or(0);
    html.push_str(&parts.welcome[..after_lead]);
    html.push_str(FINDING);
    html.push_str(&parts.welcome[after_lead..]);
    html.push_str("</section>\n");

    let mut elsewhere = Vec::new();
    for chapter in parts.chapters {
        if chapter.sections.is_empty() {
            elsewhere.extend(chapter.online.iter().cloned());
            continue;
        }
        chapter_html(&mut html, chapter);
    }

    html.push_str("<footer>\n<p>");
    html.push_str(&format!(
        "{name} is free software under the GNU General Public License, version 3 or later. \
         This manual describes the version it came with; the newest is always \
         <a href=\"{}\">online</a>.",
        escape(parts.online_manual)
    ));
    for (text, href) in &elsewhere {
        html.push_str(&format!(
            " <a href=\"{}\">{}</a>.",
            escape(href),
            escape(text)
        ));
    }
    html.push_str("</p>\n</footer>\n");
    html.push_str(&TAIL.replace("{script}", SCRIPT));
    html
}

fn chapter_html(html: &mut String, chapter: &ChapterHtml) {
    html.push_str(&format!(
        "\n<section class=\"chapter\" id=\"{}\">\n<h2>{}</h2>\n<ul class=\"pages\">\n",
        escape(&chapter.id),
        escape(&chapter.title)
    ));
    for section in &chapter.sections {
        let (name, about) = match section.summary.split_once(": ") {
            Some((n, a)) => (n, Some(a)),
            None => (section.summary.as_str(), None),
        };
        html.push_str(&format!(
            "<li><a href=\"#{}\">{}</a>",
            escape(&section.id),
            escape(name)
        ));
        if let Some(about) = about {
            html.push_str(&format!("<span>{}</span>", escape(about)));
        }
        html.push_str("</li>\n");
    }
    for (text, href) in &chapter.online {
        html.push_str(&format!(
            "<li><a href=\"{}\">{}</a><span>online</span></li>\n",
            escape(href),
            escape(text)
        ));
    }
    html.push_str("</ul>\n");
    for section in &chapter.sections {
        html.push_str(&format!(
            "\n<section class=\"topic\" id=\"{}\">\n",
            escape(&section.id)
        ));
        html.push_str(&section.html);
        html.push_str("</section>\n");
    }
    html.push_str("</section>\n");
}

/// Standard base64, for the logo.
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18u32, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(TABLE[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

const HEAD: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="How to use {name}: every tool, panel and key, step by step.">
<link rel="icon" type="image/svg+xml" href="{logo}">
<script>
/* The chosen theme is set before anything is drawn, so the page never
   flashes the other one. Storage can be refused (file://, private windows). */
(function () {
  try {
    var t = window.localStorage.getItem("guide-theme");
    if (t === "light" || t === "dark") { document.documentElement.setAttribute("data-theme", t); }
  } catch (e) { /* follow the system */ }
}());
</script>
<style>
"#;

const TOP: &str = r##"</style>
</head>
<body>
<header class="top">
  <button class="icon-button" id="toc-button" type="button" aria-controls="toc" aria-expanded="false" title="Contents">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h10"/></svg>
  </button>
  <a class="brand" href="#welcome" title="To the top">
    <span class="logo" role="img" aria-label="{name}"></span>
    <span class="word">{name}</span> <small>User manual</small>
  </a>
  <div class="crumb" id="crumb" aria-label="You are reading">
    <a class="chap" id="crumb-chapter" href="#welcome">Welcome</a>
    <span class="sep hidden" aria-hidden="true">›</span>
    <span class="sub" id="crumb-section"></span>
  </div>
  <div class="search" role="search">
    <svg class="glass" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>
    <input id="guide-search" type="search" placeholder="Search the manual" autocomplete="off" spellcheck="false" aria-label="Search the manual" aria-controls="results" aria-expanded="false">
    <span class="hint"><kbd>/</kbd><kbd>Ctrl K</kbd></span>
    <div class="results" id="results" role="listbox" aria-label="Search results"></div>
  </div>
  <button class="icon-button" id="theme-button" type="button" data-mode="system" title="Theme: Auto (follows the system). Click for Light." aria-label="Theme: Auto (follows the system). Click for Light.">
    <svg class="i-system" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="M12 4a8 8 0 0 0 0 16z" fill="currentColor"/></svg>
    <svg class="i-light" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>
    <svg class="i-dark" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true"><path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"/></svg>
    <span class="label">Auto</span>
  </button>
  <div class="progress" id="progress" aria-hidden="true"></div>
</header>
<div class="page">
<nav class="toc" id="toc" aria-label="Contents"><div class="toc-head"><span>Contents</span><span class="read" id="toc-read">0% read</span></div></nav>
<div class="scrim" id="scrim"></div>
<main id="main">
<div class="guide">
"##;

/// How to find things, under the welcome.
const FINDING: &str = "<p>Use the search box at the top (press <kbd>/</kbd> or \
<kbd>Ctrl</kbd>+<kbd>K</kbd>) to find anything, or pick a chapter on the left. Every \
heading has its own link: point at it and click the <strong>#</strong> beside it to copy \
where it is.</p>\n";

const TAIL: &str = r#"</div>
</main>
</div>
<button class="icon-button to-top" id="to-top" type="button" title="Back to the top">
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 19V5M5 12l7-7 7 7"/></svg>
  <span class="label">Top</span>
</button>
<div class="toast" id="toast" role="status" aria-live="polite"></div>
<div class="lightbox" id="lightbox" role="dialog" aria-modal="true" aria-label="Picture, large" aria-hidden="true">
  <button class="icon-button" type="button" aria-label="Close">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>
  </button>
  <img alt="">
  <p></p>
</div>
<script>
{script}</script>
</body></html>
"#;

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
