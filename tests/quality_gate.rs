//! Quality Gate: dull Seeds are rejected from the record alone.

mod support;

use ball_simulator::sim::{Rejection, Verdict};
use support::MINIMAL;

/// Splits into 2 on the first Boundary Hit and stops at a Population Cap of 2,
/// so the action ends well under a second in.
fn ends_on_first_hit(gate: &str) -> String {
    let toml = MINIMAL.replace("time_cap = 5.0", "time_cap = 20.0\npopulation_cap = 2");
    format!(
        "{toml}\n[[rules]]\nkind = \"split\"\ninto = 2\nspread = 30.0\n\
         [end_condition]\nkind = \"population_cap\"\n{gate}"
    )
}

#[test]
fn a_seed_whose_end_condition_fires_too_early_is_rejected() {
    let record = support::run(
        &ends_on_first_hit("[quality_gate]\nmin_duration = 10.0\n"),
        1,
    );

    assert_eq!(record.end.condition, "population_cap");
    assert!(matches!(
        record.quality_gate.rejection(),
        Some(Rejection::TooShort { .. })
    ));
}

#[test]
fn a_seed_whose_end_condition_never_fires_before_the_time_cap_is_rejected() {
    // The Population Cap is far out of reach without a split Rule.
    let toml = MINIMAL.replace("time_cap = 5.0", "time_cap = 5.0\npopulation_cap = 50");
    let record = support::run(
        &format!("{toml}\n[end_condition]\nkind = \"population_cap\"\n"),
        1,
    );

    assert_eq!(record.end.condition, "time_cap");
    assert_eq!(
        record.quality_gate.rejection(),
        Some(&Rejection::NeverEnded { time_cap: 5.0 })
    );
}

#[test]
fn reaching_the_time_cap_is_fine_when_the_time_cap_is_the_end_condition() {
    let record = support::run(MINIMAL, 1);

    assert_eq!(record.end.condition, "time_cap");
    assert!(record.quality_gate.is_accepted());
}

#[test]
fn a_seed_with_too_few_boundary_hits_is_rejected() {
    // A slow ball in 5 s hits the Boundary only a handful of times.
    let toml = MINIMAL.replace("speed = 900.0", "speed = 100.0");
    let record = support::run(
        &format!("{toml}\n[quality_gate]\nmin_boundary_hits = 20\n"),
        1,
    );

    let hits = record.boundary_hits.len();
    assert!(hits < 20);
    assert_eq!(
        record.quality_gate.rejection(),
        Some(&Rejection::TooQuiet { hits, min: 20 })
    );
}

#[test]
fn a_seed_whose_video_runs_too_long_is_rejected() {
    let record = support::run(
        &format!("{MINIMAL}\n[quality_gate]\nmax_duration = 6.0\n"),
        1,
    );

    // 5 s of action plus the 1.5 s hold.
    assert!(matches!(
        record.quality_gate.rejection(),
        Some(Rejection::TooLong { max, .. }) if *max == 6.0
    ));
}

#[test]
fn a_seed_meeting_every_threshold_is_accepted() {
    let record = support::run(
        &format!(
            "{MINIMAL}\n[quality_gate]\nmin_duration = 6.0\nmax_duration = 7.0\nmin_boundary_hits = 3\n"
        ),
        1,
    );

    assert_eq!(record.quality_gate, Verdict::Accepted);
}

#[test]
fn duration_is_judged_on_video_length_including_the_hold() {
    // 5 s of action is under 6 s, but the Video (with its hold) is not.
    let record = support::run(
        &format!("{MINIMAL}\n[quality_gate]\nmin_duration = 6.0\n"),
        1,
    );

    assert!(record.end.time() < 6.0);
    assert!(record.quality_gate.is_accepted());
}
