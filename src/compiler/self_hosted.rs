//! The compile through the warp-written compiler (card warp-pipeline-suite, notes/self_hosting.md): with
//! WARP_SELF_HOSTED set, lib/compiler/parser.warp parses each program and every Rust pass with a warp twin in
//! lib/compiler/passes.warp gives way to it, and emitter.warp emits each program inside its core; the passes without
//! a twin and the Rust emitter do the rest. The suite run this way (tests/self_hosted.sh) counts how much of warp the
//! warp compiler already compiles.
//! A stage is a warp program answering one request, compiled once per worker thread: the compile of the stage program
//! has thread-local state of its own (diagnostics, compile context), which a compile it runs in the middle of must keep.
use crate::compiler::node_views::{analysis_view, handled_plain_view, json_text, node_of_handled_view, view_json};
use crate::node::Node;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

const SWITCH: &str = "WARP_SELF_HOSTED";
/// WARP_SELF_HOSTED=front: parser.warp and the pass twins, the Rust emitter (a suite count of the front end alone)
const FRONT_END_ONLY: &str = "front";
/// WARP_SELF_HOSTED=emit: Rust's front end, emitter.warp (a program the warp front end cannot read yet still reaches it)
const EMITTER_ONLY: &str = "emit";
/// WARP_SELF_HOSTED=pure (card pure-runs): parser.warp, the warp passes, analyzer.warp and emitter.warp compile the
/// source, Rust only carrying each stage's JSON to the next; a program the lane refuses compiles the usual way
const PURE: &str = "pure";
/// WARP_SELF_HOSTED=pure-only: the pure lane without the usual compile behind it, a refusal is the program's error (a
/// count of what the warp compiler compiles alone)
const PURE_ONLY: &str = "pure-only";
/// The passes.warp request of the pure lane's front end: the view after parser.warp and every warp pass
const PURE_STAGE: &str = "\"pure\"";
/// WARP_SELF_ANALYSIS set beside the switch: analyzer.warp `kinds` analyzes each lowered program too and emitter.warp
/// reads its kinds (a suite count of + analyzer)
const ANALYSIS_SWITCH: &str = "WARP_SELF_ANALYSIS";
/// passes.warp `stage` of the request in the worker's input file, with the prelude's text in its setup file
const PASSES_PROGRAM: &str = "use \"{lib}/compiler/passes.warp\"\nuse os\nstage(json(read(args#1)), read(args#2))";
/// analyzer.warp `emitter_kinds` of the view in the worker's input file, as JSON (shape: notes/self_hosting.md): a
/// program of its own, as analyzer.warp's names clash with parser.warp's (its local `digits`, its field `.separator`)
const ANALYSIS_PROGRAM: &str =
	"use \"{lib}/compiler/analyzer.warp\"\nuse os\nuse json\nto_json(emitter_kinds(node_of(json(read(args#1)))))";
