//! The Quality Gate: judges a finished run from its record so dull Seeds are
//! never rendered. Thresholds come from the Scenario's `[quality_gate]` table
//! ([`QualityGateSpec`]); the verdict lands on `SimRecord::quality_gate`.

use serde::Serialize;

use super::record::SimRecord;
use crate::scenario::QualityGateSpec;

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
/// the ending's hold.
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
}

/// Judges a run. `expects_time_cap` is true when the Scenario's End Condition
/// is the time cap itself, so reaching it is the intended payoff.
pub fn judge(
    gate: &QualityGateSpec,
    expects_time_cap: bool,
    time_cap: f64,
    record: &SimRecord,
) -> Verdict {
    let seconds = record.ending.cut_time();
    let hits = record.boundary_hits.len();
    let rejection = if !expects_time_cap && record.end.condition == super::end_condition::TIME_CAP {
        Some(Rejection::NeverEnded { time_cap })
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
    } else {
        None
    };
    rejection.map_or(Verdict::Accepted, Verdict::Rejected)
}
