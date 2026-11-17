//! How much of real text a layout covers.
//!
//! Every word of a corpus is put in one of three groups:
//!
//! - **concept**: a concept token covers it, directly ("fix" → `FIX`) or as a
//!   form the decoder can produce ("fixed", "fixes" → `FIX`).
//! - **grammar**: an article or a form of "be", which the decoder adds on its
//!   own when it writes the sentence. Costs no stroke.
//! - **spelled**: everything else, typed letter by letter.
//!
//! The matching is deliberately simple and errs both ways: a suffix rule may
//! accept a form the decoder would get wrong, and a word the decoder could
//! paraphrase ("wanna" for `WANT`) counts as spelled. It measures what has a
//! chord, not whether the decoder recovers the meaning; that needs a
//! round-trip test against a real model.

use std::collections::HashMap;

use intent_core::{Action, Command, Layout};

/// Articles and forms of "be": words the decoder supplies from grammar alone.
const GRAMMAR: [&str; 11] = [
    "a", "an", "the", "be", "am", "is", "are", "was", "were", "been", "being",
];

/// Irregular forms mapped to the word a concept token would use.
const IRREGULAR: &[(&str, &str)] = &[
    ("went", "go"),
    ("gone", "go"),
    ("goes", "go"),
    ("made", "make"),
    ("got", "get"),
    ("gotten", "get"),
    ("saw", "see"),
    ("seen", "see"),
    ("took", "take"),
    ("taken", "take"),
    ("gave", "give"),
    ("given", "give"),
    ("came", "come"),
    ("knew", "know"),
    ("known", "know"),
    ("thought", "think"),
    ("told", "tell"),
    ("said", "say"),
    ("says", "say"),
    ("found", "find"),
    ("left", "leave"),
    ("kept", "keep"),
    ("ran", "run"),
    ("wrote", "write"),
    ("written", "write"),
    ("did", "do"),
    ("done", "do"),
    ("does", "do"),
    ("had", "have"),
    ("has", "have"),
    ("better", "good"),
    ("best", "good"),
    ("worse", "bad"),
    ("worst", "bad"),
    ("children", "child"),
    ("people", "person"),
    ("men", "man"),
    ("women", "woman"),
    ("bought", "buy"),
    ("brought", "bring"),
    ("felt", "feel"),
    ("meant", "mean"),
    ("met", "meet"),
    ("sent", "send"),
    ("built", "build"),
    ("began", "begin"),
    ("begun", "begin"),
    ("broke", "break"),
    ("broken", "break"),
    ("chose", "choose"),
    ("chosen", "choose"),
    ("forgot", "forget"),
    ("forgotten", "forget"),
    ("heard", "hear"),
    ("lost", "lose"),
    ("paid", "pay"),
    ("sold", "sell"),
    ("spent", "spend"),
    ("stood", "stand"),
    ("understood", "understand"),
    ("won", "win"),
    ("ate", "eat"),
    ("eaten", "eat"),
    ("drank", "drink"),
    ("drove", "drive"),
    ("driven", "drive"),
    ("fell", "fall"),
    ("fallen", "fall"),
    ("flew", "fly"),
    ("flown", "fly"),
    ("grew", "grow"),
    ("grown", "grow"),
    ("held", "hold"),
    ("hid", "hide"),
    ("led", "lead"),
    ("lent", "lend"),
    ("rode", "ride"),
    ("rang", "ring"),
    ("rose", "rise"),
    ("sat", "sit"),
    ("slept", "sleep"),
    ("spoke", "speak"),
    ("spoken", "speak"),
    ("swam", "swim"),
    ("taught", "teach"),
    ("threw", "throw"),
    ("thrown", "throw"),
    ("wore", "wear"),
    ("worn", "wear"),
    ("caught", "catch"),
    ("fought", "fight"),
    ("sought", "seek"),
    ("shot", "shoot"),
    ("hit", "hit"),
    ("cut", "cut"),
    ("set", "set"),
    ("shut", "shut"),
    ("let", "let"),
    ("put", "put"),
    ("read", "read"),
    ("cost", "cost"),
    ("hurt", "hurt"),
];

