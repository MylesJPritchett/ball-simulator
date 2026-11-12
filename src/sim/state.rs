//! Simulation state: the Arena and its balls.

use std::f64::consts::{PI, TAU};
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use serde::Serialize;

use super::record::SettleEvent;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }

    pub fn from_angle(angle: f64) -> Self {
        Vec2::new(angle.cos(), angle.sin())
    }

    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// This vector rotated by `angle` radians.
    pub fn rotated(self, angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Vec2::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        *self = *self + o;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Vec2) {
        *self = *self - o;
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

/// Circular Arena centred on the origin. Its edge is the Boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct Arena {
    pub radius: f64,
    /// Openings in the Boundary, added by Rules that can let a ball out (see
    /// `Rule::lets_balls_out`). Balls pass through them instead of bouncing.
    pub gaps: Vec<Gap>,
    /// Cracks in the Boundary, in the order they were made (see the `crack`
    /// Rule). Only drawn: balls bounce off a cracked Boundary as usual.
    pub cracks: Vec<Crack>,
    /// Set once the Boundary has shattered (see the `crack` Rule). It is then
    /// open all round, and drawn as shards flying out from where it broke.
    pub shatter: Option<Shatter>,
    /// `Some` for an Arena with a Peg this big: a fixed solid disc at its
    /// centre that a Ring's inner edge bounces off.
    pub peg_radius: Option<f64>,
    /// Set once Rings have settled into the Arena until there is no room for
    /// another (see the `settle` Rule): what is left of it is drawn filled.
    pub solid: bool,
}

/// An opening in the Boundary: the arc `width` radians wide centred on
/// `angle` (radians, `atan2(y, x)`, y down, like a Boundary Hit's angle). A
/// width of a full turn or more opens the whole Boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gap {
    pub angle: f64,
    pub width: f64,
}

/// A crack in the Boundary where a Boundary Hit landed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crack {
    /// Where on the Boundary (radians, like a Boundary Hit's angle).
    pub angle: f64,
    /// How bad it is, from 0 (a hairline) to 1 (made at max speed); the
    /// renderer draws bigger cracks longer and more branched.
    pub size: f64,
}

/// Where and when the Boundary shattered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shatter {
    /// Where on the Boundary it broke (radians, like a Boundary Hit's angle).
    pub angle: f64,
    /// The substep it broke on.
    pub substep: u64,
}

impl Arena {
    pub fn new(radius: f64) -> Self {
        Arena {
            radius,
            gaps: Vec::new(),
            cracks: Vec::new(),
            shatter: None,
            peg_radius: None,
            solid: false,
        }
    }

    /// How far from the Arena's centre a ball this big (and, for a Ring, this
    /// thick) can be: until its outer edge meets the Boundary or, for a Ring
    /// round a Peg, its inner edge meets the Peg.
    pub fn room(&self, radius: f64, thickness: Option<f64>) -> f64 {
        let boundary = self.radius - radius;
        match (self.peg_radius, thickness) {
            (Some(peg), Some(thickness)) => boundary.min((radius - thickness - peg).max(0.0)),
            _ => boundary,
        }
    }

    /// Whether the Boundary is open (in a Gap) at `angle`.
    pub fn is_open_at(&self, angle: f64) -> bool {
        self.gaps.iter().any(|gap| {
            // Angular distance from the Gap's centre, in [-PI, PI).
            let off = (angle - gap.angle + PI).rem_euclid(TAU) - PI;
            off.abs() <= gap.width / 2.0
        })
    }

