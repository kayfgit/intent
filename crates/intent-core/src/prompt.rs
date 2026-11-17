//! Building the request a decoder model receives.
//!
//! The decoder is any chat model that turns an utterance into finished text.
//! This module only builds the messages and tidies the reply. Sending them is
//! the host's job, which keeps the core free of networking and lets every
//! host share one prompt.
//!
//! The prompt is short on purpose. Free model tiers count tokens per minute
//! and per day, and every decode pays for the whole prompt again.

use crate::engine::Utterance;

/// Who a message is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Instructions for the model.
    System,
    /// What the user entered.
    User,
    /// What the model answered.
    Assistant,
}

impl Role {
    /// The name chat APIs use for this role.
    pub const fn name(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

/// One chat message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Who the message is from.
    pub role: Role,
    /// The text of the message.
    pub content: String,
}

/// An earlier exchange: what was entered and the text it became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    /// The utterance in wire format, as it was sent.
    pub utterance: String,
    /// The text the user accepted for it.
    pub text: String,
}

/// Choices that shape the decoder's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptOptions {
    /// The language to write in, such as `English` or `Portuguese`.
    pub language: String,
}

impl Default for PromptOptions {
    fn default() -> Self {
        PromptOptions {
            language: "English".to_owned(),
        }
    }
}

const INSTRUCTIONS: &str = "\
You turn compressed input into the message the user meant to write. The user is a developer talking to an AI coding assistant.

Input format:
- UPPERCASE tokens are concepts. Expand them into natural wording. A token can act as any part of speech.
- \"Quoted\" words are literal. Keep their spelling and capitals, except that a sentence starts with a capital. The quotes only mark them: never put quote marks in your reply.
- The input leaves out articles (a, an, the) and forms of \"be\" (am, is, are, was, were). Put them back wherever English needs them, next to quoted words too.
- A \"Meanings\" list, when present, says what a token stands for.

Rules:
- Reply with the finished message only, with no notes.
- Say everything the input says and nothing it does not.
- If the input opens with a question word (WHAT, WHY, HOW, WHEN, WHERE, WHICH, WHO), or with CAN, DO, HAVE, WILL or WOULD followed by a person, write a question.
- Keep it short and direct, the way a developer writes to a colleague.
- Tone tokens such as FORMAL, CASUAL, POLITE set the style. Do not write them out.
- Earlier messages are context. Use them to resolve words like IT, THIS, SAME and AGAIN.";

/// Worked examples, sent as earlier turns so small models copy the pattern.
const EXAMPLES: [(&str, &str); 5] = [
    ("FIX BUG IN \"login\"", "Fix the bug in login."),
    ("WHY TEST FAILING", "Why is the test failing?"),
    (
        "I \"stuck\" ON \"migration\"",
        "I'm stuck on the migration.",
    ),
    (
        "RENAME \"foo\" TO \"bar\" WITH_TESTS\n\nMeanings:\nWITH_TESTS = add or update tests that cover this",
        "Rename foo to bar, and add or update tests that cover it.",
    ),
    (
        "POLITE CAN YOU EXPLAIN HOW THIS FUNCTION WORKING",
        "Could you please explain how this function works?",
    ),
];

/// The user message for an utterance: its wire form, then the meaning of any
/// clause-sized concepts it uses.
pub fn user_message(utterance: &Utterance) -> String {
    let mut text = utterance.to_string();
    let glosses = utterance.glosses();
    if !glosses.is_empty() {
        text.push_str("\n\nMeanings:");
        for (token, gloss) in glosses {
            text.push('\n');
            text.push_str(token);
            text.push_str(" = ");
            text.push_str(gloss);
        }
    }
    text
}

