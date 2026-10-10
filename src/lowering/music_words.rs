//! The words of music in a program that uses sound (card sound-pro, notes/sound.md).
//! Tempo (layer 1): `1/4 beat`, `2 beats`, `1 bar` are seconds at the module's tempo, lib/sound.warp beat_seconds and
//! bar_seconds: the amount is the item written before the word, of an assignment its value (`x = 1/4 beat`).
//! Note names (layer 2): a letter A–G, a sharp (`#`, `♯`)
//! or flat (`b`, `♭`) and an octave 0–9 is its MIDI note's frequency at the program's tuning, lib/sound.warp
//! midi_hertz: `play F#4`, `melody [C4 Eb4 G4]`.
//! Parsed, not looked up in a table, so every octave and accidental works. `F#4` reads as `F # 4` (indexing); a
//! single note letter that the program does not define is no list, so there it is the note. A variable or function the
//! program names like a note (`A4 = 3`) keeps its meaning. Runs before modules::resolve, while the program is its own.
//! Scheduling (card sample-accurate): `at 2 beats play C4`, `at 1 bar { … }` is the statement between lib/sound.warp
//! sound_at(time) and sound_at_end(): its sounds start at that time on the voice, the voice then goes on where it stood.
//! Chord and scale kinds (card music-values): `chord(C4, major7)`, `scale(D4, dorian)` name their kind by a word the
//! program does not define, the text that lib/sound.warp chord_intervals and scale_intervals know it by.

use super::nodes::call;
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashSet;

const MIDI_HERTZ_CALL: &str = "midi_hertz";
/// A note letter's semitone above C is its place here, from 0 (as lib/sound.warp's note(name))
const SCALE_LETTERS: &str = "C.D.EF.G.A.B";
const SHARPS: [&str; 2] = ["#", "♯"];
const FLATS: [&str; 2] = ["b", "♭"];
/// a unit word of music and the word of lib/sound.warp that makes a plain number of that many: seconds of beats and
/// bars at the tempo, the gain factor of decibels
const UNIT_WORDS: [(&str, &str); 5] = [("beat", "beat_seconds"), ("beats", "beat_seconds"), ("bar", "bar_seconds"), ("bars", "bar_seconds"), ("dB", "decibels")];
const AT_WORD: &str = "at";
const DEF_WORD: &str = "def";
const AT_CALL: &str = "sound_at";
const AT_END_CALL: &str = "sound_at_end";
/// the words whose second argument is a kind: chord(root, kind), scale(root, kind)
const KIND_TAKERS: [&str; 2] = ["chord", "scale"];

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
	if let Some(midi) = note_midi(&node, defined) {
		return call(MIDI_HERTZ_CALL, vec![Node::Number(crate::Number::Int(midi))]).with_meta_of(&node);
	}
	if let Some((word, amount)) = glued_unit(&node, defined) {
		return call(word, vec![with_music_words(amount, defined)]);
	}
	match node.map_children(|child| with_music_words(child, defined)) {
		Node::List(items, bracket, separator) if !separator.separates_statements() => Node::List(with_kind_text(with_unit_words(items, defined), defined), bracket, separator),
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

/// `chord(C4, major7)` → `chord(C4, "major7")`
fn with_kind_text(mut items: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	let undefined_word = |node: &Node, words: &[&str]| matches!(node.drop_meta(), Node::Symbol(name) if !defined.contains(name) && (words.is_empty() || words.contains(&name.as_str())));
	if let [taker, _, kind] = items.as_mut_slice() {
		if undefined_word(taker, &KIND_TAKERS) && undefined_word(kind, &[]) {
			if let Node::Symbol(name) = kind.drop_meta() {
				*kind = Node::Text(name.clone()).with_meta_of(kind);
			}
		}
	}
	items
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

/// The MIDI note of a note name the program does not define: `Eb4` (a symbol), `F#4` (read as `F # 4`)
fn note_midi(node: &Node, defined: &HashSet<String>) -> Option<i64> {
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
	midi_of(&name)
}

/// `C#4` → 61: the letter's semitone, the accidental, the octave counted from C-1 as MIDI does
fn midi_of(name: &str) -> Option<i64> {
	let mut chars = name.chars();
	let letter = chars.next().filter(char::is_ascii_uppercase)?;
	let semitone = SCALE_LETTERS.find(letter)? as i64;
	let rest: String = chars.collect();
	let octave_digit = rest.chars().last().filter(char::is_ascii_digit)?;
	let accidental = &rest[..rest.len() - 1];
	let shift = if accidental.is_empty() { 0 } else if SHARPS.contains(&accidental) { 1 } else if FLATS.contains(&accidental) { -1 } else { return None };
	Some(12 * (octave_digit.to_digit(10)? as i64 + 1) + semitone + shift)
}

/// The names the program assigns or defines anywhere: `A4 = 3`, `B3(x) := …` (B3 and its parameter x), `def f(k) {…}`,
/// `for kind in …`
fn defined_names(program: &Node) -> HashSet<String> {
	let mut names: HashSet<String> = super::for_loop::loop_variables(program).into_iter().collect();
	program.visit(&mut |node| match node {
		Node::Key(target, Op::Assign | Op::Define, _) => names.extend(definition_names(target)),
		Node::List(items, _, _) if items.first().is_some_and(|keyword| keyword.is_symbol(DEF_WORD)) => names.extend(items.get(1).map(definition_names).unwrap_or_default()),
		_ => {}
	});
	names
}

/// `B3` → B3; `(B3 x)`, `(f (k))` → the function and its parameters
fn definition_names(target: &Node) -> Vec<String> {
	match target.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::List(items, _, _) => items.iter().flat_map(definition_names).collect(),
		_ => vec![],
	}
}
