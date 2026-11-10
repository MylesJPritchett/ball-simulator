//! Invalid Scenario files fail with clear errors.

mod support;

use ball_simulator::scenario::Scenario;
use support::{MINIMAL, NEVER_ENDS};

fn error_for(toml: &str) -> String {
    Scenario::from_toml_str("broken", toml)
        .expect_err("Scenario should be rejected")
        .to_string()
}

/// MINIMAL plus an End Condition: a complete, valid Scenario file.
fn complete() -> String {
    format!("{MINIMAL}{NEVER_ENDS}")
}

#[test]
fn unknown_rule_is_named_in_the_error() {
    let toml = format!("{}\n[[rules]]\nkind = \"teleport\"\n", complete());
    let err = error_for(&toml);
    assert!(
        err.contains("broken") && err.contains("rules[0]: unknown Rule 'teleport'"),
        "{err}"
    );
}

#[test]
fn unknown_end_condition_is_named_in_the_error() {
    let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"forever\"\n");
    let err = error_for(&toml);
    assert!(err.contains("unknown End Condition 'forever'"), "{err}");
}

#[test]
fn threshold_end_conditions_are_refused_as_not_terminal() {
    for (kind, params) in [
        ("population_cap", ""),
        ("max_speed_reached", ""),
        ("min_radius_reached", "min_radius = 10.0\n"),
        ("arena_min_reached", "min_radius = 100.0\n"),
    ] {
        let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"{kind}\"\n{params}");
        let err = error_for(&toml);
        assert!(
            err.contains("broken")
                && err.contains(&format!(
                    "end_condition: End Condition '{kind}' is not a Terminal State"
                )),
            "{kind}: {err}"
        );
    }
}

#[test]
fn the_time_cap_is_refused_as_an_end_condition() {
    let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"time_cap\"\n");
    let err = error_for(&toml);
    assert!(
        err.contains("broken")
            && err.contains("end_condition: End Condition 'time_cap' is not a Terminal State")
            && err.contains("safety net"),
        "{err}"
    );
}

#[test]
fn missing_end_condition_is_an_error() {
    let err = error_for(MINIMAL);
    assert!(
        err.contains("broken") && err.contains("missing [end_condition]"),
        "{err}"
    );
}

#[test]
fn inverted_range_names_the_field() {
    let toml = complete().replace("speed = 900.0", "speed = { min = 900.0, max = 300.0 }");
    let err = error_for(&toml);
    assert!(
        err.contains("broken") && err.contains("ball.speed: inverted range"),
        "{err}"
    );
}

#[test]
fn range_with_non_numeric_bounds_names_the_field() {
    let toml = complete().replace("radius = 460.0", "radius = { min = \"big\", max = 500.0 }");
    let err = error_for(&toml);
    assert!(
        err.contains("arena.radius: range min and max must both be numbers"),
        "{err}"
    );
}

#[test]
fn wrong_value_type_names_the_field() {
    let toml = complete().replace("speed = 900.0", "speed = \"fast\"");
    let err = error_for(&toml);
    assert!(
        err.contains("broken") && err.contains("ball.speed: invalid type: string"),
        "{err}"
    );
}

#[test]
fn ranges_are_validated_at_both_ends() {
    // A ball radius range reaching past the Arena is rejected up front, not on
    // the unlucky Seed that draws it.
    let toml = complete().replace("radius = 30.0", "radius = { min = 30.0, max = 500.0 }");
    let err = error_for(&toml);
    assert!(err.contains("ball.radius"), "{err}");
}

#[test]
fn errors_from_scenario_files_name_the_file_path() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("validation");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("no_ending.toml");
    std::fs::write(&path, MINIMAL).unwrap();

    let err = Scenario::from_file(&path).unwrap_err().to_string();
    assert!(err.contains(&path.display().to_string()), "{err}");
}

#[test]
fn quality_gate_duration_range_must_not_be_inverted() {
    let toml = format!(
        "{}\n[quality_gate]\nmin_duration = 40.0\nmax_duration = 30.0\n",
        complete()
    );
    let err = error_for(&toml);
    assert!(
        err.contains("quality_gate.min_duration must not exceed quality_gate.max_duration"),
        "{err}"
    );
}