/// Builds the chat messages that ask a model to decode an utterance.
///
/// The order is: instructions, worked examples, the given history as real
/// turns, then the utterance itself. Pass only as much history as is worth
/// its tokens; a few recent turns are enough to resolve "it" and "again".
pub fn messages(utterance: &Utterance, history: &[Turn], options: &PromptOptions) -> Vec<Message> {
    let mut system = INSTRUCTIONS.to_owned();
    let language = options.language.trim();
    if language.is_empty() || language.eq_ignore_ascii_case("english") {
        system.push_str("\n- Write in English.");
    } else {
        system.push_str("\n- Write in ");
        system.push_str(language);
        system.push_str(". The examples are in English, but your reply must be in ");
        system.push_str(language);
        system.push('.');
    }

    let mut messages = Vec::with_capacity(2 + 2 * (EXAMPLES.len() + history.len()));
    messages.push(Message {
        role: Role::System,
        content: system,
    });
    let examples = EXAMPLES
        .iter()
        .map(|(input, output)| ((*input).to_owned(), (*output).to_owned()));
    let earlier = history
        .iter()
        .map(|turn| (turn.utterance.clone(), turn.text.clone()));
    for (input, output) in examples.chain(earlier) {
        messages.push(Message {
            role: Role::User,
            content: input,
        });
        messages.push(Message {
            role: Role::Assistant,
            content: output,
        });
    }
    messages.push(Message {
        role: Role::User,
        content: user_message(utterance),
    });
    messages
}

