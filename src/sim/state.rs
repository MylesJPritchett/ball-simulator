//! Simulation state: the Arena and its balls.

use std::f64::consts::{PI, TAU};
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

use serde::Serialize;

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
}

/// An opening in the Boundary: the arc `width` radians wide centred on
/// `angle` (radians, `atan2(y, x)`, y down, like a Boundary Hit's angle). A
/// width of a full turn or more opens the whole Boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gap {
    pub angle: f64,
    pub width: f64,
}

impl Arena {
    pub fn new(radius: f64) -> Self {
        Arena {
            radius,
            gaps: Vec::new(),
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimState {
    pub arena: Arena,
    pub balls: Vec<Ball>,
    /// Balls that have vanished from the Arena, as they were just before,
    /// in the order they vanished.
    pub vanished: Vec<Ball>,
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

    /// Keeps every ball inside the Arena after Rules have changed radii: a
    /// ball is never larger than the Arena, and one poking past the Boundary
    /// is moved straight inward until it touches it. Velocity is unchanged.
    /// Escaping balls are left to carry on out.
    pub fn contain(&mut self) {
        let arena_radius = self.arena.radius;
        for ball in self.balls.iter_mut().filter(|ball| !ball.escaping) {
            ball.radius = ball.radius.min(arena_radius);
            let limit = arena_radius - ball.radius;
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
        });
        self.balls.len() - 1
    }
}
