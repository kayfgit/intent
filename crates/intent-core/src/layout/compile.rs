//! Validates a layout file and assigns every token its chord.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::spec::{FamilySpec, KindSpec, LayoutSpec, MemberSpec, SpellingSpec, ThumbsSpec};
use super::{
    Compiled, Diagnostic, Entry, Family, Kind, Layout, LayoutError, Meta, Origin, Severity,
};
use crate::action::{Action, Command, Concept};
use crate::chord::{Chord, HandShape, Plane};
use crate::effort::{finger_shape_order, spell_shape_order};
use crate::key::{Finger, Hand, Key};

pub(super) fn compile(source: &str) -> Result<Compiled, LayoutError> {
    let spec: LayoutSpec = toml::from_str(source).map_err(|error| LayoutError {
        diagnostics: vec![Diagnostic {
            severity: Severity::Error,
            message: format!("not a valid layout file: {}", error.to_string().trim_end()),
        }],
    })?;

    let mut builder = Builder::default();
    if spec.layout.name.trim().is_empty() {
        builder.error("layout.name is empty".to_owned());
    }
    builder.thumbs(spec.thumbs.as_ref());
    builder.spelling(spec.spelling.as_ref());
    for family in &spec.families {
        builder.family(family);
    }
    builder.finish(Meta {
        name: spec.layout.name,
        version: spec.layout.version,
        description: spec.layout.description,
    })
}

#[derive(Default)]
struct Builder {
    diagnostics: Vec<Diagnostic>,
    families: Vec<Family>,
    entries: BTreeMap<Chord, Entry>,
    /// Concept token to the id of the family that defined it.
    tokens: HashMap<String, String>,
    family_ids: HashSet<String>,
    /// Plane and left shape to the id of the family that claimed them.
    family_slots: HashMap<(Plane, HandShape), String>,
}

