//! A small, explicit model of how hard a chord is to press.
//!
//! The numbers are a starting hypothesis, not measurements. They encode three
//! well-established facts about hands: fingers differ in strength and
//! dexterity, lifting a finger between two pressed ones is awkward, and more
//! keys take more coordination. The trainer will record real timings and error
//! rates so these weights can be re-fitted from data.
//!
//! The model also justifies two fixed tables, [`FINGER_SHAPE_ORDER`] and
//! [`SPELL_SHAPE_ORDER`]. Layout files list tokens by priority and those
//! tables decide which shape each position gets, so cheap chords always go to
//! the tokens listed first. The tables are part of the layout format: changing
//! them moves every chord, so they are fixed constants and a test only checks
//! that they still agree with the default model.

use crate::chord::{Chord, HandShape};
use crate::key::{Finger, Hand};

/// The fifteen finger shapes of one hand (no thumb), easiest first.
pub const FINGER_SHAPE_ORDER: [&str; 15] = [
    "I", "M", "R", "P", "IM", "MR", "IR", "RP", "MP", "IMR", "IP", "MRP", "IMP", "IRP", "IMRP",
];

/// The thirty spelling shapes of one hand (fingers, optionally with that
/// hand's thumb), easiest first.
pub const SPELL_SHAPE_ORDER: [&str; 30] = [
    "I", "M", "R", "P", "TI", "TM", "IM", "TR", "TP", "MR", "TIM", "IR", "RP", "MP", "TMR", "IMR",
    "IP", "TIR", "TRP", "TMP", "MRP", "IMP", "TIMR", "TIP", "IRP", "TMRP", "TIMP", "IMRP", "TIRP",
    "TIMRP",
];

/// [`FINGER_SHAPE_ORDER`] as parsed shapes.
pub fn finger_shape_order() -> [HandShape; 15] {
    FINGER_SHAPE_ORDER.map(|text| text.parse().expect("table holds valid shapes"))
}

/// [`SPELL_SHAPE_ORDER`] as parsed shapes.
pub fn spell_shape_order() -> [HandShape; 30] {
    SPELL_SHAPE_ORDER.map(|text| text.parse().expect("table holds valid shapes"))
}

/// Weights for estimating the effort of a chord. Higher is harder.
#[derive(Debug, Clone, PartialEq)]
pub struct EffortModel {
    /// Cost of pressing each finger alone: index, middle, ring, pinky, thumb.
    pub finger: [f64; 5],
    /// Cost of each key after the first on the same hand.
    pub extra_key: f64,
    /// Cost of each lifted finger lying between two pressed ones.
    pub gap: f64,
    /// Cost of involving both hands in one chord.
    pub both_hands: f64,
}

impl Default for EffortModel {
    fn default() -> Self {
        EffortModel {
            finger: [1.0, 1.1, 1.6, 1.8, 0.8],
            extra_key: 0.2,
            gap: 0.7,
            both_hands: 0.3,
        }
    }
}

impl EffortModel {
    /// Effort of one hand's part of a chord.
    pub fn hand(&self, shape: HandShape) -> f64 {
        if shape.is_empty() {
            return 0.0;
        }
        let fingers: f64 = shape.iter().map(|finger| self.finger_cost(finger)).sum();
        fingers + self.extra_key * f64::from(shape.len() - 1) + self.gap * f64::from(shape.gaps())
    }

    /// Effort of a whole chord.
    pub fn chord(&self, chord: Chord) -> f64 {
        let left = chord.hand(Hand::Left);
        let right = chord.hand(Hand::Right);
        let both = if left.is_empty() || right.is_empty() {
            0.0
        } else {
            self.both_hands
        };
        self.hand(left) + self.hand(right) + both
    }

    fn finger_cost(&self, finger: Finger) -> f64 {
        self.finger[usize::from(finger.bit())]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::key::Key;

    const EPSILON: f64 = 1e-9;

    fn chord(text: &str) -> Chord {
        text.parse().expect("valid chord")
    }

    #[test]
    fn known_efforts() {
        let model = EffortModel::default();
        for (text, expected) in [
            ("I-", 1.0),
            ("-I", 1.0),
            ("I-I", 2.3),
            ("MI-", 2.3),
            ("RI-", 3.5),
            ("PI-", 4.4),
            ("IT-", 2.0),
            ("PRMIT-TIMRP", 14.5),
        ] {
            let effort = model.chord(chord(text));
            assert!(
                (effort - expected).abs() < EPSILON,
                "{text}: got {effort}, expected {expected}"
            );
        }
        assert!(model.chord(Chord::EMPTY).abs() < EPSILON);
    }

    #[test]
    fn hands_are_symmetric() {
        let model = EffortModel::default();
        for original in Chord::all() {
            let mirrored = Chord::from_hands(original.hand(Hand::Right), original.hand(Hand::Left));
            assert!((model.chord(original) - model.chord(mirrored)).abs() < EPSILON);
        }
    }

    #[test]
    fn adding_a_key_never_makes_a_chord_easier() {
        let model = EffortModel::default();
        for base in Chord::all() {
            for key in Key::ALL {
                if !base.contains(key) {
                    assert!(
                        model.chord(base.with(key)) > model.chord(base),
                        "{base} + {key}"
                    );
                }
            }
        }
    }

    #[test]
    fn finger_table_lists_every_finger_shape_once_easiest_first() {
        let model = EffortModel::default();
        let order = finger_shape_order();
        let unique: HashSet<HandShape> = order.iter().copied().collect();
        assert_eq!(unique.len(), 15);
        assert!(order.iter().all(|s| !s.is_empty() && !s.has_thumb()));
        for pair in order.windows(2) {
            assert!(
                model.hand(pair[0]) <= model.hand(pair[1]) + EPSILON,
                "{} should not come before {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn spell_table_lists_every_spelling_shape_once_easiest_first() {
        let model = EffortModel::default();
        let order = spell_shape_order();
        let unique: HashSet<HandShape> = order.iter().copied().collect();
        assert_eq!(unique.len(), 30);
        assert!(order.iter().all(|s| !s.fingers().is_empty()));
        for pair in order.windows(2) {
            assert!(
                model.hand(pair[0]) <= model.hand(pair[1]) + EPSILON,
                "{} should not come before {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn spell_table_shapes_land_in_the_spelling_plane() {
        use crate::chord::Plane;
        for shape in spell_shape_order() {
            for hand in Hand::ALL {
                assert_eq!(
                    Chord::of_hand(hand, shape).plane(),
                    Some(Plane::Spell(hand))
                );
            }
        }
    }
}