/// emitter.warp `typed_wasm_module` of the request in the worker's input file, [the lowered program's view, its kinds
/// from analyzer.warp or {}], the runtime words' view and the host words in its setup files: the module's bytes, or a refusal starting with OUTSIDE_THE_CORE
const EMIT_PROGRAM: &str = concat!(
	"use \"{lib}/compiler/emitter.warp\"\nuse os\n",
	"request = json(read(args#1))\n",
	"typed_wasm_module(node_of(request#1), node_of(json(read(args#2))), json(read(args#3)), request#2)"
);
/// Where a stage program names lib/, the folder library() resolves
const LIBRARY_MARK: &str = "{lib}";
/// The runtime words emitter.warp compiles into a module, lowered like a program, in the compiler's folder
const RUNTIME_WORDS: &str = "runtime.warp";
/// The module the host words are imported from
const HOST_MODULE: &str = "host";
/// The levels of lib/compiler/diagnostics.warp's message files beside a warning: errors.data, hints.data
const ERROR_LEVEL: &str = "error";
const HINT_LEVEL: &str = "hint";
/// How emitter.warp refuses a program beyond its core; the Rust emitter emits it then
const OUTSIDE_THE_CORE: &str = "outside the core: ";
/// The file each refusal of emitter.warp goes to as a line, when set
const REFUSALS_FILE: &str = "WARP_EMIT_REFUSALS";
/// The REFUSALS_FILE line of a program the pure lane compiled: the denominator of a run's refusals
const RAN_PURE: &str = "pure: ran";
/// Set: emitter.warp refuses with every refused function's first form, a line each (its survey_setting)
#[cfg(feature = "native")]
const SURVEY_SETTING: &str = "WARP_EMIT_SURVEY";
/// The WARP_ settings that change no stage program's compile, so not its kept module either: where lib/ lies is keyed
/// by its contents, hints and windows only change what a run shows, a profile only watches it
#[cfg(feature = "native")]
const SETTINGS_BESIDE_STAGES: [&str; 8] = [
	SWITCH,
	ANALYSIS_SWITCH,
	REFUSALS_FILE,
	SURVEY_SETTING,
	crate::modules::LIBRARY_SETTING,
	crate::errors::diagnostic::HINTS_VARIABLE,
	crate::graphics::paint::NO_WINDOW_VARIABLE,
	warp_runtime::profile::PROFILE_VARIABLE,
];
/// The stage request of the parse: [0, source]; a pass's is [its index in passes.warp `passes`, the view]
const PARSE_STAGE: usize = 0;
/// The extension of a stage's setup JSON in the module cache
#[cfg(feature = "native")]
const SETUP_EXTENSION: &str = "json";
/// The input files of the stage programs, one per worker
const INPUT_FOLDER: &str = "warp-self-hosted";
/// Stack of a worker's own frames: the stage program's compile recurses as deeply as a test thread's; the wasm stack
/// the engines get comes on top (worker_stack)
const WORKER_STACK: usize = 64 << 20;

/// Computing time of one stage request: a compiler stage lowering a large program (parser.warp parses and lowers
/// itself and the files it uses in about 70 s of CPU) is no runaway, so it gets more than a program's 60 s;
/// WARP_TIME_LIMIT still decides when set
const STAGE_TIME_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);

type Pass = fn(Node) -> Node;
/// Each Rust pass with a warp twin and the twin's index in passes.warp `passes` (counted from 1, as warp's `#`)
const TWINNED_PASSES: [(Pass, usize); 8] = [
	(crate::text::interpolation::lower_program, 1),
	(crate::lowering::welcome_forms::lower_match_arms, 2),
	(crate::lowering::welcome_forms::lower_arrow_result_types, 3),
	(crate::lowering::of_lookups::lower, 4),
	(crate::lowering::grouped_receivers::lower, 5),
	(crate::lowering::times_phrases::lower, 6),
	(crate::lowering::broadcasting::lower_element_wise_operators, 7),
	(crate::lowering::number_words::lower, 8),
];

thread_local! {
	/// Set in a worker: the stage program itself compiles with the Rust parser and passes
	static IN_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// One request to a worker: its JSON, and where the result's view goes as JSON
type Request = (String, Sender<String>);

/// The JSON text a stage program reads as one of its setup files, or why it could not be made
type Setup = fn() -> Result<String, Node>;

/// A stage program, the JSON texts a worker writes once for it to read as `args#2`, `args#3`…, and its idle workers
struct Stage {
	program: &'static str,
	setup: &'static [Setup],
	workers: Mutex<Vec<Sender<Request>>>,
}

static PASSES: Stage = Stage { program: PASSES_PROGRAM, setup: &[prelude_text], workers: Mutex::new(Vec::new()) };
static ANALYSIS: Stage = Stage { program: ANALYSIS_PROGRAM, setup: &[], workers: Mutex::new(Vec::new()) };
static EMIT: Stage =
	Stage { program: EMIT_PROGRAM, setup: &[runtime_words, host_words], workers: Mutex::new(Vec::new()) };

impl Stage {
	/// The stage program, naming the files of library()
	fn text(&self) -> String {
		self.program.replace(LIBRARY_MARK, &library().display().to_string())
	}
}

/// The folder of the warp files a stage program reads: the compiler's and the standard modules it uses (also those
/// runtime.warp names with `use`), resolved once per process (modules::library_folder)
fn library() -> &'static std::path::Path {
	static FOLDER: OnceLock<PathBuf> = OnceLock::new();
	FOLDER.get_or_init(crate::modules::library_folder)
}

