//! Ball simulator: generates satisfying ball-bounce Videos from a Scenario and a Seed.
//!
//! Pipeline: [`scenario`] parses a Scenario file, [`sim::run`] produces a
//! [`sim::SimRecord`] (the one test seam), then [`audio`] and [`render`] consume
//! the record / replay the simulation and [`encode`] muxes them into a Video.
//! [`pipeline`] wires these together for one Video.

pub mod audio;
pub mod encode;
pub mod meta;
pub mod pipeline;
pub mod preview;
pub mod render;
pub mod rules;
pub mod scenario;
pub mod sim;
