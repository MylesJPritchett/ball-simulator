//! End-to-end smoke test: `generate` produces a playable Video.
//! Requires `ffmpeg` and `ffprobe` on PATH (provided by the flake dev shell).

use std::path::PathBuf;
use std::process::Command;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn probe(path: &std::path::Path, entries: &str, stream: &str) -> String {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            stream,
            "-show_entries",
            entries,
        ])
        .args(["-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(path)
        .output()
        .expect("ffprobe should run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn generate_writes_a_one_second_video_with_audio_and_metadata() {
    let dir = scratch_dir("e2e_smoke");
    let scenario = dir.join("smoke.toml");
    std::fs::write(
        &scenario,
        r#"
name = "Smoke"
time_cap = 2.0
[arena]
radius = 460.0
[ball]
radius = 30.0
speed = { min = 2500.0, max = 3500.0 }
# The Arena closes in on the ball, filling it 1 s in.
[[rules]]
kind = "shrinking_arena"
rate = 430.0
[end_condition]
kind = "arena_filled"
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_ball-simulator"))
        .args(["generate", "--seed", "5", "--out"])
        .arg(&dir)
        .arg("--scenario")
        .arg(&scenario)
        .status()
        .expect("binary should run");
    assert!(status.success());

    let video_dir = dir.join("smoke-5");
    let video = video_dir.join("video.mp4");
    assert!(video.exists(), "missing {}", video.display());

    assert_eq!(probe(&video, "stream=width,height", "v:0"), "1080\n1920");
    assert_eq!(probe(&video, "stream=r_frame_rate", "v:0"), "60/1");
    assert_eq!(probe(&video, "stream=codec_type", "a:0"), "audio");
    let duration: f64 = probe(&video, "format=duration", "v:0").parse().unwrap();
    // 1 s of action, then the ending's ~1.5 s freeze + flash + hold.
    assert!((duration - 2.5).abs() < 0.1, "duration {duration}");

    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(video_dir.join("meta.json")).unwrap())
            .unwrap();
    assert_eq!(meta["scenario"], "smoke");
    assert_eq!(meta["seed"], 5);
    assert_eq!(meta["generator_version"], env!("CARGO_PKG_VERSION"));
    assert!(meta["title"].is_string());
    let speed = meta["parameters"]["ball.speed"]
        .as_f64()
        .expect("resolved ranges are recorded in meta.json");
    assert!((2500.0..3500.0).contains(&speed), "speed {speed}");
}
