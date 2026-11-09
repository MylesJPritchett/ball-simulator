//! Invalid Scenario files fail with clear errors.

mod support;

use ball_simulator::scenario::Scenario;
use support::MINIMAL;

fn error_for(toml: &str) -> String {
    Scenario::from_toml_str("broken", toml)
        .expect_err("Scenario should be rejected")
        .to_string()
}

#[test]
fn unknown_rule_is_named_in_the_error() {
    let toml = format!("{MINIMAL}\n[[rules]]\nkind = \"teleport\"\n");
    let err = error_for(&toml);
    assert!(
        err.contains("broken") && err.contains("unknown Rule 'teleport'"),
        "{err}"
    );
}

#[test]
fn unknown_end_condition_is_named_in_the_error() {
    let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"forever\"\n");
    let err = error_for(&toml);
    assert!(err.contains("unknown End Condition 'forever'"), "{err}");
}
