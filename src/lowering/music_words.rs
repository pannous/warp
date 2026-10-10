//! The words of music in a program that uses sound (card sound-pro, notes/sound.md).
//! Tempo (layer 1): `1/4 beat`, `2 beats`, `1 bar` are seconds at the module's tempo, lib/sound.warp beat_seconds and
//! bar_seconds: the amount is the item written before the word, of an assignment its value (`x = 1/4 beat`).
//! Note names (layer 2): a letter A–G, a sharp (`#`, `♯`)
//! or flat (`b`, `♭`) and an octave 0–9 is its equal-tempered frequency, A4 = 440 Hz: `play F#4`, `melody [C4 Eb4 G4]`.
//! Parsed, not looked up in a table, so every octave and accidental works. `F#4` reads as `F # 4` (indexing); a
//! single note letter that the program does not define is no list, so there it is the note. A variable or function the
//! program names like a note (`A4 = 3`) keeps its meaning. Runs before modules::resolve, while the program is its own.
//! Scheduling (card sample-accurate): `at 2 beats play C4`, `at 1 bar { … }` is the statement between lib/sound.warp
//! sound_at(time) and sound_at_end(): its sounds start at that time on the voice, the voice then goes on where it stood.

use super::nodes::call;
use crate::node::{float, Node};
use crate::operators::Op;
use std::collections::HashSet;

/// The note A4's frequency and MIDI number: the reference of equal temperament
const A4_HERTZ: f64 = 440.0;
const A4_MIDI: i64 = 69;
/// A note letter's semitone above C is its place here, from 0 (as lib/sound.warp's note(name))
const SCALE_LETTERS: &str = "C.D.EF.G.A.B";
const SHARPS: [&str; 2] = ["#", "♯"];
const FLATS: [&str; 2] = ["b", "♭"];
/// a unit word of music and the word of lib/sound.warp that makes a plain number of that many: seconds of beats and
/// bars at the tempo, the gain factor of decibels
const UNIT_WORDS: [(&str, &str); 5] = [("beat", "beat_seconds"), ("beats", "beat_seconds"), ("bar", "bar_seconds"), ("bars", "bar_seconds"), ("dB", "decibels")];
/// note frequencies are kept to a hundredth of a Hz, as written in tables (C4 = 261.63)
const HERTZ_DIGITS: f64 = 100.0;
const AT_WORD: &str = "at";
const AT_CALL: &str = "sound_at";
const AT_END_CALL: &str = "sound_at_end";

pub fn lower(program: Node) -> Node {
	if !crate::modules::uses_sound(&program) {
		return program;
	}
	let defined = defined_names(&program);
	match with_music_words(program, &defined) {
		Node::List(statements, bracket, separator) if separator.separates_statements() => Node::List(with_scheduled(statements, &defined), bracket, separator),
		statement => match scheduled(&statement, &defined) {
			Some(statements) => Node::List(statements, crate::node::Bracket::None, crate::node::Separator::Newline),
			None => statement,
		},
	}
}

fn with_music_words(node: Node, defined: &HashSet<String>) -> Node {
	if let Some(hertz) = note_frequency(&node, defined) {
		return float(hertz).with_meta_of(&node);
	}
	if let Some((word, amount)) = glued_unit(&node, defined) {
		return call(word, vec![with_music_words(amount, defined)]);
	}
	match node.map_children(|child| with_music_words(child, defined)) {
		Node::List(items, bracket, separator) if !separator.separates_statements() => Node::List(with_unit_words(items, defined), bracket, separator),
		Node::List(statements, bracket, separator) => Node::List(with_scheduled(statements, defined), bracket, separator),
		other => other,
	}
}

/// A unit word glued to its number: `2beats` (read as `2 * beats`) and `-6dB` (read as `-(6 * dB)`, whose minus
/// belongs to the amount: -6 dB, not the negated gain of 6 dB)
fn glued_unit(node: &Node, defined: &HashSet<String>) -> Option<(&'static str, Node)> {
	match node.drop_meta() {
		Node::Key(amount, Op::Mul, unit) => unit_amount_word(unit, defined).map(|word| (word, *amount.clone())),
		Node::Key(nothing, op @ (Op::Sub | Op::Neg), product) if matches!(nothing.drop_meta(), Node::Empty) => {
			let (word, amount) = glued_unit(product, defined)?;
			Some((word, Node::Key(nothing.clone(), *op, Box::new(amount))))
		}
		_ => None,
	}
}

