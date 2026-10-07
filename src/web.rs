//! The compiler in the browser (web/playground): `evaluate` compiles a program the way the CLI does and reports
//! what the CLI prints (the value, warnings, hints) plus the topics of the warnings and notes the page offers "got it" for. Without wasmtime the
//! page runs the compiled module (`run_in_host`) and hands its result back as a JSON tree that its reader
//! (web/playground/reader.js) walked through the module's reflection exports (wasm_emitter/reflection.rs).

use crate::diagnostic::{self, Acknowledger};
use crate::extensions::numbers::Number;
use crate::fixits::{self, Fix};
use crate::meta::{DataValue, DataType};
use crate::node::{Bracket, Node, Separator};
use crate::type_kinds::{Kind, KIND_MASK};
use num_bigint::{BigInt, Sign};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use crate::type_kinds::KIND_BITS;

thread_local! {
	/// The HTML the last run rendered inside its own module (host.js outcomeOf by page·render): html_of shows it instead
	/// of compiling the renderer as a program of its own (card playground-render)
	static RENDERED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The topics and expressions (`topic@expression`) the page already said "got it" to; every other warning or note shown
/// is recorded for its "got it" buttons: the topic and the key of its expression
struct PageAcknowledger {
	acknowledged: HashSet<String>,
	notes: Rc<RefCell<Vec<(String, String)>>>,
}

impl Acknowledger for PageAcknowledger {
	fn has_acknowledged(&self, key: &str) -> bool {
		self.acknowledged.contains(key)
	}

	/// The page answers later, with its "got it" buttons: this expression, or all of the kind
	fn acknowledge(&self, topic: &str, written: &str) -> crate::diagnostic::GotIt {
		self.notes.borrow_mut().push((topic.to_string(), diagnostic::expression_key(topic, written)));
		crate::diagnostic::GotIt::No
	}
}

/// Compile and run `code` as `warp file.wasp` does, with the topics the page acknowledged. The report (JSON):
/// `value` (what the CLI prints), `error`, `errors` (the failed program's errors with fixes), `warnings`, `hints`,
/// `notes` (topics of the warnings and notes shown that the user can say "got it" to), `got_it` (each of them with the
/// `topic@expression` key that silences only its expression; a warning carries its own as `expression_key`) and
/// `runtime_warnings`. Warnings, errors and hints carry `fixes` ("I meant: …" buttons, fixes_json). The acknowledged set
/// holds topics and expression keys. Acknowledging silences a warning, never changes the value.
pub fn evaluate(code: &str, acknowledged: HashSet<String>) -> Value {
	let notes = Rc::new(RefCell::new(Vec::new()));
	let acknowledger = PageAcknowledger { acknowledged, notes: notes.clone() };
	diagnostic::take_warnings();
	diagnostic::take_error_diagnostics();
	diagnostic::take_runtime_warnings();
	let (result, hints) = diagnostic::with_acknowledger(acknowledger, || crate::normalize::capture_hints(|| run_shown(code)));
	let warnings: Vec<Value> = diagnostic::take_warnings().iter().map(|warning| diagnostic_json(code, warning)).collect();
	let is_error = matches!(result.drop_meta(), Node::Error(_));
	let errors: Vec<Value> = unique(diagnostic::take_error_diagnostics()).iter().filter(|_| is_error).map(|error| diagnostic_json(code, error)).collect();
	let hints: Vec<Value> = hints.iter().map(|hint| {
		let (line, column) = hint.line_and_column();
		json!({
			"original": hint.original, "canonical": hint.canonical, "position": hint.position, "reason": hint.reason,
			"fixes": fixes_json(code, line, column, &hint.fix().into_iter().collect::<Vec<_>>()),
		})
	}).collect();
	let mut report = json!({
		"value": result.serialize(),
		"error": is_error,
		"errors": errors,
		"error_at": diagnostic::error_position(&result).map(|(line, column)| json!({"line": line, "column": column})),
		"warnings": warnings,
		"runtime_warnings": diagnostic::take_runtime_warnings(),
		"hints": hints,
		"asks": [], // no Asks any more (every ambiguity is a warning or an error); kept until the page stops reading it
		"notes": notes.borrow().iter().map(|(topic, _)| topic).collect::<Vec<_>>(),
		"got_it": notes.borrow().iter().map(|(topic, expression)| json!({"topic": topic, "expression": expression})).collect::<Vec<_>>(),
	});
	report["html"] = html_of(&result); // after the program's diagnostics are taken: the renderer is a program too
	report
}

/// Markup the page shows as DOM (card web-dom), rendered by lib/markup.wasp: inside the program's module when it renders
/// itself, else by the renderer compiled on its own
fn html_of(value: &Node) -> Value {
	let rendered = RENDERED.with(|rendered| rendered.borrow_mut().take());
	json!(crate::markup::is_markup(value).then(|| rendered.unwrap_or_else(|| crate::markup::to_html(value))))
}

/// `code` compiled and run; a program holding markup renders itself (pipeline::rendering_itself)
fn run_shown(code: &str) -> Node {
	match renders_itself(code) {
		true => crate::pipeline::rendering_itself(|| crate::wasm_emitter::eval(code)),
		false => crate::wasm_emitter::eval(code),
	}
}

/// Does the program hold markup (`div{…}`): only then it carries the renderer, which a plain program does not need
pub fn renders_itself(code: &str) -> bool {
	let mut holds_markup = false;
	// quietly: the compile that follows says what the parse finds
	diagnostic::quietly(|| crate::wasp_parser::parse(code)).visit(&mut |part| holds_markup |= crate::markup::is_markup(part));
	holds_markup
}

/// A warning or error for the page: its words, position, "got it" topic and the fixes it offers
fn diagnostic_json(code: &str, diagnostic: &diagnostic::Diagnostic) -> Value {
	json!({
		"message": diagnostic.message, "line": diagnostic.line, "column": diagnostic.column, "fix": diagnostic.fix,
		"topic": diagnostic.topic, "expression_key": diagnostic.expression_key, "fixes": fixes_json(code, diagnostic.line, diagnostic.column, &diagnostic.fixes),
	})
}

/// Each fix as the page applies it: replace the UTF-16 range `start`..`end` of the editor's text with `replacement`,
/// then each of its further `edits` (the last in the text first). A fix whose text the source does not show has
/// `start` and `end` null: the page shows it, but cannot apply it
fn fixes_json(code: &str, line: usize, column: usize, fixes: &[Fix]) -> Vec<Value> {
	fixes.iter().filter(|fix| fix.changes_something()).map(|fix| {
		let edits = fixits::edits(code, line, column, fix);
		let range = edits.as_ref().and_then(|_| fixits::edit(code, line, column, fix)).map(|edit| edit.range);
		let offset = |at: fn(&std::ops::Range<usize>) -> usize| range.as_ref().map(|range| fixits::utf16_offset(code, at(range)));
		let edits: Vec<Value> = edits.unwrap_or_default().iter().map(|edit| json!({
			"start": fixits::utf16_offset(code, edit.range.start), "end": fixits::utf16_offset(code, edit.range.end), "replacement": edit.replacement,
		})).collect();
		json!({
			"label": fix.label(), "meaning": fix.meaning, "written": fix.written, "replacement": fix.replacement,
			"start": offset(|range| range.start), "end": offset(|range| range.end), "edits": edits,
		})
	}).collect()
}

/// Lowering may build the same error twice: each one once
fn unique(diagnostics: Vec<diagnostic::Diagnostic>) -> Vec<diagnostic::Diagnostic> {
	let mut seen = Vec::new();
	diagnostics.into_iter().filter(|diagnostic| {
		let new = !seen.contains(diagnostic);
		seen.push(diagnostic.clone());
		new
	}).collect()
}

/// The topics the page acknowledged: a JSON array of topics, or the older object whose `ack:<topic>` keys are the
/// acknowledged topics (its other keys were remembered Ask answers, which no longer exist)
pub fn acknowledged_topics(json: &str) -> HashSet<String> {
	match serde_json::from_str::<Value>(json).unwrap_or_default() {
		Value::Array(topics) => topics.iter().filter_map(|topic| topic.as_str().map(str::to_string)).collect(),
		Value::Object(answers) => answers.keys().filter_map(|key| key.strip_prefix("ack:").map(str::to_string)).collect(),
		_ => HashSet::new(),
	}
}

// ============================================================================
// The result of a run in the page, read back into a Node
// ============================================================================

/// What happened when the page ran a module (`warnings`: the runtime warnings it reported): `{"result": tree}`, `{"trap": message, "trace": stack, "detail": tree?}`
/// (a runtime error; the stack names the wasm functions, `detail` is the module's trap_detail global) or
/// `{"failure": message}` (the module did not compile, link or instantiate)
pub fn run_outcome(outcome: &Value) -> Node {
	RENDERED.with(|rendered| *rendered.borrow_mut() = outcome.get("html").and_then(Value::as_str).map(str::to_string));
	for warning in outcome.get("warnings").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
		diagnostic::report_runtime_warning(warning);
	}
	if let Some(tree) = outcome.get("result") {
		return node_from_tree(tree);
	}
	if let Some(message) = outcome.get("trap").and_then(Value::as_str) {
		let mut trace = outcome.get("trace").and_then(Value::as_str).unwrap_or_default().to_string();
		if let Some(detail) = outcome.get("detail").filter(|detail| !detail.is_null()) {
			trace.push_str(&format!("\n{}{}", crate::wasm_emitter::TRAP_DETAIL_PREFIX, node_from_tree(detail).serialize()));
		}
		return crate::wasm_emitter::trap_error(&trace, engine_words(message));
	}
	if let Some(message) = outcome.get("error").and_then(Value::as_str) {
		return crate::node::error(message); // a host word failed with its own message (run_block)
	}
	let failure = outcome.get("failure").and_then(Value::as_str).unwrap_or("the page sent no outcome");
	crate::node::error(&format!("could not run the program: {failure}"))
}

