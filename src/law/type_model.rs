//! The differential check between warp's type checks and W0, the Lean model of warp's type theory whose soundness is
//! proved (notes/type_theory.md, lean/WarpTypes). A warp program in W0's subset is exported as a list of Lean `Item`s
//! (Checker.lean: names, charged names, functions, statements in source order); the model elaborates and checks it
//! and answers `ok <type>` or `rejected`. Warp's own verdict is whether `pipeline::compile` accepts the program.
//! Contract: W0 rejects ⇒ warp rejects; every exception is a known hole with a card.
use super::lean::{lean_executable, run_with_timeout};
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::TRY_MARKER;
use crate::type_kinds::Kind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The lake project of the model, in the source tree
pub const MODEL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lean/WarpTypes");
/// `lake build` of one project must not run twice at once
static MODEL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
const ACCEPTED_PREFIX: &str = "ok ";
const REJECTED: &str = "rejected";
const CONSTANT_KEYWORD: &str = "const";
const ERROR_CALL: &str = "error";
const BOOL_TYPE: &str = ".bool";
const ANY_TYPE: &str = ".any";
/// The builtin scalar type words warp checks a value against (analyzer admits) that W0 has a type for
const BUILTIN_TYPE_WORDS: [&str; 11] = ["int", "integer", "long", "exact", "float", "number", "text", "string", "str", "bool", "boolean"];
/// A value of each W0 scalar type and the run-time kind warp sees it as (a bool is an Int)
const VALUE_KINDS: [(&str, Kind); 4] = [(BOOL_TYPE, Kind::Int), (".int", Kind::Int), (".number", Kind::Float), (".text", Kind::Text)];
const APPEND_METHODS: [&str; 2] = ["add", "push"];

/// W0's answer for one program
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelVerdict {
	Accepted(String),
	Rejected,
}

type Lean = Result<String, String>;

fn unsupported(node: &Node) -> Lean {
	Err(format!("not in W0: {}", node.serialize().trim()))
}

fn quoted(text: &str) -> String {
	format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `int`, `texts`, `list`: the W0 type a warp type word names
fn type_of_word(word: &str) -> Option<String> {
	let scalar = |word: &str| -> Option<&str> {
		Some(match crate::type_kinds::canonical_type_name(word) {
			"int" | "integer" | "long" => ".int",
			"float" | "number" | "exact" => ".number",
			"text" | "string" | "str" => ".text",
			"bool" | "boolean" => BOOL_TYPE,
			"any" => ANY_TYPE,
			_ => return None,
		})
	};
	if word == "list" {
		return Some(".list .any".to_string());
	}
	match scalar(word) {
		Some(lean) => Some(lean.to_string()),
		None => word.strip_suffix('s').and_then(scalar).map(|element| format!(".list {element}")),
	}
}

fn annotation_type(annotation: &Node) -> Lean {
	match annotation.drop_meta() {
		Node::Symbol(word) => type_of_word(word).ok_or_else(|| format!("not in W0: type {word}")),
		other => unsupported(other),
	}
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(found) if found == word)
}

fn is_list_literal(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Square, _))
}

/// The statements of a program or block: an unbracketed `;`/newline list, else the one node
fn statements(node: &Node) -> Vec<&Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.iter().collect(),
		other => vec![other],
	}
}

