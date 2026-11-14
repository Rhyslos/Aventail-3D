//! The real manual makes a good guide: every page in SUMMARY.md exists,
//! every page is in it, and every link and picture leads somewhere. This
//! runs with the other tests on every pull request, so the guide shipped
//! with the app never goes stale.

#[test]
fn the_manual_makes_a_good_guide() {
    let guide = match user_guide::build(&user_guide::default_manual()) {
        Ok(guide) => guide,
        Err(problems) => {
            let list: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!(
                "The manual can't be made into the user guide:\n  {}",
                list.join("\n  ")
            );
        }
    };
    assert!(
        guide.html.contains("id=\"concepts\""),
        "Help → Words used in the app opens it"
    );
    assert!(guide.html.len() < 8 << 20, "a reasonable size");
    assert!(!guide.pictures.is_empty());
}
