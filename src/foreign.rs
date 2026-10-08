//! The host side of foreign_call(runtime, module, member, arguments) (lowering/foreign_modules.rs,
//! notes/stdlib_connectors.md): one long-lived child per runtime and warp process, started on first use, speaking one
//! JSON line per request and per answer. Python: `python3 -u` running FOREIGN_PYTHON_LOOP (`WARP_PYTHON` names another
//! interpreter); JavaScript: `node` running FOREIGN_JS_LOOP (`WARP_NODE`), a global (`Math`) or a module it requires.
//! A read gives the member itself, a call gives its result (a promise awaited); an exception is an error with the
//! runtime's message. A value without a JSON form stays in its runtime behind an id, a handle: it crosses as the record
//! `{$handle: 7, type: "date", text: "datetime.date(2020, 1, 2)"}` and is that object again when it comes back, as an
//! argument or as the receiver of `d.isoformat()` (the module position). Handles live until warp ends.

use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

/// A runtime: its name in `use <runtime> …`, the variable naming its interpreter, the default interpreter, and how it
/// runs its loop script
struct Interpreter {
	runtime: &'static str,
	variable: &'static str,
	default: &'static str,
	arguments: [&'static str; 2],
	script: &'static str,
}

const INTERPRETERS: [Interpreter; 2] = [
	Interpreter { runtime: "python", variable: "WARP_PYTHON", default: "python3", arguments: ["-u", "-c"], script: FOREIGN_PYTHON_LOOP },
	Interpreter { runtime: "js", variable: "WARP_NODE", default: "node", arguments: ["--no-warnings", "-e"], script: FOREIGN_JS_LOOP },
];
/// An integer beyond 64 bits, sent as its digits: `{"$int": "15511210043330985984000000"}`
const BIG_INTEGER_KEY: &str = "$int";
/// A codepoint (a WIT `char`, P94), which JSON would make a text: `{"$char": "b"}`
pub const CODEPOINT_KEY: &str = "$char";
/// The request loop the child runs: {"module", "member", "arguments"} → {"value"} or {"error"}
const FOREIGN_PYTHON_LOOP: &str = include_str!("../web/playground/foreign_python.py");

/// The same loop for node: a global (`Math`, `JSON`) or a module it requires or imports; a method keeps its object
const FOREIGN_JS_LOOP: &str = r#"
const handles = [];
const handle = value => (handles.push(value), { "$handle": handles.length, type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unplain = value => Array.isArray(value) ? value.map(unplain)
	: value !== null && typeof value === "object" ? ("$handle" in value ? handles[value.$handle - 1] : Object.fromEntries(Object.entries(value).map(([key, item]) => [key, unplain(item)])))
	: value;
const plain = value => {
	if (typeof value === "bigint") return value >= -(2n ** 63n) && value < 2n ** 63n ? Number(value) : { "$int": value.toString() };
	if (value === undefined || typeof value === "symbol") return null;
	if (Array.isArray(value)) return value.map(plain);
	const prototype = value !== null && typeof value === "object" ? Object.getPrototypeOf(value) : undefined;
	if (prototype === Object.prototype || prototype === null) return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, plain(item)]));
	if (typeof value === "function" || typeof value === "object") return handle(value); // a Date, a Map, an instance, a function
	return value;
};
// what wasp's operators on a value of this runtime forward to (lowering/foreign_modules.rs)
const operator = { add: (a, b) => a + b, sub: (a, b) => a - b, mul: (a, b) => a * b, truediv: (a, b) => a / b, mod: (a, b) => a % b, pow: (a, b) => a ** b,
	lt: (a, b) => a < b, gt: (a, b) => a > b, le: (a, b) => a <= b, ge: (a, b) => a >= b, eq: (a, b) => a === b, ne: (a, b) => a !== b, neg: a => -a,
	getitem: (a, i) => typeof a.get === "function" ? a.get(i) : a[i], len: a => a.length ?? a.size, list: a => Array.from(a) };
