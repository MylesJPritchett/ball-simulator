//! Rendered frames, through the renderer seam: a `Simulation` driven frame by
//! frame and drawn once per frame with its Ending, as the pipeline and preview do.

mod support;

use std::f64::consts::PI;

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

/// MINIMAL with Heat on, splitting in two on its first Boundary Hit, with
/// the given Population Cap. Glows are off: they would tint the fresh split.
fn heat_split(population_cap: usize) -> String {
    format!(
        "{}\n[[rules]]\nkind = \"split\"\ninto = 2\nspread = 10.0\n\n[look]\nheat = true\nglow = false\n",
        MINIMAL.replace(
            "time_cap = 5.0",
            &format!("time_cap = 5.0\npopulation_cap = {population_cap}")
        )
    )
}

/// Two overlapping balls on screen: the frame, both centres and the radius,
/// in pixels.
struct Pair {
    frame: Vec<u8>,
    a: (f32, f32),
    b: (f32, f32),
    radius: f32,
    background: [u8; 3],
}

impl Pair {
    /// Midway between the centres: under both balls.
    fn both(&self) -> [u8; 3] {
        pixel(
            &self.frame,
            (self.a.0 + self.b.0) / 2.0,
            (self.a.1 + self.b.1) / 2.0,
        )
    }

    /// On a's far side from b: under a only.
    fn only_a(&self) -> [u8; 3] {
        let (dx, dy) = (self.a.0 - self.b.0, self.a.1 - self.b.1);
        let len = dx.hypot(dy);
        let d = 0.7 * self.radius / len;
        pixel(&self.frame, self.a.0 + dx * d, self.a.1 + dy * d)
    }
}

/// The first frame after the split on which the two balls overlap, but
/// little enough that each ball's far side is clear of the other.
fn overlapping_pair(toml: &str) -> Pair {
    let arena = layout::arena();
    let mut found = None;
    draw_frames(toml, 1, 300, |sim, frame| {
        let balls = &sim.state().balls;
        if found.is_some() || balls.len() != 2 {
            return;
        }
        let scale = arena.radius / sim.scenario().arena.radius as f32;
        let screen = |b: &sim::Ball| {
            (
                arena.center_x + b.pos.x as f32 * scale,
                arena.center_y + b.pos.y as f32 * scale,
            )
        };
        let (a, b) = (screen(&balls[0]), screen(&balls[1]));
        let radius = balls[0].radius as f32 * scale;
        let apart = (a.0 - b.0).hypot(a.1 - b.1);
        if apart > 0.4 * radius && apart < 0.8 * radius {
            found = Some(Pair {
                frame: frame.to_vec(),
                a,
                b,
                radius,
                background: sim.palette().background,
            });
        }
    });
    found.expect("the two balls overlap on some frame after the split")
}

fn brightness([r, g, b]: [u8; 3]) -> u32 {
    r as u32 + g as u32 + b as u32
}

#[test]
fn heat_glows_hotter_where_balls_overlap() {
    let pair = overlapping_pair(&heat_split(4000));
    let (both, only_a) = (pair.both(), pair.only_a());
    // Halfway from the centre to the Boundary on the side away from the
    // balls (which have just hit it): under no ball, inside the Arena.
    let arena = layout::arena();
    let away = if pair.a.0 > arena.center_x { -1.0 } else { 1.0 };
    let none = pixel(
        &pair.frame,
        arena.center_x + away * arena.radius * 0.5,
        pair.a.1,
    );

    assert_ne!(only_a, pair.background, "a lone ball is drawn");
    assert!(
        brightness(both) > brightness(only_a),
        "two balls ({both:?}) are warmer than one ({only_a:?})"
    );
    assert_eq!(none, pair.background, "no ball, no Heat");
}

#[test]
fn heat_is_white_hot_at_the_density_expected_at_the_population_cap() {
    // (30 / 460)^2 * 470 ~ 2.0: at the cap two balls cover the average pixel.
    let pair = overlapping_pair(&heat_split(470));

    assert_eq!(pair.both(), [255, 255, 255]);
}

