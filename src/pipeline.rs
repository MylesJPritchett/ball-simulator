//! One Video, end to end: simulate -> synthesize audio -> render + encode -> metadata.

use std::path::{Path, PathBuf};

use crate::audio;
use crate::encode::{EncodeError, VideoEncoder};
use crate::meta::VideoMeta;
use crate::render::{layout, Renderer};
use crate::scenario::Scenario;
use crate::sim::{self, Simulation, FPS, SUBSTEPS_PER_FRAME};

#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("output error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> GenerateError + '_ {
    move |source| GenerateError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Folder a Video is written to: `<out_root>/<scenario>-<seed>`.
pub fn video_dir(out_root: &Path, scenario: &Scenario, seed: u64) -> PathBuf {
    out_root.join(format!("{}-{}", scenario.id, seed))
}

/// Generates one Video and returns its folder.
pub fn generate(scenario: &Scenario, seed: u64, out_root: &Path) -> Result<PathBuf, GenerateError> {
    let dir = video_dir(out_root, scenario, seed);
    std::fs::create_dir_all(&dir).map_err(io_err(&dir))?;

    // Headless run first: audio (and later the Quality Gate) use the record.
    let record = sim::run(scenario, seed);
    // The Video runs through the ending's hold, not just to the End Condition.
    let ending = record.ending;
    let frames = ending.frames();

    let wav = dir.join("audio.wav");
    let samples = audio::synthesize(&record);
    audio::write_wav(&wav, &samples, audio::SAMPLE_RATE).map_err(io_err(&wav))?;

    // Replay the same deterministic run for frames.
    let mp4 = dir.join("video.mp4");
    let mut encoder = VideoEncoder::start(&mp4, &wav, layout::WIDTH, layout::HEIGHT, FPS)?;
    let mut sim = Simulation::new(scenario, seed);
    // The Arena radius may be a range: use the Seed's resolved one.
    let mut renderer = Renderer::new(sim.world().arena.radius);
    // Frame k shows the state after k * SUBSTEPS_PER_FRAME substeps; once the
    // run has ended `step_frame` does nothing, so the action freezes and the
    // flash plays over the frozen frame.
    for frame in 0..frames {
        renderer.draw(sim.world());
        encoder.write_frame(renderer.flash(ending.flash_at(frame * SUBSTEPS_PER_FRAME)))?;
        sim.step_frame();
    }
    encoder.finish()?;
    std::fs::remove_file(&wav).map_err(io_err(&wav))?;

    let meta_path = dir.join("meta.json");
    let meta = serde_json::to_string_pretty(&VideoMeta::new(scenario, &record))
        .expect("metadata serializes");
    std::fs::write(&meta_path, meta).map_err(io_err(&meta_path))?;

    Ok(dir)
}
