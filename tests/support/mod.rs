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

pub fn scenario(toml: &str) -> Scenario {
    Scenario::from_toml_str("inline", toml).expect("inline Scenario should parse")
}

pub fn run(toml: &str, seed: u64) -> SimRecord {
    sim::run(&scenario(toml), seed)
}
