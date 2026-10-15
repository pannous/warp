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
//! playing along with the others on the beat clock, awaited as `live_voice`; `live_loop drums 4 { … }` names it `drums`
//! (card pattern-extras), so several loops can be awaited.
//! Chord and scale kinds (card music-values): `chord(C4, major7)`, `scale(D4, dorian)` name their kind by a word the
//! program does not define, the text that lib/sound.warp chord_intervals and scale_intervals know it by.

use super::memoization::definition_parts;
use super::nodes::{call, key};
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{AT_WORD, FLOAT_WORD, FOR_WORD};
use std::collections::{HashMap, HashSet};

const MIDI_HERTZ_CALL: &str = "midi_hertz";
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
/// `save sound as "song.wav"`, `save(sound, "song.wav")` (card save-sound): the word sound, as save's argument, is the
/// sounds made since the last save, lib/sound.warp's Recording, whose save overload writes them as a WAV file
const SAVE_WORD: &str = "save";
const SOUND_WORD: &str = "sound";
const RECORDING_CLASS: &str = "Recording";
const AT_CALL: &str = "sound_at";
const AT_END_CALL: &str = "sound_at_end";
const LIVE_LOOP_WORD: &str = "live_loop";
/// what `live_loop 4 { … }` is: a voice of its own that plays the block that many cycles, one after another
const LIVE_LOOP_TEMPLATE: &str = "live_voice = go { for live_cycle in 0..live_cycles { live_body } }";
/// `live_loop drums { … }` without a count plays until the program is stopped
const ENDLESS_LIVE_LOOP_TEMPLATE: &str =
	"live_voice = go { live_cycle = 0\nwhile true { live_body\nlive_cycle = live_cycle + 1 } }";
const LIVE_VOICE: &str = "live_voice";
/// what a named loop's cycle says first (lib/sound.warp): the playground hands the loop to the next run at its next
/// cycle (card playground-live), natively nothing
const LIVE_CYCLE_STARTED: &str = "live_cycle_started";
const LIVE_CYCLES: &str = "live_cycles";
const LIVE_BODY: &str = "live_body";
/// A loop defined again by its name (card live-redefine): each definition's moment on the voices' clock (lib
/// sound_moment) in a shared array, -1 until it runs; each cycle plays the latest block whose moment it has reached
const LIVE_SINCE_DECLARATION: &str = "shared live_since = float[live_count]";
const LIVE_SINCE_UNSET: &str = "live_since#live_version = -1";
const LIVE_SINCE_SET: &str = "live_since#live_version = sound_moment()";
const LIVE_CHOICE: &str = "live_moment = sound_moment()\nlive_choice";
/// a redefinition counts for a cycle starting this many seconds before its moment: the voices' clocks start a few
/// microseconds apart, and `at 2 beats` is meant to hit the cycle starting then
const LIVE_VERSION_TEMPLATE: &str = "if live_since#live_version >= 0 and live_moment + 0.05 >= live_since#live_version { live_then } else { live_else }";
const LIVE_SINCE: &str = "live_since";
const LIVE_COUNT: &str = "live_count";
const LIVE_VERSION: &str = "live_version";
const LIVE_CHOICE_WORD: &str = "live_choice";
const LIVE_MOMENT_THEN: &str = "live_then";
const LIVE_MOMENT_ELSE: &str = "live_else";
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

