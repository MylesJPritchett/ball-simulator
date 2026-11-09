//! Rendered frames, through the renderer seam: a `Simulation` driven frame by
//! frame and drawn once per frame with its Ending, as the pipeline and preview do.

mod support;

use ball_simulator::render::{layout, Renderer};
use ball_simulator::sim::{self, Effect, Ending, Simulation, SUBSTEPS_PER_FRAME, SUBSTEP_HZ};
use support::{scenario, MINIMAL};

/// Width of the Boundary stroke as drawn (just outside the Arena's edge).
const BOUNDARY_WIDTH: f32 = 6.0;

/// Draws `frames` frames (frame 0 first) with the run's Ending, calling
/// `each` on every frame.
fn draw_frames(toml: &str, seed: u64, frames: u64, each: impl FnMut(&Simulation, &[u8])) {
    draw_frames_with(toml, seed, frames, |ending| ending, each);
}

/// As [`draw_frames`], drawing with the run's Ending as `edit` changes it.
fn draw_frames_with(
    toml: &str,
    seed: u64,
    frames: u64,
    edit: impl Fn(Ending) -> Ending,
    mut each: impl FnMut(&Simulation, &[u8]),
) {
    let scenario = scenario(toml);
    let ending = edit(sim::run(&scenario, seed).ending);
    let mut sim = Simulation::new(&scenario, seed);
    let mut renderer = Renderer::new(sim.scenario());
    for k in 0..frames {
        if k > 0 {
            sim.step_frame();
        }
        let frame = renderer.draw(&sim, Some(&ending));
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

#[test]
fn the_ending_flashes_when_the_end_condition_fires_and_fades_before_the_cut() {
    // The End Condition fires on frame 30.
    let toml = MINIMAL.replace("time_cap = 5.0", "time_cap = 0.5");
    // A corner pixel, away from the Arena and the text.
    let brightness = |frame: &[u8]| pixel(frame, 2.0, 2.0).map(u32::from).iter().sum::<u32>();
    let mut frames = Vec::new();
    let mut dark = 0;
    let ending = sim::run(&scenario(&toml), 1).ending;
    draw_frames(&toml, 1, ending.frames(), |sim, frame| {
        dark = sim.palette().background.map(u32::from).iter().sum();
        frames.push(brightness(frame));
    });

    assert_eq!(frames[29], dark, "no flash during the action");
    assert!(frames[30] > dark, "the flash starts as the action freezes");
    assert!(frames[30] > frames[40], "the flash fades");
    assert_eq!(
        *frames.last().unwrap(),
        dark,
        "the flash has gone before the cut"
    );
}

/// One ball that shrinks to nothing on its third Boundary Hit and pops.
fn vanishing() -> String {
    format!(
        "{MINIMAL}\n[[rules]]\nkind = \"shrink\"\namount = 10.0\n\n\
         [end_condition]\nkind = \"vanished\"\n"
    )
}

#[test]
fn a_vanished_ball_pops_during_the_play_out_and_the_pop_is_gone_by_the_freeze() {
    let toml = format!("{}\n[look]\ntrails = false\nglow = false\n", vanishing());
    let record = support::run(&toml, 3);
    let ending = record.ending;
    let frames = ending.frames();
    // The same frames with and without the pop drawn.
    let draw = |effect: Effect| {
        let mut out = Vec::new();
        draw_frames_with(
            &toml,
            3,
            frames,
            |e| Ending { effect, ..e },
            |_, f| out.push(f.to_vec()),
        );
        out
    };
    let popped = draw(Effect::Pop);
    let plain = draw(Effect::None);

    // Where the ball was when it vanished, on screen.
    let arena = layout::arena();
    let scale = arena.radius / 460.0;
    let mut gone = Simulation::new(&scenario(&toml), 3);
    while gone.end().is_none() {
        gone.step();
    }
    let ball = &gone.state().vanished[0];
    let (bx, by) = (
        arena.center_x + ball.pos.x as f32 * scale,
        arena.center_y + ball.pos.y as f32 * scale,
    );

    let frame_of = |substep: u64| substep.div_ceil(SUBSTEPS_PER_FRAME) as usize;
    let (first, freeze) = (
        frame_of(ending.start_substep),
        frame_of(ending.freeze_substep),
    );
    for k in first..freeze {
        let differing: Vec<(usize, usize)> = (0..layout::HEIGHT as usize)
            .flat_map(|y| (0..layout::WIDTH as usize).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let i = (y * layout::WIDTH as usize + x) * 4;
                popped[k][i..i + 3] != plain[k][i..i + 3]
            })
            .collect();
        assert!(!differing.is_empty(), "frame {k}: the pop shows");
        for (x, y) in differing {
            assert!(
                (x as f32 - bx).hypot(y as f32 - by) < 250.0,
                "frame {k}: the pop is where the ball was, not at {x},{y}"
            );
        }
    }
    for k in freeze..frames as usize {
        assert!(popped[k] == plain[k], "frame {k}: the pop has gone");
    }
}

#[test]
fn the_same_seed_gives_the_same_record_frames_and_samples_with_a_play_out() {
    let toml = vanishing();
    let video = || {
        let record = support::run(&toml, 3);
        let mut frames = Vec::new();
        draw_frames(&toml, 3, record.ending.frames(), |_, f| {
            frames.push(f.to_vec())
        });
        let samples = ball_simulator::audio::synthesize(&record);
        (record, frames, samples)
    };
    let (first, second) = (video(), video());

    assert!(first.0.ending.freeze_substep > first.0.ending.start_substep);
    assert_eq!(first.0, second.0);
    assert!(first.1 == second.1, "same frames");
    assert!(first.2 == second.2, "same samples");
}
