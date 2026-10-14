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
//! Live loops (card patterns-live): `live_loop 4 { pattern "c4 e4" }` is `go { … }` repeating the block 4 times: its own voice,
//! playing along with the others on the beat clock.
//! Chord and scale kinds (card music-values): `chord(C4, major7)`, `scale(D4, dorian)` name their kind by a word the
//! program does not define, the text that lib/sound.warp chord_intervals and scale_intervals know it by.

use super::memoization::definition_parts;
use super::nodes::{call, key};
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{AT_WORD, FLOAT_WORD};
use std::collections::{HashMap, HashSet};

const MIDI_HERTZ_CALL: &str = "midi_hertz";
/// `note("C#4")`: lib/sound.warp's Note of a frequency, its note-name text read here like the bare name C#4 (card note-value)
const NOTE_CALL: &str = "note";
/// A note letter's semitone above C is its place here, from 0 (as lib/sound.warp's note(name))
const SCALE_LETTERS: &str = "C.D.EF.G.A.B";
const SHARPS: [&str; 2] = ["#", "♯"];
const FLATS: [&str; 2] = ["b", "♭"];
/// a unit word of music and the word of lib/sound.warp that makes a plain number of that many: seconds of beats and
/// bars at the tempo, the gain factor of decibels
const UNIT_WORDS: [(&str, &str); 5] = [
	("beat", "beat_seconds"),
	("beats", "beat_seconds"),
	("bar", "bar_seconds"),
	("bars", "bar_seconds"),
	("dB", "decibels"),
];
const DEF_WORD: &str = "def";
const AT_CALL: &str = "sound_at";
const AT_END_CALL: &str = "sound_at_end";
const LIVE_LOOP_WORD: &str = "live_loop";
/// what `live_loop 4 { … }` is: a voice of its own that plays the block that many cycles, one after another
const LIVE_LOOP_TEMPLATE: &str = "live_voice = go { for live_cycle in 0..live_cycles { live_body } }";
const LIVE_CYCLES: &str = "live_cycles";
const LIVE_BODY: &str = "live_body";
/// the words whose second argument is a kind: chord(root, kind), scale(root, kind)
const KIND_TAKERS: [&str; 2] = ["chord", "scale"];
const SIGNAL_WORD: &str = "play_signal";
/// `play_blocks(fill, seconds)` → BLOCKS_CALLS: the host (native.rs played_blocks) calls fill·blocks(t) for each block,
/// which hands fill the linear float array fill·block of BLOCK_SAMPLES cells to fill (BLOCK_TEMPLATES), in stereo
/// (fill of three parameters: left, right, t) fill·block and fill·right; mono first, then stereo
const BLOCKS_WORD: &str = "play_blocks";
const BLOCKS_CALLS: [&str; 2] = [
	"std_io(\"sound\", \"blocks\", [block_caller, linear_address(block_cells), si_amount(block_seconds), sample_rate])",
	"std_io(\"sound\", \"blocks\", [block_caller, [linear_address(block_cells), linear_address(block_right)], si_amount(block_seconds), sample_rate])",
];
const BLOCK_TEMPLATES: [&str; 2] = [
	"linear block_cells = float[block_samples]\nblock_caller(t: float) := { block_fill(block_cells, t); 0.0 as float }",
	"linear block_cells = float[block_samples]\nlinear block_right = float[block_samples]\nblock_caller(t: float) := { block_fill(block_cells, block_right, t); 0.0 as float }",
];
const STEREO_FILL_PARAMETERS: usize = 3;
const PLAY_WORD: &str = "play";
/// The samples a signal computes at once (sound.rs play_signal, play_blocks): 23 ms at 22050 Hz
pub const BLOCK_SAMPLES: usize = 512;

/// Before the go blocks are lowered (pipeline.rs): `live_loop 4 { … }` becomes the `go { … }` it stands for
pub fn lower_live_loops(program: Node) -> Node {
	if !crate::modules::uses_sound(&program) {
		return program;
	}
	let defined = defined_names(&program);
	expand_live_loops(program, &defined)
}

