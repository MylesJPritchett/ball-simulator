//! Settle: a Ring that can no longer move settles, becoming part of the
//! Arena, and a new Ring takes its place.
//!
//! Without a Peg, a Ring whose radius has reached the Arena's (the simulation
//! clamps a Ring grown past the Boundary to the exact fit, which also makes it
//! concentric) settles into the Boundary: it becomes a layer of the Arena in
//! its colour, the Arena's radius shrinks by its thickness, and a new Ring
//! spawns at the centre with the first Ring's radius, thickness and speed,
//! heading in a random direction. Once the Arena is no larger than a new Ring,
//! none spawns and the Arena is solid.
//!
//! Round a Peg, a Ring whose hole has reached the Peg (the simulation clamps
//! a Ring shrunk past it to the exact fit, which also makes it concentric)
//! settles onto the Peg: it becomes a layer of the Peg in its colour, the
//! Peg's radius grows by its thickness, and a new Ring spawns concentric with
//! the Peg with its outer edge on the Boundary, the first Ring's thickness and
//! speed, heading in a random direction. Once the Peg plus one thickness
//! reaches the Boundary, a new Ring's hole would already fit the Peg: none
//! spawns and the Arena is solid.
//!
//! Only Rings settle.
//!
//! ```toml
//! [[rules]]
//! kind = "settle"   # no parameters
//! ```

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{Arena, Ball, SettleEvent, SimState, Surface, Vec2};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {}

pub struct Settle {
    /// Each new Ring is a copy of the first as it started. `None` until the
    /// start, or if the ball is solid.
    new_ring: Option<NewRing>,
}

/// The first Ring as it started, which each new Ring copies. Round a Peg, a
/// new Ring is as big as the Arena instead of `radius`.
#[derive(Clone, Copy)]
struct NewRing {
    radius: f64,
    thickness: f64,
    speed: f64,
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
            Some(NewRing {
                radius: ball.radius,
                thickness: ball.thickness?,
                speed: ball.vel.length(),
            })
        });
    }

    fn on_substep_end(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        let Some(NewRing {
            radius,
            thickness,
            speed,
        }) = self.new_ring
        else {
            return;
        };
        let stuck = |ball: &Ball| {
            ball.thickness
                .is_some_and(|thickness| is_stuck(&state.arena, ball.radius, thickness))
        };
        let Some(index) = state.balls_in_arena().position(stuck) else {
            return;
        };
        let ring = state.balls.remove(index);
        let ring_thickness = ring.thickness.expect("only Rings settle");
        let surface = match state.arena.peg_radius {
            None => Surface::Boundary,
            Some(_) => Surface::Peg,
        };
        state.settles.push(SettleEvent {
            substep: ctx.substep + 1,
            colour: ring.colour,
            surface,
            radius: ring.radius,
            thickness: ring_thickness,
        });
        let arena = &mut state.arena;
        let new_radius = match surface {
            Surface::Boundary => {
                arena.radius -= ring_thickness;
                radius
            }
            Surface::Peg => {
                *arena.peg_radius.as_mut().expect("round a Peg") += ring_thickness;
                arena.radius
            }
        };
        // Once a new Ring could not move either, the Arena is solid.
        if is_stuck(arena, new_radius, thickness) {
            arena.solid = true;
            return;
        }
        let dir = ctx.rng.range_f64(0.0, std::f64::consts::TAU);
        let new = state.spawn(Vec2::ZERO, Vec2::from_angle(dir) * speed, new_radius);
        state.balls[new].thickness = Some(thickness);
    }
}

/// Whether a Ring this big and thick can no longer move: its radius has
/// reached the Arena's or, round a Peg, its hole has reached the Peg. Exactly
/// the fits the simulation clamps to.
fn is_stuck(arena: &Arena, radius: f64, thickness: f64) -> bool {
    match arena.peg_fit(thickness) {
        Some(fit) => radius <= fit,
        None => radius >= arena.radius,
    }
}
