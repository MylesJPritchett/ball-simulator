//! Quality Gate: dull Seeds are rejected from the record alone.

mod support;

use ball_simulator::sim::{self, quality_gate, FilmLeftSample, Rejection, Verdict, SUBSTEP_HZ};
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

/// 1 s of substeps.
const SECOND: u64 = SUBSTEP_HZ;

/// A Film run (its End Condition fires 5 s in) judged as if its Film left
/// had followed `series` of (substep, left) instead: no Scenario here clears
/// its Film by itself.
fn judge_film_left(gate: &str, series: &[(u64, f64)]) -> Verdict {
    let toml = format!(
        "{}[[rules]]\nkind = \"film\"\nregrow = 0.0\n{gate}",
        support::filled_after(5.0)
    );
    let scenario = support::scenario(&toml);
    let mut record = sim::run(&scenario, 1);
    record.film_left = series
        .iter()
        .map(|&(substep, left)| FilmLeftSample { substep, left })
        .collect();
    quality_gate::judge(&scenario, &record)
}

#[test]
fn a_seed_that_spends_too_long_hunting_the_last_of_the_film_is_rejected() {
    // Below 1% at 2 s, cleared at 4.5 s: a 2.5 s final stretch.
    let verdict = judge_film_left(
        "[quality_gate]\nmax_final_stretch = 2.0\n",
        &[(0, 1.0), (2 * SECOND, 0.009), (9 * SECOND / 2, 0.0)],
    );

    assert_eq!(
        verdict.rejection(),
        Some(&Rejection::FinalStretchTooLong {
            seconds: 2.5,
            max: 2.0
        })
    );
    let reason = verdict.rejection().unwrap().to_string();
    assert!(reason.contains("final stretch"), "{reason}");
}

#[test]
fn a_seed_that_clears_the_last_of_the_film_quickly_is_accepted() {
    // Still above 1% at 2 s; below it at 3 s; cleared at 4.5 s.
    let verdict = judge_film_left(
        "[quality_gate]\nmax_final_stretch = 2.0\n",
        &[
            (0, 1.0),
            (2 * SECOND, 0.011),
            (3 * SECOND, 0.004),
            (9 * SECOND / 2, 0.0),
        ],
    );

    assert_eq!(verdict, Verdict::Accepted);
}

#[test]
fn a_film_never_cleared_stretches_until_the_end_condition() {
    // Below 1% at 1 s and never cleared; the End Condition fires at 5 s.
    let verdict = judge_film_left(
        "[quality_gate]\nmax_final_stretch = 3.0\n",
        &[(0, 1.0), (SECOND, 0.005)],
    );

    assert!(matches!(
        verdict.rejection(),
        Some(Rejection::FinalStretchTooLong { seconds, .. }) if (*seconds - 4.0).abs() < 0.01
    ));
}

#[test]
fn a_seed_won_by_a_ball_too_big_for_the_arena_is_rejected() {
    // The Arena closes in until the ball (radius 30) fills it.
    let record = support::run(
        &filled_after_5_seconds("[quality_gate]\nmax_ball_radius_at_end = 0.5\n"),
        1,
    );

    assert!(matches!(
        record.quality_gate.rejection(),
        Some(Rejection::BallTooBig { fraction, max }) if *fraction > 0.99 && *max == 0.5
    ));
    let reason = record.quality_gate.rejection().unwrap().to_string();
    assert!(reason.contains("Arena radius"), "{reason}");
}

#[test]
fn a_seed_whose_balls_stay_small_is_accepted() {
    // Balls of radius 30 in an Arena of radius 460: about 0.065 of it.
    let record = support::run(
        &ends_on_first_hit("[quality_gate]\nmax_ball_radius_at_end = 0.1\n"),
        1,
    );

    assert_eq!(record.quality_gate, Verdict::Accepted);
}

#[test]
fn ball_radius_at_end_is_judged_against_the_arena_radius() {
    let record = support::run(
        &ends_on_first_hit("[quality_gate]\nmax_ball_radius_at_end = 0.06\n"),
        1,
    );

    assert!(matches!(
        record.quality_gate.rejection(),
        Some(Rejection::BallTooBig { fraction, .. }) if (*fraction - 30.0 / 460.0).abs() < 1e-9
    ));
}
