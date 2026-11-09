//! Render: anti-aliased CPU raster of the simulation state with tiny-skia.
//!
//! The renderer is fed [`World`] snapshots by replaying the simulation frame by
//! frame (see [`crate::sim::Simulation`]); it never advances physics itself.

pub mod layout;

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

use crate::sim::World;

const BACKGROUND: [u8; 3] = [12, 12, 18];
const BOUNDARY: [u8; 3] = [230, 230, 240];
/// Ball colours, indexed by `Ball::colour` (wrapping). A simple default until
/// each Video gets its own palette.
const DEFAULT_PALETTE: [[u8; 3]; 6] = [
    [255, 120, 90],
    [255, 200, 80],
    [120, 220, 130],
    [90, 190, 255],
    [170, 130, 255],
    [255, 120, 200],
];
const BOUNDARY_WIDTH: f32 = 6.0;
/// Ending flash colour and its opacity at full strength.
const FLASH: [u8; 3] = [255, 255, 255];
const FLASH_MAX_ALPHA: f32 = 0.75;

fn rgb([r, g, b]: [u8; 3]) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

pub struct Renderer {
    pixmap: Pixmap,
    arena: layout::ArenaLayout,
    /// Pixels per simulation unit, fixed from the initial Arena radius so a
    /// changing Arena changes size on screen.
    scale: f32,
}

impl Renderer {
    pub fn new(initial_arena_radius: f64) -> Self {
        let arena = layout::arena();
        Renderer {
            pixmap: Pixmap::new(layout::WIDTH, layout::HEIGHT).expect("non-zero frame size"),
            scale: arena.radius / initial_arena_radius as f32,
            arena,
        }
    }

    /// Draws one frame and returns it as RGBA bytes (width * height * 4).
    ///
    /// tiny-skia stores premultiplied RGBA; the background is opaque so every
    /// pixel is opaque and premultiplied equals straight alpha.
    pub fn draw(&mut self, world: &World) -> &[u8] {
        self.pixmap.fill(rgb(BACKGROUND));
        let (cx, cy) = (self.arena.center_x, self.arena.center_y);

        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };

        if let Some(circle) = PathBuilder::from_circle(
            cx,
            cy,
            world.arena.radius as f32 * self.scale + BOUNDARY_WIDTH / 2.0,
        ) {
            paint.set_color(rgb(BOUNDARY));
            let stroke = Stroke {
                width: BOUNDARY_WIDTH,
                ..Stroke::default()
            };
            self.pixmap
                .stroke_path(&circle, &paint, &stroke, Transform::identity(), None);
        }

        for ball in &world.balls {
            let colour = DEFAULT_PALETTE[ball.colour as usize % DEFAULT_PALETTE.len()];
            paint.set_color(rgb(colour));
            let x = cx + ball.pos.x as f32 * self.scale;
            let y = cy + ball.pos.y as f32 * self.scale;
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

        self.pixmap.data()
    }

    /// Washes the frame last drawn with the ending's flash (`strength` 0..=1,
    /// from [`crate::sim::Ending::flash_at`]) and returns it as RGBA bytes.
    pub fn flash(&mut self, strength: f32) -> &[u8] {
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
        self.pixmap.data()
    }
}
