//! The preview window: shows [`Preview`] frames scaled down, at real-time speed.
//!
//! Deliberately thin; all behaviour worth testing lives in [`Preview`].

use std::path::PathBuf;
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};

use super::{downscale_to_rgb32, Preview, Reload};
use crate::render::layout;
use crate::sim::FPS;

/// Window pixels are this many times smaller than Video pixels (360x640).
const DOWNSCALE: u32 = 3;
const WIDTH: usize = (layout::WIDTH / DOWNSCALE) as usize;
const HEIGHT: usize = (layout::HEIGHT / DOWNSCALE) as usize;
const RELOAD_POLL: Duration = Duration::from_millis(250);
/// Never fast-forward more than this many frames at once after a stall.
const MAX_CATCH_UP_FRAMES: u32 = 4;
const ERROR_BAR_HEIGHT: usize = 12;
const ERROR_RED: u32 = 0x00d0_3030;

/// Opens the preview window and runs until it is closed (or Escape is pressed).
pub fn run(path: PathBuf, seed: u64) -> Result<(), minifb::Error> {
    let title = format!("preview: {} (seed {seed})", path.display());
    let mut preview = Preview::open(path, seed);
    let mut window = Window::new(&title, WIDTH, HEIGHT, WindowOptions::default())?;
    window.set_target_fps(FPS as usize);
    report(&preview, &mut window, &title);

    let frame_time = Duration::from_secs_f64(1.0 / FPS as f64);
    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut last_poll = Instant::now();
    let mut last_tick = Instant::now();
    let mut lag = Duration::ZERO;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if last_poll.elapsed() >= RELOAD_POLL {
            last_poll = Instant::now();
            if preview.check_for_changes() != Reload::Unchanged {
                report(&preview, &mut window, &title);
            }
        }

        let now = Instant::now();
        lag += now - last_tick;
        last_tick = now;
        let mut stepped = 0;
        while lag >= frame_time && stepped < MAX_CATCH_UP_FRAMES {
            preview.advance_frame();
            lag -= frame_time;
            stepped += 1;
        }
        if stepped == MAX_CATCH_UP_FRAMES {
            lag = Duration::ZERO;
        }

        match preview.frame() {
            Some(rgba) => {
                buffer = downscale_to_rgb32(rgba, layout::WIDTH, layout::HEIGHT, DOWNSCALE)
            }
            None => buffer.fill(0),
        }
        if preview.error().is_some() {
            buffer[..WIDTH * ERROR_BAR_HEIGHT].fill(ERROR_RED);
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }
    Ok(())
}

/// Shows the load state in the title bar and on stderr. The error bar drawn
/// over the frame marks that the preview is frozen on an invalid Scenario.
fn report(preview: &Preview, window: &mut Window, title: &str) {
    match preview.error() {
        Some(e) => {
            eprintln!("error: {e}");
            window.set_title(&format!("ERROR {title}: {e}"));
        }
        None => {
            eprintln!("loaded {title}");
            window.set_title(title);
        }
    }
}
