//! Rings settling into the Boundary until the Arena is solid, through the
//! simulation seam.

mod support;

use ball_simulator::sim::{Simulation, Surface};
use support::{run, scenario, MINIMAL};

/// A Ring (radius 30, thickness 20) in an Arena of radius 460 that grows by
/// `grow` on every Boundary Hit and settles, until the Arena is solid.
fn growing_rings(grow: f64) -> String {
    MINIMAL
        .replace("time_cap = 5.0", "time_cap = 300.0")
        .replace("speed = 900.0", "speed = 900.0\nthickness = 20.0")
        + &format!(
            "\n[[rules]]\nkind = \"grow\"\namount = {grow:?}\n\
             \n[[rules]]\nkind = \"settle\"\n\
             \n[end_condition]\nkind = \"solid\"\n"
        )
}

#[test]
fn rings_settle_layer_by_layer_until_the_arena_is_solid() {
    // Each settle takes 20 off the Arena radius: 460, 440, ..., 40, 20. The
    // Ring starts at radius 30, so once the Arena is 20 no Ring fits: the
    // twenty-second settle makes it solid.
    let scenario = scenario(&growing_rings(50.0));
    let mut sim = Simulation::new(&scenario, 1);
    let mut radii = Vec::new();
    while !sim.is_frozen() {
        let before = sim.state().settles.len();
        sim.step();
        if sim.state().settles.len() > before {
            radii.push(sim.state().arena.radius);
        }
    }
    let expected: Vec<f64> = (1..=22).map(|k| 460.0 - 20.0 * k as f64).collect();
    assert_eq!(radii, expected);

    let record = sim.into_record();
    assert_eq!(record.settles.len(), 22);
    assert_eq!(record.end.condition, "solid");
    assert_eq!(
        record.end.substep, record.settles[21].substep,
        "on the last"
    );
    assert!(record.final_balls.is_empty(), "no Ring left to move");
}

/// For each settle, the Boundary Hits of the Ring that settled.
fn hits_per_ring(toml: &str, seed: u64) -> Vec<Vec<u64>> {
    let record = run(toml, seed);
    let mut rings: Vec<(u32, Vec<u64>)> = Vec::new();
    for hit in &record.boundary_hits {
        match rings.last_mut() {
            Some((id, hits)) if *id == hit.ball_id => hits.push(hit.substep),
            _ => rings.push((hit.ball_id, vec![hit.substep])),
        }
    }
    assert_eq!(rings.len(), record.settles.len(), "every Ring settled");
    for ((_, hits), settle) in rings.iter().zip(&record.settles) {
        assert_eq!(
            *hits.last().unwrap(),
            settle.substep,
            "a Ring settles on the substep its last hit grew it to fit"
        );
    }
    rings.into_iter().map(|(_, hits)| hits).collect()
}

#[test]
fn an_overshooting_grow_clamps_to_the_exact_fit_and_settles_that_substep() {
    // 30 + 9 * 50 = 480 would overshoot the 460 Arena: the ninth hit clamps
    // the Ring to fit and it settles. In the 440 Arena it takes nine hits
    // too, and in the 420 Arena 30 + 8 * 50 = 430 overshoots on the eighth.
    let rings = hits_per_ring(&growing_rings(50.0), 2);
    assert_eq!(rings.len(), 22);
    assert_eq!(rings[0].len(), 9);
    assert_eq!(rings[1].len(), 9);
    assert_eq!(rings[2].len(), 8);
}

#[test]
fn a_grow_to_the_exact_fit_settles_that_substep() {
    // 30 + 10 * 43 = 460 fits the Arena exactly.
    let rings = hits_per_ring(&growing_rings(43.0), 2);
    assert_eq!(rings[0].len(), 10);
}

#[test]
fn each_new_ring_spawns_at_the_centre_with_the_starting_radius_and_speed() {
    let scenario = scenario(&growing_rings(50.0));
    let mut sim = Simulation::new(&scenario, 3);
    let mut spawned = Vec::new();
    while !sim.is_frozen() {
        let before = sim.state().settles.len();
        sim.step();
        if sim.state().settles.len() > before {
            spawned.extend(sim.state().balls.iter().cloned());
        }
    }
    assert_eq!(
        spawned.len(),
        21,
        "a new Ring after every settle but the last"
    );
    for ring in &spawned {
        assert_eq!((ring.pos.x, ring.pos.y), (0.0, 0.0));
        assert_eq!(ring.radius, 30.0);
        assert_eq!(ring.thickness, Some(20.0));
        assert!((ring.vel.length() - 900.0).abs() < 1e-9, "{:?}", ring.vel);
    }
    let mut ids: Vec<u32> = spawned.iter().map(|r| r.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), 21, "each is a new ball");
    let mut headings: Vec<f64> = spawned.iter().map(|r| r.vel.angle()).collect();
    headings.dedup();
    assert!(headings.len() > 1, "heading in random directions");
}

#[test]
fn settles_are_on_the_record_with_substep_colour_and_surface() {
    let toml = growing_rings(50.0) + "\n[[rules]]\nkind = \"change_colour\"\n";
    let record = run(&toml, 4);
    assert_eq!(record.settles.len(), 22);
    let mut last = 0;
    for settle in &record.settles {
        assert!(settle.substep > last, "in order");
        last = settle.substep;
        assert_eq!(settle.surface, Surface::Boundary);
        // The Ring's colour is that of its last hit, which settled it.
        let hit = record
            .boundary_hits
            .iter()
            .rfind(|hit| hit.substep == settle.substep)
            .expect("settled on a hit");
        assert_eq!(settle.colour, hit.colour);
    }
    let mut colours: Vec<u32> = record.settles.iter().map(|s| s.colour).collect();
    colours.dedup();
    assert!(colours.len() > 1, "layers keep their own colours");
}

#[test]
fn the_same_seed_gives_the_same_record() {
    let toml = growing_rings(50.0);
    for seed in 0..3 {
        assert_eq!(run(&toml, seed), run(&toml, seed), "seed {seed}");
    }
    assert_ne!(run(&toml, 0), run(&toml, 1));
}
