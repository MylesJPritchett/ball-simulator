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
mod world;

pub use ending::Ending;
pub use quality_gate::{Rejection, Verdict};
pub use record::{BallCountSample, BoundaryHit, EndEvent, SimRecord, SimStats};
pub use world::{Arena, Ball, Vec2, World};

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

/// Pitch for every Boundary Hit until pitch modes exist (A4).
pub const DEFAULT_PITCH_HZ: f64 = 440.0;

/// Runs a Scenario headlessly with a Seed until an End Condition fires.
pub fn run(scenario: &Scenario, seed: u64) -> SimRecord {
    let mut sim = Simulation::new(scenario, seed);
    while !sim.is_ended() {
        sim.step();
    }
    sim.into_record()
}

pub struct Simulation {
    seed: u64,
    /// The Scenario with this Seed's ranges resolved.
    scenario: Scenario,
    parameters: Vec<ResolvedParam>,
    palette: &'static Palette,
    world: World,
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
}

impl Simulation {
    /// The Scenario must have been validated (any `Scenario` from the
    /// `scenario` module constructors has been).
    pub fn new(scenario: &Scenario, seed: u64) -> Self {
        let mut rng = Rng::from_seed(seed);
        // Ranges resolve first, before any initial-state draws.
        let (resolved, parameters) = scenario.resolve(&mut rng);
        let scenario = &resolved;
        let rules = rules::build_all(scenario).expect("Scenario Rules were validated");
        let end_condition =
            end_condition::build(scenario).expect("Scenario End Condition was validated");

        let mut world = World::new(Arena {
            radius: scenario.arena.radius,
        });
        // The ball starts somewhere in the inner half of the Arena, already moving.
        let ball = &scenario.ball;
        let max_start = (scenario.arena.radius - ball.radius) * 0.5;
        let start_angle = rng.range_f64(0.0, std::f64::consts::TAU);
        let start_dist = max_start * rng.next_f64().sqrt();
        let dir = rng.range_f64(0.0, std::f64::consts::TAU);
        world.spawn(
            Vec2::from_angle(start_angle) * start_dist,
            Vec2::from_angle(dir) * ball.speed,
            ball.radius,
        );

        let ball_count = vec![BallCountSample {
            substep: 0,
            count: world.balls.len(),
        }];

        Simulation {
            seed,
            parameters,
            palette: palette::for_video(scenario.look.palette.as_deref(), seed),
            world,
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
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// The Scenario as this Seed resolved it: per-Seed values (Arena radius,
    /// ball size...) come from here, not from the template's fields.
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

    pub fn is_ended(&self) -> bool {
        self.end.is_some()
    }

    pub fn end(&self) -> Option<&EndEvent> {
        self.end.as_ref()
    }

    /// Advances one frame (up to [`SUBSTEPS_PER_FRAME`] substeps, fewer if the
    /// run ends mid-frame).
    pub fn step_frame(&mut self) {
        for _ in 0..SUBSTEPS_PER_FRAME {
            if self.is_ended() {
                break;
            }
            self.step();
        }
    }

    /// Advances one substep. Does nothing once the run has ended.
    pub fn step(&mut self) {
        if self.is_ended() {
            return;
        }
        let dt = 1.0 / SUBSTEP_HZ as f64;

        let mut ctx = RuleContext {
            rng: &mut self.rng,
            substep: self.substep,
            dt,
        };
        for rule in &mut self.rules {
            rule.on_substep(&mut self.world, &mut ctx);
        }
        self.world.limit_speed(self.max_speed);

        let completed = self.substep + 1;
        let mut hits = Vec::new();
        for (index, ball) in self.world.balls.iter_mut().enumerate() {
            ball.pos += ball.vel * dt;
            if let Some(angle) = resolve_boundary(ball, &self.world.arena) {
                hits.push((
                    index,
                    BoundaryHit {
                        substep: completed,
                        ball_id: ball.id,
                        angle,
                        pitch: DEFAULT_PITCH_HZ,
                        speed: ball.vel.length(),
                        colour: ball.colour,
                    },
                ));
            }
        }

        for (index, mut hit) in hits {
            for rule in &mut self.rules {
                rule.on_boundary_hit(&mut self.world, index, &hit, &mut ctx);
                // Population Cap: drop balls a Rule just added beyond the cap. The
                // count was within the cap before this hit, so only new balls go
                // and pending hit indices stay valid.
                self.world.balls.truncate(self.population_cap);
            }
            hit.colour = self.world.balls[index].colour;
            self.boundary_hits.push(hit);
        }
        self.world.limit_speed(self.max_speed);

        self.substep = completed;
        self.measure();
        self.check_end();
    }

    fn measure(&mut self) {
        let arena = &self.world.arena;
        for ball in &self.world.balls {
            let overshoot = ball.pos.length() + ball.radius - arena.radius;
            if overshoot > self.stats.max_boundary_overshoot {
                self.stats.max_boundary_overshoot = overshoot;
            }
            self.stats.max_speed = self.stats.max_speed.max(ball.vel.length());
        }
        let count = self.world.balls.len();
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
            .filter(|cond| cond.is_met(&self.world, self.substep))
            .map(|cond| cond.id());
        let fired = scenario_end
            .or((self.substep >= self.time_cap_substeps).then_some(end_condition::TIME_CAP));
        if let Some(id) = fired {
            self.end = Some(EndEvent {
                condition: id.to_string(),
                substep: self.substep,
            });
        }
    }

    /// Finishes the run. Call once [`Simulation::is_ended`] is true.
    pub fn into_record(self) -> SimRecord {
        let end = self.end.expect("into_record called before the run ended");
        let mut record = SimRecord {
            seed: self.seed,
            parameters: self.parameters,
            palette: self.palette.name.to_string(),
            final_state_hash: hash_world(&self.world, self.substep),
            final_arena_radius: self.world.arena.radius,
            boundary_hits: self.boundary_hits,
            ball_count: self.ball_count,
            ending: Ending::new(end.substep),
            end,
            stats: self.stats,
            final_balls: self.world.balls,
            quality_gate: Verdict::Accepted,
        };
        record.quality_gate = quality_gate::judge(&self.scenario, &record);
        record
    }
}

/// Exact circle Boundary resolution: if the ball is touching or past the
/// Boundary, reflect its velocity about the Boundary normal (if moving outward)
/// and push it back to exactly touch the Boundary from inside. Returns the hit
/// angle on the Boundary if it hit.
fn resolve_boundary(ball: &mut Ball, arena: &Arena) -> Option<f64> {
    let limit = arena.radius - ball.radius;
    let dist = ball.pos.length();
    if dist < limit || dist == 0.0 {
        return None;
    }
    let normal = ball.pos * (1.0 / dist);
    let outward = ball.vel.dot(normal);
    if outward > 0.0 {
        ball.vel -= normal * (2.0 * outward);
    }
    ball.pos = normal * limit;
    Some(normal.angle())
}

/// FNV-1a over the bit patterns of the final state. Stable across runs and
/// platforms with the same float behaviour.
fn hash_world(world: &World, substep: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |v: u64| {
        for byte in v.to_le_bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(substep);
    feed(world.arena.radius.to_bits());
    feed(world.balls.len() as u64);
    for b in &world.balls {
        feed(b.id as u64);
        feed(b.pos.x.to_bits());
        feed(b.pos.y.to_bits());
        feed(b.vel.x.to_bits());
        feed(b.vel.y.to_bits());
        feed(b.radius.to_bits());
    }
    h
}
