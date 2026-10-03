//! The compiler in the browser (web/playground): `evaluate` compiles a program the way the CLI does and reports
//! what the CLI prints (the value, warnings, hints) plus the topics of the warnings and notes the page offers "got it" for. Without wasmtime the
//! page runs the compiled module (`run_in_host`) and hands its result back as a JSON tree that its reader
//! (web/playground/reader.js) walked through the module's reflection exports (wasm_emitter/reflection.rs).

use crate::diagnostic::{self, Acknowledger};
use crate::extensions::numbers::Number;
use crate::meta::{Dada, DataType};
use crate::node::{Bracket, Node, Separator};
use crate::type_kinds::{Kind, KIND_MASK};
use num_bigint::{BigInt, Sign};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use crate::type_kinds::KIND_BITS;

/// The topics the page already said "got it" to; every other warning or note shown is recorded for its "got it" button
struct PageAcknowledger {
	acknowledged: HashSet<String>,
	notes: Rc<RefCell<Vec<String>>>,
}

impl Acknowledger for PageAcknowledger {
	fn acknowledge(&self, topic: &str) -> bool {
		let acknowledged = self.acknowledged.contains(topic);
		if !acknowledged {
			self.notes.borrow_mut().push(topic.to_string());
		}
		acknowledged
	}
}

/// Compile and run `code` as `warp file.wasp` does, with the topics the page acknowledged. The report (JSON):
/// `value` (what the CLI prints), `error`, `warnings`, `hints`, `notes` (topics of the warnings and notes shown that
/// the user can say "got it" to) and `runtime_warnings`. Acknowledging silences a warning, never changes the value.
pub fn evaluate(code: &str, acknowledged: HashSet<String>) -> Value {
	let notes = Rc::new(RefCell::new(Vec::new()));
	let acknowledger = PageAcknowledger { acknowledged, notes: notes.clone() };
	diagnostic::take_warnings();
	diagnostic::take_runtime_warnings();
	let (result, hints) = diagnostic::with_acknowledger(acknowledger, || crate::normalize::capture_hints(|| crate::wasm_emitter::eval(code)));
	let warnings: Vec<Value> = diagnostic::take_warnings().iter().map(|warning| json!({
		"message": warning.message, "line": warning.line, "column": warning.column, "fix": warning.fix,
	})).collect();
	let hints: Vec<Value> = hints.iter().map(|hint| json!({
		"original": hint.original, "canonical": hint.canonical, "position": hint.position, "reason": hint.reason,
	})).collect();
	json!({
		"value": result.serialize(),
		"error": matches!(result.drop_meta(), Node::Error(_)),
		"warnings": warnings,
		"runtime_warnings": diagnostic::take_runtime_warnings(),
		"hints": hints,
		"asks": [], // no Asks any more (every ambiguity is a warning or an error); kept until the page stops reading it
		"notes": *notes.borrow(),
	})
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
	let failure = outcome.get("failure").and_then(Value::as_str).unwrap_or("the page sent no outcome");
	crate::node::error(&format!("could not run the program: {failure}"))
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
		tag if tag == Kind::Block as i64 => list_node(data_node(), value, Bracket::Curly),
		tag if tag == Kind::List as i64 => list_node(data_node(), value, bracket_of(info)),
		tag if tag == Kind::Data as i64 => {
			let type_name = text();
			Node::Data(Dada { data: Box::new(format!("<wasm data: {type_name}>")), type_name, data_type: DataType::Other })
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

/// A cons cell: its first item, then the items of the rest
fn list_node(first: Node, rest: Option<Node>, bracket: Bracket) -> Node {
	let mut items: Vec<Node> = Some(first).into_iter().filter(|item| *item != Node::Empty).collect();
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
		/// Milliseconds since the epoch (Date.now)
		pub fn now_ms() -> f64;
		/// A panic message of the compiler, shown instead of a bare `unreachable`
		pub fn panicked(message: *const u8, length: usize);
	}
}

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

	#[no_mangle]
	pub extern "C" fn web_report() -> *const u8 {
		REPORT.with(|kept| kept.borrow().as_ptr())
	}
}

