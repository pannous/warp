//! Twins, checked (card self-differential, notes/self_hosting.md step 4): a word of a warp-written compiler part in
//! lib/compiler/ runs over every sample and probe next to its Rust twin, and the first file where the two differ fails
//! the test with both texts around the first difference. The gate checks every GATE_SHARE-th source (a different
//! share each day), the nightly run (tests/nightly.sh) all of them (user 2026-10-16, notes/test_speed.md).
use crate::common::nightly;
use std::path::{Path, PathBuf};
use warp::warp_parser::parse;
use warp::wasm_emitter::eval;
use warp::{Bracket, Node, Number, Separator};

const SOURCE_FOLDERS: [&str; 2] = ["samples", "probes"];
const SOURCE_EXTENSION: &str = "warp";
const COMPILER_FOLDER: &str = "lib/compiler";
/// Characters shown on each side of the first difference
const CONTEXT: usize = 40;
/// Sources per evaluated program: each program's store is freed before the next, which keeps the test under 1 GB
const SOURCES_PER_PROGRAM: usize = 64;
/// Differing sources named after the first
const LISTED: usize = 12;
/// Outside the nightly run the twins check every GATE_SHARE-th source: they were 1100 s of the suite's test time
const GATE_SHARE: usize = 4;
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// Every sample and probe source, sorted; a file that is not UTF-8 text is no source
pub fn sources() -> Vec<PathBuf> {
	let mut found = Vec::new();
	for folder in SOURCE_FOLDERS {
		collect_sources(Path::new(folder), &mut found);
	}
	found.retain(|path| std::fs::read_to_string(path).is_ok());
	found.sort();
	found
}

fn collect_sources(folder: &Path, found: &mut Vec<PathBuf>) {
	for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_sources(&path, found);
		} else if path.extension().is_some_and(|extension| extension == SOURCE_EXTENSION) {
			found.push(path);
		}
	}
}

/// A Rust twin's list as warp writes it: the empty list is ø
pub fn warp_list(items: impl Iterator<Item = Node>) -> Node {
	let items: Vec<Node> = items.collect();
	if items.is_empty() {
		return Node::Empty;
	}
	Node::List(items, Bracket::Square, Separator::None)
}

/// A node as plain warp data, the view the parser and serializer twins share (notes/self_hosting.md): a number,
/// bool or ø is itself, every other node a list tagged by its kind
pub fn node_view(node: &Node) -> Node {
	match node {
		Node::True | Node::False | Node::Empty => node.clone(),
		Node::Number(number @ (Number::Int(_) | Number::Float(_))) => Node::Number(*number),
		Node::Number(number) => tagged("number", [Node::text(&number.to_string())]),
		Node::Char(c) => tagged("char", [Node::Char(*c)]),
		Node::Text(text) => tagged("text", [Node::text(text)]),
		Node::Symbol(name) => tagged("symbol", [Node::text(name)]),
		Node::Error(inner) => tagged("error", [node_view(inner)]),
		Node::Key(left, op, right) => tagged("key", [Node::text(op.as_str()), node_view(left), node_view(right)]),
		Node::List(items, bracket, separator) => {
			let mut parts = vec![Node::text(&bracket_glyphs(bracket)), Node::text(&separator_glyph(separator))];
			parts.extend(items.iter().map(node_view));
			tagged("list", parts)
		}
		Node::Type { name, body } => tagged("type", [node_view(name), node_view(body)]),
		Node::Data(data) => tagged(
			"data",
			[Node::text(&warp::numbers::units::describe(data).unwrap_or_else(|| format!("Data({data:?})")))],
		),
		Node::Meta { node: inner, .. } => meta_view(node, inner),
	}
}

/// A source literal (`01234`, `1.10`) and an annotation (`@name(value)`) survive, comments and positions go
fn meta_view(node: &Node, inner: &Node) -> Node {
	if let Some(literal) = node.source_literal() {
		return tagged("literal", [Node::text(literal), node_view(inner)]);
	}
	let attributes = node.attributes();
	if attributes.len() > inner.attributes().len() {
		let (name, value) = attributes[0];
		return tagged("attribute", [Node::text(name), node_view(value), node_view(inner)]);
	}
	node_view(inner)
}

fn tagged(kind: &str, parts: impl IntoIterator<Item = Node>) -> Node {
	warp_list(std::iter::once(Node::text(kind)).chain(parts))
}

/// The opening glyph ("{", "[", "(", "<", "" for none), both glyphs for another pair
fn bracket_glyphs(bracket: &Bracket) -> String {
	match bracket {
		Bracket::None => String::new(),
		Bracket::Other(open, close) => format!("{open}{close}"),
		_ => bracket.opening().to_string(),
	}
}

fn separator_glyph(separator: &Separator) -> String {
	separator.to_char().map(String::from).unwrap_or_default()
}

