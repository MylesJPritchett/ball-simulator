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
    let record = sim::run(&shipped("minimal"), 1);

    assert_eq!(record.boundary_hits.len(), 11);
    assert_eq!(record.ball_count.last().unwrap().count, 1);
    assert_eq!(record.end.condition, "time_cap");
    assert_eq!(record.end.substep, 4800);
    assert_eq!(record.final_state_hash, 12091774788095173436);
}

#[test]
fn doubling_seed_1() {
    let record = sim::run(&shipped("doubling"), 1);

    assert_eq!(record.boundary_hits.len(), 2015);
    assert_eq!(record.ball_count.last().unwrap().count, 2000);
    assert_eq!(record.end.condition, "population_cap");
    assert_eq!(record.end.substep, 12282);
    assert_eq!(record.final_state_hash, 2820098640751858260);
}

#[test]
fn faster_every_bounce_seed_1() {
    let record = sim::run(&shipped("faster_every_bounce"), 1);

    assert_eq!(record.end.condition, "time_cap");
    assert_eq!(record.end.substep, 9600);
    assert!(record.stats.max_speed <= 4000.0 + 1e-9);
    assert_eq!(record.boundary_hits.len(), 71);
    assert_eq!(record.final_state_hash, 920117634866952227);
}

#[test]
fn shrinking_arena_seed_1() {
    let record = sim::run(&shipped("shrinking_arena"), 1);

    assert_eq!(record.end.condition, "time_cap");
    assert_eq!(record.end.substep, 19200);
    assert_eq!(record.final_arena_radius, 80.0);
    assert_eq!(record.ball_count.last().unwrap().count, 1);
    assert_eq!(record.boundary_hits.len(), 1090);
    assert_eq!(record.final_state_hash, 18311559355663574032);
}
