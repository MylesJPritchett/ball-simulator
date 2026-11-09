//! Change colour: a ball moves to the next palette colour on each Boundary Hit.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, World};

pub struct ChangeColour;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {}

impl ChangeColour {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let Params {} = params(spec)?;
        Ok(ChangeColour)
    }
}

impl Rule for ChangeColour {
    fn on_boundary_hit(
        &mut self,
        world: &mut World,
        ball: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        // Render wraps the index to the palette length, so cycling through any
        // palette needs no knowledge of its size here.
        let ball = &mut world.balls[ball];
        ball.colour = ball.colour.wrapping_add(1);
    }
}
