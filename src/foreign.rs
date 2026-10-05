//! The host side of foreign_call(runtime, module, member, arguments) (lowering/foreign_modules.rs,
//! notes/stdlib_connectors.md): one long-lived child per runtime and warp process, started on first use, speaking one
//! JSON line per request and per answer. Python: `python3 -u` running FOREIGN_PYTHON_LOOP (`WARP_PYTHON` names another
//! interpreter). A read (`arguments` ø) gives the member itself, a call gives its result; values without a JSON form
//! come back as their repr text; an exception is an error with the Python message.

use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

const PYTHON: &str = "python";
const PYTHON_VARIABLE: &str = "WARP_PYTHON";
const DEFAULT_PYTHON: &str = "python3";
/// An integer beyond 64 bits, sent as its digits: `{"$int": "15511210043330985984000000"}`
const BIG_INTEGER_KEY: &str = "$int";
/// The request loop the child runs: {"module", "member", "arguments"} → {"value"} or {"error"}
const FOREIGN_PYTHON_LOOP: &str = r#"
import sys, json, importlib
def plain(value):
    if isinstance(value, int) and not isinstance(value, bool) and not -2**63 <= value < 2**63:
        return {"$int": str(value)}
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, (list, tuple, set, frozenset)):
        return [plain(item) for item in value]
    if isinstance(value, dict):
        return {str(key): plain(item) for key, item in value.items()}
    if hasattr(value, "tolist"):
        return plain(value.tolist())
    return repr(value)
for line in sys.stdin:
    request = json.loads(line)
    try:
        value = importlib.import_module(request["module"])
        for part in request["member"].split("."):
            value = getattr(value, part)
        if request["arguments"] is not None:
            value = value(*request["arguments"])
        answer = {"value": plain(value)}
    except BaseException as failure:
        answer = {"error": type(failure).__name__ + ": " + str(failure)}
    print(json.dumps(answer), flush=True)
"#;

struct Runtime {
	_child: Child,
	input: ChildStdin,
	output: BufReader<ChildStdout>,
}

static PYTHON_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

/// `runtime.module.member(arguments…)` (a read when `arguments` is ø) as a Node
pub fn call(runtime: &str, module: &str, member: &str, arguments: &Node) -> Result<Node, String> {
	if runtime != PYTHON {
		return Err(format!("no foreign runtime {runtime}: known are {}", crate::foreign_modules::FOREIGN_RUNTIMES.join(", ")));
	}
	let arguments = match arguments.drop_meta() {
		Node::Empty => Value::Null,
		Node::List(items, _, _) => Value::Array(items.iter().map(json_of).collect()),
		single => Value::Array(vec![json_of(single)]),
	};
	let request = serde_json::json!({"module": module, "member": member, "arguments": arguments});
	let answer = exchange(&request.to_string()).map_err(|failure| format!("{runtime}: {failure}"))?;
	let answer: Value = serde_json::from_str(&answer).map_err(|failure| format!("{runtime} answered no JSON: {failure}"))?;
	match (answer.get("value"), answer.get("error")) {
		(_, Some(Value::String(error))) => Err(format!("{runtime} {module}.{member}: {error}")),
		(Some(value), _) => Ok(node_of(value)),
		_ => Err(format!("{runtime} answered neither a value nor an error: {answer}")),
	}
}

/// One request line out, one answer line back; the child is started on first use and again after it ended
fn exchange(request: &str) -> Result<String, String> {
	let mut runtime = PYTHON_RUNTIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if runtime.is_none() {
		*runtime = Some(start_python()?);
	}
	let running = runtime.as_mut().expect("started");
	let mut answer = String::new();
	let sent = writeln!(running.input, "{request}").and_then(|_| running.input.flush());
	if sent.is_err() || running.output.read_line(&mut answer).map_err(|failure| failure.to_string())? == 0 {
		*runtime = None;
		return Err("the interpreter ended".into());
	}
	Ok(answer)
}

fn start_python() -> Result<Runtime, String> {
	let python = std::env::var(PYTHON_VARIABLE).unwrap_or_else(|_| DEFAULT_PYTHON.to_string());
	let mut child = Command::new(&python).args(["-u", "-c", FOREIGN_PYTHON_LOOP]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
		.spawn().map_err(|failure| format!("cannot start {python} ({failure}); set {PYTHON_VARIABLE} to an interpreter"))?;
	let input = child.stdin.take().ok_or("no stdin")?;
	let output = BufReader::new(child.stdout.take().ok_or("no stdout")?);
	Ok(Runtime { _child: child, input, output })
}

fn json_of(node: &Node) -> Value {
	match node.drop_meta() {
		Node::List(items, Bracket::Square | Bracket::Round | Bracket::None, _) => Value::Array(items.iter().map(json_of).collect()),
		Node::Key(key, Op::Colon, value) => serde_json::json!({ key.name(): json_of(value) }),
		// an exact rational (`2.5` is 5/2) crosses as a float, an integer as itself
		Node::Number(Number::Int(integer)) => Value::from(*integer),
		Node::Number(number) => Value::from(f64::from(*number)),
		other => serde_json::from_str(&other.to_json_compact().unwrap_or_default()).unwrap_or(Value::Null),
	}
}

/// A JSON value as a Node: null ø, booleans 1/0, objects `{key:value …}`
fn node_of(value: &Value) -> Node {
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
		Value::Object(entries) => Node::List(
			entries.iter().map(|(key, value)| Node::Key(Box::new(Node::Symbol(key.clone())), Op::Colon, Box::new(node_of(value)))).collect(),
			Bracket::Curly,
			Separator::Space,
		),
	}
}