/// What the page shows after a page event's handler (worker.js showHandled): the value as the compiler writes it, and
/// its HTML when it is markup (card web-element)
pub fn shown(outcome: &Value) -> Value {
	let value = run_outcome(outcome);
	json!({ "value": value.serialize(), "html": html_of(&value) })
}

/// The page's run_block (host.js; src/host.rs natively): the block and what it sees as trees (`{block, names, values,
/// definitions}`), its value as a tree back (`{"result": tree}`) or `{"error": message}`
pub fn eval_block_report(request: &Value) -> Value {
	let tree = |key: &str| request.get(key).map(node_from_tree).unwrap_or(Node::Empty);
	match crate::pipeline::eval_block(tree("block"), &tree("names"), &tree("values"), &tree("definitions")) {
		Err(message) => json!({ "error": message }),
		Ok(result) => match tree_of(&result) {
			Some(tree) => json!({ "result": tree }),
			None => json!({ "error": crate::pipeline::cannot_hand_back(&result) }),
		},
	}
}

/// A value as the tree host.js buildValue builds in a module (the inverse of node_from_tree for the values a task or a
/// block hands back); None for what has no constructor there yet (an error, a type)
fn tree_of(node: &Node) -> Option<Value> {
	let kind = |kind: Kind| (kind as i64).to_string();
	Some(match node.drop_meta() {
		Node::Empty => json!({ "kind": kind(Kind::Empty), "data": null, "chain": [] }),
		Node::Number(Number::Int(n)) if crate::wasm_emitter::is_fixnum(*n) => json!({ "kind": kind(Kind::Int), "data": { "int": n.to_string() }, "chain": [] }),
		// beyond the fixnums: host.js composes it from fixnum pieces (src/tasks.rs EXACT_BUILDERS)
		Node::Number(Number::Int(n)) => json!({ "kind": kind(Kind::Int), "data": { "exact": [n.to_string(), "1"] }, "chain": [] }),
		Node::Number(Number::BigInt(n)) => json!({ "kind": kind(Kind::Int), "data": { "exact": [n.to_string(), "1"] }, "chain": [] }),
		Node::Number(Number::Quotient(n, d)) => json!({ "kind": kind(Kind::Int), "data": { "exact": [n.to_string(), d.to_string()] }, "chain": [] }),
		Node::Number(Number::BigQuotient(q)) => json!({ "kind": kind(Kind::Int), "data": { "exact": [q.numerator.to_string(), q.denominator.to_string()] }, "chain": [] }),
		Node::Number(Number::Float(x)) if x.is_finite() => json!({ "kind": kind(Kind::Float), "data": { "float": x }, "chain": [] }),
		Node::Text(text) => json!({ "kind": kind(Kind::Text), "data": { "text": text }, "chain": [] }),
		Node::Symbol(text) => json!({ "kind": kind(Kind::Symbol), "data": { "text": text }, "chain": [] }),
		Node::Char(c) => json!({ "kind": kind(Kind::Codepoint), "data": { "i31": *c as u32 }, "chain": [] }),
		Node::Key(left, op, right) => {
			let key_kind = (crate::operators::op_to_code(op) << KIND_BITS) | Kind::Key as i64;
			json!({ "kind": key_kind.to_string(), "key": [tree_of(left)?, tree_of(right)?] })
		}
		Node::List(items, bracket, _) => {
			let list_kind = (crate::wasm_emitter::bracket_info(bracket) << KIND_BITS) | Kind::List as i64;
			json!({ "kind": list_kind.to_string(), "items": items.iter().map(tree_of).collect::<Option<Vec<_>>>()? })
		}
		_ => return None,
	})
}

