//! The pure engine of the Intent chord input system.
//!
//! This crate has no I/O and no clock. Hosts (a desktop app, a web page, a
//! wearable's firmware) feed it key events and act on what it returns, so the
//! same engine runs everywhere and every behaviour can be tested.

pub mod chord;
pub mod effort;
pub mod key;
pub mod stroke;

pub use chord::{Chord, HandShape, ParseChordError, ParseShapeError, Plane};
pub use effort::EffortModel;
pub use key::{Finger, Hand, Key};
pub use stroke::{KeyAction, KeyEvent, Stroke, StrokeDetector};