/// `word(read(path))` of lib/compiler/<part>.warp for every source, each written as the Rust twin's value of the same
/// text is written; fails with the first difference and the names of the next sources that differ
pub fn differential(part: &str, word: &str, rust_twin: impl Fn(&str) -> Node) {
	differential_on(&sources(), part, word, rust_twin)
}

/// `differential` over the given sources only
pub fn differential_on(paths: &[PathBuf], part: &str, word: &str, rust_twin: impl Fn(&str) -> Node) {
	report(word, &differences(&checked_share(paths), part, word, rust_twin));
}

/// All sources in the nightly run, every GATE_SHARE-th otherwise, starting at the day's offset: the gates of four
/// days sweep every source at the cost of one share
fn checked_share(paths: &[PathBuf]) -> Vec<PathBuf> {
	if nightly() {
		return paths.to_vec();
	}
	paths.iter().skip(todays_offset()).step_by(GATE_SHARE).cloned().collect()
}

fn todays_offset() -> usize {
	let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
	(since_epoch.as_secs() / SECONDS_PER_DAY) as usize % GATE_SHARE
}

/// `word(view)` of lib/compiler/<part>.warp for the node view of every parsed source, against the Rust parse of the
/// source serialized: the serializer twin
pub fn view_differential(part: &str, word: &str) {
	let view_of_source = |_: &Path, text: &str| json_argument(&node_view(&parse(text)));
	let serialized = |text: &str| Node::text(&parse(text).serialize());
	report(word, &differences_with(&checked_share(&sources()), part, word, view_of_source, serialized));
}

/// Fails with the first difference and the names of the next sources that differ
fn report(word: &str, differing: &[(String, String, String)]) {
	let Some((path, warp_text, rust_text)) = differing.first() else {
		return;
	};
	let others: Vec<&str> = differing[1..].iter().take(LISTED).map(|(path, _, _)| path.as_str()).collect();
	panic!(
		"{word} differs from its Rust twin on {} sources, first on {path}:\n  warp: …{}…\n  rust: …{}…\nnext: {}",
		differing.len(),
		around_difference(warp_text, rust_text),
		around_difference(rust_text, warp_text),
		others.join(" ")
	);
}

/// The sources where the twins differ: path, warp's and Rust's text
pub fn differences(
	paths: &[PathBuf],
	part: &str,
	word: &str,
	rust_twin: impl Fn(&str) -> Node,
) -> Vec<(String, String, String)> {
	let read_source = |path: &Path, _: &str| format!("read(\"{}\")", path.display());
	differences_with(paths, part, word, read_source, rust_twin)
}

/// The sources where `word(argument)` differs from the Rust twin, the argument written from the source's path and text
fn differences_with(
	paths: &[PathBuf],
	part: &str,
	word: &str,
	argument: impl Fn(&Path, &str) -> String,
	rust_twin: impl Fn(&str) -> Node,
) -> Vec<(String, String, String)> {
	let cases = paths.iter().map(|path| {
		let text = std::fs::read_to_string(path).unwrap();
		let call = format!("{word}({})", argument(path, &text));
		Case { path: path.display().to_string(), call, expected: rust_twin(&text) }
	});
	differences_of(cases, part)
}

/// One source of a differential: the call of the warp twin and the value the Rust twin gave
struct Case {
	path: String,
	call: String,
	expected: Node,
}

/// The cases where the warp call's value differs from the expected value: path, warp's and Rust's text
fn differences_of(cases: impl Iterator<Item = Case>, part: &str) -> Vec<(String, String, String)> {
	let cases: Vec<Case> = cases.collect();
	let mut differing = vec![];
	for cases in cases.chunks(SOURCES_PER_PROGRAM) {
		let calls: Vec<String> = cases.iter().map(|case| case.call.clone()).collect();
		for (case, value) in cases.iter().zip(warp_values(part, &calls)) {
			let (warp_text, rust_text) =
				(as_compared(value).serialize(), as_compared(case.expected.clone()).serialize());
			if warp_text != rust_text {
				differing.push((case.path.clone(), warp_text, rust_text));
			}
		}
	}
	differing
}

/// A lowering pass's twin (card twin-harness, notes/self_hosting.md step 10): for every source the Rust passes before
/// `pass` run, the program goes to `word` of lib/compiler/<part>.warp as a node (walk.warp node_of), and the view of
/// its result is compared with the view after the Rust pass. Literal and attribute layers are left out on both sides
/// (a warp node has none); a source the pipeline refuses before the passes, or whose passes fail, is no case.
pub fn pass_differential(pass: fn(Node) -> Node, part: &str, word: &str) {
	let cases = checked_share(&sources()).into_iter().filter_map(|path| {
		let text = std::fs::read_to_string(&path).ok()?;
		let (before, after) = std::panic::catch_unwind(|| warp::pipeline::around_source_pass(&text, pass)).ok()??;
		let call = format!("view_of({word}(node_of({})))", json_argument(&plain_view(&before)));
		Some(Case { path: path.display().to_string(), call, expected: plain_view(&after) })
	});
	report(word, &differences_of(cases, part));
}

