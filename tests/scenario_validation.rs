//! Invalid Scenario files fail with clear errors.

mod support;

use ball_simulator::scenario::Scenario;
use support::MINIMAL;

const TIME_CAP_END: &str = "\n[end_condition]\nkind = \"time_cap\"\n";

fn error_for(toml: &str) -> String {
    Scenario::from_toml_str("broken", toml)
        .expect_err("Scenario should be rejected")
        .to_string()
}

/// MINIMAL plus an End Condition: a complete, valid Scenario file.
fn complete() -> String {
    format!("{MINIMAL}{TIME_CAP_END}")
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
