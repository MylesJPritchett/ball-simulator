//! End Conditions. Each Scenario picks one via `[end_condition] kind = "..."`,
//! and it must reach a Terminal State (ADR 0002). The time cap is built into
//! the simulation as a safety net, not a choosable End Condition: a Seed that
//! reaches it is rejected by the Quality Gate.
//!
//! Adding an End Condition: implement [`EndCondition`] (in this file or a
//! sibling module) and add a match arm in [`build`]. Override
//! [`EndCondition::ending`] to give its Ending a Play-out, or to drop the
//! flash or the final chord.

use crate::rules::Rule;
use crate::scenario::{RuleSpec, Scenario};
use crate::sim::ending::{Accent, EndingStyle, Finale, PlayOut};
use crate::sim::SimState;

/// The id a run ends with when its End Condition never fired.
pub const TIME_CAP: &str = "time_cap";
pub const ARENA_FILLED: &str = "arena_filled";
pub const ARENA_COVERED: &str = "arena_covered";
pub const VANISHED: &str = "vanished";
pub const ESCAPED: &str = "escaped";
pub const MELTDOWN: &str = "meltdown";
pub const SOLID: &str = "solid";

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

/// End Conditions that stopped at a hidden threshold. They were deleted
/// (ADR 0002); Scenarios that still name one are refused with this hint.
const THRESHOLDS: [(&str, &str); 4] = [
    ("population_cap", MELTDOWN),
    ("max_speed_reached", ESCAPED),
    ("min_radius_reached", VANISHED),
    ("arena_min_reached", ARENA_FILLED),
];

/// Builds the Scenario's End Condition.
pub fn build(scenario: &Scenario) -> Result<Box<dyn EndCondition>, String> {
    let spec = &scenario.end_condition;
    match spec.kind.as_str() {
        TIME_CAP => Err(format!(
            "End Condition '{TIME_CAP}' is not a Terminal State: the time cap is only a \
             safety net, and a Seed that reaches it is rejected"
        )),
        ARENA_FILLED => {
            no_params(spec)?;
            Ok(Box::new(ArenaFilled))
        }
        ARENA_COVERED => {
            no_params(spec)?;
            Ok(Box::new(ArenaCovered))
        }
        VANISHED => {
            no_params(spec)?;
            Ok(Box::new(Vanished))
        }
        ESCAPED => {
            no_params(spec)?;
            needs_rule(
                scenario,
                |rule| rule.lets_balls_out(),
                "can let a ball out",
                crate::rules::GAP,
            )?;
            Ok(Box::new(Escaped::new(scenario.arena.radius)))
        }
        MELTDOWN => {
            no_params(spec)?;
            needs_rule(
                scenario,
                |rule| rule.adds_balls(),
                "adds balls",
                crate::rules::SPLIT,
            )?;
            Ok(Box::new(Meltdown {
                cap: scenario.population_cap,
            }))
        }
        SOLID => {
            no_params(spec)?;
            needs_rule(
                scenario,
                |rule| rule.settles_rings(),
                "settles Rings",
                crate::rules::SETTLE,
            )?;
            if scenario.ball.thickness.is_none() {
                return Err(format!(
                    "End Condition '{SOLID}' needs a Ring ball (give the ball a thickness)"
                ));
            }
            Ok(Box::new(Solid))
        }
        kind => {
            if let Some((_, instead)) = THRESHOLDS.iter().find(|(old, _)| *old == kind) {
                return Err(format!(
                    "End Condition '{kind}' is not a Terminal State and was removed \
                     (e.g. use '{instead}' instead)"
                ));
            }
            Err(format!("unknown End Condition '{kind}'"))
        }
    }
}

/// Refuses the Scenario's End Condition unless one of its Rules passes
/// `can`, which `does` describes (e.g. "adds balls"), naming `example` as a
/// Rule that would.
fn needs_rule(
    scenario: &Scenario,
    can: impl Fn(&dyn Rule) -> bool,
    does: &str,
    example: &str,
) -> Result<(), String> {
    let rules = crate::rules::build_all(scenario)?;
    if rules.iter().any(|rule| can(rule.as_ref())) {
        Ok(())
    } else {
        Err(format!(
            "End Condition '{}' needs a Rule that {does} (e.g. '{example}')",
            scenario.end_condition.kind
        ))
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
        // grid is only sampled once that sum could reach COVERED_FRACTION.
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

/// How long a vanished ball's pop plays out before the freeze.
pub const POP_SECONDS: f64 = 0.3;

/// A ball has vanished from the Arena (e.g. shrunk to nothing by the shrink
/// Rule). It pops: a ring expands and fades where it was, under the flash and
/// the final chord.
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
            accent: Accent::AtFire,
            finale: Finale::Pop,
            ..EndingStyle::default()
        }
    }
}

/// The longest an escape plays out, should the ball be too slow to leave the
/// frame.
pub const ESCAPE_PLAY_OUT_MAX_SECONDS: f64 = 3.0;

/// A ball has escaped: it is fully outside the Arena, through a Gap or
/// whatever else a Rule opened in the Boundary. The Play-out runs until every
/// escaping ball has left the Video frame; the flash and the final chord wait
/// for the freeze, when the escape is complete on screen.
struct Escaped {
    /// The Video frame in simulation units, as (left, top, right, bottom)
    /// around the Arena's centre.
    frame: (f64, f64, f64, f64),
}

impl Escaped {
    /// `arena_radius` is the Arena's starting radius, which the renderer
    /// scales to the laid-out Arena.
    fn new(arena_radius: f64) -> Self {
        let arena = crate::layout::arena();
        let scale = arena_radius / arena.radius as f64;
        let (cx, cy) = (arena.center_x as f64, arena.center_y as f64);
        let (width, height) = (crate::layout::WIDTH as f64, crate::layout::HEIGHT as f64);
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
            accent: Accent::AtFreeze,
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

/// How long the Meltdown plays out before the cut to its broken frame.
pub const MELTDOWN_SECONDS: f64 = 1.5;

/// The ball count has reached the Scenario's Population Cap and the Video
/// appears to break down under its own load. Staged, not real: rendering is
/// offline (ADR 0001), so the renderer and audio draw the breakdown from the
/// Seed. No flash and no final chord, so the breakdown does not resolve neatly.
struct Meltdown {
    cap: usize,
}

impl EndCondition for Meltdown {
    fn id(&self) -> &'static str {
        MELTDOWN
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        state.balls.len() >= self.cap
    }

    fn ending(&self) -> EndingStyle {
        EndingStyle {
            play_out: PlayOut::For {
                seconds: MELTDOWN_SECONDS,
            },
            flash: false,
            final_chord: false,
            accent: Accent::AtFire,
            finale: Finale::Meltdown,
        }
    }
}

/// The Arena is solid: Rings have settled into it until there is no room for
/// another (see the `settle` Rule).
struct Solid;

impl EndCondition for Solid {
    fn id(&self) -> &'static str {
        SOLID
    }

    fn is_met(&self, state: &SimState, _substep: u64) -> bool {
        state.arena.solid
    }
}
