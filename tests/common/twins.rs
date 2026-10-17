//! Twins, checked (card self-differential, notes/self_hosting.md step 4): a word of a warp-written compiler part in
//! lib/compiler/ runs over every sample and probe next to its Rust twin, and the first file where the two differ fails
//! the test with both texts around the first difference. The gate checks every GATE_SHARE-th source (a different
//! share each day), the nightly run (tests/nightly.sh) all of them (user 2026-10-16, notes/test_speed.md).
use crate::common::nightly;
use std::path::{Path, PathBuf};
use warp::host::with_captured_stdout;
use warp::warp_parser::parse;
use warp::{Bracket, Node, Number, Separator};

const SOURCE_FOLDERS: [&str; 2] = ["samples", "probes"];
const SOURCE_EXTENSION: &str = "warp";
const COMPILER_FOLDER: &str = "lib/compiler";
/// The input files of the twin programs, the arguments of their calls (scratch/: temporary, ignored)
const INPUT_FOLDER: &str = "scratch/twins";
/// The fringe forms a pass twin leaves out, with how to bring each back
const DROPPED_NOTES: &str = "notes/self_hosting_dropped.md";
/// Characters shown on each side of the first difference
const CONTEXT: usize = 40;
/// Sources per evaluated program: each program's store is freed before the next, which keeps the test under 1 GB
const SOURCES_PER_PROGRAM: usize = 64;
/// Differing sources named after the first
const LISTED: usize = 12;
/// Outside the nightly run the twins check every GATE_SHARE-th source: they were 1100 s of the suite's test time
const GATE_SHARE: usize = 4;
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;
/// Threads computing the Rust side of a pass differential: the passes before the pass lower every `use`d module again
/// for each source, 95 s for 352 sources in one thread (card pass-twins)
const RUST_SIDE_THREADS: usize = 4;
/// Stack of each of those threads: the passes recurse as deeply as a test thread's
const RUST_SIDE_STACK: usize = 64 << 20;

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
	view(node, false)
}

/// The view with the source position of each node that has one, ["at", line, column, view] (card carries-line): a pass
/// twin reports where its Rust twin does
pub fn positioned_view(node: &Node) -> Node {
	view(node, true)
}

fn view(node: &Node, positions: bool) -> Node {
	let node_view = |node: &Node| view(node, positions);
	match node {
		Node::True | Node::False | Node::Empty => node.clone(),
		Node::Number(number @ (Number::Int(_) | Number::Float(_))) => Node::Number(*number),
		Node::Number(number) => tagged("number", [Node::text(&number.to_string())]),
		Node::Char(c) => tagged("char", [Node::Char(*c)]),
		Node::Text(text) => tagged("text", [Node::text(text)]),
		Node::Symbol(name) => tagged("symbol", [Node::text(name)]),
		// an error and a type stay opaque to a pass twin (walk.warp node_of): no positions inside
		Node::Error(inner) => tagged("error", [view(inner, false)]),
		Node::Key(left, op, right) => tagged("key", [Node::text(op.as_str()), node_view(left), node_view(right)]),
		Node::List(items, bracket, separator) => {
			let mut parts = vec![Node::text(&bracket_glyphs(bracket)), Node::text(&separator_glyph(separator))];
			parts.extend(items.iter().map(node_view));
			tagged("list", parts)
		}
		Node::Type { name, body } => tagged("type", [view(name, false), view(body, false)]),
		Node::Data(data) => tagged(
			"data",
			[Node::text(&warp::numbers::units::describe(data).unwrap_or_else(|| format!("Data({data:?})")))],
		),
		Node::Meta { node: inner, .. } => match node.get_lineinfo() {
			Some(info) if positions && inner.get_lineinfo().is_none() => {
				let at = [Node::int(info.line_nr as i64), Node::int(info.column as i64)];
				tagged("at", at.into_iter().chain([meta_view(node, inner, positions)]))
			}
			_ => meta_view(node, inner, positions),
		},
	}
}

