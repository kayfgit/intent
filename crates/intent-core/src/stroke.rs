//! Turning raw key events into strokes.
//!
//! A stroke starts with the first key going down and ends when every key is
//! back up. Its chord is every key that was down at any point in between, so
//! the keys of a chord do not have to land at the same instant. This is the
//! rule steno machines use.
//!
//! The detector has no clock: the host stamps each event with a time in
//! milliseconds, which keeps the logic deterministic and lets the trainer
//! measure how long each stroke took.

use crate::chord::Chord;
use crate::key::Key;

/// Whether a key went down or came up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    /// The key went down.
    Press,
    /// The key came up.
    Release,
}

/// One key going down or coming up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// The key that changed.
    pub key: Key,
    /// What happened to it.
    pub action: KeyAction,
    /// When it happened, in milliseconds on the host's clock.
    pub at_ms: u64,
}

impl KeyEvent {
    /// A key going down.
    pub const fn press(key: Key, at_ms: u64) -> KeyEvent {
        KeyEvent {
            key,
            action: KeyAction::Press,
            at_ms,
        }
    }

    /// A key coming up.
    pub const fn release(key: Key, at_ms: u64) -> KeyEvent {
        KeyEvent {
            key,
            action: KeyAction::Release,
            at_ms,
        }
    }
}

/// A completed chord and when it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stroke {
    /// Every key that was down during the stroke.
    pub chord: Chord,
    /// When the first key went down.
    pub started_ms: u64,
    /// When the last key came up.
    pub ended_ms: u64,
}

impl Stroke {
    /// How long the stroke took from first press to last release.
    pub const fn duration_ms(&self) -> u64 {
        self.ended_ms.saturating_sub(self.started_ms)
    }
}

/// Collects key events into strokes.
#[derive(Debug, Clone, Default)]
pub struct StrokeDetector {
    held: Chord,
    pending: Chord,
    started_ms: u64,
}

impl StrokeDetector {
    /// A detector with no keys down.
    pub fn new() -> StrokeDetector {
        StrokeDetector::default()
    }

    /// The keys that are down right now.
    pub const fn held(&self) -> Chord {
        self.held
    }

    /// The chord built so far in the stroke in progress.
    ///
    /// Hosts use this to show live hints while keys are held.
    pub const fn pending(&self) -> Chord {
        self.pending
    }

    /// Feeds one event. Returns a stroke when the last key comes up.
    ///
    /// Repeated presses of a held key (keyboard auto-repeat) and releases of
    /// keys the detector never saw go down are ignored.
    pub fn on_event(&mut self, event: KeyEvent) -> Option<Stroke> {
        match event.action {
            KeyAction::Press => {
                if self.held.contains(event.key) {
                    return None;
                }
                if self.pending.is_empty() {
                    self.started_ms = event.at_ms;
                }
                self.held = self.held.with(event.key);
                self.pending = self.pending.with(event.key);
                None
            }
            KeyAction::Release => {
                if !self.held.contains(event.key) {
                    return None;
                }
                self.held = self.held.without(event.key);
                if !self.held.is_empty() {
                    return None;
                }
                let stroke = Stroke {
                    chord: self.pending,
                    started_ms: self.started_ms,
                    ended_ms: event.at_ms,
                };
                self.pending = Chord::EMPTY;
                Some(stroke)
            }
        }
    }

