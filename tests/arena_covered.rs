//! The arena_covered End Condition, tested through the seam.

mod support;

use ball_simulator::scenario::Scenario;
use support::{run, MINIMAL};

const ARENA_COVERED_END: &str = "\n[end_condition]\nkind = \"arena_covered\"\n";

/// A still ball of radius 400 while the Arena shrinks from 460 at 10 units/s.
/// The ball stays inside the Arena, so it covers (400 / R)^2 of it: 98% at
/// R = 400 / sqrt(0.98) = 404.06, which the Boundary reaches after 5.594 s.
fn still_ball_in_a_shrinking_arena() -> String {
    MINIMAL
        .replace("time_cap = 5.0", "time_cap = 10.0")
        .replace("radius = 30.0", "radius = 400.0")
        .replace("speed = 900.0", "speed = 0.0")
        + "\n[[rules]]\nkind = \"shrinking_arena\"\nrate = 10.0\nmin_radius = 300.0\n"
}

#[test]
fn arena_covered_fires_once_the_balls_cover_98_percent_of_the_arena() {
    let record = run(&(still_ball_in_a_shrinking_arena() + ARENA_COVERED_END), 1);

    // Neither early nor late by more than 0.2 units of Boundary (~0.1% coverage).
    assert_eq!(record.end.condition, "arena_covered");
    assert!(
        (record.end.time() - 5.594).abs() < 0.02,
        "fired at {} s",
        record.end.time()
    );
}

#[test]
fn overlapping_balls_cover_the_arena_only_once() {
    // One ball of radius 200 splits into 10 on its first Boundary Hit, fanned
    // across a sliver of a degree, so the copies stay stacked on top of each
    // other. Their areas add up to 1.9 Arenas, but together they cover only
    // what one of them does, (200 / 460)^2 = 19%.
    let toml = MINIMAL
        .replace("time_cap = 5.0", "time_cap = 5.0\npopulation_cap = 10")
        .replace("radius = 30.0", "radius = 200.0")
        + "\n[[rules]]\nkind = \"split\"\ninto = 10\nspread = 0.001\n"
        + ARENA_COVERED_END;
    let record = run(&toml, 1);

    assert_eq!(record.final_balls.len(), 10);
    assert_eq!(record.end.condition, "time_cap");
}

#[test]
fn arena_covered_takes_no_parameters() {
    let toml = format!("{MINIMAL}{ARENA_COVERED_END}fraction = 0.9\n");
    let err = Scenario::from_toml_str("broken", &toml)
        .expect_err("Scenario should be rejected")
        .to_string();
    assert!(
        err.contains("End Condition 'arena_covered' has no parameter 'fraction'"),
        "{err}"
    );
}