const load = async name => name === "operator" ? operator : name in globalThis ? globalThis[name] : (() => { try { return require(name); } catch (failure) { return import(name); } })();
// the answers own stdout: what a module prints (console.log) goes to stderr
const reply = process.stdout.write.bind(process.stdout);
process.stdout.write = process.stderr.write.bind(process.stderr);
let pending = Promise.resolve();
require("readline").createInterface({ input: process.stdin }).on("line", line => {
	pending = pending.then(async () => {
		const request = JSON.parse(line);
		let answer;
		try {
			let owner = null, value = typeof request.module === "object" ? unplain(request.module) : await load(request.module);
			for (const part of request.member.split(".")) {
				if (value?.[part] === undefined) throw new ReferenceError(`${request.module} has no ${request.member}`);
				[owner, value] = [value, value[part]];
			}
			if (request.arguments !== null) value = await value.apply(owner, unplain(request.arguments));
			answer = { value: plain(value) };
		} catch (failure) {
			answer = { error: failure instanceof Error ? failure.name + ": " + failure.message : String(failure) };
		}
		reply(JSON.stringify(answer) + "\n");
	});
});
"#;

struct Runtime {
	_child: Child,
	input: ChildStdin,
	output: BufReader<ChildStdout>,
}

/// The running children, by runtime
static RUNNING: Mutex<Vec<(&'static str, Runtime)>> = Mutex::new(Vec::new());

/// `runtime.module.member(arguments…)`, or the member itself when it is no call, as a Node; `module` is a module's name
/// or a handle
pub fn call(runtime: &str, module: &Node, member: &str, is_call: bool, arguments: &Node) -> Result<Node, String> {
	let arguments = match arguments.drop_meta() {
		_ if !is_call => Value::Null,
		Node::Empty => Value::Array(vec![]),
		Node::List(items, _, _) => Value::Array(items.iter().map(json_of).collect()),
		single => Value::Array(vec![json_of(single)]),
	};
	if runtime == crate::foreign_modules::COMPONENT_RUNTIME {
		return component_call(module, member, &arguments);
	}
	let Some(interpreter) = INTERPRETERS.iter().find(|interpreter| interpreter.runtime == runtime) else {
		return Err(format!("no foreign runtime {runtime}: known are {}", crate::foreign_modules::FOREIGN_RUNTIMES.join(", ")));
	};
	let (module, module_json) = match module.drop_meta() {
		Node::Text(name) | Node::Symbol(name) => (name.clone(), Value::String(name.clone())),
		handle => (handle.serialize(), json_of(handle)),
	};
	let request = serde_json::json!({"module": module_json, "member": member, "arguments": arguments});
	let answer = exchange(interpreter, &request.to_string()).map_err(|failure| format!("{runtime}: {failure}"))?;
	let answer: Value = serde_json::from_str(&answer).map_err(|failure| format!("{runtime} answered no JSON: {failure}"))?;
	match (answer.get("value"), answer.get("error")) {
		(_, Some(Value::String(error))) => Err(format!("{runtime} {module}.{member}: {error}")),
		(Some(value), _) => Ok(node_of(value)),
		_ => Err(format!("{runtime} answered neither a value nor an error: {answer}")),
	}
}

/// `lib.f(x)` of a component (src/components.rs), or `handle.method(x)` of a resource it gave: only calls
fn component_call(module: &Node, member: &str, arguments: &Value) -> Result<Node, String> {
	let receiver = match module.drop_meta() {
		Node::Text(path) | Node::Symbol(path) => path.clone(),
		handle => json_of(handle).get("text").and_then(Value::as_str).map_or_else(|| handle.serialize(), str::to_string),
	};
	let Value::Array(arguments) = arguments else {
		return Err(format!("wasm {receiver}: {member} is a function of the component, call it: {member}(…)"));
	};
	let result = match module.drop_meta() {
		Node::Text(path) | Node::Symbol(path) => crate::components::call(path, member, arguments),
		handle => crate::components::call_method(&json_of(handle), member, arguments),
	};
	result.map(|value| node_of(&value)).map_err(|failure| format!("wasm {receiver}: {failure}"))
}

/// One request line out, one answer line back; the child is started on first use and again after it ended
fn exchange(interpreter: &'static Interpreter, request: &str) -> Result<String, String> {
	let mut running = RUNNING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if !running.iter().any(|(runtime, _)| *runtime == interpreter.runtime) {
		running.push((interpreter.runtime, start(interpreter)?));
	}
	let index = running.iter().position(|(runtime, _)| *runtime == interpreter.runtime).expect("started");
	let child = &mut running[index].1;
	let mut answer = String::new();
	let sent = writeln!(child.input, "{request}").and_then(|_| child.input.flush());
	if sent.is_err() || child.output.read_line(&mut answer).map_err(|failure| failure.to_string())? == 0 {
		running.remove(index);
		return Err("the interpreter ended".into());
	}
	Ok(answer)
}

fn start(interpreter: &Interpreter) -> Result<Runtime, String> {
	let program = std::env::var(interpreter.variable).unwrap_or_else(|_| interpreter.default.to_string());
	let mut child = Command::new(&program).args(interpreter.arguments).arg(interpreter.script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
		.spawn().map_err(|failure| format!("cannot start {program} ({failure}); set {} to an interpreter", interpreter.variable))?;
	let input = child.stdin.take().ok_or("no stdin")?;
	let output = BufReader::new(child.stdout.take().ok_or("no stdout")?);
	Ok(Runtime { _child: child, input, output })
}

pub(crate) fn json_of(node: &Node) -> Value {
	match node.drop_meta() {
		Node::List(items, Bracket::Square | Bracket::Round | Bracket::None, _) => Value::Array(items.iter().map(json_of).collect()),
		Node::Key(key, Op::Colon, value) => serde_json::json!({ key.name(): json_of(value) }),
		// an object, a handle among them: `{$handle: 7, …}`
		Node::List(items, Bracket::Curly, _) if items.iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) => Value::Object(
			items.iter().filter_map(|item| match item.drop_meta() {
				Node::Key(key, _, value) => Some((key.name(), json_of(value))),
				_ => None,
			}).collect(),
		),
		// an exact rational (`2.5` is 5/2) crosses as a float, an integer as itself
		Node::Number(Number::Int(integer)) => Value::from(*integer),
		Node::Number(number) => Value::from(f64::from(*number)),
		other => serde_json::from_str(&other.to_json_compact().unwrap_or_default()).unwrap_or(Value::Null),
	}
}

/// The character of `{"$char": "b"}`
fn codepoint_of(entries: &serde_json::Map<String, Value>) -> Option<char> {
	let mut characters = entries.get(CODEPOINT_KEY)?.as_str()?.chars();
	match (entries.len(), characters.next(), characters.next()) {
		(1, Some(character), None) => Some(character),
		_ => None,
	}
}

/// A JSON value as a Node: null ø, booleans 1/0, objects `{key:value …}`
pub(crate) fn node_of(value: &Value) -> Node {
	match value {
		Value::Null => Node::Empty,
		Value::Bool(truth) => Node::Number(Number::Int(i64::from(*truth))),
		Value::Number(number) => match number.as_i64() {
			Some(integer) => Node::Number(Number::Int(integer)),
			None => Node::Number(Number::Float(number.as_f64().unwrap_or(f64::NAN))),
		},
		Value::String(text) => Node::Text(text.clone()),
		Value::Array(items) => Node::List(items.iter().map(node_of).collect(), Bracket::Square, Separator::Space),
		Value::Object(entries) if entries.len() == 1 && entries.get(BIG_INTEGER_KEY).is_some_and(Value::is_string) => {
			let digits = entries[BIG_INTEGER_KEY].as_str().unwrap_or_default();
			digits.parse::<num_bigint::BigInt>().map(|big| Node::Number(Number::BigInt(Box::leak(Box::new(big))))).unwrap_or_else(|_| Node::Text(digits.to_string()))
		}
		Value::Object(entries) if let Some(character) = codepoint_of(entries) => Node::Char(character),
		Value::Object(entries) => Node::List(
			entries.iter().map(|(key, value)| Node::Key(Box::new(Node::Symbol(key.clone())), Op::Colon, Box::new(node_of(value)))).collect(),
			Bracket::Curly,
			Separator::Space,
		),
	}
}
