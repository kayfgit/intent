//! Plain data the web page receives, and the session that produces it.
//!
//! Everything here is ordinary Rust with no browser types, so it is tested
//! natively. The page gets these values as JSON; the field names are the
//! contract with the TypeScript side.

use intent_core::effort::{
    FINGER_SHAPE_ORDER, SPELL_SHAPE_ORDER, finger_shape_order, spell_shape_order,
};
use intent_core::layout::{Entry, Kind};
use intent_core::prompt::{self, PromptOptions, Turn};
use intent_core::{
    Action, Chord, Effect, Finger, Hand, HandShape, Item, Key, KeyEvent, Layout, Outcome, Plane,
    Session, Utterance,
};
use serde::{Deserialize, Serialize};

/// A chord and what it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryView {
    /// The chord in `PRMIT-TIMRP` notation.
    pub chord: String,
    /// The chord as key bits, bit 0 being the left pinky.
    pub bits: u16,
    /// `concept`, `text` or `command`.
    pub kind: &'static str,
    /// The token, the characters typed, or the command name.
    pub label: String,
    /// What a clause-sized concept stands for.
    pub gloss: Option<String>,
}

impl EntryView {
    fn of(entry: &Entry) -> EntryView {
        let (kind, label, gloss) = match &entry.action {
            Action::Concept(concept) => ("concept", concept.token.clone(), concept.gloss.clone()),
            Action::Text(text) => ("text", text.clone(), None),
            Action::Command(command) => ("command", command.name().to_owned(), None),
        };
        EntryView {
            chord: entry.chord.to_string(),
            bits: entry.chord.bits(),
            kind,
            label,
            gloss,
        }
    }
}

/// One member of a family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberView {
    /// The right-hand finger shape that selects it, such as `IM`.
    pub right: String,
    /// The full chord and its action.
    #[serde(flatten)]
    pub entry: EntryView,
}

/// A family of two-hand chords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FamilyView {
    /// Short identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// `core`, `extended`, `literal` or `user`.
    pub plane: &'static str,
    /// `concept`, `text` or `command`.
    pub kind: &'static str,
    /// The left-hand finger shape that selects the family.
    pub left: String,
    /// Its members, easiest shape first.
    pub members: Vec<MemberView>,
}

/// One spelling shape. Both hands type the same character with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpellView {
    /// The one-hand shape, such as `TI`.
    pub shape: String,
    /// The character it types.
    pub text: String,
    /// The character it types with the opposite thumb held.
    pub shifted: Option<String>,
}

/// Everything static about a layout, for reference panels and hints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayoutView {
    /// The layout's name.
    pub name: String,
    /// The layout's version.
    pub version: String,
    /// The layout's one-line description.
    pub description: String,
    /// The fifteen finger shapes, easiest first.
    pub finger_shapes: Vec<&'static str>,
    /// The thirty spelling shapes, easiest first.
    pub spell_shapes: Vec<&'static str>,
    /// The thumb-only chords.
    pub thumbs: Vec<EntryView>,
    /// The spelling map.
    pub spelling: Vec<SpellView>,
    /// Every family, in file order.
    pub families: Vec<FamilyView>,
}

/// The stroke in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingView {
    /// Key bits collected so far.
    pub bits: u16,
    /// The same as notation, or an empty string when nothing is held.
    pub chord: String,
    /// Left-hand finger shape, without the thumb.
    pub left: String,
    /// Right-hand finger shape, without the thumb.
    pub right: String,
    /// Whether the left thumb is part of the stroke.
    pub left_thumb: bool,
    /// Whether the right thumb is part of the stroke.
    pub right_thumb: bool,
    /// The two-hand plane the thumbs select: `core`, `extended`, `literal` or `user`.
    pub family_plane: &'static str,
    /// What releasing now would do, if anything.
    pub preview: Option<EntryView>,
}

/// One piece of the buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemView {
    /// `concept` or `word`.
    pub kind: &'static str,
    /// The token or the spelled word.
    pub text: String,
}

/// The buffer as it stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BufferView {
    /// What has been entered.
    pub items: Vec<ItemView>,
    /// Whether the last item is a word still being spelled.
    pub word_open: bool,
    /// The buffer in wire format.
    pub wire: String,
}

