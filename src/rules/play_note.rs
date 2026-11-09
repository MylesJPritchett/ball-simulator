//! Play note: every Boundary Hit plays a note. Without this Rule a Scenario's
//! Boundary Hits are silent (the final chord still plays).
//!
//! ```toml
//! [[rules]]
//! kind = "play_note"
//! ```
//!
//! The pitch is assigned by the simulation (from the Video's scale and pitch
//! mode) whether or not the hit sounds; this Rule only switches the note on.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, SimState};

pub struct PlayNote;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {}

impl PlayNote {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let Params {} = params(spec)?;
        Ok(PlayNote)
    }
}

impl Rule for PlayNote {
    fn on_boundary_hit(
        &mut self,
        _state: &mut SimState,
        _ball: usize,
        hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        hit.plays_note = true;
    }
}
