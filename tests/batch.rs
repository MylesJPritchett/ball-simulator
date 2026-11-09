//! Batch generation: Scenario picking and the batch plan.

mod support;

use ball_simulator::scenario::Scenario;
use ball_simulator::sim::rng::Rng;
use ball_simulator::{batch, pipeline};
use support::MINIMAL;

/// Draws `n` Scenarios in a row out of `count` from a shuffle-bag, none retired.
fn picks(count: usize, n: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng::from_seed(seed);
    let mut bag = batch::ShuffleBag::new(count);
    (0..n).map(|_| bag.next(&mut rng, &[]).unwrap()).collect()
}

#[test]
fn picks_never_repeat_a_scenario_back_to_back_but_use_them_all() {
    let picked = picks(3, 300, 11);

    assert!(picked.windows(2).all(|w| w[0] != w[1]), "{picked:?}");
    for i in 0..3 {
        assert!(picked.contains(&i), "Scenario {i} never picked");
    }
}

#[test]
fn cycles_come_in_different_orders() {
    let picked = picks(4, 400, 3);
    let mut orders: Vec<&[usize]> = picked.chunks(4).collect();
    orders.sort();
    orders.dedup();
    assert!(
        orders.len() > 5,
        "only {} distinct cycle orders",
        orders.len()
    );
}

#[test]
fn a_single_scenario_is_picked_every_time() {
    assert_eq!(picks(1, 4, 1), vec![0, 0, 0, 0]);
}

#[test]
fn retired_scenarios_are_never_picked() {
    let mut rng = Rng::from_seed(5);
    let mut bag = batch::ShuffleBag::new(4);
    for _ in 0..50 {
        let pick = bag.next(&mut rng, &[1, 3]).unwrap();
        assert!(pick == 0 || pick == 2, "{pick}");
    }
    // Only one Scenario is left: repeating it beats stopping.
    assert_eq!(bag.next(&mut rng, &[0, 1, 3]), Some(2));
    assert_eq!(bag.next(&mut rng, &[0, 1, 3]), Some(2));
    assert_eq!(bag.next(&mut rng, &[0, 1, 2, 3]), None);
}

fn named(id: &str, extra: &str) -> Scenario {
    let toml = format!("{MINIMAL}\n[end_condition]\nkind = \"time_cap\"\n{extra}");
    Scenario::from_toml_str(id, &toml).unwrap()
}

/// Seeds below speed ~600 are too quiet for this gate, so some get rejected.
fn sometimes_quiet(id: &str) -> Scenario {
    let toml = MINIMAL.replace("speed = 900.0", "speed = { min = 200.0, max = 1600.0 }");
    let toml = format!(
        "{toml}\n[end_condition]\nkind = \"time_cap\"\n[quality_gate]\nmin_boundary_hits = 6\n"
    );
    Scenario::from_toml_str(id, &toml).unwrap()
}

/// One slow ball never makes 1000 Boundary Hits in 5 s.
fn always_fails(id: &str) -> Scenario {
    named(id, "[quality_gate]\nmin_boundary_hits = 1000\n")
}

#[test]
fn a_batch_plans_count_videos_alternating_scenarios_through_the_quality_gate() {
    let scenarios = [sometimes_quiet("a"), sometimes_quiet("b"), named("c", "")];
    let plan = batch::plan(&scenarios, 6, 42);

    assert_eq!(plan.videos.len(), 6);
    assert!(plan.failures.is_empty());
    let order: Vec<usize> = plan.videos.iter().map(|v| v.scenario).collect();
    assert!(order.windows(2).all(|w| w[0] != w[1]), "{order:?}");
    for video in &plan.videos {
        assert!(video.accepted.record.quality_gate.is_accepted());
    }
}

#[test]
fn a_batch_uses_every_scenario_once_per_cycle_without_back_to_back_repeats() {
    let scenarios = [
        named("a", ""),
        named("b", ""),
        named("c", ""),
        named("d", ""),
    ];
    for batch_seed in 0..10 {
        let plan = batch::plan(&scenarios, 20, batch_seed);
        let order: Vec<usize> = plan.videos.iter().map(|v| v.scenario).collect();

        for cycle in order.chunks(4) {
            let mut sorted = cycle.to_vec();
            sorted.sort();
            assert_eq!(sorted, vec![0, 1, 2, 3], "batch {batch_seed}: {order:?}");
        }
        assert!(
            order.windows(2).all(|w| w[0] != w[1]),
            "batch {batch_seed}: {order:?}"
        );
    }
}

#[test]
fn every_planned_video_is_reproduced_by_its_scenario_and_accepted_seed() {
    let scenarios = [sometimes_quiet("a"), sometimes_quiet("b")];
    let plan = batch::plan(&scenarios, 8, 7);
    assert!(
        plan.videos.iter().any(|v| !v.accepted.rejected.is_empty()),
        "test wants at least one Seed retry in the batch"
    );

    for video in &plan.videos {
        let scenario = &scenarios[video.scenario];
        let again =
            pipeline::pick_seed(scenario, pipeline::seeds_from(video.accepted.seed)).unwrap();
        assert_eq!(again.seed, video.accepted.seed);
        assert!(again.rejected.is_empty());
        assert_eq!(again.record, video.accepted.record);
    }
}

#[test]
fn the_batch_seed_decides_the_whole_batch() {
    let scenarios = [named("a", ""), named("b", ""), named("c", "")];
    let seeds = |batch_seed| -> Vec<(usize, u64)> {
        batch::plan(&scenarios, 5, batch_seed)
            .videos
            .iter()
            .map(|v| (v.scenario, v.accepted.seed))
            .collect()
    };

    assert_eq!(seeds(3), seeds(3));
    assert_ne!(seeds(3), seeds(4));
}

#[test]
fn a_scenario_failing_its_quality_gate_is_reported_and_the_batch_carries_on() {
    let scenarios = [named("good", ""), always_fails("broken")];
    let plan = batch::plan(&scenarios, 3, 1);

    assert_eq!(plan.videos.len(), 3);
    assert!(plan.videos.iter().all(|v| v.scenario == 0));
    assert_eq!(plan.failures.len(), 1, "a failed Scenario is not retried");
    assert_eq!(plan.failures[0].scenario, "broken");
}

#[test]
fn a_batch_where_every_scenario_fails_plans_nothing() {
    let scenarios = [always_fails("x"), always_fails("y")];
    let plan = batch::plan(&scenarios, 3, 1);

    assert!(plan.videos.is_empty());
    let failed: Vec<&str> = plan.failures.iter().map(|f| f.scenario.as_str()).collect();
    assert_eq!(failed.len(), 2);
    assert!(failed.contains(&"x") && failed.contains(&"y"));
}
