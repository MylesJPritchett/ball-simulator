//! Video metadata written next to the `.mp4` as `meta.json`.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::scenario::{ParamValue, Scenario};
use crate::sim::SimRecord;

pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
pub struct VideoMeta {
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
    /// Scenario id (file stem); with `seed` and `generator_version` this
    /// recreates the Video exactly.
    pub scenario: String,
    pub seed: u64,
    pub generator_version: String,
    /// Each Scenario range (by field path) and the value this Seed resolved it to.
    pub parameters: BTreeMap<String, ParamValue>,
}

impl VideoMeta {
    /// Placeholder copy until metadata templates exist: the Hook Title.
    pub fn new(scenario: &Scenario, record: &SimRecord) -> Self {
        VideoMeta {
            title: scenario.hook_title.clone(),
            description: scenario.hook_title.clone(),
            hashtags: Vec::new(),
            scenario: scenario.id.clone(),
            seed: record.seed,
            generator_version: GENERATOR_VERSION.to_string(),
            parameters: record
                .parameters
                .iter()
                .map(|p| (p.field.clone(), p.value))
                .collect(),
        }
    }
}
