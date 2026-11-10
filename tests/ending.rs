//! The ending: when an End Condition fires, an optional Play-out runs, then the
//! action freezes, a flash plays and the frame holds ~1.5 s before the Video
//! cuts. Its timing is on the record.

mod support;

use ball_simulator::sim::{Simulation, FPS, SUBSTEPS_PER_FRAME, SUBSTEP_HZ};
use support::{run, scenario, MINIMAL};

/// One ball that shrinks to nothing on its third Boundary Hit and vanishes.
fn vanishing() -> String {
    format!(
        "{}\n[[rules]]\nkind = \"shrink\"\namount = 10.0\n\n\
         [end_condition]\nkind = \"vanished\"\n",
        MINIMAL
    )
}

#[test]
fn ending_starts_when_the_end_condition_fires_and_holds_for_one_and_a_half_seconds() {
    let record = run(MINIMAL, 1);
    let ending = &record.ending;

    assert_eq!(ending.start_substep, record.end.substep);
    assert_eq!(ending.freeze_substep, ending.start_substep, "no Play-out");
    assert_eq!(
        ending.cut_substep - ending.start_substep,
        SUBSTEP_HZ * 3 / 2
    );
    assert!((ending.cut_time() - (record.end.time() + 1.5)).abs() < 1e-9);
}

#[test]
fn video_frames_cover_the_action_and_the_whole_hold() {
    // A time cap that is not a whole number of frames.
    let toml = MINIMAL.replace("time_cap = 5.0", "time_cap = 1.001");
    let ending = run(&toml, 1).ending;

    let frames = ending.frames();
    assert!(frames * SUBSTEPS_PER_FRAME >= ending.cut_substep);
    assert!((frames - 1) * SUBSTEPS_PER_FRAME < ending.cut_substep);
    assert!((ending.duration() - frames as f64 / FPS as f64).abs() < 1e-12);
}

#[test]
fn flash_peaks_when_the_ending_starts_and_fades_out_before_the_cut() {
    let ending = run(MINIMAL, 1).ending;
    let start = ending.start_substep;

    assert_eq!(
        ending.flash_at(start - 1),
        0.0,
        "no flash during the action"
    );
    assert_eq!(ending.flash_at(start), 1.0);
    let mut last = 1.0;
    for substep in start..=ending.cut_substep {
        let flash = ending.flash_at(substep);
        assert!(flash <= last, "flash only fades");
        last = flash;
    }
    assert_eq!(last, 0.0, "flash has faded by the cut");
}

#[test]
fn vanished_plays_out_before_the_freeze_then_holds_for_one_and_a_half_seconds() {
    let record = run(&vanishing(), 3);
    let ending = &record.ending;

    assert_eq!(record.end.condition, "vanished");
    assert_eq!(ending.start_substep, record.end.substep);
    // A ~0.3 s pop.
    assert_eq!(
        ending.freeze_substep - ending.start_substep,
        SUBSTEP_HZ * 3 / 10
    );
    assert_eq!(
        ending.cut_substep - ending.freeze_substep,
        SUBSTEP_HZ * 3 / 2
    );
    assert!(ending.final_chord);
}

#[test]
fn the_simulation_keeps_going_through_the_play_out_and_stops_at_the_freeze() {
    let scenario = scenario(&vanishing());
    let mut sim = Simulation::new(&scenario, 3);
    while sim.end().is_none() {
        sim.step();
    }
    let ending = sim.ending().expect("the End Condition has fired");
    assert_eq!(sim.substep(), ending.start_substep);
    assert!(!sim.is_frozen(), "the Play-out has started");

    for _ in 0..ending.freeze_substep * 2 {
        sim.step();
    }
    assert_eq!(sim.substep(), ending.freeze_substep);
    assert!(sim.is_frozen());
    assert_eq!(sim.ending(), Some(ending), "the Ending did not change");
    assert_eq!(sim.into_record().ending, ending);
}

#[test]
fn the_flash_starts_when_the_end_condition_fires_not_at_the_freeze() {
    let ending = run(&vanishing(), 3).ending;

    assert_eq!(ending.flash_at(ending.start_substep - 1), 0.0);
    assert_eq!(ending.flash_at(ending.start_substep), 1.0);
}
