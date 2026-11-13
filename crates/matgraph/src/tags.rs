//! Tags on graphs and your own boxes ([decision 0292](../../../docs/decisions/0292-library-browser.md)):
//! short words ("stone", "wall", "hand-painted") the library browser
//! searches and sorts by. A graph or own box keeps them tidy: each in
//! lower case with single spaces, none empty or too long, no two alike,
//! sorted, and no more than [`MAX_TAGS`].
//!
//! Searching ([`matches`]) is forgiving on purpose: every word typed has
//! to appear somewhere in the name or a tag, in any order and any case.

/// The most tags one graph or own box keeps.
pub const MAX_TAGS: usize = 12;

/// The most letters in one tag.
pub const MAX_TAG: usize = 32;

/// `tag` tidied: trimmed, in lower case, runs of spaces made one, and
/// `None` when nothing is left or it is longer than [`MAX_TAG`].
pub fn tidy_tag(tag: &str) -> Option<String> {
    let tidy = tag
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    (!tidy.is_empty() && tidy.chars().count() <= MAX_TAG).then_some(tidy)
}

/// `tags` tidied ([`tidy_tag`]), the ones left over dropped, without
/// repeats, sorted, and at most [`MAX_TAGS`] (the first ones given are
/// kept).
pub fn tidy_tags<'t>(tags: impl IntoIterator<Item = &'t str>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for tag in tags {
        if kept.len() == MAX_TAGS {
            break;
        }
        if let Some(tag) = tidy_tag(tag)
            && !kept.contains(&tag)
        {
            kept.push(tag);
        }
    }
    kept.sort();
    kept
}

/// Whether `tags` are tidy already (what [`tidy_tags`] would give).
pub fn are_tidy(tags: &[String]) -> bool {
    tidy_tags(tags.iter().map(String::as_str)) == tags
}

/// Whether something called `name` with `tags` is found by `query`:
/// every word of the query appears in the name or one of the tags,
/// ignoring case. An empty query finds everything.
pub fn matches(query: &str, name: &str, tags: &[impl AsRef<str>]) -> bool {
    let name = name.to_lowercase();
    let tags: Vec<String> = tags.iter().map(|t| t.as_ref().to_lowercase()).collect();
    query.split_whitespace().all(|word| {
        let word = word.to_lowercase();
        name.contains(&word) || tags.iter().any(|t| t.contains(&word))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_tidied() {
        assert_eq!(tidy_tag("  Hand   Painted "), Some("hand painted".into()));
        assert_eq!(tidy_tag("Stone"), Some("stone".into()));
        assert_eq!(tidy_tag("   "), None);
        assert_eq!(tidy_tag(""), None);
        assert_eq!(tidy_tag(&"a".repeat(MAX_TAG)), Some("a".repeat(MAX_TAG)));
        assert_eq!(tidy_tag(&"a".repeat(MAX_TAG + 1)), None);
        // Letters, not bytes, are counted.
        assert!(tidy_tag(&"é".repeat(MAX_TAG)).is_some());
    }

    #[test]
    fn tags_are_tidied_sorted_and_counted() {
        let tags = tidy_tags(["Wall", "stone", " wall ", "", "Moss"]);
        assert_eq!(tags, ["moss", "stone", "wall"]);
        assert!(are_tidy(&tags));
        assert!(!are_tidy(&["b".into(), "a".into()]));
        assert!(!are_tidy(&["A".into()]));
        let many: Vec<String> = (0..30).map(|i| format!("tag {i:02}")).collect();
        let kept = tidy_tags(many.iter().map(String::as_str));
        assert_eq!(kept.len(), MAX_TAGS);
        assert_eq!(kept[0], "tag 00", "the first ones given are kept");
        assert!(tidy_tags([]).is_empty());
    }

    #[test]
    fn searching_finds_every_word_in_the_name_or_tags() {
        let tags = ["stone".to_string(), "hand painted".to_string()];
        assert!(matches("", "Bricks", &tags));
        assert!(matches("brick", "Bricks", &tags));
        assert!(matches("STONE bri", "Bricks", &tags));
        assert!(matches("painted", "Bricks", &tags));
        assert!(!matches("wood", "Bricks", &tags));
        assert!(!matches("stone wood", "Bricks", &tags));
        let none: [&str; 0] = [];
        assert!(matches("  ", "Bricks", &none));
    }
}
