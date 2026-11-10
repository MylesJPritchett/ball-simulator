//! Gap: an opening in the Boundary that turns at a steady rate. Balls reach
//! it and pass through instead of bouncing (see `Arena::gaps`).
//!
//! The Gap's starting angle comes from the Seed, drawn when the simulation
//! starts, unless the Scenario sets `start_angle` (which may be a range).

use serde::Deserialize;

use super::{finite, params, positive_finite, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{Gap as Opening, SimState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    /// Width of the opening in degrees (at most 360).
    width: f64,
    /// Degrees per second the Gap turns (y down, so positive is clockwise on
    /// screen); 0 keeps it still.
    rotation_speed: f64,
    /// Where the Gap's centre starts, in degrees (y down, so 90 is straight
    /// down); omitted, the Seed picks any angle.
    #[serde(default)]
    start_angle: Option<f64>,
}

pub struct Gap {
    /// In radians.
    width: f64,
    /// In radians per second.
    rotation_speed: f64,
    /// In radians; `None` to draw it from the Seed.
    start_angle: Option<f64>,
    /// This Gap's index in `state.arena.gaps`, once `on_start` has opened it.
    index: usize,
}

impl Gap {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        let width = positive_finite(spec, "width", p.width)?;
        if width > 360.0 {
            return Err(format!(
                "'{}': width must be at most 360 degrees, got {width}",
                spec.kind
            ));
        }
        Ok(Gap {
            width: width.to_radians(),
            rotation_speed: finite(spec, "rotation_speed", p.rotation_speed)?.to_radians(),
            start_angle: p
                .start_angle
                .map(|a| finite(spec, "start_angle", a).map(f64::to_radians))
                .transpose()?,
            index: 0,
        })
    }
}

impl Rule for Gap {
    fn lets_balls_out(&self) -> bool {
        true
    }

    fn on_start(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        let gaps = &mut state.arena.gaps;
        gaps.push(Opening {
            angle: self
                .start_angle
                .unwrap_or_else(|| ctx.rng.range_f64(0.0, std::f64::consts::TAU)),
            width: self.width,
        });
        self.index = gaps.len() - 1;
    }

    fn on_substep(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        state.arena.gaps[self.index].angle += self.rotation_speed * ctx.dt;
    }
}