/// V8's trap messages and the words wasmtime uses for them, so a runtime error reads the same in the CLI and the page
const TRAP_WORDS: [(&str, &str); 12] = [
	("unreachable", "wasm `unreachable` instruction executed"),
	("memory access out of bounds", "out of bounds memory access"),
	("divide by zero", "integer divide by zero"),
	("remainder by zero", "integer divide by zero"),
	("divide result unrepresentable", "integer overflow"),
	("float unrepresentable in integer range", "invalid conversion to integer"),
	("null function or function signature mismatch", "indirect call type mismatch"),
	("dereferencing a null pointer", "null reference"),
	("illegal cast", "cast failure"),
	("array element access out of bounds", "out of bounds array access"),
	("table index is out of bounds", "undefined element: out of bounds table access"),
	("Maximum call stack size exceeded", "call stack exhausted"),
];
const TRAP_PREFIX: &str = "wasm trap: ";

/// The CLI's message for a trap the page reports in the browser engine's words
fn engine_words(message: &str) -> String {
	let words = TRAP_WORDS.iter().find(|(engine, _)| message == *engine).map_or(message, |(_, wasmtime)| wasmtime);
	format!("{TRAP_PREFIX}{words}")
}

/// A node read by the page: `{kind, data, chain}`, where `kind` is the decimal i64 kind field, `data` the payload
/// and `chain` the nodes reached by following `value` fields (a list's cons cells), flat so deep lists need no recursion
pub fn node_from_tree(tree: &Value) -> Node {
	let empty = Vec::new();
	let chain = tree.get("chain").and_then(Value::as_array).unwrap_or(&empty);
	let mut value = None;
	for cell in chain.iter().rev().chain(std::iter::once(tree)) {
		value = Some(cell_node(cell, value));
	}
	value.unwrap_or(Node::Empty)
}

