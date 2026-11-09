//! Seed retries: rejected Seeds are replaced until one passes the Quality Gate.

mod support;

use ball_simulator::pipeline::{self, MAX_SEED_ATTEMPTS};
use ball_simulator::sim::{self, Rejection};
use support::MINIMAL;

/// Ball speed is a range, so some Seeds are too slow to make 6 Boundary Hits
/// in 5 s and others are fast enough.
fn sometimes_quiet() -> String {
    let toml = MINIMAL.replace("speed = 900.0", "speed = { min = 200.0, max = 1600.0 }");
    format!("{toml}\n[quality_gate]\nmin_boundary_hits = 6\n")
}

#[test]
fn a_passing_seed_is_accepted_first_time() {
    let accepted = pipeline::pick_seed(&support::scenario(MINIMAL), [7, 8, 9]).unwrap();

    assert_eq!(accepted.seed, 7);
    assert!(accepted.rejected.is_empty());
    assert!(accepted.record.quality_gate.is_accepted());
    assert_eq!(accepted.record, sim::run(&support::scenario(MINIMAL), 7));
}

#[test]
fn rejected_seeds_are_retried_with_the_next_seeds() {
    let scenario = support::scenario(&sometimes_quiet());
    // Seeds 4 and 5 are too slow for this Scenario (checked below); 6 is not.
    let accepted = pipeline::pick_seed(&scenario, pipeline::seeds_from(4)).unwrap();

    assert_eq!(accepted.seed, 6);
    assert!(accepted.record.quality_gate.is_accepted());
    let rejected: Vec<u64> = accepted.rejected.iter().map(|r| r.seed).collect();
    assert_eq!(rejected, vec![4, 5]);
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
        "{MINIMAL}\n[quality_gate]\nmin_boundary_hits = 1000\n"
    ));
    let err = pipeline::pick_seed(&scenario, pipeline::seeds_from(1)).unwrap_err();

    assert_eq!(err.scenario, "inline");
    assert_eq!(err.rejected.len(), MAX_SEED_ATTEMPTS);
    let message = err.to_string();
    assert!(message.contains("'inline'"), "{message}");
    assert!(message.contains("too few Boundary Hits"), "{message}");
}
