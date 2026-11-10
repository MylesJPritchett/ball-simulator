//! Escapes through a Gap in the Boundary, tested through the seam: the `gap`
//! Rule and the `escaped` End Condition.

mod support;

use std::f64::consts::{PI, TAU};

use ball_simulator::render::layout;
use ball_simulator::sim::end_condition::ESCAPE_PLAY_OUT_MAX_SECONDS;
use ball_simulator::sim::{Ball, Simulation, SUBSTEP_HZ};
use support::{run, scenario, MINIMAL};

/// MINIMAL with a Gap `width` degrees wide turning at `rotation_speed`
/// degrees per second.
fn with_gap(width: f64, rotation_speed: f64) -> String {
    format!(
        "{MINIMAL}\n[[rules]]\nkind = \"gap\"\nwidth = {width:?}\nrotation_speed = {rotation_speed:?}\n"
    )
}

#[test]
fn a_ball_reaching_the_gap_passes_through_without_a_boundary_hit() {
    // A Gap all the way round: wherever the ball reaches the Boundary, it is
    // in the Gap.
    for seed in 0..5 {
        let record = run(&with_gap(360.0, 0.0), seed);

        assert!(record.boundary_hits.is_empty(), "seed {seed}");
        let ball = &record.final_balls[0];
        assert!(
            ball.pos.length() - ball.radius > 460.0,
            "seed {seed}: the ball carried on out of the Arena"
        );
    }
}

/// The Gap's starting centre angle for a Seed.
fn gap_angle(toml: &str, seed: u64) -> f64 {
    Simulation::new(&scenario(toml), seed).state().arena.gaps[0].angle
}

/// Signed angular distance from `b` to `a`, in [-PI, PI).
fn angle_between(a: f64, b: f64) -> f64 {
    (a - b + PI).rem_euclid(TAU) - PI
}

#[test]
fn the_ball_bounces_off_the_boundary_outside_the_gap_until_it_falls_through() {
    // Gravity keeps the bounces from settling into an orbit that never
    // reaches the Gap, and the Gap sweeps round to wherever the ball can reach.
    let toml = with_gap(60.0, 30.0).replace("time_cap = 5.0", "time_cap = 60.0")
        + "\n[[rules]]\nkind = \"gravity\"\nstrength = 800.0\n";
    let mut hits = 0;
    for seed in 0..10 {
        let start = gap_angle(&toml, seed);
        let record = run(&toml, seed);

        hits += record.boundary_hits.len();
        for hit in &record.boundary_hits {
            // The Gap turns before balls move on every substep.
            let turned = 30f64.to_radians() * hit.substep as f64 / SUBSTEP_HZ as f64;
            let gap = start + turned;
            assert!(
                angle_between(hit.angle, gap).abs() > 30f64.to_radians(),
                "seed {seed}: a Boundary Hit at {} in the Gap at {gap}",
                hit.angle
            );
        }
        let ball = &record.final_balls[0];
        assert!(ball.pos.length() - ball.radius > 460.0, "seed {seed}");
        // Only balls the Boundary still holds count towards the overshoot.
        assert!(
            record.stats.max_boundary_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_boundary_overshoot
        );
    }
    assert!(hits >= 10, "the Boundary outside the Gap is hit: {hits}");
}

#[test]
fn a_rotating_gap_turns_at_its_rotation_speed() {
    let toml = with_gap(30.0, 45.0);
    let mut sim = Simulation::new(&scenario(&toml), 1);
    let start = sim.state().arena.gaps[0].angle;
    for _ in 0..SUBSTEP_HZ {
        sim.step();
    }
    let turned = sim.state().arena.gaps[0].angle - start;

    assert!(
        (turned - 45f64.to_radians()).abs() < 1e-9,
        "turned {turned}"
    );
}

#[test]
fn a_gap_can_start_at_a_given_angle() {
    // 90 degrees is straight down (y points down).
    let toml = with_gap(30.0, 0.0) + "start_angle = 90.0\n";
    for seed in 0..3 {
        let angle = gap_angle(&toml, seed);
        assert!((angle - 90f64.to_radians()).abs() < 1e-12, "seed {seed}");
    }
}

