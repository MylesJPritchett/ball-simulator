//! The starter Scenario set: every shipped Scenario is a real premise that is
//! ready to publish. Each one passes its Quality Gate for most Seeds and aims
//! its Videos at 25-55 s.
//!
//! `cargo test --test starter_scenarios -- --nocapture` prints each
//! Scenario's pass rate over the Seeds tried.

use std::path::{Path, PathBuf};

use ball_simulator::look::palette::BALL_SLOTS;
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
    assert!(
        shipped().len() >= 6,
        "only {} Scenarios ship",
        shipped().len()
    );
}

#[test]
fn every_shipped_scenario_has_its_own_hook_title() {
    let scenarios = shipped();
    let mut titles: Vec<&str> = scenarios
        .iter()
        .map(|(_, s)| s.hook_title.as_str())
        .collect();
    titles.sort();
    titles.dedup();
    assert_eq!(titles.len(), scenarios.len(), "Hook Titles must differ");
}

#[test]
fn doubling_alone_ships_with_heat() {
    // gravity_split keeps per-ball colour so it looks different from doubling.
    let with_heat: Vec<String> = shipped()
        .into_iter()
        .filter(|(_, s)| s.look.heat)
        .map(|(_, s)| s.id)
        .collect();
    assert_eq!(with_heat, ["doubling"]);
}

#[test]
fn doubling_melts_down_at_its_population_cap_glowing_through_ember() {
    let (_, doubling) = shipped()
        .into_iter()
        .find(|(_, s)| s.id == "doubling")
        .expect("doubling ships");
    assert_eq!(doubling.end_condition.kind, "meltdown");
    // Ember's Heat ramp runs red -> orange -> white-hot.
    assert_eq!(doubling.look.palette.as_deref(), Some("ember"));
    assert_eq!(
        doubling.hook_title,
        "Every bounce it doubles — until it breaks"
    );
}

#[test]
fn changing_colour_ends_when_its_ball_escapes_through_a_gap() {
    let (_, scenario) = shipped()
        .into_iter()
        .find(|(_, s)| s.id == "changing_colour")
        .expect("changing_colour ships");

    assert!(scenario.rules.iter().any(|rule| rule.kind == "gap"));
    assert_eq!(scenario.end_condition.kind, "escaped");
    assert!(
        scenario.hook_title.contains("escapes"),
        "{}",
        scenario.hook_title
    );
    let title = scenario.meta.title.as_deref().unwrap_or_default();
    assert!(title.contains("escape"), "{title}");
    let record = sim::run(&scenario, 1);
    assert_eq!(record.end.condition, "escaped");
}

#[test]
fn faster_every_bounce_ends_when_its_ball_breaks_out() {
    let (_, scenario) = shipped()
        .into_iter()
        .find(|(_, s)| s.id == "faster_every_bounce")
        .expect("faster_every_bounce ships");

    assert!(scenario.rules.iter().any(|rule| rule.kind == "crack"));
    assert_eq!(scenario.end_condition.kind, "escaped");
    assert!(
        scenario.hook_title.contains("breaks out"),
        "{}",
        scenario.hook_title
    );
    let record = sim::run(&scenario, 1);
    assert_eq!(record.end.condition, "escaped");
    assert!(record.final_balls[0].escaping);
}

#[test]
fn growing_rings_settles_into_a_solid_disc_of_distinct_layers() {
    let (_, scenario) = shipped()
        .into_iter()
        .find(|(_, s)| s.id == "growing_rings")
        .expect("growing_rings ships");
    assert_eq!(scenario.hook_title, "Every bounce it grows");
    assert_eq!(scenario.end_condition.kind, "solid");

    let mut layers = Vec::new();
    for seed in SEEDS {
        let record = sim::run(&scenario, seed);
        if !record.quality_gate.is_accepted() {
            continue;
        }
        assert_eq!(record.end.condition, "solid", "seed {seed}");
        // Neighbouring layers in the same colour would merge into one band.
        let colours: Vec<usize> = record
            .settles
            .iter()
            .map(|settle| settle.colour as usize % BALL_SLOTS)
            .collect();
        assert!(
            colours.windows(2).all(|pair| pair[0] != pair[1]),
            "seed {seed}: neighbouring layers share a colour: {colours:?}"
        );
        layers.push(record.settles.len());
    }
    layers.sort();
    layers.dedup();
    assert!(layers.len() > 1, "Seeds give different numbers of layers");
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    let scenarios = shipped();
    // Every (Scenario, Seed) run is a job shared out across the cores: a
    // million-ball doubling run takes far longer than any other.
    let jobs: Vec<(usize, u64)> = (0..scenarios.len())
        .flat_map(|s| SEEDS.map(move |seed| (s, seed)))
        .collect();
    let next = AtomicUsize::new(0);
    // (scenario index, seed, accepted, summary line)
    let done: Mutex<Vec<(usize, u64, bool, String)>> = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(&(s, seed)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let record = sim::run(&scenarios[s].1, seed);
                    let line = format!(
                        "seed {seed}: {:.1} s, {} hits, {} balls, {} {}",
                        record.ending.cut_time(),
                        record.boundary_hits.len(),
                        record.ball_count.last().unwrap().count,
                        record.end.condition,
                        record
                            .quality_gate
                            .rejection()
                            .map_or("ok".to_string(), |r| r.to_string()),
                    );
                    let ok = record.quality_gate.is_accepted();
                    done.lock().unwrap().push((s, seed, ok, line));
                }
            });
        }
    });
    let mut done = done.into_inner().unwrap();
    done.sort_by_key(|&(s, seed, ..)| (s, seed));
    let results: Vec<(String, usize, Vec<String>)> = scenarios
        .iter()
        .enumerate()
        .map(|(s, (path, _))| {
            let runs: Vec<_> = done.iter().filter(|d| d.0 == s).collect();
            let id = path.file_stem().unwrap().to_string_lossy().into_owned();
            let accepted = runs.iter().filter(|d| d.2).count();
            (id, accepted, runs.iter().map(|d| d.3.clone()).collect())
        })
        .collect();

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
