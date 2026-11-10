//! Quality Gate: dull Seeds are rejected from the record alone.

mod support;

use ball_simulator::sim::{Rejection, Verdict};
use support::MINIMAL;

/// Splits into 2 on the first Boundary Hit and melts down at a Population
/// Cap of 2, so the Video (with its Play-out and hold) is about 3.5 s long.
fn ends_on_first_hit(gate: &str) -> String {
    let toml = MINIMAL.replace("time_cap = 5.0", "time_cap = 20.0\npopulation_cap = 2");
    format!(
        "{toml}\n[[rules]]\nkind = \"split\"\ninto = 2\nspread = 30.0\n\
         [end_condition]\nkind = \"meltdown\"\n{gate}"
    )
}

/// The ball fills the Arena 5 s in.
fn filled_after_5_seconds(gate: &str) -> String {
    format!("{}{gate}", support::filled_after(5.0))
}

#[test]
fn a_seed_whose_end_condition_fires_too_early_is_rejected() {
    let record = support::run(
        &ends_on_first_hit("[quality_gate]\nmin_duration = 10.0\n"),
        1,
    );

    assert_eq!(record.end.condition, "meltdown");
    assert!(matches!(
        record.quality_gate.rejection(),
        Some(Rejection::TooShort { .. })
    ));
}

#[test]
fn a_seed_that_reaches_the_time_cap_is_rejected_whatever_its_end_condition() {
    // Nothing grows, splits or closes in, so no End Condition can fire.
    for kind in ["arena_filled", "arena_covered"] {
        let record = support::run(
            &format!("{MINIMAL}\n[end_condition]\nkind = \"{kind}\"\n"),
            1,
        );

        assert_eq!(record.end.condition, "time_cap", "{kind}");
        assert_eq!(
            record.quality_gate.rejection(),
            Some(&Rejection::NeverEnded { time_cap: 5.0 }),
            "{kind}"
        );
    }
}

#[test]
fn a_seed_with_too_few_boundary_hits_is_rejected() {
    // The Meltdown follows the first Boundary Hit within 1.5 s.
    let record = support::run(
        &ends_on_first_hit("[quality_gate]\nmin_boundary_hits = 20\n"),
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
        &filled_after_5_seconds("[quality_gate]\nmax_duration = 6.0\n"),
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
        &filled_after_5_seconds(
            "[quality_gate]\nmin_duration = 6.0\nmax_duration = 7.0\nmin_boundary_hits = 3\n",
        ),
        1,
    );

    assert_eq!(record.quality_gate, Verdict::Accepted);
}

#[test]
fn duration_is_judged_on_video_length_including_the_hold() {
    // 5 s of action is under 6 s, but the Video (with its hold) is not.
    let record = support::run(
        &filled_after_5_seconds("[quality_gate]\nmin_duration = 6.0\n"),
        1,
    );

    assert!(record.end.time() < 6.0);
    assert!(record.quality_gate.is_accepted());
}
