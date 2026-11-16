//! Hand shapes, chords, and the planes that give the chord space its grammar.
//!
//! A chord is any set of the ten keys pressed together. Chords are written
//! like steno outlines: the left hand in board order (`PRMIT`), a hyphen, then
//! the right hand in board order (`TIMRP`). `I-I` is both index fingers and
//! `PRMIT-TIMRP` is all ten keys.

use std::fmt::{self, Write as _};
use std::str::FromStr;

use crate::key::{Finger, Hand, Key};

/// The keys one hand is holding: four fingers plus that hand's thumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct HandShape(u8);

impl HandShape {
    /// No keys.
    pub const EMPTY: HandShape = HandShape(0);
    /// All five keys of one hand.
    pub const FULL: HandShape = HandShape(0b1_1111);

    const FINGER_MASK: u8 = 0b0_1111;
    const THUMB_MASK: u8 = 0b1_0000;

    /// Builds a shape from raw bits (bit 0 = index … bit 3 = pinky, bit 4 = thumb).
    pub const fn from_bits(bits: u8) -> Option<HandShape> {
        if bits <= Self::FULL.0 {
            Some(HandShape(bits))
        } else {
            None
        }
    }

    /// The raw bits of this shape.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether the given finger is pressed.
    pub const fn contains(self, finger: Finger) -> bool {
        self.0 & (1 << finger.bit()) != 0
    }

    /// This shape with one more finger pressed.
    #[must_use]
    pub const fn with(self, finger: Finger) -> HandShape {
        HandShape(self.0 | (1 << finger.bit()))
    }

    /// This shape without the thumb.
    #[must_use]
    pub const fn fingers(self) -> HandShape {
        HandShape(self.0 & Self::FINGER_MASK)
    }

    /// Whether the thumb is pressed.
    pub const fn has_thumb(self) -> bool {
        self.0 & Self::THUMB_MASK != 0
    }

    /// Whether no key is pressed.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Number of keys pressed.
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// The pressed fingers, index to pinky, thumb last.
    pub fn iter(self) -> impl Iterator<Item = Finger> {
        Finger::ALL
            .into_iter()
            .filter(move |finger| self.contains(*finger))
    }

    /// Number of lifted fingers lying between pressed fingers.
    ///
    /// `IR` has one gap (the middle finger) and `IP` has two. Gaps are what
    /// make a shape awkward: fingers do not move fully independently. The
    /// thumb is ignored.
    pub const fn gaps(self) -> u32 {
        let fingers = self.0 & Self::FINGER_MASK;
        if fingers == 0 {
            return 0;
        }
        let lowest = fingers.trailing_zeros();
        let highest = fingers.ilog2();
        (highest - lowest + 1) - fingers.count_ones()
    }
}

impl fmt::Display for HandShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const ORDER: [Finger; 5] = [
            Finger::Thumb,
            Finger::Index,
            Finger::Middle,
            Finger::Ring,
            Finger::Pinky,
        ];
        for finger in ORDER {
            if self.contains(finger) {
                f.write_char(finger.letter())?;
            }
        }
        Ok(())
    }
}

/// Why a hand shape such as `"IM"` failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseShapeError {
    /// A letter that is not one of `T I M R P`.
    UnknownFinger(char),
    /// The same finger written twice.
    DuplicateFinger(char),
}

impl fmt::Display for ParseShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseShapeError::UnknownFinger(letter) => {
                write!(f, "unknown finger '{letter}' (expected one of T I M R P)")
            }
            ParseShapeError::DuplicateFinger(letter) => {
                write!(f, "finger '{letter}' is written twice")
            }
        }
    }
}

impl std::error::Error for ParseShapeError {}

impl FromStr for HandShape {
    type Err = ParseShapeError;

