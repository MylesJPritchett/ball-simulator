//! End Conditions. The time cap is built into the simulation and always
//! applies; a Scenario may add one more via `[end_condition] kind = "..."`.
//!
//! Adding an End Condition: implement [`EndCondition`] (in this file or a
//! sibling module) and add a match arm in [`build`]. Override
//! [`EndCondition::ending`] to give its Ending a Play-out, or to drop the
//! flash or the final chord.

use serde::Deserialize;

use crate::scenario::{RuleSpec, Scenario};
use crate::sim::ending::{Effect, EndingStyle, PlayOut};
use crate::sim::SimState;

pub const TIME_CAP: &str = "time_cap";
pub const POPULATION_CAP: &str = "population_cap";
pub const ARENA_FILLED: &str = "arena_filled";
pub const ARENA_COVERED: &str = "arena_covered";
pub const MAX_SPEED_REACHED: &str = "max_speed_reached";
pub const MIN_RADIUS_REACHED: &str = "min_radius_reached";
pub const ARENA_MIN_REACHED: &str = "arena_min_reached";
pub const VANISHED: &str = "vanished";
pub const ESCAPED: &str = "escaped";

pub trait EndCondition {
    /// Id written to `SimRecord::end.condition`.
    fn id(&self) -> &'static str;

    /// Checked at the end of every substep, after Rules have run.
    fn is_met(&self, state: &SimState, substep: u64) -> bool;

    /// The shape of the Ending once this fires. Default: no Play-out, with a
    /// flash and the final chord.
    fn ending(&self) -> EndingStyle {
        EndingStyle::default()
    }

    /// For a [`PlayOut::Until`] Play-out: whether the Terminal State has
    /// finished on screen, so the action can freeze. Checked at the end of
    /// every substep from the one this fired on.
    fn play_out_done(&self, _state: &SimState, _substep: u64) -> bool {
        true
    }
}

/// Builds the Scenario's End Condition. `time_cap` builds nothing: the
/// time cap built into the simulation alone ends the Video.
pub fn build(scenario: &Scenario) -> Result<Option<Box<dyn EndCondition>>, String> {
    let spec = &scenario.end_condition;
    match spec.kind.as_str() {
        TIME_CAP => Ok(None),
        POPULATION_CAP => {
            no_params(spec)?;
            Ok(Some(Box::new(PopulationCap {
                cap: scenario.population_cap,
            })))
        }
        ARENA_FILLED => {
            no_params(spec)?;
            Ok(Some(Box::new(ArenaFilled)))
        }
        ARENA_COVERED => {
            no_params(spec)?;
            Ok(Some(Box::new(ArenaCovered)))
        }
        MAX_SPEED_REACHED => {
            no_params(spec)?;
            Ok(Some(Box::new(MaxSpeedReached {
                max_speed: scenario.ball.max_speed,
            })))
        }
        MIN_RADIUS_REACHED => Ok(Some(Box::new(MinRadiusReached {
            min_radius: min_radius(spec)?,
        }))),
        ARENA_MIN_REACHED => Ok(Some(Box::new(ArenaMinReached {
            min_radius: min_radius(spec)?,
        }))),
        VANISHED => {
            no_params(spec)?;
            Ok(Some(Box::new(Vanished)))
        }
        ESCAPED => {
            no_params(spec)?;
            let rules = crate::rules::build_all(scenario)?;
            if !rules.iter().any(|rule| rule.lets_balls_out()) {
                return Err(format!(
                    "End Condition '{ESCAPED}' needs a Rule that can let a ball out (e.g. '{}')",
                    crate::rules::GAP
                ));
            }
            Ok(Some(Box::new(Escaped::new(scenario.arena.radius))))
        }
        other => Err(format!("unknown End Condition '{other}'")),
    }
}

fn no_params(spec: &RuleSpec) -> Result<(), String> {
    match spec.params.keys().next() {
        Some(key) => Err(format!(
            "End Condition '{}' has no parameter '{key}'",
            spec.kind
        )),
        None => Ok(()),
    }
}

/// Reads the one `min_radius` parameter (finite, > 0).
fn min_radius(spec: &RuleSpec) -> Result<f64, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Params {
        min_radius: f64,
    }
    let p: Params = crate::rules::params(spec)?;
    crate::rules::positive_finite(spec, "min_radius", p.min_radius)
}

