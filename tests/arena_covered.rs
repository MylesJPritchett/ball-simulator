//! The arena_covered End Condition, tested through the seam.

mod support;

use ball_simulator::scenario::Scenario;
use support::{run, MINIMAL};

const ARENA_COVERED_END: &str = "\n[end_condition]\nkind = \"arena_covered\"\n";

/// A ball of radius 400 resting on the bottom of an Arena that shrinks from
/// 460 at 10 units/s. Light gravity settles it onto the Boundary within 5 s
/// whatever the Seed, without bouncing, and it then rides the Boundary in.
fn resting_ball_in_a_shrinking_arena() -> String {
    MINIMAL
        .replace("time_cap = 5.0", "time_cap = 10.0")
        .replace("radius = 30.0", "radius = 400.0")
        .replace("speed = 900.0", "speed = 0.0")
        + "\n[[rules]]\nkind = \"gravity\"\nstrength = 4.0\n"
        + "\n[[rules]]\nkind = \"shrinking_arena\"\nrate = 10.0\nmin_radius = 300.0\n"
}

#[test]
fn arena_covered_fires_once_the_balls_cover_98_percent_inside_the_rim_margin() {
    // Coverage is measured inside the rim margin, a circle of 0.98 R. The
    // ball (offset R - 400 from the centre) covers 98% of that circle at
    // R = 409.87 (circle-circle intersection, worked out by hand), which the
    // Boundary reaches after 5.013 s. Ignoring the margin, the ball would
    // need R = 400 / sqrt(0.98) = 404.06, at 5.594 s.
    for seed in 0..5 {
        let record = run(
            &(resting_ball_in_a_shrinking_arena() + ARENA_COVERED_END),
            seed,
        );

        // Neither early nor late by more than 0.2 units of Boundary (~0.1% coverage).
        assert_eq!(record.end.condition, "arena_covered", "seed {seed}");
        assert!(
            (record.end.time() - 5.013).abs() < 0.02,
            "seed {seed}: fired at {} s",
            record.end.time()
        );
    }
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