/// Words a suffix rule would misread: "news" is not the plural of "new".
const NOT_INFLECTED: &[&str] = &["news", "always", "perhaps", "series", "species"];

/// Which group a word fell in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// Covered by a concept token.
    Concept,
    /// Supplied by the decoder from grammar.
    Grammar,
    /// Typed letter by letter.
    Spelled,
}

/// Splits text into lowercase words, expanding contractions into the words
/// they stand for. Punctuation is dropped: the decoder adds it.
pub fn words(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let normalised = text.replace(['\u{2019}', '\u{2018}'], "'").to_lowercase();
    for raw in normalised.split(|c: char| !(c.is_alphanumeric() || c == '\'')) {
        let word = raw.trim_matches('\'');
        if word.is_empty() {
            continue;
        }
        found.extend(expand_contraction(word));
    }
    found
}

/// Pieces of contractions that some corpora split off as separate words, as
/// in "don t" or "it s". Standing alone, they are almost never anything else.
const FRAGMENTS: &[(&str, &str)] = &[
    ("s", "is"),
    ("t", "not"),
    ("m", "am"),
    ("re", "are"),
    ("ll", "will"),
    ("ve", "have"),
    ("d", "would"),
    ("don", "do"),
    ("didn", "did"),
    ("doesn", "does"),
    ("isn", "is"),
    ("wasn", "was"),
    ("aren", "are"),
    ("weren", "were"),
    ("couldn", "could"),
    ("wouldn", "would"),
    ("shouldn", "should"),
    ("haven", "have"),
    ("hasn", "has"),
    ("hadn", "had"),
];

fn expand_contraction(word: &str) -> Vec<String> {
    if let Some((_, whole)) = FRAGMENTS.iter().find(|(fragment, _)| *fragment == word) {
        return vec![(*whole).to_owned()];
    }
    let special = match word {
        "can't" | "cannot" => Some(["can", "not"]),
        "won't" => Some(["will", "not"]),
        "shan't" => Some(["shall", "not"]),
        "let's" => Some(["let", "we"]),
        _ => None,
    };
    if let Some(parts) = special {
        return parts.iter().map(|part| (*part).to_owned()).collect();
    }
    for (suffix, expansion) in [
        ("n't", "not"),
        ("'m", "am"),
        ("'re", "are"),
        ("'ll", "will"),
        ("'ve", "have"),
        ("'d", "would"),
        ("'s", "is"),
    ] {
        if let Some(stem) = word.strip_suffix(suffix)
            && !stem.is_empty()
        {
            return vec![stem.to_owned(), expansion.to_owned()];
        }
    }
    // Anything else with an apostrophe, such as "o'clock", stays one word.
    vec![word.to_owned()]
}

/// Base words a form could come from, most likely first. Always starts with
/// the word itself.
pub fn base_forms(word: &str) -> Vec<String> {
    let mut forms = vec![word.to_owned()];
    // Pronoun case ("me" → "i") and modals ("would" → "will") are left out on
    // purpose: in round-trip tests the decoder could not restore them.
    for (form, base) in IRREGULAR {
        if *form == word && !forms.iter().any(|known| known == base) {
            forms.push((*base).to_owned());
        }
    }
    // Suffix rules guess, so they never shrink a word to a single letter.
    let mut add = |form: String| {
        if form.len() >= 2 && !forms.contains(&form) {
            forms.push(form);
        }
    };
    if NOT_INFLECTED.contains(&word) {
        return forms;
    }
    // "thank" → THANKS, "doc" → DOCS: tokens that are themselves plural.
    add(format!("{word}s"));
    let rules: [(&str, &[&str]); 9] = [
        ("ies", &["y"]),
        ("ied", &["y"]),
        ("es", &["", "e"]),
        ("s", &[""]),
        ("ed", &["", "e"]),
        ("ing", &["", "e"]),
        ("ier", &["y"]),
        ("er", &["", "e"]),
        ("ly", &[""]),
    ];
    for (suffix, replacements) in rules {
        let Some(stem) = word.strip_suffix(suffix) else {
            continue;
        };
        for replacement in replacements {
            add(format!("{stem}{replacement}"));
        }
        // stopped → stop, running → run
        let bytes = stem.as_bytes();
        if bytes.len() >= 3 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
            add(stem[..stem.len() - 1].to_owned());
        }
    }
    forms
}

