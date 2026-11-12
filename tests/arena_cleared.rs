//! The arena_cleared End Condition, tested through the seam.

mod support;

use ball_simulator::render::Renderer;
use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{self, Ending, Finale, Simulation};
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
fn a_cleared_arena_ends_with_a_shimmer_not_a_flash() {
    for seed in 0..3 {
        let record = run(&growing_ball_wiping(), seed);
        assert_eq!(record.end.condition, "arena_cleared", "seed {seed}");
        assert_eq!(record.ending.finale, Finale::Shimmer, "seed {seed}");
        assert_eq!(record.ending.flash_substeps, 0, "seed {seed}");
        assert!(record.ending.final_chord, "seed {seed}");
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

/// The frames from ten before the action freezes to the cut of the run of
/// `toml`, drawn with its Ending's Finale replaced by `finale` (trails off,
/// so the frames before need not be drawn), and the index of the first frame
/// of the hold among them.
fn hold_frames(toml: &str, seed: u64, finale: Finale) -> (Vec<Vec<u8>>, usize) {
    const BEFORE: u64 = 10;
    let scenario = support::scenario(toml);
    let ending = Ending {
        finale,
        ..sim::run(&scenario, seed).ending
    };
    let freeze = ending.freeze_substep.div_ceil(sim::SUBSTEPS_PER_FRAME);
    let mut sim = Simulation::new(&scenario, seed);
    for _ in 0..freeze - BEFORE {
        sim.step_frame();
    }
    let mut renderer = Renderer::new(sim.scenario());
    let frames = (freeze - BEFORE..ending.frames())
        .map(|k| {
            if k > freeze - BEFORE {
                sim.step_frame();
            }
            renderer.draw(&sim, Some(&ending)).to_vec()
        })
        .collect();
    (frames, BEFORE as usize)
}

#[test]
fn a_bright_band_sweeps_once_across_the_cleared_arena_during_the_hold() {
    use ball_simulator::layout;
    let toml = growing_ball_wiping() + "\n[look]\ntrails = false\nglow = false\n";
    let (shimmered, freeze) = hold_frames(&toml, 1, Finale::Shimmer);
    let (plain, _) = hold_frames(&toml, 1, Finale::None);
    let arena = layout::arena();
    let row = arena.center_y.round() as usize;
    let left = (arena.center_x - arena.radius).ceil() as usize;
    let right = (arena.center_x + arena.radius).floor() as usize;

    // Where the band crosses the Arena's middle row: the mean x of the
    // pixels it brightens, or `None` when it is not on the row.
    let band = |k: usize| {
        let lit: Vec<usize> = (left..=right)
            .filter(|&x| {
                let i = (row * layout::WIDTH as usize + x) * 4;
                let sum = |f: &[u8]| f[i..i + 3].iter().map(|&c| c as u32).sum::<u32>();
                let (s, p) = (sum(&shimmered[k]), sum(&plain[k]));
                assert!(s >= p, "frame {k}: the shimmer darkens x = {x}");
                s > p
            })
            .collect();
        (!lit.is_empty()).then(|| lit.iter().sum::<usize>() as f32 / lit.len() as f32)
    };

    for k in 0..freeze {
        assert_eq!(
            shimmered[k], plain[k],
            "frame {k}: a shimmer before the hold"
        );
    }
    let sweep: Vec<f32> = (freeze..shimmered.len()).filter_map(band).collect();
    assert!(sweep.len() > 30, "the band shows on {} frames", sweep.len());
    assert!(
        sweep.windows(2).all(|w| w[1] > w[0]),
        "the band moves one way across the Arena: {sweep:?}"
    );
    let (first, last) = (sweep[0], *sweep.last().unwrap());
    assert!(
        first < arena.center_x - 0.5 * arena.radius,
        "starts at {first}"
    );
    assert!(last > arena.center_x + 0.5 * arena.radius, "ends at {last}");
}
