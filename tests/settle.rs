//! Rings settling into the Boundary, or onto the Peg, until the Arena is
//! solid, through the simulation seam.

mod support;

use std::path::Path;

use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{self, Simulation, Surface};
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

/// A Ring (radius 450, thickness 20) round a Peg of 100 in an Arena of
/// radius 460 that shrinks by `shrink` on every Boundary Hit and settles onto
/// the Peg, until the Arena is solid.
fn shrinking_rings(shrink: f64) -> String {
    MINIMAL
        .replace("time_cap = 5.0", "time_cap = 300.0")
        .replace("radius = 460.0", "radius = 460.0\npeg_radius = 100.0")
        .replace("radius = 30.0", "radius = 450.0")
        .replace("speed = 900.0", "speed = 900.0\nthickness = 20.0")
        + &format!(
            "\n[[rules]]\nkind = \"shrink\"\namount = {shrink:?}\n\
             \n[[rules]]\nkind = \"settle\"\n\
             \n[end_condition]\nkind = \"solid\"\n"
        )
}

#[test]
fn rings_settle_onto_the_peg_layer_by_layer_until_it_fills_the_arena() {
    // Each settle adds 20 to the Peg: 120, 140, ..., 440. A Peg of 440 plus
    // one thickness reaches the 460 Arena, so a new Ring would already fit
    // it and could not move: the seventeenth settle makes the Arena solid.
    let scenario = scenario(&shrinking_rings(50.0));
    let mut sim = Simulation::new(&scenario, 1);
    let mut pegs = Vec::new();
    while !sim.is_frozen() {
        let before = sim.state().settles.len();
        sim.step();
        if sim.state().settles.len() > before {
            pegs.push(sim.state().arena.peg_radius.unwrap());
            assert_eq!(sim.state().arena.radius, 460.0, "the Arena stays put");
        }
    }
    let expected: Vec<f64> = (1..=17).map(|k| 100.0 + 20.0 * k as f64).collect();
    assert_eq!(pegs, expected);

    let record = sim.into_record();
    assert_eq!(record.settles.len(), 17);
    assert_eq!(record.end.condition, "solid");
    assert_eq!(
        record.end.substep, record.settles[16].substep,
        "on the last"
    );
    assert!(record.final_balls.is_empty(), "no Ring left to move");
    for (settle, peg) in record.settles.iter().zip(&expected) {
        assert_eq!(settle.surface, Surface::Peg);
        // Its layer runs from the old Peg's edge to the new one's.
        assert_eq!((settle.radius, settle.thickness), (*peg, 20.0));
    }
}

#[test]
fn an_overshooting_shrink_clamps_to_the_exact_fit_and_settles_that_substep() {
    // A Ring settles round a Peg of p once its radius is down to p + 20.
    // From 450, 450 - 7 * 50 = 100 overshoots the 120 that fits the Peg of
    // 100 on the seventh hit. The next Ring starts at 460: round the Peg of
    // 120, 460 - 7 * 50 = 110 overshoots 140 on the seventh; round 140,
    // 460 - 6 * 50 = 160 fits exactly on the sixth; round 160,
    // 460 - 6 * 50 = 160 overshoots 180 on the sixth.
    let rings = hits_per_ring(&shrinking_rings(50.0), 2);
    assert_eq!(rings.len(), 17);
    let hits: Vec<usize> = rings[..4].iter().map(Vec::len).collect();
    assert_eq!(hits, [7, 7, 6, 6]);
}

#[test]
fn a_shrink_past_the_whole_ring_still_clamps_and_settles() {
    // Shrinking by more than the Ring's radius would make a ball vanish;
    // round a Peg the hole stops at the Peg and every Ring settles on its
    // first hit.
    let rings = hits_per_ring(&shrinking_rings(500.0), 2);
    assert_eq!(rings.len(), 17);
    assert!(rings.iter().all(|hits| hits.len() == 1), "{rings:?}");
}

