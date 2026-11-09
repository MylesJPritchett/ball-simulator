//! Shrinking Arena: the Arena's radius closes in at a constant rate until it
//! reaches a minimum.
//!
//! The Arena shrinks in `on_substep`, before balls move, so the same substep's
//! Boundary resolution keeps every ball inside the new radius; a ball caught by
//! the closing Boundary registers a Boundary Hit like any other.
//!
//! While the Boundary is closing, a Boundary Hit sends the ball inward at no
//! less than [`LEAVE_FACTOR`] times the closing speed. Otherwise a ball slower
//! than the Boundary would ride it, hitting (and sounding) every substep.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, World};

/// Minimum inward speed after a hit on a closing Boundary, as a multiple of
/// the closing speed. Above 1 so the ball separates from the Boundary.
const LEAVE_FACTOR: f64 = 2.0;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    /// Arena radius lost per second, in simulation units.
    rate: f64,
    /// The Arena stops shrinking at this radius.
    min_radius: f64,
}

pub struct ShrinkingArena {
    rate: f64,
    min_radius: f64,
    /// Whether the Arena shrank this substep.
    closing: bool,
}

impl ShrinkingArena {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        if !(p.rate.is_finite() && p.rate >= 0.0) {
            return Err(format!(
                "Rule 'shrinking_arena': rate must be >= 0, got {}",
                p.rate
            ));
        }
        if !(p.min_radius.is_finite() && p.min_radius > 0.0) {
            return Err(format!(
                "Rule 'shrinking_arena': min_radius must be > 0, got {}",
                p.min_radius
            ));
        }
        Ok(ShrinkingArena {
            rate: p.rate,
            min_radius: p.min_radius,
            closing: false,
        })
    }
}

impl Rule for ShrinkingArena {
    fn on_substep(&mut self, world: &mut World, ctx: &mut RuleContext) {
        let arena = &mut world.arena;
        self.closing = arena.radius > self.min_radius && self.rate > 0.0;
        if self.closing {
            arena.radius = (arena.radius - self.rate * ctx.dt).max(self.min_radius);
        }
    }

    fn on_boundary_hit(
        &mut self,
        world: &mut World,
        ball: usize,
        _hit: &BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        if !self.closing {
            return;
        }
        let ball = &mut world.balls[ball];
        let dist = ball.pos.length();
        if dist == 0.0 {
            return;
        }
        let normal = ball.pos * (1.0 / dist);
        let min_inward = LEAVE_FACTOR * self.rate;
        let inward = -ball.vel.dot(normal);
        if inward < min_inward {
            ball.vel -= normal * (min_inward - inward);
        }
    }
}
