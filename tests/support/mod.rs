//! Shared helpers for tests that go through the simulation seam.
#![allow(dead_code)]

use ball_simulator::scenario::Scenario;
use ball_simulator::sim::{self, SimRecord};

/// One ball, no Rules, circular Arena. Override pieces with `format!` in tests.
pub const MINIMAL: &str = r#"
name = "Minimal"
time_cap = 5.0

[arena]
radius = 460.0

[ball]
radius = 30.0
speed = 900.0
"#;

/// An End Condition that never fires unless a Rule grows a ball or shrinks
/// the Arena all the way, so a run without either ends at the time cap.
pub const NEVER_ENDS: &str = "\n[end_condition]\nkind = \"arena_filled\"\n";

/// MINIMAL with a terminal ending at a known time: the Arena closes in on the
/// ball (radius 30) from 460 and fills it exactly `seconds` in, a second
/// before the time cap.
pub fn filled_after(seconds: f64) -> String {
    let rate = 430.0 / seconds;
    let time_cap = seconds + 1.0;
    MINIMAL.replace("time_cap = 5.0", &format!("time_cap = {time_cap:?}"))
        + &format!("\n[[rules]]\nkind = \"shrinking_arena\"\nrate = {rate:?}\n")
        + "\n[end_condition]\nkind = \"arena_filled\"\n"
}

/// Parses an inline Scenario. Scenario files must declare an End Condition;
/// for brevity, inline test Scenarios without one get [`NEVER_ENDS`].
pub fn scenario(toml: &str) -> Scenario {
    let toml = if toml.contains("[end_condition]") {
        toml.to_string()
    } else {
        format!("{toml}{NEVER_ENDS}")
    };
    Scenario::from_toml_str("inline", &toml).expect("inline Scenario should parse")
}

pub fn run(toml: &str, seed: u64) -> SimRecord {
    sim::run(&scenario(toml), seed)
}