/// The ball count has reached the Scenario's Population Cap (which the
/// simulation never lets it exceed).
struct PopulationCap {
    cap: usize,
}

impl EndCondition for PopulationCap {
    fn id(&self) -> &'static str {
        POPULATION_CAP
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        state.balls.len() >= self.cap
    }
}

/// A ball has grown to fill the Arena. The simulation never lets a ball be
/// larger than the Arena, so "filled" means its radius has reached the Arena's.
struct ArenaFilled;

impl EndCondition for ArenaFilled {
    fn id(&self) -> &'static str {
        ARENA_FILLED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        state
            .balls
            .iter()
            .any(|ball| ball.radius >= state.arena.radius)
    }
}

/// How much of the Arena the balls must cover for it to read as full. Fixed,
/// not a parameter: below this the Arena visibly has holes (ADR 0002).
pub const COVERED_FRACTION: f64 = 0.98;

/// Width of the rim left out of the coverage measure, as a fraction of the
/// Arena radius (about the Boundary stroke's width on screen). A bouncing ball
/// covers the rim only at the instant it touches the Boundary, so even a dense
/// swarm leaves a thin scalloped gap there that does not stop the Arena
/// reading as full.
pub const RIM_MARGIN: f64 = 0.02;

/// Rows (and columns) of the grid that [`coverage`] samples the Arena on.
const COVERAGE_GRID: usize = 256;

/// The balls together cover [`COVERED_FRACTION`] of the Arena inside its
/// [`RIM_MARGIN`], with overlaps counted once.
struct ArenaCovered;

impl EndCondition for ArenaCovered {
    fn id(&self) -> &'static str {
        ARENA_COVERED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        let radius = state.arena.radius * (1.0 - RIM_MARGIN);
        // The union is never larger than the sum of the balls' areas, so the
        // grid is only sampled once that sum could reach the threshold.
        let ball_area: f64 = state.balls.iter().map(|b| b.radius * b.radius).sum();
        ball_area >= COVERED_FRACTION * radius * radius
            && coverage(state, radius) >= COVERED_FRACTION
    }
}

/// Fraction of the circle of `radius` round the Arena's centre covered by at
/// least one ball, estimated on a fixed grid over it: the share of grid cells
/// inside the circle whose centre is inside a ball. Each ball fills one span
/// of cells per grid row, so the cost grows with the balls' total height in
/// rows, not their area.
fn coverage(state: &SimState, radius: f64) -> f64 {
    let cell = 2.0 * radius / COVERAGE_GRID as f64;
    let centre = |index: usize| -radius + (index as f64 + 0.5) * cell;
    // The cells (first, last) whose centres lie within [lo, hi] on one axis.
    let cells = |lo: f64, hi: f64| {
        let first = ((lo + radius) / cell - 0.5).ceil().max(0.0);
        let last = ((hi + radius) / cell - 0.5)
            .floor()
            .min((COVERAGE_GRID - 1) as f64);
        (first <= last).then_some((first as usize, last as usize))
    };
    let half_chord = |r: f64, offset: f64| (r * r - offset * offset).max(0.0).sqrt();

    // Each row's span of cells inside the circle.
    let circle_spans: Vec<Option<(usize, usize)>> = (0..COVERAGE_GRID)
        .map(|row| {
            let w = half_chord(radius, centre(row));
            cells(-w, w)
        })
        .collect();
    let circle_cells: usize = circle_spans
        .iter()
        .flatten()
        .map(|(first, last)| last - first + 1)
        .sum();

    let mut covered = vec![false; COVERAGE_GRID * COVERAGE_GRID];
    for ball in &state.balls {
        let Some((top, bottom)) = cells(ball.pos.y - ball.radius, ball.pos.y + ball.radius) else {
            continue;
        };
        for (row, circle_span) in circle_spans.iter().enumerate().take(bottom + 1).skip(top) {
            let Some((circle_first, circle_last)) = *circle_span else {
                continue;
            };
            let w = half_chord(ball.radius, centre(row) - ball.pos.y);
            let Some((first, last)) = cells(ball.pos.x - w, ball.pos.x + w) else {
                continue;
            };
            let (first, last) = (first.max(circle_first), last.min(circle_last));
            if first <= last {
                let start = row * COVERAGE_GRID;
                covered[start + first..=start + last].fill(true);
            }
        }
    }
    covered.iter().filter(|&&c| c).count() as f64 / circle_cells as f64
}

