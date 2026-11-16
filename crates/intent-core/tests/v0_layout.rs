//! Guards the shipped layout: it must compile cleanly and keep its promises.

use std::collections::HashSet;

use intent_core::effort::finger_shape_order;
use intent_core::layout::{Compiled, Layout, Origin};
use intent_core::{Action, Chord, Command, Hand, HandShape, Plane};

const SOURCE: &str = include_str!("../../../layouts/intent-v0.toml");

fn compiled() -> Compiled {
    match Layout::from_toml(SOURCE) {
        Ok(compiled) => compiled,
        Err(error) => panic!("intent-v0 must compile, but:\n{error}"),
    }
}

fn chord(text: &str) -> Chord {
    text.parse().expect("valid chord")
}

#[test]
fn compiles_without_warnings() {
    assert_eq!(compiled().warnings, []);
}

#[test]
fn every_letter_can_be_spelled_with_either_hand_in_both_cases() {
    let layout = compiled().layout;
    for letter in 'a'..='z' {
        for text in [letter.to_string(), letter.to_ascii_uppercase().to_string()] {
            let hands: HashSet<Hand> = layout
                .chords_for_text(&text)
                .into_iter()
                .filter_map(|chord| match chord.plane() {
                    Some(Plane::Spell(hand) | Plane::SpellShift(hand)) => Some(hand),
                    _ => None,
                })
                .collect();
            assert_eq!(hands.len(), 2, "{text:?} should be on both hands");
        }
    }
}

#[test]
fn the_three_everyday_commands_are_on_the_thumbs() {
    let layout = compiled().layout;
    assert_eq!(layout.chords_for_command(Command::Undo), [chord("T-")]);
    assert_eq!(layout.chords_for_command(Command::Space), [chord("-T")]);
    assert_eq!(layout.chords_for_command(Command::Send), [chord("T-T")]);
}

#[test]
fn every_command_is_reachable() {
    let layout = compiled().layout;
    for command in Command::ALL {
        assert!(
            !layout.chords_for_command(command).is_empty(),
            "no chord for <{command}>"
        );
    }
}

#[test]
fn every_concept_resolves_back_to_its_own_chord() {
    let layout = compiled().layout;
    let mut concepts = 0;
    for entry in layout.entries() {
        if let Action::Concept(concept) = &entry.action {
            concepts += 1;
            assert_eq!(layout.chord_for_token(&concept.token), Some(entry.chord));
            assert_eq!(layout.lookup(entry.chord), Some(entry));
        }
    }
    assert_eq!(concepts, 195);
}

#[test]
fn directives_explain_themselves_to_the_decoder() {
    let layout = compiled().layout;
    let directive = layout
        .families()
        .iter()
        .position(|family| family.id == "directive")
        .expect("directive family");
    let members: Vec<_> = layout
        .entries()
        .iter()
        .filter(|entry| entry.origin == Origin::Family(directive))
        .collect();
    assert_eq!(members.len(), 15);
    for entry in members {
        let Action::Concept(concept) = &entry.action else {
            panic!("directives are concepts");
        };
        assert!(
            concept.gloss.as_deref().is_some_and(|g| !g.is_empty()),
            "{} needs a gloss",
            concept.token
        );
    }
}

#[test]
fn families_fill_their_easiest_slots_first_with_no_holes() {
    let layout = compiled().layout;
    let order = finger_shape_order();
    for (index, family) in layout.families().iter().enumerate() {
        let used: HashSet<HandShape> = layout
            .entries()
            .iter()
            .filter(|entry| entry.origin == Origin::Family(index))
            .map(|entry| entry.chord.hand(Hand::Right).fingers())
            .collect();
        let easiest: HashSet<HandShape> = order.iter().copied().take(used.len()).collect();
        assert_eq!(used, easiest, "family {}", family.id);
    }
    // The single cheapest two-hand chord holds the first action.
    let entry = layout.lookup(chord("I-I")).expect("I-I is assigned");
    assert_eq!(entry.action.to_string(), "MAKE");
}

#[test]
fn arrows_sit_on_single_fingers() {
    let layout = compiled().layout;
    assert_eq!(layout.chords_for_command(Command::Left), [chord("MIT-I")]);
    assert_eq!(layout.chords_for_command(Command::Down), [chord("MIT-M")]);
    assert_eq!(layout.chords_for_command(Command::Up), [chord("MIT-R")]);
    assert_eq!(layout.chords_for_command(Command::Right), [chord("MIT-P")]);
}

#[test]
fn plane_usage_adds_up() {
    let layout = compiled().layout;
    let total: usize = Plane::ALL
        .into_iter()
        .map(|plane| layout.plane_usage(plane))
        .sum();
    assert_eq!(total, layout.entries().len());
    assert_eq!(layout.plane_usage(Plane::Thumbs), 3);
    assert_eq!(layout.plane_usage(Plane::Spell(Hand::Left)), 30);
    assert_eq!(layout.plane_usage(Plane::SpellShift(Hand::Right)), 30);
    assert_eq!(layout.plane_usage(Plane::Core), 150);
    assert_eq!(layout.plane_usage(Plane::Extended), 45);
    assert_eq!(layout.plane_usage(Plane::User), 0);
}
