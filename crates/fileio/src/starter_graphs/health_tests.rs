//! The material health check (decision 0290) on every starter graph:
//! starters are where people learn what a good material looks like, so
//! they pass it, apart from the warnings listed here on purpose.

use super::*;
use matgraph::health::{Concern, check};
use matgraph::{Evaluator, Registry, Size};

/// What the check says about a starter on purpose, and why.
fn expected(key: &str) -> Vec<Concern> {
    match key {
        // A weave is fine detail by nature: the check rightly says it
        // fades within a few metres (the manual uses it as the example).
        "fabric" => vec![Concern::Mush],
        // Painted darker than real metal for a hand-painted look; the
        // check says what the Realistic look and engines make of it.
        "metal_plate" => vec![Concern::DarkMetal],
        // A new project has no trim rows (the fix makes them), and the
        // bolts and copper pipe are painted darker than real metal.
        "trim_sheet" => vec![Concern::NoTrimRows, Concern::DarkMetal],
        _ => Vec::new(),
    }
}

#[test]
fn every_starter_is_healthy_but_for_what_it_is_meant_to_show() {
    let registry = Registry::standard();
    for starter in &STARTERS {
        let file = starter.load().unwrap();
        let evaluation = Evaluator::new().run(registry, &file.graph, Size::new(256).unwrap(), &[]);
        let warnings = check(registry, &file.graph, &evaluation, &[]);
        let concerns: Vec<Concern> = warnings.iter().map(|w| w.concern).collect();
        assert_eq!(
            concerns,
            expected(starter.key),
            "{}: {:?}",
            starter.key,
            warnings.iter().map(|w| &w.words).collect::<Vec<_>>()
        );
        // Every warning offers at least one way to fix it.
        assert!(
            warnings.iter().all(|w| !w.remedies.is_empty()),
            "{}",
            starter.key
        );
    }
}

#[test]
fn the_trim_sheet_starter_is_healthy_once_its_rows_are_made() {
    let starter = STARTERS.iter().find(|s| s.key == "trim_sheet").unwrap();
    let file = starter.load().unwrap();
    // The rows Make these the project's trim rows would make: the strips
    // by their own heights (1, 3, 2 and 2 eighths).
    let rows = matgraph::TrimRows::new(
        [(0.0, 0.125), (0.125, 0.5), (0.5, 0.75), (0.75, 1.0)]
            .iter()
            .enumerate()
            .map(|(i, (from, to))| matgraph::TrimBand {
                name: format!("Strip {}", i + 1),
                from: *from,
                to: *to,
            }),
    );
    let registry = Registry::standard().clone().with_trim_rows(rows);
    let evaluation = Evaluator::new().run(&registry, &file.graph, Size::new(256).unwrap(), &[]);
    let concerns: Vec<Concern> = check(&registry, &file.graph, &evaluation, &[])
        .iter()
        .map(|w| w.concern)
        .collect();
    assert_eq!(concerns, vec![Concern::DarkMetal]);
}