/// `f(y)`, `f(y: int)`: a definition head of one parameter
fn function_head(head: &Node) -> Option<(&str, &Node)> {
	match head.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[name, parameter] => match name.drop_meta() {
				Node::Symbol(name) => Some((name.as_str(), parameter)),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// A bool place (a declared variable or parameter) takes the literals 1 and 0 as yes and no (P199)
fn bool_literal(declared: Option<&str>, value: &Node) -> Option<String> {
	match value.drop_meta() {
		Node::Number(Number::Int(n @ (0 | 1))) if declared == Some(BOOL_TYPE) => Some(format!(".bool {}", *n == 1)),
		_ => None,
	}
}

/// `y`, `y: int`: the parameter's name and W0 type
fn parameter_type(parameter: &Node) -> Result<(String, String), String> {
	match parameter.drop_meta() {
		Node::Symbol(parameter) => Ok((parameter.clone(), ANY_TYPE.to_string())),
		Node::Key(parameter, Op::Colon, annotation) => Ok((parameter.name(), annotation_type(annotation)?)),
		other => Err(unsupported(other).unwrap_err()),
	}
}

#[derive(Default)]
struct Exporter {
	/// the functions of the program with their parameter's W0 type
	functions: HashMap<String, String>,
	/// main-level names bound so far, with their declared type when annotated
	names: HashMap<String, Option<String>>,
	locals: Vec<String>,
}

impl Exporter {
	fn items(&mut self, program: &Node) -> Result<Vec<String>, String> {
		let program = statements(program);
		for statement in &program {
			if let Node::Key(head, Op::Define, _) = statement.drop_meta() {
				if let Some((name, parameter)) = function_head(head) {
					self.functions.insert(name.to_string(), parameter_type(parameter)?.1);
				}
			}
		}
		program.into_iter().map(|statement| self.item(statement)).collect()
	}

	fn item(&mut self, statement: &Node) -> Lean {
		match statement.drop_meta() {
			Node::Key(head, Op::Define, body) => match (head.drop_meta(), function_head(head)) {
				(Node::Symbol(name), _) => {
					self.names.insert(name.clone(), None);
					Ok(format!(".charged {} ({})", quoted(name), self.expression(body)?))
				}
				(_, Some((name, parameter))) => self.function(name, parameter, body),
				_ => unsupported(statement),
			},
			Node::List(items, _, _) if items.len() == 2 && is_word(&items[0], CONSTANT_KEYWORD) => match items[1].drop_meta() {
				Node::Key(target, Op::Assign, value) => self.binding(target, ".const", value),
				other => unsupported(other),
			},
			Node::Key(target, Op::Assign, value) if !self.is_bound(target) => self.binding(target, ".var", value),
			other => Ok(format!(".statement ({})", self.expression(other)?)),
		}
	}

	fn is_bound(&self, target: &Node) -> bool {
		matches!(target.drop_meta(), Node::Symbol(name) if self.names.contains_key(name))
	}

	/// the first binding of a main-level name: `x = 1`, `x: int = 1`, `const x = 1`
	fn binding(&mut self, target: &Node, mode: &str, value: &Node) -> Lean {
		let (name, annotation) = match target.drop_meta() {
			Node::Symbol(name) => (name.clone(), None),
			Node::Key(name, Op::Colon, annotation) => (name.name(), Some(annotation_type(annotation)?)),
			other => return unsupported(other),
		};
		let value = self.stored_value(annotation.as_deref(), value)?;
		self.names.insert(name.clone(), annotation.clone());
		let annotation = annotation.map_or("none".to_string(), |lean| format!("(some ({lean}))"));
		Ok(format!(".bind {} {mode} {annotation} ({value}) false", quoted(&name)))
	}

	/// A call's result stored in a declared list is checked item by item at run time (list-element-types): a cast
	fn stored_value(&mut self, declared: Option<&str>, value: &Node) -> Lean {
		if let Some(yes_or_no) = bool_literal(declared, value) {
			return Ok(yes_or_no);
		}
		let lean = self.expression(value)?;
		Ok(match declared {
			Some(declared) if declared.starts_with(".list") && self.is_call(value) => format!(".cast ({lean}) ({declared})"),
			_ => lean,
		})
	}

	fn is_call(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.functions.contains_key(name)))
	}

	fn function(&mut self, name: &str, parameter: &Node, body: &Node) -> Lean {
		let (parameter, parameter_type) = parameter_type(parameter)?;
		self.locals.push(parameter.clone());
		let body = self.expression(body);
		self.locals.pop();
		Ok(format!(".function {} {} ({parameter_type}) ({})", quoted(name), quoted(&parameter), body?))
	}

	fn assignment(&mut self, name: &str, value: &Node) -> Lean {
		let declared = self.names.get(name).cloned().flatten();
		let value = self.stored_value(declared.as_deref(), value)?;
		let operation = if self.names.contains_key(name) { ".assign" } else { ".init" };
		Ok(format!("{operation} {} ({value})", quoted(name)))
	}

	fn binary(&mut self, constructor: &str, left: &Node, right: &Node) -> Lean {
		Ok(format!("{constructor} ({}) ({})", self.expression(left)?, self.expression(right)?))
	}

	fn expression(&mut self, node: &Node) -> Lean {
		match node.drop_meta() {
			Node::Number(Number::Int(n)) => Ok(format!(".int ({n})")),
			Node::Number(Number::Float(_) | Number::Quotient(_, _)) => Ok(".num 0".to_string()),
			Node::True => Ok(".bool true".to_string()),
			Node::False => Ok(".bool false".to_string()),
			Node::Text(text) => Ok(format!(".text {}", quoted(text))),
			Node::Char(c) => Ok(format!(".text {}", quoted(&c.to_string()))), // `"a"` parses as a codepoint
			Node::Empty => Ok(".nil".to_string()), // ø is the empty list (`xs = []` parses as ø)
			Node::Symbol(name) if self.locals.contains(name) => Ok(format!(".loc {}", quoted(name))),
			Node::Symbol(name) if self.names.contains_key(name) => Ok(format!(".glob {}", quoted(name))),
			Node::List(items, Bracket::Square, _) => {
				let elements: Result<Vec<String>, String> = items.iter().map(|item| self.expression(item)).collect();
				Ok(elements?.iter().rev().fold(".nil".to_string(), |tail, head| format!(".cons ({head}) ({tail})")))
			}
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.expression(&items[0]),
			Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) if items.len() > 1 => {
				let statements: Result<Vec<String>, String> = items.iter().map(|item| self.expression(item)).collect();
				let mut statements = statements?;
				let last = statements.pop().expect("more than one statement");
				Ok(statements.iter().rev().fold(last, |rest, statement| format!(".seq ({statement}) ({rest})")))
			}
			Node::List(items, _, _) => match items.as_slice() {
				[marker, body, handler] if is_word(marker, TRY_MARKER) => self.binary(".tryCatch", body, handler),
				[call, message] if is_word(call, ERROR_CALL) => match message.drop_meta() {
					Node::Text(message) => Ok(format!(".error {}", quoted(message))),
					_ => unsupported(node),
				},
				[call, argument] if self.functions.contains_key(&call.name()) => {
					let declared = self.functions[&call.name()].clone();
					let argument = match bool_literal(Some(&declared), argument) {
						Some(yes_or_no) => yes_or_no,
						None => self.expression(argument)?,
					};
					Ok(format!(".call {} ({argument})", quoted(&call.name())))
				}
				_ => unsupported(node),
			},
			Node::Key(target, Op::Assign, value) => match target.drop_meta() {
				Node::Symbol(name) => self.assignment(name, value),
				_ => unsupported(node),
			},
			// `xs.add(v)` is `xs = xs ++ [v]`: lists are values
			Node::Key(list, Op::Dot, call) => match (list.drop_meta(), call.drop_meta()) {
				(Node::Symbol(name), Node::List(items, _, _)) if items.len() == 2 && APPEND_METHODS.iter().any(|method| is_word(&items[0], method)) => {
					let item = self.expression(&items[1])?;
					let operation = if self.names.contains_key(name) { ".assign" } else { return unsupported(node) };
					Ok(format!("{operation} {} (.append (.glob {}) (.cons ({item}) .nil))", quoted(name), quoted(name)))
				}
				_ => unsupported(node),
			},
			Node::Key(if_then, Op::Else, otherwise) => match if_then.drop_meta() {
				Node::Key(condition, Op::Then, then) => match condition.drop_meta() {
					Node::Key(empty, Op::If, condition) if empty.is_nothing() => {
						Ok(format!(".ite ({}) ({}) ({})", self.expression(condition)?, self.expression(then)?, self.expression(otherwise)?))
					}
					_ => unsupported(node),
				},
				_ => unsupported(node),
			},
			Node::Key(condition, Op::Then, then) => match condition.drop_meta() {
				Node::Key(empty, Op::If, condition) if empty.is_nothing() => {
					Ok(format!(".ite ({}) ({}) .unit", self.expression(condition)?, self.expression(then)?))
				}
				_ => unsupported(node),
			},
			Node::Key(condition, Op::Do, body) => match condition.drop_meta() {
				Node::Key(empty, Op::While, condition) if empty.is_nothing() => self.binary(".loop", condition, body),
				_ => unsupported(node),
			},
			Node::Key(left, Op::Add, right) if is_list_literal(left) || is_list_literal(right) => self.binary(".append", left, right),
			Node::Key(left, Op::Add | Op::Sub | Op::Mul, right) if !left.is_nothing() => self.binary(".add", left, right),
			Node::Key(left, Op::Lt | Op::Le, right) => self.binary(".lt", left, right),
			Node::Key(left, Op::Gt | Op::Ge, right) => self.binary(".lt", right, left),
			Node::Key(left, Op::Eq | Op::Ne, right) => self.binary(".eq", left, right),
			Node::Key(list, Op::Hash, index) if !list.is_nothing() => self.binary(".index", list, index),
			_ => unsupported(node),
		}
	}
}

