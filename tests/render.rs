//! Rendered frames, through the renderer seam: a `Simulation` driven frame by
//! frame and drawn once per frame, as the pipeline and preview do.

mod support;

use ball_simulator::render::{layout, Renderer};
use ball_simulator::sim::{self, Simulation, SUBSTEP_HZ};
use support::{scenario, MINIMAL};

/// Width of the Boundary stroke as drawn (just outside the Arena's edge).
const BOUNDARY_WIDTH: f32 = 6.0;

/// Draws `frames` frames (frame 0 first), calling `each` on every frame.
fn draw_frames(toml: &str, seed: u64, frames: u64, mut each: impl FnMut(&Simulation, &[u8])) {
    let scenario = scenario(toml);
    let mut sim = Simulation::new(&scenario, seed);
    let mut renderer = Renderer::new(sim.scenario());
    for k in 0..frames {
        if k > 0 {
            sim.step_frame();
        }
        let frame = renderer.draw(&sim);
        each(&sim, frame);
    }
}

fn pixel(frame: &[u8], x: f32, y: f32) -> [u8; 3] {
    let i = (y.round() as usize * layout::WIDTH as usize + x.round() as usize) * 4;
    [frame[i], frame[i + 1], frame[i + 2]]
}

#[test]
fn trails_and_glows_never_spill_outside_the_boundary() {
    // A big, fast ball: its trail and its hit glows reach the Boundary often.
    let toml = MINIMAL
        .replace("time_cap = 5.0", "time_cap = 3.0")
        .replace("radius = 30.0", "radius = 200.0")
        .replace("speed = 900.0", "speed = 3000.0");
    let arena = layout::arena();
    let mut checked = 0;
    draw_frames(&toml, 1, 180, |sim, frame| {
        let background = sim.palette().background;
        let outside = arena.radius + BOUNDARY_WIDTH + 2.0;
        for step in 0..360 {
            let (sin, cos) = (step as f32).to_radians().sin_cos();
            for d in (0..40).step_by(4) {
                let r = outside + d as f32;
                let (x, y) = (arena.center_x + cos * r, arena.center_y + sin * r);
                assert_eq!(
                    pixel(frame, x, y),
                    background,
                    "substep {}: pixel at {x:.0},{y:.0} is outside the Boundary",
                    sim.substep()
                );
                checked += 1;
            }
        }
    });
    assert!(checked > 0);
}

#[test]
fn hit_glows_fade_out_during_the_ending_hold() {
    // End the run 16 substeps after the first Boundary Hit, so a fresh glow
    // is showing when the action freezes.
    let first_hit = sim::run(&scenario(MINIMAL), 2).boundary_hits[0].substep;
    let cap = (first_hit + 16) as f64 / SUBSTEP_HZ as f64;
    let toml = |glow: bool| {
        format!(
            "{}\n[look]\ntrails = false\nglow = {glow}\n",
            MINIMAL.replace("time_cap = 5.0", &format!("time_cap = {cap:?}"))
        )
    };
    let end_frame = (first_hit + 16).div_ceil(8);
    // Glows last 0.4 s (24 frames); 40 frames into the hold it must be gone.
    let frames = end_frame + 40;
    let last_frame = |glow: bool| {
        let mut at_end = Vec::new();
        let mut last = Vec::new();
        let mut k = 0;
        draw_frames(&toml(glow), 2, frames + 1, |_, frame| {
            if k == end_frame {
                at_end = frame.to_vec();
            }
            last = frame.to_vec();
            k += 1;
        });
        (at_end, last)
    };
    let (glowing_end, glowing_last) = last_frame(true);
    let (plain_end, plain_last) = last_frame(false);

    assert_ne!(
        glowing_end, plain_end,
        "a glow shows when the action freezes"
    );
    assert!(glowing_last == plain_last, "the glow faded during the hold");
}
