//! Scenario parameter ranges resolve from the Seed, tested through the seam.

mod support;

use support::{run, MINIMAL};

fn with_time_cap(value: &str) -> String {
    MINIMAL.replace("time_cap = 5.0", &format!("time_cap = {value}"))
}

#[test]
fn a_ranged_time_cap_resolves_within_its_range_for_every_seed() {
    let toml = with_time_cap("{ min = 2.0, max = 4.0 }");
    for seed in 0..20 {
        let record = run(&toml, seed);
        let t = record.end.time();
        assert!((2.0..=4.0).contains(&t), "seed {seed}: time cap {t}");
        assert_eq!(record, run(&toml, seed), "seed {seed} is deterministic");
    }
}

#[test]
fn the_record_names_each_ranged_field_and_its_resolved_value() {
    let record = run(&with_time_cap("{ min = 2.0, max = 4.0 }"), 3);

    assert_eq!(record.parameters.len(), 1);
    assert_eq!(record.parameters[0].field, "time_cap");
    assert!((record.parameters[0].value.as_f64() - record.end.time()).abs() < 1.0 / 480.0);
}

#[test]
fn different_seeds_resolve_to_different_values() {
    let toml = MINIMAL.replace("speed = 900.0", "speed = { min = 600.0, max = 1200.0 }");
    let mut speeds: Vec<f64> = (0..10)
        .map(|seed| run(&toml, seed).parameters[0].value.as_f64())
        .collect();
    speeds.sort_by(f64::total_cmp);
    speeds.dedup();
    assert_eq!(speeds.len(), 10, "{speeds:?}");
}

#[test]
fn pinned_values_are_used_as_is_for_every_seed() {
    let toml = with_time_cap("3.0");
    for seed in 0..10 {
        let record = run(&toml, seed);
        assert!((record.end.time() - 3.0).abs() < 1e-9);
        assert!(record.parameters.is_empty());
    }
}

#[test]
fn an_integer_range_resolves_to_whole_numbers_including_both_ends() {
    let toml = MINIMAL.replace("radius = 30.0", "radius = { min = 20, max = 22 }");
    let mut seen: Vec<f64> = (0..40)
        .map(|seed| run(&toml, seed).parameters[0].value.as_f64())
        .collect();
    seen.sort_by(f64::total_cmp);
    seen.dedup();
    assert_eq!(seen, vec![20.0, 21.0, 22.0]);
}

#[test]
fn a_ranged_max_speed_resolves_and_caps_the_ball() {
    let toml = MINIMAL.replace(
        "speed = 900.0",
        "speed = 900.0\nmax_speed = { min = 400.0, max = 600.0 }",
    );
    for seed in 0..10 {
        let record = run(&toml, seed);
        assert_eq!(record.parameters[0].field, "ball.max_speed");
        let cap = record.parameters[0].value.as_f64();
        assert!(
            (400.0..600.0).contains(&cap),
            "seed {seed}: max speed {cap}"
        );
        assert!(record.stats.max_speed <= cap + 1e-9, "seed {seed}");
    }
}
