//! Audio: pitch assignment through the simulation seam (pitches land in the
//! record), and the soundtrack rendered from a record.

mod support;

use ball_simulator::sim::SimRecord;
use support::{run, MINIMAL};

/// Semitone offsets of each scale from its root, from music theory (not from
/// the code under test).
fn intervals(scale: &str) -> &'static [i64] {
    match scale {
        "major_pentatonic" => &[0, 2, 4, 7, 9],
        "minor_pentatonic" => &[0, 3, 5, 7, 10],
        "lydian" => &[0, 2, 4, 6, 7, 9, 11],
        "dorian" => &[0, 2, 3, 5, 7, 9, 10],
        "mixolydian" => &[0, 2, 4, 5, 7, 9, 10],
        "aeolian" => &[0, 2, 3, 5, 7, 8, 10],
        "suspended_pentatonic" => &[0, 2, 5, 7, 10],
        "hirajoshi" => &[0, 2, 3, 7, 8],
        other => panic!("unknown scale {other}"),
    }
}

/// Semitones of `pitch` above the record's root; panics if not a whole semitone.
fn semitones(record: &SimRecord, pitch: f64) -> i64 {
    let s = 12.0 * (pitch / record.sound.root_hz).log2();
    assert!(
        (s - s.round()).abs() < 1e-6,
        "{pitch} Hz is not a whole number of semitones from the root"
    );
    s.round() as i64
}

fn assert_in_scale(record: &SimRecord, pitch: f64) {
    let scale = record.sound.scale.name();
    let class = semitones(record, pitch).rem_euclid(12);
    assert!(
        intervals(scale).contains(&class),
        "{pitch} Hz (pitch class {class}) is not in {scale}"
    );
}

/// Every Scale name a Scenario may pin.
const SCALES: [&str; 8] = [
    "major_pentatonic",
    "minor_pentatonic",
    "lydian",
    "dorian",
    "mixolydian",
    "aeolian",
    "suspended_pentatonic",
    "hirajoshi",
];

fn with_sound(sound: &str) -> String {
    format!("{MINIMAL}\n[sound]\n{sound}\n")
}

#[test]
fn default_pitch_mode_walks_up_the_scale_one_step_per_hit_and_wraps() {
    // Fast ball: plenty of hits to wrap at least once.
    let fast =
        with_sound("scale = \"major_pentatonic\"").replace("speed = 900.0", "speed = 6000.0");
    let record = run(&fast, 3);

    assert_eq!(record.sound.pitch_mode.name(), "step");
    // Two octaves of major pentatonic, then back to the root.
    let expected = [0, 2, 4, 7, 9, 12, 14, 16, 19, 21, 0, 2, 4];
    assert!(record.boundary_hits.len() >= expected.len());
    for (hit, want) in record.boundary_hits.iter().zip(expected) {
        assert_eq!(semitones(&record, hit.pitch), want);
    }
}

#[test]
fn pitches_are_always_in_the_chosen_scale() {
    let doubling =
        include_str!("../scenarios/doubling.toml").replace("time_cap = 60.0", "time_cap = 8.0");
    for mode in ["step", "ball_size", "angle"] {
        for (seed, scale) in SCALES.into_iter().enumerate() {
            let toml =
                format!("{doubling}\n[sound]\npitch_mode = \"{mode}\"\nscale = \"{scale}\"\n");
            let record = run(&toml, seed as u64);
            assert_eq!(record.sound.scale.name(), scale);
            assert!(!record.boundary_hits.is_empty());
            for hit in &record.boundary_hits {
                assert_in_scale(&record, hit.pitch);
            }
        }
    }
}

#[test]
fn ball_size_pitch_mode_plays_bigger_balls_lower() {
    let sized = |radius: &str| {
        let toml = with_sound("pitch_mode = \"ball_size\"\nscale = \"dorian\"")
            .replace("radius = 30.0", &format!("radius = {radius}"));
        run(&toml, 2)
    };
    let pitches = |r: &SimRecord| -> Vec<f64> { r.boundary_hits.iter().map(|h| h.pitch).collect() };
    let small = pitches(&sized("10.0"));
    let big = pitches(&sized("200.0"));

    assert!(!small.is_empty() && !big.is_empty());
    // One ball that never changes size plays one note.
    assert!(small.iter().all(|&p| p == small[0]));
    assert!(big.iter().all(|&p| p == big[0]));
    assert!(
        big[0] < small[0],
        "big {} should be lower than small {}",
        big[0],
        small[0]
    );
}

#[test]
fn angle_pitch_mode_rises_around_the_boundary() {
    let toml = with_sound("pitch_mode = \"angle\"").replace("speed = 900.0", "speed = 6000.0");
    let record = run(&toml, 5);

    let mut hits: Vec<(f64, f64)> = record
        .boundary_hits
        .iter()
        .map(|h| (h.angle, h.pitch))
        .collect();
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    for pair in hits.windows(2) {
        assert!(
            pair[0].1 <= pair[1].1,
            "pitch falls between angles {} and {}",
            pair[0].0,
            pair[1].0
        );
    }
    let mut distinct: Vec<f64> = hits.iter().map(|h| h.1).collect();
    distinct.dedup();
    assert!(
        distinct.len() >= 3,
        "only {} distinct pitches",
        distinct.len()
    );
}

