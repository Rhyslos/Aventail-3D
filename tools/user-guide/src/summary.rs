//! Reading `SUMMARY.md`, the manual's table of contents: its title, the
//! words before the first group, and each `##` group with its pages.

/// The table of contents.
#[derive(Debug, Default)]
pub(crate) struct Summary {
    /// The `#` line: the guide's title.
    pub(crate) title: String,
    /// The Markdown between the title and the first group: the welcome.
    pub(crate) preamble: String,
    /// The `##` groups, in order.
    pub(crate) groups: Vec<Group>,
}

/// One `##` group: a chapter of the guide.
#[derive(Debug, Default)]
pub(crate) struct Group {
    /// The group's name.
    pub(crate) title: String,
    /// The list's links, in order.
    pub(crate) entries: Vec<Entry>,
}

/// One `- [text](target)` line.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    /// The link's words.
    pub(crate) text: String,
    /// Where it points, as written.
    pub(crate) target: String,
}

/// Reads the table of contents. Lines it doesn't know (notes in italics,
/// blank lines) are skipped; a `---` line after the groups ends it, so
/// notes for contributors below it stay out of the guide.
pub(crate) fn parse(text: &str) -> Summary {
    let mut summary = Summary::default();
    let mut preamble = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("## ") {
            summary.groups.push(Group {
                title: name.trim().to_string(),
                entries: Vec::new(),
            });
            continue;
        }
        let Some(group) = summary.groups.last_mut() else {
            match trimmed.strip_prefix("# ") {
                Some(title) if summary.title.is_empty() => summary.title = title.trim().to_string(),
                _ => preamble.push(line),
            }
            continue;
        };
        if trimmed == "---" {
            break;
        }
        if let Some(entry) = entry(trimmed) {
            group.entries.push(entry);
        }
    }
    summary.preamble = preamble.join("\n").trim().to_string();
    summary
}

/// A list line's link: `- [text](target)`, or `* [text](target)`.
fn entry(line: &str) -> Option<Entry> {
    let rest = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))?
        .trim_start()
        .strip_prefix('[')?;
    let split = rest.find("](")?;
    let text = &rest[..split];
    let after = &rest[split + 2..];
    let close = after.find(')')?;
    let target = after[..close].trim();
    if target.is_empty() {
        return None;
    }
    Some(Entry {
        text: text.trim().to_string(),
        target: target.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# The Manual\n\nHello [there](concepts.md).\n\n\
        ## Basics\n- [Concepts: the ideas](concepts.md)\n- [Tour](interface-tour.md)\n\n\
        ## Modelling\n- [Bevel](bevel.md)\n- *More pages later.*\n\n\
        ## Reference\n- [What's new](../../CHANGELOG.md)\n\n---\n\n\
        For contributors: [template](_template.md).\n- [Not a page](x.md)\n";

    #[test]
    fn reads_title_preamble_and_groups() {
        let s = parse(SAMPLE);
        assert_eq!(s.title, "The Manual");
        assert_eq!(s.preamble, "Hello [there](concepts.md).");
        let names: Vec<_> = s.groups.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(names, ["Basics", "Modelling", "Reference"]);
        assert_eq!(s.groups[0].entries.len(), 2);
        assert_eq!(s.groups[0].entries[0].text, "Concepts: the ideas");
        assert_eq!(s.groups[0].entries[0].target, "concepts.md");
    }

    #[test]
    fn lines_without_links_are_skipped() {
        let s = parse(SAMPLE);
        assert_eq!(s.groups[1].entries.len(), 1);
    }

    #[test]
    fn the_rule_ends_the_contents() {
        let s = parse(SAMPLE);
        assert_eq!(s.groups[2].entries.len(), 1);
        assert_eq!(s.groups[2].entries[0].target, "../../CHANGELOG.md");
    }

    #[test]
    fn empty_and_odd_input_gives_an_empty_summary() {
        let s = parse("");
        assert!(s.title.is_empty() && s.groups.is_empty());
        let s = parse("- [x](\n## A\n- [](a.md\n- [a]()\n");
        assert_eq!(s.groups.len(), 1);
        assert!(s.groups[0].entries.is_empty());
    }
}
