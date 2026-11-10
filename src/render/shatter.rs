//! Geometry for a cracked and shattered Boundary (see the `crack` Rule), in
//! pixels. Every jag and shard comes from a small RNG seeded by where the
//! crack or shatter is, so the same simulation state always draws the same.

use std::f32::consts::TAU;

use crate::sim::rng::Rng;
use crate::sim::{Crack, Shatter};

/// Length (px) of a crack made at no speed and at max speed.
const CRACK_MIN_LENGTH: f32 = 14.0;
const CRACK_MAX_LENGTH: f32 = 170.0;
/// Branches off a crack made at max speed; slower ones get fewer.
const CRACK_MAX_BRANCHES: f32 = 4.0;
/// Jags in a crack's trunk.
const CRACK_JAGS: usize = 5;
/// Stroke widths (px) of a crack's trunk and its branches.
pub const TRUNK_WIDTH: f32 = 3.0;
pub const BRANCH_WIDTH: f32 = 2.0;

/// Pieces the Boundary shatters into.
const SHARDS: usize = 16;
/// Angle (radians) left between neighbouring shards.
const SHARD_SPACING: f32 = 0.02;
/// How fast (px/s) the shard at the point that broke flies out; the far side
/// goes at [`SHARD_FAR_SPEED`] of that.
const SHARD_SPEED: f32 = 900.0;
const SHARD_FAR_SPEED: f32 = 0.4;
/// Fastest a shard turns as it flies (radians/s).
const SHARD_MAX_SPIN: f32 = 1.2;

/// A polyline in pixels.
pub type Line = Vec<(f32, f32)>;

/// A crack's lines (trunk first, then its branches) for a Boundary of
/// radius `r` (px) centred on `centre`, running in from the Boundary.
pub fn crack_lines(crack: &Crack, centre: (f32, f32), r: f32) -> Vec<Line> {
    let mut rng = Rng::from_seed(crack.angle.to_bits());
    let size = crack.size as f32;
    let length = CRACK_MIN_LENGTH + (CRACK_MAX_LENGTH - CRACK_MIN_LENGTH) * size;
    let (sin, cos) = (crack.angle as f32).sin_cos();
    // `u` runs in from the Boundary, `v` along it.
    let at = |u: f32, v: f32| {
        (
            centre.0 + cos * (r - u) - sin * v,
            centre.1 + sin * (r - u) + cos * v,
        )
    };
    let mut jitter = |amount: f32| rng.range_f64(-1.0, 1.0) as f32 * amount;

    let mut trunk = vec![(0.0, 0.0)];
    for i in 1..=CRACK_JAGS {
        let u = length * i as f32 / CRACK_JAGS as f32;
        trunk.push((u, jitter(length * 0.08)));
    }
    let branches = (CRACK_MAX_BRANCHES * size).floor() as usize;
    let mut lines = vec![trunk.iter().map(|&(u, v)| at(u, v)).collect::<Line>()];
    for b in 0..branches {
        // Off a jag of the trunk, alternating sides, angled further in.
        let (u0, v0) = trunk[1 + b % (CRACK_JAGS - 1)];
        let side = if b % 2 == 0 { 1.0 } else { -1.0 };
        let turn = side * (0.5 + jitter(0.2).abs());
        let reach = (length - u0) * (0.4 + jitter(0.15).abs());
        let (s, c) = turn.sin_cos();
        let mid = (
            u0 + c * reach * 0.5,
            v0 + s * reach * 0.5 + jitter(reach * 0.1),
        );
        let end = (u0 + c * reach, v0 + s * reach);
        lines.push(vec![at(u0, v0), at(mid.0, mid.1), at(end.0, end.1)]);
    }
    lines
}

/// One piece of a shattered Boundary at a moment in its flight.
pub struct Shard {
    /// The arc of the Boundary it was (radians, start < end).
    pub start: f64,
    pub end: f64,
    /// How far it has flown, in px.
    offset: (f32, f32),
    /// How far it has turned about `pivot` (its middle as it was), radians.
    turn: f32,
    pivot: (f32, f32),
}

impl Shard {
    /// Where a point that was on this shard (px) is now.
    pub fn place(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let (s, c) = self.turn.sin_cos();
        let (dx, dy) = (x - self.pivot.0, y - self.pivot.1);
        (
            self.pivot.0 + dx * c - dy * s + self.offset.0,
            self.pivot.1 + dx * s + dy * c + self.offset.1,
        )
    }

    /// Whether the Boundary at `angle` (radians) is part of this shard.
    pub fn holds(&self, angle: f64) -> bool {
        let turn = std::f64::consts::TAU;
        let a = (angle - self.start).rem_euclid(turn) + self.start;
        a < self.end
    }
}

/// The shards of a Boundary of radius `r` (px) centred on `centre`,
/// `elapsed` seconds after it shattered.
pub fn shards(shatter: &Shatter, centre: (f32, f32), r: f32, elapsed: f32) -> Vec<Shard> {
    let mut rng = Rng::from_seed(shatter.angle.to_bits() ^ 0x5bd1_e995);
    let step = TAU / SHARDS as f32;
    // Cut points round the Boundary, the first just past the point it broke.
    let cuts: Vec<f32> = (0..SHARDS)
        .map(|i| shatter.angle as f32 + step * (i as f32 + 0.5 + rng.range_f64(-0.3, 0.3) as f32))
        .collect();
    (0..SHARDS)
        .map(|i| {
            let start = cuts[i];
            let end = cuts.get(i + 1).copied().unwrap_or(cuts[0] + TAU);
            let middle = (start + end) / 2.0;
            // Shards nearest where it broke fly fastest.
            let near = (1.0 + (middle - shatter.angle as f32).cos()) / 2.0;
            let speed = SHARD_SPEED
                * (SHARD_FAR_SPEED + (1.0 - SHARD_FAR_SPEED) * near)
                * rng.range_f64(0.8, 1.2) as f32;
            let spin = rng.range_f64(-1.0, 1.0) as f32 * SHARD_MAX_SPIN;
            let (sin, cos) = middle.sin_cos();
            Shard {
                start: (start + SHARD_SPACING / 2.0) as f64,
                end: (end - SHARD_SPACING / 2.0) as f64,
                offset: (cos * speed * elapsed, sin * speed * elapsed),
                turn: spin * elapsed,
                pivot: (centre.0 + cos * r, centre.1 + sin * r),
            }
        })
        .collect()
}
