//! Crack: every Boundary Hit cracks the Boundary where it lands, the harder
//! the faster the ball (as a fraction of the Scenario's max speed). The first
//! Boundary Hit at max speed shatters it: the whole Boundary opens and the
//! ball flies out the way it was going.

use std::f64::consts::TAU;

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, Crack as Cracked, Gap, Shatter, SimState};

/// A Boundary Hit this close to the max speed counts as at it (the simulation
/// caps speeds at the max, give or take rounding).
const AT_MAX_SPEED: f64 = 1.0 - 1e-9;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {}

pub struct Crack;

impl Crack {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let _: Params = params(spec)?;
        Ok(Crack)
    }
}

impl Rule for Crack {
    fn lets_balls_out(&self) -> bool {
        true
    }

    fn needs_max_speed(&self) -> bool {
        true
    }

    fn on_boundary_hit(
        &mut self,
        state: &mut SimState,
        ball: usize,
        hit: &mut BoundaryHit,
        ctx: &mut RuleContext,
    ) {
        // Open here means another ball shattered the Boundary earlier this
        // substep, after this one had already been reflected: it gets out too.
        if !state.arena.is_open_at(hit.angle) {
            let size = (hit.speed / ctx.max_speed).min(1.0);
            state.arena.cracks.push(Cracked {
                angle: hit.angle,
                size,
            });
            if size < AT_MAX_SPEED {
                return;
            }
            state.arena.gaps.push(Gap {
                angle: hit.angle,
                width: TAU,
            });
            state.arena.shatter = Some(Shatter {
                angle: hit.angle,
                substep: hit.substep,
            });
        }
        // Boundary resolution has already reflected the ball (Rules run after
        // it): undo that so it carries on out the way it hit.
        let ball = &mut state.balls[ball];
        let normal = ball.pos * (1.0 / ball.pos.length());
        ball.vel -= normal * (2.0 * ball.vel.dot(normal));
        ball.escaping = true;
    }
}
