//! How a Video looks: palette, visual toggles and which Counter is shown.
//!
//! Scenario files set these in an optional `[look]` table:
//!
//! ```toml
//! [look]
//! palette = "neon"          # optional; omitted = chosen by the Seed
//! trails = true             # fading motion trails (default true)
//! glow = true               # glow where each Boundary Hit lands (default true)
//! counter = "ball_count"    # "boundary_hits" (default), "ball_count" or "speed"
//! ```

pub mod counter;
pub mod palette;

use serde::Deserialize;

pub use counter::Counter;
pub use palette::Palette;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LookSpec {
    /// Pinned palette name; `None` lets the Seed choose.
    pub palette: Option<String>,
    pub trails: bool,
    pub glow: bool,
    pub counter: Counter,
}

impl Default for LookSpec {
    fn default() -> Self {
        LookSpec {
            palette: None,
            trails: true,
            glow: true,
            counter: Counter::default(),
        }
    }
}

impl LookSpec {
    pub fn validate(&self) -> Result<(), String> {
        match &self.palette {
            Some(name) if palette::by_name(name).is_none() => {
                let known: Vec<&str> = palette::PALETTES.iter().map(|p| p.name).collect();
                Err(format!(
                    "unknown palette '{name}' (known: {})",
                    known.join(", ")
                ))
            }
            _ => Ok(()),
        }
    }
}
