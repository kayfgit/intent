//! What a chord does once it is recognised.

use std::fmt;

/// The meaning a layout gives to one chord.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    /// A unit of meaning for the decoder to expand, such as `FIX`.
    Concept(Concept),
    /// Literal characters, such as a letter, a digit or a symbol.
    Text(String),
    /// Something to do rather than something to say.
    Command(Command),
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Concept(concept) => f.write_str(&concept.token),
            Action::Text(text) => write!(f, "\"{text}\""),
            Action::Command(command) => write!(f, "<{command}>"),
        }
    }
}

/// A unit of meaning. The decoder turns a run of concepts into fluent text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Concept {
    /// The uppercase name, unique within a layout.
    pub token: String,
    /// An optional plain-language explanation passed to the decoder.
    ///
    /// One-word concepts rarely need one. Concepts that stand for a whole
    /// clause, such as `WITH_TESTS`, do.
    pub gloss: Option<String>,
}

/// A command a chord can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Command {
    /// End the word being spelled.
    Space,
    /// Take back the last stroke.
    Undo,
    /// Hand the buffer to the decoder.
    Send,
    /// Throw the buffer away.
    Clear,
    /// The Enter key.
    Enter,
    /// The Tab key.
    Tab,
    /// The Escape key.
    Escape,
    /// The Backspace key.
    Backspace,
    /// The Delete key.
    Delete,
    /// The Left arrow key.
    Left,
    /// The Right arrow key.
    Right,
    /// The Up arrow key.
    Up,
    /// The Down arrow key.
    Down,
    /// The Home key.
    Home,
    /// The End key.
    End,
    /// Move one word to the left.
    WordLeft,
    /// Move one word to the right.
    WordRight,
    /// The Page Up key.
    PageUp,
    /// The Page Down key.
    PageDown,
}

impl Command {
    /// Every command.
    pub const ALL: [Command; 19] = [
        Command::Space,
        Command::Undo,
        Command::Send,
        Command::Clear,
        Command::Enter,
        Command::Tab,
        Command::Escape,
        Command::Backspace,
        Command::Delete,
        Command::Left,
        Command::Right,
        Command::Up,
        Command::Down,
        Command::Home,
        Command::End,
        Command::WordLeft,
        Command::WordRight,
        Command::PageUp,
        Command::PageDown,
    ];

    /// The name used in layout files.
    pub const fn name(self) -> &'static str {
        match self {
            Command::Space => "space",
            Command::Undo => "undo",
            Command::Send => "send",
            Command::Clear => "clear",
            Command::Enter => "enter",
            Command::Tab => "tab",
            Command::Escape => "escape",
            Command::Backspace => "backspace",
            Command::Delete => "delete",
            Command::Left => "left",
            Command::Right => "right",
            Command::Up => "up",
            Command::Down => "down",
            Command::Home => "home",
            Command::End => "end",
            Command::WordLeft => "word-left",
            Command::WordRight => "word-right",
            Command::PageUp => "page-up",
            Command::PageDown => "page-down",
        }
    }

    /// Looks a command up by its layout-file name.
    pub fn from_name(name: &str) -> Option<Command> {
        Command::ALL
            .into_iter()
            .find(|command| command.name() == name)
    }

    /// Whether this command edits Intent's own buffer.
    ///
    /// The other commands are plain keys that the host passes on to whatever
    /// application has focus.
    pub const fn edits_buffer(self) -> bool {
        matches!(
            self,
            Command::Space | Command::Undo | Command::Send | Command::Clear
        )
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn command_names_are_unique_and_round_trip() {
        let names: HashSet<&str> = Command::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(names.len(), Command::ALL.len());
        for command in Command::ALL {
            assert_eq!(Command::from_name(command.name()), Some(command));
        }
        assert_eq!(Command::from_name("Send"), None);
        assert_eq!(Command::from_name("nope"), None);
    }

    #[test]
    fn only_four_commands_edit_the_buffer() {
        let editing: Vec<Command> = Command::ALL
            .into_iter()
            .filter(|c| c.edits_buffer())
            .collect();
        assert_eq!(
            editing,
            [Command::Space, Command::Undo, Command::Send, Command::Clear]
        );
    }

    #[test]
    fn actions_display_distinctly() {
        let concept = Action::Concept(Concept {
            token: "FIX".into(),
            gloss: None,
        });
        assert_eq!(concept.to_string(), "FIX");
        assert_eq!(Action::Text("a".into()).to_string(), "\"a\"");
        assert_eq!(
            Action::Command(Command::WordLeft).to_string(),
            "<word-left>"
        );
    }
}
