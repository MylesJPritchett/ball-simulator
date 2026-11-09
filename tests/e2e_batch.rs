//! End-to-end batch: `generate --count N` writes N Video folders with filled-in
//! metadata, survives a failing Scenario, and any Video it made is reproduced
//! by `generate --scenario X --seed N`. Requires ffmpeg on PATH.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Tiny Scenario: 0.2 s of action, so each Video is mostly the ending hold.
fn tiny(name: &str, extra: &str) -> String {
    format!(
        r#"
name = "{name}"
hook_title = "{name} hook"
time_cap = 0.2
[arena]
radius = 460.0
[ball]
radius = 30.0
speed = {{ min = 3000.0, max = 4000.0 }}
[end_condition]
kind = "time_cap"
[meta]
title = "{name} at {{ball.speed}}"
hashtags = ["{name}"]
{extra}
"#
    )
}

fn generate(args: &[&str], out: &Path) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_ball-simulator"))
        .arg("generate")
        .args(args)
        .arg("--out")
        .arg(out)
        .output()
        .expect("binary should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn meta(dir: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap()
}

#[test]
fn a_batch_writes_count_videos_and_each_reproduces_from_its_scenario_and_seed() {
    let root = scratch_dir("e2e_batch");
    let scenarios = root.join("scenarios");
    std::fs::create_dir_all(&scenarios).unwrap();
    std::fs::write(scenarios.join("alpha.toml"), tiny("alpha", "")).unwrap();
    std::fs::write(scenarios.join("beta.toml"), tiny("beta", "")).unwrap();
    // Never makes 1000 Boundary Hits in 0.2 s.
    std::fs::write(
        scenarios.join("dull.toml"),
        tiny("dull", "[quality_gate]\nmin_boundary_hits = 1000"),
    )
    .unwrap();
    let out = root.join("out");

    let scenarios_arg = scenarios.to_str().unwrap();
    let batch = generate(
        &["--count", "2", "--seed", "3", "--scenarios", scenarios_arg],
        &out,
    );

    let dirs: Vec<PathBuf> = String::from_utf8(batch.stdout)
        .unwrap()
        .lines()
        .map(PathBuf::from)
        .collect();
    assert_eq!(dirs.len(), 2, "one folder per Video");
    let metas: Vec<_> = dirs.iter().map(|d| meta(d)).collect();
    assert_ne!(metas[0]["scenario"], metas[1]["scenario"]);
    for (dir, meta) in dirs.iter().zip(&metas) {
        assert!(dir.join("video.mp4").exists());
        let name = meta["scenario"].as_str().unwrap();
        assert_ne!(name, "dull");
        let speed = meta["parameters"]["ball.speed"].as_f64().unwrap();
        assert_eq!(
            meta["title"],
            format!("{name} at {}", (speed * 10.0).round() / 10.0)
        );
        assert_eq!(meta["hashtags"][0], format!("#{name}"));
        assert!(meta["hashtags"].as_array().unwrap().len() > 1);
    }
    let stderr = String::from_utf8(batch.stderr).unwrap();
    assert!(stderr.contains("Batch Seed 3"), "{stderr}");
    // Batch Seed 3 happens to pick "dull" first: it is reported, never
    // rendered, and the batch still makes its 2 Videos.
    assert!(
        stderr.contains("'dull' failed the Quality Gate"),
        "{stderr}"
    );
    assert_eq!(out.read_dir().unwrap().count(), 2);

    // Reproduce the first Video on its own.
    let again_out = root.join("again");
    let scenario_file = scenarios.join(format!("{}.toml", metas[0]["scenario"].as_str().unwrap()));
    let seed = metas[0]["seed"].to_string();
    let single = generate(
        &[
            "--scenario",
            scenario_file.to_str().unwrap(),
            "--seed",
            &seed,
        ],
        &again_out,
    );
    let again_dir = PathBuf::from(String::from_utf8(single.stdout).unwrap().trim());
    assert_eq!(again_dir.file_name(), dirs[0].file_name());
    assert_eq!(meta(&again_dir), metas[0]);
    assert_eq!(
        std::fs::read(again_dir.join("video.mp4")).unwrap(),
        std::fs::read(dirs[0].join("video.mp4")).unwrap(),
        "same Scenario and Seed give a byte-identical Video"
    );
}