#[test]
fn without_a_start_angle_the_seed_places_the_gap() {
    let toml = with_gap(30.0, 0.0);
    assert_ne!(gap_angle(&toml, 1), gap_angle(&toml, 2));
}

const ESCAPED_END: &str = "\n[end_condition]\nkind = \"escaped\"\n";

/// Whether a ball is fully outside the 460-unit Arena.
fn outside_the_arena(ball: &Ball) -> bool {
    ball.pos.length() - ball.radius >= 460.0
}

/// Whether a ball is drawn wholly off the Video frame.
fn outside_the_frame(ball: &Ball) -> bool {
    let arena = layout::arena();
    let scale = (arena.radius / 460.0) as f64;
    let x = arena.center_x as f64 + ball.pos.x * scale;
    let y = arena.center_y as f64 + ball.pos.y * scale;
    let r = ball.radius * scale;
    x + r <= 0.0 || y + r <= 0.0 || x - r >= layout::WIDTH as f64 || y - r >= layout::HEIGHT as f64
}

#[test]
fn escaped_fires_once_the_ball_is_fully_outside_the_arena() {
    let toml = with_gap(360.0, 0.0) + ESCAPED_END;
    for seed in 0..5 {
        let mut sim = Simulation::new(&scenario(&toml), seed);
        while sim.end().is_none() {
            assert!(
                !outside_the_arena(&sim.state().balls[0]),
                "seed {seed}: fully out before escaped fired"
            );
            sim.step();
        }

        assert_eq!(sim.end().unwrap().condition, "escaped", "seed {seed}");
        assert!(outside_the_arena(&sim.state().balls[0]), "seed {seed}");
    }
}

#[test]
fn the_escape_plays_out_until_the_ball_has_left_the_frame() {
    let toml = with_gap(360.0, 0.0) + ESCAPED_END;
    for seed in 0..5 {
        let mut sim = Simulation::new(&scenario(&toml), seed);
        while !sim.is_frozen() {
            assert!(
                !outside_the_frame(&sim.state().balls[0]),
                "seed {seed}: off the frame before the freeze"
            );
            sim.step();
        }
        let ending = sim.ending().unwrap();

        assert!(outside_the_frame(&sim.state().balls[0]), "seed {seed}");
        assert_eq!(ending.freeze_substep, sim.substep());
        assert!(ending.freeze_substep > ending.start_substep, "a Play-out");
        assert!(ending.final_chord && ending.flash_substeps > 0);
    }
}

#[test]
fn a_ball_that_never_leaves_the_frame_freezes_at_the_play_out_maximum() {
    // At 10 units/s the escaped ball needs over 10 s to get from the Arena's
    // edge to the frame's.
    let toml = MINIMAL.replace("speed = 900.0", "speed = 10.0")
        + "\n[[rules]]\nkind = \"gap\"\nwidth = 360.0\nrotation_speed = 0.0\n"
        + ESCAPED_END;
    let toml = toml.replace("time_cap = 5.0", "time_cap = 120.0");
    let record = run(&toml, 1);
    let ending = record.ending;

    assert_eq!(record.end.condition, "escaped");
    assert_eq!(
        ending.freeze_substep - ending.start_substep,
        (ESCAPE_PLAY_OUT_MAX_SECONDS * SUBSTEP_HZ as f64).round() as u64
    );
}

#[test]
fn the_escape_flashes_and_sounds_its_final_chord_at_the_freeze_not_when_it_fires() {
    let toml = with_gap(360.0, 0.0) + ESCAPED_END;
    for seed in 0..5 {
        let ending = run(&toml, seed).ending;

        assert!(ending.freeze_substep > ending.start_substep, "a Play-out");
        assert_eq!(ending.accent_substep, ending.freeze_substep, "seed {seed}");
        assert_eq!(ending.flash_at(ending.start_substep), 0.0, "seed {seed}");
        assert_eq!(ending.flash_at(ending.freeze_substep - 1), 0.0);
        assert_eq!(ending.flash_at(ending.freeze_substep), 1.0);
    }
}