fn expand_live_loops(node: Node, defined: &HashSet<String>) -> Node {
	if let Some(voice) = live_loop_statement(&node, defined) {
		return voice;
	}
	node.map_children(|child| expand_live_loops(child, defined))
}

pub fn lower(program: Node) -> Node {
	if !crate::modules::uses_sound(&program) {
		return program;
	}
	let defined = defined_names(&program);
	let counts = parameter_counts(&program);
	let program = with_block_signals(with_float_signals(with_played_functions(program, &counts)), &counts);
	match with_music_words(program, &defined) {
		Node::List(statements, bracket, separator) if separator.separates_statements() => {
			Node::List(with_scheduled(statements, &defined), bracket, separator)
		}
		statement => match scheduled(&statement, &defined) {
			Some(statements) => Node::List(statements, crate::node::Bracket::None, crate::node::Separator::Newline),
			None => statement,
		},
	}
}

/// The functions `play_signal(hum, 2s)` plays are of floats to a float: an untyped parameter is the time in seconds the
/// host calls them with for every sample (sound.rs play_signal), their value a float even where it is whole (`3.0`)
fn with_float_signals(program: Node) -> Node {
	let mut signals = HashSet::new();
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node.drop_meta() {
			if let [word, signal, ..] = items.as_slice() {
				if word.is_symbol(SIGNAL_WORD) {
					// in stereo a pair [left, right]
					let channels = match signal.drop_meta() {
						Node::List(pair, _, _) => pair.clone(),
						mono => vec![mono.clone()],
					};
					signals.extend(channels.iter().filter_map(|channel| channel.symbol_name().map(str::to_string)));
				}
			}
		}
	});
	if signals.is_empty() {
		program
	} else {
		with_float_parameters(program, &signals)
	}
}

/// The functions `play_blocks(fill, 2s)` plays fill a block of samples at a time (BLOCKS_WORD): each gets its block
/// and the function the host calls, before the program's statements
fn with_block_signals(program: Node, counts: &HashMap<String, usize>) -> Node {
	let mut fills = Vec::new();
	let program = with_block_calls(program, counts, &mut fills);
	if fills.is_empty() {
		return program;
	}
	let declarations = fills.iter().flat_map(|(fill, stereo)| {
		match block_template(BLOCK_TEMPLATES[*stereo as usize], fill, Node::Empty) {
			Node::List(statements, _, separator) if separator.separates_statements() => statements,
			other => vec![other],
		}
	});
	let (statements, bracket, separator) = match program {
		Node::List(statements, bracket, separator) if separator.separates_statements() => {
			(statements, bracket, separator)
		}
		statement => (vec![statement], Bracket::None, Separator::Newline),
	};
	Node::List(declarations.chain(statements).collect(), bracket, separator)
}

/// The template's placeholders replaced by the names of the block-wise signal fill, played for `seconds`
fn block_template(template: &str, fill: &str, seconds: Node) -> Node {
	let part = |name: &str| Node::Symbol(format!("{fill}{}{name}", crate::analyzer::TEMPORARY_SEPARATOR));
	let names = [
		("block_cells", part("block")),
		("block_right", part("right")),
		("block_caller", part("blocks")),
		("block_fill", Node::symbol(fill)),
		("block_samples", crate::node::int(BLOCK_SAMPLES as i64)),
		("block_seconds", seconds),
	];
	names.into_iter().fold(crate::warp_parser::parse(template), |node, (placeholder, value)| {
		crate::library_words::substitute(node, placeholder, &value)
	})
}

/// `play(hum, 2s)` of the program's function hum (PLAY_WORD): play_signal(hum, 2s) of a function of the time (or
/// a pair [left, right] of them), play_blocks(hum, 2s) of a block function (hum(block, t), hum(left, right, t))
fn with_played_functions(node: Node, counts: &HashMap<String, usize>) -> Node {
	let node = node.map_children(|child| with_played_functions(child, counts));
	match node.call_parts() {
		Some((PLAY_WORD, [played, seconds])) if let Some(word) = signal_word(played, counts) => {
			call(word, vec![played.clone(), seconds.clone()])
		}
		_ => node,
	}
}

