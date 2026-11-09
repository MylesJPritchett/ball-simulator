//! Fixed safe-zone layout for 1080x1920 vertical Video.
//!
//! Top ~15%: Hook Title. Bottom ~20% and the right edge: platform UI, kept clear.
//! The Arena is centred in the band between them; the Counter sits below it.

pub const WIDTH: u32 = 1080;
pub const HEIGHT: u32 = 1920;

/// Height reserved for the Hook Title at the top.
pub const TITLE_BAND: f32 = HEIGHT as f32 * 0.15;
/// Height kept clear of content at the bottom for platform captions.
pub const BOTTOM_UI_BAND: f32 = HEIGHT as f32 * 0.20;
/// Height reserved for the Counter just above the bottom UI band.
pub const COUNTER_BAND: f32 = 140.0;
/// Side margin, also keeping clear of the right-edge buttons.
pub const SIDE_MARGIN: f32 = 90.0;

/// Where the Arena is drawn, in pixels.
#[derive(Debug, Clone, Copy)]
pub struct ArenaLayout {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
}

pub fn arena() -> ArenaLayout {
    let top = TITLE_BAND;
    let bottom = HEIGHT as f32 - BOTTOM_UI_BAND - COUNTER_BAND;
    let radius = ((bottom - top) / 2.0).min(WIDTH as f32 / 2.0 - SIDE_MARGIN);
    ArenaLayout {
        center_x: WIDTH as f32 / 2.0,
        center_y: (top + bottom) / 2.0,
        radius,
    }
}
