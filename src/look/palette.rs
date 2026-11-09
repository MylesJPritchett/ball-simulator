//! Colour palettes. Each Video uses one, chosen by its Seed unless the
//! Scenario pins one by name.
//!
//! Every palette has exactly [`BALL_SLOTS`] ball colours, so simulation code
//! (e.g. a change-colour Rule) can pick a colour slot `0..BALL_SLOTS` without
//! knowing which palette the Video uses; render maps the slot to RGB.

use crate::sim::rng::Rng;

/// Straight (non-premultiplied) sRGB colour.
pub type Rgb = [u8; 3];

/// Number of ball colours in every palette.
pub const BALL_SLOTS: usize = 5;

#[derive(Debug, PartialEq, Eq)]
pub struct Palette {
    pub name: &'static str,
    /// Dark background behind everything.
    pub background: Rgb,
    pub boundary: Rgb,
    /// Hook Title and Counter text.
    pub text: Rgb,
    pub balls: [Rgb; BALL_SLOTS],
}

impl Palette {
    /// Colour for a ball colour slot (wraps, so any index is valid).
    pub fn ball(&self, slot: usize) -> Rgb {
        self.balls[slot % BALL_SLOTS]
    }
}

/// Every palette, in a fixed order. Appending is safe for pinned Scenarios but
/// changes which palette existing Seeds get; reordering changes it too.
pub const PALETTES: &[Palette] = &[
    Palette {
        name: "sunset",
        background: [16, 10, 24],
        boundary: [246, 232, 222],
        text: [250, 240, 234],
        balls: [
            [255, 107, 107],
            [255, 169, 77],
            [255, 212, 59],
            [247, 131, 172],
            [177, 151, 252],
        ],
    },
    Palette {
        name: "neon",
        background: [8, 8, 18],
        boundary: [222, 240, 255],
        text: [236, 246, 255],
        balls: [
            [0, 245, 212],
            [241, 91, 181],
            [254, 228, 64],
            [0, 187, 249],
            [155, 93, 229],
        ],
    },
    Palette {
        name: "ocean",
        background: [6, 14, 26],
        boundary: [214, 238, 250],
        text: [232, 246, 252],
        balls: [
            [76, 201, 240],
            [72, 149, 239],
            [114, 239, 221],
            [128, 255, 219],
            [184, 192, 255],
        ],
    },
    Palette {
        name: "candy",
        background: [20, 12, 24],
        boundary: [252, 236, 246],
        text: [255, 244, 250],
        balls: [
            [255, 153, 200],
            [252, 246, 189],
            [208, 244, 222],
            [169, 222, 249],
            [228, 193, 249],
        ],
    },
    Palette {
        name: "ember",
        background: [18, 8, 8],
        boundary: [255, 228, 206],
        text: [255, 240, 228],
        balls: [
            [255, 77, 0],
            [255, 136, 0],
            [255, 183, 0],
            [255, 208, 0],
            [255, 0, 110],
        ],
    },
    Palette {
        name: "aurora",
        background: [6, 12, 18],
        boundary: [220, 250, 240],
        text: [236, 255, 248],
        balls: [
            [124, 255, 107],
            [45, 226, 230],
            [157, 78, 221],
            [247, 37, 133],
            [181, 255, 60],
        ],
    },
];

pub fn by_name(name: &str) -> Option<&'static Palette> {
    PALETTES.iter().find(|p| p.name == name)
}

/// Mixed into the Seed so palette choice has its own random stream and never
/// consumes draws from the simulation's RNG.
const PALETTE_STREAM: u64 = 0x7061_6c65_7474_6521;

/// The palette for a Video: the pinned one if given (must be a valid name,
/// which Scenario validation guarantees), else one chosen by the Seed.
pub fn for_video(pinned: Option<&str>, seed: u64) -> &'static Palette {
    match pinned {
        Some(name) => by_name(name).expect("pinned palette was validated"),
        None => {
            let draw = Rng::from_seed(seed ^ PALETTE_STREAM).next_u64();
            &PALETTES[(draw % PALETTES.len() as u64) as usize]
        }
    }
}