/// A source literal (`01234`, `1.10`) and an annotation (`@name(value)`) survive, comments go
fn meta_view(node: &Node, inner: &Node, positions: bool) -> Node {
	if let Some(template) = warp::text::interpolation::template_of(node) {
		return tagged("template", [Node::text(template)]);
	}
	if let Some(literal) = node.source_literal() {
		return tagged("literal", [Node::text(literal), view(inner, positions)]);
	}
	let attributes = node.attributes();
	if attributes.len() > inner.attributes().len() {
		let (name, value) = attributes[0];
		return tagged("attribute", [Node::text(name), view(value, positions), view(inner, positions)]);
	}
	view(inner, positions)
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
	let view_of_source = |_: &Path, text: &str| view_json(&node_view(&parse(text)));
	let serialized = |text: &str| Node::text(&parse(text).serialize());
	let call = format!("{word}(json(argument))");
	report(word, &differences_with(&checked_share(&sources()), part, &call, view_of_source, serialized));
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
	let source_path = |path: &Path, _: &str| path.display().to_string();
	differences_with(paths, part, &format!("{word}(read(argument))"), source_path, rust_twin)
}

/// The sources where `call` differs from the Rust twin, its `argument` written from the source's path and text
fn differences_with(
	paths: &[PathBuf],
	part: &str,
	call: &str,
	argument: impl Fn(&Path, &str) -> String,
	rust_twin: impl Fn(&str) -> Node,
) -> Vec<(String, String, String)> {
	let cases = paths.iter().map(|path| {
		let text = std::fs::read_to_string(path).unwrap();
		Case::new(path, argument(path, &text), rust_twin(&text))
	});
	differences_of(cases, part, call)
}

/// One source of a differential: the text the warp twin's call gets as `argument` (a path, a view as JSON) and the
/// value the Rust twin gave, as compared (text: a case is made in one thread and checked in another)
struct Case {
	path: String,
	argument: String,
	expected: String,
}

impl Case {
	fn new(path: &Path, argument: String, expected: Node) -> Self {
		Case { path: path.display().to_string(), argument, expected: as_compared(expected).serialize() }
	}
}

/// The cases where `call`, a warp expression of `argument`, differs from the expected value: path, warp's and Rust's
/// text. One program serves every case: it reads the arguments at run time, so it compiles once (program_cache) for
/// all chunks and for the halves of a chunk that fails (card pass-twins)
fn differences_of(cases: impl Iterator<Item = Case>, part: &str, call: &str) -> Vec<(String, String, String)> {
	let cases: Vec<Case> = cases.collect();
	let twin = TwinProgram::new(part, call);
	let mut differing = vec![];
	for cases in cases.chunks(SOURCES_PER_PROGRAM) {
		let arguments: Vec<&str> = cases.iter().map(|case| case.argument.as_str()).collect();
		for (case, value) in cases.iter().zip(twin.values(&arguments)) {
			let warp_text = as_compared(value).serialize();
			if warp_text != case.expected {
				differing.push((case.path.clone(), warp_text, case.expected.clone()));
			}
		}
	}
	differing
}

/// A lowering pass's twin (card twin-harness, notes/self_hosting.md step 10): for every source the Rust passes before
/// `pass` run, the program goes to `word` of lib/compiler/<part>.warp as a node (walk.warp node_of), and the view of
/// its result is compared with the view after the Rust pass, next to what each reported (walk.warp report) as
/// [line, column, message, preferred]. Literal and attribute layers are left out on both sides (a warp node has none);
/// the nodes carry their positions where the Rust pass reports or makes an error (card carries-line). A source the
/// pipeline refuses before the passes, or whose passes fail, is no case.
/// `dropped`: the sources of the fringe forms the twin leaves out, each named in DROPPED_NOTES with how to bring it
/// back (user 2026-10-17: the port keeps the clean core, not every Rust case)
pub fn pass_differential(pass: fn(Node) -> Node, part: &str, word: &str, dropped: &[&str]) {
	let cases = in_parallel(twin_sources(dropped), |path| {
		let text = std::fs::read_to_string(path).ok()?;
		let reporting = || warp::pipeline::around_source_pass_reporting(&text, pass);
		let (before, after, diagnostics) = std::panic::catch_unwind(reporting).ok()??;
		let reports = warp_list(diagnostics.iter().map(report_view));
		let after = plain_view(&after);
		let given = if reports != Node::Empty || mentions_error(&after) { positioned_plain_view } else { plain_view };
		let given = given(&before);
		let expected = warp_list([after, reports].into_iter());
		Some(Case::new(path, view_json(&given), expected))
	});
	report(
		word,
		&differences_of(cases.into_iter(), part, &format!("reported(view_of({word}(node_of(json(argument)))))")),
	);
}

