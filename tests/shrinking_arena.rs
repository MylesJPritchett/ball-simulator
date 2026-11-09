//! The shrinking-Arena Rule, tested through the seam.

mod support;

use ball_simulator::scenario::Scenario;
use support::{run, MINIMAL};

fn shrinking(rate: f64, min_radius: f64, time_cap: f64) -> String {
    let base = MINIMAL.replace("time_cap = 5.0", &format!("time_cap = {time_cap:?}"));
    format!(
        "{base}\n[[rules]]\nkind = \"shrinking_arena\"\nrate = {rate:?}\nmin_radius = {min_radius:?}\n"
    )
}

#[test]
fn arena_radius_shrinks_at_the_configured_rate() {
    // 460 units, shrinking 50 units/s for 2 s.
    let record = run(&shrinking(50.0, 100.0, 2.0), 1);

    assert!(
        (record.final_arena_radius - 360.0).abs() < 1e-6,
        "{}",
        record.final_arena_radius
    );
}

#[test]
fn arena_stops_shrinking_at_the_minimum_radius() {
    // 460 -> 200 at 100 units/s takes 2.6 s; the run lasts 5 s.
    let record = run(&shrinking(100.0, 200.0, 5.0), 1);

    assert_eq!(record.final_arena_radius, 200.0);
}

#[test]
fn arena_min_reached_ends_when_the_arena_stops_shrinking() {
    // 460 -> 200 at 100 units/s takes 2.6 s; the time cap is 5 s.
    let toml = shrinking(100.0, 200.0, 5.0)
        + "\n[end_condition]\nkind = \"arena_min_reached\"\nmin_radius = 200.0\n";
    let record = run(&toml, 1);

    assert_eq!(record.end.condition, "arena_min_reached");
    assert!(
        (record.end.time() - 2.6).abs() <= 1.0 / 240.0,
        "{}",
        record.end.time()
    );
    assert_eq!(record.final_arena_radius, 200.0);
}

#[test]
fn no_ball_ever_ends_a_substep_outside_the_shrinking_arena() {
    // Fast shrink down to just above the ball's size, fast ball.
    let toml = shrinking(400.0, 35.0, 3.0).replace("speed = 900.0", "speed = 5000.0");
    for seed in 0..20 {
        let record = run(&toml, seed);
        assert!(
            record.stats.max_boundary_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_boundary_overshoot
        );
    }
}

#[test]
fn boundary_hits_get_more_frequent_as_the_arena_shrinks() {
    // 460 -> 120 over the first 8.5 s of a 10 s run.
    let toml = shrinking(40.0, 120.0, 10.0);
    for seed in 0..5 {
        let record = run(&toml, seed);
        let split = record.end.substep / 2;
        let first = record
            .boundary_hits
            .iter()
            .filter(|h| h.substep <= split)
            .count();
        let second = record.boundary_hits.len() - first;
        assert!(
            second > first * 2,
            "seed {seed}: {first} hits in the first half, {second} in the second"
        );
    }
}

#[test]
fn ball_caught_by_the_closing_boundary_registers_a_boundary_hit() {
    // A very slow ball cannot reach the Boundary on its own within the run;
    // only the closing Boundary can produce hits.
    let toml = shrinking(400.0, 40.0, 2.0).replace("speed = 900.0", "speed = 1.0");
    let record = run(&toml, 3);

    assert!(!record.boundary_hits.is_empty());
}

#[test]
fn invalid_parameters_are_rejected_with_clear_errors() {
    let error_for = |toml: &str| {
        let toml = format!("{toml}\n[end_condition]\nkind = \"time_cap\"\n");
        Scenario::from_toml_str("broken", &toml)
            .expect_err("Scenario should be rejected")
            .to_string()
    };

    let err = error_for(&shrinking(-5.0, 100.0, 5.0));
    assert!(
        err.contains("shrinking_arena") && err.contains("rate"),
        "{err}"
    );

    let err = error_for(&shrinking(5.0, 0.0, 5.0));
    assert!(
        err.contains("shrinking_arena") && err.contains("min_radius"),
        "{err}"
    );

    let missing = format!("{MINIMAL}\n[[rules]]\nkind = \"shrinking_arena\"\nrate = 5.0\n");
    let err = error_for(&missing);
    assert!(err.contains("min_radius"), "{err}");
}

#[test]
fn closing_boundary_knocks_the_ball_away_instead_of_dragging_it() {
    // Without this, a ball slower than the Boundary would ride it and hit on
    // every substep: a burst of hundreds of notes.
    let toml = shrinking(400.0, 40.0, 2.0).replace("speed = 900.0", "speed = 1.0");
    for seed in 0..10 {
        let record = run(&toml, seed);
        for pair in record.boundary_hits.windows(2) {
            assert!(
                pair[1].substep - pair[0].substep > 1,
                "seed {seed}: hits on consecutive substeps {} and {}",
                pair[0].substep,
                pair[1].substep
            );
        }
    }
}
