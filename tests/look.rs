//! The look of a Video as far as the simulation seam can see it: palette
//! choice, visual toggles and the live Counter.

mod support;

use std::collections::BTreeSet;

use ball_simulator::look::{palette, Counter};
use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{SimRecord, Simulation};
use support::{run, scenario, MINIMAL};

/// MINIMAL with a `[look]` table appended (it ends with the `[ball]` table).
fn with_look(look: &str) -> String {
    format!("{MINIMAL}\n[look]\n{look}\n")
}

#[test]
fn palette_varies_by_seed() {
    let names: BTreeSet<String> = (0..20).map(|seed| run(MINIMAL, seed).palette).collect();

    assert!(names.len() >= 3, "only {names:?} across 20 Seeds");
    for name in &names {
        assert!(palette::by_name(name).is_some(), "unknown palette {name}");
    }
}

#[test]
fn same_seed_gives_same_palette() {
    assert_eq!(run(MINIMAL, 5).palette, run(MINIMAL, 5).palette);
}

#[test]
fn pinned_palette_is_used_for_every_seed() {
    let toml = with_look("palette = \"neon\"");
    for seed in 0..20 {
        assert_eq!(run(&toml, seed).palette, "neon", "seed {seed}");
    }
}

#[test]
fn unknown_pinned_palette_is_rejected() {
    let toml = format!(
        "{}\n[end_condition]\nkind = \"arena_filled\"\n",
        with_look("palette = \"plaid\"")
    );
    let err = Scenario::from_toml_str("broken", &toml)
        .expect_err("unknown palette should be rejected")
        .to_string();
    assert!(err.contains("unknown palette 'plaid'"), "{err}");
}

#[test]
fn choosing_a_palette_does_not_change_the_simulation() {
    // Palette choice must not draw from the simulation's RNG: the shipped
    // snapshot for minimal seed 1 stays the same.
    let pinned = with_look("palette = \"neon\"");
    assert_eq!(
        run(MINIMAL, 1).final_state_hash,
        run(&pinned, 1).final_state_hash
    );
}

/// Steps a Scenario frame by frame, collecting the live Counter reading and
/// the substep it was read at; also returns the finished record.
fn read_counter_every_frame(toml: &str, seed: u64) -> (Vec<(u64, u64)>, SimRecord) {
    let scenario = scenario(toml);
    let counter = scenario.look.counter;
    let mut sim = Simulation::new(&scenario, seed);
    let mut readings = Vec::new();
    loop {
        readings.push((counter.read(&sim), sim.substep()));
        if sim.is_frozen() {
            break;
        }
        sim.step_frame();
    }
    (readings, sim.into_record())
}

#[test]
fn boundary_hit_counter_matches_the_record_on_every_frame() {
    let (readings, record) = read_counter_every_frame(MINIMAL, 7);

    assert!(record.boundary_hits.len() >= 5);
    assert_eq!(
        readings.last().unwrap().0,
        record.boundary_hits.len() as u64
    );
    for (value, substep) in readings {
        let expected = record
            .boundary_hits
            .iter()
            .filter(|h| h.substep <= substep)
            .count() as u64;
        assert_eq!(value, expected, "at substep {substep}");
    }
}

#[test]
fn ball_count_counter_matches_the_record_on_every_frame() {
    let (readings, record) = read_counter_every_frame(&with_look("counter = \"ball_count\""), 3);

    for (value, substep) in readings {
        let expected = record
            .ball_count
            .iter()
            .rev()
            .find(|s| s.substep <= substep)
            .unwrap()
            .count as u64;
        assert_eq!(value, expected, "at substep {substep}");
    }
}

#[test]
fn speed_counter_shows_the_ball_speed() {
    // No Rules: the ball keeps its initial speed of 900 forever.
    let (readings, _) = read_counter_every_frame(&with_look("counter = \"speed\""), 2);

    for (value, substep) in readings {
        assert_eq!(value, 900, "at substep {substep}");
    }
}

#[test]
fn trails_and_glow_are_on_by_default_and_can_be_switched_off() {
    let default = scenario(MINIMAL).look;
    assert!(default.trails && default.glow);

    let off = scenario(&with_look("trails = false\nglow = false")).look;
    assert!(!off.trails && !off.glow);
}

/// WCAG contrast ratio between two sRGB colours (1 to 21).
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |c: [u8; 3]| {
        let lin = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[test]
fn the_film_stands_out_from_the_cleared_arena_in_every_palette() {
    // The cleared Arena is the background. 2.5:1 is well clear of the dark
    // backgrounds while leaving the balls room to stand out from the Film.
    for p in palette::PALETTES {
        let ratio = contrast(p.film, p.background);
        assert!(ratio >= 2.5, "{}: Film contrast {ratio:.2}", p.name);
    }
}

#[test]
fn unknown_counter_is_rejected() {
    let err = Scenario::from_toml_str("broken", &with_look("counter = \"vibes\""))
        .expect_err("unknown Counter should be rejected")
        .to_string();
    assert!(err.contains("vibes"), "{err}");
}

/// Steps a Scenario frame by frame, collecting the Counter text as the
/// renderer draws it, with the Film left at that frame.
fn counter_text_every_frame(toml: &str, seed: u64) -> Vec<(String, f64)> {
    let scenario = scenario(toml);
    let counter = scenario.look.counter;
    let mut sim = Simulation::new(&scenario, seed);
    let mut readings = Vec::new();
    loop {
        let left = sim.state().film.as_ref().map_or(0.0, |f| f.left());
        readings.push((counter.text(&sim), left));
        if sim.is_frozen() {
            break;
        }
        sim.step_frame();
    }
    readings
}

/// A big ball wiping a Film for `seconds`, with the Film left Counter.
fn wiping_with_film_left_counter(seconds: f64) -> String {
    MINIMAL
        .replace("time_cap = 5.0", &format!("time_cap = {seconds:?}"))
        .replace("radius = 30.0", "radius = 300.0")
        + "\n[[rules]]\nkind = \"film\"\nregrow = 0.0\n"
        + "\n[look]\ncounter = \"film_left\"\n"
}

#[test]
fn film_left_counter_counts_down_from_100_percent_to_0_percent() {
    // The ball grows on every Boundary Hit until it fills the Arena, wiping
    // all of the Film.
    let growing =
        wiping_with_film_left_counter(30.0) + "\n[[rules]]\nkind = \"grow\"\namount = 20.0\n";
    let readings = counter_text_every_frame(&growing, 1);

    assert_eq!(readings[0].0, "100%");
    let (last, left) = readings.last().unwrap();
    assert_eq!(*left, 0.0, "the Film should be cleared by the end");
    assert_eq!(last, "0%");
    assert_eq!(Counter::FilmLeft.label(0), "FILM LEFT");
}

#[test]
fn film_left_counter_never_shows_0_percent_while_film_is_left() {
    // After 10 s a tiny patch, well under half a percent, is still left.
    let readings = counter_text_every_frame(&wiping_with_film_left_counter(10.0), 1);

    let (last, left) = readings.last().unwrap();
    assert!(*left > 0.0 && *left < 0.005, "left {left}");
    assert_eq!(last, "1%");
    for (text, left) in &readings {
        assert_eq!(text == "0%", *left == 0.0, "{text} with {left} left");
    }
}