/// One node from its kind and payload, `value` being the node its value field holds; mirrors Node::from_gc_object
fn cell_node(cell: &Value, value: Option<Node>) -> Node {
	let kind: i64 = match cell.get("kind") {
		Some(Value::String(text)) => text.parse().unwrap_or(0),
		Some(number) => number.as_i64().unwrap_or(0),
		None => return Node::Empty,
	};
	let data = cell.get("data").unwrap_or(&Value::Null);
	let info = (kind >> KIND_BITS) & KIND_MASK;
	let text = || data.get("text").and_then(Value::as_str).unwrap_or_default().to_string();
	let data_node = || data.get("node").map(node_from_tree).unwrap_or(Node::Empty);
	let value_node = || value.clone().unwrap_or(Node::Empty);
	match kind & KIND_MASK {
		tag if tag == Kind::Empty as i64 => Node::Empty,
		tag if tag == Kind::Int as i64 => payload_number(data).map_or_else(|| crate::node::error("unreadable Int"), Node::Number),
		tag if tag == Kind::Float as i64 => Node::Number(Number::Float(payload_float(data).unwrap_or(0.0))),
		tag if tag == Kind::Text as i64 => Node::Text(text()),
		tag if tag == Kind::Error as i64 => Node::Error(Box::new(Node::Text(text()))),
		tag if tag == Kind::Symbol as i64 => Node::Symbol(text()),
		tag if tag == Kind::Codepoint as i64 => {
			Node::Char(data.get("i31").and_then(Value::as_u64).and_then(|code| char::from_u32(code as u32)).unwrap_or('\0'))
		}
		tag if tag == Kind::Key as i64 => Node::Key(Box::new(data_node()), crate::operators::code_to_op(info), Box::new(value_node())),
		tag if tag == Kind::Block as i64 => list_node(data.get("node").map(node_from_tree), value, Bracket::Curly),
		tag if tag == Kind::List as i64 => list_node(data.get("node").map(node_from_tree), value, bracket_of(info)),
		tag if tag == Kind::Data as i64 => {
			let type_name = text();
			Node::Data(DataValue { data: Box::new(format!("<wasm data: {type_name}>")), type_name, data_type: DataType::Other })
		}
		tag if tag == Kind::Function as i64 => value_node(), // a closure reads as the name of its function
		tag if tag == Kind::TypeDef as i64 => Node::Type { name: Box::new(data_node()), body: Box::new(value_node()) },
		tag => Node::Text(format!("Unknown Kind: {tag}")),
	}
}