/// The folder of the compiler's warp files, where runtime.warp's parts are named from
fn compiler_folder() -> PathBuf {
	library().join(crate::modules::COMPILER_FOLDER)
}

/// The switch's value: None when off
fn switch() -> Option<&'static str> {
	static VALUE: OnceLock<Option<String>> = OnceLock::new();
	let value = VALUE.get_or_init(|| crate::setting(SWITCH)).as_deref();
	value.filter(|_| !IN_WORKER.with(|inside| inside.get()))
}

fn enabled() -> bool {
	switch().is_some()
}

/// emitter.warp emits too, unless the switch says FRONT_END_ONLY
fn emits() -> bool {
	switch().is_some_and(|value| value != FRONT_END_ONLY)
}

/// parser.warp and the pass twins run, unless the switch says EMITTER_ONLY
fn reads() -> bool {
	switch().is_some_and(|value| value != EMITTER_ONLY)
}

/// The program parser.warp parses from the source; None when not self-hosted
pub fn parse(code: &str) -> Option<Node> {
	reads().then(|| crate::warp_parser::sugar::equalities_read(stage(PARSE_STAGE, &json_text(code), &[]), code))
}

/// The node the warp twin of `pass` makes of `node`; None when not self-hosted or the pass has no twin
pub fn pass(pass: Pass, node: &Node) -> Option<Node> {
	if !reads() {
		return None;
	}
	let (_, index) = TWINNED_PASSES.iter().find(|(twinned, _)| std::ptr::fn_addr_eq(*twinned, pass))?;
	let (view, handles) = handled_plain_view(node);
	Some(stage(*index, &view_json(&view), &handles))
}

/// The kinds analyzer.warp finds in the program `view` shows, as emitter.warp reads them (`emitter_kinds`); None when
/// not self-hosted, the failure when warp fails
pub fn analysis(view: &Node) -> Option<Result<serde_json::Value, String>> {
	enabled().then(|| kinds_of(view))
}

/// With ANALYSIS_SWITCH on, the kinds analyzer.warp finds in the lowered program, which emitter.warp then reads (warp's
/// kinds alone, never compared with src/analyzer's: decision ignore-rust); an error when the stage fails; None when off
pub fn program_kinds(program: &Node) -> Result<Option<serde_json::Value>, Node> {
	static ANALYZES: OnceLock<bool> = OnceLock::new();
	if !enabled() || !*ANALYZES.get_or_init(|| crate::setting(ANALYSIS_SWITCH).is_some()) {
		return Ok(None);
	}
	kinds_of(&analysis_view(program)).map(Some).map_err(|failure| crate::node::error(&failure))
}

/// The kinds analyzer.warp finds in the program `view` shows, self-hosted or not
pub fn kinds_of(view: &Node) -> Result<serde_json::Value, String> {
	let kinds = answer(&ANALYSIS, view_json(view))?;
	let failed = || format!("the self-hosted analysis failed: {kinds}");
	let kinds: String = serde_json::from_str(&kinds).map_err(|_| failed())?;
	serde_json::from_str(&kinds).map_err(|_| failed())
}

