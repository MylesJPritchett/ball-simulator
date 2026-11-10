//! Headless, deterministic simulation.
//!
//! The one public seam is [`run`]: Scenario + Seed -> [`SimRecord`].
//! [`Simulation`] exposes the same run step by step so render can replay it
//! frame by frame; both paths execute identical code, so frames and the record
//! always agree.

pub mod end_condition;
pub mod ending;
pub mod quality_gate;
pub mod record;
pub mod rng;
mod state;

pub use ending::{Effect, Ending, EndingStyle, PlayOut};
pub use quality_gate::{Rejection, Verdict};
pub use record::{BallCountSample, BoundaryHit, EndEvent, SimRecord, SimStats};
pub use state::{Arena, Ball, Crack, Gap, Shatter, SimState, Vec2};

use crate::audio::music::Sound;
use crate::look::{palette, Palette};
use crate::rules::{self, Rule, RuleContext};
use crate::scenario::{ResolvedParam, Scenario};
use end_condition::EndCondition;
use rng::Rng;

/// Video frame rate.
pub const FPS: u64 = 60;
/// Physics substeps per Video frame.
pub const SUBSTEPS_PER_FRAME: u64 = 8;
/// Physics substep rate (480 Hz).
pub const SUBSTEP_HZ: u64 = FPS * SUBSTEPS_PER_FRAME;

/// Runs a Scenario headlessly with a Seed until an End Condition fires and its
/// Play-out (if any) is over.
pub fn run(scenario: &Scenario, seed: u64) -> SimRecord {
    let mut sim = Simulation::new(scenario, seed);
    while !sim.is_frozen() {
        sim.step();
    }
    sim.into_record()
}

pub struct Simulation {
    seed: u64,
    /// The Scenario with this Seed's ranges resolved.
    scenario: Scenario,
    parameters: Vec<ResolvedParam>,
    sound: Sound,
    palette: &'static Palette,
    state: SimState,
    rules: Vec<Box<dyn Rule>>,
    end_condition: Option<Box<dyn EndCondition>>,
    rng: Rng,
    substep: u64,
    max_speed: f64,
    time_cap_substeps: u64,
    population_cap: usize,
    boundary_hits: Vec<BoundaryHit>,
    ball_count: Vec<BallCountSample>,
    stats: SimStats,
    end: Option<EndEvent>,
    /// The fired End Condition's Ending style.
    ending_style: EndingStyle,
    /// Substep the action froze, once the Play-out is over.
    frozen_at: Option<u64>,
}

impl Simulation {
    /// The Scenario must have been validated (any `Scenario` from the
    /// `scenario` module constructors has been).
    pub fn new(scenario: &Scenario, seed: u64) -> Self {
        let mut rng = Rng::from_seed(seed);
        // Ranges resolve first, before any initial-state draws.
        let (resolved, parameters) = scenario.resolve(&mut rng);
        let scenario = &resolved;
        let mut rules = rules::build_all(scenario).expect("Scenario Rules were validated");
        let end_condition =
            end_condition::build(scenario).expect("Scenario End Condition was validated");

        let mut state = SimState::new(Arena::new(scenario.arena.radius));
        // The ball starts somewhere in the inner half of the Arena, already moving.
        let ball = &scenario.ball;
        let max_start = (scenario.arena.radius - ball.radius) * 0.5;
        let start_angle = rng.range_f64(0.0, std::f64::consts::TAU);
        let start_dist = max_start * rng.next_f64().sqrt();
        let dir = rng.range_f64(0.0, std::f64::consts::TAU);
        state.spawn(
            Vec2::from_angle(start_angle) * start_dist,
            Vec2::from_angle(dir) * ball.speed,
            ball.radius,
        );
        let mut ctx = RuleContext {
            rng: &mut rng,
            substep: 0,
            dt: 1.0 / SUBSTEP_HZ as f64,
            max_speed: ball.max_speed,
        };
        for rule in &mut rules {
            rule.on_start(&mut state, &mut ctx);
        }

        // Own RNG stream: choosing the sound draws nothing from `rng`.
        let sound = Sound::for_video(&scenario.sound, seed);

        let ball_count = vec![BallCountSample {
            substep: 0,
            count: state.balls.len(),
        }];

        Simulation {
            seed,
            parameters,
            sound,
            palette: palette::for_video(scenario.look.palette.as_deref(), seed),
            state,
            rules,
            end_condition,
            rng,
            substep: 0,
            max_speed: ball.max_speed,
            time_cap_substeps: (scenario.time_cap * SUBSTEP_HZ as f64).round() as u64,
            population_cap: scenario.population_cap,
            scenario: resolved,
            boundary_hits: Vec::new(),
            ball_count,
            stats: SimStats::default(),
            end: None,
            ending_style: EndingStyle::default(),
            frozen_at: None,
        }
    }

