//! Shrink: a ball's radius decreases by `amount` on each Boundary Hit, never
//! below `min_radius`.
//!
//! ```toml
//! [[rules]]
//! kind = "shrink"
//! amount = 2.0      # sim units removed from the radius per Boundary Hit (> 0)
//! min_radius = 4.0  # optional, default 4.0 (> 0); the ball stays visible
//! ```

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, SimState};

/// Smallest radius when a Scenario doesn't set one: still clearly visible.
pub const DEFAULT_MIN_RADIUS: f64 = 4.0;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    amount: f64,
    #[serde(default = "default_min_radius")]
    min_radius: f64,
}

fn default_min_radius() -> f64 {
    DEFAULT_MIN_RADIUS
}

pub struct Shrink {
    amount: f64,
    min_radius: f64,
}

impl Shrink {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        if !(p.amount.is_finite() && p.amount > 0.0) {
            return Err("Rule 'shrink': amount must be a finite number > 0".into());
        }
        if !(p.min_radius.is_finite() && p.min_radius > 0.0) {
            return Err("Rule 'shrink': min_radius must be a finite number > 0".into());
        }
        Ok(Shrink {
            amount: p.amount,
            min_radius: p.min_radius,
        })
    }
}

impl Rule for Shrink {
    fn on_boundary_hit(
        &mut self,
        state: &mut SimState,
        ball: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        let ball = &mut state.balls[ball];
        // A ball already below the minimum (e.g. its starting size) is left as is.
        if ball.radius > self.min_radius {
            ball.radius = (ball.radius - self.amount).max(self.min_radius);
        }
    }
}