/// The module emitter.warp emits for the lowered `node`; None when it does not emit (not self-hosted, or the front end
/// only) or the program is outside its core,
/// an error when emitter.warp fails
pub fn emit(node: &Node, kinds: Option<&serde_json::Value>) -> Option<Result<crate::pipeline::CompiledModule, Node>> {
	if !emits() {
		return None;
	}
	if wraps_ints() {
		record_refusal(&format!("{OUTSIDE_THE_CORE}fast arithmetic"));
		return None;
	}
	let kinds = kinds.map_or_else(|| "{}".to_string(), serde_json::Value::to_string);
	let request = format!("[{}, {kinds}]", view_json(&analysis_view(node)));
	let module = answer(&EMIT, request).and_then(|answer| module_of(&answer));
	match module {
		Err(refusal) if refusal.starts_with(OUTSIDE_THE_CORE) => {
			log::trace!("self-hosted emit: {refusal}");
			record_refusal(&refusal);
			None
		}
		Err(failure) => Some(Err(crate::node::error(&format!("the self-hosted emitter failed: {failure}")))),
		Ok(mut bytes) => {
			// the Rust front took the program's `use fuel`, so emitter.warp never saw it
			if let Some(mark) = crate::host::fuel_mark() {
				wasm_encoder::Section::append_to(&mark, &mut bytes);
			}
			let needs_host = imports_from(&bytes, HOST_MODULE);
			Some(Ok(crate::pipeline::CompiledModule { bytes, needs_host, needs_wasi: true, needs_ffi: false }))
		}
	}
}

/// Whether the program has fast functions (card fast-scope), whose Ints wrap as i64: emitter.warp's never do
fn wraps_ints() -> bool {
	crate::compiler::compile_mode::is_fast_program()
		|| crate::compiler::compile_context::read(|context| !context.fast_functions.is_empty())
}

/// The module the pure lane compiles the source to; None when the lane is off or refuses the program, the refusal then
/// recorded (REFUSALS_FILE) and logged, and the program compiled the usual way; under PURE_ONLY the refusal is the error
pub fn pure_module(code: &str) -> Option<Result<crate::pipeline::CompiledModule, Node>> {
	let strict = switch() == Some(PURE_ONLY);
	if !strict && switch() != Some(PURE) {
		return None;
	}
	match pure_bytes(code) {
		Ok(Err(diagnosed)) => Some(Err(diagnosed)),
		Ok(Ok(bytes)) => {
			record_lines(&format!("{RAN_PURE}\n"));
			crate::wasm_emitter::write_debug_module(&bytes);
			let needs_host = imports_from(&bytes, HOST_MODULE);
			Some(Ok(crate::pipeline::CompiledModule { bytes, needs_host, needs_wasi: true, needs_ffi: false }))
		}
		Err(refusal) => {
			log::info!("pure lane refused: {refusal}");
			let refused = refusal.starts_with(OUTSIDE_THE_CORE);
			let refusal = format!("{OUTSIDE_THE_CORE}pure: {}", refusal.trim_start_matches(OUTSIDE_THE_CORE));
			record_refusal(&refusal);
			let failure = if refused { refusal } else { format!("the pure lane failed: {refusal}") };
			strict.then(|| Err(crate::node::error(&failure)))
		}
	}
}

/// The source's module through the warp stages: the view the passes leave, its kinds, the module emitter.warp gives;
/// the program's compile error instead when the checks of the passes stage diagnose one. Their warnings are said once
/// the module is made, so a refusal leaves them to the Rust lane, and fail it under `use strict`
fn pure_bytes(code: &str) -> Result<Result<Vec<u8>, Node>, String> {
	let checked = answer(&PASSES, format!("[{PURE_STAGE}, {}, {}]", json_text(code), json_text(&program_folder())))?;
	let failed = || format!("the passes stage failed: {checked}");
	let parts: serde_json::Value = serde_json::from_str(&checked).map_err(|_| failed())?;
	let [view, diagnostics] = parts.as_array().map(Vec::as_slice).ok_or_else(failed)? else { return Err(failed()) };
	let warnings = match diagnosed(diagnostics) {
		Ok(warnings) => warnings,
		Err(error) => return Ok(Err(error)),
	};
	let view = view.to_string();
	let analysis = answer(&ANALYSIS, view.clone())?;
	let kinds: String =
		serde_json::from_str(&analysis).map_err(|_| format!("the analysis stage failed: {analysis}"))?;
	let bytes = module_of(&answer(&EMIT, format!("[{view}, {kinds}]"))?)?;
	let said = crate::errors::diagnostic::in_source_mode(code, || crate::errors::diagnostic::report(&warnings));
	Ok(said.map(|()| bytes))
}

