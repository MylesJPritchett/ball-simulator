//! The Film, through the simulation seam: what the record says about Film left.

mod support;

use ball_simulator::scenario::Scenario;
use support::{run, MINIMAL, NEVER_ENDS};

/// A `film` Rule that never grows back.
const FILM: &str = "\n[[rules]]\nkind = \"film\"\nregrow = 0.0\n";

#[test]
fn the_film_covers_the_whole_arena_at_the_start() {
    let record = run(&format!("{MINIMAL}{FILM}"), 1);

    let first = &record.film_left[0];
    assert_eq!(first.substep, 0);
    assert_eq!(first.left, 1.0);
}

#[test]
fn without_a_film_rule_there_is_no_film() {
    assert!(run(MINIMAL, 1).film_left.is_empty());
}

/// MINIMAL with a Film and the ball (radius `radius`) moving at `speed` for
/// `seconds`. The ball starts within (460 - radius) / 2 of the centre, so if
/// it travels less than that it never reaches the Boundary.
fn straight_pass(radius: f64, speed: f64, seconds: f64) -> String {
    MINIMAL
        .replace("time_cap = 5.0", &format!("time_cap = {seconds:?}"))
        .replace("radius = 30.0", &format!("radius = {radius:?}"))
        .replace("speed = 900.0", &format!("speed = {speed:?}"))
        + FILM
}

/// The fraction of the Arena (radius 460) the ball wiped by the end of the run.
fn wiped(toml: &str, seed: u64) -> f64 {
    1.0 - run(toml, seed).film_left.last().expect("a Film").left
}

#[test]
fn a_ball_wipes_a_band_as_wide_as_itself() {
    // A ball of radius 30 travelling 200 units in a straight line sweeps a
    // 60 x 200 band plus a disc at each end: 12000 + 900 pi = 14827.4 of the
    // Arena's 460^2 pi = 664761.0, so 2.2305% of it.
    for seed in 0..5 {
        let wiped = wiped(&straight_pass(30.0, 100.0, 2.0), seed);
        assert!(
            (wiped - 0.022305).abs() < 0.0004,
            "seed {seed}: wiped {wiped}"
        );
    }
}

#[test]
fn a_fast_ball_leaves_no_film_between_substeps() {
    // A ball of radius 2 at 4000 units/s moves 8.3 units a substep, over
    // twice its diameter. Travelling 200 units it sweeps 4 x 200 + 4 pi =
    // 812.6 (0.12224% of the Arena); its discs at each substep alone would
    // cover only 25 x 4 pi = 314.2.
    for seed in 0..5 {
        let wiped = wiped(&straight_pass(2.0, 4000.0, 0.05), seed);
        assert!(
            (wiped - 0.0012224).abs() < 0.0001,
            "seed {seed}: wiped {wiped}"
        );
    }
}

#[test]
fn wiped_film_never_comes_back_without_regrowth() {
    // The ball bounces round the Arena for 5 s, crossing its own path.
    let record = run(&format!("{MINIMAL}{FILM}"), 3);

    assert!(record.film_left.len() > 100);
    for pair in record.film_left.windows(2) {
        assert!(pair[1].left < pair[0].left, "{pair:?}");
    }
}

fn rejected(toml: &str) -> String {
    Scenario::from_toml_str("broken", &format!("{toml}{NEVER_ENDS}"))
        .expect_err("Scenario should be rejected")
        .to_string()
}

#[test]
fn a_scenario_has_at_most_one_film() {
    let err = rejected(&format!("{MINIMAL}{FILM}{FILM}"));
    assert!(err.contains("at most one 'film' Rule"), "{err}");
}

#[test]
fn regrow_must_not_be_negative() {
    let err = rejected(&format!("{MINIMAL}{}", FILM.replace("0.0", "-1.0")));
    assert!(err.contains("regrow must be a finite number >= 0"), "{err}");
}

#[test]
fn regrow_can_be_a_range() {
    let toml = format!(
        "{MINIMAL}{}",
        FILM.replace("0.0", "{ min = 0.0, max = 20.0 }")
    );
    let record = run(&toml, 1);
    assert_eq!(record.parameters[0].field, "rules[0].regrow");
}

#[test]
fn the_same_seed_wipes_the_same_film() {
    let toml = format!("{MINIMAL}{FILM}");
    assert_eq!(run(&toml, 4), run(&toml, 4));
}
