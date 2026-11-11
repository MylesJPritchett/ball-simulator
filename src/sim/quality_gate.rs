//! The Quality Gate: judges a finished run from its record so dull Seeds are
//! never rendered. Thresholds come from the Scenario's `[quality_gate]` table
//! ([`crate::scenario::QualityGateSpec`]); the verdict lands on `SimRecord::quality_gate`.

use serde::Serialize;

use super::end_condition::TIME_CAP;
use super::record::{substep_to_seconds, SimRecord};
use crate::scenario::Scenario;

/// Whether a Seed may be rendered.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Verdict {
    Accepted,
    Rejected(Rejection),
}

impl Verdict {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Verdict::Accepted)
    }

    pub fn rejection(&self) -> Option<&Rejection> {
        match self {
            Verdict::Accepted => None,
            Verdict::Rejected(r) => Some(r),
        }
    }
}

/// Why a Seed was rejected. Durations are Video length in seconds, including
/// the Ending's Play-out and hold.
#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
pub enum Rejection {
    #[error("the End Condition never fired before the {time_cap} s time cap")]
    NeverEnded { time_cap: f64 },
    #[error("the Video is too short ({seconds:.1} s, minimum {min} s)")]
    TooShort { seconds: f64, min: f64 },
    #[error("the Video is too long ({seconds:.1} s, maximum {max} s)")]
    TooLong { seconds: f64, max: f64 },
    #[error("too few Boundary Hits ({hits}, minimum {min})")]
    TooQuiet { hits: usize, min: usize },
    #[error("the Film's final stretch drags on ({seconds:.1} s from under 1% left to cleared, maximum {max} s)")]
    FinalStretchTooLong { seconds: f64, max: f64 },
    #[error("a ball ends too big ({fraction:.2} of the Arena radius, maximum {max})")]
    BallTooBig { fraction: f64, max: f64 },
}

/// The largest ball's radius when the run ended, as a fraction of the Arena
/// radius then.
fn ball_radius_at_end(record: &SimRecord) -> f64 {
    let largest = record
        .final_balls
        .iter()
        .map(|b| b.radius)
        .fold(0.0, f64::max);
    largest / record.final_arena_radius
}

/// The Film's final stretch starts when Film left first drops below this.
pub const FINAL_STRETCH_BELOW: f64 = 0.01;

/// Seconds from Film left first dropping below [`FINAL_STRETCH_BELOW`] to the
/// Film being cleared, or to the End Condition if it is not cleared by then.
/// `None` without a Film or if it never got that low before the End Condition.
fn final_stretch(record: &SimRecord) -> Option<f64> {
    let end = record.end.substep;
    let before_end = || record.film_left.iter().filter(|s| s.substep <= end);
    let start = before_end().find(|s| s.left < FINAL_STRETCH_BELOW)?.substep;
    let cleared = before_end()
        .find(|s| s.left == 0.0)
        .map_or(end, |s| s.substep);
    Some(substep_to_seconds(cleared - start))
}

/// Judges a run of `scenario` (with this Seed's ranges resolved) from its record.
pub fn judge(scenario: &Scenario, record: &SimRecord) -> Verdict {
    let gate = &scenario.quality_gate;
    let seconds = record.ending.cut_time();
    let hits = record.boundary_hits.len();
    // The time cap is only a safety net: a run that reaches it never ended.
    let rejection = if record.end.condition == TIME_CAP {
        Some(Rejection::NeverEnded {
            time_cap: scenario.time_cap,
        })
    } else if seconds < gate.min_duration {
        Some(Rejection::TooShort {
            seconds,
            min: gate.min_duration,
        })
    } else if seconds > gate.max_duration {
        Some(Rejection::TooLong {
            seconds,
            max: gate.max_duration,
        })
    } else if hits < gate.min_boundary_hits {
        Some(Rejection::TooQuiet {
            hits,
            min: gate.min_boundary_hits,
        })
    } else if let Some((seconds, max)) = gate
        .max_final_stretch
        .and_then(|max| Some((final_stretch(record)?, max)))
        .filter(|(seconds, max)| seconds > max)
    {
        Some(Rejection::FinalStretchTooLong { seconds, max })
    } else if let Some((fraction, max)) = gate
        .max_ball_radius_at_end
        .map(|max| (ball_radius_at_end(record), max))
        .filter(|(fraction, max)| fraction > max)
    {
        Some(Rejection::BallTooBig { fraction, max })
    } else {
        None
    };
    rejection.map_or(Verdict::Accepted, Verdict::Rejected)
}
