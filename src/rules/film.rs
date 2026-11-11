//! Film: lays a Film over the inside of the Arena when the simulation starts.
//! Balls wipe it wherever their discs pass (the simulation does that every
//! substep for any Film, see `Film::wipe`).
//!
//! ```toml
//! [[rules]]
//! kind = "film"
//! regrow = 0.0     # how fast the Film grows back (>= 0; 0 = never)
//! ```
//!
//! A Scenario has at most one `film` Rule.

use serde::Deserialize;

use super::{non_negative_finite, params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{Film as Layer, SimState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    regrow: f64,
}

pub struct Film;

impl Film {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        // Checked now so Scenarios can set it; the Film does not grow back yet.
        non_negative_finite(spec, "regrow", p.regrow)?;
        Ok(Film)
    }
}

impl Rule for Film {
    fn on_start(&mut self, state: &mut SimState, _ctx: &mut RuleContext) {
        state.film = Some(Layer::new(state.arena.radius));
    }
}