/// Tidies a model's reply: trims it and removes one pair of wrapping quotes.
///
/// Models sometimes quote their answer despite being told not to. Quotes
/// that are part of the sentence are left alone.
pub fn clean(reply: &str) -> String {
    let trimmed = reply.trim();
    for (open, close) in [('"', '"'), ('\u{201c}', '\u{201d}'), ('\'', '\'')] {
        if let Some(inner) = trimmed
            .strip_prefix(open)
            .and_then(|rest| rest.strip_suffix(close))
            && !inner.is_empty()
            && !inner.contains(open)
            && !inner.contains(close)
        {
            return inner.trim().to_owned();
        }
    }
    trimmed.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Concept;
    use crate::engine::Item;

    fn concept(token: &str, gloss: Option<&str>) -> Item {
        Item::Concept(Concept {
            token: token.to_owned(),
            gloss: gloss.map(str::to_owned),
        })
    }

    fn word(text: &str) -> Item {
        Item::Word(text.to_owned())
    }

    fn utterance(items: Vec<Item>) -> Utterance {
        Utterance { items }
    }

    #[test]
    fn user_message_is_the_wire_form() {
        let utterance = utterance(vec![concept("FIX", None), word("login")]);
        assert_eq!(user_message(&utterance), "FIX \"login\"");
    }

    #[test]
    fn user_message_explains_clause_sized_concepts_once() {
        let utterance = utterance(vec![
            concept("FIX", None),
            concept("WITH_TESTS", Some("add tests")),
            concept("BRIEF", Some("keep it short")),
            concept("WITH_TESTS", Some("add tests")),
        ]);
        assert_eq!(
            user_message(&utterance),
            "FIX WITH_TESTS BRIEF WITH_TESTS\n\nMeanings:\nWITH_TESTS = add tests\nBRIEF = keep it short"
        );
    }

    #[test]
    fn messages_run_instructions_examples_history_then_the_utterance() {
        let history = [Turn {
            utterance: "ADD TEST".into(),
            text: "Add a test.".into(),
        }];
        let built = messages(
            &utterance(vec![concept("AGAIN", None)]),
            &history,
            &PromptOptions::default(),
        );

        assert_eq!(built.len(), 1 + 2 * EXAMPLES.len() + 2 + 1);
        assert_eq!(built[0].role, Role::System);
        assert!(built[0].content.starts_with("You turn compressed input"));
        assert!(built[0].content.ends_with("- Write in English."));

        let roles: Vec<Role> = built[1..].iter().map(|message| message.role).collect();
        for (index, role) in roles.iter().enumerate() {
            let expected = if index % 2 == 0 {
                Role::User
            } else {
                Role::Assistant
            };
            assert_eq!(*role, expected, "message {}", index + 1);
        }

        let last = built.len() - 1;
        assert_eq!(built[last - 2].content, "ADD TEST");
        assert_eq!(built[last - 1].content, "Add a test.");
        assert_eq!(built[last].content, "AGAIN");
        assert_eq!(built[last].role, Role::User);
    }

    #[test]
    fn examples_use_the_same_format_as_real_input() {
        let gloss_example = EXAMPLES[3].0;
        let same = utterance(vec![
            concept("RENAME", None),
            word("foo"),
            concept("TO", None),
            word("bar"),
            concept("WITH_TESTS", Some("add or update tests that cover this")),
        ]);
        assert_eq!(user_message(&same), gloss_example);
    }

    #[test]
    fn another_language_is_asked_for_explicitly() {
        let options = PromptOptions {
            language: " Portuguese ".into(),
        };
        let built = messages(&utterance(vec![concept("OK", None)]), &[], &options);
        assert!(built[0].content.ends_with(
            "- Write in Portuguese. The examples are in English, but your reply must be in Portuguese."
        ));
    }

    #[test]
    fn a_blank_or_english_language_means_english() {
        for language in ["", "  ", "english", "English"] {
            let options = PromptOptions {
                language: language.into(),
            };
            let built = messages(&utterance(vec![concept("OK", None)]), &[], &options);
            assert!(
                built[0].content.ends_with("- Write in English."),
                "{language:?}"
            );
        }
    }

    fn system_prompt() -> String {
        messages(
            &utterance(vec![concept("OK", None)]),
            &[],
            &PromptOptions::default(),
        )[0]
        .content
        .clone()
    }

    // The three rules below come from the round trip in docs/COVERAGE.md,
    // where the decoder broke each of them.

    #[test]
    fn instructions_ask_to_restore_dropped_articles_and_be() {
        let system = system_prompt();
        assert!(system.contains("articles (a, an, the)"), "{system}");
        assert!(system.contains("forms of \"be\""), "{system}");
        assert!(system.contains("next to quoted words too"), "{system}");
    }

    #[test]
    fn instructions_forbid_quote_marks_in_the_reply() {
        assert!(system_prompt().contains("never put quote marks in your reply"));
    }

    #[test]
    fn instructions_say_when_to_write_a_question() {
        let system = system_prompt();
        assert!(system.contains("write a question"), "{system}");
        assert!(
            system.contains("WHAT, WHY, HOW, WHEN, WHERE, WHICH, WHO"),
            "{system}"
        );
    }

    #[test]
    fn an_example_restores_be_next_to_a_literal_word() {
        let shown = EXAMPLES.iter().any(|(input, output)| {
            input.contains('"')
                && !input.contains("BE")
                && (output.contains(" am ") || output.contains("'m ") || output.contains(" are "))
        });
        assert!(shown, "no example puts back a dropped form of be");
    }

    #[test]
    fn the_prompt_stays_small() {
        let built = messages(
            &utterance(vec![concept("FIX", None)]),
            &[],
            &PromptOptions::default(),
        );
        let characters: usize = built.iter().map(|message| message.content.len()).sum();
        // Roughly four characters per token: keep the fixed cost near 500 tokens.
        assert!(characters < 2100, "prompt grew to {characters} characters");
    }

    #[test]
    fn role_names_match_chat_apis() {
        assert_eq!(Role::System.name(), "system");
        assert_eq!(Role::User.name(), "user");
        assert_eq!(Role::Assistant.name(), "assistant");
    }

    #[test]
    fn clean_trims_and_unwraps_one_pair_of_quotes() {
        for (reply, expected) in [
            ("  Fix the bug.\n", "Fix the bug."),
            ("\"Fix the bug.\"", "Fix the bug."),
            ("\u{201c}Fix the bug.\u{201d}", "Fix the bug."),
            ("'Fix the bug.'", "Fix the bug."),
            ("\" Fix the bug. \"", "Fix the bug."),
            ("Rename \"foo\" to \"bar\".", "Rename \"foo\" to \"bar\"."),
            ("\"foo\" and \"bar\"", "\"foo\" and \"bar\""),
            ("\"\"", "\"\""),
            ("\"", "\""),
            ("", ""),
        ] {
            assert_eq!(clean(reply), expected, "reply {reply:?}");
        }
    }
}
