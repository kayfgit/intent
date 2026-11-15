//! The pure engine of the Intent chord input system.
//!
//! This crate has no I/O and no clock. Hosts (a desktop app, a web page, a
//! wearable's firmware) feed it key events and act on what it returns, so the
//! same engine runs everywhere and every behaviour can be tested.

pub mod chord;
pub mod key;

pub use chord::{Chord, HandShape, ParseChordError, ParseShapeError, Plane};
pub use key::{Finger, Hand, Key};