/// What the checks of the passes stage diagnosed, [level, message, fix, topic@expression] each
/// (lib/compiler/diagnostics.warp): an error is the program's value, the warnings and hints are said as the Rust
/// checks' are
fn diagnosed(diagnostics: &serde_json::Value) -> Result<Vec<crate::errors::diagnostic::Diagnostic>, Node> {
	use crate::errors::diagnostic::{Diagnostic, Level};
	let mut said = Vec::new();
	for diagnostic in diagnostics.as_array().into_iter().flatten() {
		let text_at = |index: usize| diagnostic.get(index).and_then(serde_json::Value::as_str).unwrap_or_default();
		let fix = Some(text_at(2)).filter(|fix| !fix.is_empty()).map(String::from);
		let found = Diagnostic { message: text_at(1).to_string(), fix, ..Default::default() };
		// what "got it" silences: `topic@expression`
		let found = match text_at(3).split_once('@') {
			Some((topic, written)) => found.about(topic, written),
			None => found,
		};
		match text_at(0) {
			ERROR_LEVEL => return Err(found.into_error()),
			HINT_LEVEL => said.push(found.level(Level::Hint)),
			_ => said.push(found),
		}
	}
	Ok(said)
}

/// The folder the program's `use`s name files of, as an absolute path: its file's, else the working directory
fn program_folder() -> String {
	let file = crate::modules::program_file();
	let folder = file.as_deref().map(crate::modules::folder_of).unwrap_or_else(|| PathBuf::from("."));
	folder.canonicalize().unwrap_or(folder).display().to_string()
}

/// The refusal's lines (one, or one per refused function under SURVEY_SETTING) in the file REFUSALS_FILE names, when
/// set: a run over many programs ranks what keeps them outside the core (`sort | uniq -c | sort -rn`)
fn record_refusal(refusal: &str) {
	record_lines(
		&refusal
			.lines()
			.filter(|line| line.starts_with(OUTSIDE_THE_CORE))
			.map(|line| format!("{line}\n"))
			.collect::<String>(),
	);
}

/// Appends the lines to the file REFUSALS_FILE names, when set
fn record_lines(lines: &str) {
	use std::io::Write;
	let Some(file) = crate::setting(REFUSALS_FILE) else { return };
	let appended = std::fs::OpenOptions::new().create(true).append(true).open(&file);
	let _ = appended.and_then(|mut file| file.write_all(lines.as_bytes()));
}

/// Does the module import a function of `module`
fn imports_from(bytes: &[u8], module: &str) -> bool {
	wasmparser::Parser::new(0).parse_all(bytes).any(|payload| match payload {
		Ok(wasmparser::Payload::ImportSection(imports)) => {
			imports.into_imports().flatten().any(|import| import.module == module)
		}
		_ => false,
	})
}

/// The bytes of the module emitter.warp answers as a list of byte numbers, else its answer as the failure
fn module_of(answer: &str) -> Result<Vec<u8>, String> {
	match serde_json::from_str(answer) {
		Ok(serde_json::Value::Array(items)) => {
			let bytes: Option<Vec<u8>> =
				items.iter().map(|item| item.as_u64().and_then(|byte| u8::try_from(byte).ok())).collect();
			bytes.ok_or_else(|| format!("a module byte beyond 0..255: {answer}"))
		}
		Ok(serde_json::Value::String(refusal)) => Err(refusal),
		_ => Err(answer.to_string()),
	}
}

