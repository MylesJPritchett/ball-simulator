//! Video metadata: templates from the Scenario filled in with resolved values,
//! and Scenario hashtags merged with the shared hashtag pool.

use std::collections::BTreeMap;

use ball_simulator::meta::{self, VideoMeta};
use ball_simulator::scenario::Scenario;
use ball_simulator::sim;

mod support;
use support::MINIMAL;

fn with_meta(meta_table: &str) -> Scenario {
    let toml = MINIMAL
        .replace("time_cap = 5.0", "time_cap = 5.0\npopulation_cap = 50")
        .replace("speed = 900.0", "speed = { min = 600, max = 1200 }");
    support::scenario(&format!("{toml}\n[meta]\n{meta_table}"))
}

#[test]
fn video_meta_fills_scenario_templates_with_this_seeds_values() {
    let scenario = with_meta(
        r#"
title = "Can it reach {population_cap}? {ball.speed} px/s"
description = "{boundary_hits} Boundary Hits in {duration} s"
hashtags = ["bounce{population_cap}", "satisfying"]
"#,
    );
    let record = sim::run(&scenario, 3);

    let meta = VideoMeta::new(&scenario, &record);

    let speed = meta.parameters["ball.speed"].as_f64();
    assert!(
        (600.0..=1200.0).contains(&speed) && speed != 600.0,
        "{speed}"
    );
    assert_eq!(meta.title, format!("Can it reach 50? {speed} px/s"));
    // 5 s of action plus the 1.5 s ending hold, rounded.
    assert_eq!(
        meta.description,
        format!("{} Boundary Hits in 7 s", record.boundary_hits.len())
    );
    assert_eq!(meta.hashtags[..2], ["#bounce50", "#satisfying"]);
    for shared in meta::SHARED_HASHTAGS {
        assert!(meta.hashtags.contains(&format!("#{shared}")), "{shared}");
    }
}

#[test]
fn without_templates_the_title_is_the_hook_title_and_tags_are_the_shared_pool() {
    let scenario = support::scenario(MINIMAL);
    let meta = VideoMeta::new(&scenario, &sim::run(&scenario, 1));

    assert_eq!(meta.title, "Minimal");
    assert_eq!(meta.description, "Minimal");
    assert_eq!(meta.hashtags.len(), meta::SHARED_HASHTAGS.len());
}

#[test]
fn a_scenario_with_an_unknown_placeholder_is_invalid() {
    let toml = format!(
        "{MINIMAL}\n[end_condition]\nkind = \"time_cap\"\n[meta]\ntitle = \"Reach {{cap_typo}}\"\n"
    );
    let err = Scenario::from_toml_str("typo", &toml)
        .unwrap_err()
        .to_string();

    assert!(err.contains("meta.title"), "{err}");
    assert!(err.contains("{cap_typo}"), "{err}");
}

fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn hashtags_put_scenario_tags_first_then_the_shared_pool_without_duplicates() {
    let merged = meta::merge_hashtags(
        &["doubling".into(), "#Satisfying".into(), "physics".into()],
        &["satisfying", "#shorts", "physics"],
    );

    assert_eq!(
        merged,
        vec!["#doubling", "#Satisfying", "#physics", "#shorts"]
    );
}

#[test]
fn hashtags_drop_whitespace_and_empty_tags() {
    let merged = meta::merge_hashtags(&[" ball  count ".into(), "#".into(), "".into()], &[]);

    assert_eq!(merged, vec!["#ballcount"]);
}

#[test]
fn templates_fill_in_placeholders_with_values() {
    let text = meta::interpolate(
        "Every bounce it doubles - can it reach {population_cap}? ({ball.speed} px/s)",
        &vars(&[("population_cap", "2000"), ("ball.speed", "250")]),
    );

    assert_eq!(
        text.unwrap(),
        "Every bounce it doubles - can it reach 2000? (250 px/s)"
    );
}

#[test]
fn doubled_braces_are_literal_braces() {
    let text = meta::interpolate("{{not a placeholder}} {n}", &vars(&[("n", "3")]));

    assert_eq!(text.unwrap(), "{not a placeholder} 3");
}

#[test]
fn unknown_or_unclosed_placeholders_are_errors_naming_the_problem() {
    let unknown = meta::interpolate("reach {cap}", &vars(&[("population_cap", "2000")]));
    let unclosed = meta::interpolate("reach {population_cap", &vars(&[("population_cap", "1")]));

    let unknown = unknown.unwrap_err();
    assert!(unknown.contains("{cap}"), "{unknown}");
    assert!(
        unknown.contains("population_cap"),
        "lists known names: {unknown}"
    );
    assert!(unclosed.unwrap_err().contains("unclosed"));
}

#[test]
fn shipped_doubling_metadata_names_its_population_cap() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios/doubling.toml");
    let scenario = Scenario::from_file(&path).unwrap();
    let meta = VideoMeta::new(&scenario, &sim::run(&scenario, 1));

    assert_eq!(
        meta.title,
        "Every bounce it doubles - until it breaks at 1000000"
    );
    assert!(
        meta.description
            .contains("until there are 1000000 and it melts down."),
        "{}",
        meta.description
    );
    assert_eq!(meta.hashtags[0], "#doubling");
}
