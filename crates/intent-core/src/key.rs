//! The ten physical keys: one per finger.
//!
//! Keys are named by hand and finger, never by a keyboard letter, so the same
//! layout works on a QWERTY keyboard, a split board, or a wearable.

use std::fmt;

/// Which hand a key belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Hand {
    /// The left hand.
    Left,
    /// The right hand.
    Right,
}

impl Hand {
    /// Both hands, left first.
    pub const ALL: [Hand; 2] = [Hand::Left, Hand::Right];

    /// The opposite hand.
    pub const fn other(self) -> Hand {
        match self {
            Hand::Left => Hand::Right,
            Hand::Right => Hand::Left,
        }
    }
}

/// A finger. The four fingers come first, from index to pinky, then the thumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Finger {
    /// Index finger, written `I`.
    Index,
    /// Middle finger, written `M`.
    Middle,
    /// Ring finger, written `R`.
    Ring,
    /// Pinky finger, written `P`.
    Pinky,
    /// Thumb, written `T`.
    Thumb,
}

impl Finger {
    /// All fingers in index-to-pinky order, thumb last.
    pub const ALL: [Finger; 5] = [
        Finger::Index,
        Finger::Middle,
        Finger::Ring,
        Finger::Pinky,
        Finger::Thumb,
    ];

    /// The four non-thumb fingers in physical order, index to pinky.
    pub const FINGERS: [Finger; 4] = [Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky];

    /// The one-letter name used in chord notation.
    pub const fn letter(self) -> char {
        match self {
            Finger::Index => 'I',
            Finger::Middle => 'M',
            Finger::Ring => 'R',
            Finger::Pinky => 'P',
            Finger::Thumb => 'T',
        }
    }

    /// Parses a one-letter finger name.
    pub const fn from_letter(letter: char) -> Option<Finger> {
        match letter {
            'I' => Some(Finger::Index),
            'M' => Some(Finger::Middle),
            'R' => Some(Finger::Ring),
            'P' => Some(Finger::Pinky),
            'T' => Some(Finger::Thumb),
            _ => None,
        }
    }

    /// Position of this finger inside a [`crate::HandShape`] bit set.
    pub(crate) const fn bit(self) -> u8 {
        match self {
            Finger::Index => 0,
            Finger::Middle => 1,
            Finger::Ring => 2,
            Finger::Pinky => 3,
            Finger::Thumb => 4,
        }
    }
}

/// One of the ten keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    /// The hand that presses this key.
    pub hand: Hand,
    /// The finger that presses this key.
    pub finger: Finger,
}

impl Key {
    /// All ten keys in board order, left pinky to right pinky.
    pub const ALL: [Key; 10] = [
        Key::new(Hand::Left, Finger::Pinky),
        Key::new(Hand::Left, Finger::Ring),
        Key::new(Hand::Left, Finger::Middle),
        Key::new(Hand::Left, Finger::Index),
        Key::new(Hand::Left, Finger::Thumb),
        Key::new(Hand::Right, Finger::Thumb),
        Key::new(Hand::Right, Finger::Index),
        Key::new(Hand::Right, Finger::Middle),
        Key::new(Hand::Right, Finger::Ring),
        Key::new(Hand::Right, Finger::Pinky),
    ];

    /// Creates a key from its hand and finger.
    pub const fn new(hand: Hand, finger: Finger) -> Key {
        Key { hand, finger }
    }

    /// Position of this key in board order: 0 is the left pinky, 9 the right pinky.
    pub const fn index(self) -> u8 {
        match (self.hand, self.finger) {
            (Hand::Left, Finger::Pinky) => 0,
            (Hand::Left, Finger::Ring) => 1,
            (Hand::Left, Finger::Middle) => 2,
            (Hand::Left, Finger::Index) => 3,
            (Hand::Left, Finger::Thumb) => 4,
            (Hand::Right, Finger::Thumb) => 5,
            (Hand::Right, Finger::Index) => 6,
            (Hand::Right, Finger::Middle) => 7,
            (Hand::Right, Finger::Ring) => 8,
            (Hand::Right, Finger::Pinky) => 9,
        }
    }

    /// The key at a board position, if the position is in `0..10`.
    pub fn from_index(index: u8) -> Option<Key> {
        Key::ALL.get(usize::from(index)).copied()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hand = match self.hand {
            Hand::Left => 'L',
            Hand::Right => 'R',
        };
        write!(f, "{hand}{}", self.finger.letter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_order_matches_index() {
        for (position, key) in Key::ALL.iter().enumerate() {
            assert_eq!(usize::from(key.index()), position);
            assert_eq!(Key::from_index(key.index()), Some(*key));
        }
        assert_eq!(Key::from_index(10), None);
    }

    #[test]
    fn finger_letters_round_trip() {
        for finger in Finger::ALL {
            assert_eq!(Finger::from_letter(finger.letter()), Some(finger));
        }
        assert_eq!(Finger::from_letter('X'), None);
        assert_eq!(Finger::from_letter('i'), None);
    }

    #[test]
    fn keys_display_as_hand_then_finger() {
        assert_eq!(Key::new(Hand::Left, Finger::Index).to_string(), "LI");
        assert_eq!(Key::new(Hand::Right, Finger::Thumb).to_string(), "RT");
    }

    #[test]
    fn other_hand_flips() {
        assert_eq!(Hand::Left.other(), Hand::Right);
        assert_eq!(Hand::Right.other(), Hand::Left);
    }
}