fn bracket_of(info: i64) -> Bracket {
	match info {
		0 => Bracket::Curly,
		1 => Bracket::Square,
		2 => Bracket::Round,
		3 => Bracket::Less,
		_ => Bracket::None,
	}
}

/// A cons cell: its first item, then the items of the rest; no first item is the empty list's cell, a ø item is kept
fn list_node(first: Option<Node>, rest: Option<Node>, bracket: Bracket) -> Node {
	let mut items: Vec<Node> = first.into_iter().collect();
	match rest {
		Some(Node::List(rest_items, _, _)) => items.extend(rest_items),
		Some(Node::Empty) | None => {}
		Some(other) => items.push(other),
	}
	Node::List(items, bracket, Separator::None)
}

/// A Float payload: `{"float": number}`, or a text for what JSON has no number for (`"NaN"`, `"Infinity"`, `"-0"`)
fn payload_float(payload: &Value) -> Option<f64> {
	match payload.get("float")? {
		Value::String(text) => text.parse().ok(),
		number => number.as_f64(),
	}
}

/// An Int payload: `{"int": "decimal"}`, `{"big": {"negative", "limbs"}}` or `{"ratio": [numerator, denominator]}`
fn payload_number(payload: &Value) -> Option<Number> {
	if let Some(decimal) = payload.get("int") {
		return match decimal {
			Value::String(text) => text.parse().ok().map(Number::Int),
			number => number.as_i64().map(Number::Int),
		};
	}
	if let Some(big) = payload.get("big") {
		let limbs: Vec<u32> = big.get("limbs")?.as_array()?.iter().filter_map(Value::as_u64).map(|limb| limb as u32).collect();
		let sign = if big.get("negative").and_then(Value::as_bool).unwrap_or(false) { Sign::Minus } else { Sign::Plus };
		return Some(Number::from_bigint(BigInt::from_slice(sign, &limbs)));
	}
	let [numerator, denominator] = payload.get("ratio")?.as_array()?.as_slice() else { return None };
	Some(Number::ratio(payload_number(numerator)?, payload_number(denominator)?))
}

