//! The safe-zone layout keeps the Hook Title, Arena and Counter clear of
//! platform UI (bottom ~20%, right edge).

use ball_simulator::layout::{self, Region};

/// Bottom 20% of the frame: captions, usernames, music ticker.
const BOTTOM_UI_TOP: f32 = layout::HEIGHT as f32 * 0.80;
/// Right edge: like / comment / share buttons.
const RIGHT_UI_LEFT: f32 = layout::WIDTH as f32 - 100.0;

fn arena_region() -> Region {
    let a = layout::arena();
    Region {
        left: a.center_x - a.radius,
        top: a.center_y - a.radius,
        right: a.center_x + a.radius,
        bottom: a.center_y + a.radius,
    }
}

fn assert_clear_of_platform_ui(name: &str, r: Region) {
    assert!(r.left >= 0.0 && r.top >= 0.0, "{name} off frame: {r:?}");
    assert!(r.bottom <= BOTTOM_UI_TOP, "{name} in bottom UI zone: {r:?}");
    assert!(
        r.right <= RIGHT_UI_LEFT,
        "{name} in right-edge UI zone: {r:?}"
    );
}

#[test]
fn hook_title_sits_in_the_top_15_percent() {
    let title = layout::hook_title();
    assert_clear_of_platform_ui("Hook Title", title);
    assert!(title.bottom <= layout::HEIGHT as f32 * 0.15, "{title:?}");
}

#[test]
fn arena_and_counter_are_clear_of_platform_ui() {
    assert_clear_of_platform_ui("Arena", arena_region());
    assert_clear_of_platform_ui("Counter", layout::counter());
}

#[test]
fn arena_is_below_the_title_and_the_counter_below_the_arena() {
    let (title, arena, counter) = (layout::hook_title(), arena_region(), layout::counter());
    assert!(title.bottom <= arena.top, "{title:?} {arena:?}");
    assert!(arena.bottom <= counter.top, "{arena:?} {counter:?}");
}

#[test]
fn arena_is_horizontally_centred() {
    let a = layout::arena();
    assert_eq!(a.center_x, layout::WIDTH as f32 / 2.0);
}