fn with_scheduled(statements: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	statements.into_iter().flat_map(|statement| scheduled(&statement, defined).unwrap_or_else(|| vec![statement])).collect()
}

/// `at 2 beats play C4` → `sound_at(beat_seconds(2))`, `play C4`, `sound_at_end()`; a block's statements in place
fn scheduled(statement: &Node, defined: &HashSet<String>) -> Option<Vec<Node>> {
	let Node::List(items, _, separator) = statement.drop_meta() else { return None };
	let [at, time, action @ ..] = items.as_slice() else { return None };
	if separator.separates_statements() || action.is_empty() || defined.contains(AT_WORD) || !at.is_symbol(AT_WORD) {
		return None;
	}
	let actions = match action {
		[block] => match block.drop_meta() {
			Node::List(statements, crate::node::Bracket::Curly, inner) if inner.separates_statements() || statements.len() == 1 => statements.clone(),
			Node::List(words, crate::node::Bracket::Curly, inner) => vec![Node::List(words.clone(), crate::node::Bracket::None, inner.clone())],
			single => vec![single.clone()],
		},
		_ => vec![Node::List(action.to_vec(), crate::node::Bracket::None, separator.clone())],
	};
	Some([call(AT_CALL, vec![time.clone()])].into_iter().chain(actions).chain([call(AT_END_CALL, vec![])]).collect())
}

/// `[for, 1/4, beat]` → `[for, beat_seconds(1/4)]`; `[x = 2, bars]` → `[x = bar_seconds(2)]`
fn with_unit_words(items: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	let mut written: Vec<Node> = vec![];
	for item in items {
		match (unit_amount_word(&item, defined), written.pop()) {
			(Some(word), Some(amount)) => written.push(match amount.drop_meta() {
				Node::Key(target, Op::Assign, value) => Node::Key(target.clone(), Op::Assign, Box::new(call(word, vec![*value.clone()]))).with_meta_of(&amount),
				_ => call(word, vec![amount]),
			}),
			(_, before) => written.extend(before.into_iter().chain([item])),
		}
	}
	written
}

fn unit_amount_word(node: &Node, defined: &HashSet<String>) -> Option<&'static str> {
	match node.drop_meta() {
		Node::Symbol(name) if !defined.contains(name) => UNIT_WORDS.iter().find(|(word, _)| word == name).map(|(_, seconds)| *seconds),
		_ => None,
	}
}

/// The frequency of a note name the program does not define: `Eb4` (a symbol), `F#4` (read as `F # 4`)
fn note_frequency(node: &Node, defined: &HashSet<String>) -> Option<f64> {
	let (written, name) = match node.drop_meta() {
		Node::Symbol(name) => (name.clone(), name.clone()),
		Node::Key(letter, Op::Hash, octave) => match (letter.drop_meta(), octave.drop_meta()) {
			(Node::Symbol(letter), Node::Number(crate::Number::Int(octave))) if letter.len() == 1 => (letter.clone(), format!("{letter}#{octave}")),
			_ => return None,
		},
		_ => return None,
	};
	if defined.contains(&written) {
		return None;
	}
	frequency_of(&name)
}

/// `C#4` → 277.18: the letter's semitone, the accidental, the octave counted from C-1 as MIDI does
fn frequency_of(name: &str) -> Option<f64> {
	let mut chars = name.chars();
	let letter = chars.next().filter(char::is_ascii_uppercase)?;
	let semitone = SCALE_LETTERS.find(letter)? as i64;
	let rest: String = chars.collect();
	let octave_digit = rest.chars().last().filter(char::is_ascii_digit)?;
	let accidental = &rest[..rest.len() - 1];
	let shift = if accidental.is_empty() { 0 } else if SHARPS.contains(&accidental) { 1 } else if FLATS.contains(&accidental) { -1 } else { return None };
	let midi = 12 * (octave_digit.to_digit(10)? as i64 + 1) + semitone + shift;
	Some((A4_HERTZ * 2f64.powf((midi - A4_MIDI) as f64 / 12.0) * HERTZ_DIGITS).round() / HERTZ_DIGITS)
}

/// The names the program assigns or defines anywhere: `A4 = 3`, `B3(x) := …`
fn defined_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| if let Node::Key(target, Op::Assign | Op::Define, _) = node {
		let mut target = target.drop_meta();
		while let Node::List(items, _, _) = target {
			match items.first() {
				Some(first) => target = first.drop_meta(),
				None => break,
			}
		}
		if let Node::Symbol(name) = target {
			names.insert(name.clone());
		}
	});
	names
}
