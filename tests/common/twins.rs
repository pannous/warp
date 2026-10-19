//! The emitter and analyzer twins (notes/self_hosting.md): a warp-written compiler part in lib/compiler/ runs over
//! every sample and probe, judged by the program's value next to Rust's, until the pipeline runs completely in warp
//! (AGENTS.md "Outcome tests only"); the first file where the two differ fails the test. The gate checks every GATE_SHARE-th source (a different
//! share each day), the nightly run (tests/nightly.sh) all of them (user 2026-10-16, notes/test_speed.md).
use crate::common::nightly;
use std::path::{Path, PathBuf};
use warp::analyzer::{covers, kind_slots};
pub use warp::compiler::node_views::{analysis_view, plain_view, view_json};
use warp::host::with_captured_stdout;
use warp::{Node, Number};

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

/// The analyzer's twin (card inferred-kinds, notes/self_hosting.md step 14): each source the pipeline lowers goes to
/// `inferred_kinds` of lib/compiler/analyzer.warp as a node, its kinds compared with src/analyzer/inferred_kinds.rs
/// on the same program slot by slot: warp's kind is Rust's or a more general one (`covers`), never a narrower one
pub fn analyzer_differential(paths: &[PathBuf]) {
	let cases: Vec<(String, String, Vec<String>)> = paths
		.iter()
		.filter_map(|path| {
			let (program, kinds) = analyzed(&std::fs::read_to_string(path).ok()?)?;
			Some((path.display().to_string(), view_json(&program), kinds))
		})
		.collect();
	let twin = TwinProgram::new("analyzer", "inferred_kinds(node_of(json(argument)))");
	let mut differing = vec![];
	for cases in cases.chunks(SOURCES_PER_PROGRAM) {
		let arguments: Vec<&str> = cases.iter().map(|(_, argument, _)| argument.as_str()).collect();
		for ((path, _, rust), value) in cases.iter().zip(twin.values(&arguments)) {
			let warp: Vec<String> = match value.drop_meta() {
				Node::List(items, ..) => items.iter().filter_map(Node::text_value).collect(),
				other => vec![other.serialize()],
			};
			if !covers(&kind_slots(&warp), &kind_slots(rust)) {
				differing.push((path.clone(), warp.join(" | "), rust.join(" | ")));
			}
		}
	}
	report("inferred_kinds", &differing);
}

/// The view of the program after every lowering pass and the kinds the Rust analyzer infers in it; None for a program
/// refused or answered before code generation
pub fn analyzed(code: &str) -> Option<(Node, Vec<String>)> {
	let program = std::panic::catch_unwind(|| warp::pipeline::lower(code).ok()).ok()??;
	Some((analysis_view(&program), warp::analyzer::inferred_kinds(&program)))
}

/// The emitter core's twin (card emits-wasm, notes/self_hosting.md step 15): every lowered program (pipeline::lower)
/// goes to lib/compiler/emitter.warp `wasm_module` as a node, the module it gives runs, and its value is compared with
/// the program's value run by Rust. A program outside the core (a text "outside the core: …") is no case; fails
/// unless at least `least` programs are inside it. Returns the paths inside the core
pub fn emitter_differential(paths: &[PathBuf], least: usize) -> Vec<String> {
	let cases = in_parallel(paths.to_vec(), |path| {
		let text = std::fs::read_to_string(path).ok()?;
		let lowered = std::panic::catch_unwind(|| warp::pipeline::lower(&text)).ok()?.ok()?;
		Some((path.display().to_string(), view_json(&analysis_view(&lowered)), text))
	});
	// one file per call: tests running at once rewrote a shared file while another read it (truncated JSON)
	static RUNTIME_FILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
	let number = RUNTIME_FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let runtime = Path::new(INPUT_FOLDER).join(format!("emitter-runtime-{}-{number}.json", std::process::id()));
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
				refusals.entry(refusal).or_insert_with(Vec::new).push(path.clone());
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
		ranked.sort_by_key(|(_, paths)| std::cmp::Reverse(paths.len()));
		eprintln!("{} inside; outside the core:", inside.len());
		ranked.iter().for_each(|(refusal, paths)| {
			eprintln!("{:5} {refusal}  e.g. {}", paths.len(), paths[..paths.len().min(3)].join(" "))
		});
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

/// The text around its first character that differs from the other text
pub fn around_difference(text: &str, other: &str) -> String {
	let chars: Vec<char> = text.chars().collect();
	let first = chars.iter().zip(other.chars()).take_while(|(a, b)| *a == b).count();
	let start = first.saturating_sub(CONTEXT);
	chars[start..(first + CONTEXT).min(chars.len())].iter().collect()
}