    /// Parses finger letters in any order. The empty string is the empty shape.
    fn from_str(text: &str) -> Result<HandShape, ParseShapeError> {
        let mut shape = HandShape::EMPTY;
        for letter in text.trim().chars() {
            let finger =
                Finger::from_letter(letter).ok_or(ParseShapeError::UnknownFinger(letter))?;
            if shape.contains(finger) {
                return Err(ParseShapeError::DuplicateFinger(letter));
            }
            shape = shape.with(finger);
        }
        Ok(shape)
    }
}

/// A set of keys pressed together.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Chord(u16);

impl Chord {
    /// No keys.
    pub const EMPTY: Chord = Chord(0);
    /// All ten keys.
    pub const ALL: Chord = Chord(0x3FF);
    /// How many distinct non-empty chords exist.
    pub const COUNT: usize = 1023;

    /// Builds a chord from raw bits, one per key in board order.
    pub const fn from_bits(bits: u16) -> Option<Chord> {
        if bits <= Self::ALL.0 {
            Some(Chord(bits))
        } else {
            None
        }
    }

    /// The raw bits of this chord, one per key in board order.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Builds a chord from the shape of each hand.
    pub fn from_hands(left: HandShape, right: HandShape) -> Chord {
        Chord::of_hand(Hand::Left, left).union(Chord::of_hand(Hand::Right, right))
    }

    /// Builds a chord pressed by one hand only.
    pub fn of_hand(hand: Hand, shape: HandShape) -> Chord {
        shape.iter().fold(Chord::EMPTY, |chord, finger| {
            chord.with(Key::new(hand, finger))
        })
    }

    /// The shape one hand is holding in this chord.
    pub fn hand(self, hand: Hand) -> HandShape {
        Finger::ALL
            .into_iter()
            .filter(|finger| self.contains(Key::new(hand, *finger)))
            .fold(HandShape::EMPTY, HandShape::with)
    }

    /// Whether the given key is part of this chord.
    pub const fn contains(self, key: Key) -> bool {
        self.0 & (1 << key.index()) != 0
    }

    /// This chord with one more key.
    #[must_use]
    pub const fn with(self, key: Key) -> Chord {
        Chord(self.0 | (1 << key.index()))
    }

    /// This chord with one key removed.
    #[must_use]
    pub const fn without(self, key: Key) -> Chord {
        Chord(self.0 & !(1 << key.index()))
    }

    /// All keys of both chords.
    #[must_use]
    pub const fn union(self, other: Chord) -> Chord {
        Chord(self.0 | other.0)
    }

    /// Whether this chord contains every key of `other`.
    pub const fn is_superset_of(self, other: Chord) -> bool {
        self.0 & other.0 == other.0
    }

    /// Number of keys in this chord.
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether this chord has no keys.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The keys of this chord in board order.
    pub fn keys(self) -> impl Iterator<Item = Key> {
        Key::ALL.into_iter().filter(move |key| self.contains(*key))
    }

    /// Every non-empty chord, in ascending bit order.
    pub fn all() -> impl Iterator<Item = Chord> {
        (1..=Self::ALL.0).map(Chord)
    }

    /// A ten-character picture of the chord, such as `...x. .x...` for `I-I`.
    pub fn diagram(self) -> String {
        let mut picture = String::with_capacity(11);
        for key in Key::ALL {
            if key.hand == Hand::Right && key.finger == Finger::Thumb {
                picture.push(' ');
            }
            picture.push(if self.contains(key) { 'x' } else { '.' });
        }
        picture
    }

