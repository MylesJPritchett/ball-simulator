//! Grow and shrink Rules and the Arena-filled End Condition, tested through the seam.

mod support;

use support::{run, MINIMAL};

const SEED: u64 = 3;

fn with_rule(rule: &str) -> String {
    format!("{MINIMAL}\n[[rules]]\n{rule}\n")
}

#[test]
fn grow_adds_the_configured_amount_per_boundary_hit() {
    let record = run(&with_rule("kind = \"grow\"\namount = 5.0"), SEED);

    let hits = record.boundary_hits.len();
    assert!(hits >= 3, "expected several hits, got {hits}");
    let radius = record.final_balls[0].radius;
    assert!(
        (radius - (30.0 + 5.0 * hits as f64)).abs() < 1e-9,
        "radius {radius} after {hits} hits"
    );
}

#[test]
fn shrink_removes_the_configured_amount_per_boundary_hit() {
    let record = run(
        &with_rule("kind = \"shrink\"\namount = 2.0\nmin_radius = 1.0"),
        SEED,
    );

    let hits = record.boundary_hits.len();
    assert!(hits >= 3, "expected several hits, got {hits}");
    let radius = record.final_balls[0].radius;
    assert!(
        (radius - (30.0 - 2.0 * hits as f64)).abs() < 1e-9,
        "radius {radius} after {hits} hits"
    );
}

#[test]
fn shrink_never_goes_below_its_minimum() {
    let record = run(
        &with_rule("kind = \"shrink\"\namount = 10.0\nmin_radius = 4.0"),
        SEED,
    );

    // 30 -> 20 -> 10 -> 4 (not 0) after three hits.
    assert!(record.boundary_hits.len() > 3, "enough hits to bottom out");
    assert_eq!(record.final_balls[0].radius, 4.0);
}

#[test]
fn min_radius_reached_ends_on_the_hit_that_shrinks_a_ball_to_the_minimum() {
    let toml = format!(
        "{}\n[end_condition]\nkind = \"min_radius_reached\"\nmin_radius = 4.0\n",
        with_rule("kind = \"shrink\"\namount = 10.0\nmin_radius = 4.0")
    );
    let record = run(&toml, SEED);

    assert_eq!(record.end.condition, "min_radius_reached");
    // 30 -> 20 -> 10 -> 4: the third hit reaches the minimum.
    assert_eq!(record.boundary_hits.len(), 3);
    assert_eq!(record.boundary_hits[2].substep, record.end.substep);
    assert_eq!(record.final_balls[0].radius, 4.0);
}

#[test]
fn a_growing_ball_never_ends_a_substep_outside_the_arena() {
    for seed in 0..10 {
        let record = run(&with_rule("kind = \"grow\"\namount = 25.0"), seed);
        assert!(
            record.stats.max_boundary_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_boundary_overshoot
        );
        for ball in &record.final_balls {
            assert!(
                ball.radius <= 460.0 + 1e-9,
                "seed {seed}: radius {}",
                ball.radius
            );
        }
    }
}

fn filling(amount: f64) -> String {
    let toml = MINIMAL.replace("time_cap = 5.0", "time_cap = 60.0");
    format!(
        "{toml}\n[[rules]]\nkind = \"grow\"\namount = {amount:?}\n\n\
         [end_condition]\nkind = \"arena_filled\"\n"
    )
}

#[test]
fn arena_filled_fires_on_the_hit_that_grows_the_ball_to_the_arena() {
    let record = run(&filling(25.0), SEED);

    assert_eq!(record.end.condition, "arena_filled");
    assert!(record.end.time() < 60.0, "ended before the time cap");
    // 30 + 25 * 17 = 455 < 460: the 18th hit fills the Arena.
    assert_eq!(record.boundary_hits.len(), 18);
    assert_eq!(
        record.boundary_hits.last().unwrap().substep,
        record.end.substep
    );
    assert_eq!(record.final_balls[0].radius, 460.0);
    assert!(record.stats.max_boundary_overshoot <= 1e-9);
}

#[test]
fn arena_filled_is_followed_by_the_standard_ending() {
    let record = run(&filling(25.0), SEED);

    assert_eq!(record.ending.start_substep, record.end.substep);
    assert!((record.ending.cut_time() - (record.end.time() + 1.5)).abs() < 1e-9);
}

#[test]
fn arena_filled_does_not_fire_while_the_ball_is_smaller_than_the_arena() {
    let toml = format!(
        "{}\n[end_condition]\nkind = \"arena_filled\"\n",
        with_rule("kind = \"grow\"\namount = 1.0")
    );
    let record = run(&toml, SEED);

    assert_eq!(record.end.condition, "time_cap");
}