    /// The arcs of the Boundary left between its Gaps, as (start, end) angles
    /// with start < end, in increasing angle. One whole turn without Gaps;
    /// none if the Gaps open it all.
    pub fn solid_arcs(&self) -> Vec<(f64, f64)> {
        // Each Gap as a (start, end) interval with start in [0, TAU), merged
        // where they overlap.
        let mut open: Vec<(f64, f64)> = self
            .gaps
            .iter()
            .map(|gap| {
                let start = (gap.angle - gap.width / 2.0).rem_euclid(TAU);
                (start, start + gap.width.min(TAU))
            })
            .collect();
        open.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (start, end) in open {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        // The last interval can run on past a full turn into the first ones.
        while merged.len() > 1 && merged[merged.len() - 1].1 >= merged[0].0 + TAU {
            let first = merged.remove(0);
            let last = merged.last_mut().expect("more than one interval");
            last.1 = last.1.max(first.1 + TAU);
        }
        let Some(&(first_start, _)) = merged.first() else {
            return vec![(0.0, TAU)];
        };
        let mut arcs = Vec::new();
        for (i, &(_, end)) in merged.iter().enumerate() {
            let next = merged.get(i + 1).map_or(first_start + TAU, |gap| gap.0);
            if next > end {
                arcs.push((end, next));
            }
        }
        arcs
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ball {
    /// Stable identifier, unique within a simulation run.
    pub id: u32,
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f64,
    /// Index into the Video's palette (render wraps it to the palette length).
    pub colour: u32,
    /// Whether the ball has passed through the Boundary (through a Gap) and
    /// is on its way out. The Boundary no longer holds it and forces no
    /// longer act on it, so it flies straight out.
    pub escaping: bool,
    /// `Some` for a Ring: how thick it is (its hole's radius is `radius`
    /// minus this). It stays the same when the radius changes. Without a Peg
    /// the hole is only how a Ring looks; it moves like a solid ball.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickness: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimState {
    pub arena: Arena,
    pub balls: Vec<Ball>,
    /// Balls that have vanished from the Arena, as they were just before,
    /// in the order they vanished.
    pub vanished: Vec<Ball>,
    /// Rings that have settled, in order (see the `settle` Rule).
    pub settles: Vec<SettleEvent>,
    /// Balls marked by [`SimState::vanish`] this substep, still in `balls`.
    vanishing: Vec<u32>,
    next_ball_id: u32,
}

impl SimState {
    pub fn new(arena: Arena) -> Self {
        SimState {
            arena,
            balls: Vec::new(),
            vanished: Vec::new(),
            settles: Vec::new(),
            vanishing: Vec::new(),
            next_ball_id: 0,
        }
    }

    /// Scales down any ball moving faster than `max_speed`, keeping its direction.
    pub fn limit_speed(&mut self, max_speed: f64) {
        for ball in &mut self.balls {
            let speed = ball.vel.length();
            if speed > max_speed {
                ball.vel = ball.vel * (max_speed / speed);
            }
        }
    }

    /// The balls still in the Arena: all but the escaping ones, which have
    /// passed through the Boundary and which the Arena (its gravity, drag and
    /// Boundary) no longer acts on.
    pub fn balls_in_arena(&self) -> impl Iterator<Item = &Ball> {
        self.balls.iter().filter(|ball| !ball.escaping)
    }

    /// As [`SimState::balls_in_arena`], mutably.
    pub fn balls_in_arena_mut(&mut self) -> impl Iterator<Item = &mut Ball> {
        self.balls.iter_mut().filter(|ball| !ball.escaping)
    }

    /// Keeps every ball inside the Arena after Rules have changed radii: a
    /// ball is never larger than the Arena, a Ring round a Peg never has a
    /// hole smaller than the Peg, and one poking past the Boundary (or a Ring
    /// whose inner edge overlaps the Peg) is moved straight inward until it
    /// touches it. Velocity is unchanged.
    /// Escaping balls are left to carry on out.
    pub fn contain(&mut self) {
        let SimState { arena, balls, .. } = self;
        for ball in balls.iter_mut().filter(|ball| !ball.escaping) {
            if let (Some(peg), Some(thickness)) = (arena.peg_radius, ball.thickness) {
                ball.radius = ball.radius.max(peg + thickness);
            }
            ball.radius = ball.radius.min(arena.radius);
            let limit = arena.room(ball.radius, ball.thickness);
            let dist = ball.pos.length();
            // The tolerance leaves balls that Boundary resolution already
            // placed on the Boundary untouched (bit for bit).
            if dist > limit + 1e-9 {
                ball.pos = ball.pos * (limit / dist);
            }
        }
    }

    /// Marks the ball at `index` as vanished. It stays in `balls` (so ball
    /// indices stay valid while Boundary Hits are handled) until the simulation
    /// removes it at the end of the substep, and is recorded in `vanished` as
    /// it is now.
    pub fn vanish(&mut self, index: usize) {
        let ball = &self.balls[index];
        if !self.vanishing.contains(&ball.id) {
            self.vanishing.push(ball.id);
            self.vanished.push(ball.clone());
        }
    }

    /// Removes the balls marked by [`SimState::vanish`] from the Arena.
    pub(super) fn remove_vanished(&mut self) {
        if !self.vanishing.is_empty() {
            let gone = std::mem::take(&mut self.vanishing);
            self.balls.retain(|ball| !gone.contains(&ball.id));
        }
    }

    /// Adds a ball with a fresh id and returns its index in `balls`.
    pub fn spawn(&mut self, pos: Vec2, vel: Vec2, radius: f64) -> usize {
        let id = self.next_ball_id;
        self.next_ball_id += 1;
        self.balls.push(Ball {
            id,
            pos,
            vel,
            radius,
            colour: 0,
            escaping: false,
            thickness: None,
        });
        self.balls.len() - 1
    }
}
