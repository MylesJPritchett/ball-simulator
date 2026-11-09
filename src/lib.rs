//! Ball simulator: generates satisfying ball-bounce Videos from a Scenario and a Seed.
//!
//! Pipeline: [`scenario`] parses a Scenario file, [`sim::run`] produces a
//! [`sim::SimRecord`] (the one test seam), then [`audio`] and [`render`] consume
//! the record / replay the simulation and [`encode`] muxes them into a Video.

pub mod rules;
pub mod scenario;
pub mod sim;