impl Builder {
    fn error(&mut self, message: String) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message,
        });
    }

    fn warning(&mut self, message: String) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            message,
        });
    }

    /// The planes are disjoint and each section fills its own, so a slot can
    /// never be taken twice once the per-section checks have passed.
    fn place(&mut self, chord: Chord, action: Action, origin: Origin) {
        let previous = self.entries.insert(
            chord,
            Entry {
                chord,
                action,
                origin,
            },
        );
        debug_assert!(previous.is_none(), "chord {chord} assigned twice");
    }

    fn thumbs(&mut self, spec: Option<&ThumbsSpec>) {
        let Some(spec) = spec else { return };
        let left = Key::new(Hand::Left, Finger::Thumb);
        let right = Key::new(Hand::Right, Finger::Thumb);
        let slots = [
            ("left", &spec.left, Chord::EMPTY.with(left)),
            ("right", &spec.right, Chord::EMPTY.with(right)),
            ("both", &spec.both, Chord::EMPTY.with(left).with(right)),
        ];
        for (name, value, chord) in slots {
            let Some(value) = value else { continue };
            match Command::from_name(value) {
                Some(command) => self.place(chord, Action::Command(command), Origin::Thumbs),
                None => self.error(format!(
                    "thumbs.{name}: unknown command \"{value}\" (known commands: {})",
                    command_names()
                )),
            }
        }
    }

    fn spelling(&mut self, spec: Option<&SpellingSpec>) {
        let Some(spec) = spec else { return };
        let order = spell_shape_order();
        let count = spec.keys.chars().count();
        if count > order.len() {
            self.error(format!(
                "spelling.keys has {count} characters, but one hand has only {} spelling shapes",
                order.len()
            ));
        }

        let mut seen = HashSet::new();
        for (character, shape) in spec.keys.chars().zip(order) {
            if character == ' ' {
                continue;
            }
            if !seen.insert(character) {
                self.error(format!("spelling.keys lists '{character}' more than once"));
                continue;
            }
            let shifted = shifted(spec, character);
            for hand in Hand::ALL {
                let chord = Chord::of_hand(hand, shape);
                self.place(chord, Action::Text(character.to_string()), Origin::Spelling);
                if let Some(shifted) = &shifted {
                    let opposite_thumb = Key::new(hand.other(), Finger::Thumb);
                    self.place(
                        chord.with(opposite_thumb),
                        Action::Text(shifted.clone()),
                        Origin::Spelling,
                    );
                }
            }
        }

        for (base, replacement) in &spec.shift {
            let mut characters = base.chars();
            let known = match (characters.next(), characters.next()) {
                (Some(character), None) => seen.contains(&character),
                _ => false,
            };
            if !known {
                self.error(format!(
                    "spelling.shift: \"{base}\" is not one of the characters in spelling.keys"
                ));
            } else if replacement.is_empty() {
                self.error(format!(
                    "spelling.shift: the shifted form of \"{base}\" is empty"
                ));
            }
        }

        let missing: String = ('a'..='z')
            .filter(|letter| !seen.contains(letter))
            .collect();
        if !missing.is_empty() {
            self.warning(format!(
                "spelling.keys has no chord for: {missing} (words with these letters cannot be spelled)"
            ));
        }
    }

    fn family(&mut self, spec: &FamilySpec) {
        if spec.id.trim().is_empty() {
            self.error("a family has an empty id".to_owned());
            return;
        }
        let label = format!("family \"{}\"", spec.id);
        if !self.family_ids.insert(spec.id.clone()) {
            self.error(format!("{label}: this id is used by more than one family"));
            return;
        }
        let plane = match spec.plane.as_str() {
            "core" => Plane::Core,
            "extended" => Plane::Extended,
            "literal" => Plane::Literal,
            "user" => Plane::User,
            other => {
                self.error(format!(
                    "{label}: unknown plane \"{other}\" (expected core, extended, literal or user)"
                ));
                return;
            }
        };
        let left: HandShape = match spec.left.parse() {
            Ok(shape) => shape,
            Err(error) => {
                self.error(format!("{label}: left shape \"{}\": {error}", spec.left));
                return;
            }
        };
        if left.is_empty() {
            self.error(format!("{label}: left shape is empty"));
            return;
        }
        if left.has_thumb() {
            self.error(format!(
                "{label}: left shape \"{}\" includes the thumb, but thumbs are set by the plane",
                spec.left
            ));
            return;
        }
        if let Some(owner) = self.family_slots.get(&(plane, left)) {
            let owner = owner.clone();
            self.error(format!(
                "{label}: left shape {left} in the {plane} plane is already used by family \"{owner}\""
            ));
            return;
        }
        self.family_slots.insert((plane, left), spec.id.clone());

        let order = finger_shape_order();
        if spec.members.len() > order.len() {
            self.error(format!(
                "{label}: {} members, but a family holds at most {}",
                spec.members.len(),
                order.len()
            ));
            return;
        }
        if spec.members.iter().all(MemberSpec::is_blank) {
            self.warning(format!("{label}: has no members"));
        }

        let index = self.families.len();
        self.families.push(Family {
            id: spec.id.clone(),
            name: spec.name.clone().unwrap_or_else(|| spec.id.clone()),
            plane,
            kind: match spec.kind {
                KindSpec::Concept => Kind::Concept,
                KindSpec::Text => Kind::Text,
                KindSpec::Command => Kind::Command,
            },
            left,
        });

        let (left_thumb, right_thumb) = plane.thumbs().expect("family planes are two-hand planes");
        let left = if left_thumb {
            left.with(Finger::Thumb)
        } else {
            left
        };
        for (position, (member, right)) in spec.members.iter().zip(order).enumerate() {
            if member.is_blank() {
                continue;
            }
            let location = format!("{label}, member {}", position + 1);
            let Some(action) = self.member_action(&location, &spec.id, spec.kind, member) else {
                continue;
            };
            let right = if right_thumb {
                right.with(Finger::Thumb)
            } else {
                right
            };
            self.place(
                Chord::from_hands(left, right),
                action,
                Origin::Family(index),
            );
        }
    }

    fn member_action(
        &mut self,
        location: &str,
        family_id: &str,
        kind: KindSpec,
        member: &MemberSpec,
    ) -> Option<Action> {
        match (kind, member) {
            (KindSpec::Concept, _) => {
                let (token, gloss) = match member {
                    MemberSpec::Simple(token) => (token, None),
                    MemberSpec::Detailed(detailed) => (&detailed.token, detailed.gloss.clone()),
                };
                if !is_token(token) {
                    self.error(format!(
                        "{location}: \"{token}\" is not a valid token (use UPPERCASE letters, digits and _, starting with a letter)"
                    ));
                    return None;
                }
                if let Some(owner) = self.tokens.get(token) {
                    let message = if owner == family_id {
                        format!("{location}: token {token} is listed twice in this family")
                    } else {
                        format!(
                            "{location}: token {token} is already defined in family \"{owner}\""
                        )
                    };
                    self.error(message);
                    return None;
                }
                self.tokens.insert(token.clone(), family_id.to_owned());
                Some(Action::Concept(Concept {
                    token: token.clone(),
                    gloss,
                }))
            }
            (KindSpec::Text, MemberSpec::Simple(text)) => Some(Action::Text(text.clone())),
            (KindSpec::Command, MemberSpec::Simple(name)) => {
                let command = Command::from_name(name);
                if command.is_none() {
                    self.error(format!(
                        "{location}: unknown command \"{name}\" (known commands: {})",
                        command_names()
                    ));
                }
                command.map(Action::Command)
            }
            (KindSpec::Text | KindSpec::Command, MemberSpec::Detailed(_)) => {
                self.error(format!(
                    "{location}: only concept families take {{ token, gloss }} members; use a plain string here"
                ));
                None
            }
        }
    }

    fn finish(self, meta: Meta) -> Result<Compiled, LayoutError> {
        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Err(LayoutError {
                diagnostics: self.diagnostics,
            });
        }
        let entries = self.entries.into_values().collect();
        Ok(Compiled {
            layout: Layout::new(meta, self.families, entries),
            warnings: self.diagnostics,
        })
    }
}

