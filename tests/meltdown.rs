//! The meltdown End Condition and its staged Play-out, tested through the
//! sim, renderer and audio seams.

mod support;

use ball_simulator::sim::{Finale, SUBSTEP_HZ};
use support::{run, MINIMAL};

/// One ball that splits in two on every Boundary Hit and melts down once
/// there are 50.
fn melting() -> String {
    MINIMAL.replace("time_cap = 5.0", "time_cap = 5.0\npopulation_cap = 50")
        + "\n[[rules]]\nkind = \"split\"\ninto = 2\nspread = 40.0\n\n\
           [end_condition]\nkind = \"meltdown\"\n"
}

#[test]
fn meltdown_fires_when_the_ball_count_reaches_the_population_cap() {
    for seed in 0..5 {
        let record = run(&melting(), seed);
        let reached = record
            .ball_count
            .iter()
            .find(|s| s.count == 50)
            .expect("cap reached");

        assert_eq!(record.end.condition, "meltdown", "seed {seed}");
        assert_eq!(record.end.substep, reached.substep, "seed {seed}");
    }
}

#[test]
fn the_meltdown_plays_out_without_a_flash_or_final_chord_then_holds() {
    let record = run(&melting(), 1);
    let ending = &record.ending;

    assert_eq!(ending.finale, Finale::Meltdown);
    assert_eq!(ending.flash_substeps, 0, "no flash");
    assert!(!ending.final_chord, "no final chord");
    // A 1.5 s Play-out, then the usual 1.5 s hold.
    assert_eq!(
        ending.freeze_substep - ending.start_substep,
        SUBSTEP_HZ * 3 / 2
    );
    assert_eq!(
        ending.cut_substep - ending.freeze_substep,
        SUBSTEP_HZ * 3 / 2
    );
}
