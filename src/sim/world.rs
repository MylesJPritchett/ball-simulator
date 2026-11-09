//! Simulation state: the Arena and its balls.

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub arena: Arena,
    pub balls: Vec<Ball>,
    next_ball_id: u32,
}

impl World {
    pub fn new(arena: Arena) -> Self {
        World {
            arena,
            balls: Vec::new(),
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
    pub fn contain(&mut self) {
        let arena_radius = self.arena.radius;
        for ball in &mut self.balls {
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
        });
        self.balls.len() - 1
    }
}
