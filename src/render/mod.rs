//! Render: anti-aliased CPU raster of the simulation state with tiny-skia.
//!
//! The renderer is fed a [`Simulation`] after each frame's substeps (see
//! [`crate::pipeline`]); it never advances physics itself, so anything that
//! drives a `Simulation` frame by frame (the encoder, a preview window) gets
//! identical frames.
//!
//! Draw order: background, fading trails, Boundary, hit glows, balls, Hook
//! Title, Counter.

pub mod layout;
pub mod text;

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, Paint, PathBuilder, Pixmap, PixmapPaint, Point,
    RadialGradient, Shader, SpreadMode, Stroke, Transform,
};

use crate::look::palette::{Palette, Rgb};
use crate::look::{Counter, LookSpec};
use crate::scenario::Scenario;
use crate::sim::{Ball, Simulation, SUBSTEP_HZ};
use text::{FittedText, TextPainter};

const BOUNDARY_WIDTH: f32 = 6.0;

/// Fraction of trail brightness kept from one frame to the next (of 256):
/// 215/256 per frame fades to ~5% in about 17 frames (~0.3 s).
const TRAIL_KEEP: u32 = 215;
/// Opacity of the newest trail segment.
const TRAIL_ALPHA: u8 = 110;

/// How long a Boundary Hit glows, in substeps (0.4 s).
const GLOW_SUBSTEPS: u64 = SUBSTEP_HZ * 2 / 5;
/// Glow radius in pixels when the hit lands; it widens a little as it fades.
const GLOW_RADIUS: f32 = 100.0;
/// Most recent glows drawn per frame, to bound cost with many balls.
const MAX_GLOWS: usize = 96;

const TITLE_MAX_PX: f32 = 76.0;
const COUNTER_NUMBER_PX: f32 = 132.0;
const COUNTER_LABEL_PX: f32 = 34.0;
/// Opacity of the Counter caption relative to the text colour.
const COUNTER_LABEL_ALPHA: f32 = 0.6;

fn rgb([r, g, b]: Rgb) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

fn rgba([r, g, b]: Rgb, a: u8) -> Color {
    Color::from_rgba8(r, g, b, a)
}

/// Mixes `colour` towards `background` (t = 0 keeps the colour).
fn mix(colour: Rgb, background: Rgb, t: f32) -> Rgb {
    let m = |c: u8, b: u8| (c as f32 + (b as f32 - c as f32) * t).round() as u8;
    [
        m(colour[0], background[0]),
        m(colour[1], background[1]),
        m(colour[2], background[2]),
    ]
}

/// The palette colour slot a ball is drawn in (wrapped by [`Palette::ball`]).
fn ball_slot(ball: &Ball) -> usize {
    ball.colour as usize
}

pub struct Renderer {
    pixmap: Pixmap,
    /// Persistent, transparent layer that balls paint into and that fades a
    /// little every frame. `None` when the Scenario switches trails off.
    trails: Option<Pixmap>,
    /// Each ball's position on screen last frame, indexed by ball id.
    last_pos: Vec<Option<(f32, f32)>>,
    glow: bool,
    arena: layout::ArenaLayout,
    /// Pixels per simulation unit, fixed from the initial Arena radius so a
    /// changing Arena changes size on screen.
    scale: f32,
    text: TextPainter,
    title: FittedText,
    counter: Counter,
}

impl Renderer {
    pub fn new(scenario: &Scenario) -> Self {
        let arena = layout::arena();
        let LookSpec {
            trails,
            glow,
            counter,
            ..
        } = scenario.look;
        let text = TextPainter::new();
        let title = text.fit(&scenario.hook_title, layout::hook_title(), TITLE_MAX_PX);
        Renderer {
            pixmap: Pixmap::new(layout::WIDTH, layout::HEIGHT).expect("non-zero frame size"),
            trails: trails
                .then(|| Pixmap::new(layout::WIDTH, layout::HEIGHT).expect("non-zero frame size")),
            last_pos: Vec::new(),
            glow,
            scale: arena.radius / scenario.arena.radius as f32,
            arena,
            text,
            title,
            counter,
        }
    }