    /// The plane this chord belongs to, or `None` for the empty chord.
    ///
    /// Planes are decided by structure alone: which hands have fingers down
    /// and which thumbs are held. Every non-empty chord is in exactly one.
    pub fn plane(self) -> Option<Plane> {
        let left = self.hand(Hand::Left);
        let right = self.hand(Hand::Right);
        let left_fingers = !left.fingers().is_empty();
        let right_fingers = !right.fingers().is_empty();
        match (left_fingers, right_fingers) {
            (false, false) => (!self.is_empty()).then_some(Plane::Thumbs),
            (true, false) if right.has_thumb() => Some(Plane::SpellShift(Hand::Left)),
            (true, false) => Some(Plane::Spell(Hand::Left)),
            (false, true) if left.has_thumb() => Some(Plane::SpellShift(Hand::Right)),
            (false, true) => Some(Plane::Spell(Hand::Right)),
            (true, true) => Some(match (left.has_thumb(), right.has_thumb()) {
                (false, false) => Plane::Core,
                (false, true) => Plane::Extended,
                (true, false) => Plane::Literal,
                (true, true) => Plane::User,
            }),
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for key in Key::ALL {
            if key.hand == Hand::Right && key.finger == Finger::Thumb {
                f.write_char('-')?;
            }
            if self.contains(key) {
                f.write_char(key.finger.letter())?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Chord({self})")
    }
}

/// Why a chord such as `"PM-I"` failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseChordError {
    /// The hyphen between the hands is missing.
    MissingHyphen,
    /// More than one hyphen.
    ExtraHyphen,
    /// A letter that is not one of `P R M I T`.
    UnknownKey(char),
    /// The same key written twice on one hand.
    DuplicateKey(Hand, char),
    /// No keys at all.
    Empty,
}

impl fmt::Display for ParseChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseChordError::MissingHyphen => {
                write!(
                    f,
                    "missing '-' between the left and right hand (for example \"I-I\")"
                )
            }
            ParseChordError::ExtraHyphen => write!(f, "more than one '-'"),
            ParseChordError::UnknownKey(letter) => {
                write!(f, "unknown key '{letter}' (expected one of P R M I T)")
            }
            ParseChordError::DuplicateKey(hand, letter) => {
                let hand = match hand {
                    Hand::Left => "left",
                    Hand::Right => "right",
                };
                write!(f, "key '{letter}' is written twice on the {hand} hand")
            }
            ParseChordError::Empty => write!(f, "a chord needs at least one key"),
        }
    }
}

impl std::error::Error for ParseChordError {}

impl FromStr for Chord {
    type Err = ParseChordError;

    fn from_str(text: &str) -> Result<Chord, ParseChordError> {
        let (left, right) = text
            .trim()
            .split_once('-')
            .ok_or(ParseChordError::MissingHyphen)?;
        if right.contains('-') {
            return Err(ParseChordError::ExtraHyphen);
        }
        let mut chord = Chord::EMPTY;
        for (hand, side) in [(Hand::Left, left), (Hand::Right, right)] {
            for letter in side.chars() {
                let finger =
                    Finger::from_letter(letter).ok_or(ParseChordError::UnknownKey(letter))?;
                let key = Key::new(hand, finger);
                if chord.contains(key) {
                    return Err(ParseChordError::DuplicateKey(hand, letter));
                }
                chord = chord.with(key);
            }
        }
        if chord.is_empty() {
            return Err(ParseChordError::Empty);
        }
        Ok(chord)
    }
}

/// A region of the chord space with one job.
///
/// | Plane | Structure | Job |
/// |---|---|---|
/// | `Thumbs` | thumbs only | the most frequent commands |
/// | `Spell` | fingers of one hand, optionally its own thumb | letters |
/// | `SpellShift` | a `Spell` chord plus the opposite thumb | capitals |
/// | `Core` | fingers of both hands | everyday concepts |
/// | `Extended` | both hands plus the right thumb | more concepts |
/// | `Literal` | both hands plus the left thumb | digits, symbols, keys |
/// | `User` | both hands plus both thumbs | personal vocabulary |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Plane {
    /// Thumbs only: three chords.
    Thumbs,
    /// Fingers of one hand, with or without that hand's thumb: thirty chords per hand.
    Spell(Hand),
    /// A spelling chord plus the opposite thumb: thirty chords per hand.
    SpellShift(Hand),
    /// Fingers of both hands, no thumbs: 225 chords.
    Core,
    /// Fingers of both hands plus the right thumb: 225 chords.
    Extended,
    /// Fingers of both hands plus the left thumb: 225 chords.
    Literal,
    /// Fingers of both hands plus both thumbs: 225 chords.
    User,
}

