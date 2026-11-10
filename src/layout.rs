//! Video frame geometry: the fixed safe-zone layout for 1080x1920 vertical
//! Video. The renderer draws to it, and the simulation reads it to know when
//! an escaping ball has left the frame.
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
pub const COUNTER_BAND: f32 = 180.0;
/// Side margin on both sides (the Arena stays centred), wide enough to keep
/// clear of the right-edge buttons.
pub const SIDE_MARGIN: f32 = 120.0;
/// Space kept clear at the very top for the status bar / platform header.
const TOP_MARGIN: f32 = 72.0;
/// Gap between neighbouring regions.
const GAP: f32 = 24.0;

/// An axis-aligned box in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Region {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn center_x(&self) -> f32 {
        (self.left + self.right) / 2.0
    }

    pub fn center_y(&self) -> f32 {
        (self.top + self.bottom) / 2.0
    }
}

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

/// The box the Hook Title is fitted into.
pub fn hook_title() -> Region {
    Region {
        left: SIDE_MARGIN,
        top: TOP_MARGIN,
        right: WIDTH as f32 - SIDE_MARGIN,
        bottom: TITLE_BAND - GAP / 2.0,
    }
}

/// The box the Counter is fitted into: below the Arena, above the bottom UI band.
pub fn counter() -> Region {
    let a = arena();
    Region {
        left: SIDE_MARGIN,
        top: a.center_y + a.radius + GAP,
        right: WIDTH as f32 - SIDE_MARGIN,
        bottom: HEIGHT as f32 - BOTTOM_UI_BAND - GAP,
    }
}