    fn to_screen(&self, x: f64, y: f64) -> (f32, f32) {
        (
            self.arena.center_x + x as f32 * self.scale,
            self.arena.center_y + y as f32 * self.scale,
        )
    }

    /// Draws one frame of the simulation's current state and returns it as
    /// RGBA bytes (width * height * 4). Call once per frame, in order: trails
    /// depend on the previous frames drawn.
    ///
    /// tiny-skia stores premultiplied RGBA; the background is opaque so every
    /// pixel is opaque and premultiplied equals straight alpha.
    pub fn draw(&mut self, sim: &Simulation) -> &[u8] {
        let palette = sim.palette();
        self.pixmap.fill(rgb(palette.background));

        if self.trails.is_some() {
            self.draw_trails(sim, palette);
        }
        self.draw_boundary(sim, palette);
        if self.glow {
            self.draw_glows(sim, palette);
        }
        self.draw_balls(sim, palette);
        self.draw_text(sim, palette);

        self.frame()
    }

    /// The most recently drawn frame as RGBA bytes (blank before the first draw).
    pub fn frame(&self) -> &[u8] {
        self.pixmap.data()
    }

    fn draw_trails(&mut self, sim: &Simulation, palette: &Palette) {
        let balls = &sim.world().balls;
        let segments: Vec<_> = balls
            .iter()
            .map(|ball| {
                let now = self.to_screen(ball.pos.x, ball.pos.y);
                let last = self.last_pos.get(ball.id as usize).copied().flatten();
                (ball, now, last)
            })
            .collect();
        for (ball, now, _) in &segments {
            let id = ball.id as usize;
            if self.last_pos.len() <= id {
                self.last_pos.resize(id + 1, None);
            }
            self.last_pos[id] = Some(*now);
        }

        let layer = self.trails.as_mut().expect("trails are on");
        // Fade what is already there. Scaling all four premultiplied channels
        // keeps pixels valid and flooring lets them reach exactly zero.
        for byte in layer.data_mut() {
            *byte = ((*byte as u32 * TRAIL_KEEP) >> 8) as u8;
        }

        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        for (ball, (x, y), last) in segments {
            paint.set_color(rgba(palette.ball(ball_slot(ball)), TRAIL_ALPHA));
            let width = 2.0 * ball.radius as f32 * self.scale;
            // A capsule from last frame's position to this one keeps the trail
            // continuous however fast the ball moves.
            let (lx, ly) = last.unwrap_or((x, y));
            let mut pb = PathBuilder::new();
            pb.move_to(lx, ly);
            pb.line_to(x + 0.01, y);
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width,
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                };
                layer.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }

        self.pixmap.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }

    fn boundary_radius(&self, sim: &Simulation) -> f32 {
        sim.world().arena.radius as f32 * self.scale
    }

    fn draw_boundary(&mut self, sim: &Simulation, palette: &Palette) {
        let (cx, cy) = (self.arena.center_x, self.arena.center_y);
        let r = self.boundary_radius(sim) + BOUNDARY_WIDTH / 2.0;
        if let Some(circle) = PathBuilder::from_circle(cx, cy, r) {
            let mut paint = Paint {
                anti_alias: true,
                ..Paint::default()
            };
            paint.set_color(rgb(palette.boundary));
            let stroke = Stroke {
                width: BOUNDARY_WIDTH,
                ..Stroke::default()
            };
            self.pixmap
                .stroke_path(&circle, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// A soft glow on the Boundary where each recent Boundary Hit landed, in
    /// the hitting ball's colour, fading out over [`GLOW_SUBSTEPS`].
    fn draw_glows(&mut self, sim: &Simulation, palette: &Palette) {
        let now = sim.substep();
        let boundary = self.boundary_radius(sim) + BOUNDARY_WIDTH / 2.0;
        let balls = &sim.world().balls;
        let recent = sim
            .boundary_hits()
            .iter()
            .rev()
            .take_while(|h| now.saturating_sub(h.substep) < GLOW_SUBSTEPS)
            .take(MAX_GLOWS);
        for hit in recent {
            let t = (now - hit.substep) as f32 / GLOW_SUBSTEPS as f32;
            let strength = (1.0 - t) * (1.0 - t);
            // Balls are only ever appended, so a ball's id is its index.
            let colour = balls
                .get(hit.ball_id as usize)
                .filter(|b| b.id == hit.ball_id)
                .map(|b| palette.ball(ball_slot(b)))
                .unwrap_or(palette.boundary);
            let (sin, cos) = (hit.angle as f32).sin_cos();
            let x = self.arena.center_x + cos * boundary;
            let y = self.arena.center_y + sin * boundary;
            let radius = GLOW_RADIUS * (1.0 + 0.6 * t);
            let alpha = (strength * 255.0) as u8;
            let core = mix(colour, [255, 255, 255], 0.35);
            let stops = vec![
                GradientStop::new(0.0, rgba(core, alpha)),
                GradientStop::new(0.3, rgba(colour, (alpha as f32 * 0.7) as u8)),
                GradientStop::new(1.0, rgba(colour, 0)),
            ];
            let Some(shader) = RadialGradient::new(
                Point::from_xy(x, y),
                Point::from_xy(x, y),
                radius,
                stops,
                SpreadMode::Pad,
                Transform::identity(),
            ) else {
                continue;
            };
            if let Some(circle) = PathBuilder::from_circle(x, y, radius) {
                let paint = Paint {
                    shader,
                    anti_alias: true,
                    ..Paint::default()
                };
                self.pixmap.fill_path(
                    &circle,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    fn draw_balls(&mut self, sim: &Simulation, palette: &Palette) {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        for ball in &sim.world().balls {
            let (x, y) = self.to_screen(ball.pos.x, ball.pos.y);
            paint.shader = Shader::SolidColor(rgb(palette.ball(ball_slot(ball))));
            if let Some(circle) = PathBuilder::from_circle(x, y, ball.radius as f32 * self.scale) {
                self.pixmap.fill_path(
                    &circle,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    fn draw_text(&mut self, sim: &Simulation, palette: &Palette) {
        self.text.draw_centred(
            &mut self.pixmap,
            &self.title,
            layout::hook_title(),
            palette.text,
        );

        // Counter: a large number with a small caption under it.
        let region = layout::counter();
        let value = self.counter.read(sim);
        let number = value.to_string();
        let label = self.counter.label(value);
        let gap = COUNTER_LABEL_PX * 0.6;
        let block =
            COUNTER_NUMBER_PX * text::CAP_HEIGHT + gap + COUNTER_LABEL_PX * text::CAP_HEIGHT;
        let number_baseline =
            region.center_y() - block / 2.0 + COUNTER_NUMBER_PX * text::CAP_HEIGHT;
        let label_baseline = number_baseline + gap + COUNTER_LABEL_PX * text::CAP_HEIGHT;

        let number_x = region.center_x() - self.text.line_width(&number, COUNTER_NUMBER_PX) / 2.0;
        self.text.draw_line(
            &mut self.pixmap,
            &number,
            number_x,
            number_baseline,
            COUNTER_NUMBER_PX,
            palette.text,
        );
        let label_x = region.center_x() - self.text.line_width(label, COUNTER_LABEL_PX) / 2.0;
        self.text.draw_line(
            &mut self.pixmap,
            label,
            label_x,
            label_baseline,
            COUNTER_LABEL_PX,
            mix(palette.text, palette.background, 1.0 - COUNTER_LABEL_ALPHA),
        );
    }
}