/// Matches words against one layout.
pub struct Matcher<'a> {
    layout: &'a Layout,
    tokens: HashMap<String, String>,
    space_strokes: usize,
}

impl<'a> Matcher<'a> {
    pub fn new(layout: &'a Layout) -> Matcher<'a> {
        let tokens = layout
            .entries()
            .iter()
            .filter_map(|entry| match &entry.action {
                // Clause-sized tokens such as WITH_TESTS are not single words.
                Action::Concept(concept) if !concept.token.contains('_') => {
                    Some((concept.token.to_lowercase(), concept.token.clone()))
                }
                _ => None,
            })
            .collect();
        let space_strokes = usize::from(!layout.chords_for_command(Command::Space).is_empty());
        Matcher {
            layout,
            tokens,
            space_strokes,
        }
    }

    /// The concept token covering a word, if any.
    pub fn concept(&self, word: &str) -> Option<&str> {
        base_forms(word)
            .iter()
            .find_map(|form| self.tokens.get(form))
            .map(String::as_str)
    }

    /// Which group a word falls in.
    pub fn classify(&self, word: &str) -> Class {
        if self.concept(word).is_some() {
            Class::Concept
        } else if GRAMMAR.contains(&word) {
            Class::Grammar
        } else {
            Class::Spelled
        }
    }

    /// Strokes to spell a word, or `None` when a character has no chord.
    pub fn spelling_strokes(&self, word: &str) -> Option<usize> {
        word.chars()
            .map(|c| (!self.layout.chords_for_text(&c.to_string()).is_empty()).then_some(1))
            .sum()
    }
}

/// What a user would enter for a sentence: a concept token for each covered
/// word, the spelled word for the rest, and nothing for grammar words.
pub fn encode(matcher: &Matcher<'_>, text: &str) -> intent_core::Utterance {
    let mut items = Vec::new();
    for word in words(text) {
        match matcher.classify(&word) {
            Class::Grammar => {}
            Class::Concept => {
                let token = matcher.concept(&word).expect("classified as a concept");
                let entry = matcher
                    .layout
                    .chord_for_token(token)
                    .and_then(|chord| matcher.layout.lookup(chord));
                if let Some(intent_core::layout::Entry {
                    action: Action::Concept(concept),
                    ..
                }) = entry
                {
                    items.push(intent_core::Item::Concept(concept.clone()));
                }
            }
            Class::Spelled => items.push(intent_core::Item::Word(word)),
        }
    }
    intent_core::Utterance { items }
}

/// Totals for a corpus.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Word tokens counted, with each frequency-list entry weighted by its count.
    pub words: u64,
    pub concept: u64,
    pub grammar: u64,
    pub spelled: u64,
    /// Spelled words containing a character no chord can type.
    pub unenterable: u64,
    /// Strokes to enter everything: concepts, letters and spaces, plus one send per utterance.
    pub strokes: u64,
    /// Letters plus one space per word: what typing the words on a keyboard takes.
    pub keystrokes: u64,
    /// Utterances seen (zero for frequency lists).
    pub utterances: u64,
    /// Utterances that need no spelling at all.
    pub utterances_without_spelling: u64,
    /// Spelled words and how often they occurred.
    pub missing: HashMap<String, u64>,
}

