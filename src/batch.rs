//! Batch generation: N Videos from Scenarios drawn from a seeded shuffle-bag
//! (each Scenario once per cycle, never the same one twice in a row), each
//! through the Quality Gate.

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
/// the shuffle-bag order of Scenarios and the first Seed tried for each, so the same
/// Scenarios and batch Seed always plan the same batch. A Scenario that
/// keeps failing its Quality Gate is reported and not picked again; the plan
/// is short only if every Scenario fails.
pub fn plan(scenarios: &[Scenario], count: usize, batch_seed: u64) -> BatchPlan {
    let mut rng = Rng::from_seed(batch_seed);
    let mut plan = BatchPlan::default();
    let mut retired = Vec::new();
    let mut bag = ShuffleBag::new(scenarios.len());
    while plan.videos.len() < count {
        let Some(index) = bag.next(&mut rng, &retired) else {
            break;
        };
        let first_seed = rng.next_u64();
        match pipeline::pick_seed(&scenarios[index], pipeline::seeds_from(first_seed)) {
            Ok(accepted) => {
                plan.videos.push(PlannedVideo {
                    scenario: index,
                    accepted,
                });
            }
            Err(failure) => {
                plan.failures.push(failure);
                retired.push(index);
            }
        }
    }
    plan
}

/// Seeded shuffle-bag of Scenario indices: every live Scenario comes up once
/// per cycle, in a fresh shuffled order each cycle, and a cycle never starts
/// with the Scenario that ended the last one (unless it is the only one left).
#[derive(Debug, Clone)]
pub struct ShuffleBag {
    count: usize,
    /// What is left of the current cycle; the next pick is the last element.
    bag: Vec<usize>,
    previous: Option<usize>,
}

impl ShuffleBag {
    /// A bag over Scenario indices `0..count`.
    pub fn new(count: usize) -> Self {
        ShuffleBag {
            count,
            bag: Vec::new(),
            previous: None,
        }
    }

    /// The next Scenario index, skipping `retired` ones; `None` once every
    /// Scenario is retired.
    pub fn next(&mut self, rng: &mut Rng, retired: &[usize]) -> Option<usize> {
        self.bag.retain(|i| !retired.contains(i));
        if self.bag.is_empty() {
            self.refill(rng, retired);
        }
        let pick = self.bag.pop()?;
        self.previous = Some(pick);
        Some(pick)
    }

    fn refill(&mut self, rng: &mut Rng, retired: &[usize]) {
        self.bag = (0..self.count).filter(|i| !retired.contains(i)).collect();
        // Fisher-Yates, drawing only from the batch RNG.
        for i in (1..self.bag.len()).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            self.bag.swap(i, j);
        }
        // No back-to-back repeat across the cycle boundary: the first pick
        // (the last element) swaps with the start of the cycle.
        let n = self.bag.len();
        if n > 1 && self.bag.last().copied() == self.previous {
            self.bag.swap(0, n - 1);
        }
    }
}