/// `case` of each path, in the paths' order, computed in RUST_SIDE_THREADS threads; each thread takes every
/// RUST_SIDE_THREADS-th path, so the slow sources of one folder spread over all of them
fn in_parallel<T: Send>(paths: Vec<PathBuf>, case: impl Fn(&Path) -> Option<T> + Sync) -> Vec<T> {
	let (paths, case) = (&paths, &case);
	let mut numbered: Vec<(usize, T)> = std::thread::scope(|scope| {
		let threads: Vec<_> = (0..RUST_SIDE_THREADS)
			.map(|first| {
				let share = move || {
					let mine = (first..paths.len()).step_by(RUST_SIDE_THREADS);
					mine.filter_map(|index| Some((index, case(&paths[index])?))).collect::<Vec<_>>()
				};
				std::thread::Builder::new().stack_size(RUST_SIDE_STACK).spawn_scoped(scope, share).unwrap()
			})
			.collect();
		threads.into_iter().flat_map(|thread| thread.join().unwrap()).collect()
	});
	numbered.sort_by_key(|(index, _)| *index);
	numbered.into_iter().map(|(_, case)| case).collect()
}

/// The warp pipeline so far (card parser-tree, notes/self_hosting.md step 13): `word(source text)` of
/// lib/compiler/<part>.warp parses with parser.warp and runs the pass twins in pipeline order; compared with the Rust
/// parse of the same text after the Rust twins of those passes in that order (only those: the others have no twin yet),
/// over the day's share of the sources as the parser differential, without the `dropped` ones (twin_sources)
pub fn chain_differential(
	paths: &[PathBuf],
	part: &str,
	word: &str,
	rust_passes: &[fn(Node) -> Node],
	dropped: &[&str],
) {
	let cases = twin_sources_of(paths, dropped).into_iter().filter_map(|path| {
		let text = std::fs::read_to_string(&path).ok()?;
		let lowered =
			std::panic::catch_unwind(|| rust_passes.iter().fold(parse(&text), |node, pass| pass(node))).ok()?;
		Some(Case::new(&path, path.display().to_string(), plain_view(&lowered)))
	});
	report(word, &differences_of(cases, part, &format!("{word}(read(argument))")));
}

/// A hint or warning as a twin reports it: [line, column, message, preferred form or ø]
fn report_view(diagnostic: &warp::errors::diagnostic::Diagnostic) -> Node {
	let preferred = diagnostic.preferred.as_deref().map_or(Node::Empty, Node::text);
	let parts =
		[Node::int(diagnostic.line as i64), Node::int(diagnostic.column as i64), Node::text(&diagnostic.message)];
	warp_list(parts.into_iter().chain([preferred]))
}

fn mentions_error(view: &Node) -> bool {
	match view {
		Node::List(items, ..) => {
			items.first().and_then(Node::text_value).as_deref() == Some("error") || items.iter().any(mentions_error)
		}
		_ => false,
	}
}

/// The bridge every pass twin relies on: the view of each parsed source, made a node and viewed again, is itself
pub fn bridge_differential() {
	bridge_differential_of(plain_view)
}

/// The bridge with positions: a positioned node is the node, its position read by `n.line` and `n.column`
pub fn positioned_bridge_differential() {
	bridge_differential_of(positioned_plain_view)
}

fn bridge_differential_of(given: fn(&Node) -> Node) {
	let cases = twin_sources(&[]).into_iter().map(|path| {
		let parsed = parse(&std::fs::read_to_string(&path).unwrap());
		Case::new(&path, view_json(&given(&parsed)), plain_view(&parsed))
	});
	report("view_of", &differences_of(cases, "walk", "view_of(node_of(json(argument)))"));
}