impl Report {
    fn add_word(
        &mut self,
        matcher: &Matcher<'_>,
        word: &str,
        weight: u64,
        after_spelled: bool,
    ) -> Class {
        let class = matcher.classify(word);
        self.words += weight;
        self.keystrokes += weight * (word.chars().count() as u64 + 1);
        match class {
            Class::Concept => {
                self.concept += weight;
                self.strokes += weight;
            }
            Class::Grammar => self.grammar += weight,
            Class::Spelled => {
                self.spelled += weight;
                *self.missing.entry(word.to_owned()).or_default() += weight;
                match matcher.spelling_strokes(word) {
                    Some(letters) => {
                        let space = if after_spelled {
                            matcher.space_strokes
                        } else {
                            0
                        };
                        self.strokes += weight * (letters + space) as u64;
                    }
                    None => self.unenterable += weight,
                }
            }
        }
        class
    }

    /// Counts one utterance of running text.
    pub fn add_utterance(&mut self, matcher: &Matcher<'_>, text: &str) {
        let words = words(text);
        if words.is_empty() {
            return;
        }
        self.utterances += 1;
        self.strokes += 1; // the send
        let mut previous_spelled = false;
        let mut any_spelled = false;
        for word in &words {
            let class = self.add_word(matcher, word, 1, previous_spelled);
            previous_spelled = class == Class::Spelled;
            any_spelled |= previous_spelled;
        }
        if !any_spelled {
            self.utterances_without_spelling += 1;
        }
    }

    /// Counts one frequency-list entry: a word that occurs `count` times.
    pub fn add_frequency(&mut self, matcher: &Matcher<'_>, word: &str, count: u64) {
        for part in words(word) {
            self.add_word(matcher, &part, count, false);
        }
    }

    /// The most frequent spelled words, most frequent first.
    pub fn top_missing(&self, limit: usize) -> Vec<(&str, u64)> {
        let mut missing: Vec<(&str, u64)> = self
            .missing
            .iter()
            .map(|(word, count)| (word.as_str(), *count))
            .collect();
        missing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        missing.truncate(limit);
        missing
    }
}

