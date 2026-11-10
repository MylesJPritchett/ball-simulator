//! Seed retries: rejected Seeds are replaced until one passes the Quality Gate.

mod support;

use ball_simulator::pipeline::{self, MAX_SEED_ATTEMPTS};
use ball_simulator::sim::{self, Rejection};
use support::filled_after;

/// Ball speed is a range, so some Seeds are too slow to make 115 Boundary Hits
/// before the ball fills the Arena 5 s in, and others are fast enough.
fn sometimes_quiet() -> String {
    let toml = filled_after(5.0).replace("speed = 900.0", "speed = { min = 200.0, max = 1600.0 }");
    format!("{toml}\n[quality_gate]\nmin_boundary_hits = 115\n")
}

#[test]
fn a_passing_seed_is_accepted_first_time() {
    let accepted = pipeline::pick_seed(&support::scenario(&filled_after(5.0)), [7, 8, 9]).unwrap();

    assert_eq!(accepted.seed, 7);
    assert!(accepted.rejected.is_empty());
    assert!(accepted.record.quality_gate.is_accepted());
    assert_eq!(
        accepted.record,
        sim::run(&support::scenario(&filled_after(5.0)), 7)
    );
}

#[test]
fn rejected_seeds_are_retried_with_the_next_seeds() {
    let scenario = support::scenario(&sometimes_quiet());
    // Seeds 11 to 13 are too slow for this Scenario (checked below); 14 is not.
    let accepted = pipeline::pick_seed(&scenario, pipeline::seeds_from(11)).unwrap();

    assert_eq!(accepted.seed, 14);
    assert!(accepted.record.quality_gate.is_accepted());
    let rejected: Vec<u64> = accepted.rejected.iter().map(|r| r.seed).collect();
    assert_eq!(rejected, vec![11, 12, 13]);
    for r in &accepted.rejected {
        assert!(matches!(r.reason, Rejection::TooQuiet { .. }));
        assert_eq!(
            sim::run(&scenario, r.seed).quality_gate.rejection(),
            Some(&r.reason)
        );
    }
}

#[test]
fn a_scenario_that_keeps_failing_gives_up_after_the_retry_limit() {
    // One slow ball never makes 1000 Boundary Hits in 5 s.
    let scenario = support::scenario(&format!(
        "{}\n[quality_gate]\nmin_boundary_hits = 1000\n",
        filled_after(5.0)
    ));
    let err = pipeline::pick_seed(&scenario, pipeline::seeds_from(1)).unwrap_err();

    assert_eq!(err.scenario, "inline");
    assert_eq!(err.rejected.len(), MAX_SEED_ATTEMPTS);
    let message = err.to_string();
    assert!(message.contains("'inline'"), "{message}");
    assert!(message.contains("too few Boundary Hits"), "{message}");
}
