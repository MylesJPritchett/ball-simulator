//! Film: lays a Film over the inside of the Arena when the simulation starts.
//! Balls wipe it wherever their discs pass (the simulation does that every
//! substep for any Film, see `Film::wipe`).
//!
//! ```toml
//! [[rules]]
//! kind = "film"
//! regrow = 0.0     # how fast the Film's edge grows back, units/s (>= 0; 0 = never)
//! ```
//!
//! Cleared Film grows back only from the Film that is left: its edge advances
//! about `regrow` Arena units a second into the cleared area, so a ball's
//! path closes up behind it. Once no Film is left, nothing grows back.
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

pub struct Film {
    regrow: f64,
}

impl Film {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        let regrow = non_negative_finite(spec, "regrow", p.regrow)?;
        Ok(Film { regrow })
    }
}

impl Rule for Film {
    fn on_start(&mut self, state: &mut SimState, _ctx: &mut RuleContext) {
        state.film = Some(Layer::new(state.arena.radius));
    }

    fn on_substep(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        if self.regrow > 0.0 {
            if let Some(film) = &mut state.film {
                film.regrow(self.regrow * ctx.dt);
            }
        }
    }
}