#[test]
fn unknown_quality_gate_threshold_is_named_in_the_error() {
    let toml = format!("{}\n[quality_gate]\nmin_hits = 3\n", complete());
    let err = error_for(&toml);
    assert!(
        err.contains("quality_gate") && err.contains("min_hits"),
        "{err}"
    );
}

#[test]
fn shrink_has_no_minimum_radius() {
    // A shrinking ball vanishes instead of stopping at a minimum.
    let toml = format!(
        "{}\n[[rules]]\nkind = \"shrink\"\namount = 2.0\nmin_radius = 4.0\n",
        complete()
    );
    let err = error_for(&toml);
    assert!(
        err.contains("broken") && err.contains("shrink") && err.contains("min_radius"),
        "{err}"
    );
}

#[test]
fn escaped_needs_a_rule_that_can_let_a_ball_out() {
    let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"escaped\"\n");
    let err = error_for(&toml);
    assert!(
        err.contains("broken")
            && err.contains(
                "end_condition: End Condition 'escaped' needs a Rule that can let a ball out"
            ),
        "{err}"
    );
}

#[test]
fn meltdown_needs_a_rule_that_adds_balls() {
    let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"meltdown\"\n");
    let err = error_for(&toml);
    assert!(
        err.contains("broken")
            && err.contains("end_condition: End Condition 'meltdown' needs a Rule that adds balls"),
        "{err}"
    );
}

#[test]
fn meltdown_with_a_split_is_valid() {
    let toml = format!(
        "{MINIMAL}\n[[rules]]\nkind = \"split\"\ninto = 2\nspread = 30.0\n\
         \n[end_condition]\nkind = \"meltdown\"\n"
    );
    Scenario::from_toml_str("ok", &toml).expect("meltdown with a split is valid");
}

#[test]
fn escaped_with_a_gap_is_valid() {
    let toml = format!(
        "{MINIMAL}\n[[rules]]\nkind = \"gap\"\nwidth = 40.0\nrotation_speed = 10.0\n\
         \n[end_condition]\nkind = \"escaped\"\n"
    );
    Scenario::from_toml_str("ok", &toml).expect("escaped with a gap is valid");
}

#[test]
fn bad_gap_parameters_are_named_in_the_error() {
    for (params, expected) in [
        (
            "width = 0.0\nrotation_speed = 0.0",
            "'gap': width must be a finite number > 0, got 0",
        ),
        (
            "width = 400.0\nrotation_speed = 0.0",
            "'gap': width must be at most 360 degrees, got 400",
        ),
        (
            "width = 30.0\nrotation_speed = inf",
            "'gap': rotation_speed must be a finite number, got inf",
        ),
        ("width = 30.0", "missing field `rotation_speed`"),
        (
            "width = 30.0\nrotation_speed = 0.0\nspin = 1.0",
            "unknown field `spin`",
        ),
    ] {
        let toml = format!("{}\n[[rules]]\nkind = \"gap\"\n{params}\n", complete());
        let err = error_for(&toml);
        assert!(
            err.contains("rules[0]") && err.contains(expected),
            "{params}: {err}"
        );
    }
}

/// MINIMAL with a max speed and the `crack` Rule, ending on `escaped`.
fn cracking() -> String {
    format!(
        "{}\n[[rules]]\nkind = \"crack\"\n\n[end_condition]\nkind = \"escaped\"\n",
        MINIMAL.replace("speed = 900.0", "speed = 900.0\nmax_speed = 2000.0")
    )
}

#[test]
fn escaped_with_a_crack_is_valid() {
    Scenario::from_toml_str("ok", &cracking()).expect("escaped with a crack is valid");
}

#[test]
fn crack_needs_a_max_speed() {
    let toml = cracking().replace("max_speed = 2000.0\n", "");
    let err = error_for(&toml);
    assert!(
        err.contains("broken") && err.contains("rules[0]: Rule 'crack' needs ball.max_speed"),
        "{err}"
    );
}

#[test]
fn crack_has_no_parameters() {
    let toml = cracking().replace("kind = \"crack\"", "kind = \"crack\"\nlength = 3.0");
    let err = error_for(&toml);
    assert!(
        err.contains("rules[0]") && err.contains("unknown field `length`"),
        "{err}"
    );
}