/// What a completed stroke did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EffectView {
    /// The buffer changed.
    Updated,
    /// Understood, but there was nothing to act on.
    Ignored,
    /// The buffer was handed over for decoding.
    Send {
        /// The utterance in wire format.
        wire: String,
        /// The utterance piece by piece.
        items: Vec<ItemView>,
    },
    /// A key for the host to pass on.
    Key {
        /// The command name, such as `left`.
        key: &'static str,
    },
    /// The layout gives the chord no meaning.
    Unmapped,
}

/// A completed stroke.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutcomeView {
    /// The chord in notation.
    pub chord: String,
    /// The chord as key bits.
    pub bits: u16,
    /// When the first key went down.
    pub started_ms: u64,
    /// When the last key came up.
    pub ended_ms: u64,
    /// What the layout says the chord does, if anything.
    pub action: Option<EntryView>,
    /// What it did this time.
    pub effect: EffectView,
}

/// The state of the session after a call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateView {
    /// Key bits that are down right now.
    pub held: u16,
    /// The stroke in progress.
    pub pending: PendingView,
    /// The stroke that just completed, if this call completed one.
    pub outcome: Option<OutcomeView>,
    /// The buffer after the call.
    pub buffer: BufferView,
}

/// One chat message for a decoder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageView {
    /// `system`, `user` or `assistant`.
    pub role: &'static str,
    /// The text of the message.
    pub content: String,
}

/// An earlier exchange, as the page passes it in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TurnInput {
    /// The utterance in wire format.
    pub utterance: String,
    /// The text it became.
    pub text: String,
}

/// A session a web page can drive, speaking only plain data.
#[derive(Debug, Clone)]
pub struct Demo {
    session: Session,
    last_sent: Option<Utterance>,
}

impl Demo {
    /// Compiles a layout and starts a session with it.
    pub fn new(layout_toml: &str) -> Result<Demo, String> {
        let compiled = Layout::from_toml(layout_toml).map_err(|error| error.to_string())?;
        Ok(Demo {
            session: Session::new(compiled.layout),
            last_sent: None,
        })
    }

    fn layout_ref(&self) -> &Layout {
        self.session.engine().layout()
    }

    /// Everything static about the layout.
    pub fn layout(&self) -> LayoutView {
        let layout = self.layout_ref();
        let meta = layout.meta();

        let thumbs = layout
            .entries()
            .iter()
            .filter(|entry| entry.chord.plane() == Some(Plane::Thumbs))
            .map(EntryView::of)
            .collect();

        let right_thumb = Key::new(Hand::Right, Finger::Thumb);
        let spelling = spell_shape_order()
            .into_iter()
            .filter_map(|shape| {
                let chord = Chord::of_hand(Hand::Left, shape);
                let text = text_of(layout.lookup(chord)?)?;
                let shifted = layout.lookup(chord.with(right_thumb)).and_then(text_of);
                Some(SpellView {
                    shape: shape.to_string(),
                    text,
                    shifted,
                })
            })
            .collect();

        let families = layout
            .families()
            .iter()
            .map(|family| {
                let (left_thumb, right_thumb) = family.plane.thumbs().unwrap_or((false, false));
                let left = with_thumb(family.left, left_thumb);
                let members = finger_shape_order()
                    .into_iter()
                    .filter_map(|right| {
                        let chord = Chord::from_hands(left, with_thumb(right, right_thumb));
                        layout.lookup(chord).map(|entry| MemberView {
                            right: right.to_string(),
                            entry: EntryView::of(entry),
                        })
                    })
                    .collect();
                FamilyView {
                    id: family.id.clone(),
                    name: family.name.clone(),
                    plane: family.plane.name(),
                    kind: match family.kind {
                        Kind::Concept => "concept",
                        Kind::Text => "text",
                        Kind::Command => "command",
                    },
                    left: family.left.to_string(),
                    members,
                }
            })
            .collect();

        LayoutView {
            name: meta.name.clone(),
            version: meta.version.clone(),
            description: meta.description.clone(),
            finger_shapes: FINGER_SHAPE_ORDER.to_vec(),
            spell_shapes: SPELL_SHAPE_ORDER.to_vec(),
            thumbs,
            spelling,
            families,
        }
    }

    /// A key went down. `key` is its board position, 0 being the left pinky.
    pub fn press(&mut self, key: u8, at_ms: f64) -> Result<UpdateView, String> {
        let event = KeyEvent::press(key_at(key)?, whole_ms(at_ms));
        Ok(self.feed(event))
    }

    /// A key came up.
    pub fn release(&mut self, key: u8, at_ms: f64) -> Result<UpdateView, String> {
        let event = KeyEvent::release(key_at(key)?, whole_ms(at_ms));
        Ok(self.feed(event))
    }