/// The word playing `played` if it is a function of the time, a pair of them, or a block function
fn signal_word(played: &Node, counts: &HashMap<String, usize>) -> Option<&'static str> {
	let count = |node: &Node| node.symbol_name().and_then(|name| counts.get(name)).copied();
	match played.drop_meta() {
		Node::List(pair, Bracket::Square, _)
			if pair.len() == 2 && pair.iter().all(|function| count(function) == Some(1)) =>
		{
			Some(SIGNAL_WORD)
		}
		function => match count(function)? {
			1 => Some(SIGNAL_WORD),
			2 | STEREO_FILL_PARAMETERS => Some(BLOCKS_WORD),
			_ => None,
		},
	}
}

/// The functions the program defines and how many parameters each takes
fn parameter_counts(program: &Node) -> HashMap<String, usize> {
	let mut counts = HashMap::new();
	program.visit(&mut |node| {
		if let Some(Node::List(items, _, _)) = definition_parts(node).map(|(head, _, _)| head.drop_meta().clone()) {
			counts.extend(items.first().and_then(Node::symbol_name).map(|name| (name.to_string(), items.len() - 1)));
		}
	});
	counts
}

/// Each play_blocks call written out, its fill and whether it is in stereo in `fills`
fn with_block_calls(node: Node, parameter_counts: &HashMap<String, usize>, fills: &mut Vec<(String, bool)>) -> Node {
	match node.map_children(|child| with_block_calls(child, parameter_counts, fills)) {
		Node::List(ref items, _, _)
			if let [word, fill, seconds] = items.as_slice()
				&& word.is_symbol(BLOCKS_WORD)
				&& let Some(name) = fill.symbol_name() =>
		{
			let stereo = parameter_counts.get(name) == Some(&STEREO_FILL_PARAMETERS);
			if !fills.iter().any(|(known, _)| known == name) {
				fills.push((name.to_string(), stereo));
			}
			block_template(BLOCKS_CALLS[stereo as usize], name, seconds.clone())
		}
		other => other,
	}
}

fn with_float_parameters(node: Node, functions: &HashSet<String>) -> Node {
	let Some((head, body, rebuild)) = definition_parts(&node) else {
		return node.map_children(|child| with_float_parameters(child, functions));
	};
	match head.drop_meta() {
		Node::List(items, bracket, separator)
			if items.first().and_then(Node::symbol_name).is_some_and(|name| functions.contains(name)) =>
		{
			let typed = items.iter().enumerate().map(|(index, item)| {
				if index > 0 && item.symbol_name().is_some() {
					key(item.clone(), Op::Colon, Node::symbol(FLOAT_WORD))
				} else {
					item.clone()
				}
			});
			rebuild(
				Node::List(typed.collect(), bracket.clone(), separator.clone()),
				key(parens(vec![body]), Op::As, Node::symbol(FLOAT_WORD)),
			)
		}
		_ => node,
	}
}

fn with_music_words(node: Node, defined: &HashSet<String>) -> Node {
	if let Some(midi) = note_midi(&node, defined) {
		return call(MIDI_HERTZ_CALL, vec![Node::Number(crate::Number::Int(midi))]).with_meta_of(&node);
	}
	if let Some(named) = with_note_name_text(&node, defined) {
		return named;
	}
	if let Some((word, amount)) = glued_unit(&node, defined) {
		return call(word, vec![with_music_words(amount, defined)]);
	}
	match node.map_children(|child| with_music_words(child, defined)) {
		// a block of one statement, a loop's or an if's body: `for i in 1 to 3 { at 0 bars play(C4, 1s) }`
		Node::List(items, Bracket::Curly, separator)
			if !separator.separates_statements()
				&& let Some(statements) = scheduled(
					&Node::List(with_unit_words(items.clone(), defined), Bracket::None, separator.clone()),
					defined,
				) =>
		{
			Node::List(statements, Bracket::Curly, Separator::Newline)
		}
		Node::List(items, bracket, separator) if !separator.separates_statements() => {
			Node::List(with_kind_text(with_unit_words(items, defined), defined), bracket, separator)
		}
		Node::List(statements, bracket, separator) => {
			Node::List(with_scheduled(statements, defined), bracket, separator)
		}
		other => other,
	}
}