/// The shifted form of a spelling character: an explicit override, or its
/// uppercase form when it has one.
fn shifted(spec: &SpellingSpec, character: char) -> Option<String> {
    let base = character.to_string();
    if let Some(replacement) = spec.shift.get(&base) {
        return (!replacement.is_empty()).then(|| replacement.clone());
    }
    let upper: String = character.to_uppercase().collect();
    (upper != base).then_some(upper)
}

fn is_token(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
        && characters.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn command_names() -> String {
    Command::ALL.map(Command::name).join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "[layout]\nname = \"test\"\nversion = \"0.0.0\"\n";

    fn compiled(body: &str) -> Compiled {
        match Layout::from_toml(&format!("{HEADER}{body}")) {
            Ok(compiled) => compiled,
            Err(error) => panic!("layout should compile, but:\n{error}"),
        }
    }

    fn layout(body: &str) -> Layout {
        let compiled = compiled(body);
        assert_eq!(compiled.warnings, [], "unexpected warnings");
        compiled.layout
    }

    fn errors(body: &str) -> Vec<String> {
        let error = Layout::from_toml(&format!("{HEADER}{body}")).expect_err("layout should fail");
        error
            .diagnostics
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message)
            .collect()
    }

    fn chord(text: &str) -> Chord {
        text.parse().expect("valid chord")
    }

    fn concept(layout: &Layout, text: &str) -> Option<String> {
        match &layout.lookup(chord(text))?.action {
            Action::Concept(concept) => Some(concept.token.clone()),
            _ => None,
        }
    }

    fn text(layout: &Layout, chord_text: &str) -> Option<String> {
        match &layout.lookup(chord(chord_text))?.action {
            Action::Text(text) => Some(text.clone()),
            _ => None,
        }
    }

    const FULL_ALPHABET: &str = "[spelling]\nkeys = \"eaoistnrdumlchgpfvbwykqjxz\"\n";

    #[test]
    fn empty_layout_compiles() {
        let layout = layout("");
        assert_eq!(layout.meta().name, "test");
        assert!(layout.entries().is_empty());
        assert!(layout.families().is_empty());
    }

    #[test]
    fn members_get_right_hand_shapes_from_easiest_to_hardest() {
        let layout = layout(
            r#"
            [[family]]
            id = "act"
            name = "Actions"
            plane = "core"
            left = "I"
            members = ["MAKE", "FIX", "CHANGE", "ADD", "REMOVE"]
            "#,
        );
        assert_eq!(concept(&layout, "I-I").as_deref(), Some("MAKE"));
        assert_eq!(concept(&layout, "I-M").as_deref(), Some("FIX"));
        assert_eq!(concept(&layout, "I-R").as_deref(), Some("CHANGE"));
        assert_eq!(concept(&layout, "I-P").as_deref(), Some("ADD"));
        assert_eq!(concept(&layout, "I-IM").as_deref(), Some("REMOVE"));
        assert_eq!(layout.lookup(chord("I-MR")), None);
        assert_eq!(layout.chord_for_token("FIX"), Some(chord("I-M")));
        assert_eq!(layout.chord_for_token("NOPE"), None);

        let family = &layout.families()[0];
        assert_eq!(family.name, "Actions");
        assert_eq!(family.kind, Kind::Concept);
        let entry = layout.lookup(chord("I-I")).expect("entry");
        assert_eq!(layout.family_of(entry), Some(family));
    }

    #[test]
    fn plane_adds_its_thumbs_to_every_member() {
        let layout = layout(
            r#"
            [[family]]
            id = "a"
            plane = "core"
            left = "M"
            members = ["A"]
            [[family]]
            id = "b"
            plane = "extended"
            left = "M"
            members = ["B"]
            [[family]]
            id = "c"
            plane = "literal"
            left = "M"
            members = ["C"]
            [[family]]
            id = "d"
            plane = "user"
            left = "M"
            members = ["D"]
            "#,
        );
        assert_eq!(concept(&layout, "M-I").as_deref(), Some("A"));
        assert_eq!(concept(&layout, "M-TI").as_deref(), Some("B"));
        assert_eq!(concept(&layout, "MT-I").as_deref(), Some("C"));
        assert_eq!(concept(&layout, "MT-TI").as_deref(), Some("D"));
        for plane in [Plane::Core, Plane::Extended, Plane::Literal, Plane::User] {
            assert_eq!(layout.plane_usage(plane), 1);
        }
    }

    #[test]
    fn family_name_defaults_to_its_id() {
        let layout = layout(
            "[[family]]\nid = \"act\"\nplane = \"core\"\nleft = \"I\"\nmembers = [\"GO\"]\n",
        );
        assert_eq!(layout.families()[0].name, "act");
    }

    #[test]
    fn blank_member_leaves_its_slot_free() {
        let layout = layout(
            "[[family]]\nid = \"act\"\nplane = \"core\"\nleft = \"I\"\nmembers = [\"MAKE\", \"\", \"CHANGE\"]\n",
        );
        assert_eq!(concept(&layout, "I-I").as_deref(), Some("MAKE"));
        assert_eq!(layout.lookup(chord("I-M")), None);
        assert_eq!(concept(&layout, "I-R").as_deref(), Some("CHANGE"));
    }

    #[test]
    fn glosses_are_kept() {
        let layout = layout(
            r#"
            [[family]]
            id = "directive"
            plane = "core"
            left = "I"
            members = [{ token = "WITH_TESTS", gloss = "add tests" }, { token = "BRIEF" }]
            "#,
        );
        let entry = layout.lookup(chord("I-I")).expect("entry");
        assert_eq!(
            entry.action,
            Action::Concept(Concept {
                token: "WITH_TESTS".into(),
                gloss: Some("add tests".into()),
            })
        );
        assert_eq!(concept(&layout, "I-M").as_deref(), Some("BRIEF"));
    }

    #[test]
    fn text_and_command_families() {
        let layout = layout(
            r#"
            [[family]]
            id = "digits"
            plane = "literal"
            kind = "text"
            left = "I"
            members = ["1", "2"]
            [[family]]
            id = "keys"
            plane = "literal"
            kind = "command"
            left = "M"
            members = ["left", "word-right"]
            "#,
        );
        assert_eq!(text(&layout, "IT-I").as_deref(), Some("1"));
        assert_eq!(text(&layout, "IT-M").as_deref(), Some("2"));
        assert_eq!(layout.chords_for_text("2"), [chord("IT-M")]);
        assert_eq!(layout.chords_for_command(Command::Left), [chord("MT-I")]);
        assert_eq!(
            layout.chords_for_command(Command::WordRight),
            [chord("MT-M")]
        );
        assert_eq!(layout.chords_for_command(Command::Send), []);
        assert_eq!(layout.families()[0].kind, Kind::Text);
        assert_eq!(layout.families()[1].kind, Kind::Command);
    }

    #[test]
    fn thumbs_hold_commands() {
        let layout = layout("[thumbs]\nleft = \"undo\"\nright = \"space\"\nboth = \"send\"\n");
        assert_eq!(layout.chords_for_command(Command::Undo), [chord("T-")]);
        assert_eq!(layout.chords_for_command(Command::Space), [chord("-T")]);
        assert_eq!(layout.chords_for_command(Command::Send), [chord("T-T")]);
        assert_eq!(layout.plane_usage(Plane::Thumbs), 3);
        let entry = layout.lookup(chord("T-")).expect("entry");
        assert_eq!(entry.origin, Origin::Thumbs);
        assert_eq!(layout.family_of(entry), None);
    }

    #[test]
    fn spelling_is_the_same_on_both_hands() {
        let layout = layout(FULL_ALPHABET);
        assert_eq!(text(&layout, "I-").as_deref(), Some("e"));
        assert_eq!(text(&layout, "-I").as_deref(), Some("e"));
        assert_eq!(text(&layout, "M-").as_deref(), Some("a"));
        assert_eq!(text(&layout, "IT-").as_deref(), Some("s"));
        assert_eq!(text(&layout, "-TI").as_deref(), Some("s"));
        assert_eq!(layout.chords_for_text("e"), [chord("I-"), chord("-I")]);
        assert_eq!(layout.plane_usage(Plane::Spell(Hand::Left)), 26);
        assert_eq!(layout.plane_usage(Plane::Spell(Hand::Right)), 26);
    }

    #[test]
    fn opposite_thumb_shifts_to_uppercase() {
        let layout = layout(FULL_ALPHABET);
        assert_eq!(text(&layout, "I-T").as_deref(), Some("E"));
        assert_eq!(text(&layout, "T-I").as_deref(), Some("E"));
        assert_eq!(text(&layout, "IT-T").as_deref(), Some("S"));
        assert_eq!(layout.plane_usage(Plane::SpellShift(Hand::Left)), 26);
    }

    #[test]
    fn shift_overrides_and_unshiftable_characters() {
        let layout = layout(
            "[spelling]\nkeys = \"eaoistnrdumlchgpfvbwykqjxz.-9\"\n[spelling.shift]\n\".\" = \":\"\n",
        );
        assert_eq!(text(&layout, "PMIT-").as_deref(), Some("."));
        assert_eq!(text(&layout, "PMIT-T").as_deref(), Some(":"));
        // "-" and "9" have no uppercase and no override: their shifted slots stay free.
        assert_eq!(text(&layout, "PRMI-").as_deref(), Some("-"));
        assert_eq!(layout.lookup(chord("PRMI-T")), None);
        assert_eq!(layout.plane_usage(Plane::SpellShift(Hand::Left)), 27);
    }

    #[test]
    fn a_space_in_spelling_keys_skips_a_slot() {
        let compiled = compiled("[spelling]\nkeys = \"e o\"\n");
        assert_eq!(text(&compiled.layout, "I-").as_deref(), Some("e"));
        assert_eq!(compiled.layout.lookup(chord("M-")), None);
        assert_eq!(text(&compiled.layout, "R-").as_deref(), Some("o"));
    }

    #[test]
    fn missing_letters_are_a_warning_not_an_error() {
        let compiled = compiled("[spelling]\nkeys = \"abc\"\n");
        assert_eq!(compiled.warnings.len(), 1);
        assert_eq!(compiled.warnings[0].severity, Severity::Warning);
        assert!(
            compiled.warnings[0]
                .message
                .contains("defghijklmnopqrstuvwxyz")
        );
    }

    #[test]
    fn empty_family_is_a_warning() {
        let compiled = compiled("[[family]]\nid = \"x\"\nplane = \"core\"\nleft = \"I\"\n");
        assert_eq!(compiled.warnings.len(), 1);
        assert!(compiled.warnings[0].message.contains("has no members"));
    }

    #[test]
    fn completions_list_what_more_keys_would_reach() {
        let layout = layout(
            r#"
            [[family]]
            id = "act"
            plane = "core"
            left = "I"
            members = ["MAKE", "FIX"]
            [[family]]
            id = "thing"
            plane = "core"
            left = "M"
            members = ["THIS"]
            "#,
        );
        let held = chord("I-");
        let reachable: Vec<String> = layout
            .completions(held)
            .map(|e| e.action.to_string())
            .collect();
        assert_eq!(reachable, ["MAKE", "FIX"]);
        assert_eq!(layout.completions(Chord::EMPTY).count(), 3);
        assert_eq!(layout.completions(chord("P-")).count(), 0);
    }

    #[test]
    fn every_entry_sits_in_the_plane_its_section_owns() {
        let layout = layout(&format!(
            "{FULL_ALPHABET}[thumbs]\nboth = \"send\"\n[[family]]\nid = \"a\"\nplane = \"extended\"\nleft = \"IM\"\nmembers = [\"A\", \"B\"]\n"
        ));
        for entry in layout.entries() {
            let plane = entry.chord.plane().expect("entries are non-empty chords");
            match entry.origin {
                Origin::Thumbs => assert_eq!(plane, Plane::Thumbs),
                Origin::Spelling => {
                    assert!(matches!(plane, Plane::Spell(_) | Plane::SpellShift(_)));
                }
                Origin::Family(index) => assert_eq!(plane, layout.families()[index].plane),
            }
        }
    }

    #[test]
    fn duplicate_token_names_both_families() {
        let found = errors(
            r#"
            [[family]]
            id = "act"
            plane = "core"
            left = "I"
            members = ["RUN"]
            [[family]]
            id = "flow"
            plane = "core"
            left = "M"
            members = ["BUILD", "RUN"]
            "#,
        );
        assert_eq!(
            found,
            ["family \"flow\", member 2: token RUN is already defined in family \"act\""]
        );
    }

    #[test]
    fn duplicate_token_inside_one_family() {
        let found = errors(
            "[[family]]\nid = \"act\"\nplane = \"core\"\nleft = \"I\"\nmembers = [\"RUN\", \"RUN\"]\n",
        );
        assert_eq!(
            found,
            ["family \"act\", member 2: token RUN is listed twice in this family"]
        );
    }

    #[test]
    fn two_families_cannot_share_a_left_shape_in_one_plane() {
        let found = errors(
            r#"
            [[family]]
            id = "act"
            plane = "core"
            left = "IM"
            members = ["A"]
            [[family]]
            id = "other"
            plane = "core"
            left = "MI"
            members = ["B"]
            "#,
        );
        assert_eq!(
            found,
            ["family \"other\": left shape IM in the core plane is already used by family \"act\""]
        );
    }

    #[test]
    fn all_problems_are_reported_in_one_pass() {
        let found = errors(
            r#"
            [thumbs]
            left = "explode"
            [spelling]
            keys = "aab"
            [spelling.shift]
            "z" = "Z"
            "a" = ""
            [[family]]
            id = "one"
            plane = "sideways"
            left = "I"
            [[family]]
            id = "two"
            plane = "core"
            left = "X"
            [[family]]
            id = "three"
            plane = "core"
            left = "TI"
            [[family]]
            id = "four"
            plane = "core"
            left = ""
            [[family]]
            id = "five"
            plane = "core"
            left = "I"
            members = ["lowercase", "9LIVES", "OK_1"]
            [[family]]
            id = "five"
            plane = "core"
            left = "M"
            [[family]]
            id = "six"
            plane = "literal"
            kind = "command"
            left = "I"
            members = ["fly", { token = "LEFT" }]
            [[family]]
            id = "seven"
            plane = "literal"
            kind = "text"
            left = "M"
            members = [{ token = "X" }]
            [[family]]
            id = ""
            plane = "core"
            left = "P"
            "#,
        );
        let expected = [
            "thumbs.left: unknown command \"explode\"",
            "spelling.keys lists 'a' more than once",
            "spelling.shift: the shifted form of \"a\" is empty",
            "spelling.shift: \"z\" is not one of the characters in spelling.keys",
            "family \"one\": unknown plane \"sideways\"",
            "family \"two\": left shape \"X\": unknown finger 'X'",
            "family \"three\": left shape \"TI\" includes the thumb",
            "family \"four\": left shape is empty",
            "family \"five\", member 1: \"lowercase\" is not a valid token",
            "family \"five\", member 2: \"9LIVES\" is not a valid token",
            "family \"five\": this id is used by more than one family",
            "family \"six\", member 1: unknown command \"fly\"",
            "family \"six\", member 2: only concept families take { token, gloss } members",
            "family \"seven\", member 1: only concept families take { token, gloss } members",
            "a family has an empty id",
        ];
        assert_eq!(found.len(), expected.len(), "errors were: {found:#?}");
        for (message, start) in found.iter().zip(expected) {
            assert!(
                message.starts_with(start),
                "{message:?} should start with {start:?}"
            );
        }
    }

    #[test]
    fn too_many_members() {
        let members: Vec<String> = (0..16).map(|n| format!("\"T{n}\"")).collect();
        let found = errors(&format!(
            "[[family]]\nid = \"big\"\nplane = \"core\"\nleft = \"I\"\nmembers = [{}]\n",
            members.join(", ")
        ));
        assert_eq!(
            found,
            ["family \"big\": 16 members, but a family holds at most 15"]
        );
    }

    #[test]
    fn too_many_spelling_keys() {
        let found = errors("[spelling]\nkeys = \"abcdefghijklmnopqrstuvwxyz01234\"\n");
        assert_eq!(
            found,
            ["spelling.keys has 31 characters, but one hand has only 30 spelling shapes"]
        );
    }

    #[test]
    fn unknown_fields_and_bad_toml_are_rejected() {
        let found =
            errors("[[family]]\nid = \"a\"\nplane = \"core\"\nleft = \"I\"\nmember = [\"A\"]\n");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].starts_with("not a valid layout file:"),
            "{found:?}"
        );
        assert!(found[0].contains("member"), "{found:?}");

        let error = Layout::from_toml("this is not toml").expect_err("should fail");
        assert!(
            error
                .to_string()
                .starts_with("error: not a valid layout file:")
        );
    }

    #[test]
    fn empty_layout_name_is_an_error() {
        let error = Layout::from_toml("[layout]\nname = \" \"\nversion = \"1\"\n")
            .expect_err("should fail");
        assert_eq!(error.to_string(), "error: layout.name is empty");
    }

    #[test]
    fn error_display_lists_every_diagnostic_on_its_own_line() {
        let error = Layout::from_toml(&format!(
            "{HEADER}[thumbs]\nleft = \"x\"\n[spelling]\nkeys = \"a\"\n"
        ))
        .expect_err("should fail");
        let text = error.to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("error: thumbs.left"));
        assert!(lines[1].starts_with("warning: spelling.keys has no chord for"));
    }

    #[test]
    fn token_syntax() {
        for good in ["A", "FIX", "WITH_TESTS", "V2", "I"] {
            assert!(is_token(good), "{good}");
        }
        for bad in ["", "fix", "Fix", "_X", "2X", "A-B", "A B", "É"] {
            assert!(!is_token(bad), "{bad}");
        }
    }
}