    fn feed(&mut self, event: KeyEvent) -> UpdateView {
        let outcome = self.session.on_event(event);
        self.update(outcome.as_ref())
    }

    /// Applies a chord written in notation, as if it had just been stroked.
    pub fn strike(&mut self, chord: &str, at_ms: f64) -> Result<UpdateView, String> {
        let chord: Chord = chord
            .parse()
            .map_err(|error| format!("\"{chord}\" is not a chord: {error}"))?;
        let at = whole_ms(at_ms);
        let effect = self.session.strike(chord);
        let outcome = Outcome {
            stroke: intent_core::Stroke {
                chord,
                started_ms: at,
                ended_ms: at,
            },
            effect,
        };
        Ok(self.update(Some(&outcome)))
    }

    /// Drops any stroke in progress. Call when the page loses focus.
    pub fn release_all(&mut self) -> UpdateView {
        self.session.release_all();
        self.update(None)
    }

    /// The current state, without changing anything.
    pub fn state(&self) -> UpdateView {
        self.view(None)
    }

    fn update(&mut self, outcome: Option<&Outcome>) -> UpdateView {
        if let Some(Outcome {
            effect: Effect::Send(utterance),
            ..
        }) = outcome
        {
            self.last_sent = Some(utterance.clone());
        }
        self.view(outcome)
    }

    fn view(&self, outcome: Option<&Outcome>) -> UpdateView {
        let layout = self.layout_ref();
        let pending = self.session.pending();
        let left = pending.hand(Hand::Left);
        let right = pending.hand(Hand::Right);
        let buffer = self.session.engine().buffer();

        UpdateView {
            held: self.session.held().bits(),
            pending: PendingView {
                bits: pending.bits(),
                chord: if pending.is_empty() {
                    String::new()
                } else {
                    pending.to_string()
                },
                left: left.fingers().to_string(),
                right: right.fingers().to_string(),
                left_thumb: left.has_thumb(),
                right_thumb: right.has_thumb(),
                family_plane: match (left.has_thumb(), right.has_thumb()) {
                    (false, false) => Plane::Core,
                    (false, true) => Plane::Extended,
                    (true, false) => Plane::Literal,
                    (true, true) => Plane::User,
                }
                .name(),
                preview: layout.lookup(pending).map(EntryView::of),
            },
            outcome: outcome.map(|outcome| OutcomeView {
                chord: outcome.stroke.chord.to_string(),
                bits: outcome.stroke.chord.bits(),
                started_ms: outcome.stroke.started_ms,
                ended_ms: outcome.stroke.ended_ms,
                action: layout.lookup(outcome.stroke.chord).map(EntryView::of),
                effect: match &outcome.effect {
                    Effect::Updated => EffectView::Updated,
                    Effect::Ignored => EffectView::Ignored,
                    Effect::Send(utterance) => EffectView::Send {
                        wire: utterance.to_string(),
                        items: items_of(utterance),
                    },
                    Effect::Key(command) => EffectView::Key {
                        key: command.name(),
                    },
                    Effect::Unmapped(_) => EffectView::Unmapped,
                },
            }),
            buffer: BufferView {
                items: items_of(&buffer.utterance),
                word_open: buffer.word_open,
                wire: buffer.utterance.to_string(),
            },
        }
    }

    /// The chat messages that ask a model to decode the last sent utterance.
    pub fn decode_messages(
        &self,
        history: &[TurnInput],
        language: &str,
    ) -> Result<Vec<MessageView>, String> {
        let utterance = self.last_sent.as_ref().ok_or("nothing has been sent yet")?;
        let history: Vec<Turn> = history
            .iter()
            .map(|turn| Turn {
                utterance: turn.utterance.clone(),
                text: turn.text.clone(),
            })
            .collect();
        let options = PromptOptions {
            language: language.to_owned(),
        };
        Ok(prompt::messages(utterance, &history, &options)
            .into_iter()
            .map(|message| MessageView {
                role: message.role.name(),
                content: message.content,
            })
            .collect())
    }
}

fn key_at(index: u8) -> Result<Key, String> {
    Key::from_index(index).ok_or_else(|| format!("key {index} does not exist; keys are 0 to 9"))
}

/// Browser timestamps are fractional milliseconds. Whole ones are plenty.
fn whole_ms(at_ms: f64) -> u64 {
    if at_ms.is_finite() && at_ms > 0.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // checked finite and positive; float-to-int casts saturate
        let whole = at_ms as u64;
        whole
    } else {
        0
    }
}

