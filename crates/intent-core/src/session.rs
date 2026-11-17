//! A stroke detector and an engine wired together.
//!
//! Every host does the same thing with key events: detect strokes, then
//! apply them. `Session` is that wiring, so hosts only translate their
//! platform's key events in and act on the outcomes that come back.

use crate::chord::Chord;
use crate::engine::{Effect, Engine};
use crate::layout::Layout;
use crate::stroke::{KeyEvent, Stroke, StrokeDetector};

/// A completed stroke and what it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The stroke that just ended.
    pub stroke: Stroke,
    /// What the host should do about it.
    pub effect: Effect,
}

/// Key events in, outcomes out.
#[derive(Debug, Clone)]
pub struct Session {
    detector: StrokeDetector,
    engine: Engine,
}

impl Session {
    /// A session with no keys down and an empty buffer.
    pub fn new(layout: Layout) -> Session {
        Session {
            detector: StrokeDetector::new(),
            engine: Engine::new(layout),
        }
    }

    /// Feeds one key event. Returns an outcome when a stroke completes.
    pub fn on_event(&mut self, event: KeyEvent) -> Option<Outcome> {
        let stroke = self.detector.on_event(event)?;
        let effect = self.engine.stroke(stroke.chord);
        Some(Outcome { stroke, effect })
    }

    /// Applies a chord directly, as if it had just been stroked.
    ///
    /// For on-screen buttons and tests, where there are no key events.
    pub fn strike(&mut self, chord: Chord) -> Effect {
        self.engine.stroke(chord)
    }

    /// The keys that are down right now.
    pub fn held(&self) -> Chord {
        self.detector.held()
    }

    /// The chord built so far in the stroke in progress.
    pub fn pending(&self) -> Chord {
        self.detector.pending()
    }

    /// Drops any stroke in progress, keeping the buffer.
    ///
    /// Hosts call this when they lose keyboard focus, since the key releases
    /// will never arrive.
    pub fn release_all(&mut self) {
        self.detector.reset();
    }

    /// The engine, for reading the buffer and the layout.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{Finger, Hand, Key};

    const V0: &str = include_str!("../../../layouts/intent-v0.toml");
    const LI: Key = Key::new(Hand::Left, Finger::Index);
    const RM: Key = Key::new(Hand::Right, Finger::Middle);

    fn session() -> Session {
        Session::new(Layout::from_toml(V0).expect("intent-v0 compiles").layout)
    }

    fn chord(text: &str) -> Chord {
        text.parse().expect("valid chord")
    }

    #[test]
    fn key_events_become_outcomes() {
        let mut session = session();
        assert_eq!(session.on_event(KeyEvent::press(LI, 10)), None);
        assert_eq!(session.on_event(KeyEvent::press(RM, 25)), None);
        assert_eq!(session.held(), chord("I-M"));
        assert_eq!(session.pending(), chord("I-M"));
        assert_eq!(session.on_event(KeyEvent::release(LI, 80)), None);
        let outcome = session
            .on_event(KeyEvent::release(RM, 95))
            .expect("stroke completes");
        assert_eq!(outcome.stroke.chord, chord("I-M"));
        assert_eq!(outcome.stroke.duration_ms(), 85);
        assert_eq!(outcome.effect, Effect::Updated);
        assert_eq!(session.engine().buffer().utterance.to_string(), "FIX");
        assert!(session.pending().is_empty());
    }

    #[test]
    fn strike_applies_a_chord_without_key_events() {
        let mut session = session();
        assert_eq!(session.strike(chord("I-I")), Effect::Updated);
        assert_eq!(session.engine().buffer().utterance.to_string(), "MAKE");
        assert!(session.held().is_empty());
    }

    #[test]
    fn release_all_drops_the_stroke_but_keeps_the_buffer() {
        let mut session = session();
        session.strike(chord("I-I"));
        session.on_event(KeyEvent::press(LI, 0));
        session.release_all();
        assert!(session.held().is_empty());
        assert!(session.pending().is_empty());
        assert_eq!(session.on_event(KeyEvent::release(LI, 10)), None);
        assert_eq!(session.engine().buffer().utterance.to_string(), "MAKE");
    }
}
