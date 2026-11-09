//! Core simulation behaviour, tested only through the seam:
//! Scenario + Seed -> simulation record.

mod support;

use support::{run, MINIMAL};

#[test]
fn minimal_scenario_ends_at_its_time_cap() {
    let record = run(MINIMAL, 1);

    assert_eq!(record.end.condition, "time_cap");
    assert!((record.end.time() - 5.0).abs() < 1e-9);
}

#[test]
fn ball_never_ends_a_substep_outside_the_arena() {
    // Fast ball, small Arena: many Boundary Hits per second.
    let fast = MINIMAL.replace("speed = 900.0", "speed = 20000.0");
    for seed in 0..20 {
        let record = run(&fast, seed);
        assert!(
            record.stats.max_boundary_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_boundary_overshoot
        );
    }
}

#[test]
fn ball_bouncing_in_the_arena_produces_regular_boundary_hits() {
    // Speed 900 across an effective diameter of 2 * (460 - 30) = 860 means
    // roughly one Boundary Hit per second in the worst case (through the centre);
    // off-centre chords are shorter, so at least ~5 in 5 seconds.
    let record = run(MINIMAL, 7);

    assert!(record.boundary_hits.len() >= 5, "{:?}", record.boundary_hits.len());
    for pair in record.boundary_hits.windows(2) {
        assert!(pair[0].substep < pair[1].substep, "hits are in time order");
    }
    for hit in &record.boundary_hits {
        assert_eq!(hit.ball_id, 0);
        assert!(hit.pitch > 0.0);
        assert!(hit.time() <= record.end.time());
    }
}

#[test]
fn ball_count_stays_one_without_rules() {
    let record = run(MINIMAL, 3);

    assert_eq!(record.ball_count.len(), 1);
    assert_eq!(record.ball_count[0].substep, 0);
    assert_eq!(record.ball_count[0].count, 1);
}

#[test]
fn same_scenario_and_seed_give_identical_records() {
    assert_eq!(run(MINIMAL, 42), run(MINIMAL, 42));
}

#[test]
fn different_seeds_give_different_runs() {
    assert_ne!(
        run(MINIMAL, 1).final_state_hash,
        run(MINIMAL, 2).final_state_hash
    );
}