fn with_thumb(shape: HandShape, thumb: bool) -> HandShape {
    if thumb {
        shape.with(Finger::Thumb)
    } else {
        shape
    }
}

fn text_of(entry: &Entry) -> Option<String> {
    match &entry.action {
        Action::Text(text) => Some(text.clone()),
        Action::Concept(_) | Action::Command(_) => None,
    }
}

fn items_of(utterance: &Utterance) -> Vec<ItemView> {
    utterance
        .items
        .iter()
        .map(|item| match item {
            Item::Concept(concept) => ItemView {
                kind: "concept",
                text: concept.token.clone(),
            },
            Item::Word(word) => ItemView {
                kind: "word",
                text: word.clone(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const V0: &str = include_str!("../../../layouts/intent-v0.toml");
    // Board positions.
    const LEFT_INDEX: u8 = 3;
    const LEFT_THUMB: u8 = 4;
    const RIGHT_THUMB: u8 = 5;
    const RIGHT_INDEX: u8 = 6;
    const RIGHT_MIDDLE: u8 = 7;

    fn demo() -> Demo {
        Demo::new(V0).expect("intent-v0 compiles")
    }

    /// Presses the keys in order, then releases them in order.
    fn stroke(demo: &mut Demo, keys: &[u8], at: f64) -> UpdateView {
        for key in keys {
            demo.press(*key, at).expect("valid key");
        }
        let mut last = None;
        for key in keys {
            last = Some(demo.release(*key, at + 50.0).expect("valid key"));
        }
        last.expect("at least one key")
    }

    #[test]
    fn a_broken_layout_reports_its_errors() {
        let error =
            Demo::new("[layout]\nname = \"x\"\nversion = \"1\"\n[thumbs]\nleft = \"fly\"\n")
                .expect_err("should fail");
        assert!(
            error.starts_with("error: thumbs.left: unknown command \"fly\""),
            "{error}"
        );
    }

    #[test]
    fn layout_view_describes_the_whole_layout() {
        let view = demo().layout();
        assert_eq!(view.name, "intent-v0");
        assert_eq!(view.finger_shapes.len(), 15);
        assert_eq!(view.spell_shapes.len(), 30);
        assert_eq!(view.finger_shapes[..4], ["I", "M", "R", "P"]);

        let thumbs: Vec<(&str, &str)> = view
            .thumbs
            .iter()
            .map(|entry| (entry.chord.as_str(), entry.label.as_str()))
            .collect();
        assert_eq!(thumbs, [("T-", "undo"), ("-T", "space"), ("T-T", "send")]);

        assert_eq!(view.spelling.len(), 30);
        assert_eq!(
            view.spelling[0],
            SpellView {
                shape: "I".into(),
                text: "e".into(),
                shifted: Some("E".into()),
            }
        );
        let full_stop = view
            .spelling
            .iter()
            .find(|s| s.text == ".")
            .expect("full stop");
        assert_eq!(full_stop.shape, "IMRP");
        assert_eq!(full_stop.shifted.as_deref(), Some(":"));

        assert_eq!(view.families.len(), 23);
        let act = &view.families[0];
        assert_eq!(
            (act.id.as_str(), act.plane, act.kind, act.left.as_str()),
            ("act", "core", "concept", "I")
        );
        assert_eq!(act.members.len(), 15);
        assert_eq!(act.members[1].right, "M");
        assert_eq!(act.members[1].entry.label, "FIX");
        assert_eq!(act.members[1].entry.chord, "I-M");
    }

    #[test]
    fn family_members_carry_their_plane_thumbs_and_glosses() {
        let view = demo().layout();
        let find = |id: &str| view.families.iter().find(|f| f.id == id).expect("family");

        let tech = find("tech");
        assert_eq!(tech.plane, "extended");
        assert_eq!(tech.members[0].entry.chord, "I-TI");

        let digits = find("digits");
        assert_eq!((digits.plane, digits.kind), ("literal", "text"));
        assert_eq!(digits.members.len(), 10);
        assert_eq!(digits.members[0].entry.chord, "IT-I");
        assert_eq!(digits.members[0].entry.label, "1");

        let keys = find("keys");
        assert_eq!(keys.kind, "command");
        assert_eq!(keys.members[0].entry.label, "left");

        let directive = find("directive");
        assert_eq!(
            directive.members[0].entry.gloss.as_deref(),
            Some("add or update tests that cover this")
        );
    }

    #[test]
    fn holding_keys_shows_the_stroke_in_progress_and_a_preview() {
        let mut demo = demo();
        let update = demo.press(LEFT_INDEX, 10.0).expect("valid key");
        assert_eq!(update.held, 1 << LEFT_INDEX);
        assert_eq!(update.pending.chord, "I-");
        assert_eq!(update.pending.left, "I");
        assert_eq!(update.pending.right, "");
        assert_eq!(update.pending.family_plane, "core");
        // One hand alone spells: left index is "e".
        assert_eq!(
            update.pending.preview.as_ref().map(|p| p.label.as_str()),
            Some("e")
        );
        assert_eq!(update.outcome, None);

        let update = demo.press(RIGHT_MIDDLE, 20.0).expect("valid key");
        assert_eq!(update.pending.chord, "I-M");
        assert_eq!(update.pending.right, "M");
        let preview = update.pending.preview.expect("preview");
        assert_eq!((preview.kind, preview.label.as_str()), ("concept", "FIX"));
    }

    #[test]
    fn thumbs_select_the_family_plane() {
        let mut demo = demo();
        demo.press(LEFT_INDEX, 0.0).expect("valid key");
        let update = demo.press(RIGHT_THUMB, 0.0).expect("valid key");
        assert_eq!(update.pending.family_plane, "extended");
        assert!(update.pending.right_thumb);
        assert_eq!(update.pending.left, "I");
        let update = demo.press(LEFT_THUMB, 0.0).expect("valid key");
        assert_eq!(update.pending.family_plane, "user");
        let update = demo.release_all();
        assert_eq!(update.pending.family_plane, "core");
        assert_eq!(update.pending.chord, "");
        assert_eq!(update.held, 0);
    }

    #[test]
    fn a_completed_stroke_reports_what_it_did() {
        let mut demo = demo();
        let update = stroke(&mut demo, &[LEFT_INDEX, RIGHT_MIDDLE], 100.0);
        let outcome = update.outcome.expect("outcome");
        assert_eq!(outcome.chord, "I-M");
        assert_eq!((outcome.started_ms, outcome.ended_ms), (100, 150));
        assert_eq!(outcome.effect, EffectView::Updated);
        assert_eq!(outcome.action.map(|a| a.label), Some("FIX".into()));
        assert_eq!(update.buffer.wire, "FIX");
        assert_eq!(update.pending.chord, "");
        assert_eq!(update.held, 0);
    }

    #[test]
    fn spelling_shows_an_open_word() {
        let mut demo = demo();
        stroke(&mut demo, &[LEFT_INDEX], 0.0);
        let update = stroke(&mut demo, &[RIGHT_INDEX], 100.0);
        assert_eq!(
            update.buffer.items,
            [ItemView {
                kind: "word",
                text: "ee".into(),
            }]
        );
        assert!(update.buffer.word_open);
        let update = stroke(&mut demo, &[RIGHT_THUMB], 200.0);
        assert!(!update.buffer.word_open);
    }

    #[test]
    fn send_hands_over_the_utterance_and_enables_decoding() {
        let mut demo = demo();
        assert_eq!(
            demo.decode_messages(&[], "English"),
            Err("nothing has been sent yet".to_owned())
        );
        stroke(&mut demo, &[LEFT_INDEX, RIGHT_MIDDLE], 0.0);
        let update = stroke(&mut demo, &[LEFT_THUMB, RIGHT_THUMB], 100.0);
        let outcome = update.outcome.expect("outcome");
        assert_eq!(
            outcome.effect,
            EffectView::Send {
                wire: "FIX".into(),
                items: vec![ItemView {
                    kind: "concept",
                    text: "FIX".into(),
                }],
            }
        );
        assert_eq!(update.buffer.wire, "");

        let history = [TurnInput {
            utterance: "ADD TEST".into(),
            text: "Add a test.".into(),
        }];
        let messages = demo
            .decode_messages(&history, "Portuguese")
            .expect("messages");
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("Write in Portuguese"));
        let last = messages.len() - 1;
        assert_eq!(
            (messages[last].role, messages[last].content.as_str()),
            ("user", "FIX")
        );
        assert_eq!(messages[last - 1].content, "Add a test.");
        assert_eq!(messages[last - 2].content, "ADD TEST");
    }

    #[test]
    fn strike_applies_a_chord_by_name() {
        let mut demo = demo();
        let update = demo.strike("I-I", 5.9).expect("valid chord");
        assert_eq!(update.buffer.wire, "MAKE");
        let outcome = update.outcome.expect("outcome");
        assert_eq!((outcome.started_ms, outcome.ended_ms), (5, 5));

        let update = demo.strike("MIT-I", 6.0).expect("valid chord");
        assert_eq!(
            update.outcome.expect("outcome").effect,
            EffectView::Key { key: "left" }
        );
        let update = demo.strike("PRMIT-TIMRP", 7.0).expect("valid chord");
        let outcome = update.outcome.expect("outcome");
        assert_eq!(outcome.effect, EffectView::Unmapped);
        assert_eq!(outcome.action, None);
        let update = demo.strike("-T", 8.0).expect("valid chord");
        assert_eq!(update.outcome.expect("outcome").effect, EffectView::Ignored);

        let error = demo.strike("hello", 9.0).expect_err("not a chord");
        assert!(
            error.starts_with("\"hello\" is not a chord: missing '-'"),
            "{error}"
        );
    }

    #[test]
    fn keys_outside_the_board_are_rejected() {
        let mut demo = demo();
        let expected = Err("key 10 does not exist; keys are 0 to 9".to_owned());
        assert_eq!(demo.press(10, 0.0), expected);
        assert_eq!(demo.release(10, 0.0), expected);
    }

    #[test]
    fn odd_timestamps_do_not_break_anything() {
        assert_eq!(whole_ms(12.9), 12);
        assert_eq!(whole_ms(-5.0), 0);
        assert_eq!(whole_ms(f64::NAN), 0);
        assert_eq!(whole_ms(f64::INFINITY), 0);
    }

    #[test]
    fn state_reads_without_changing_anything() {
        let mut demo = demo();
        demo.strike("I-I", 0.0).expect("valid chord");
        demo.press(LEFT_INDEX, 1.0).expect("valid key");
        let first = demo.state();
        assert_eq!(first, demo.state());
        assert_eq!(first.pending.chord, "I-");
        assert_eq!(first.buffer.wire, "MAKE");
        assert_eq!(first.outcome, None);
    }

    /// The JSON shape is the contract with the web page. Changing it means
    /// changing `web/src/engine.ts` too.
    #[test]
    fn update_json_shape_is_stable() {
        let mut demo = demo();
        demo.press(LEFT_INDEX, 10.0).expect("valid key");
        demo.press(RIGHT_MIDDLE, 20.0).expect("valid key");
        demo.release(LEFT_INDEX, 60.0).expect("valid key");
        let update = demo.release(RIGHT_MIDDLE, 70.0).expect("valid key");
        assert_eq!(
            serde_json::to_value(&update).expect("serialises"),
            json!({
                "held": 0,
                "pending": {
                    "bits": 0,
                    "chord": "",
                    "left": "",
                    "right": "",
                    "left_thumb": false,
                    "right_thumb": false,
                    "family_plane": "core",
                    "preview": null,
                },
                "outcome": {
                    "chord": "I-M",
                    "bits": 136,
                    "started_ms": 10,
                    "ended_ms": 70,
                    "action": {
                        "chord": "I-M",
                        "bits": 136,
                        "kind": "concept",
                        "label": "FIX",
                        "gloss": null,
                    },
                    "effect": { "type": "updated" },
                },
                "buffer": {
                    "items": [{ "kind": "concept", "text": "FIX" }],
                    "word_open": false,
                    "wire": "FIX",
                },
            })
        );
    }

    #[test]
    fn member_json_is_flat() {
        let view = demo().layout();
        let member = serde_json::to_value(&view.families[0].members[0]).expect("serialises");
        assert_eq!(
            member,
            json!({
                "right": "I",
                "chord": "I-I",
                "bits": 72,
                "kind": "concept",
                "label": "MAKE",
                "gloss": null,
            })
        );
    }

    #[test]
    fn effects_serialise_with_a_type_tag() {
        let send = EffectView::Send {
            wire: "A".into(),
            items: vec![],
        };
        assert_eq!(
            serde_json::to_value(send).expect("serialises"),
            json!({ "type": "send", "wire": "A", "items": [] })
        );
        assert_eq!(
            serde_json::to_value(EffectView::Key { key: "left" }).expect("serialises"),
            json!({ "type": "key", "key": "left" })
        );
        assert_eq!(
            serde_json::to_value(EffectView::Unmapped).expect("serialises"),
            json!({ "type": "unmapped" })
        );
    }
}