/// The program as the Lean list of W0 items, or why it is outside W0
pub fn export(code: &str) -> Result<String, String> {
	let program = crate::wasp_parser::parse(code);
	let items = Exporter::default().items(&program)?;
	Ok(format!("[{}]", items.join(",\n  ")))
}

/// Runs Lean commands against the built model, from `.lake/<name>.lean` (one file per kind of request: tests run in
/// parallel)
fn ask_model(name: &str, requests: &str) -> Result<String, String> {
	let _one_lake_at_a_time = MODEL_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	build_model()?;
	let file = Path::new(MODEL_DIR).join(".lake").join(format!("{name}.lean"));
	std::fs::write(&file, format!("import WarpTypes\nopen Warp\n\n{requests}")).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
	let mut command = Command::new(lean_executable("lake"));
	command.current_dir(MODEL_DIR).arg("env").arg("lean").arg(&file);
	run_with_timeout(command)
}

/// Builds the model (`lake build` fails on any proof error) and returns its verdict for each exported program
pub fn verdicts(exported: &[String]) -> Result<Vec<ModelVerdict>, String> {
	let requests: String = exported.iter().map(|items| format!("#eval IO.println (verdict {items})\n")).collect();
	let output = ask_model("verdicts", &requests)?;
	let lines: Vec<&str> = output.lines().filter(|line| line.starts_with(ACCEPTED_PREFIX) || *line == REJECTED).collect();
	if lines.len() != exported.len() {
		return Err(format!("the model answered {} of {} programs:\n{output}", lines.len(), exported.len()));
	}
	Ok(lines.iter().map(|line| match line.strip_prefix(ACCEPTED_PREFIX) {
		Some(type_name) => ModelVerdict::Accepted(type_name.to_string()),
		None => ModelVerdict::Rejected,
	}).collect())
}