/// The words a module imports from the host (src/host.rs), as JSON: {name: [[parameter kind…], [result kind…]]}, a kind
/// as emitter.warp names it (an anyref is a "node")
fn host_words() -> Result<String, Node> {
	use wasm_encoder::ValType;
	let kind = |value_type: &ValType| match value_type {
		ValType::I64 => "int",
		ValType::F64 => "float",
		ValType::I32 => "i32",
		_ => "node",
	};
	let kinds = |types: &[ValType]| types.iter().map(kind).collect::<Vec<_>>();
	let words = crate::host::HOST_WORD_SIGNATURES
		.iter()
		.map(|(name, parameters, results)| (name.to_string(), serde_json::json!([kinds(parameters), kinds(results)])));
	Ok(serde_json::Value::Object(words.collect()).to_string())
}

/// The prelude's source, whose definitions a pure program reaches (lib/compiler/prelude_words.warp)
fn prelude_text() -> Result<String, Node> {
	Ok(crate::modules::std_module(crate::modules::PRELUDE_MODULE).unwrap_or_default().to_string())
}

/// The view of lib/compiler/runtime.warp lowered, as JSON, its declared types shown as a program's are (`f(): int`
/// keeps its result's kind); a warp older than lib/ may fail to lower it (card crash-3)
fn runtime_words() -> Result<String, Node> {
	runtime_source()
		.map_err(|failure| crate::node::error(&failure.to_string()))
		.and_then(|source| crate::pipeline::lower(&source))
		.map(|lowered| view_json(&analysis_view(&lowered)))
		.map_err(|failure| {
			crate::node::error(&format!("lib/compiler/runtime.warp does not lower: {}", failure.serialize()))
		})
}

/// runtime.warp with the files its `use` lines name in their place, after it: a module of lib (`use case_tables`) or
/// one of its parts (`use "runtime/text.warp"`, relative to lib/compiler). Their words become runtime words, compiled
/// into a module only when called
pub fn runtime_source() -> std::io::Result<String> {
	let source = std::fs::read_to_string(compiler_folder().join(RUNTIME_WORDS))?;
	let mut joined = String::new();
	let mut used = String::new();
	for line in source.lines() {
		match line.strip_prefix("use ").map(str::trim) {
			Some(name) => used += &std::fs::read_to_string(used_file(name))?,
			None => joined += line,
		}
		joined.push('\n');
	}
	Ok(joined + &used)
}

/// The file of a `use` line of runtime.warp: a quoted path beside runtime.warp, else a module of lib
fn used_file(name: &str) -> PathBuf {
	match name.strip_prefix('"').and_then(|quoted| quoted.strip_suffix('"')) {
		Some(path) => compiler_folder().join(path),
		None => library().join(format!("{name}.warp")),
	}
}

/// The node of the view the stage gives, its data handles the nodes in `handles`; a warp failure is an error node, so the program fails as it should
fn stage(index: usize, argument: &str, handles: &[Node]) -> Node {
	let answer = match answer(&PASSES, format!("[{index}, {argument}]")) {
		Ok(answer) => answer,
		Err(failure) => return crate::node::error(&failure),
	};
	match serde_json::from_str(&answer) {
		Ok(view) => node_of_handled_view(&view_of_json(view), handles),
		Err(_) => crate::node::error(&format!("self-hosted stage {index} failed: {answer}")),
	}
}

/// The JSON an idle worker of `stage` answers the request with, or the failure text when warp fails
fn answer(stage: &'static Stage, request: String) -> Result<String, String> {
	let worker = stage.workers.lock().unwrap().pop().unwrap_or_else(|| spawn_worker(stage));
	let (reply_to, reply) = channel();
	let answer = worker.send((request, reply_to)).ok().and_then(|_| reply.recv().ok());
	let answer = answer.ok_or("the self-hosted stage's worker died")?;
	stage.workers.lock().unwrap().push(worker);
	Ok(answer)
}

/// A worker's stack: its own frames and the whole wasm stack its engines may use (warp's main thread gives wasm
/// 496 MB, warp_runtime::engine::give_main_stack_to_wasm): a deep stage run then traps in wasm instead of overflowing
/// the thread (parser.warp's compile aborted the process); only the pages used are committed
#[cfg(feature = "native")]
fn worker_stack() -> usize {
	WORKER_STACK + warp_runtime::engine::wasm_stack_bytes()
}
#[cfg(not(feature = "native"))]
fn worker_stack() -> usize {
	WORKER_STACK
}