/// The bridge every pass twin relies on: the view of each parsed source, made a node and viewed again, is itself
pub fn bridge_differential() {
	let cases = checked_share(&sources()).into_iter().map(|path| {
		let view = plain_view(&parse(&std::fs::read_to_string(&path).unwrap()));
		let call = format!("view_of(node_of({}))", json_argument(&view));
		Case { path: path.display().to_string(), call, expected: view }
	});
	report("view_of", &differences_of(cases, "walk"));
}

/// A node's view as a warp node gives it back: no literal and attribute layers, a character as a text (warp holds a
/// one-character text as the character, card character-text)
fn plain_view(node: &Node) -> Node {
	plain(node_view(node))
}

fn plain(view: Node) -> Node {
	match view {
		Node::List(mut items, bracket, separator) => match items.first().and_then(Node::text_value).as_deref() {
			Some("literal" | "attribute") => plain(items.pop().unwrap_or(Node::Empty)),
			Some("char") => {
				items[0] = Node::text("text");
				Node::List(items, bracket, separator)
			}
			_ => Node::List(items.into_iter().map(plain).collect(), bracket, separator),
		},
		other => other,
	}
}

/// A view as the warp code reading it: JSON text given to `json(…)`
fn json_argument(view: &Node) -> String {
	format!("json({})", warp_text_literal(&view_json(view)))
}

/// A value with each one-character text as its character: warp compares `"a" == 'a'`, holds a literal `"a"` as the
/// character and a one-character text passed on or cut at run time as a text (card character-text)
fn as_compared(value: Node) -> Node {
	match value {
		Node::Text(text) if text.chars().count() == 1 => Node::Char(text.chars().next().expect("one character")),
		Node::List(items, bracket, separator) => {
			Node::List(items.into_iter().map(as_compared).collect(), bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(as_compared(*left)), op, Box::new(as_compared(*right))),
		other => other,
	}
}

/// The calls' values, in one program; when it fails, each half alone, down to a single call that gives the warp error
/// as its value (so it differs from the Rust twin instead of hiding the other sources)
fn warp_values(part: &str, calls: &[String]) -> Vec<Node> {
	match warp_values_at_once(part, calls) {
		Ok(values) => values,
		Err(failure) if calls.len() == 1 => vec![Node::text(&format!("warp failed: {failure}"))],
		Err(_) => {
			let (first, second) = calls.split_at(calls.len() / 2);
			let mut values = warp_values(part, first);
			values.extend(warp_values(part, second));
			values
		}
	}
}

fn warp_values_at_once(part: &str, calls: &[String]) -> Result<Vec<Node>, String> {
	let program = format!("use \"{COMPILER_FOLDER}/{part}.warp\"\n[{}]", calls.join(",\n"));
	let result = eval(&program);
	match result.drop_meta() {
		Node::List(values, ..) if values.len() == calls.len() => Ok(values.clone()),
		other => Err(other.serialize()),
	}
}

/// A node view as JSON (ø is null): the warp twin reads it with `json(…)`, data the host parses, where the same view
/// as a warp literal is code to compile (a 100 KB chunk of sources took 2.4 GB in a debug build)
pub fn view_json(view: &Node) -> String {
	match view {
		Node::Text(text) => json_text(text),
		Node::Char(character) => json_text(&character.to_string()),
		Node::List(items, ..) => format!("[{}]", items.iter().map(view_json).collect::<Vec<_>>().join(", ")),
		Node::Empty => "null".into(),
		Node::True => "true".into(),
		Node::False => "false".into(),
		number => number.serialize(),
	}
}

fn json_text(text: &str) -> String {
	quoted(text, |character| character.is_control().then(|| format!("\\u{:04x}", character as u32)))
}

/// A text as a warp text literal: `${…}` would be a hole
fn warp_text_literal(text: &str) -> String {
	quoted(text, |character| (character == '$').then(|| "\\$".into()))
}

/// The text in double quotes, its quotes, backslashes, newlines, tabs and returns escaped, other characters by `extra`
fn quoted(text: &str, extra: impl Fn(char) -> Option<String>) -> String {
	let escaped: String = text
		.chars()
		.map(|character| match character {
			'"' | '\\' => format!("\\{character}"),
			'\n' => "\\n".into(),
			'\t' => "\\t".into(),
			'\r' => "\\r".into(),
			other => extra(other).unwrap_or_else(|| other.to_string()),
		})
		.collect();
	format!("\"{escaped}\"")
}

/// The text around its first character that differs from the other text
pub fn around_difference(text: &str, other: &str) -> String {
	let chars: Vec<char> = text.chars().collect();
	let first = chars.iter().zip(other.chars()).take_while(|(a, b)| *a == b).count();
	let start = first.saturating_sub(CONTEXT);
	chars[start..(first + CONTEXT).min(chars.len())].iter().collect()
}
