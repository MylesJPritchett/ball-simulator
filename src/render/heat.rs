//! Heat: every pixel coloured by how many balls overlap it.
//!
//! Each frame adds every ball's footprint (anti-aliased coverage) to a
//! per-pixel density buffer, then maps the buffer through the palette's Heat
//! ramp once. That costs a few adds per covered pixel per ball, far less than
//! filling a path per ball, which is what lets Heat draw huge populations.
//!
//! The ramp is on a log scale: a lone ball is its coolest colour and it is
//! white-hot at the density expected on average at the Population Cap (the
//! cap's total ball area spread over the Arena), so the Arena heats up as the
//! count climbs.

use tiny_skia::Pixmap;

use super::{layout, mix};
use crate::look::palette::{Palette, Rgb, HEAT_STOPS};
use crate::scenario::Scenario;

pub struct Heat {
    /// Balls covering each pixel, row-major over the frame.
    density: Vec<f32>,
    /// Pixel box (x0, y0, x1, y1), exclusive ends, touched since the last
    /// clear: only it needs clearing and painting.
    dirty: Option<(usize, usize, usize, usize)>,
    /// ln of the white-hot density; at or below 0 any full ball is white-hot.
    ln_white_hot: f32,
}

impl Heat {
    /// Sized from the Scenario's starting ball and Arena, so Rules that grow
    /// or shrink either don't move where white-hot lands.
    pub fn new(scenario: &Scenario) -> Self {
        let ratio = scenario.ball.radius / scenario.arena.radius;
        let white_hot = scenario.population_cap as f64 * ratio * ratio;
        Heat {
            density: vec![0.0; (layout::WIDTH * layout::HEIGHT) as usize],
            dirty: None,
            ln_white_hot: white_hot.ln() as f32,
        }
    }

    pub fn clear(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.dirty.take() {
            let width = layout::WIDTH as usize;
            for y in y0..y1 {
                self.density[y * width + x0..y * width + x1].fill(0.0);
            }
        }
    }

    /// Adds a ball centred at (`x`, `y`) with `radius`, all in pixels. Pixel
    /// (i, j) has its centre at (i + 0.5, j + 0.5); edge pixels get partial
    /// coverage, approximated by distance across a one-pixel band.
    pub fn add_ball(&mut self, x: f32, y: f32, radius: f32) {
        let (width, height) = (layout::WIDTH as usize, layout::HEIGHT as usize);
        let outer = radius + 0.5;
        let clamp = |v: f32, max: usize| (v.max(0.0) as usize).min(max);
        let (x0, x1) = (clamp(x - outer, width), clamp(x + outer + 1.0, width));
        let (y0, y1) = (clamp(y - outer, height), clamp(y + outer + 1.0, height));
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        self.dirty = Some(match self.dirty {
            Some((a, b, c, d)) => (a.min(x0), b.min(y0), c.max(x1), d.max(y1)),
            None => (x0, y0, x1, y1),
        });

        let outer2 = outer * outer;
        let inner = (radius - 0.5).max(0.0);
        let inner2 = inner * inner;
        for j in y0..y1 {
            let dy = j as f32 + 0.5 - y;
            let row = &mut self.density[j * width..(j + 1) * width];
            for (i, cell) in row.iter_mut().enumerate().take(x1).skip(x0) {
                let dx = i as f32 + 0.5 - x;
                let d2 = dx * dx + dy * dy;
                if d2 < inner2 {
                    *cell += 1.0;
                } else if d2 < outer2 {
                    *cell += (outer - d2.sqrt()).min(1.0);
                }
            }
        }
    }

    /// Paints the density over `pixmap` (already filled with the background):
    /// uncovered pixels are left alone, partly covered ones blend from the
    /// background to the coolest colour.
    pub fn paint(&self, pixmap: &mut Pixmap, palette: &Palette) {
        let Some((x0, y0, x1, y1)) = self.dirty else {
            return;
        };
        let width = layout::WIDTH as usize;
        let data = pixmap.data_mut();
        for y in y0..y1 {
            for x in x0..x1 {
                let d = self.density[y * width + x];
                if d <= 0.0 {
                    continue;
                }
                let colour = if d < 1.0 {
                    mix(palette.background, palette.heat[0], d)
                } else if self.ln_white_hot <= 0.0 {
                    palette.heat[HEAT_STOPS - 1]
                } else {
                    ramp(&palette.heat, d.ln() / self.ln_white_hot)
                };
                let i = (y * width + x) * 4;
                // Opaque, so premultiplied equals straight RGB.
                data[i..i + 4].copy_from_slice(&[colour[0], colour[1], colour[2], 255]);
            }
        }
    }
}

/// The ramp's colour at `t` (0 = coolest, 1 and above = white-hot).
fn ramp(stops: &[Rgb; HEAT_STOPS], t: f32) -> Rgb {
    let at = t.clamp(0.0, 1.0) * (HEAT_STOPS - 1) as f32;
    let i = (at as usize).min(HEAT_STOPS - 2);
    mix(stops[i], stops[i + 1], at - i as f32)
}
