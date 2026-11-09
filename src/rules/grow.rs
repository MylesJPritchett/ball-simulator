//! Grow: a ball's radius increases by `amount` on each Boundary Hit.
//!
//! ```toml
//! [[rules]]
//! kind = "grow"
//! amount = 4.0     # sim units added to the radius per Boundary Hit (> 0)
//! ```

use serde::Deserialize;

use super::{params, positive_finite, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, SimState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    amount: f64,
}

pub struct Grow {
    amount: f64,
}

impl Grow {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        Ok(Grow {
            amount: positive_finite(spec, "amount", p.amount)?,
        })
    }
}

impl Rule for Grow {
    fn on_boundary_hit(
        &mut self,
        state: &mut SimState,
        ball: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        state.balls[ball].radius += self.amount;
    }
}
