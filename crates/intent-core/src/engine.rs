//! The engine: strokes go in, an utterance comes out.
//!
//! The engine keeps a log of every stroke that changed the buffer and derives
//! the buffer from that log. Undo is therefore exact: it removes the last log
//! entry, whatever it was, and the buffer returns to precisely its earlier
//! state. Even a clear can be taken back.

use std::fmt;

use crate::action::{Action, Command, Concept};
use crate::chord::Chord;
use crate::layout::Layout;

/// One piece of an utterance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    /// A unit of meaning for the decoder to expand.
    Concept(Concept),
    /// Literal text, exactly as spelled.
    Word(String),
}

/// What the user has entered and wants turned into text.
///
/// Its text form is the wire format handed to a decoder: concepts are bare
/// uppercase tokens and literal words are quoted, so the two can never be
/// confused. For example: `FIX BUG IN "login"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Utterance {
    /// The pieces, in the order they were entered.
    pub items: Vec<Item>,
}

impl Utterance {
    /// Whether nothing has been entered.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Each distinct concept that carries a gloss, in order of first use.
    ///
    /// A decoder prompt includes these so the model knows what clause-sized
    /// concepts such as `WITH_TESTS` stand for.
    pub fn glosses(&self) -> Vec<(&str, &str)> {
        let mut found: Vec<(&str, &str)> = Vec::new();
        for item in &self.items {
            if let Item::Concept(Concept {
                token,
                gloss: Some(gloss),
            }) = item
                && !found.iter().any(|(seen, _)| seen == token)
            {
                found.push((token, gloss));
            }
        }
        found
    }
}

impl fmt::Display for Utterance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, item) in self.items.iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            match item {
                Item::Concept(concept) => f.write_str(&concept.token)?,
                Item::Word(word) => {
                    f.write_str("\"")?;
                    for character in word.chars() {
                        if character == '"' || character == '\\' {
                            f.write_str("\\")?;
                        }
                        write!(f, "{character}")?;
                    }
                    f.write_str("\"")?;
                }
            }
        }
        Ok(())
    }
}

/// The buffer as it stands, for showing to the user.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Buffer {
    /// What has been entered so far.
    pub utterance: Utterance,
    /// Whether the last item is a word still being spelled.
    pub word_open: bool,
}

/// What the host should do after a stroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// The buffer changed. Redraw it.
    Updated,
    /// The stroke was understood but had nothing to act on, such as an undo
    /// with an empty buffer.
    Ignored,
    /// Hand this utterance to the decoder. The buffer is now empty.
    Send(Utterance),
    /// Pass this key on to the focused application.
    Key(Command),
    /// The layout gives this chord no meaning.
    Unmapped(Chord),
}

/// One stroke that changed the buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Concept(Concept),
    Text(String),
    Space,
    Clear,
}

/// Interprets strokes against a layout and keeps the buffer.
#[derive(Debug, Clone)]
pub struct Engine {
    layout: Layout,
    log: Vec<Op>,
}

impl Engine {
    /// An engine with an empty buffer.
    pub fn new(layout: Layout) -> Engine {
        Engine {
            layout,
            log: Vec::new(),
        }
    }

    /// The layout in use.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Applies one stroke and says what the host should do about it.
    pub fn stroke(&mut self, chord: Chord) -> Effect {
        let Some(entry) = self.layout.lookup(chord) else {
            return Effect::Unmapped(chord);
        };
        match &entry.action {
            Action::Concept(concept) => self.record(Op::Concept(concept.clone())),
            Action::Text(text) => self.record(Op::Text(text.clone())),
            Action::Command(command) => self.command(*command),
        }
    }

    fn command(&mut self, command: Command) -> Effect {
        match command {
            Command::Space if self.word_open() => self.record(Op::Space),
            Command::Clear if !self.is_empty() => self.record(Op::Clear),
            Command::Space | Command::Clear => Effect::Ignored,
            Command::Undo => {
                if self.log.pop().is_some() {
                    Effect::Updated
                } else {
                    Effect::Ignored
                }
            }
            Command::Send => {
                let utterance = self.buffer().utterance;
                if utterance.is_empty() {
                    return Effect::Ignored;
                }
                self.log.clear();
                Effect::Send(utterance)
            }
            key => Effect::Key(key),
        }
    }

    fn record(&mut self, op: Op) -> Effect {
        self.log.push(op);
        Effect::Updated
    }

    fn word_open(&self) -> bool {
        matches!(self.log.last(), Some(Op::Text(_)))
    }

    /// Whether the buffer holds nothing.
    pub fn is_empty(&self) -> bool {
        !self
            .log
            .iter()
            .rev()
            .take_while(|op| **op != Op::Clear)
            .any(|op| matches!(op, Op::Concept(_) | Op::Text(_)))
    }

