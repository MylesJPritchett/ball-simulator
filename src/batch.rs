//! Batch generation: N Videos from Scenarios picked at random, never the same
//! Scenario twice in a row, each through the Quality Gate.

use std::path::Path;

use crate::pipeline::{self, AcceptedSeed, QualityGateError};
use crate::scenario::{Scenario, ScenarioError};
use crate::sim::rng::Rng;

/// Every `*.toml` Scenario file in `dir`, sorted by file name so a batch Seed
/// means the same batch wherever it runs.
pub fn load_scenarios(dir: &Path) -> Result<Vec<Scenario>, ScenarioError> {
    let io = |source| ScenarioError::Io {
        path: dir.display().to_string(),
        source,
    };
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.extension().is_some_and(|e| e == "toml") {
            paths.push(path);
        }
    }
    paths.sort();
    paths.iter().map(|p| Scenario::from_file(p)).collect()
}

/// One Video of a batch: which Scenario (index into the batch's Scenarios)
/// and the Seed the Quality Gate accepted. `generate --scenario X --seed N`
/// with that Seed reproduces it.
#[derive(Debug, Clone)]
pub struct PlannedVideo {
    pub scenario: usize,
    pub accepted: AcceptedSeed,
}

/// The Videos a batch will render, and the Scenarios that failed their
/// Quality Gate on the way (each is dropped from the rest of the batch).
#[derive(Debug, Clone, Default)]
pub struct BatchPlan {
    pub videos: Vec<PlannedVideo>,
    pub failures: Vec<QualityGateError>,
}

/// Plans `count` Videos headlessly. Everything is drawn from `batch_seed`:
/// which Scenario comes next and the first Seed tried for it, so the same
/// Scenarios and batch Seed always plan the same batch. A Scenario that
/// keeps failing its Quality Gate is reported and not picked again; the plan
/// is short only if every Scenario fails.
pub fn plan(scenarios: &[Scenario], count: usize, batch_seed: u64) -> BatchPlan {
    let mut rng = Rng::from_seed(batch_seed);
    let mut plan = BatchPlan::default();
    let mut retired = Vec::new();
    let mut previous = None;
    while plan.videos.len() < count {
        let Some(index) = pick_scenario(&mut rng, scenarios.len(), previous, &retired) else {
            break;
        };
        let first_seed = rng.next_u64();
        match pipeline::pick_seed(&scenarios[index], pipeline::seeds_from(first_seed)) {
            Ok(accepted) => {
                plan.videos.push(PlannedVideo {
                    scenario: index,
                    accepted,
                });
                previous = Some(index);
            }
            Err(failure) => {
                plan.failures.push(failure);
                retired.push(index);
            }
        }
    }
    plan
}

/// Picks the next Scenario index out of `count`, skipping `retired` ones and,
/// when any other is left, the `previous` one.
pub fn pick_scenario(
    rng: &mut Rng,
    count: usize,
    previous: Option<usize>,
    retired: &[usize],
) -> Option<usize> {
    let live: Vec<usize> = (0..count).filter(|i| !retired.contains(i)).collect();
    let fresh: Vec<usize> = live
        .iter()
        .copied()
        .filter(|&i| Some(i) != previous)
        .collect();
    let candidates = if fresh.is_empty() { live } else { fresh };
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[(rng.next_u64() % candidates.len() as u64) as usize])
}