/// Speed limits are applied by scaling, so a ball at the limit can be a
/// rounding error under it.
const AT_LIMIT: f64 = 1.0 - 1e-9;

/// A ball has reached the Scenario's max speed (e.g. after speeding up on
/// every Boundary Hit).
struct MaxSpeedReached {
    max_speed: f64,
}

impl EndCondition for MaxSpeedReached {
    fn id(&self) -> &'static str {
        MAX_SPEED_REACHED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        let limit = self.max_speed * AT_LIMIT;
        state.balls.iter().any(|ball| ball.vel.length() >= limit)
    }
}

/// A ball has shrunk to `min_radius` (pair with the shrink Rule's minimum).
struct MinRadiusReached {
    min_radius: f64,
}

impl EndCondition for MinRadiusReached {
    fn id(&self) -> &'static str {
        MIN_RADIUS_REACHED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        state
            .balls
            .iter()
            .any(|ball| ball.radius <= self.min_radius)
    }
}

/// The Arena has shrunk to `min_radius` (pair with the shrinking_arena Rule's
/// minimum).
struct ArenaMinReached {
    min_radius: f64,
}

impl EndCondition for ArenaMinReached {
    fn id(&self) -> &'static str {
        ARENA_MIN_REACHED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        state.arena.radius <= self.min_radius
    }
}

/// How long a vanished ball's pop plays out before the freeze.
pub const POP_SECONDS: f64 = 0.3;

/// A ball has vanished from the Arena (e.g. shrunk to nothing by the shrink
/// Rule). It pops: a ring expands and fades where it was.
struct Vanished;

impl EndCondition for Vanished {
    fn id(&self) -> &'static str {
        VANISHED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        !state.vanished.is_empty()
    }

    fn ending(&self) -> EndingStyle {
        EndingStyle {
            play_out: PlayOut::For {
                seconds: POP_SECONDS,
            },
            effect: Effect::Pop,
            ..EndingStyle::default()
        }
    }
}

/// The longest an escape plays out, should the ball be too slow to leave the
/// frame.
pub const ESCAPE_PLAY_OUT_MAX_SECONDS: f64 = 3.0;

/// A ball has escaped: it is fully outside the Arena, through a Gap or
/// whatever else a Rule opened in the Boundary. The Play-out runs until every
/// escaping ball has left the Video frame.
struct Escaped {
    /// The Video frame in simulation units, as (left, top, right, bottom)
    /// around the Arena's centre.
    frame: (f64, f64, f64, f64),
}

impl Escaped {
    /// `arena_radius` is the Arena's starting radius, which the renderer
    /// scales to the laid-out Arena.
    fn new(arena_radius: f64) -> Self {
        let arena = crate::render::layout::arena();
        let scale = arena_radius / arena.radius as f64;
        let (cx, cy) = (arena.center_x as f64, arena.center_y as f64);
        let (width, height) = (
            crate::render::layout::WIDTH as f64,
            crate::render::layout::HEIGHT as f64,
        );
        Escaped {
            frame: (
                -cx * scale,
                -cy * scale,
                (width - cx) * scale,
                (height - cy) * scale,
            ),
        }
    }
}

impl EndCondition for Escaped {
    fn id(&self) -> &'static str {
        ESCAPED
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        let arena = state.arena.radius;
        state
            .balls
            .iter()
            .any(|ball| ball.escaping && ball.pos.length() - ball.radius >= arena)
    }

    fn ending(&self) -> EndingStyle {
        EndingStyle {
            play_out: PlayOut::Until {
                max_seconds: ESCAPE_PLAY_OUT_MAX_SECONDS,
            },
            ..EndingStyle::default()
        }
    }

    fn play_out_done(&self, state: &SimState, _substep: u64) -> bool {
        let (left, top, right, bottom) = self.frame;
        state.balls.iter().filter(|ball| ball.escaping).all(|ball| {
            let (p, r) = (ball.pos, ball.radius);
            p.x + r <= left || p.y + r <= top || p.x - r >= right || p.y - r >= bottom
        })
    }
}