    /// The buffer as it stands.
    pub fn buffer(&self) -> Buffer {
        let mut items: Vec<Item> = Vec::new();
        let mut word_open = false;
        for op in &self.log {
            match op {
                Op::Concept(concept) => {
                    items.push(Item::Concept(concept.clone()));
                    word_open = false;
                }
                Op::Text(text) => {
                    match items.last_mut() {
                        Some(Item::Word(word)) if word_open => word.push_str(text),
                        _ => items.push(Item::Word(text.clone())),
                    }
                    word_open = true;
                }
                Op::Space => word_open = false,
                Op::Clear => {
                    items.clear();
                    word_open = false;
                }
            }
        }
        Buffer {
            utterance: Utterance { items },
            word_open,
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const V0: &str = include_str!("../../../layouts/intent-v0.toml");

    fn layout() -> Layout {
        Layout::from_toml(V0).expect("intent-v0 compiles").layout
    }

    fn engine() -> Engine {
        Engine::new(layout())
    }

    fn chord(text: &str) -> Chord {
        text.parse().expect("valid chord")
    }

    /// Strokes written as a token (`FIX`), a command (`<send>`) or text (`'a'`).
    fn play(engine: &mut Engine, strokes: &[&str]) -> Vec<Effect> {
        strokes
            .iter()
            .map(|stroke| {
                let layout = engine.layout();
                let chord = if let Some(name) = stroke
                    .strip_prefix('<')
                    .and_then(|rest| rest.strip_suffix('>'))
                {
                    let command = Command::from_name(name).expect("known command");
                    layout.chords_for_command(command)[0]
                } else if let Some(text) = stroke
                    .strip_prefix('\'')
                    .and_then(|rest| rest.strip_suffix('\''))
                {
                    layout.chords_for_text(text)[0]
                } else {
                    layout.chord_for_token(stroke).expect("known token")
                };
                engine.stroke(chord)
            })
            .collect()
    }

    fn wire(engine: &Engine) -> String {
        engine.buffer().utterance.to_string()
    }

    #[test]
    fn concepts_build_an_utterance() {
        let mut engine = engine();
        assert!(engine.is_empty());
        let effects = play(&mut engine, &["FIX", "THIS", "BUG"]);
        assert_eq!(effects, [Effect::Updated, Effect::Updated, Effect::Updated]);
        assert_eq!(wire(&engine), "FIX THIS BUG");
        assert!(!engine.is_empty());
        assert!(!engine.buffer().word_open);
    }

    #[test]
    fn letters_join_into_one_word_until_a_space() {
        let mut engine = engine();
        play(&mut engine, &["'l'", "'o'", "'g'"]);
        assert_eq!(wire(&engine), "\"log\"");
        assert!(engine.buffer().word_open);
        play(&mut engine, &["<space>", "'i'", "'n'"]);
        assert_eq!(wire(&engine), "\"log\" \"in\"");
    }

    #[test]
    fn a_concept_ends_the_word_being_spelled() {
        let mut engine = engine();
        play(
            &mut engine,
            &["RENAME", "'f'", "'o'", "'o'", "TO", "'b'", "'a'", "'r'"],
        );
        assert_eq!(wire(&engine), "RENAME \"foo\" TO \"bar\"");
    }

    #[test]
    fn capitals_and_symbols_are_kept_exactly() {
        let mut engine = engine();
        play(&mut engine, &["'A'", "'p'", "'i'", "'_'", "'1'"]);
        assert_eq!(wire(&engine), "\"Api_1\"");
    }

    #[test]
    fn space_without_an_open_word_is_ignored() {
        let mut engine = engine();
        assert_eq!(play(&mut engine, &["<space>"]), [Effect::Ignored]);
        play(&mut engine, &["FIX"]);
        assert_eq!(play(&mut engine, &["<space>"]), [Effect::Ignored]);
        play(&mut engine, &["'a'", "<space>"]);
        assert_eq!(play(&mut engine, &["<space>"]), [Effect::Ignored]);
        assert_eq!(wire(&engine), "FIX \"a\"");
    }

    #[test]
    fn undo_takes_back_exactly_one_stroke() {
        let mut engine = engine();
        play(&mut engine, &["FIX", "'a'", "'b'"]);
        assert_eq!(play(&mut engine, &["<undo>"]), [Effect::Updated]);
        assert_eq!(wire(&engine), "FIX \"a\"");
        assert!(engine.buffer().word_open);
        play(&mut engine, &["<undo>"]);
        assert_eq!(wire(&engine), "FIX");
        play(&mut engine, &["<undo>"]);
        assert_eq!(wire(&engine), "");
        assert!(engine.is_empty());
        assert_eq!(play(&mut engine, &["<undo>"]), [Effect::Ignored]);
    }

    #[test]
    fn undoing_a_space_reopens_the_word() {
        let mut engine = engine();
        play(&mut engine, &["'a'", "<space>", "<undo>", "'b'"]);
        assert_eq!(wire(&engine), "\"ab\"");
    }

    #[test]
    fn undoing_a_concept_reopens_the_word_before_it() {
        let mut engine = engine();
        play(&mut engine, &["'a'", "FIX", "<undo>", "'b'"]);
        assert_eq!(wire(&engine), "\"ab\"");
    }

    #[test]
    fn send_hands_over_the_utterance_and_empties_the_buffer() {
        let mut engine = engine();
        play(&mut engine, &["ADD", "TEST", "WITH_TESTS"]);
        let effects = play(&mut engine, &["<send>"]);
        let [Effect::Send(utterance)] = effects.as_slice() else {
            panic!("expected a send, got {effects:?}");
        };
        assert_eq!(utterance.to_string(), "ADD TEST WITH_TESTS");
        assert!(engine.is_empty());
        assert_eq!(play(&mut engine, &["<send>"]), [Effect::Ignored]);
        assert_eq!(play(&mut engine, &["<undo>"]), [Effect::Ignored]);
    }

    #[test]
    fn clear_empties_the_buffer_and_can_be_undone() {
        let mut engine = engine();
        assert_eq!(play(&mut engine, &["<clear>"]), [Effect::Ignored]);
        play(&mut engine, &["FIX", "'a'"]);
        assert_eq!(play(&mut engine, &["<clear>"]), [Effect::Updated]);
        assert!(engine.is_empty());
        assert_eq!(wire(&engine), "");
        assert_eq!(play(&mut engine, &["<clear>"]), [Effect::Ignored]);
        assert_eq!(play(&mut engine, &["<send>"]), [Effect::Ignored]);
        play(&mut engine, &["<undo>"]);
        assert_eq!(wire(&engine), "FIX \"a\"");
        assert!(engine.buffer().word_open);
    }

    #[test]
    fn typing_after_a_clear_starts_fresh() {
        let mut engine = engine();
        play(&mut engine, &["'a'", "<clear>", "'b'"]);
        assert_eq!(wire(&engine), "\"b\"");
    }

    #[test]
    fn keys_pass_through_without_touching_the_buffer() {
        let mut engine = engine();
        play(&mut engine, &["FIX"]);
        assert_eq!(
            play(&mut engine, &["<left>", "<enter>"]),
            [Effect::Key(Command::Left), Effect::Key(Command::Enter)]
        );
        assert_eq!(wire(&engine), "FIX");
    }

    #[test]
    fn unmapped_chords_are_reported() {
        let mut engine = engine();
        let all_ten = Chord::ALL;
        assert_eq!(engine.stroke(all_ten), Effect::Unmapped(all_ten));
        assert!(engine.is_empty());
        assert_eq!(engine.layout().meta().name, "intent-v0");
    }

    #[test]
    fn wire_format_escapes_quotes_and_backslashes() {
        let utterance = Utterance {
            items: vec![Item::Word("a\"b\\c".into())],
        };
        assert_eq!(utterance.to_string(), r#""a\"b\\c""#);
        let mut engine = engine();
        engine.stroke(chord("MT-MP")); // the " key in the brackets family
        assert_eq!(wire(&engine), r#""\"""#);
    }

    #[test]
    fn glosses_list_each_explained_concept_once() {
        let mut engine = engine();
        play(&mut engine, &["FIX", "WITH_TESTS", "BRIEF", "WITH_TESTS"]);
        let utterance = engine.buffer().utterance;
        assert_eq!(
            utterance.glosses(),
            [
                ("WITH_TESTS", "add or update tests that cover this"),
                ("BRIEF", "keep the answer short"),
            ]
        );
        assert_eq!(Utterance::default().glosses(), [] as [(&str, &str); 0]);
        assert!(Utterance::default().is_empty());
    }

    proptest! {
        /// Whatever came before, undoing a stroke that changed the buffer
        /// restores the buffer exactly.
        #[test]
        fn undo_is_an_exact_inverse(picks in prop::collection::vec(any::<prop::sample::Index>(), 0..80)) {
            let layout = layout();
            let chords: Vec<Chord> = layout.entries().iter().map(|entry| entry.chord).collect();
            let undo = layout.chords_for_command(Command::Undo)[0];
            let mut engine = Engine::new(layout);
            for pick in picks {
                let chord = chords[pick.index(chords.len())];
                let before = engine.buffer();
                let was_empty = engine.is_empty();
                match engine.stroke(chord) {
                    Effect::Updated if chord != undo => {
                        prop_assert_eq!(engine.stroke(undo), Effect::Updated);
                        prop_assert_eq!(engine.buffer(), before);
                        prop_assert_eq!(engine.is_empty(), was_empty);
                        engine.stroke(chord);
                    }
                    Effect::Ignored | Effect::Key(_) => prop_assert_eq!(engine.buffer(), before),
                    Effect::Send(utterance) => {
                        prop_assert_eq!(utterance, before.utterance);
                        prop_assert!(engine.is_empty());
                    }
                    Effect::Updated => {}
                    Effect::Unmapped(_) => prop_assert!(false, "picked chords are all mapped"),
                }
                prop_assert_eq!(engine.is_empty(), engine.buffer().utterance.is_empty());
            }
        }
    }
}
