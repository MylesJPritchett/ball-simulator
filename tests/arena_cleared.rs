//! The arena_cleared End Condition, tested through the seam.

mod support;

use ball_simulator::render::Renderer;
use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{self, Ending, Simulation};
use support::{run, MINIMAL};

/// A `film` Rule that never grows back.
const FILM: &str = "\n[[rules]]\nkind = \"film\"\nregrow = 0.0\n";

const ARENA_CLEARED_END: &str = "\n[end_condition]\nkind = \"arena_cleared\"\n";

/// A ball that grows 20 units on every Boundary Hit over a Film, so it wipes
/// ever wider bands until, almost as big as the Arena, it wipes the last of
/// the Film at the rim.
fn growing_ball_wiping() -> String {
    MINIMAL.replace("time_cap = 5.0", "time_cap = 30.0")
        + "\n[[rules]]\nkind = \"grow\"\namount = 20.0\n"
        + FILM
        + ARENA_CLEARED_END
}

#[test]
fn arena_cleared_fires_on_the_substep_the_last_film_is_wiped() {
    for seed in 0..3 {
        let record = run(&growing_ball_wiping(), seed);

        assert_eq!(record.end.condition, "arena_cleared", "seed {seed}");
        let last = record.film_left.last().expect("a Film");
        assert_eq!(last.left, 0.0, "seed {seed}");
        assert_eq!(last.substep, record.end.substep, "seed {seed}");
    }
}

#[test]
fn the_final_chord_sounds_when_the_arena_is_cleared() {
    use ball_simulator::audio::{voices, SAMPLES_PER_SUBSTEP};
    let record = run(&growing_ball_wiping(), 1);
    assert_eq!(record.end.condition, "arena_cleared");
    assert!(record.ending.final_chord);
    let fired = (record.end.substep * SAMPLES_PER_SUBSTEP) as usize;

    let chord = voices(&record).iter().filter(|v| v.start == fired).count();
    assert!(chord >= 3, "chord has {chord} notes");
}

/// The pixels of `frame` drawn exactly in `colour`.
fn pixels_of(frame: &[u8], colour: [u8; 3]) -> usize {
    let (pixels, _) = frame.as_chunks::<4>();
    pixels.iter().filter(|p| p[..3] == colour).count()
}

#[test]
fn a_frame_of_a_cleared_arena_has_no_film_in_it() {
    let toml = growing_ball_wiping() + "\n[look]\ntrails = false\nglow = false\n";
    for seed in 0..3 {
        let scenario = support::scenario(&toml);
        // Without the flash, so the frozen frame shows the Arena as it is.
        let ending = Ending {
            flash_substeps: 0,
            ..sim::run(&scenario, seed).ending
        };
        let mut sim = Simulation::new(&scenario, seed);
        let mut renderer = Renderer::new(sim.scenario());
        let film = sim.palette().film;
        let first = pixels_of(renderer.draw(&sim, Some(&ending)), film);
        assert!(
            first > 10_000,
            "seed {seed}: {first} Film pixels at the start"
        );

        while !sim.is_frozen() {
            sim.step_frame();
        }
        let last = pixels_of(renderer.draw(&sim, Some(&ending)), film);
        assert_eq!(last, 0, "seed {seed}: Film pixels once cleared");
    }
}

#[test]
fn a_ball_that_never_moves_leaves_film_and_the_time_cap_hits() {
    let toml = MINIMAL.replace("speed = 900.0", "speed = 0.0") + FILM + ARENA_CLEARED_END;
    let record = run(&toml, 1);

    assert_eq!(record.end.condition, "time_cap");
    assert!(record.film_left.last().expect("a Film").left > 0.99);
}

#[test]
fn arena_cleared_takes_no_parameters() {
    let toml = format!("{MINIMAL}{FILM}{ARENA_CLEARED_END}fraction = 0.99\n");
    let err = Scenario::from_toml_str("broken", &toml)
        .expect_err("Scenario should be rejected")
        .to_string();
    assert!(
        err.contains("End Condition 'arena_cleared' has no parameter 'fraction'"),
        "{err}"
    );
}

#[test]
fn arena_cleared_needs_a_film_rule() {
    let toml = format!("{MINIMAL}{ARENA_CLEARED_END}");
    let err = Scenario::from_toml_str("broken", &toml)
        .expect_err("Scenario should be rejected")
        .to_string();
    assert!(
        err.contains("End Condition 'arena_cleared' needs a Rule that lays a Film (e.g. 'film')"),
        "{err}"
    );
}