impl Plane {
    /// Every plane.
    pub const ALL: [Plane; 9] = [
        Plane::Thumbs,
        Plane::Spell(Hand::Left),
        Plane::Spell(Hand::Right),
        Plane::SpellShift(Hand::Left),
        Plane::SpellShift(Hand::Right),
        Plane::Core,
        Plane::Extended,
        Plane::Literal,
        Plane::User,
    ];

    /// How many chords this plane holds.
    pub const fn capacity(self) -> usize {
        match self {
            Plane::Thumbs => 3,
            Plane::Spell(_) | Plane::SpellShift(_) => 30,
            Plane::Core | Plane::Extended | Plane::Literal | Plane::User => 225,
        }
    }

    /// For the two-hand planes, which thumbs select it: `(left, right)`.
    pub const fn thumbs(self) -> Option<(bool, bool)> {
        match self {
            Plane::Core => Some((false, false)),
            Plane::Extended => Some((false, true)),
            Plane::Literal => Some((true, false)),
            Plane::User => Some((true, true)),
            Plane::Thumbs | Plane::Spell(_) | Plane::SpellShift(_) => None,
        }
    }

    /// The lowercase name used in layout files and reports.
    pub const fn name(self) -> &'static str {
        match self {
            Plane::Thumbs => "thumbs",
            Plane::Spell(Hand::Left) => "spell-left",
            Plane::Spell(Hand::Right) => "spell-right",
            Plane::SpellShift(Hand::Left) => "shift-left",
            Plane::SpellShift(Hand::Right) => "shift-right",
            Plane::Core => "core",
            Plane::Extended => "extended",
            Plane::Literal => "literal",
            Plane::User => "user",
        }
    }
}