    pub fn state(&self) -> &SimState {
        &self.state
    }

    /// The Scenario as this Seed resolved it: per-Seed values (Arena radius,
    /// ball size...) come from here, not from the parsed Scenario's fields.
    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    /// The Video's palette.
    pub fn palette(&self) -> &'static Palette {
        self.palette
    }

    /// Every Boundary Hit so far, in order.
    pub fn boundary_hits(&self) -> &[BoundaryHit] {
        &self.boundary_hits
    }

    /// Substeps completed so far.
    pub fn substep(&self) -> u64 {
        self.substep
    }

    /// Whether the action has frozen: the End Condition has fired and its
    /// Play-out (if any) is over. The simulation no longer changes.
    pub fn is_frozen(&self) -> bool {
        self.frozen_at.is_some()
    }

    /// The End Condition that fired, once one has (the Play-out may still be
    /// running).
    pub fn end(&self) -> Option<&EndEvent> {
        self.end.as_ref()
    }

    /// The Ending that follows the End Condition, once it has fired. While an
    /// open-ended ([`PlayOut::Until`]) Play-out is still running, its freeze
    /// (and so the cut) is the Play-out's maximum; it is final once frozen.
    pub fn ending(&self) -> Option<Ending> {
        let end = self.end.as_ref()?;
        let style = &self.ending_style;
        let freeze = self
            .frozen_at
            .unwrap_or(end.substep + style.play_out.max_substeps());
        Some(Ending::new(end.substep, freeze, style))
    }

    /// Advances one frame (up to [`SUBSTEPS_PER_FRAME`] substeps, fewer if the
    /// action freezes mid-frame).
    pub fn step_frame(&mut self) {
        for _ in 0..SUBSTEPS_PER_FRAME {
            if self.is_frozen() {
                break;
            }
            self.step();
        }
    }

    /// Advances one substep. Keeps going through the Play-out after the End
    /// Condition fires; does nothing once the action has frozen.
    pub fn step(&mut self) {
        if self.is_frozen() {
            return;
        }
        let dt = 1.0 / SUBSTEP_HZ as f64;

        let mut ctx = RuleContext {
            rng: &mut self.rng,
            substep: self.substep,
            dt,
            max_speed: self.max_speed,
        };
        let radius_before = self.state.arena.radius;
        for rule in &mut self.rules {
            rule.on_substep(&mut self.state, &mut ctx);
        }
        self.state.limit_speed(self.max_speed);
        // How fast the Boundary moved inward this substep (0 for a fixed Arena).
        let closing_speed = (radius_before - self.state.arena.radius) / dt;

        let completed = self.substep + 1;
        let mut hits = Vec::new();
        let arena_radius = self.state.arena.radius;
        for (index, ball) in self.state.balls.iter_mut().enumerate() {
            ball.pos += ball.vel * dt;
            if let Some(angle) = resolve_boundary(ball, &self.state.arena, closing_speed) {
                let hit_index = self.boundary_hits.len() + hits.len();
                let pitch = self
                    .sound
                    .pitch(hit_index, angle, ball.radius / arena_radius);
                hits.push((
                    index,
                    BoundaryHit {
                        substep: completed,
                        ball_id: ball.id,
                        angle,
                        pitch,
                        speed: ball.vel.length(),
                        colour: ball.colour,
                        plays_note: false,
                    },
                ));
            }
        }

        for (index, mut hit) in hits {
            for rule in &mut self.rules {
                rule.on_boundary_hit(&mut self.state, index, &mut hit, &mut ctx);
                // Population Cap: drop balls a Rule just added beyond the cap. The
                // count was within the cap before this hit, so only new balls go
                // and pending hit indices stay valid.
                self.state.balls.truncate(self.population_cap);
            }
            hit.colour = self.state.balls[index].colour;
            self.boundary_hits.push(hit);
        }
        self.state.remove_vanished();
        // Rules may have grown balls past the Boundary.
        self.state.contain();
        self.state.limit_speed(self.max_speed);

        self.substep = completed;
        self.measure();
        if self.end.is_none() {
            self.check_end();
        }
        self.check_freeze();
    }

    fn measure(&mut self) {
        let arena = &self.state.arena;
        for ball in &self.state.balls {
            self.stats.max_speed = self.stats.max_speed.max(ball.vel.length());
            // An escaping ball has passed through the Boundary: it is meant to
            // be outside.
            if ball.escaping {
                continue;
            }
            let overshoot = ball.pos.length() + ball.radius - arena.radius;
            if overshoot > self.stats.max_boundary_overshoot {
                self.stats.max_boundary_overshoot = overshoot;
            }
        }
        let count = self.state.balls.len();
        if self.ball_count.last().map(|s| s.count) != Some(count) {
            self.ball_count.push(BallCountSample {
                substep: self.substep,
                count,
            });
        }
    }

    fn check_end(&mut self) {
        let scenario_end = self
            .end_condition
            .as_ref()
            .filter(|cond| cond.is_met(&self.state, self.substep))
            .map(|cond| (cond.id(), cond.ending()));
        // The time cap gets the default Ending.
        let fired = scenario_end.or((self.substep >= self.time_cap_substeps)
            .then(|| (end_condition::TIME_CAP, EndingStyle::default())));
        if let Some((id, style)) = fired {
            self.ending_style = style;
            self.end = Some(EndEvent {
                condition: id.to_string(),
                substep: self.substep,
            });
        }
    }

    /// Freezes the action once the fired End Condition's Play-out is over.
    fn check_freeze(&mut self) {
        let Some(end) = &self.end else {
            return;
        };
        let play_out = self.ending_style.play_out;
        let over = self.substep - end.substep >= play_out.max_substeps()
            || matches!(play_out, PlayOut::Until { .. })
                && self
                    .end_condition
                    .as_ref()
                    .is_some_and(|cond| cond.play_out_done(&self.state, self.substep));
        if over {
            self.frozen_at = Some(self.substep);
        }
    }

    /// Finishes the run. Call once [`Simulation::is_frozen`] is true.
    pub fn into_record(self) -> SimRecord {
        let ending = self
            .ending()
            .expect("into_record called before the run ended");
        let end = self.end.expect("ended");
        let mut record = SimRecord {
            seed: self.seed,
            parameters: self.parameters,
            sound: self.sound,
            palette: self.palette.name.to_string(),
            final_state_hash: hash_state(&self.state, self.substep),
            final_arena_radius: self.state.arena.radius,
            boundary_hits: self.boundary_hits,
            ball_count: self.ball_count,
            ending,
            end,
            stats: self.stats,
            final_balls: self.state.balls,
            quality_gate: Verdict::Accepted,
        };
        record.quality_gate = quality_gate::judge(&self.scenario, &record);
        record
    }
}