/// Where warp's run-time admission of a value to a builtin type (analyzer admits) differs from W0's `Ty.sub`, for
/// every type word and W0 scalar value type: "word ← value type: warp admits / W0 sub"
pub fn admits_disagreements() -> Result<Vec<String>, String> {
	let pairs: Vec<(&str, &str, Kind)> = BUILTIN_TYPE_WORDS.iter().flat_map(|word| VALUE_KINDS.iter().map(move |(value, kind)| (*word, *value, *kind))).collect();
	let requests: String = pairs.iter().map(|(word, value, _)| format!("#eval IO.println (Ty.sub ({value}) ({}))\n", type_of_word(word).expect("a W0 type word"))).collect();
	let output = ask_model("admits", &requests)?;
	let answers: Vec<bool> = output.lines().filter_map(|line| line.parse().ok()).collect();
	if answers.len() != pairs.len() {
		return Err(format!("the model answered {} of {} pairs:\n{output}", answers.len(), pairs.len()));
	}
	Ok(pairs.iter().zip(answers).filter_map(|((word, value, kind), sub)| {
		let admits = crate::analyzer::admits(word, *kind);
		(admits != sub).then(|| format!("{word} ← {value}: warp admits {admits} / W0 sub {sub}"))
	}).collect())
}

/// The axioms the named theorems rest on, as `#print axioms` lists them (a `sorry` shows as sorryAx)
pub fn axioms(theorems: &[&str]) -> Result<String, String> {
	ask_model("axioms", &theorems.iter().map(|theorem| format!("#print axioms {theorem}\n")).collect::<String>())
}

/// `lake build` of the model: every theorem is checked again
pub fn build_model() -> Result<String, String> {
	let mut command = Command::new(lean_executable("lake"));
	command.current_dir(MODEL_DIR).arg("build");
	run_with_timeout(command)
}

/// The proof files of the model
pub fn model_sources() -> Vec<PathBuf> {
	let sources = Path::new(MODEL_DIR).join("WarpTypes");
	let mut files: Vec<PathBuf> = std::fs::read_dir(&sources).map(|entries| entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect()).unwrap_or_default();
	files.retain(|path| path.extension().is_some_and(|extension| extension == "lean"));
	files.sort();
	files
}

/// Warp's own verdict: does it compile the program? An `Err` that is no Error is the constant answer of a program
/// that needs no module, an acceptance
pub fn warp_verdict(code: &str) -> Result<(), String> {
	match crate::pipeline::compile(code) {
		Err(answer) => match answer.drop_meta() {
			Node::Error(why) => Err(why.serialize()),
			_ => Ok(()),
		},
		Ok(_) => Ok(()),
	}
}

/// `warp types <code>`: the program in W0, the model's verdict and warp's
pub fn report(code: &str) -> String {
	let exported = match export(code) {
		Ok(items) => items,
		Err(why) => return why,
	};
	let model = match verdicts(std::slice::from_ref(&exported)) {
		Ok(verdicts) => format!("{:?}", verdicts[0]),
		Err(why) => format!("model failed: {why}"),
	};
	let warp = match warp_verdict(code) {
		Ok(()) => "Accepted".to_string(),
		Err(why) => format!("Rejected: {why}"),
	};
	format!("W0: {exported}\nmodel: {model}\nwarp: {warp}")
}