/// A unit word glued to its number: `2beats` (read as `2 * beats`) and `-6dB` (read as `-(6 * dB)`, whose minus
/// belongs to the amount: -6 dB, not the negated gain of 6 dB)
fn glued_unit(node: &Node, defined: &HashSet<String>) -> Option<(&'static str, Node)> {
	match node.drop_meta() {
		Node::Key(amount, Op::Mul, unit) => unit_amount_word(unit, defined).map(|word| (word, *amount.clone())),
		Node::Key(nothing, op @ (Op::Sub | Op::Neg), product) if nothing.is_empty_node() => {
			let (word, amount) = glued_unit(product, defined)?;
			Some((word, Node::Key(nothing.clone(), *op, Box::new(amount))))
		}
		_ => None,
	}
}

fn with_scheduled(statements: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	statements
		.into_iter()
		.flat_map(|statement| scheduled(&statement, defined).unwrap_or_else(|| vec![statement]))
		.collect()
}

/// `at 2 beats play C4` → `sound_at(beat_seconds(2))`, `play C4`, `sound_at_end()`; a block's statements in place
fn scheduled(statement: &Node, defined: &HashSet<String>) -> Option<Vec<Node>> {
	let Node::List(items, _, separator) = statement.drop_meta() else {
		return None;
	};
	let [at, time, action @ ..] = items.as_slice() else {
		return None;
	};
	if separator.separates_statements() || action.is_empty() || defined.contains(AT_WORD) || !at.is_symbol(AT_WORD) {
		return None;
	}
	let actions = match action {
		[block] => match block.drop_meta() {
			Node::List(statements, crate::node::Bracket::Curly, inner)
				if inner.separates_statements() || statements.len() == 1 =>
			{
				statements.clone()
			}
			Node::List(words, crate::node::Bracket::Curly, inner) => {
				vec![Node::List(words.clone(), crate::node::Bracket::None, inner.clone())]
			}
			single => vec![single.clone()],
		},
		_ => vec![Node::List(action.to_vec(), crate::node::Bracket::None, separator.clone())],
	};
	Some([call(AT_CALL, vec![time.clone()])].into_iter().chain(actions).chain([call(AT_END_CALL, vec![])]).collect())
}

/// `live_loop 4 { body }` → `go { for live_cycle in 0..4 { body } }`
fn live_loop_statement(statement: &Node, defined: &HashSet<String>) -> Option<Node> {
	let Node::List(items, _, separator) = statement.drop_meta() else {
		return None;
	};
	let [word, cycles, rest @ ..] = items.as_slice() else {
		return None;
	};
	let [block] = rest else { return None };
	let Node::List(_, crate::node::Bracket::Curly, _) = block.drop_meta() else {
		return None;
	};
	if separator.separates_statements() || defined.contains(LIVE_LOOP_WORD) || !word.is_symbol(LIVE_LOOP_WORD) {
		return None;
	}
	let made = substituted(
		crate::warp_parser::parse(LIVE_LOOP_TEMPLATE),
		&[(LIVE_CYCLES, cycles.clone()), (LIVE_BODY, block.clone())],
	);
	Some(match made {
		Node::List(mut statements, _, _) if statements.len() == 1 => statements.remove(0),
		made => made,
	})
}

/// The node with each named symbol replaced; a block holding only such a symbol is replaced whole (a block, not a block in a block)
fn substituted(node: Node, replacements: &[(&str, Node)]) -> Node {
	let alone = match node.drop_meta() {
		Node::List(items, crate::node::Bracket::Curly, _) => match items.as_slice() {
			[only] => match only.drop_meta() {
				Node::Symbol(name) => Some(name),
				_ => None,
			},
			_ => None,
		},
		Node::Symbol(name) => Some(name),
		_ => None,
	};
	if let Some(name) = alone {
		if let Some((_, replacement)) = replacements.iter().find(|(word, _)| word == name) {
			return replacement.clone();
		}
	}
	node.map_children(|child| substituted(child, replacements))
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
				Node::Key(target, Op::Assign, value) => {
					Node::Key(target.clone(), Op::Assign, Box::new(call(word, vec![*value.clone()])))
						.with_meta_of(&amount)
				}
				_ => call(word, vec![amount]),
			}),
			(_, before) => written.extend(before.into_iter().chain([item])),
		}
	}
	written
}