/// Outward speed (relative to the Boundary, units/s) below which a ball
/// touching the Boundary is resting on it rather than hitting it. Gravity
/// pushes a settled ball outward by `strength / SUBSTEP_HZ` every substep
/// (about 2 units/s for the strongest shipped gravity); a bounce this slow
/// would rise less than a unit, so nothing visible or audible is lost.
const RESTING_SPEED: f64 = 30.0;

/// Exact circle Boundary resolution. A ball touching or past the Boundary is
/// put back to exactly touch it from inside. It is a Boundary Hit only if the
/// ball is moving outward relative to the Boundary, which moves inward at
/// `closing_speed` while the Arena shrinks: then its velocity is reflected
/// about the Boundary normal in the Boundary's frame. A ball resting on
/// (outward slower than [`RESTING_SPEED`]) or already leaving the Boundary is
/// not hitting it. Returns the hit angle on the Boundary if it hit.
///
/// A ball moving outward whose contact point is in a Gap is not held: it
/// starts escaping and is never resolved against the Boundary again.
fn resolve_boundary(ball: &mut Ball, arena: &Arena, closing_speed: f64) -> Option<f64> {
    if ball.escaping {
        return None;
    }
    let limit = arena.radius - ball.radius;
    let dist = ball.pos.length();
    if dist < limit || dist == 0.0 {
        return None;
    }
    let normal = ball.pos * (1.0 / dist);
    if ball.vel.dot(normal) > 0.0 && arena.is_open_at(normal.angle()) {
        ball.escaping = true;
        return None;
    }
    ball.pos = normal * limit;
    let outward = ball.vel.dot(normal) + closing_speed;
    if outward <= 0.0 {
        return None;
    }
    if outward < RESTING_SPEED {
        // Resting contact: the ball moves with the Boundary instead of
        // bouncing off it by an invisible amount every substep.
        ball.vel -= normal * outward;
        return None;
    }
    ball.vel -= normal * (2.0 * outward);
    Some(normal.angle())
}

/// FNV-1a over the bit patterns of the final state. Stable across runs and
/// platforms with the same float behaviour.
fn hash_state(state: &SimState, substep: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |v: u64| {
        for byte in v.to_le_bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(substep);
    feed(state.arena.radius.to_bits());
    feed(state.balls.len() as u64);
    for b in &state.balls {
        feed(b.id as u64);
        feed(b.pos.x.to_bits());
        feed(b.pos.y.to_bits());
        feed(b.vel.x.to_bits());
        feed(b.vel.y.to_bits());
        feed(b.radius.to_bits());
    }
    h
}