fn spawn_worker(stage: &'static Stage) -> Sender<Request> {
	static WORKERS_SPAWNED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
	let number = WORKERS_SPAWNED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let folder = std::env::temp_dir().join(INPUT_FOLDER);
	let input = folder.join(format!("{}-{number}.txt", std::process::id()));
	let (requests, incoming) = channel::<Request>();
	let work = move || {
		IN_WORKER.with(|inside| inside.set(true));
		let _ = std::fs::create_dir_all(&folder);
		let stages = StageProgram::new(stage, input);
		let limit = warp_runtime::fuel::time_limit_from_environment().unwrap_or(STAGE_TIME_LIMIT);
		for (request, reply_to) in incoming {
			let _ = reply_to.send(crate::util::with_time_limit(limit, || stages.run(&request)));
		}
	};
	std::thread::Builder::new().stack_size(worker_stack()).spawn(work).expect("a self-hosted stage worker");
	requests
}

/// A stage program answering the JSON its input file holds, compiled once per worker, and kept in the module cache across
/// runs while warp, lib/ and the WARP_ settings stay as they are (a compile of it takes seconds)
struct StageProgram {
	module: Result<crate::pipeline::CompiledModule, Node>,
	input: PathBuf,
	setup: Vec<PathBuf>,
}

impl StageProgram {
	fn new(stage: &Stage, input: PathBuf) -> Self {
		let mut failed_setup = None;
		let setup_file = |(index, setup): (usize, &Setup)| {
			let file = input.with_extension(format!("setup-{index}.json"));
			match kept_setup(&format!("{}\nsetup {index}", stage.text()), *setup) {
				Ok(json) => drop(std::fs::write(&file, json)),
				Err(failure) => failed_setup = Some(failure),
			}
			file
		};
		let setup: Vec<PathBuf> = stage.setup.iter().enumerate().map(setup_file).collect();
		// the files are the worker's arguments, so the program, and its kept module, is the same for every worker
		#[cfg(feature = "native")]
		crate::modules::std_adapters::set_program_arguments(
			std::iter::once(&input).chain(&setup).map(|file| file.display().to_string()).collect(),
		);
		use crate::compiler::compile_context::scoped;
		// the stage reads warp's lib/ alone, whatever folder warp runs in (a program's text.warp is no `text` of it)
		let module = match failed_setup {
			Some(failure) => Err(failure),
			None => scoped(|context| &mut context.beyond_working_directory, true, || compiled_stages(&stage.text())),
		};
		StageProgram { module, input, setup }
	}

	/// The view the stage gives as JSON, or the failure as a text that is no JSON
	fn run(&self, request: &str) -> String {
		let value = match (&self.module, std::fs::write(&self.input, request)) {
			(Ok(module), Ok(())) => crate::pipeline::run_module(module.clone()),
			(Err(failure), _) => failure.clone(),
			(_, Err(failure)) => crate::node::error(&failure.to_string()),
		};
		match value.drop_meta() {
			Node::Error(_) => value.serialize(),
			view => view_json(view),
		}
	}
}

impl Drop for StageProgram {
	fn drop(&mut self) {
		std::iter::once(&self.input).chain(&self.setup).for_each(|file| {
			let _ = std::fs::remove_file(file);
		});
	}
}

/// The setup JSON `setup` makes, or the one kept under `name` (lowering the runtime words takes seconds)
#[cfg(feature = "native")]
fn kept_setup(name: &str, setup: Setup) -> Result<String, Node> {
	use crate::run::module_cache::{keep_bytes, kept_bytes};
	let key = stage_key(name);
	let kept = key.as_deref().and_then(|key| kept_bytes(key, SETUP_EXTENSION));
	if let Some(json) = kept.and_then(|bytes| String::from_utf8(bytes).ok()) {
		return Ok(json);
	}
	let json = setup()?;
	if let Some(key) = key {
		keep_bytes(&key, SETUP_EXTENSION, json.as_bytes());
	}
	Ok(json)
}

