//! Heading anchors made the way GitHub makes them, so a link such as
//! `bevel.md#segments-and-profile` that works on GitHub finds the same
//! heading in the guide.

use std::collections::HashMap;

/// GitHub's anchor for a heading's text: lower case, letters, digits,
/// `-` and `_` kept, each space a `-`, everything else dropped.
pub(crate) fn github(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.trim().chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

/// Gives each heading of one page its anchor, numbering repeats the way
/// GitHub does: `settings`, then `settings-1`, `settings-2`.
#[derive(Default)]
pub(crate) struct PageSlugs {
    seen: HashMap<String, usize>,
}

impl PageSlugs {
    /// The anchor for the next heading with this text.
    pub(crate) fn next(&mut self, text: &str) -> String {
        let base = github(text);
        let count = self.seen.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 {
            base.clone()
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        // A numbered anchor can equal a later heading's own text
        // ("a-1"): mark it seen too, as GitHub does.
        if slug != base {
            self.seen.entry(slug.clone()).or_insert(1);
        }
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_words_become_dashed_lower_case() {
        assert_eq!(github("When to use it"), "when-to-use-it");
        assert_eq!(
            github("Move, rotate and scale: the gizmo"),
            "move-rotate-and-scale-the-gizmo"
        );
    }

    #[test]
    fn punctuation_goes_and_spaces_stay_as_dashes() {
        assert_eq!(github("Ctrl + C"), "ctrl--c");
        assert_eq!(github("Why is this here?"), "why-is-this-here");
        assert_eq!(github("UVs (and seams)"), "uvs-and-seams");
        assert_eq!(github("snake_case-and-dash"), "snake_case-and-dash");
    }

    #[test]
    fn letters_outside_english_are_kept() {
        assert_eq!(github("Größe"), "größe");
    }

    #[test]
    fn repeats_are_numbered() {
        let mut slugs = PageSlugs::default();
        assert_eq!(slugs.next("Settings"), "settings");
        assert_eq!(slugs.next("Settings"), "settings-1");
        assert_eq!(slugs.next("Settings"), "settings-2");
        assert_eq!(slugs.next("Other"), "other");
    }

    #[test]
    fn empty_text_gives_an_empty_anchor() {
        assert_eq!(github("  "), "");
        assert_eq!(github("!!!"), "");
    }
}
