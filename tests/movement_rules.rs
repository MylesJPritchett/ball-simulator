//! Movement and escalation Rules (gravity, drag, speed-up, change colour) and
//! max speed enforcement, tested through the seam.

mod support;

use support::{run, MINIMAL};

/// MINIMAL with the ball starting at rest.
fn at_rest() -> String {
    MINIMAL.replace("speed = 900.0", "speed = 0.0")
}

#[test]
fn gravity_accelerates_a_ball_dropped_from_rest_downward_at_the_configured_rate() {
    let toml = format!("{}\n[[rules]]\nkind = \"gravity\"\nstrength = 1000.0\n", at_rest());
    for seed in 0..5 {
        let record = run(&toml, seed);
        let first = record.boundary_hits.first().expect("the ball falls onto the Boundary");

        // Speed grows linearly with time under constant acceleration: v = g * t.
        let expected = 1000.0 * first.time();
        assert!(
            (first.speed - expected).abs() < 1e-6,
            "seed {seed}: speed {} vs {expected}",
            first.speed
        );
        // y points down, so a falling ball lands on the lower half of the Boundary.
        assert!(first.angle.sin() > 0.0, "seed {seed}: angle {}", first.angle);
    }
}

#[test]
fn drag_reduces_speed_over_time_between_hits() {
    let toml = format!("{MINIMAL}\n[[rules]]\nkind = \"drag\"\nstrength = 0.2\n");
    let record = run(&toml, 7);

    assert!(record.boundary_hits.len() >= 3);
    for hit in &record.boundary_hits {
        // Drag of k per second decays speed exponentially: v = v0 * e^(-k t).
        let expected = 900.0 * (-0.2 * hit.time()).exp();
        assert!(
            (hit.speed - expected).abs() < 1e-6,
            "speed {} vs {expected} at {}s",
            hit.speed,
            hit.time()
        );
    }
    for pair in record.boundary_hits.windows(2) {
        assert!(pair[1].speed < pair[0].speed);
    }
}

fn speed_up(max_speed: f64) -> String {
    MINIMAL.replace("speed = 900.0", &format!("speed = 900.0\nmax_speed = {max_speed:?}"))
        + "\n[[rules]]\nkind = \"speed_up\"\npercent = 10.0\n"
}

#[test]
fn speed_up_increases_speed_by_the_configured_percentage_per_boundary_hit() {
    let record = run(&speed_up(1.0e6), 7);

    assert!(record.boundary_hits.len() >= 3);
    assert!((record.boundary_hits[0].speed - 900.0).abs() < 1e-9);
    for pair in record.boundary_hits.windows(2) {
        let ratio = pair[1].speed / pair[0].speed;
        assert!((ratio - 1.1).abs() < 1e-9, "ratio {ratio}");
    }
}

#[test]
fn speed_up_never_exceeds_max_speed() {
    let record = run(&speed_up(1500.0), 7);

    let speeds: Vec<f64> = record.boundary_hits.iter().map(|h| h.speed).collect();
    assert!(speeds.iter().all(|&s| s <= 1500.0 + 1e-9), "{speeds:?}");
    assert!(
        (speeds.last().unwrap() - 1500.0).abs() < 1e-9,
        "reaches max speed: {speeds:?}"
    );
    assert!(record.stats.max_speed <= 1500.0 + 1e-9);
}

#[test]
fn gravity_cannot_push_a_ball_past_max_speed() {
    let toml = MINIMAL.replace("speed = 900.0", "speed = 900.0\nmax_speed = 1000.0")
        + "\n[[rules]]\nkind = \"gravity\"\nstrength = 5000.0\n";
    let record = run(&toml, 3);

    assert!(record.stats.max_speed <= 1000.0 + 1e-9, "{}", record.stats.max_speed);
}

#[test]
fn change_colour_recolours_the_ball_on_each_boundary_hit() {
    let toml = format!("{MINIMAL}\n[[rules]]\nkind = \"change_colour\"\n");
    let record = run(&toml, 7);

    assert!(record.boundary_hits.len() >= 3);
    // The ball starts in colour 0, so the first hit must already change it.
    assert_ne!(record.boundary_hits[0].colour, 0);
    for pair in record.boundary_hits.windows(2) {
        assert_ne!(pair[0].colour, pair[1].colour);
    }
}

#[test]
fn without_change_colour_the_ball_keeps_its_colour() {
    let record = run(MINIMAL, 7);

    assert!(record.boundary_hits.iter().all(|h| h.colour == 0));
}

#[test]
fn at_max_speed_a_ball_never_ends_a_substep_outside_the_arena() {
    // 30000 units/s at 480 Hz is ~62 units per substep, over twice the ball's radius.
    let fast = MINIMAL.replace("speed = 900.0", "speed = 30000.0\nmax_speed = 30000.0")
        + "\n[[rules]]\nkind = \"speed_up\"\npercent = 50.0\n";
    for seed in 0..20 {
        let record = run(&fast, seed);
        assert!(record.boundary_hits.len() > 100, "seed {seed}");
        assert!(
            (record.stats.max_speed - 30000.0).abs() < 1e-6,
            "seed {seed}: at max speed"
        );
        assert!(
            record.stats.max_boundary_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_boundary_overshoot
        );
    }
}
