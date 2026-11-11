//! Render: anti-aliased CPU raster of the simulation state with tiny-skia.
//!
//! The renderer is fed a [`Simulation`] after each frame's substeps, with its
//! [`Ending`] (see [`crate::pipeline`]); it never advances physics itself, so
//! anything that drives a `Simulation` frame by frame (the encoder, a preview
//! window) gets identical frames, the Ending's flash and Finale included.
//!
//! Draw order: background, fading trails, Boundary (with its cracks), Peg,
//! hit glows, settled layers, balls, the Ending's Finale (e.g. a pop), Hook
//! Title, Counter, then the Ending's flash over everything; a Meltdown breaks
//! the finished frame down last. Trails and glows are clipped to the Arena
//! (Boundary included). With Heat on, the Heat field takes the place of
//! trails and balls.

mod heat;
mod meltdown;
mod shatter;
pub mod text;

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap,
    PixmapPaint, Point, RadialGradient, Rect, Shader, SpreadMode, Stroke, Transform,
};

use crate::layout;
use crate::look::palette::{Palette, Rgb};
use crate::look::{Counter, LookSpec};
use crate::scenario::Scenario;
use crate::sim::{Ball, Ending, Finale, Simulation, Surface, SUBSTEPS_PER_FRAME, SUBSTEP_HZ};
use heat::Heat;
use meltdown::Meltdown;
use text::{FittedText, TextPainter};

const BOUNDARY_WIDTH: f32 = 6.0;
/// How far the Boundary's colour is mixed towards the background when there
/// is a Peg, so it reads as a faint target outline.
const TARGET_OUTLINE_FADE: f32 = 0.75;
/// Angle (radians) between the points of a Boundary arc drawn around a Gap:
/// half a degree strays well under a pixel from the true circle.
const BOUNDARY_ARC_STEP: f64 = std::f64::consts::PI / 360.0;
/// Ending flash colour and its opacity at full strength.
const FLASH: [u8; 3] = [255, 255, 255];
const FLASH_MAX_ALPHA: f32 = 0.75;

/// How far (px) a pop's ring expands beyond the vanished ball's edge.
const POP_GROWTH: f32 = 160.0;
/// The pop ring's stroke width (px) as it starts; it thins as it fades.
const POP_WIDTH: f32 = 12.0;

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
/// A glow this close (in pixels) to a newer one is merged into it, so hits
/// landing on the same spot don't stack into a solid blob.
const GLOW_MERGE_PX: f32 = GLOW_RADIUS;
/// Hits looked at per frame when choosing glows (bounds the merge cost when
/// thousands of balls hit at once).
const MAX_GLOW_CANDIDATES: usize = MAX_GLOWS * 16;

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

/// Adds the Boundary arc from `start` to `end` (radians) at radius `r` (px)
/// round `centre` to `pb` as a polyline fine enough to read as a curve, each
/// point moved by `place`.
fn push_arc(
    pb: &mut PathBuilder,
    centre: (f32, f32),
    r: f32,
    start: f64,
    end: f64,
    place: impl Fn((f32, f32)) -> (f32, f32),
) {
    let steps = ((end - start) / BOUNDARY_ARC_STEP).ceil().max(1.0) as usize;
    push_polyline(
        pb,
        (0..=steps).map(|i| {
            let angle = start + (end - start) * i as f64 / steps as f64;
            let (sin, cos) = (angle as f32).sin_cos();
            place((centre.0 + cos * r, centre.1 + sin * r))
        }),
    );
}

