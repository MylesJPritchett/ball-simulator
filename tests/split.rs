//! Split Rule and Population Cap, tested through the seam.

mod support;

use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{Vec2, SUBSTEP_HZ};
use support::{run, MINIMAL};

const SEED: u64 = 4;

/// MINIMAL with a different time cap (a top-level key, so `.replace()`).
fn with_time_cap(toml: &str, seconds: f64) -> String {
    toml.replace("time_cap = 5.0", &format!("time_cap = {seconds:?}"))
}

fn split(n: u32, spread_degrees: f64) -> String {
    format!("{MINIMAL}\n[[rules]]\nkind = \"split\"\ninto = {n}\nspread = {spread_degrees:?}\n")
}

fn angle_between(a: Vec2, b: Vec2) -> f64 {
    let cross = a.x * b.y - a.y * b.x;
    cross.atan2(a.dot(b))
}

#[test]
fn split_into_three_fans_symmetrically_about_the_reflected_velocity() {
    // Without the Rule, stopping exactly at the first Boundary Hit gives the
    // reflected velocity of the single ball.
    let first_hit = run(MINIMAL, SEED).boundary_hits[0].substep;
    let stop = first_hit as f64 / SUBSTEP_HZ as f64;
    let plain = run(&with_time_cap(MINIMAL, stop), SEED);
    let reflected = plain.final_balls[0].vel;

    let record = run(&with_time_cap(&split(3, 40.0), stop), SEED);

    assert_eq!(record.boundary_hits.len(), 1);
    assert_eq!(record.ball_count.last().unwrap().count, 3);
    assert_eq!(record.final_balls.len(), 3);
    let mut angles: Vec<f64> = record
        .final_balls
        .iter()
        .map(|b| {
            assert!((b.vel.length() - 900.0).abs() < 1e-9, "speed is preserved");
            assert_eq!(
                b.pos, plain.final_balls[0].pos,
                "children start at the hit point"
            );
            angle_between(reflected, b.vel).to_degrees()
        })
        .collect();
    angles.sort_by(f64::total_cmp);
    for (got, want) in angles.iter().zip([-20.0, 0.0, 20.0]) {
        assert!((got - want).abs() < 1e-9, "angles {angles:?}");
    }
}

#[test]
fn split_children_keep_the_parents_colour() {
    let first_hit = run(MINIMAL, SEED).boundary_hits[0].substep;
    let stop = first_hit as f64 / SUBSTEP_HZ as f64;
    // change_colour runs first, so the parent is on colour 1 when it splits.
    let toml = format!(
        "{}\n[[rules]]\nkind = \"change_colour\"\n[[rules]]\nkind = \"split\"\ninto = 3\nspread = 40.0\n",
        with_time_cap(MINIMAL, stop)
    );
    let record = run(&toml, SEED);

    let colours: Vec<u32> = record.final_balls.iter().map(|b| b.colour).collect();
    assert_eq!(colours, [1, 1, 1]);
}

/// A Scenario with a Population Cap (a top-level key, so `.replace()`).
fn with_cap(toml: &str, cap: usize) -> String {
    toml.replace(
        "time_cap = 5.0",
        &format!("time_cap = 5.0\npopulation_cap = {cap}"),
    )
}

#[test]
fn ball_count_never_exceeds_the_population_cap() {
    let toml = with_cap(&split(3, 60.0), 100);
    for seed in 0..10 {
        let record = run(&toml, seed);
        let max = record.ball_count.iter().map(|s| s.count).max().unwrap();
        assert_eq!(
            max, 100,
            "seed {seed}: splitting reaches but never passes the cap"
        );
        assert_eq!(record.final_balls.len(), 100);
    }
}

const POPULATION_CAP_END: &str = "\n[end_condition]\nkind = \"population_cap\"\n";

#[test]
fn reaching_the_population_cap_ends_the_run_on_that_substep() {
    let toml = with_cap(&split(2, 40.0), 50) + POPULATION_CAP_END;
    for seed in 0..10 {
        let record = run(&toml, seed);
        let reached = record
            .ball_count
            .iter()
            .find(|s| s.count == 50)
            .expect("cap reached");

        assert_eq!(record.end.condition, "population_cap", "seed {seed}");
        assert_eq!(record.end.substep, reached.substep, "seed {seed}");
        assert_eq!(record.ball_count.last(), Some(reached));
    }
}

#[test]
fn time_cap_still_ends_a_run_whose_population_cap_is_never_reached() {
    // No split Rule: the single ball can never reach a cap of 50.
    let toml = with_cap(MINIMAL, 50) + POPULATION_CAP_END;
    let record = run(&toml, 1);

    assert_eq!(record.end.condition, "time_cap");
    assert_eq!(record.end.substep, 5 * SUBSTEP_HZ);
}

#[test]
fn invalid_split_and_population_cap_values_are_rejected_clearly() {
    // Each has a valid End Condition, so only the value under test is wrong.
    let error_for = |toml: &str| {
        let toml = format!("{toml}\n[end_condition]\nkind = \"time_cap\"\n");
        Scenario::from_toml_str("broken", &toml)
            .expect_err("Scenario should be rejected")
            .to_string()
    };

    let err = error_for(&split(1, 30.0));
    assert!(err.contains("into must be at least 2"), "{err}");
    let err = error_for(&split(2, 180.0));
    assert!(err.contains("spread must be between 0 and 180"), "{err}");
    let err = error_for(&with_cap(MINIMAL, 0));
    assert!(err.contains("population_cap must be at least 1"), "{err}");
}

#[test]
fn population_cap_may_be_a_range_resolved_per_seed() {
    let toml = split(3, 60.0).replace(
        "time_cap = 5.0",
        "time_cap = 5.0\npopulation_cap = { min = 20, max = 80 }",
    ) + POPULATION_CAP_END;
    let mut caps = std::collections::BTreeSet::new();
    for seed in 0..10 {
        let record = run(&toml, seed);
        let cap = record.final_balls.len();
        assert!((20..=80).contains(&cap), "seed {seed}: {cap}");
        assert_eq!(record.end.condition, "population_cap", "seed {seed}");
        caps.insert(cap);
    }
    assert!(caps.len() > 1, "different Seeds resolve different caps");

    let toml = with_cap(MINIMAL, 1).replace(
        "population_cap = 1",
        "population_cap = { min = 0, max = 5 }",
    ) + "\n[end_condition]\nkind = \"time_cap\"\n";
    let err = Scenario::from_toml_str("broken", &toml)
        .expect_err("a cap range reaching 0 is rejected")
        .to_string();
    assert!(err.contains("population_cap must be at least 1"), "{err}");
}