impl fmt::Display for Plane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn chord(text: &str) -> Chord {
        text.parse().expect("valid chord")
    }

    fn shape(text: &str) -> HandShape {
        text.parse().expect("valid shape")
    }

    #[test]
    fn every_chord_round_trips_through_its_notation() {
        let mut count = 0;
        for original in Chord::all() {
            let parsed: Chord = original.to_string().parse().expect("own notation parses");
            assert_eq!(parsed, original);
            count += 1;
        }
        assert_eq!(count, Chord::COUNT);
    }

    #[test]
    fn notation_is_board_order() {
        assert_eq!(Chord::ALL.to_string(), "PRMIT-TIMRP");
        assert_eq!(chord("I-I").to_string(), "I-I");
        assert_eq!(chord("TIM-PT").to_string(), "MIT-TP");
        assert_eq!(chord("-T").to_string(), "-T");
        assert_eq!(chord(" P- ").to_string(), "P-");
    }

    #[test]
    fn diagram_shows_each_key() {
        assert_eq!(chord("I-I").diagram(), "...x. .x...");
        assert_eq!(Chord::ALL.diagram(), "xxxxx xxxxx");
        assert_eq!(chord("P-P").diagram(), "x.... ....x");
    }

    #[test]
    fn bad_chords_explain_themselves() {
        assert_eq!("II".parse::<Chord>(), Err(ParseChordError::MissingHyphen));
        assert_eq!("I-I-".parse::<Chord>(), Err(ParseChordError::ExtraHyphen));
        assert_eq!(
            "X-I".parse::<Chord>(),
            Err(ParseChordError::UnknownKey('X'))
        );
        assert_eq!(
            "i-I".parse::<Chord>(),
            Err(ParseChordError::UnknownKey('i'))
        );
        assert_eq!(
            "I-MM".parse::<Chord>(),
            Err(ParseChordError::DuplicateKey(Hand::Right, 'M'))
        );
        assert_eq!("-".parse::<Chord>(), Err(ParseChordError::Empty));
    }

    #[test]
    fn hands_round_trip() {
        for original in Chord::all() {
            let rebuilt = Chord::from_hands(original.hand(Hand::Left), original.hand(Hand::Right));
            assert_eq!(rebuilt, original);
        }
    }

    #[test]
    fn set_operations() {
        let both = chord("I-I");
        let left = chord("I-");
        assert!(both.is_superset_of(left));
        assert!(!left.is_superset_of(both));
        assert!(both.is_superset_of(Chord::EMPTY));
        assert_eq!(both.without(Key::new(Hand::Right, Finger::Index)), left);
        assert_eq!(left.union(chord("-I")), both);
        assert_eq!(both.len(), 2);
        assert_eq!(both.keys().count(), 2);
        assert!(Chord::EMPTY.is_empty());
        assert_eq!(Chord::from_bits(0x400), None);
    }

    #[test]
    fn shapes_parse_in_any_order_and_print_thumb_first() {
        assert_eq!(shape("MI").to_string(), "IM");
        assert_eq!(shape("PIT").to_string(), "TIP");
        assert_eq!(shape(""), HandShape::EMPTY);
        assert_eq!(
            "IX".parse::<HandShape>(),
            Err(ParseShapeError::UnknownFinger('X'))
        );
        assert_eq!(
            "II".parse::<HandShape>(),
            Err(ParseShapeError::DuplicateFinger('I'))
        );
        assert_eq!(HandShape::from_bits(0b10_0000), None);
    }

    #[test]
    fn gaps_count_lifted_fingers_between_pressed_ones() {
        for (text, gaps) in [
            ("", 0),
            ("I", 0),
            ("IM", 0),
            ("IR", 1),
            ("MP", 1),
            ("IP", 2),
            ("IMP", 1),
            ("IMRP", 0),
            ("T", 0),
            ("TIP", 2),
        ] {
            assert_eq!(shape(text).gaps(), gaps, "shape {text:?}");
        }
    }

    #[test]
    fn planes_partition_the_whole_chord_space() {
        let mut sizes: HashMap<Plane, usize> = HashMap::new();
        for chord in Chord::all() {
            let plane = chord.plane().expect("non-empty chords have a plane");
            *sizes.entry(plane).or_default() += 1;
        }
        for plane in Plane::ALL {
            assert_eq!(sizes.get(&plane), Some(&plane.capacity()), "plane {plane}");
        }
        assert_eq!(sizes.values().sum::<usize>(), Chord::COUNT);
        assert_eq!(Chord::EMPTY.plane(), None);
    }

    #[test]
    fn plane_examples() {
        for (text, plane) in [
            ("T-", Plane::Thumbs),
            ("-T", Plane::Thumbs),
            ("T-T", Plane::Thumbs),
            ("I-", Plane::Spell(Hand::Left)),
            ("IT-", Plane::Spell(Hand::Left)),
            ("-TI", Plane::Spell(Hand::Right)),
            ("I-T", Plane::SpellShift(Hand::Left)),
            ("IT-T", Plane::SpellShift(Hand::Left)),
            ("T-I", Plane::SpellShift(Hand::Right)),
            ("I-I", Plane::Core),
            ("I-TI", Plane::Extended),
            ("IT-I", Plane::Literal),
            ("IT-TI", Plane::User),
            ("PRMIT-TIMRP", Plane::User),
        ] {
            assert_eq!(chord(text).plane(), Some(plane), "chord {text}");
        }
    }

    #[test]
    fn two_hand_planes_know_their_thumbs() {
        for plane in [Plane::Core, Plane::Extended, Plane::Literal, Plane::User] {
            let (left_thumb, right_thumb) = plane.thumbs().expect("two-hand plane");
            let mut left = shape("I");
            let mut right = shape("I");
            if left_thumb {
                left = left.with(Finger::Thumb);
            }
            if right_thumb {
                right = right.with(Finger::Thumb);
            }
            assert_eq!(Chord::from_hands(left, right).plane(), Some(plane));
        }
        assert_eq!(Plane::Thumbs.thumbs(), None);
    }
}