/// The names of the program's named live loops, from the `live_cycle_started("drums")` each cycle says (host.rs
/// LIVE_LOOPS_SECTION)
pub fn live_loop_names(program: &Node) -> std::collections::BTreeSet<String> {
	let mut names = std::collections::BTreeSet::new();
	program.visit(&mut |node| {
		if let Some((LIVE_CYCLE_STARTED, [name])) = node.call_parts() {
			if let Node::Text(name) = name.drop_meta() {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// Before the go blocks are lowered (pipeline.rs): `live_loop 4 { … }` becomes the `go { … }` it stands for
pub fn lower_live_loops(program: Node) -> Node {
	if !crate::modules::uses_sound(&program) {
		return program;
	}
	let defined = defined_names(&program);
	expand_live_loops(program, &defined)
}

fn expand_live_loops(node: Node, defined: &HashSet<String>) -> Node {
	let inside = |node: Node| node.map_children(|child| expand_live_loops(child, defined));
	match node {
		meta @ Node::Meta { .. } => meta.under_meta(|plain| expand_live_loops(plain, defined)),
		Node::List(statements, bracket, separator) if separator.separates_statements() => {
			let statements = statements.into_iter().map(inside).collect();
			Node::List(with_live_loops(statements, defined), bracket, separator)
		}
		node => {
			let node = inside(node);
			match live_loop_of(&node, defined) {
				Some(live_loop) => live_loop.voice(std::slice::from_ref(&live_loop.block)),
				None => node,
			}
		}
	}
}

/// A block's statements with their live loops as voices; a named loop defined again there sets the moment its
/// redefinition starts (LIVE_SINCE_SET), its first definition a voice choosing among all the blocks
fn with_live_loops(statements: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	let loops: Vec<Option<LiveLoop>> = statements.iter().map(|statement| live_loop_of(statement, defined)).collect();
	let blocks_of = |name: &str| -> Vec<Node> {
		loops
			.iter()
			.flatten()
			.filter(|live_loop| live_loop.name() == Some(name))
			.map(|live_loop| live_loop.block.clone())
			.collect()
	};
	let mut versions: HashMap<String, usize> = HashMap::new();
	let mut lowered = vec![];
	for (statement, live_loop) in statements.into_iter().zip(&loops) {
		let Some(live_loop) = live_loop else {
			lowered.push(statement);
			continue;
		};
		let Some(name) = live_loop.name() else {
			lowered.push(live_loop.voice(std::slice::from_ref(&live_loop.block)));
			continue;
		};
		let version = versions.entry(name.to_string()).and_modify(|count| *count += 1).or_insert(1);
		let since = Node::Symbol(format!("{name}_since"));
		let filled = |template: &str, version: usize| {
			substituted(
				crate::warp_parser::parse(template),
				&[(LIVE_SINCE, since.clone()), (LIVE_VERSION, Node::int(version as i64))],
			)
		};
		let blocks = blocks_of(name);
		match (*version, blocks.len()) {
			(_, 1) => lowered.push(live_loop.voice(&blocks)),
			(1, count) => {
				lowered.push(substituted(
					crate::warp_parser::parse(LIVE_SINCE_DECLARATION),
					&[(LIVE_SINCE, since.clone()), (LIVE_COUNT, Node::int(count as i64))],
				));
				lowered.extend((2..=count).map(|later| filled(LIVE_SINCE_UNSET, later)));
				lowered.push(live_loop.voice_choosing(&blocks, &since));
			}
			(version, _) => lowered.push(live_loop.with_prefix(filled(LIVE_SINCE_SET, version))),
		}
	}
	lowered
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
		// `play hum for 2s` (lib play's `for duration`, preposition_calls.rs reads it for the other plays)
		Some((PLAY_WORD, [played, preposition, seconds]))
			if preposition.is_symbol(FOR_WORD)
				&& let Some(word) = signal_word(played, counts) =>
		{
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
		Node::List(items, bracket, separator) if !separator.separates_statements() => Node::List(
			with_recording(with_kind_text(with_unit_words(items, defined), defined), defined),
			bracket,
			separator,
		),
		Node::List(statements, bracket, separator) => {
			Node::List(with_scheduled(statements, defined), bracket, separator)
		}
		other => other,
	}
}

/// `save sound as "x.wav"` (`save` with `sound as "x.wav"`) and `save(sound, …)`: the sound a Recording(), unless the
/// program names something sound
fn with_recording(items: Vec<Node>, defined: &HashSet<String>) -> Vec<Node> {
	if !items.first().is_some_and(|word| word.is_symbol(SAVE_WORD)) || defined.contains(SOUND_WORD) {
		return items;
	}
	let recording = |item: Node| match item.drop_meta() {
		Node::Symbol(word) if word == SOUND_WORD => call(RECORDING_CLASS, vec![]).with_meta_of(&item),
		Node::Key(made, Op::As, file) if made.is_symbol(SOUND_WORD) => {
			key(call(RECORDING_CLASS, vec![]), Op::As, file.as_ref().clone())
		}
		_ => item,
	};
	items.into_iter().map(recording).collect()
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

/// `[at 2 beats] live_loop [name] [cycles] { body }`: its statements before the word (an `at`), its voice, count and block
struct LiveLoop {
	prefix: Vec<Node>,
	voice: Node,
	cycles: Option<Node>,
	block: Node,
	separator: Separator,
}

/// `live_loop 4 { body }`, `live_loop drums 4 { body }`, `live_loop drums { body }` (endless), after an `at` too; a
/// symbol before the block is the count when the program defines it (`live_loop n { … }`), else the voice's name
fn live_loop_of(statement: &Node, defined: &HashSet<String>) -> Option<LiveLoop> {
	let Node::List(items, _, separator) = statement.drop_meta() else {
		return None;
	};
	if separator.separates_statements() || defined.contains(LIVE_LOOP_WORD) {
		return None;
	}
	let start = items.iter().position(|item| item.is_symbol(LIVE_LOOP_WORD))?;
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if !defined.contains(name));
	let unnamed = || Node::Symbol(LIVE_VOICE.into());
	let (voice, cycles, block) = match &items[start + 1..] {
		[name, block] if is_name(name) => (name.clone(), None, block),
		[cycles, block] => (unnamed(), Some(cycles.clone()), block),
		[name, cycles, block] if is_name(name) => (name.clone(), Some(cycles.clone()), block),
		_ => return None,
	};
	let Node::List(_, Bracket::Curly, _) = block.drop_meta() else {
		return None;
	};
	Some(LiveLoop {
		prefix: items[..start].to_vec(),
		voice,
		cycles,
		block: block.clone(),
		separator: separator.clone(),
	})
}

impl LiveLoop {
	/// the voice's name, none for `live_loop 4 { … }`
	fn name(&self) -> Option<&str> {
		match self.voice.drop_meta() {
			Node::Symbol(name) if name != LIVE_VOICE => Some(name),
			_ => None,
		}
	}

	/// `drums = go { for live_cycle in 0..4 { body } }`
	fn voice(&self, blocks: &[Node]) -> Node {
		let template = if self.cycles.is_some() { LIVE_LOOP_TEMPLATE } else { ENDLESS_LIVE_LOOP_TEMPLATE };
		let made = substituted(
			crate::warp_parser::parse(template),
			&[
				(LIVE_VOICE, self.voice.clone()),
				(LIVE_CYCLES, self.cycles.clone().unwrap_or(Node::Empty)),
				(LIVE_BODY, self.announced(&blocks[0])),
			],
		);
		let voice = match made {
			Node::List(mut statements, _, _) if statements.len() == 1 => statements.remove(0),
			made => made,
		};
		self.with_prefix(voice)
	}

	/// the voice whose cycles each play the latest of `blocks` they reached: `if drums_since#2 … { second } else { first }`
	fn voice_choosing(&self, blocks: &[Node], since: &Node) -> Node {
		let choice = blocks.iter().enumerate().skip(1).fold(blocks[0].clone(), |earlier, (index, block)| {
			substituted(
				crate::warp_parser::parse(LIVE_VERSION_TEMPLATE),
				&[
					(LIVE_SINCE, since.clone()),
					(LIVE_VERSION, Node::int(index as i64 + 1)),
					(LIVE_MOMENT_THEN, block.clone()),
					(LIVE_MOMENT_ELSE, earlier),
				],
			)
		});
		let body = substituted(crate::warp_parser::parse(LIVE_CHOICE), &[(LIVE_CHOICE_WORD, choice)]);
		self.voice(&[Node::List(body.sequence_statements().to_vec(), Bracket::Curly, Separator::Newline)])
	}

	/// a named loop's block, saying its name first each cycle: `{ live_cycle_started("drums")\n body }`
	fn announced(&self, block: &Node) -> Node {
		let Some(name) = self.name() else { return block.clone() };
		let said = Node::List(
			vec![Node::Symbol(LIVE_CYCLE_STARTED.into()), Node::Text(name.into())],
			Bracket::Round,
			Separator::None,
		);
		let statements = std::iter::once(said).chain(block.body_statements()).collect();
		Node::List(statements, Bracket::Curly, Separator::Newline)
	}

	/// the statement after the loop's `at 2 beats`, if any
	fn with_prefix(&self, statement: Node) -> Node {
		match self.prefix.is_empty() {
			true => statement,
			false => Node::List([self.prefix.clone(), vec![statement]].concat(), Bracket::None, self.separator.clone()),
		}
	}
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