    /// Forgets all held keys and any stroke in progress.
    ///
    /// Hosts call this when input is switched off mid-stroke.
    pub fn reset(&mut self) {
        *self = StrokeDetector::default();
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::key::{Finger, Hand};

    const LI: Key = Key::new(Hand::Left, Finger::Index);
    const RI: Key = Key::new(Hand::Right, Finger::Index);
    const RM: Key = Key::new(Hand::Right, Finger::Middle);

    fn chord(text: &str) -> Chord {
        text.parse().expect("valid chord")
    }

    #[test]
    fn single_key_tap_is_a_stroke() {
        let mut detector = StrokeDetector::new();
        assert_eq!(detector.on_event(KeyEvent::press(LI, 100)), None);
        assert_eq!(detector.held(), chord("I-"));
        let stroke = detector
            .on_event(KeyEvent::release(LI, 180))
            .expect("stroke on release");
        assert_eq!(stroke.chord, chord("I-"));
        assert_eq!(stroke.started_ms, 100);
        assert_eq!(stroke.ended_ms, 180);
        assert_eq!(stroke.duration_ms(), 80);
        assert!(detector.held().is_empty());
        assert!(detector.pending().is_empty());
    }

    #[test]
    fn keys_do_not_have_to_land_together() {
        let mut detector = StrokeDetector::new();
        assert_eq!(detector.on_event(KeyEvent::press(LI, 0)), None);
        assert_eq!(detector.on_event(KeyEvent::press(RI, 30)), None);
        assert_eq!(detector.pending(), chord("I-I"));
        assert_eq!(detector.on_event(KeyEvent::release(LI, 60)), None);
        let stroke = detector
            .on_event(KeyEvent::release(RI, 90))
            .expect("stroke");
        assert_eq!(stroke.chord, chord("I-I"));
        assert_eq!(stroke.duration_ms(), 90);
    }

    #[test]
    fn a_key_released_and_another_pressed_before_full_release_joins_the_stroke() {
        let mut detector = StrokeDetector::new();
        detector.on_event(KeyEvent::press(LI, 0));
        detector.on_event(KeyEvent::press(RI, 10));
        detector.on_event(KeyEvent::release(RI, 20));
        assert_eq!(detector.held(), chord("I-"));
        detector.on_event(KeyEvent::press(RM, 30));
        detector.on_event(KeyEvent::release(RM, 40));
        let stroke = detector
            .on_event(KeyEvent::release(LI, 50))
            .expect("stroke");
        assert_eq!(stroke.chord, chord("I-IM"));
    }

    #[test]
    fn auto_repeat_is_ignored() {
        let mut detector = StrokeDetector::new();
        detector.on_event(KeyEvent::press(LI, 0));
        assert_eq!(detector.on_event(KeyEvent::press(LI, 500)), None);
        assert_eq!(detector.on_event(KeyEvent::press(LI, 530)), None);
        let stroke = detector
            .on_event(KeyEvent::release(LI, 600))
            .expect("stroke");
        assert_eq!(stroke.chord, chord("I-"));
        assert_eq!(stroke.started_ms, 0);
    }

    #[test]
    fn stray_release_is_ignored() {
        let mut detector = StrokeDetector::new();
        assert_eq!(detector.on_event(KeyEvent::release(LI, 0)), None);
        detector.on_event(KeyEvent::press(RI, 10));
        assert_eq!(detector.on_event(KeyEvent::release(LI, 20)), None);
        let stroke = detector
            .on_event(KeyEvent::release(RI, 30))
            .expect("stroke");
        assert_eq!(stroke.chord, chord("-I"));
    }

    #[test]
    fn reset_drops_the_stroke_in_progress() {
        let mut detector = StrokeDetector::new();
        detector.on_event(KeyEvent::press(LI, 0));
        detector.reset();
        assert!(detector.held().is_empty());
        assert_eq!(detector.on_event(KeyEvent::release(LI, 10)), None);
    }

    #[test]
    fn time_going_backwards_does_not_panic() {
        let mut detector = StrokeDetector::new();
        detector.on_event(KeyEvent::press(LI, 100));
        let stroke = detector
            .on_event(KeyEvent::release(LI, 50))
            .expect("stroke");
        assert_eq!(stroke.duration_ms(), 0);
    }

    fn any_event() -> impl Strategy<Value = (u8, bool)> {
        (0u8..10, any::<bool>())
    }

    proptest! {
        /// Checks the detector against a tiny reference model for any event order.
        #[test]
        fn matches_reference_model(events in prop::collection::vec(any_event(), 0..200)) {
            let mut detector = StrokeDetector::new();
            let mut down = Chord::EMPTY;
            let mut seen = Chord::EMPTY;
            for (time, (index, is_press)) in (0u64..).zip(events) {
                let key = Key::from_index(index).expect("index below ten");
                let event = if is_press {
                    KeyEvent::press(key, time)
                } else {
                    KeyEvent::release(key, time)
                };
                let was_down = down.contains(key);
                let expected = if is_press {
                    down = down.with(key);
                    seen = seen.with(key);
                    None
                } else if was_down {
                    down = down.without(key);
                    if down.is_empty() {
                        let finished = seen;
                        seen = Chord::EMPTY;
                        Some(finished)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let stroke = detector.on_event(event);
                prop_assert_eq!(stroke.map(|s| s.chord), expected);
                prop_assert_eq!(detector.held(), down);
                prop_assert_eq!(detector.pending(), seen);
                if let Some(stroke) = stroke {
                    prop_assert!(!stroke.chord.is_empty());
                    prop_assert!(stroke.started_ms <= stroke.ended_ms);
                }
            }
        }
    }
}
