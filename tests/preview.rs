//! Preview behaviour (without a window): frames, hot reload and error state.

mod support;

use std::path::PathBuf;

use ball_simulator::preview::{Preview, Reload};
use ball_simulator::render::{layout, Renderer};
use ball_simulator::sim::Simulation;

/// `support::MINIMAL` plus an End Condition: a complete Scenario file.
fn minimal() -> String {
    format!(
        "{}\n[end_condition]\nkind = \"time_cap\"\n",
        support::MINIMAL
    )
}

/// A fresh Scenario file path per test so tests can run in parallel.
fn scenario_file(name: &str, contents: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("preview");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.toml"));
    std::fs::write(&path, contents).unwrap();
    path
}

/// The frame the Video renderer produces after `frames` frames.
fn rendered_frame(toml: &str, seed: u64, frames: u64) -> Vec<u8> {
    let scenario = support::scenario(toml);
    let mut sim = Simulation::new(&scenario, seed);
    for _ in 0..frames {
        sim.step_frame();
    }
    let initial_radius = Simulation::new(&scenario, seed).world().arena.radius;
    Renderer::new(initial_radius).draw(sim.world()).to_vec()
}

#[test]
fn preview_shows_the_same_frames_as_the_renderer() {
    let path = scenario_file("same_frames", &minimal());
    let mut preview = Preview::open(path, 3);
    assert_eq!(preview.error(), None);

    let first = preview
        .frame()
        .expect("a valid Scenario has a frame")
        .to_vec();
    assert_eq!(first.len(), (layout::WIDTH * layout::HEIGHT * 4) as usize);
    assert!(first == rendered_frame(&minimal(), 3, 0));

    for _ in 0..30 {
        preview.advance_frame();
    }
    assert!(preview.frame().unwrap() == rendered_frame(&minimal(), 3, 30));
}

#[test]
fn unchanged_file_does_not_reload() {
    let path = scenario_file("unchanged", &minimal());
    let mut preview = Preview::open(path, 1);
    preview.advance_frame();
    assert_eq!(preview.check_for_changes(), Reload::Unchanged);
    assert!(preview.frame().unwrap() == rendered_frame(&minimal(), 1, 1));
}

#[test]
fn saving_the_scenario_reloads_and_restarts() {
    let path = scenario_file("reload", &minimal());
    let mut preview = Preview::open(path.clone(), 1);
    for _ in 0..20 {
        preview.advance_frame();
    }

    let bigger = minimal().replace("radius = 30.0", "radius = 80.0");
    std::fs::write(&path, &bigger).unwrap();
    assert_eq!(preview.check_for_changes(), Reload::Reloaded);
    assert_eq!(preview.error(), None);
    assert!(preview.frame().unwrap() == rendered_frame(&bigger, 1, 0));
}

#[test]
fn invalid_scenario_shows_error_and_keeps_last_frame() {
    let path = scenario_file("invalid", &minimal());
    let mut preview = Preview::open(path.clone(), 1);
    preview.advance_frame();

    std::fs::write(
        &path,
        minimal().replace("time_cap = 5.0", "time_cap = -1.0"),
    )
    .unwrap();
    assert_eq!(preview.check_for_changes(), Reload::Failed);
    let error = preview
        .error()
        .expect("an invalid Scenario reports an error");
    assert!(error.contains("time_cap"), "{error}");
    // The last good frame stays on screen, frozen.
    preview.advance_frame();
    assert!(preview.frame().unwrap() == rendered_frame(&minimal(), 1, 1));

    // Fixing the file recovers.
    std::fs::write(&path, minimal()).unwrap();
    assert_eq!(preview.check_for_changes(), Reload::Reloaded);
    assert_eq!(preview.error(), None);
    assert!(preview.frame().unwrap() == rendered_frame(&minimal(), 1, 0));
}

#[test]
fn invalid_scenario_at_startup_has_no_frame_but_recovers() {
    let path = scenario_file("invalid_start", "name = ");
    let mut preview = Preview::open(path.clone(), 1);
    assert!(preview.error().is_some());
    assert!(preview.frame().is_none());

    std::fs::write(&path, minimal()).unwrap();
    assert_eq!(preview.check_for_changes(), Reload::Reloaded);
    assert!(preview.frame().is_some());
}

#[test]
fn missing_scenario_file_is_an_error_not_a_crash() {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("preview/does-not-exist.toml");
    let mut preview = Preview::open(path, 1);
    assert!(preview.error().is_some());
    assert!(preview.frame().is_none());
}

#[test]
fn preview_loops_after_the_end_condition_and_a_hold() {
    let short = minimal().replace("time_cap = 5.0", "time_cap = 0.5");
    let path = scenario_file("loops", &short);
    let mut preview = Preview::open(path, 2);
    // 30 frames of action, then the final frame is held before restarting.
    for _ in 0..30 + Preview::HOLD_FRAMES {
        preview.advance_frame();
    }
    assert!(preview.frame().unwrap() == rendered_frame(&short, 2, 30));
    preview.advance_frame();
    assert!(preview.frame().unwrap() == rendered_frame(&short, 2, 0));
}

#[test]
fn downscale_averages_blocks_into_window_pixels() {
    // 2x2 RGBA image -> 1x1 at factor 2.
    let rgba = [
        0, 0, 0, 255, /**/ 200, 100, 40, 255, //
        100, 0, 0, 255, /**/ 100, 100, 0, 255,
    ];
    let out = ball_simulator::preview::downscale_to_rgb32(&rgba, 2, 2, 2);
    assert_eq!(out, vec![(100 << 16) | (50 << 8) | 10]);
}