/// `part` as a percentage of `whole`, one decimal.
pub fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "0.0%".to_owned();
    }
    #[allow(clippy::cast_precision_loss)] // corpus counts are far below 2^52
    let value = part as f64 * 100.0 / whole as f64;
    format!("{value:.1}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    const V0: &str = include_str!("../../../layouts/intent-v0.toml");

    fn layout() -> Layout {
        Layout::from_toml(V0).expect("intent-v0 compiles").layout
    }

    #[test]
    fn words_are_lowercased_and_contractions_expanded() {
        assert_eq!(words("I don't know, OK?"), ["i", "do", "not", "know", "ok"]);
        assert_eq!(
            words("It\u{2019}s fine. We'll see; can't stop"),
            [
                "it", "is", "fine", "we", "will", "see", "can", "not", "stop"
            ]
        );
        assert_eq!(
            words("won't let's o'clock 'quoted' 42"),
            ["will", "not", "let", "we", "o'clock", "quoted", "42"]
        );
        assert_eq!(words("  ...  "), Vec::<String>::new());
        // Corpora that split contractions apart.
        assert_eq!(
            words("it s fine , don t worry , i m sure"),
            ["it", "is", "fine", "do", "not", "worry", "i", "am", "sure"]
        );
    }

    #[test]
    fn base_forms_cover_regular_and_irregular_forms() {
        let has = |word: &str, base: &str| base_forms(word).contains(&base.to_owned());
        assert!(has("fixed", "fix"));
        assert!(has("fixes", "fix"));
        assert!(has("tests", "test"));
        assert!(has("making", "make"));
        assert!(has("used", "use"));
        assert!(has("stopped", "stop"));
        assert!(has("running", "run"));
        assert!(has("tries", "try"));
        assert!(has("tried", "try"));
        assert!(has("went", "go"));
        assert!(has("thought", "think"));
        // Changes of meaning are not forms of the same word.
        assert!(!has("me", "i"));
        assert!(!has("would", "will"));
        assert!(!has("news", "new"));
        assert!(has("quickly", "quick"));
        assert!(has("thank", "thanks"));
        assert_eq!(base_forms("fix")[0], "fix");
        // Never shrinks a word to a single letter.
        assert!(!base_forms("is").contains(&"i".to_owned()));
    }

    #[test]
    fn words_fall_into_the_right_group() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        assert_eq!(matcher.concept("fixed"), Some("FIX"));
        assert_eq!(matcher.concept("me"), None);
        assert_eq!(matcher.concept("news"), None);
        assert_eq!(matcher.concept("thinking"), Some("THINK"));
        assert_eq!(
            matcher.classify("FIX".to_lowercase().as_str()),
            Class::Concept
        );
        assert_eq!(matcher.classify("the"), Class::Grammar);
        assert_eq!(matcher.classify("were"), Class::Grammar);
        assert_eq!(matcher.classify("banana"), Class::Spelled);
        // Clause-sized tokens never match a single word.
        assert_eq!(matcher.concept("brief"), Some("BRIEF"));
        assert_eq!(matcher.concept("with_tests"), None);
    }

    #[test]
    fn spelling_counts_one_stroke_per_character_that_has_a_chord() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        assert_eq!(matcher.spelling_strokes("login"), Some(5));
        assert_eq!(matcher.spelling_strokes("42"), Some(2));
        assert_eq!(matcher.spelling_strokes("naïve"), None);
    }

    #[test]
    fn an_utterance_counts_concepts_grammar_spelling_and_strokes() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        let mut report = Report::default();
        // fix=concept, the=grammar, bug=concept, in=concept, login=spelled(5)
        report.add_utterance(&matcher, "Fix the bug in login.");
        assert_eq!(
            (report.words, report.concept, report.grammar, report.spelled),
            (5, 3, 1, 1)
        );
        // 3 concepts + 5 letters + 1 send
        assert_eq!(report.strokes, 9);
        // fix+1, the+1, bug+1, in+1, login+1 = 4+4+4+3+6
        assert_eq!(report.keystrokes, 21);
        assert_eq!(
            (report.utterances, report.utterances_without_spelling),
            (1, 0)
        );

        report.add_utterance(&matcher, "Thanks!");
        assert_eq!(report.utterances_without_spelling, 1);
        report.add_utterance(&matcher, "?!");
        assert_eq!(
            report.utterances, 2,
            "an utterance with no words is skipped"
        );
    }

    #[test]
    fn adjacent_spelled_words_need_a_space_between_them() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        let mut report = Report::default();
        report.add_utterance(&matcher, "banana bread");
        // 6 + 1 space + 5 + 1 send
        assert_eq!(report.strokes, 13);
        assert_eq!(report.top_missing(5), [("banana", 1), ("bread", 1)]);
    }

    #[test]
    fn frequency_entries_are_weighted_by_their_count() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        let mut report = Report::default();
        report.add_frequency(&matcher, "you", 100);
        report.add_frequency(&matcher, "the", 50);
        report.add_frequency(&matcher, "pizza", 10);
        report.add_frequency(&matcher, "don't", 5);
        assert_eq!(report.words, 170);
        // "don't" is "do" (spelled: v0 has no DO) and "not" (a concept).
        assert_eq!(report.concept, 100 + 5);
        assert_eq!(report.grammar, 50);
        assert_eq!(report.spelled, 10 + 5);
        assert_eq!(report.top_missing(2), [("pizza", 10), ("do", 5)]);
        assert_eq!(report.utterances, 0);
    }

    #[test]
    fn unenterable_words_are_counted_apart() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        let mut report = Report::default();
        report.add_utterance(&matcher, "café");
        assert_eq!((report.spelled, report.unenterable), (1, 1));
    }

    #[test]
    fn encoding_keeps_concepts_spells_the_rest_and_drops_grammar() {
        let layout = layout();
        let matcher = Matcher::new(&layout);
        assert_eq!(
            encode(&matcher, "I fixed the bug in login.").to_string(),
            "I FIX BUG IN \"login\""
        );
        assert_eq!(encode(&matcher, "the").to_string(), "");
    }

    #[test]
    fn percentages_have_one_decimal() {
        assert_eq!(percent(1, 3), "33.3%");
        assert_eq!(percent(0, 0), "0.0%");
        assert_eq!(percent(5, 5), "100.0%");
    }
}
