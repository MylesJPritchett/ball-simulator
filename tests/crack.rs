//! The Boundary cracking and shattering, through the sim seam: the `crack`
//! Rule with the `escaped` End Condition.

mod support;

use ball_simulator::sim::Simulation;
use support::{scenario, MINIMAL};

const MAX_SPEED: f64 = 2000.0;

/// MINIMAL speeding up 10% a hit to a max speed of 2000, cracking the
/// Boundary, and ending on `escaped`.
fn cracking() -> String {
    MINIMAL
        .replace("time_cap = 5.0", "time_cap = 30.0")
        .replace("speed = 900.0", "speed = 900.0\nmax_speed = 2000.0")
        + "\n[[rules]]\nkind = \"speed_up\"\npercent = 10.0\n\
           \n[[rules]]\nkind = \"crack\"\n\
           \n[end_condition]\nkind = \"escaped\"\n"
}

/// Whether a Boundary Hit's speed is the max speed (which the simulation
/// caps speeds at, give or take rounding).
fn at_max_speed(speed: f64) -> bool {
    speed >= MAX_SPEED * (1.0 - 1e-9)
}

#[test]
fn below_max_speed_the_ball_is_reflected_and_at_it_the_boundary_shatters() {
    for seed in 0..5 {
        let mut sim = Simulation::new(&scenario(&cracking()), seed);
        let mut reflected = 0;
        let shattered_at = loop {
            let hits_before = sim.boundary_hits().len();
            sim.step();
            let Some(hit) = sim.boundary_hits()[hits_before..].first().cloned() else {
                assert!(sim.end().is_none(), "seed {seed}: ended before shattering");
                continue;
            };
            let ball = &sim.state().balls[0];
            if at_max_speed(hit.speed) {
                assert!(ball.vel.dot(ball.pos) > 0.0, "seed {seed}: reflected");
                break sim.boundary_hits().len();
            }
            assert!(
                ball.vel.dot(ball.pos) < 0.0 && !ball.escaping,
                "seed {seed}: not reflected at {}",
                hit.speed
            );
            assert!(sim.state().arena.gaps.is_empty(), "seed {seed}: opened");
            reflected += 1;
        };
        assert!(reflected >= 5, "seed {seed}: {reflected} hits before");

        while !sim.is_frozen() {
            sim.step();
        }

        assert_eq!(
            sim.boundary_hits().len(),
            shattered_at,
            "seed {seed}: nothing is reflected after the shatter"
        );
        assert_eq!(sim.end().unwrap().condition, "escaped", "seed {seed}");
        let ball = &sim.state().balls[0];
        assert!(ball.escaping && ball.pos.length() > 460.0, "seed {seed}");
        assert!(sim.state().arena.solid_arcs().is_empty(), "seed {seed}");
    }
}

#[test]
fn every_boundary_hit_cracks_the_boundary_where_it_lands_harder_the_faster() {
    let record_hits = |seed| {
        let mut sim = Simulation::new(&scenario(&cracking()), seed);
        while !sim.is_frozen() {
            sim.step();
        }
        (
            sim.boundary_hits().to_vec(),
            sim.state().arena.cracks.clone(),
        )
    };
    for seed in 0..5 {
        let (hits, cracks) = record_hits(seed);

        assert_eq!(cracks.len(), hits.len(), "seed {seed}");
        for (hit, crack) in hits.iter().zip(&cracks) {
            assert_eq!(crack.angle, hit.angle, "seed {seed}");
            let expected = (hit.speed / MAX_SPEED).min(1.0);
            assert!(
                (crack.size - expected).abs() < 1e-9,
                "seed {seed}: size {} at {} units/s",
                crack.size,
                hit.speed
            );
        }
        // Speeding up every hit, each crack is bigger than the one before.
        assert!(cracks
            .windows(2)
            .all(|w| w[1].size > w[0].size || w[1].size == 1.0));
    }
}
