//! Shrink: a ball's radius decreases by `amount` on each Boundary Hit. The
//! hit that would take it to zero or below makes the ball vanish instead
//! (pair with the `vanished` End Condition).
//!
//! ```toml
//! [[rules]]
//! kind = "shrink"
//! amount = 2.0      # sim units removed from the radius per Boundary Hit (> 0)
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

pub struct Shrink {
    amount: f64,
}

impl Shrink {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        Ok(Shrink {
            amount: positive_finite(spec, "amount", p.amount)?,
        })
    }
}

impl Rule for Shrink {
    fn on_boundary_hit(
        &mut self,
        state: &mut SimState,
        index: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        let ball = &mut state.balls[index];
        if ball.radius - self.amount > 0.0 {
            ball.radius -= self.amount;
        } else {
            state.vanish(index);
        }
    }
}