/// The setup JSON of a stage program made; the browser build keeps nothing
#[cfg(not(feature = "native"))]
fn kept_setup(_name: &str, setup: Setup) -> Result<String, Node> {
	setup()
}

/// The stage program compiled; the browser build keeps no modules
#[cfg(not(feature = "native"))]
fn compiled_stages(program: &str) -> Result<crate::pipeline::CompiledModule, Node> {
	crate::pipeline::compile(program)
}

/// The stage program compiled, or the module kept for it
#[cfg(feature = "native")]
fn compiled_stages(program: &str) -> Result<crate::pipeline::CompiledModule, Node> {
	use crate::run::program_cache::{keep_under, kept_under};
	let key = stage_key(program);
	if let Some(module) = key.as_deref().and_then(kept_under) {
		return Ok(module);
	}
	let module = crate::pipeline::compile(program)?;
	if let Some(key) = key {
		keep_under(&key, &module);
	}
	Ok(module)
}

/// With the switch on, the key of the warp compiler that compiles programs (lib/ and the binary): a kept program of an
/// older lib/compiler is no program of this one
#[cfg(feature = "native")]
pub fn compiler_key() -> Option<&'static str> {
	static KEY: OnceLock<Option<String>> = OnceLock::new();
	enabled().then(|| KEY.get_or_init(|| stage_key("the self-hosted compiler")).as_deref()).flatten()
}

/// The cache key of a stage program: the running binary, the program, the WARP_ settings and every file of lib/ with its
/// path inside lib/, so an identical lib/ elsewhere (another worktree) reuses the kept modules; none when the binary has
/// no path
#[cfg(feature = "native")]
fn stage_key(program: &str) -> Option<String> {
	use sha2::{Digest, Sha256};
	// canonical: macOS gives the path the binary was started by, `../../scratch/warp` keyed every folder apart
	let binary = std::env::current_exe().ok()?.canonicalize().ok()?;
	let mut hasher = Sha256::new();
	let library = library();
	let program = program.replace(&library.display().to_string(), LIBRARY_MARK);
	let mut settings = crate::run::program_cache::warp_settings();
	settings.retain(|(name, _)| !SETTINGS_BESIDE_STAGES.contains(&name.as_str()));
	hasher.update(format!(
		"{}\n{:?}\n{settings:?}\n{program}\n",
		binary.display(),
		crate::run::program_cache::stamp(&binary)
	));
	let mut files: Vec<PathBuf> = files_below(library.to_path_buf());
	files.sort();
	for file in files {
		hasher.update(format!("{}\n", file.strip_prefix(library).unwrap_or(&file).display()));
		hasher.update(std::fs::read(&file).unwrap_or_default());
	}
	Some(crate::extensions::hex_text(&hasher.finalize()))
}

/// The files below `folder`, at any depth
#[cfg(feature = "native")]
fn files_below(folder: PathBuf) -> Vec<PathBuf> {
	let entries = std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|entry| entry.path());
	entries.flat_map(|path| if path.is_dir() { files_below(path) } else { vec![path] }).collect()
}

/// The view a JSON value writes: an array a list, null ø
fn view_of_json(json: serde_json::Value) -> Node {
	use serde_json::Value;
	match json {
		Value::Null => Node::Empty,
		Value::Bool(true) => Node::True,
		Value::Bool(false) => Node::False,
		Value::Number(number) => {
			number.as_i64().map_or_else(|| Node::float(number.as_f64().unwrap_or(f64::NAN)), Node::int)
		}
		Value::String(text) => Node::Text(text),
		Value::Array(items) => crate::compiler::node_views::warp_list(items.into_iter().map(view_of_json)),
		Value::Object(object) => crate::node::error(&format!("a view is no object: {}", Value::Object(object))),
	}
}
