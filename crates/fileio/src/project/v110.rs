//! Version 110 of the project file ([decision 0292](../../../../docs/decisions/0292-library-browser.md)):
//! version 109's, where a graph (`v95::GraphV95::tags`) and one of your
//! own boxes (`v98::OwnBoxV98::tags`) may keep tags, the words the
//! library browser finds them by. Each is stored only when there are
//! some.
//!
//! Tags are only words to search by, so reading tidies them rather than
//! refusing a file (`matgraph::tags::tidy_tags`): too many, too long or
//! repeated ones are dropped. An older app would drop them without a
//! word, so the version goes up; files before version 110 hold no tags.

use crate::limits::Bounded;

/// Tags as stored.
pub(crate) type TagsV110 = Bounded<String>;

/// Whether there are no tags to store.
pub(crate) fn no_tags(tags: &TagsV110) -> bool {
    tags.is_empty()
}

/// `tags` as stored.
pub(crate) fn tags_of(tags: &[String]) -> TagsV110 {
    tags.iter().cloned().collect()
}

/// Whether a graph or any own box inside it keeps tags (graph and box
/// files then need format 6).
pub(crate) fn holds_tags(graph: &matgraph::Graph) -> bool {
    !graph.tags().is_empty()
}
