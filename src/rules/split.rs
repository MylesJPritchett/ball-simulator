//! Split: on a Boundary Hit, a ball becomes `into` balls fanned symmetrically
//! around its reflected direction across `spread` degrees.
//!
//! ```toml
//! [[rules]]
//! kind = "split"
//! into = 2         # balls after the split (>= 2)
//! spread = 30.0    # total fan angle in degrees, 0 < spread < 180
//! ```
//!
//! The ball itself takes the first direction of the fan; the rest are new
//! balls with the same position, speed, radius and colour. The Population Cap is
//! enforced by the simulation, not here.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, SimState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    into: u32,
    spread: f64,
}

pub struct Split {
    into: u32,
    spread: f64,
}

impl Split {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        if p.into < 2 {
            return Err("Rule 'split': into must be at least 2".into());
        }
        // Keeping the fan under 180 degrees keeps every child moving inward.
        if !(p.spread > 0.0 && p.spread < 180.0) {
            return Err("Rule 'split': spread must be between 0 and 180 degrees".into());
        }
        Ok(Split {
            into: p.into,
            spread: p.spread.to_radians(),
        })
    }
}

impl Rule for Split {
    fn on_boundary_hit(
        &mut self,
        state: &mut SimState,
        ball: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        let parent = state.balls[ball].clone();
        let step = self.spread / (self.into - 1) as f64;
        let offset = |i: u32| -self.spread / 2.0 + step * i as f64;
        state.balls[ball].vel = parent.vel.rotated(offset(0));
        for i in 1..self.into {
            let child = state.spawn(parent.pos, parent.vel.rotated(offset(i)), parent.radius);
            state.balls[child].colour = parent.colour;
        }
    }
}
