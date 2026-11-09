//! The starter Scenario set: every shipped Scenario is a real premise that is
//! ready to publish. Each one passes its Quality Gate for most Seeds and aims
//! its Videos at 25-55 s.
//!
//! `cargo test --test starter_scenarios -- --nocapture` prints each
//! Scenario's pass rate over the Seeds tried.

use std::path::{Path, PathBuf};

use ball_simulator::scenario::Scenario;
use ball_simulator::sim;

const SEEDS: std::ops::RangeInclusive<u64> = 1..=20;
/// "Most Seeds": at least this many of the 20 must pass.
const MIN_ACCEPTED: usize = 15;

fn shipped() -> Vec<(PathBuf, Scenario)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    let mut scenarios: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "toml"))
        .map(|path| {
            let scenario = Scenario::from_file(&path).unwrap_or_else(|e| panic!("{e}"));
            (path, scenario)
        })
        .collect();
    scenarios.sort_by(|a, b| a.0.cmp(&b.0));
    scenarios
}

#[test]
fn at_least_six_premises_ship() {
    assert!(shipped().len() >= 6, "only {} Scenarios ship", shipped().len());
}

#[test]
fn every_shipped_scenario_has_its_own_hook_title() {
    let scenarios = shipped();
    let mut titles: Vec<&str> = scenarios.iter().map(|(_, s)| s.hook_title.as_str()).collect();
    titles.sort();
    titles.dedup();
    assert_eq!(titles.len(), scenarios.len(), "Hook Titles must differ");
}

#[test]
fn every_shipped_scenario_has_metadata_templates() {
    for (path, scenario) in shipped() {
        let meta = &scenario.meta;
        assert!(
            meta.title.is_some() && meta.description.is_some() && !meta.hashtags.is_empty(),
            "{} needs a [meta] title, description and hashtags",
            path.display()
        );
    }
}

#[test]
fn every_shipped_scenario_aims_at_25_to_55_second_videos() {
    for (path, scenario) in shipped() {
        let gate = &scenario.quality_gate;
        assert!(
            gate.min_duration >= 25.0 && gate.max_duration <= 55.0,
            "{}: Quality Gate allows {}-{} s",
            path.display(),
            gate.min_duration,
            gate.max_duration
        );
    }
}

#[test]
fn every_shipped_scenario_passes_its_quality_gate_for_most_seeds() {
    let scenarios = shipped();
    let results: Vec<(String, usize, Vec<String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = scenarios
            .iter()
            .map(|(path, scenario)| {
                scope.spawn(move || {
                    let mut accepted = 0;
                    let mut runs = Vec::new();
                    for seed in SEEDS {
                        let record = sim::run(scenario, seed);
                        let ok = record.quality_gate.is_accepted();
                        accepted += usize::from(ok);
                        runs.push(format!(
                            "seed {seed}: {:.1} s, {} hits, {} balls, {} {}",
                            record.ending.cut_time(),
                            record.boundary_hits.len(),
                            record.ball_count.last().unwrap().count,
                            record.end.condition,
                            record
                                .quality_gate
                                .rejection()
                                .map_or("ok".to_string(), |r| r.to_string()),
                        ));
                    }
                    let id = path.file_stem().unwrap().to_string_lossy().into_owned();
                    (id, accepted, runs)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    for (id, accepted, runs) in &results {
        eprintln!("{id}: {accepted}/{} Seeds accepted", SEEDS.count());
        for run in runs {
            eprintln!("    {run}");
        }
    }
    for (id, accepted, _) in &results {
        assert!(
            *accepted >= MIN_ACCEPTED,
            "{id}: only {accepted}/{} Seeds pass its Quality Gate",
            SEEDS.count()
        );
    }
}
