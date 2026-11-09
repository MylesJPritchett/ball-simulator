//! The look of a Video as far as the simulation seam can see it: palette
//! choice, visual toggles and the live Counter.

mod support;

use std::collections::BTreeSet;

use ball_simulator::look::palette;
use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{SimRecord, Simulation};
use support::{run, scenario, MINIMAL};

/// MINIMAL with a `[look]` table appended (it ends with the `[ball]` table).
fn with_look(look: &str) -> String {
    format!("{MINIMAL}\n[look]\n{look}\n")
}

#[test]
fn palette_varies_by_seed() {
    let names: BTreeSet<String> = (0..20).map(|seed| run(MINIMAL, seed).palette).collect();

    assert!(names.len() >= 3, "only {names:?} across 20 Seeds");
    for name in &names {
        assert!(palette::by_name(name).is_some(), "unknown palette {name}");
    }
}

#[test]
fn same_seed_gives_same_palette() {
    assert_eq!(run(MINIMAL, 5).palette, run(MINIMAL, 5).palette);
}

#[test]
fn pinned_palette_is_used_for_every_seed() {
    let toml = with_look("palette = \"neon\"");
    for seed in 0..20 {
        assert_eq!(run(&toml, seed).palette, "neon", "seed {seed}");
    }
}

#[test]
fn unknown_pinned_palette_is_rejected() {
    let err = Scenario::from_toml_str("broken", &with_look("palette = \"plaid\""))
        .expect_err("unknown palette should be rejected")
        .to_string();
    assert!(err.contains("unknown palette 'plaid'"), "{err}");
}

#[test]
fn choosing_a_palette_does_not_change_the_simulation() {
    // Palette choice must not draw from the simulation's RNG: the shipped
    // snapshot for minimal seed 1 stays the same.
    let pinned = with_look("palette = \"neon\"");
    assert_eq!(
        run(MINIMAL, 1).final_state_hash,
        run(&pinned, 1).final_state_hash
    );
}

/// Steps a Scenario frame by frame, collecting the live Counter reading and
/// the substep it was read at; also returns the finished record.
fn read_counter_every_frame(toml: &str, seed: u64) -> (Vec<(u64, u64)>, SimRecord) {
    let scenario = scenario(toml);
    let counter = scenario.look.counter;
    let mut sim = Simulation::new(&scenario, seed);
    let mut readings = Vec::new();
    loop {
        readings.push((counter.read(&sim), sim.substep()));
        if sim.is_ended() {
            break;
        }
        sim.step_frame();
    }
    (readings, sim.into_record())
}

#[test]
fn boundary_hit_counter_matches_the_record_on_every_frame() {
    let (readings, record) = read_counter_every_frame(MINIMAL, 7);

    assert!(record.boundary_hits.len() >= 5);
    assert_eq!(
        readings.last().unwrap().0,
        record.boundary_hits.len() as u64
    );
    for (value, substep) in readings {
        let expected = record
            .boundary_hits
            .iter()
            .filter(|h| h.substep <= substep)
            .count() as u64;
        assert_eq!(value, expected, "at substep {substep}");
    }
}

#[test]
fn ball_count_counter_matches_the_record_on_every_frame() {
    let (readings, record) = read_counter_every_frame(&with_look("counter = \"ball_count\""), 3);

    for (value, substep) in readings {
        let expected = record
            .ball_count
            .iter()
            .rev()
            .find(|s| s.substep <= substep)
            .unwrap()
            .count as u64;
        assert_eq!(value, expected, "at substep {substep}");
    }
}

#[test]
fn speed_counter_shows_the_ball_speed() {
    // No Rules: the ball keeps its initial speed of 900 forever.
    let (readings, _) = read_counter_every_frame(&with_look("counter = \"speed\""), 2);

    for (value, substep) in readings {
        assert_eq!(value, 900, "at substep {substep}");
    }
}

#[test]
fn trails_and_glow_are_on_by_default_and_can_be_switched_off() {
    let default = scenario(MINIMAL).look;
    assert!(default.trails && default.glow);

    let off = scenario(&with_look("trails = false\nglow = false")).look;
    assert!(!off.trails && !off.glow);
}

#[test]
fn unknown_counter_is_rejected() {
    let err = Scenario::from_toml_str("broken", &with_look("counter = \"vibes\""))
        .expect_err("unknown Counter should be rejected")
        .to_string();
    assert!(err.contains("vibes"), "{err}");
}
