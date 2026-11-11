//! The Film: a layer over the inside of the Arena, held as a fixed grid of
//! cells over the Arena's bounding square. Only cells whose centre is inside
//! the Arena count; each either has Film or has been wiped. The renderer draws
//! the Film from this same grid, so what is drawn and what is counted agree.

use super::Vec2;

/// Cells across the Arena's diameter: the Arena is drawn about 840 px across,
/// so a cell is under two pixels and the Film's edge looks smooth once the
/// renderer blends between cells.
const CELLS: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub struct Film {
    /// Radius of the Arena the Film was laid over.
    radius: f64,
    /// Side of a cell, in simulation units.
    cell: f64,
    /// Whether each cell has Film, row-major from the top-left corner of the
    /// bounding square. Cells outside the Arena never do.
    cells: Vec<bool>,
    /// Cells inside the Arena.
    total: usize,
    /// Cells that still have Film.
    left: usize,
}

impl Film {
    /// A Film covering the whole of an Arena of `radius`.
    pub fn new(radius: f64) -> Self {
        let cell = 2.0 * radius / CELLS as f64;
        let mut film = Film {
            radius,
            cell,
            cells: vec![false; CELLS * CELLS],
            total: 0,
            left: 0,
        };
        for row in 0..CELLS {
            for col in 0..CELLS {
                if film.centre(col, row).length() <= radius {
                    film.cells[row * CELLS + col] = true;
                    film.total += 1;
                }
            }
        }
        film.left = film.total;
        film
    }

    /// The fraction of the Arena that still has Film, from 1 (all of it) to
    /// 0 (cleared).
    pub fn left(&self) -> f64 {
        self.left as f64 / self.total as f64
    }

    /// Whether no Film is left.
    pub fn is_cleared(&self) -> bool {
        self.left == 0
    }

    /// Wipes the Film a disc of `radius` swept moving from `from` to `to`:
    /// every cell whose centre is in the capsule between them. Only the
    /// cells in the capsule's bounding box are looked at.
    pub fn wipe(&mut self, from: Vec2, to: Vec2, radius: f64) {
        let lo = |v: f64| ((v - radius + self.radius) / self.cell).floor().max(0.0) as usize;
        let hi =
            |v: f64| (((v + radius + self.radius) / self.cell).ceil().max(0.0) as usize).min(CELLS);
        let (col0, col1) = (lo(from.x.min(to.x)), hi(from.x.max(to.x)));
        let (row0, row1) = (lo(from.y.min(to.y)), hi(from.y.max(to.y)));
        let path = to - from;
        let length2 = path.dot(path);
        let radius2 = radius * radius;
        for row in row0..row1 {
            for col in col0..col1 {
                let index = row * CELLS + col;
                if !self.cells[index] {
                    continue;
                }
                // The closest point to the cell's centre on the ball's path.
                let offset = self.centre(col, row) - from;
                let t = if length2 > 0.0 {
                    (offset.dot(path) / length2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let gap = offset - path * t;
                if gap.dot(gap) <= radius2 {
                    self.cells[index] = false;
                    self.left -= 1;
                }
            }
        }
    }

    /// How much Film there is at `p`, from 0 to 1, blended between the
    /// nearest cells so the Film's edge is smooth rather than a staircase.
    pub fn coverage_at(&self, p: Vec2) -> f64 {
        let gx = (p.x + self.radius) / self.cell - 0.5;
        let gy = (p.y + self.radius) / self.cell - 0.5;
        let (col, row) = (gx.floor(), gy.floor());
        let (fx, fy) = (gx - col, gy - row);
        let has = |c: f64, r: f64| {
            let inside = (0.0..CELLS as f64).contains(&c) && (0.0..CELLS as f64).contains(&r);
            if inside && self.cells[r as usize * CELLS + c as usize] {
                1.0
            } else {
                0.0
            }
        };
        let top = has(col, row) * (1.0 - fx) + has(col + 1.0, row) * fx;
        let bottom = has(col, row + 1.0) * (1.0 - fx) + has(col + 1.0, row + 1.0) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// The centre of the cell at `col`, `row`.
    fn centre(&self, col: usize, row: usize) -> Vec2 {
        Vec2::new(
            -self.radius + (col as f64 + 0.5) * self.cell,
            -self.radius + (row as f64 + 0.5) * self.cell,
        )
    }
}