/// The emitter core's twin (card emits-wasm, notes/self_hosting.md step 15): every lowered program (pipeline::lower)
/// goes to lib/compiler/emitter.warp `wasm_module` as a node, the module it gives runs, and its value is compared with
/// the program's value run by Rust. A program outside the core (a text "outside the core: …") is no case; fails
/// unless at least `least` programs are inside it. Returns the paths inside the core
pub fn emitter_differential(paths: &[PathBuf], least: usize) -> Vec<String> {
	let cases = in_parallel(paths.to_vec(), |path| {
		let text = std::fs::read_to_string(path).ok()?;
		let lowered = std::panic::catch_unwind(|| warp::pipeline::lower(&text)).ok()?.ok()?;
		Some((path.display().to_string(), view_json(&plain_view(&lowered)), text))
	});
	let runtime = Path::new(INPUT_FOLDER).join(format!("emitter-runtime-{}.json", std::process::id()));
	let runtime_source = std::fs::read_to_string(Path::new(COMPILER_FOLDER).join(RUNTIME_WORDS)).unwrap();
	let lowered_runtime = warp::pipeline::lower(&runtime_source).expect("lib/compiler/runtime.warp lowers");
	std::fs::create_dir_all(INPUT_FOLDER).unwrap();
	std::fs::write(&runtime, view_json(&plain_view(&lowered_runtime))).unwrap();
	let setup = format!("runtime_words = node_of(json(read(\"{}\")))", runtime.display());
	let twin = TwinProgram::with_setup("emitter", &setup, "wasm_module(node_of(json(argument)), runtime_words)");
	let (mut inside, mut differing, mut refusals) = (vec![], vec![], std::collections::BTreeMap::new());
	for cases in cases.chunks(SOURCES_PER_PROGRAM) {
		let arguments: Vec<&str> = cases.iter().map(|(_, view, _)| view.as_str()).collect();
		for ((path, _, text), module) in cases.iter().zip(twin.values(&arguments)) {
			if let Some(refusal) = module.text_value().filter(|text| text.starts_with(OUTSIDE_THE_CORE)) {
				*refusals.entry(refusal).or_insert(0) += 1;
				continue;
			}
			inside.push(path.clone());
			let (warp_value, warp_output) = with_captured_stdout(|| module_value(&module));
			let (rust_value, rust_output) = with_captured_stdout(|| warp::wasm_emitter::eval(text).drop_meta().clone());
			if !same_value(&warp_value, &rust_value) || warp_output != rust_output {
				let shown = |value: &Node, output: &str| format!("{} printing {output:?}", value.serialize());
				differing.push((path.clone(), shown(&warp_value, &warp_output), shown(&rust_value, &rust_output)));
			}
		}
	}
	if std::env::var_os(REFUSALS_REPORT).is_some() {
		let mut ranked: Vec<_> = refusals.into_iter().collect();
		ranked.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
		eprintln!("{} inside; outside the core:", inside.len());
		ranked.iter().for_each(|(refusal, count)| eprintln!("{count:5} {refusal}"));
	}
	let _ = std::fs::remove_file(&runtime);
	report("wasm_module", &differing);
	assert!(
		inside.len() >= least,
		"only {} programs inside the emitter core, {least} expected: {inside:?}",
		inside.len()
	);
	inside
}

/// The runtime words emitter.warp compiles into a module, lowered like a program, in COMPILER_FOLDER
const RUNTIME_WORDS: &str = "runtime.warp";
/// What lib/compiler/emitter.warp says of a program it leaves to the Rust emitter
const OUTSIDE_THE_CORE: &str = "outside the core";
/// Set, the emitter twin lists why the sources outside the core are outside, the most frequent first
const REFUSALS_REPORT: &str = "EMITTER_REFUSALS";

/// The value main of a module given as a list of bytes returns (its prints through WASI), or the failure as a text
fn module_value(module: &Node) -> Node {
	let Node::List(items, ..) = module.drop_meta() else {
		return Node::text(&format!("no module: {}", module.serialize()));
	};
	let byte = |item: &Node| match item.drop_meta() {
		Node::Number(Number::Int(byte)) => u8::try_from(*byte).ok(),
		_ => None,
	};
	let bytes: Option<Vec<u8>> = items.iter().map(byte).collect();
	let wasi = warp::run::wasm_reader::Imports { wasi: true, ..Default::default() };
	match bytes.map(|bytes| warp::run::wasm_reader::read_bytes_with_imports(&bytes, wasi)) {
		Some(Ok(value)) => value,
		Some(Err(failure)) => Node::text(&format!("{MODULE_FAILED}{failure}")),
		None => Node::text("a module byte beyond 0..255"),
	}
}

const MODULE_FAILED: &str = "module failed: ";

/// Equal values; a float equals another number of exactly its value (the core's 3.5 * 2 is 7.0, Rust's 7); a failure on
/// both sides is the same outcome: emitter.warp's module traps where Rust's gives its error (an index out of range,
/// probes/readme/oob.warp; a million nested calls, probes/functions/deep_recursion.warp)
fn same_value(warp_value: &Node, rust_value: &Node) -> bool {
	match (warp_value, rust_value) {
		(Node::Text(failure), Node::Error(_)) => failure.starts_with(MODULE_FAILED),
		(Node::Number(a @ Number::Float(_)), Node::Number(b))
		| (Node::Number(a), Node::Number(b @ Number::Float(_))) => f64::from(*a) == f64::from(*b),
		_ => warp_value == rust_value,
	}
}