/// Adds a polyline through `points` to `pb` as a new subpath.
fn push_polyline(pb: &mut PathBuilder, points: impl IntoIterator<Item = (f32, f32)>) {
    for (k, (x, y)) in points.into_iter().enumerate() {
        if k == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
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
    /// Ball overlap density, drawn instead of trails and balls when on.
    heat: Option<Heat>,
    /// The frame the Meltdown is holding, if it is under way.
    meltdown: Meltdown,
    glow: bool,
    /// Render time of the next frame, in substeps. It keeps running through
    /// the ending's hold while the simulation is frozen, so glows fade out
    /// and the frame knows where in the Ending it falls.
    clock: u64,
    /// Clip for trails and glows: the Arena inside the Boundary's outer
    /// edge, with the Boundary radius (px) it was built for.
    clip: Option<(f32, Mask)>,
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
            heat,
            counter,
            ..
        } = scenario.look;
        let text = TextPainter::new();
        let title = text.fit(&scenario.hook_title, layout::hook_title(), TITLE_MAX_PX);
        let scale = arena.radius / scenario.arena.radius as f32;
        Renderer {
            pixmap: Pixmap::new(layout::WIDTH, layout::HEIGHT).expect("non-zero frame size"),
            trails: (trails && !heat)
                .then(|| Pixmap::new(layout::WIDTH, layout::HEIGHT).expect("non-zero frame size")),
            last_pos: Vec::new(),
            heat: heat.then(|| Heat::new(scenario)),
            meltdown: Meltdown::new(),
            glow,
            clock: 0,
            clip: None,
            scale,
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

    /// Draws one Video frame of the simulation's current state and returns it
    /// as RGBA bytes (width * height * 4). Call once per frame, in order:
    /// trails depend on the previous frames drawn, and the frame's place in
    /// the `ending` (`None` while it is not yet known, as in the preview
    /// before the End Condition fires) comes from how many frames came before.
    ///
    /// tiny-skia stores premultiplied RGBA; the background is opaque so every
    /// pixel is opaque and premultiplied equals straight alpha.
    pub fn draw(&mut self, sim: &Simulation, ending: Option<&Ending>) -> &[u8] {
        let palette = sim.palette();
        let now = self.clock.max(sim.substep());
        self.clock = now + SUBSTEPS_PER_FRAME;
        self.pixmap.fill(rgb(palette.background));
        self.update_clip(sim);

        if self.heat.is_some() {
            self.draw_heat(sim, palette);
        } else if self.trails.is_some() {
            self.draw_trails(sim, palette);
        }
        self.draw_boundary(sim, palette);
        self.draw_peg(sim, palette);
        if self.glow {
            self.draw_glows(sim, palette, now);
        }
        self.draw_layers(sim, palette);
        if self.heat.is_none() {
            self.draw_balls(sim, palette);
        }
        let play_out = ending.and_then(|e| Some((e.finale, e.play_out_at(now)?)));
        if let Some((Finale::Pop, t)) = play_out {
            self.draw_pops(sim, palette, t);
        }
        self.draw_text(sim, palette);
        if let Some(ending) = ending {
            self.draw_flash(ending.flash_at(now));
            if ending.finale == Finale::Meltdown {
                self.draw_meltdown(sim, ending, now);
            }
        }

        self.frame()
    }

    /// The most recently drawn frame as RGBA bytes (blank before the first draw).
    pub fn frame(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// Rebuilds the trail/glow clip when the Boundary has moved.
    fn update_clip(&mut self, sim: &Simulation) {
        let outer = self.boundary_radius(sim) + BOUNDARY_WIDTH;
        if self.clip.as_ref().is_some_and(|(r, _)| *r == outer) {
            return;
        }
        let mut mask = Mask::new(layout::WIDTH, layout::HEIGHT).expect("non-zero frame size");
        if let Some(circle) =
            PathBuilder::from_circle(self.arena.center_x, self.arena.center_y, outer)
        {
            mask.fill_path(&circle, FillRule::Winding, true, Transform::identity());
        }
        self.clip = Some((outer, mask));
    }

    fn draw_trails(&mut self, sim: &Simulation, palette: &Palette) {
        let balls = &sim.state().balls;
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

        let clip = self.clip.as_ref().map(|(_, mask)| mask);
        self.pixmap.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            clip,
        );
    }

    fn draw_heat(&mut self, sim: &Simulation, palette: &Palette) {
        let heat = self.heat.as_mut().expect("heat is on");
        heat.clear();
        for ball in &sim.state().balls {
            let x = self.arena.center_x + ball.pos.x as f32 * self.scale;
            let y = self.arena.center_y + ball.pos.y as f32 * self.scale;
            heat.add_ball(x, y, ball.radius as f32 * self.scale);
        }
        heat.paint(&mut self.pixmap, palette);
    }

    fn boundary_radius(&self, sim: &Simulation) -> f32 {
        sim.state().arena.radius as f32 * self.scale
    }

    /// Where the Boundary is drawn (px): on the Arena's edge, or round the
    /// outermost settled layer once Rings have settled into it, so it does
    /// not move when one does.
    fn boundary_stroke_radius(&self, sim: &Simulation) -> f32 {
        let state = sim.state();
        let outer = state
            .settles
            .iter()
            .filter(|settle| settle.surface == Surface::Boundary)
            .map(|settle| settle.radius)
            .fold(state.arena.radius, f64::max);
        outer as f32 * self.scale
    }

    /// The Boundary, with any Gaps left out, and its cracks; once it has
    /// shattered, its shards (cracks and all) flying outward.
    fn draw_boundary(&mut self, sim: &Simulation, palette: &Palette) {
        let centre = (self.arena.center_x, self.arena.center_y);
        let inner = self.boundary_stroke_radius(sim);
        let r = inner + BOUNDARY_WIDTH / 2.0;
        let arena = &sim.state().arena;
        let shards = arena.shatter.map(|shatter| {
            let elapsed = sim.substep().saturating_sub(shatter.substep) as f32 / SUBSTEP_HZ as f32;
            shatter::shards(&shatter, centre, r, elapsed)
        });
        let path = match &shards {
            None if arena.gaps.is_empty() => PathBuilder::from_circle(centre.0, centre.1, r),
            None => {
                let mut pb = PathBuilder::new();
                for (start, end) in arena.solid_arcs() {
                    push_arc(&mut pb, centre, r, start, end, |p| p);
                }
                pb.finish()
            }
            Some(shards) => {
                let mut pb = PathBuilder::new();
                for shard in shards {
                    push_arc(&mut pb, centre, r, shard.start, shard.end, |p| {
                        shard.place(p)
                    });
                }
                pb.finish()
            }
        };
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        // With a Peg, the Boundary is the faint outline the Peg grows towards.
        let colour = match arena.peg_radius {
            Some(_) => mix(palette.boundary, palette.background, TARGET_OUTLINE_FADE),
            None => palette.boundary,
        };
        paint.set_color(rgb(colour));
        if let Some(path) = path {
            let stroke = Stroke {
                width: BOUNDARY_WIDTH,
                ..Stroke::default()
            };
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Trunks and branches, each as one path.
        let (mut trunks, mut branches) = (PathBuilder::new(), PathBuilder::new());
        for crack in &arena.cracks {
            let shard = match &shards {
                // A crack in the space between two shards went with neither.
                Some(shards) => match shards.iter().find(|s| s.holds(crack.angle)) {
                    Some(shard) => Some(shard),
                    None => continue,
                },
                None => None,
            };
            let place = |p| shard.map_or(p, |s| s.place(p));
            for (i, line) in shatter::crack_lines(crack, centre, inner)
                .iter()
                .enumerate()
            {
                let pb = if i == 0 { &mut trunks } else { &mut branches };
                push_polyline(pb, line.iter().map(|&point| place(point)));
            }
        }
        for (pb, width) in [
            (trunks, shatter::TRUNK_WIDTH),
            (branches, shatter::BRANCH_WIDTH),
        ] {
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    ..Stroke::default()
                };
                self.pixmap
                    .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }

    /// The Peg, if there is one: a solid disc at the Arena's centre in the
    /// Boundary's colour.
    fn draw_peg(&mut self, sim: &Simulation, palette: &Palette) {
        let Some(peg) = sim.state().arena.peg_radius else {
            return;
        };
        let (x, y) = (self.arena.center_x, self.arena.center_y);
        if let Some(disc) = PathBuilder::from_circle(x, y, peg as f32 * self.scale) {
            let mut paint = Paint {
                anti_alias: true,
                ..Paint::default()
            };
            paint.set_color(rgb(palette.boundary));
            self.pixmap.fill_path(
                &disc,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// A soft glow on the Boundary (or the Peg) where each recent Boundary
    /// Hit landed, in the hitting ball's colour, fading out over
    /// [`GLOW_SUBSTEPS`] of render time (`now`). Hits close to a newer one
    /// share its glow, and glows are clipped to the Arena.
    fn draw_glows(&mut self, sim: &Simulation, palette: &Palette, now: u64) {
        let boundary = self.boundary_radius(sim) + BOUNDARY_WIDTH / 2.0;
        let peg = sim.state().arena.peg_radius.unwrap_or(0.0) as f32 * self.scale;
        let balls = &sim.state().balls;
        let mut glows: Vec<(&crate::sim::BoundaryHit, f32, f32)> = Vec::new();
        for hit in sim
            .boundary_hits()
            .iter()
            .rev()
            .take_while(|h| now.saturating_sub(h.substep) < GLOW_SUBSTEPS)
            .take(MAX_GLOW_CANDIDATES)
        {
            let (sin, cos) = (hit.angle as f32).sin_cos();
            let r = match hit.surface {
                Surface::Boundary => boundary,
                Surface::Peg => peg,
            };
            let x = self.arena.center_x + cos * r;
            let y = self.arena.center_y + sin * r;
            let near_newer = glows
                .iter()
                .any(|&(_, gx, gy)| (gx - x).hypot(gy - y) < GLOW_MERGE_PX);
            if !near_newer {
                glows.push((hit, x, y));
                if glows.len() == MAX_GLOWS {
                    break;
                }
            }
        }
        let clip = self.clip.as_ref().map(|(_, mask)| mask);
        for (hit, x, y) in glows {
            let t = (now - hit.substep) as f32 / GLOW_SUBSTEPS as f32;
            let strength = (1.0 - t) * (1.0 - t);
            // Balls are appended, so a ball's id is its index until one
            // vanishes; a glow whose ball is not found there takes the
            // Boundary's colour. Heat draws no per-ball colour, so its glows
            // take the Boundary's too.
            let colour = balls
                .get(hit.ball_id as usize)
                .filter(|_| self.heat.is_none())
                .filter(|b| b.id == hit.ball_id)
                .map(|b| palette.ball(ball_slot(b)))
                .unwrap_or(palette.boundary);
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
                    clip,
                );
            }
        }
    }

    /// Each settled Ring as a concentric annulus in its colour, where it was
    /// as it settled, so nothing moves when one settles; layers settled onto
    /// the Peg cover it, so it reads as its layers on its starting colour.
    /// Once the Arena is solid, what is left of it (round the Peg, if there
    /// is one) is filled in the last layer's colour.
    fn draw_layers(&mut self, sim: &Simulation, palette: &Palette) {
        let state = sim.state();
        let (x, y) = self.to_screen(0.0, 0.0);
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        for settle in &state.settles {
            paint.set_color(rgb(palette.ball(settle.colour as usize)));
            let mut pb = PathBuilder::new();
            pb.push_circle(x, y, settle.radius as f32 * self.scale);
            let hole = (settle.radius - settle.thickness) as f32 * self.scale;
            if hole > 0.0 {
                pb.push_circle(x, y, hole);
            }
            if let Some(path) = pb.finish() {
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::EvenOdd,
                    Transform::identity(),
                    None,
                );
            }
        }
        if let (true, Some(last)) = (state.arena.solid, state.settles.last()) {
            paint.set_color(rgb(palette.ball(last.colour as usize)));
            let mut pb = PathBuilder::new();
            pb.push_circle(x, y, self.boundary_radius(sim));
            if let Some(peg) = state.arena.peg_radius {
                pb.push_circle(x, y, peg as f32 * self.scale);
            }
            if let Some(rest) = pb.finish() {
                self.pixmap.fill_path(
                    &rest,
                    &paint,
                    FillRule::EvenOdd,
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
        for ball in &sim.state().balls {
            let (x, y) = self.to_screen(ball.pos.x, ball.pos.y);
            paint.shader = Shader::SolidColor(rgb(palette.ball(ball_slot(ball))));
            // A Ring is an annulus: its hole is a second circle, left
            // unfilled by the even-odd rule so the background shows through.
            let mut pb = PathBuilder::new();
            pb.push_circle(x, y, ball.radius as f32 * self.scale);
            if let Some(thickness) = ball.thickness {
                let hole = (ball.radius - thickness) as f32 * self.scale;
                if hole > 0.0 {
                    pb.push_circle(x, y, hole);
                }
            }
            if let Some(path) = pb.finish() {
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::EvenOdd,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    /// A ring where each vanished ball was, in its colour, expanding from its
    /// edge and fading out as the Play-out runs (`t` 0..1).
    fn draw_pops(&mut self, sim: &Simulation, palette: &Palette, t: f32) {
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        let alpha = ((1.0 - t) * 255.0).round() as u8;
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        let stroke = Stroke {
            width: POP_WIDTH * (1.0 - t) + 1.0,
            ..Stroke::default()
        };
        for ball in &sim.state().vanished {
            let (x, y) = self.to_screen(ball.pos.x, ball.pos.y);
            let radius = ball.radius as f32 * self.scale + POP_GROWTH * ease;
            let colour = mix(palette.ball(ball_slot(ball)), [255, 255, 255], 0.35);
            paint.set_color(rgba(colour, alpha));
            if let Some(circle) = PathBuilder::from_circle(x, y, radius) {
                self.pixmap
                    .stroke_path(&circle, &paint, &stroke, Transform::identity(), None);
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

    /// Breaks the finished frame down: stuttering and glitching through the
    /// Play-out, then the broken frame from the freeze to the cut.
    fn draw_meltdown(&mut self, sim: &Simulation, ending: &Ending, now: u64) {
        let frame = self.pixmap.data_mut();
        if let Some(t) = ending.play_out_at(now) {
            let index = ((now - ending.start_substep) / SUBSTEPS_PER_FRAME) as usize;
            self.meltdown.play_out(frame, sim.seed(), index, t);
        } else if now >= ending.freeze_substep {
            self.meltdown.broken(frame, sim.seed());
        }
    }

    /// Washes the frame with the ending's flash (`strength` 0..=1, from
    /// [`Ending::flash_at`]).
    fn draw_flash(&mut self, strength: f32) {
        let alpha = (strength.clamp(0.0, 1.0) * FLASH_MAX_ALPHA * 255.0).round() as u8;
        if alpha > 0 {
            let [r, g, b] = FLASH;
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(r, g, b, alpha));
            let full = Rect::from_xywh(0.0, 0.0, layout::WIDTH as f32, layout::HEIGHT as f32)
                .expect("non-zero frame size");
            self.pixmap
                .fill_rect(full, &paint, Transform::identity(), None);
        }
    }
}
