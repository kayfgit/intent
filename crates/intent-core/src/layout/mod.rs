//! Layouts: the data that gives chords their meaning.
//!
//! A layout is a TOML file. It never spells out chords by hand. Instead it
//! lists tokens in priority order and the format assigns shapes from easiest
//! to hardest (see [`crate::effort`]). That makes two classes of bug
//! impossible: two tokens on one chord, and a common token stuck on an awkward
//! chord while an easy one sits unused.
//!
//! ```toml
//! [layout]
//! name = "example"
//! version = "0.1.0"
//!
//! [thumbs]              # thumb-only chords hold commands
//! left = "undo"
//! right = "space"
//! both = "send"
//!
//! [spelling]            # one-hand chords spell; both hands get the same map
//! keys = "eaoi"         # priority order; a space leaves a slot free
//!
//! [[family]]            # two-hand chords: left hand picks the family,
//! id = "act"            # right hand picks the member
//! plane = "core"        # core | extended | literal | user
//! left = "I"
//! members = ["MAKE", { token = "FIX", gloss = "repair what is broken" }]
//! ```

mod compile;
mod spec;

use std::collections::HashMap;
use std::fmt;

use crate::action::{Action, Command};
use crate::chord::{Chord, HandShape, Plane};

/// A validated layout: every chord has at most one meaning.
#[derive(Debug, Clone)]
pub struct Layout {
    meta: Meta,
    families: Vec<Family>,
    entries: Vec<Entry>,
    by_chord: HashMap<Chord, usize>,
    by_token: HashMap<String, usize>,
    by_text: HashMap<String, Vec<usize>>,
    by_command: HashMap<Command, Vec<usize>>,
}

/// A layout that compiled, plus anything worth a second look.
#[derive(Debug, Clone)]
pub struct Compiled {
    /// The layout, ready to use.
    pub layout: Layout,
    /// Problems that do not stop the layout from working.
    pub warnings: Vec<Diagnostic>,
}

/// Name and version of a layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    /// Short name, such as `intent-v0`.
    pub name: String,
    /// Version of the layout itself.
    pub version: String,
    /// One-line description.
    pub description: String,
}

/// A group of two-hand chords that share a left-hand shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    /// Short identifier, unique within the layout.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The plane the family lives in.
    pub plane: Plane,
    /// What kind of action its members are.
    pub kind: Kind,
    /// The left-hand finger shape that selects this family.
    pub left: HandShape,
}

/// The kind of action a family's members produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Units of meaning for the decoder.
    Concept,
    /// Literal characters.
    Text,
    /// Commands.
    Command,
}

/// One chord and what it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The chord.
    pub chord: Chord,
    /// What the chord does.
    pub action: Action,
    /// Which part of the layout file defined it.
    pub origin: Origin,
}

/// The part of a layout file an entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The `[thumbs]` table.
    Thumbs,
    /// The `[spelling]` table.
    Spelling,
    /// The family at this index of [`Layout::families`].
    Family(usize),
}

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// The layout cannot be used.
    Error,
    /// The layout works but something looks off.
    Warning,
}

/// One problem found in a layout file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// How serious it is.
    pub severity: Severity,
    /// What is wrong, in plain words.
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{label}: {}", self.message)
    }
}

/// A layout file that cannot be used, with every problem found in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutError {
    /// All problems, errors and warnings alike, in file order.
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LayoutError {}

impl Layout {
    /// Parses and validates a layout file.
    ///
    /// All problems are collected in one pass, so a broken file reports
    /// everything that is wrong with it at once.
    pub fn from_toml(source: &str) -> Result<Compiled, LayoutError> {
        compile::compile(source)
    }

    fn new(meta: Meta, families: Vec<Family>, entries: Vec<Entry>) -> Layout {
        let mut by_chord = HashMap::new();
        let mut by_token = HashMap::new();
        let mut by_text: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_command: HashMap<Command, Vec<usize>> = HashMap::new();
        for (index, entry) in entries.iter().enumerate() {
            by_chord.insert(entry.chord, index);
            match &entry.action {
                Action::Concept(concept) => {
                    by_token.insert(concept.token.clone(), index);
                }
                Action::Text(text) => by_text.entry(text.clone()).or_default().push(index),
                Action::Command(command) => by_command.entry(*command).or_default().push(index),
            }
        }
        Layout {
            meta,
            families,
            entries,
            by_chord,
            by_token,
            by_text,
            by_command,
        }
    }

    /// Name and version.
    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    /// All families, in file order.
    pub fn families(&self) -> &[Family] {
        &self.families
    }

    /// Every assigned chord, in ascending chord order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// What a chord does, if anything.
    pub fn lookup(&self, chord: Chord) -> Option<&Entry> {
        self.by_chord.get(&chord).map(|index| &self.entries[*index])
    }

    /// The chord for a concept token.
    pub fn chord_for_token(&self, token: &str) -> Option<Chord> {
        self.by_token
            .get(token)
            .map(|index| self.entries[*index].chord)
    }

    /// Every chord that types the given text, in ascending chord order.
    ///
    /// Letters usually have two: one per hand.
    pub fn chords_for_text(&self, text: &str) -> Vec<Chord> {
        self.chords(self.by_text.get(text))
    }

    /// Every chord that triggers the given command, in ascending chord order.
    pub fn chords_for_command(&self, command: Command) -> Vec<Chord> {
        self.chords(self.by_command.get(&command))
    }

    fn chords(&self, indexes: Option<&Vec<usize>>) -> Vec<Chord> {
        indexes
            .into_iter()
            .flatten()
            .map(|index| self.entries[*index].chord)
            .collect()
    }

    /// Every entry reachable by pressing more keys on top of `held`.
    ///
    /// Hosts use this for live hints: hold a family's left-hand shape and see
    /// its members.
    pub fn completions(&self, held: Chord) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(move |entry| entry.chord.is_superset_of(held))
    }

    /// The family an entry belongs to, if it came from one.
    pub fn family_of(&self, entry: &Entry) -> Option<&Family> {
        match entry.origin {
            Origin::Family(index) => self.families.get(index),
            Origin::Thumbs | Origin::Spelling => None,
        }
    }

    /// How many chords of a plane are assigned.
    pub fn plane_usage(&self, plane: Plane) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.chord.plane() == Some(plane))
            .count()
    }
}
