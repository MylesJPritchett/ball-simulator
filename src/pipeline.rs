//! One Video, end to end: simulate -> Quality Gate (retrying Seeds) ->
//! synthesize audio -> render + encode -> metadata.

use std::path::{Path, PathBuf};

use crate::audio;
use crate::encode::{EncodeError, VideoEncoder};
use crate::meta::VideoMeta;
use crate::render::{layout, Renderer};
use crate::scenario::Scenario;
use crate::sim::{self, Rejection, SimRecord, Simulation, FPS};

#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("output error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    QualityGate(#[from] QualityGateError),
}

/// Seeds tried per Video before giving up on a Scenario.
pub const MAX_SEED_ATTEMPTS: usize = 10;

/// A Seed the Quality Gate turned down, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct RejectedSeed {
    pub seed: u64,
    pub reason: Rejection,
}

/// The first Seed that passed the Quality Gate, with its dry-run record and
/// the Seeds rejected before it.
#[derive(Debug, Clone)]
pub struct AcceptedSeed {
    pub seed: u64,
    pub record: SimRecord,
    pub rejected: Vec<RejectedSeed>,
}

/// Every Seed tried for a Scenario was rejected.
#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "Scenario '{scenario}' failed the Quality Gate on {} Seeds in a row (last, Seed {}: {})",
    rejected.len(),
    last_seed(rejected),
    last_reason(rejected)
)]
pub struct QualityGateError {
    /// Scenario id.
    pub scenario: String,
    /// Every Seed tried, in order.
    pub rejected: Vec<RejectedSeed>,
}

fn last_seed(rejected: &[RejectedSeed]) -> String {
    rejected
        .last()
        .map_or_else(|| "none".into(), |r| r.seed.to_string())
}

fn last_reason(rejected: &[RejectedSeed]) -> String {
    rejected
        .last()
        .map_or_else(|| "no Seeds to try".into(), |r| r.reason.to_string())
}

/// Candidate Seeds starting at `first`: `first, first + 1, ...`. The first
/// Seed is always tried first, so a Seed that passed once reproduces its Video.
pub fn seeds_from(first: u64) -> impl Iterator<Item = u64> {
    (0..).map(move |i| first.wrapping_add(i))
}

/// Dry-runs candidate Seeds headlessly, at most [`MAX_SEED_ATTEMPTS`] of them,
/// and returns the first the Quality Gate accepts.
pub fn pick_seed(
    scenario: &Scenario,
    seeds: impl IntoIterator<Item = u64>,
) -> Result<AcceptedSeed, QualityGateError> {
    let mut rejected = Vec::new();
    for seed in seeds.into_iter().take(MAX_SEED_ATTEMPTS) {
        let record = sim::run(scenario, seed);
        match record.quality_gate.rejection() {
            None => {
                return Ok(AcceptedSeed {
                    seed,
                    record,
                    rejected,
                })
            }
            Some(reason) => rejected.push(RejectedSeed {
                seed,
                reason: reason.clone(),
            }),
        }
    }
    Err(QualityGateError {
        scenario: scenario.id.clone(),
        rejected,
    })
}

/// A rendered Video: its folder, the Seed it used and any Seeds rejected first.
#[derive(Debug, Clone)]
pub struct Generated {
    pub dir: PathBuf,
    pub seed: u64,
    pub rejected: Vec<RejectedSeed>,
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

/// Generates one Video, starting from `seed` and moving on to the next Seeds
/// while the Quality Gate rejects them.
pub fn generate(
    scenario: &Scenario,
    seed: u64,
    out_root: &Path,
) -> Result<Generated, GenerateError> {
    let accepted = pick_seed(scenario, seeds_from(seed))?;
    render_accepted(scenario, accepted, out_root)
}

/// Renders the Video for a Seed the Quality Gate already accepted.
pub fn render_accepted(
    scenario: &Scenario,
    accepted: AcceptedSeed,
    out_root: &Path,
) -> Result<Generated, GenerateError> {
    let dir = render(scenario, &accepted.record, out_root)?;
    Ok(Generated {
        dir,
        seed: accepted.seed,
        rejected: accepted.rejected,
    })
}

/// Renders the Video for a Seed's dry-run record and returns its folder.
fn render(
    scenario: &Scenario,
    record: &SimRecord,
    out_root: &Path,
) -> Result<PathBuf, GenerateError> {
    let seed = record.seed;
    let dir = video_dir(out_root, scenario, seed);
    std::fs::create_dir_all(&dir).map_err(io_err(&dir))?;

    // The Video runs through the ending's hold, not just to the End Condition.
    let ending = record.ending;
    let frames = ending.frames();

    let wav = dir.join("audio.wav");
    let samples = audio::synthesize(record);
    audio::write_wav(&wav, &samples, audio::SAMPLE_RATE).map_err(io_err(&wav))?;

    // Replay the same deterministic run for frames.
    let mp4 = dir.join("video.mp4");
    let mut encoder = VideoEncoder::start(&mp4, &wav, layout::WIDTH, layout::HEIGHT, FPS)?;
    let mut sim = Simulation::new(scenario, seed);
    // Built from the Seed's resolved Scenario (ranges such as the Arena radius
    // resolved), not the parsed file's lower bounds.
    let mut renderer = Renderer::new(sim.scenario());
    // Frame k shows the state after k * SUBSTEPS_PER_FRAME substeps; once the
    // run has ended `step_frame` does nothing, so the action freezes, the
    // trails keep fading and the renderer plays the flash over the frozen frame.
    for _ in 0..frames {
        encoder.write_frame(renderer.draw(&sim, Some(&ending)))?;
        sim.step_frame();
    }
    encoder.finish()?;
    std::fs::remove_file(&wav).map_err(io_err(&wav))?;

    let meta_path = dir.join("meta.json");
    let meta = serde_json::to_string_pretty(&VideoMeta::new(scenario, record))
        .expect("metadata serializes");
    std::fs::write(&meta_path, meta).map_err(io_err(&meta_path))?;

    Ok(dir)
}