#[test]
fn boundary_hits_play_notes_only_with_the_play_note_rule() {
    use ball_simulator::audio::{voices, SAMPLES_PER_SUBSTEP};
    let silent = run(MINIMAL, 3);
    let playing = run(&format!("{MINIMAL}\n[[rules]]\nkind = \"play_note\"\n"), 3);
    assert!(!silent.boundary_hits.is_empty());
    assert_eq!(silent.boundary_hits, {
        let mut hits = playing.boundary_hits.clone();
        hits.iter_mut().for_each(|h| h.plays_note = false);
        hits
    });

    let hit_notes = |record: &SimRecord| {
        let chord_at = (record.ending.start_substep * SAMPLES_PER_SUBSTEP) as usize;
        voices(record)
            .iter()
            .filter(|v| v.start != chord_at)
            .count()
    };
    // Without the Rule, only the final chord sounds.
    assert!(silent.boundary_hits.iter().all(|h| !h.plays_note));
    assert_eq!(hit_notes(&silent), 0);
    // With it, every Boundary Hit plays its assigned pitch.
    assert!(playing
        .boundary_hits
        .iter()
        .all(|h| h.plays_note && h.pitch > 0.0));
    assert_eq!(hit_notes(&playing), playing.boundary_hits.len());
}

#[test]
fn instrument_and_scale_vary_by_seed() {
    let mut instruments = std::collections::HashSet::new();
    let mut scales = std::collections::HashSet::new();
    for seed in 0..100 {
        let record = run(MINIMAL, seed);
        instruments.insert(record.sound.instrument.name());
        scales.insert(record.sound.scale.name());
    }
    assert_eq!(instruments.len(), 3, "{instruments:?}");
    assert_eq!(scales.len(), SCALES.len(), "{scales:?}");
}

#[test]
fn pinned_instrument_scale_and_pitch_mode_are_respected() {
    let toml = with_sound("instrument = \"marimba\"\nscale = \"lydian\"\npitch_mode = \"angle\"");
    for seed in 0..10 {
        let record = run(&toml, seed);
        assert_eq!(record.sound.instrument.name(), "marimba");
        assert_eq!(record.sound.scale.name(), "lydian");
        assert_eq!(record.sound.pitch_mode.name(), "angle");
    }
    for seed in 0..10 {
        let record = run(&with_sound("scale = \"hirajoshi\""), seed);
        assert_eq!(record.sound.scale.name(), "hirajoshi");
    }
}

/// The shipped doubling Scenario: thousands of hits from up to 2000 balls.
fn doubling(seed: u64, instrument: &str) -> SimRecord {
    let toml = format!(
        "{}\n[sound]\ninstrument = \"{instrument}\"\n",
        include_str!("../scenarios/doubling.toml")
    );
    run(&toml, seed)
}

#[test]
fn simultaneous_voices_never_exceed_the_cap() {
    use ball_simulator::audio::{voices, MAX_VOICES};
    let record = doubling(1, "pluck");
    let voices = voices(&record);
    assert!(
        voices.len() > MAX_VOICES,
        "the test needs enough notes to hit the cap"
    );

    // Sweep start/end events in time order; ends (exclusive) before starts.
    let mut events: Vec<(usize, i32)> = voices
        .iter()
        .flat_map(|v| [(v.start, 1), (v.end, -1)])
        .collect();
    events.sort_by_key(|&(at, delta)| (at, delta));
    let (mut sounding, mut most) = (0i32, 0i32);
    for (_, delta) in events {
        sounding += delta;
        most = most.max(sounding);
    }
    assert!(
        most as usize <= MAX_VOICES,
        "{most} Voices at once (cap {MAX_VOICES})"
    );
    assert_eq!(
        most as usize, MAX_VOICES,
        "a 2000-ball Video should reach the cap"
    );
}

#[test]
fn a_final_chord_plays_when_the_end_condition_fires() {
    use ball_simulator::audio::{voices, SAMPLES_PER_SUBSTEP};
    for scale in SCALES {
        let record = run(&with_sound(&format!("scale = \"{scale}\"")), 4);
        assert_eq!(record.sound.scale.name(), scale);
        let at = (record.ending.start_substep * SAMPLES_PER_SUBSTEP) as usize;

        let chord: Vec<f64> = voices(&record)
            .iter()
            .filter(|v| v.start == at)
            .map(|v| v.pitch)
            .collect();
        for &pitch in &chord {
            assert_in_scale(&record, pitch);
        }
        let mut distinct = chord.clone();
        distinct.sort_by(f64::total_cmp);
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            chord.len(),
            "{scale}: chord notes are distinct"
        );
        assert!(
            distinct.len() >= 3,
            "{scale}: chord has {} notes",
            distinct.len()
        );
    }
}

#[test]
fn the_final_mix_never_clips_even_with_2000_balls() {
    use ball_simulator::audio::{synthesize, SAMPLE_RATE};
    for instrument in ["sine_bell", "pluck", "marimba"] {
        let record = doubling(1, instrument);
        let samples = synthesize(&record);

        let expected = (record.ending.duration() * SAMPLE_RATE as f64).round() as usize;
        assert_eq!(samples.len(), expected);
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak <= 1.0, "{instrument}: peak {peak}");
        assert!(peak > 0.1, "{instrument}: suspiciously quiet (peak {peak})");
    }
}

#[test]
fn unknown_sound_pins_are_rejected() {
    use ball_simulator::scenario::Scenario;
    for bad in [
        "instrument = \"kazoo\"",
        "scale = \"locrian\"",
        "pitch_mode = \"random\"",
        "tempo = 3",
    ] {
        let toml = format!(
            "{}\n[end_condition]\nkind = \"time_cap\"\n",
            with_sound(bad)
        );
        let err = Scenario::from_toml_str("inline", &toml)
            .expect_err(bad)
            .to_string();
        assert!(err.contains("sound"), "{bad}: {err}");
    }
}
