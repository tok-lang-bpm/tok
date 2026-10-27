//! Discrete-event simulator of the tok process language.
//!
//! This crate ports `sim.py` of the Python implementation. Every random draw of
//! `sim.py` comes from one `random.Random(seed)`; [`PyRandom`] reproduces that
//! generator exactly, so the same model and seed give the same event log.

pub mod pyrandom;

pub use pyrandom::{PyRandom, SeedError};
