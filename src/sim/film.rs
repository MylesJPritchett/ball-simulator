//! The Film: a layer over the inside of the Arena, held as a fixed grid of
//! cells over the Arena's bounding square. Only cells whose centre is inside
//! the Arena count; each either has Film or has been wiped. The renderer draws
//! the Film from this same grid, so what is drawn and what is counted agree.

use super::Vec2;

/// Cells across the Arena's diameter: the Arena is drawn about 840 px across,
/// so a cell is under two pixels and the Film's edge looks smooth once the
/// renderer blends between cells.
const CELLS: usize = 512;

/// How far, in cells, the edge advances for each ring grown. Rings alternate
/// between side and corner neighbours, so after `2m` rings Film has reached
/// the octagon |x|, |y| <= 2m, |x| + |y| <= 3m (in cells), of area 14 m^2:
/// the same area as a disc of radius 2m x sqrt(14 / 4 pi). Counting each ring
/// as that much further keeps the edge at the right speed on average: it is
/// about 5% slower along the grid's axes and within 1% along its diagonals.
const RING: f64 = 1.055_531;

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
    /// Cleared cells that may border Film, in the order they were found:
    /// regrowth only looks at these, so its cost follows the Film's edge
    /// rather than the whole grid. Cells that turn out to border no Film are
    /// dropped and come back when a neighbour grows.
    frontier: Vec<usize>,
    /// Whether each cell is in `frontier`.
    queued: Vec<bool>,
    /// How far the edge is owed to advance, in simulation units, short of a
    /// whole cell.
    owed: f64,
    /// Rings grown so far; rings alternate between growing into the four
    /// side neighbours and all eight, so the edge advances at nearly the same
    /// speed in every direction rather than as a diamond or a square.
    rings: u64,
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
            frontier: Vec::new(),
            queued: vec![false; CELLS * CELLS],
            owed: 0.0,
            rings: 0,
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
                    self.queue(index);
                }
            }
        }
    }

    /// Grows the Film back by `distance`: its edge advances that far into
    /// the cleared Arena, a ring of cells at a time, spreading
    /// only from the Film that is left. Once no Film is left nothing grows.
    pub fn regrow(&mut self, distance: f64) {
        self.owed += distance;
        let ring = RING * self.cell;
        while self.owed >= ring {
            self.owed -= ring;
            self.grow_ring();
        }
    }

    /// Grows Film into every cleared cell that borders Film: by a side on
    /// even rings, by a side or a corner on odd ones.
    fn grow_ring(&mut self) {
        let corners = self.rings % 2 == 1;
        self.rings += 1;
        let mut grows = Vec::new();
        let mut kept = Vec::new();
        for index in std::mem::take(&mut self.frontier) {
            if self.borders_film(index, corners) {
                grows.push(index);
            } else if self.borders_film(index, true) {
                kept.push(index);
            } else {
                self.queued[index] = false;
            }
        }
        self.frontier = kept;
        for &index in &grows {
            self.queued[index] = false;
            self.cells[index] = true;
            self.left += 1;
        }
        for &index in &grows {
            for next in self.neighbours(index, true) {
                if !self.cells[next] && self.is_inside(next) {
                    self.queue(next);
                }
            }
        }
    }

    /// Adds a cleared cell to the frontier, once.
    fn queue(&mut self, index: usize) {
        if !self.queued[index] {
            self.queued[index] = true;
            self.frontier.push(index);
        }
    }

    /// Whether a side neighbour of the cell (or, with `corners`, any of its
    /// eight neighbours) has Film.
    fn borders_film(&self, index: usize, corners: bool) -> bool {
        self.neighbours(index, corners).any(|next| self.cells[next])
    }

    /// The cells around `index` on the grid: its four side neighbours, and
    /// with `corners` its four corner neighbours too.
    fn neighbours(&self, index: usize, corners: bool) -> impl Iterator<Item = usize> {
        const SIDES: [(isize, isize); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];
        const ALL: [(isize, isize); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        let offsets: &'static [(isize, isize)] = if corners { &ALL } else { &SIDES };
        let (col, row) = ((index % CELLS) as isize, (index / CELLS) as isize);
        offsets.iter().filter_map(move |&(dc, dr)| {
            let (c, r) = (col + dc, row + dr);
            let on_grid = (0..CELLS as isize).contains(&c) && (0..CELLS as isize).contains(&r);
            on_grid.then(|| r as usize * CELLS + c as usize)
        })
    }

    /// Whether the cell's centre is inside the Arena.
    fn is_inside(&self, index: usize) -> bool {
        self.centre(index % CELLS, index / CELLS).length() <= self.radius
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
