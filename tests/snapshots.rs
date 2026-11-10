//! Snapshot tests: each shipped Scenario with a fixed Seed.
//!
//! These catch accidental behaviour changes. If a change is intentional,
//! update the expected values deliberately and say why in the commit.

use std::path::Path;

use ball_simulator::scenario::Scenario;
use ball_simulator::sim;

fn shipped(id: &str) -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join(format!("{id}.toml"));
    Scenario::from_file(&path).expect("shipped Scenario should be valid")
}

/// The smallest Scenario (one ball, no Rules): a test fixture, not shipped.
fn minimal() -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.toml");
    Scenario::from_file(&path).expect("fixture Scenario should be valid")
}

#[test]
fn every_shipped_scenario_is_valid() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "toml") {
            Scenario::from_file(&path).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}

#[test]
fn every_shipped_scenario_sets_quality_gate_thresholds_its_snapshot_seed_passes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "toml") {
            let scenario = Scenario::from_file(&path).unwrap();
            let gate = &scenario.quality_gate;
            assert!(
                gate.min_duration > 0.0 && gate.max_duration.is_finite(),
                "{} needs a [quality_gate] duration range",
                path.display()
            );
            assert!(gate.min_boundary_hits > 0, "{}", path.display());
            let verdict = sim::run(&scenario, 1).quality_gate;
            assert!(verdict.is_accepted(), "{}: {verdict:?}", path.display());
        }
    }
}

#[test]
fn minimal_seed_1() {
    let record = sim::run(&minimal(), 1);

    assert_eq!(record.boundary_hits.len(), 11);
    assert_eq!(record.ball_count.last().unwrap().count, 1);
    assert_eq!(record.end.condition, "time_cap");
    assert_eq!(record.end.substep, 4800);
    assert_eq!(record.final_state_hash, 12091774788095173436);
}

/// What a shipped Scenario does with Seed 1.
struct Snapshot {
    boundary_hits: usize,
    final_ball_count: usize,
    end_condition: &'static str,
    end_substep: u64,
    final_state_hash: u64,
}

fn assert_seed_1(id: &str, want: Snapshot) {
    let record = sim::run(&shipped(id), 1);

    assert_eq!(record.boundary_hits.len(), want.boundary_hits, "{id}");
    assert_eq!(
        record.ball_count.last().unwrap().count,
        want.final_ball_count,
        "{id}"
    );
    assert_eq!(record.end.condition, want.end_condition, "{id}");
    assert_eq!(record.end.substep, want.end_substep, "{id}");
    assert_eq!(record.final_state_hash, want.final_state_hash, "{id}");
}

#[test]
fn changing_colour_seed_1() {
    assert_seed_1(
        "changing_colour",
        Snapshot {
            boundary_hits: 51,
            final_ball_count: 1,
            end_condition: "time_cap",
            end_substep: 14129,
            final_state_hash: 5589162356901613303,
        },
    );
}

#[test]
fn closing_in_seed_1() {
    assert_seed_1(
        "closing_in",
        Snapshot {
            boundary_hits: 94,
            final_ball_count: 1,
            end_condition: "arena_filled",
            end_substep: 13851,
            final_state_hash: 5573714443546182967,
        },
    );
}

#[test]
fn doubling_seed_1() {
    assert_seed_1(
        "doubling",
        Snapshot {
            boundary_hits: 2216904,
            final_ball_count: 1000000,
            end_condition: "meltdown",
            end_substep: 22897,
            final_state_hash: 15806495560928583018,
        },
    );
}

#[test]
fn faster_every_bounce_seed_1() {
    assert_seed_1(
        "faster_every_bounce",
        Snapshot {
            boundary_hits: 62,
            final_ball_count: 1,
            end_condition: "max_speed_reached",
            end_substep: 14852,
            final_state_hash: 2899949589808386359,
        },
    );
}

#[test]
fn gravity_split_seed_1() {
    assert_seed_1(
        "gravity_split",
        Snapshot {
            boundary_hits: 1499,
            final_ball_count: 1500,
            end_condition: "population_cap",
            end_substep: 18384,
            final_state_hash: 8991188671581940454,
        },
    );
}

#[test]
fn growing_seed_1() {
    assert_seed_1(
        "growing",
        Snapshot {
            boundary_hits: 74,
            final_ball_count: 1,
            end_condition: "arena_filled",
            end_substep: 16801,
            final_state_hash: 9740918032704497141,
        },
    );
}

#[test]
fn shrinking_seed_1() {
    assert_seed_1(
        "shrinking",
        Snapshot {
            boundary_hits: 87,
            final_ball_count: 0,
            end_condition: "vanished",
            end_substep: 16129,
            final_state_hash: 4565493003704689149,
        },
    );
}

#[test]
fn shrinking_arena_seed_1() {
    assert_seed_1(
        "shrinking_arena",
        Snapshot {
            boundary_hits: 305,
            final_ball_count: 1,
            end_condition: "arena_filled",
            end_substep: 18032,
            final_state_hash: 14872965655936800902,
        },
    );
}