#[test]
fn heat_draws_no_trails_or_per_ball_colour() {
    // MINIMAL's ball changing colour on every hit, with trails on, renders
    // exactly like a plain ball without trails.
    let heat = |extra: &str, look: &str| format!("{MINIMAL}\n{extra}\n[look]\nheat = true\n{look}");
    let plain = heat("", "trails = false\n");
    let colourful = heat("[[rules]]\nkind = \"change_colour\"\n", "trails = true\n");
    let mut frames = Vec::new();
    draw_frames(&plain, 4, 240, |_, frame| frames.push(frame.to_vec()));
    let mut k = 0;
    draw_frames(&colourful, 4, 240, |sim, frame| {
        assert!(frame == frames[k], "substep {}", sim.substep());
        k += 1;
    });
}

/// FNV-1a over a frame's bytes.
fn frame_hash(frame: &[u8]) -> u64 {
    frame.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn scenarios_without_heat_render_as_before() {
    // Frame 120 of MINIMAL (trails and glows on) and of a split, as drawn
    // before Heat existed.
    let split = format!("{MINIMAL}\n[[rules]]\nkind = \"split\"\ninto = 2\nspread = 10.0\n");
    let mut hashes = Vec::new();
    for toml in [MINIMAL.to_string(), split] {
        let mut last = 0;
        draw_frames(&toml, 1, 121, |_, frame| last = frame_hash(frame));
        hashes.push(last);
    }

    assert_eq!(hashes, [16720176563455639489, 14864366792312218522]);
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

#[test]
fn the_boundary_is_drawn_with_the_gap_missing_and_the_gap_turns() {
    // A slow ball that stays clear of the Boundary, and a 60-degree Gap
    // turning 90 degrees a second.
    let toml = MINIMAL.replace("speed = 900.0", "speed = 10.0")
        + "\n[[rules]]\nkind = \"gap\"\nwidth = 60.0\nrotation_speed = 90.0\n\
           \n[look]\ntrails = false\nglow = false\n";
    let arena = layout::arena();
    let on_boundary = |frame: &[u8], angle: f64| {
        let r = arena.radius + BOUNDARY_WIDTH / 2.0;
        let (sin, cos) = (angle as f32).sin_cos();
        pixel(frame, arena.center_x + cos * r, arena.center_y + sin * r)
    };
    let mut start = 0.0;
    draw_frames(&toml, 1, 61, |sim, frame| {
        let palette = sim.palette();
        let gap = sim.state().arena.gaps[0].angle;
        if sim.substep() == 0 {
            start = gap;
        }
        if sim.substep() % SUBSTEP_HZ != 0 {
            return;
        }
        for off in [-25.0f64, 0.0, 25.0] {
            assert_eq!(
                on_boundary(frame, gap + off.to_radians()),
                palette.background,
                "substep {}: drawn {off} degrees into the Gap",
                sim.substep()
            );
        }
        for off in [-40.0f64, 40.0, 90.0, 180.0, 270.0] {
            assert_eq!(
                on_boundary(frame, gap + off.to_radians()),
                palette.boundary,
                "substep {}: missing {off} degrees from the Gap",
                sim.substep()
            );
        }
    });
    // One second in, the Gap has turned a quarter turn from where it began:
    // the Boundary is back where it was open.
    let mut last = Vec::new();
    draw_frames(&toml, 1, 61, |_, frame| last = frame.to_vec());
    let palette = Simulation::new(&scenario(&toml), 1).palette();
    assert_eq!(on_boundary(&last, start), palette.boundary);
    assert_eq!(
        on_boundary(&last, start + 90f64.to_radians()),
        palette.background
    );
}

/// MINIMAL speeding up 10% a hit to `max_speed`, cracking the Boundary and
/// ending on `escaped`, with trails and glows off.
fn cracking(max_speed: f64) -> String {
    MINIMAL.replace(
        "speed = 900.0",
        &format!("speed = 900.0\nmax_speed = {max_speed:?}"),
    ) + "\n[[rules]]\nkind = \"speed_up\"\npercent = 10.0\n\
           \n[[rules]]\nkind = \"crack\"\n\
           \n[end_condition]\nkind = \"escaped\"\n\
           \n[look]\ntrails = false\nglow = false\n"
}

/// Pixels drawn (not background) in the Arena's rim within `width` degrees
/// of `angle`, from just inside the Boundary's anti-aliased edge up to 200 px
/// in, leaving out the ball.
fn drawn_in_rim(sim: &Simulation, frame: &[u8], angle: f64, width: f64) -> usize {
    let arena = layout::arena();
    let scale = arena.radius / 460.0;
    let ball = &sim.state().balls[0];
    let (bx, by) = (
        arena.center_x + ball.pos.x as f32 * scale,
        arena.center_y + ball.pos.y as f32 * scale,
    );
    let ball_r = ball.radius as f32 * scale + 3.0;
    let background = sim.palette().background;
    let mut drawn = 0;
    for step in 0..=(width * 10.0) as i32 {
        let a = angle + (step as f64 / 10.0 - width / 2.0).to_radians();
        let (sin, cos) = (a as f32).sin_cos();
        for d in 3..200 {
            let r = arena.radius - d as f32;
            let (x, y) = (arena.center_x + cos * r, arena.center_y + sin * r);
            if (x - bx).hypot(y - by) > ball_r && pixel(frame, x, y) != background {
                drawn += 1;
            }
        }
    }
    drawn
}

#[test]
fn a_crack_appears_where_the_ball_hits_and_is_bigger_the_faster_it_hit() {
    // The first hit is at 900 units/s: 90% of a 1000 max speed, 10% of 9000.
    let first_hit = sim::run(&scenario(&cracking(9000.0)), 1).boundary_hits[0].clone();
    let hit_frame = first_hit.substep.div_ceil(SUBSTEPS_PER_FRAME);
    // A third of a second on, the ball is well clear of the crack.
    let later = hit_frame + 20;
    let crack_at = |max_speed: f64| {
        let (mut before, mut after, mut opposite) = (usize::MAX, 0, usize::MAX);
        let mut k = 0;
        draw_frames(&cracking(max_speed), 1, later + 1, |sim, frame| {
            if k + 1 == hit_frame {
                before = drawn_in_rim(sim, frame, first_hit.angle, 30.0);
            }
            if k == later {
                after = drawn_in_rim(sim, frame, first_hit.angle, 30.0);
                opposite = drawn_in_rim(sim, frame, first_hit.angle + PI, 30.0);
            }
            k += 1;
        });
        (before, after, opposite)
    };
    let (small_before, small, small_opposite) = crack_at(9000.0);
    let (big_before, big, big_opposite) = crack_at(1000.0);

    assert_eq!(
        (small_before, big_before),
        (0, 0),
        "no crack before the hit"
    );
    assert_eq!(
        (small_opposite, big_opposite),
        (0, 0),
        "nothing across the Arena"
    );
    assert!(small > 0, "a slow hit still cracks the Boundary");
    assert!(big > small * 3, "big {big} vs small {small}");
}

#[test]
fn the_boundary_is_drawn_shattered_once_the_ball_breaks_out() {
    let toml = cracking(1500.0);
    let record = sim::run(&scenario(&toml), 1);
    assert_eq!(record.end.condition, "escaped");
    let shatter_frame = record
        .boundary_hits
        .last()
        .unwrap()
        .substep
        .div_ceil(SUBSTEPS_PER_FRAME);
    let arena = layout::arena();
    // Points on a circle `r` px from the Arena's centre drawn in `colour`.
    let on_circle = |frame: &[u8], r: f32, colour: [u8; 3]| {
        (0..720)
            .filter(|&step| {
                let (sin, cos) = (step as f32 / 2.0).to_radians().sin_cos();
                pixel(frame, arena.center_x + cos * r, arena.center_y + sin * r) == colour
            })
            .count()
    };
    let ring = arena.radius + BOUNDARY_WIDTH / 2.0;
    let (mut k, mut checked) = (0, 0);
    // No flash, which would tint the shards.
    let no_flash = |ending: Ending| Ending {
        flash_substeps: 0,
        ..ending
    };
    draw_frames_with(&toml, 1, record.ending.frames(), no_flash, |sim, frame| {
        let boundary = sim.palette().boundary;
        if k + 1 == shatter_frame {
            assert_eq!(on_circle(frame, ring, boundary), 720, "whole before");
        }
        // From a tenth of a second after the shatter to the cut, the
        // Boundary is in shards flying outward.
        if k >= shatter_frame + 6 {
            let on_ring = on_circle(frame, ring, boundary);
            assert!(on_ring < 720 / 4, "frame {k}: {on_ring} of the ring left");
            let shards: usize = (1..120)
                .step_by(4)
                .map(|d| on_circle(frame, ring + d as f32, boundary))
                .sum();
            assert!(shards > 200, "frame {k}: {shards} shard pixels");
            checked += 1;
        }
        k += 1;
    });
    assert!(checked > 60, "{checked} frames checked");
}
