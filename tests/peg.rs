//! A Peg (a fixed solid disc at the Arena's centre) and the Ring moving
//! around it, tested through the simulation seam.

mod support;

use ball_simulator::sim::{Simulation, Surface};
use support::{run, scenario, MINIMAL};

/// MINIMAL with a Peg `peg` in radius and its ball made a Ring `radius` in
/// radius and `thickness` thick. The Ring's centre can be at most
/// `radius - thickness - peg` from the Arena's centre before its inner edge
/// meets the Peg, and `460 - radius` before its outer edge meets the
/// Boundary.
fn around_a_peg(peg: f64, radius: f64, thickness: f64) -> String {
    MINIMAL
        .replace(
            "radius = 460.0",
            &format!("radius = 460.0\npeg_radius = {peg:?}"),
        )
        .replace("radius = 30.0", &format!("radius = {radius:?}"))
        .replace(
            "speed = 900.0",
            &format!("speed = 900.0\nthickness = {thickness:?}"),
        )
}

#[test]
fn a_ring_around_a_peg_hits_the_peg_with_its_inner_edge() {
    // Hole 180 round a Peg of 100: the centre can wander 80 from the middle,
    // well short of the 260 the Boundary allows, so every hit is on the Peg.
    let toml = around_a_peg(100.0, 200.0, 20.0);
    for seed in 0..5 {
        let mut sim = Simulation::new(&scenario(&toml), seed);
        let mut checked = 0;
        while !sim.is_frozen() {
            let before = sim.boundary_hits().len();
            sim.step();
            let Some(hit) = sim.boundary_hits().get(before) else {
                continue;
            };
            let ball = &sim.state().balls[0];
            assert_eq!(hit.surface, Surface::Peg, "seed {seed}");
            // The hole's edge touches the Peg on the side away from the
            // Ring's centre: there, the Peg's surface faces the Ring's centre
            // straight on.
            let dist = ball.pos.length();
            assert!((dist - 80.0).abs() < 1e-9, "seed {seed}: centre at {dist}");
            let contact = ball.pos * (-100.0 / dist);
            assert!(
                (contact.angle() - hit.angle).abs() < 1e-9,
                "seed {seed}: hit at {} but touching at {}",
                hit.angle,
                contact.angle()
            );
            checked += 1;
        }
        assert!(checked >= 10, "seed {seed}: only {checked} Peg hits");
    }
}

#[test]
fn a_rings_inner_edge_never_overlaps_the_peg() {
    // A fast Ring round a Peg leaving it little room.
    let toml = around_a_peg(150.0, 200.0, 20.0).replace("speed = 900.0", "speed = 5000.0");
    for seed in 0..20 {
        let record = run(&toml, seed);
        assert!(record.boundary_hits.len() > 50, "seed {seed}");
        assert!(
            record.stats.max_peg_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_peg_overshoot
        );
    }
}

#[test]
fn on_hit_rules_fire_on_peg_hits_as_on_boundary_hits() {
    let toml = around_a_peg(100.0, 200.0, 20.0)
        + "\n[[rules]]\nkind = \"shrink\"\namount = 1.0\n"
        + "\n[[rules]]\nkind = \"change_colour\"\n"
        + "\n[[rules]]\nkind = \"play_note\"\n";
    for seed in 0..5 {
        let record = run(&toml, seed);
        let hits = &record.boundary_hits;
        assert!(hits.len() >= 10, "seed {seed}");
        assert!(hits.iter().all(|hit| hit.surface == Surface::Peg));
        assert!(hits.iter().all(|hit| hit.plays_note), "every hit plays");
        let ball = &record.final_balls[0];
        assert_eq!(ball.radius, 200.0 - hits.len() as f64, "seed {seed}");
        assert_eq!(ball.colour, hits.len() as u32, "seed {seed}");
    }
}

#[test]
fn a_ring_round_a_peg_hits_whichever_its_edges_meet_first() {
    // Hole 380 round a Peg of 60: its inner edge meets the Peg 320 out, its
    // outer edge the Boundary 60 out, so this big Ring only hits the
    // Boundary.
    let record = run(&around_a_peg(60.0, 400.0, 20.0), 1);
    assert!(record.boundary_hits.len() >= 10);
    assert!(record
        .boundary_hits
        .iter()
        .all(|hit| hit.surface == Surface::Boundary));
    assert!(record.stats.max_boundary_overshoot <= 1e-9);
}

#[test]
fn the_same_seed_gives_the_same_record_round_a_peg() {
    let toml = around_a_peg(100.0, 200.0, 20.0) + "\n[[rules]]\nkind = \"shrink\"\namount = 1.0\n";
    for seed in 0..5 {
        assert_eq!(run(&toml, seed), run(&toml, seed), "seed {seed}");
    }
}

#[test]
fn a_ring_starts_clear_of_the_peg() {
    // The centre can be at most 80 from the middle; the Boundary alone would
    // let it start up to 130 out.
    let toml = around_a_peg(100.0, 200.0, 20.0);
    for seed in 0..50 {
        let sim = Simulation::new(&scenario(&toml), seed);
        let dist = sim.state().balls[0].pos.length();
        assert!(dist <= 80.0, "seed {seed}: starts {dist} out");
    }
}

#[test]
fn a_shrinking_rings_inner_edge_never_overlaps_the_peg() {
    // Each Peg hit shrinks the hole by 0.5, from 180 down towards the Peg's
    // 100, which it would reach on hit 160; the run stops after a second.
    let toml = around_a_peg(100.0, 200.0, 20.0)
        .replace("speed = 900.0", "speed = 3000.0")
        .replace("time_cap = 5.0", "time_cap = 1.0")
        + "\n[[rules]]\nkind = \"shrink\"\namount = 0.5\n";
    for seed in 0..20 {
        let record = run(&toml, seed);
        let hits = record.boundary_hits.len();
        assert!(hits > 10 && hits < 160, "seed {seed}: {hits} hits");
        assert!(
            record.stats.max_peg_overshoot <= 1e-9,
            "seed {seed}: overshoot {}",
            record.stats.max_peg_overshoot
        );
    }
}
