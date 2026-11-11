//! Rings (balls with a thickness) tested through the simulation seam.

mod support;

use support::{run, MINIMAL};

/// MINIMAL with its ball made a Ring `thickness` thick.
fn ring(thickness: &str) -> String {
    MINIMAL.replace(
        "speed = 900.0",
        &format!("speed = 900.0\nthickness = {thickness}"),
    )
}

#[test]
fn a_ring_without_a_peg_bounces_exactly_like_a_solid_ball() {
    for seed in 0..5 {
        let solid = run(MINIMAL, seed);
        let ring = run(&ring("10.0"), seed);

        assert!(solid.boundary_hits.len() >= 3, "seed {seed}: several hits");
        assert_eq!(ring.boundary_hits, solid.boundary_hits, "seed {seed}");
        assert_eq!(ring.final_balls[0].pos, solid.final_balls[0].pos);
        assert_eq!(ring.final_balls[0].thickness, Some(10.0), "it is a Ring");
        assert_eq!(solid.final_balls[0].thickness, None, "it is solid");
    }
}

#[test]
fn grow_and_shrink_change_a_rings_radius_but_not_its_thickness() {
    for (kind, amount, sign) in [("grow", 5.0, 1.0), ("shrink", 1.0, -1.0)] {
        let toml = format!(
            "{}\n[[rules]]\nkind = \"{kind}\"\namount = {amount:?}\n",
            ring("4.0")
        );
        let record = run(&toml, 3);

        let hits = record.boundary_hits.len();
        assert!(hits >= 3, "{kind}: expected several hits, got {hits}");
        let ball = &record.final_balls[0];
        assert!(
            (ball.radius - (30.0 + sign * amount * hits as f64)).abs() < 1e-9,
            "{kind}: radius {} after {hits} hits",
            ball.radius
        );
        assert_eq!(ball.thickness, Some(4.0), "{kind}: thickness unchanged");
    }
}

#[test]
fn a_ranged_thickness_resolves_per_seed_and_the_same_seed_gives_the_same_record() {
    let toml = format!(
        "{}\n[[rules]]\nkind = \"grow\"\namount = 5.0\n",
        ring("{ min = 4.0, max = 12.0 }")
    );
    let mut thicknesses = Vec::new();
    for seed in 0..10 {
        let record = run(&toml, seed);
        let param = &record.parameters[0];
        assert_eq!(param.field, "ball.thickness");
        let thickness = record.final_balls[0].thickness.expect("a Ring");
        assert_eq!(thickness, param.value.as_f64(), "seed {seed}");
        assert!((4.0..12.0).contains(&thickness), "seed {seed}: {thickness}");
        assert_eq!(record, run(&toml, seed), "seed {seed} is deterministic");
        thicknesses.push(thickness);
    }
    thicknesses.dedup();
    assert!(thicknesses.len() > 1, "Seeds resolve different thicknesses");
}