// ============================================================================
// The host side in the browser: the page runs modules, the compiler exports its entry points
// ============================================================================

#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
mod page {
	#[link(wasm_import_module = "warp_host")]
	extern "C" {
		/// Instantiate and run the module, keep the outcome JSON; returns its length in bytes
		pub fn run(wasm: *const u8, length: usize) -> usize;
		/// Copy the kept outcome JSON to `into`
		pub fn take(into: *mut u8);
		/// Run the handler of page event `event` (`click·1`) with `detail` (JSON) in the last run that listens, then read its
		/// page·value; keep that outcome JSON, returns its length in bytes (headless.rs)
		pub fn page_event(event: *const u8, event_length: usize, detail: *const u8, detail_length: usize) -> usize;
		/// Milliseconds since the epoch (Date.now)
		pub fn now_ms() -> f64;
		/// A panic message of the compiler, shown instead of a bare `unreachable`
		pub fn panicked(message: *const u8, length: usize);
		/// Fetch a file (a path of the served repository, a PAGE_PREFIX file of the page or a URL) and keep its bytes; their
		/// count, -1 when missing
		pub fn fetch(url: *const u8, length: usize) -> isize;
		/// Copy the kept fetched bytes to `into`
		pub fn take_fetched(into: *mut u8);
	}
}

