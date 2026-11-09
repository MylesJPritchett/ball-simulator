//! Preview: a live, hot-reloading view of a Scenario for tuning.
//!
//! [`Preview`] owns everything except the window: it loads the Scenario file,
//! replays the simulation frame by frame through the same [`Renderer`] the
//! encoder uses (so what you tune is what you get), reloads when the file
//! changes on disk, and holds an error instead of crashing when the file is
//! invalid. [`window`] is the thin display loop on top.

pub mod window;

use std::path::PathBuf;

use crate::render::Renderer;
use crate::scenario::Scenario;
use crate::sim::{Ending, Simulation, SUBSTEPS_PER_FRAME};

/// Outcome of [`Preview::check_for_changes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reload {
    /// The Scenario file is unchanged since the last check.
    Unchanged,
    /// The file changed and the new Scenario is running from its first frame.
    Reloaded,
    /// The file changed but could not be loaded; see [`Preview::error`].
    Failed,
}

struct Running {
    scenario: Scenario,
    sim: Simulation,
    renderer: Renderer,
    /// Frames shown since the End Condition fired.
    held: u64,
}

impl Running {
    /// A fresh run with its first frame drawn. The renderer is stateful (trails
    /// build up over frames), so it draws every frame, exactly like the Video
    /// pipeline, and restarts with the run.
    fn start(scenario: Scenario, seed: u64) -> Self {
        let sim = Simulation::new(&scenario, seed);
        // Built from the Seed's resolved Scenario, not the template.
        let mut renderer = Renderer::new(sim.scenario());
        renderer.draw(&sim);
        Running {
            scenario,
            sim,
            renderer,
            held: 0,
        }
    }
}

pub struct Preview {
    path: PathBuf,
    seed: u64,
    /// File contents at the last check; `None` if the file could not be read.
    last_contents: Option<Vec<u8>>,
    running: Option<Running>,
    error: Option<String>,
}

impl Preview {
    /// Frames the final frame is held after the End Condition fires, before
    /// the preview restarts from the first frame.
    pub const HOLD_FRAMES: u64 = 90;

    /// Opens a preview of the Scenario file. An unreadable or invalid file is
    /// reported through [`Preview::error`], never a panic.
    pub fn open(path: PathBuf, seed: u64) -> Self {
        let mut preview = Preview {
            path,
            seed,
            last_contents: None,
            running: None,
            error: None,
        };
        let contents = std::fs::read(&preview.path).ok();
        preview.load(contents);
        preview
    }

    /// Reloads the Scenario if the file's contents changed since the last check.
    pub fn check_for_changes(&mut self) -> Reload {
        let contents = std::fs::read(&self.path).ok();
        if contents == self.last_contents {
            return Reload::Unchanged;
        }
        if self.load(contents) {
            Reload::Reloaded
        } else {
            Reload::Failed
        }
    }

    /// Why the current Scenario file cannot be previewed, if it cannot.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Advances one frame. After the End Condition fires the last frame is held
    /// for [`Preview::HOLD_FRAMES`], then the run restarts. Frozen while the
    /// Scenario file is invalid.
    pub fn advance_frame(&mut self) {
        if self.error.is_some() {
            return;
        }
        let Some(running) = &mut self.running else {
            return;
        };
        if running.sim.is_ended() {
            running.held += 1;
            if running.held > Self::HOLD_FRAMES {
                *running = Running::start(running.scenario.clone(), self.seed);
                return;
            }
        } else {
            running.sim.step_frame();
        }
        // Keep drawing during the hold (the action is frozen, trails fade), as
        // the Video pipeline does, with the ending's flash on top, timed from
        // the frame the End Condition fired on.
        running.renderer.draw(&running.sim);
        if running.sim.is_ended() {
            running
                .renderer
                .flash(Ending::new(0).flash_at(running.held * SUBSTEPS_PER_FRAME));
        }
    }

    /// The current frame as full-resolution RGBA, exactly as the Video renderer
    /// draws it. `None` until a valid Scenario has been loaded.
    pub fn frame(&mut self) -> Option<&[u8]> {
        let running = self.running.as_mut()?;
        Some(running.renderer.frame())
    }

    /// Loads `contents` as the Scenario; returns whether it succeeded. On
    /// failure the previous run (if any) is kept, frozen.
    fn load(&mut self, contents: Option<Vec<u8>>) -> bool {
        let result = Scenario::from_file(&self.path).map_err(|e| e.to_string());
        self.last_contents = contents;
        match result {
            Ok(scenario) => {
                self.running = Some(Running::start(scenario, self.seed));
                self.error = None;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
}

/// Box-filters full-resolution RGBA (opaque) down by `factor` into `0RGB`
/// `u32` pixels, the format window buffers use. Trailing pixels that do not
/// fill a whole block are dropped.
pub fn downscale_to_rgb32(rgba: &[u8], width: u32, height: u32, factor: u32) -> Vec<u32> {
    let (w, h, f) = (width as usize, height as usize, factor.max(1) as usize);
    let (out_w, out_h) = (w / f, h / f);
    let area = (f * f) as u32;
    let mut out = Vec::with_capacity(out_w * out_h);
    for oy in 0..out_h {
        for ox in 0..out_w {
            let mut sum = [0u32; 3];
            for y in oy * f..(oy + 1) * f {
                let row = &rgba[(y * w + ox * f) * 4..(y * w + (ox + 1) * f) * 4];
                for px in row.as_chunks::<4>().0 {
                    for (s, &c) in sum.iter_mut().zip(px) {
                        *s += c as u32;
                    }
                }
            }
            let [r, g, b] = sum.map(|s| (s + area / 2) / area);
            out.push((r << 16) | (g << 8) | b);
        }
    }
    out
}