/// The checked share of the sources without the dropped ones, which DROPPED_NOTES must name; TWIN_ONLY=<path suffix>
/// checks only the sources ending so (one source while fixing a twin)
pub fn twin_sources(dropped: &[&str]) -> Vec<PathBuf> {
	twin_sources_of(&sources(), dropped)
}

fn twin_sources_of(paths: &[PathBuf], dropped: &[&str]) -> Vec<PathBuf> {
	let notes = std::fs::read_to_string(DROPPED_NOTES).unwrap_or_default();
	let unexplained: Vec<&&str> = dropped.iter().filter(|source| !notes.contains(**source)).collect();
	assert!(unexplained.is_empty(), "dropped sources not named in {DROPPED_NOTES}: {unexplained:?}");
	let kept = |path: &PathBuf| !dropped.iter().any(|source| path.ends_with(source));
	match std::env::var("TWIN_ONLY") {
		Ok(only) => paths.iter().filter(|path| path.ends_with(&only)).cloned().collect(),
		Err(_) => checked_share(paths).into_iter().filter(kept).collect(),
	}
}

/// A node's view as a warp node gives it back: no literal and attribute layers, a character as a text (warp holds a
/// one-character text as the character, card character-text), an empty group as ø (warp's empty `{}` / `()` is ø,
/// decision empty-group-node)
fn plain_view(node: &Node) -> Node {
	plain(node_view(node))
}

/// The plain view with the positions: ["at", line, column, view] around each positioned node
fn positioned_plain_view(node: &Node) -> Node {
	plain(positioned_view(node))
}

fn plain(view: Node) -> Node {
	match view {
		Node::List(mut items, bracket, separator) => match items.first().and_then(Node::text_value).as_deref() {
			Some("literal" | "attribute") => plain(items.pop().unwrap_or(Node::Empty)),
			Some("list") if items.len() == 3 => Node::Empty,
			Some("char") => {
				items[0] = Node::text("text");
				Node::List(items, bracket, separator)
			}
			_ => Node::List(items.into_iter().map(plain).collect(), bracket, separator),
		},
		other => other,
	}
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

/// The program of a differential: `call` of each argument of the input file, one per line (JSON and paths have no
/// newline), compiled once and run for each chunk of arguments and each half of a failing one
struct TwinProgram {
	module: Result<warp::pipeline::CompiledModule, Node>,
	input: PathBuf,
}

impl TwinProgram {
	fn new(part: &str, call: &str) -> Self {
		Self::with_setup(part, "", call)
	}

	/// As new, with `setup` run once before the calls (a value every call shares)
	fn with_setup(part: &str, setup: &str, call: &str) -> Self {
		static DIFFERENTIALS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
		let number = DIFFERENTIALS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		std::fs::create_dir_all(INPUT_FOLDER).unwrap();
		let input = Path::new(INPUT_FOLDER).join(format!("{part}-{}-{number}.txt", std::process::id()));
		let program = format!(
			"use \"{COMPILER_FOLDER}/{part}.warp\"\n{setup}\ntwin_case(argument: text) := {call}\ntwin_values = []\n\
			 for argument in read(\"{}\").split(\"\\n\") {{ twin_values += [twin_case(argument)] }}\ntwin_values",
			input.display()
		);
		TwinProgram { module: warp::pipeline::compile(&program), input }
	}

	/// The call's value for each argument; when the program fails, each half alone, down to a single argument that
	/// gives the warp error as its value (so it differs from the Rust twin instead of hiding the other sources)
	fn values(&self, arguments: &[&str]) -> Vec<Node> {
		match self.values_at_once(arguments) {
			Ok(values) => values,
			Err(failure) if arguments.len() == 1 => vec![Node::text(&format!("warp failed: {failure}"))],
			Err(_) => {
				let (first, second) = arguments.split_at(arguments.len() / 2);
				let mut values = self.values(first);
				values.extend(self.values(second));
				values
			}
		}
	}

	fn values_at_once(&self, arguments: &[&str]) -> Result<Vec<Node>, String> {
		assert!(arguments.iter().all(|argument| !argument.contains('\n')), "an argument is one line");
		std::fs::write(&self.input, arguments.join("\n")).unwrap();
		let value = match &self.module {
			Ok(module) => warp::pipeline::run_module(module.clone()),
			Err(failure) => failure.clone(),
		};
		match value.drop_meta() {
			Node::List(values, ..) if values.len() == arguments.len() => Ok(values.clone()),
			other => Err(other.serialize()),
		}
	}
}

impl Drop for TwinProgram {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.input);
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
