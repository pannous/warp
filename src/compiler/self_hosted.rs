//! The compile through the warp-written compiler (card warp-pipeline-suite, notes/self_hosting.md): with
//! WARP_SELF_HOSTED set, lib/compiler/parser.warp parses each program and every Rust pass with a warp twin in
//! lib/compiler/passes.warp gives way to it; the passes without a twin and the emitter stay Rust. The suite run this
//! way (tests/self_hosted.sh) counts how much of warp the warp compiler already compiles.
//! A stage is passes.warp `stage(request)`, compiled once per worker thread: the compile of the stage program has
//! thread-local state of its own (diagnostics, compile context), which a compile it runs in the middle of must keep.
use crate::compiler::node_views::{handled_plain_view, json_text, node_of_handled_view, view_json};
use crate::node::Node;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

const SWITCH: &str = "WARP_SELF_HOSTED";
const STAGES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lib/compiler/passes.warp");
/// The stage request of the parse: [0, source]; a pass's is [its index in passes.warp `passes`, the view]
const PARSE_STAGE: usize = 0;
/// The input files of the stage programs, one per worker
const INPUT_FOLDER: &str = "warp-self-hosted";
/// Stack of a worker: the stage program's compile recurses as deeply as a test thread's
const WORKER_STACK: usize = 64 << 20;

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

/// The idle workers
static WORKERS: Mutex<Vec<Sender<Request>>> = Mutex::new(Vec::new());

fn enabled() -> bool {
	static SWITCHED_ON: OnceLock<bool> = OnceLock::new();
	*SWITCHED_ON.get_or_init(|| std::env::var_os(SWITCH).is_some()) && !IN_WORKER.with(|inside| inside.get())
}

/// The program parser.warp parses from the source; None when not self-hosted
pub fn parse(code: &str) -> Option<Node> {
	enabled().then(|| stage(PARSE_STAGE, &json_text(code), &[]))
}

/// The node the warp twin of `pass` makes of `node`; None when not self-hosted or the pass has no twin
pub fn pass(pass: Pass, node: &Node) -> Option<Node> {
	if !enabled() {
		return None;
	}
	let (_, index) = TWINNED_PASSES.iter().find(|(twinned, _)| std::ptr::fn_addr_eq(*twinned, pass))?;
	let (view, handles) = handled_plain_view(node);
	Some(stage(*index, &view_json(&view), &handles))
}

/// The node of the view the stage gives, its data handles the nodes in `handles`; a warp failure is an error node, so the program fails as it should
fn stage(index: usize, argument: &str, handles: &[Node]) -> Node {
	let request = format!("[{index}, {argument}]");
	let worker = WORKERS.lock().unwrap().pop().unwrap_or_else(spawn_worker);
	let (reply_to, reply) = channel();
	let answer = worker.send((request, reply_to)).ok().and_then(|_| reply.recv().ok());
	let Some(answer) = answer else {
		return crate::node::error("the self-hosted stage's worker died");
	};
	WORKERS.lock().unwrap().push(worker);
	match serde_json::from_str(&answer) {
		Ok(view) => node_of_handled_view(&view_of_json(view), handles),
		Err(_) => crate::node::error(&format!("self-hosted stage {index} failed: {answer}")),
	}
}

fn spawn_worker() -> Sender<Request> {
	static WORKERS_SPAWNED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
	let number = WORKERS_SPAWNED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let folder = std::env::temp_dir().join(INPUT_FOLDER);
	let input = folder.join(format!("{}-{number}.txt", std::process::id()));
	let (requests, incoming) = channel::<Request>();
	let work = move || {
		IN_WORKER.with(|inside| inside.set(true));
		let _ = std::fs::create_dir_all(&folder);
		let stages = StageProgram::new(input);
		for (request, reply_to) in incoming {
			let _ = reply_to.send(stages.run(&request));
		}
	};
	std::thread::Builder::new().stack_size(WORKER_STACK).spawn(work).expect("a self-hosted stage worker");
	requests
}

/// passes.warp `stage` of the JSON its input file holds, compiled once
struct StageProgram {
	module: Result<crate::pipeline::CompiledModule, Node>,
	input: PathBuf,
}

impl StageProgram {
	fn new(input: PathBuf) -> Self {
		let program = format!("use \"{STAGES}\"\nstage(json(read(\"{}\")))", input.display());
		StageProgram { module: crate::pipeline::compile(&program), input }
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
		let _ = std::fs::remove_file(&self.input);
	}
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