#[test]
fn each_new_ring_spawns_concentric_with_the_peg_with_its_outer_edge_on_the_boundary() {
    let scenario = scenario(&shrinking_rings(50.0));
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
        16,
        "a new Ring after every settle but the last"
    );
    for ring in &spawned {
        assert_eq!((ring.pos.x, ring.pos.y), (0.0, 0.0));
        assert_eq!(ring.radius, 460.0);
        assert_eq!(ring.thickness, Some(20.0));
        assert!((ring.vel.length() - 900.0).abs() < 1e-9, "{:?}", ring.vel);
    }
    let mut ids: Vec<u32> = spawned.iter().map(|r| r.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), 16, "each is a new ball");
    let mut headings: Vec<f64> = spawned.iter().map(|r| r.vel.angle()).collect();
    headings.dedup();
    assert!(headings.len() > 1, "heading in random directions");
}

#[test]
fn the_same_seed_gives_the_same_record_round_a_peg() {
    let toml = shrinking_rings(50.0) + "\n[[rules]]\nkind = \"change_colour\"\n";
    for seed in 0..3 {
        assert_eq!(run(&toml, seed), run(&toml, seed), "seed {seed}");
    }
    assert_ne!(run(&toml, 0), run(&toml, 1));
}

fn shipped(id: &str) -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join(format!("{id}.toml"));
    Scenario::from_file(&path).expect("shipped Scenario should be valid")
}

/// The shipped Ring Scenarios and the Surface their Rings settle against.
const RING_SCENARIOS: [(&str, Surface); 2] = [
    ("growing_rings", Surface::Boundary),
    ("shrinking_rings", Surface::Peg),
];

#[test]
fn a_settling_ring_becomes_its_layer_with_no_visible_jump() {
    // The layer a Ring settles into is drawn where the Ring was the substep
    // before, give or take the one grow (or shrink) of the hit that settled
    // it. (A frame can hold several hits at these speeds, so this is checked
    // substep by substep: across a frame the Ring may have grown twice.)
    for (id, surface) in RING_SCENARIOS {
        for seed in 1..=3 {
            let mut sim = Simulation::new(&shipped(id), seed);
            let amount = sim.scenario().rules[0].params["amount"]
                .as_float()
                .expect("the first Rule grows or shrinks by an amount");
            let tolerance = amount + 1e-9;
            let mut checked = 0;
            while !sim.is_frozen() {
                let before = sim.state().balls.clone();
                let settled = sim.state().settles.len();
                sim.step();
                let Some(layer) = sim.state().settles.get(settled) else {
                    continue;
                };
                let [ring] = before.as_slice() else {
                    panic!("{id} seed {seed}: one Ring at a time, got {before:?}");
                };
                assert_eq!(layer.surface, surface, "{id} seed {seed}");
                assert_eq!(Some(layer.thickness), ring.thickness, "{id} seed {seed}");
                // A layer is concentric with the Arena.
                let off_centre = ring.pos.length();
                let resized = (ring.radius - layer.radius).abs();
                assert!(
                    off_centre <= tolerance && resized <= tolerance,
                    "{id} seed {seed}, settle {settled}: Ring drawn {off_centre} off \
                     centre at radius {}, layer at radius {} (tolerance {tolerance})",
                    ring.radius,
                    layer.radius
                );
                checked += 1;
            }
            assert!(
                checked > 10,
                "{id} seed {seed}: only {checked} settles seen"
            );
        }
    }
}

#[test]
fn each_ring_takes_less_time_to_settle_than_the_last() {
    // As the Arena fills (or the Peg grows), each Ring has less far to go:
    // the substeps between successive settles never grow, and the last cycle
    // is a small fraction of the first.
    for (id, _) in RING_SCENARIOS {
        for seed in 1..=3 {
            let record = sim::run(&shipped(id), seed);
            let mut previous = 0;
            let cycles: Vec<u64> = record
                .settles
                .iter()
                .map(|settle| {
                    let cycle = settle.substep - previous;
                    previous = settle.substep;
                    cycle
                })
                .collect();
            assert!(cycles.len() > 10, "{id} seed {seed}: {cycles:?}");
            assert!(
                cycles.windows(2).all(|pair| pair[1] <= pair[0]),
                "{id} seed {seed}: a cycle got longer: {cycles:?}"
            );
            let (first, last) = (cycles[0], cycles[cycles.len() - 1]);
            assert!(last * 10 < first, "{id} seed {seed}: {cycles:?}");
        }
    }
}
