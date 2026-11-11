//! Settle: a Ring that can no longer move settles, becoming part of the
//! Arena, and a new Ring takes its place.
//!
//! A Ring whose radius has reached the Arena's (the simulation clamps a Ring
//! grown past the Boundary to the exact fit, which also makes it concentric)
//! settles into the Boundary: it becomes a layer of the Arena in its colour,
//! the Arena's radius shrinks by its thickness, and a new Ring spawns at the
//! centre with the first Ring's radius, thickness and speed, heading in a
//! random direction. Once the Arena is no larger than a new Ring, none spawns
//! and the Arena is solid. Only Rings settle.
//!
//! ```toml
//! [[rules]]
//! kind = "settle"   # no parameters
//! ```

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{Settle as Settled, SimState, Surface, Vec2};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {}

pub struct Settle {
    /// Each new Ring is a copy of the first as it started: (radius,
    /// thickness, speed). `None` until the start, or if the ball is solid.
    new_ring: Option<(f64, f64, f64)>,
}

impl Settle {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let Params {} = params(spec)?;
        Ok(Settle { new_ring: None })
    }
}

impl Rule for Settle {
    fn settles_rings(&self) -> bool {
        true
    }

    fn on_start(&mut self, state: &mut SimState, _ctx: &mut RuleContext) {
        self.new_ring = state.balls.first().and_then(|ball| {
            let thickness = ball.thickness?;
            Some((ball.radius, thickness, ball.vel.length()))
        });
    }

    fn on_substep_end(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        let Some((radius, thickness, speed)) = self.new_ring else {
            return;
        };
        let arena = state.arena.radius;
        let Some(index) = state
            .balls_in_arena()
            .position(|ball| ball.thickness.is_some() && ball.radius >= arena)
        else {
            return;
        };
        let ring = state.balls.remove(index);
        let ring_thickness = ring.thickness.expect("only Rings settle");
        state.settles.push(Settled {
            substep: ctx.substep + 1,
            colour: ring.colour,
            surface: Surface::Boundary,
            radius: arena,
            thickness: ring_thickness,
        });
        state.arena.radius = arena - ring_thickness;
        if state.arena.radius <= radius {
            state.arena.solid = true;
            return;
        }
        let dir = ctx.rng.range_f64(0.0, std::f64::consts::TAU);
        let new = state.spawn(Vec2::ZERO, Vec2::from_angle(dir) * speed, radius);
        state.balls[new].thickness = Some(thickness);
    }
}