/// The text of a file (module sources, packages, C headers): see read_bytes; `None` when missing
pub fn read_text(path: &str) -> Option<String> {
	read_bytes(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Is there a file at `path` (see read_bytes)
pub fn file_exists(path: &str) -> bool {
	#[cfg(not(all(target_arch = "wasm32", not(feature = "native"))))]
	return std::path::Path::new(path).is_file();
	#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
	return read_bytes(path).is_some();
}

/// Outside the page: the file itself
#[cfg(not(all(target_arch = "wasm32", not(feature = "native"))))]
pub fn read_bytes(path: &str) -> Option<Vec<u8>> {
	std::fs::read(path).ok()
}

/// A file the page fetches for the compiler, remembered per address: a path of the served repository, a file of the page
/// itself (PAGE_PREFIX `page:lib/libc.h`, resolved by web/playground/host.js against the page) or a URL
#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
pub fn read_bytes(address: &str) -> Option<Vec<u8>> {
	// the browser tests (wasm32-wasip1) have a file system, the served repository plus the files a test writes
	// (web/playground/wasi.js): read files there first, unremembered, since a test may write them later
	#[cfg(target_os = "wasi")]
	if !address.contains(':') {
		if let Ok(bytes) = std::fs::read(address) {
			return Some(bytes);
		}
	}
	thread_local! {
		static FETCHED: RefCell<std::collections::HashMap<String, Option<Vec<u8>>>> = RefCell::new(std::collections::HashMap::new());
	}
	if let Some(known) = FETCHED.with(|fetched| fetched.borrow().get(address).cloned()) {
		return known;
	}
	let length = unsafe { page::fetch(address.as_ptr(), address.len()) };
	let bytes = (length >= 0).then(|| {
		let mut bytes = vec![0u8; length as usize];
		unsafe { page::take_fetched(bytes.as_mut_ptr()) };
		bytes
	});
	FETCHED.with(|fetched| fetched.borrow_mut().insert(address.to_string(), bytes.clone()));
	bytes
}

/// Addresses of the page's own files (web/playground/host.js PAGE_PREFIX): `lib/libc.h` sits beside the page, which is
/// the site root when deployed but web/playground/ of the served repository locally
pub const PAGE_PREFIX: &str = "page:";

/// Run a compiled module in the embedding host and read its outcome
#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
pub fn run_in_host(wasm: &[u8]) -> Node {
	let mut outcome = vec![0u8; unsafe { page::run(wasm.as_ptr(), wasm.len()) }];
	unsafe { page::take(outcome.as_mut_ptr()) };
	match serde_json::from_slice::<Value>(&outcome) {
		Ok(outcome) => run_outcome(&outcome),
		Err(problem) => crate::node::error(&format!("could not read the outcome of the run: {problem}")),
	}
}

/// A page event in the embedding host's last listening run: what the page shows after its handler (headless.rs)
#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
pub fn page_event_in_host(event: &str, detail: &Value) -> Node {
	let detail = detail.to_string();
	let mut outcome = vec![0u8; unsafe { page::page_event(event.as_ptr(), event.len(), detail.as_ptr(), detail.len()) }];
	unsafe { page::take(outcome.as_mut_ptr()) };
	match serde_json::from_slice::<Value>(&outcome) {
		Ok(outcome) => run_outcome(&outcome),
		Err(problem) => crate::node::error(&format!("could not read the outcome of the page event: {problem}")),
	}
}

#[cfg(not(any(target_arch = "wasm32", feature = "native")))]
pub fn run_in_host(_wasm: &[u8]) -> Node {
	crate::node::error("this build of warp has no runner: build it with the native feature or run it in web/playground")
}

#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
pub fn now_nanos() -> i128 {
	(unsafe { page::now_ms() } * 1e6) as i128
}

#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
mod exports {
	use super::*;

	thread_local! {
		static REPORT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
	}

	fn text<'a>(pointer: *const u8, length: usize) -> &'a str {
		if length == 0 {
			return "";
		}
		std::str::from_utf8(unsafe { std::slice::from_raw_parts(pointer, length) }).unwrap_or_default()
	}

	/// Memory for the page to write a text into, freed by `web_free`
	#[no_mangle]
	pub extern "C" fn web_alloc(length: usize) -> *mut u8 {
		std::mem::ManuallyDrop::new(vec![0u8; length.max(1)]).as_mut_ptr()
	}

	#[no_mangle]
	pub extern "C" fn web_free(pointer: *mut u8, length: usize) {
		drop(unsafe { Vec::from_raw_parts(pointer, length.max(1), length.max(1)) });
	}

	/// Evaluate the code with the acknowledged topics (see `acknowledged_topics`); returns the length of the report,
	/// read at `web_report()`
	#[no_mangle]
	pub extern "C" fn web_evaluate(code: *const u8, code_length: usize, acknowledged: *const u8, acknowledged_length: usize) -> usize {
		static HOOK: std::sync::Once = std::sync::Once::new();
		HOOK.call_once(|| std::panic::set_hook(Box::new(|info| {
			let message = info.to_string();
			unsafe { page::panicked(message.as_ptr(), message.len()) }
		})));
		let acknowledged = super::acknowledged_topics(text(acknowledged, acknowledged_length));
		let report = evaluate(text(code, code_length), acknowledged).to_string().into_bytes();
		REPORT.with(|kept| {
			*kept.borrow_mut() = report;
			kept.borrow().len()
		})
	}

	/// Run a block of a running program (host.js run_block): the request JSON as eval_block_report takes it; returns the
	/// length of the report, read at `web_report()`
	#[no_mangle]
	pub extern "C" fn web_eval_block(request: *const u8, request_length: usize) -> usize {
		let request = serde_json::from_str::<Value>(text(request, request_length)).unwrap_or_default();
		let report = eval_block_report(&request).to_string().into_bytes();
		REPORT.with(|kept| {
			*kept.borrow_mut() = report;
			kept.borrow().len()
		})
	}

	/// The value of a run outcome (as run_outcome reads it) as wasp text: what a page event handler gave (worker.js
	/// handleEvent, notes/signals.md phase 7); returns the length of the text, read at `web_report()`
	#[no_mangle]
	pub extern "C" fn web_show(outcome: *const u8, outcome_length: usize) -> usize {
		let outcome = serde_json::from_str::<Value>(text(outcome, outcome_length)).unwrap_or_default();
		let shown = super::shown(&outcome).to_string().into_bytes();
		REPORT.with(|kept| {
			*kept.borrow_mut() = shown;
			kept.borrow().len()
		})
	}

	#[no_mangle]
	pub extern "C" fn web_report() -> *const u8 {
		REPORT.with(|kept| kept.borrow().as_ptr())
	}
}