fn unit_amount_word(node: &Node, defined: &HashSet<String>) -> Option<&'static str> {
	match node.drop_meta() {
		Node::Symbol(name) if !defined.contains(name) => {
			UNIT_WORDS.iter().find(|(word, _)| word == name).map(|(_, seconds)| *seconds)
		}
		_ => None,
	}
}

/// `note("C#4")` → `note(midi_hertz(61))`: a note name written as a text, in a call of note the program does not define
fn with_note_name_text(node: &Node, defined: &HashSet<String>) -> Option<Node> {
	let Node::List(items, bracket, separator) = node.drop_meta() else { return None };
	let [head, argument] = items.as_slice() else { return None };
	if !matches!(head.drop_meta(), Node::Symbol(name) if name == NOTE_CALL) || defined.contains(NOTE_CALL) {
		return None;
	}
	let hertz = |text: &Node| match text.drop_meta() {
		Node::Text(name) => midi_of(name)
			.map(|midi| call(MIDI_HERTZ_CALL, vec![Node::Number(crate::Number::Int(midi))]).with_meta_of(text)),
		_ => None,
	};
	let argument = match argument.drop_meta() {
		Node::List(inner, inner_bracket, inner_separator) if inner.len() == 1 => {
			Node::List(vec![hertz(&inner[0])?], inner_bracket.clone(), inner_separator.clone()).with_meta_of(argument)
		}
		text => hertz(text)?.with_meta_of(argument),
	};
	Some(Node::List(vec![head.clone(), argument], bracket.clone(), separator.clone()).with_meta_of(node))
}

/// The MIDI note of a note name the program does not define: `Eb4` (a symbol), `F#4` (read as `F # 4`)
fn note_midi(node: &Node, defined: &HashSet<String>) -> Option<i64> {
	let (written, name) = match node.drop_meta() {
		Node::Symbol(name) => (name.clone(), name.clone()),
		Node::Key(letter, Op::Hash, octave) => match (letter.drop_meta(), octave.drop_meta()) {
			(Node::Symbol(letter), Node::Number(crate::Number::Int(octave))) if letter.len() == 1 => {
				(letter.clone(), format!("{letter}#{octave}"))
			}
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
	let shift = if accidental.is_empty() {
		0
	} else if SHARPS.contains(&accidental) {
		1
	} else if FLATS.contains(&accidental) {
		-1
	} else {
		return None;
	};
	Some(12 * (octave_digit.to_digit(10)? as i64 + 1) + semitone + shift)
}

/// The names the program assigns or defines anywhere: `A4 = 3`, `B3(x) := …` (B3 and its parameter x), `def f(k) {…}`,
/// `for kind in …`
fn defined_names(program: &Node) -> HashSet<String> {
	let mut names: HashSet<String> = super::for_loop::loop_variables(program).into_iter().collect();
	program.visit(&mut |node| match node {
		Node::Key(target, Op::Assign | Op::Define, _) => names.extend(definition_names(target)),
		Node::List(items, _, _) if items.first().is_some_and(|keyword| keyword.is_symbol(DEF_WORD)) => {
			names.extend(items.get(1).map(definition_signature_names).unwrap_or_default())
		}
		_ => {}
	});
	names
}

/// `def h(v) { play C4 }` is `def ((h (v)) {body})`: the names of the signature, not of the body
fn definition_signature_names(definition: &Node) -> Vec<String> {
	match definition.drop_meta() {
		Node::List(parts, _, _)
			if parts.last().is_some_and(|last| matches!(last.drop_meta(), Node::List(_, Bracket::Curly, _))) =>
		{
			parts[..parts.len() - 1].iter().flat_map(definition_names).collect()
		}
		_ => definition_names(definition),
	}
}

/// `B3` → B3; `(B3 x)`, `(f (k))` → the function and its parameters
fn definition_names(target: &Node) -> Vec<String> {
	match target.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::List(items, _, _) => items.iter().flat_map(definition_names).collect(),
		_ => vec![],
	}
}
