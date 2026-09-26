//! Progressive verification: `law` declarations.
//!
//! ```text
//! square(x) := x*x
//! law square(-x) == square(x)
//! ```
//!
//! Assurance rises without changing the source:
//! stated → asserted (debug builds, at concrete call sites) → tested (generated inputs) → proved (Lean).
//! Laws will later attach to the semantic FunctionDecl; until that IR exists they live beside the Node AST.
pub mod lean;

use crate::analyzer::{collect_functions, extract_user_functions};
use crate::context::Context;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::wasm_emitter::eval_parsed;
use std::collections::HashMap;
use std::fmt;

pub const LAW_KEYWORD: &str = "law";
pub const PROPERTY_TRIALS: usize = 64;
const RANDOM_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const RANDOM_INT_RANGE: i64 = 1000;
/// 3037000500² is the first square beyond i64::MAX: Warp Int wraps, so laws must survive overflow.
const EDGE_INTS: [i64; 9] = [0, 1, -1, 2, -2, i64::MAX, i64::MIN, 3_037_000_500, -3_037_000_500];
const EDGE_FLOATS: [f64; 5] = [0.0, 1.0, -1.0, 0.5, -2.5];

#[derive(Clone, Debug, PartialEq)]
pub struct Law {
	pub function: String,
	pub statement: Node,
	pub variables: Vec<(String, Kind)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Assurance {
	Stated,
	Asserted,
	Tested,
	Proved,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
	Holds,
	Violated(String),
	Unknown(String),
}

#[derive(Clone, Debug)]
pub struct LawReport {
	pub law: Law,
	pub assurance: Assurance,
	pub verdict: Verdict,
}

impl LawReport {
	pub fn failed(&self) -> bool {
		matches!(self.verdict, Verdict::Violated(_))
	}
}

impl fmt::Display for Law {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "law {}", self.statement.serialize().trim())
	}
}

impl fmt::Display for LawReport {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self.verdict {
			Verdict::Holds => write!(f, "{:?}  {}", self.assurance, self.law),
			Verdict::Violated(why) => write!(f, "FAILED  {}  {}", self.law, why),
			Verdict::Unknown(why) => write!(f, "{:?}  {}  ({})", self.assurance, self.law, why),
		}
	}
}

/// A program split into its executable part and its laws.
pub struct Lawful {
	pub program: Node,
	pub laws: Vec<Law>,
	pub functions: Vec<FunctionDefinition>,
}

/// A top-level function definition with parameter kinds, the shape laws and exports need.
#[derive(Clone, Debug)]
pub struct FunctionDefinition {
	pub name: String,
	pub parameters: Vec<(String, Kind)>,
	pub body: Node,
	pub source: Node,
}

fn law_statement(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && items[0].drop_meta() == &Node::Symbol(LAW_KEYWORD.into()) => {
			Some(&items[1])
		}
		_ => None,
	}
}

fn top_level_items(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => vec![node.clone()],
	}
}

fn rebuild_block(items: Vec<Node>, like: &Node) -> Node {
	match (items.len(), like.drop_meta()) {
		(0, _) => Node::Empty,
		(1, _) => items.into_iter().next().unwrap(),
		(_, Node::List(_, bracket, separator)) => Node::List(items, bracket.clone(), separator.clone()),
		_ => Node::List(items, Bracket::None, Separator::Semicolon),
	}
}

/// Split laws from the program. Without laws the parsed node is returned untouched.
pub fn separate_laws(parsed: Node) -> Lawful {
	let items = top_level_items(&parsed);
	if !items.iter().any(|item| law_statement(item).is_some()) {
		return Lawful { program: parsed, laws: vec![], functions: vec![] };
	}
	let (law_items, program_items): (Vec<Node>, Vec<Node>) =
		items.into_iter().partition(|item| law_statement(item).is_some());
	let functions: Vec<FunctionDefinition> = program_items.iter().filter_map(function_definition).collect();
	let laws = law_items
		.iter()
		.filter_map(law_statement)
		.map(|statement| make_law(statement, &functions))
		.collect();
	Lawful { program: rebuild_block(program_items, &parsed), laws, functions }
}

pub fn extract_laws(parsed: &Node) -> Vec<Law> {
	separate_laws(parsed.clone()).laws
}

/// Reuses the analyzer's recognizers, then reads parameter kinds from the declaration.
fn function_definition(item: &Node) -> Option<FunctionDefinition> {
	let mut context = Context::new();
	extract_user_functions(&mut context, item);
	if let Some(def) = context.user_functions.into_values().next() {
		let kinds = declared_parameter_kinds(item);
		let parameters = def
			.params
			.iter()
			.map(|(name, _)| (name.clone(), kinds.get(name).copied().unwrap_or(Kind::Int)))
			.collect();
		return Some(FunctionDefinition { name: def.name, parameters, body: *def.body, source: item.clone() });
	}
	let registry = collect_functions(item);
	let function = registry.all().first()?;
	Some(FunctionDefinition {
		name: function.name.clone(),
		parameters: function.signature.parameters.iter().map(|arg| (arg.name.clone(), arg.kind)).collect(),
		body: function.body.as_deref().cloned().unwrap_or(Node::Empty),
		source: item.clone(),
	})
}

fn declared_parameter_kinds(item: &Node) -> HashMap<String, Kind> {
	let mut kinds = HashMap::new();
	if let Node::Key(signature, Op::Assign | Op::Define, _) = item.drop_meta() {
		if let Node::List(params, _, _) = signature.drop_meta() {
			for param in params.iter().skip(1) {
				if let Node::Key(name, Op::Colon, type_name) = param.drop_meta() {
					kinds.insert(name.name(), kind_of_type_name(&type_name.name()));
				}
			}
		}
	}
	kinds
}

fn kind_of_type_name(name: &str) -> Kind {
	match name.to_lowercase().as_str() {
		"float" | "f32" | "f64" | "double" | "real" | "number" => Kind::Float,
		_ => Kind::Int,
	}
}

fn call_parts(node: &Node) -> Option<(&str, &[Node])> {
	match node.drop_meta() {
		Node::List(items, _, _) if !items.is_empty() => match items[0].drop_meta() {
			Node::Symbol(name) => Some((name.as_str(), &items[1..])),
			_ => None,
		},
		_ => None,
	}
}

fn visit<'a>(node: &'a Node, action: &mut dyn FnMut(&'a Node)) {
	let node = node.drop_meta();
	action(node);
	match node {
		Node::Key(left, _, right) => {
			visit(left, action);
			visit(right, action);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| visit(item, action)),
		_ => {}
	}
}

fn make_law(statement: &Node, functions: &[FunctionDefinition]) -> Law {
	let known = |name: &str| functions.iter().find(|f| f.name == name);
	let mut function = String::new();
	let mut variables: Vec<(String, Kind)> = vec![];
	visit(statement, &mut |node| {
		if let Some((name, args)) = call_parts(node) {
			if let Some(definition) = known(name) {
				if function.is_empty() {
					function = name.to_string();
				}
				for (arg, (_, kind)) in args.iter().zip(&definition.parameters) {
					visit(arg, &mut |inner| {
						if let Node::Symbol(var) = inner {
							if known(var).is_none() && !variables.iter().any(|(v, _)| v == var) {
								variables.push((var.clone(), *kind));
							}
						}
					});
				}
			}
		}
	});
	visit(statement, &mut |node| {
		if let Node::Symbol(var) = node {
			if known(var).is_none() && !variables.iter().any(|(v, _)| v == var) {
				variables.push((var.clone(), Kind::Int));
			}
		}
	});
	Law { function, statement: statement.clone(), variables }
}

pub fn substitute(node: &Node, bindings: &HashMap<String, Node>) -> Node {
	match node {
		Node::Symbol(name) => bindings.get(name).cloned().unwrap_or_else(|| node.clone()),
		Node::Key(left, op, right) => {
			Node::Key(Box::new(substitute(left, bindings)), *op, Box::new(substitute(right, bindings)))
		}
		Node::List(items, bracket, separator) => {
			Node::List(items.iter().map(|item| substitute(item, bindings)).collect(), bracket.clone(), separator.clone())
		}
		Node::Meta { node: inner, data } => Node::Meta { node: Box::new(substitute(inner, bindings)), data: data.clone() },
		_ => node.clone(),
	}
}

/// Evaluate one instance of a law through the full parse → wasm → Node machinery.
fn check_instance(lawful: &Lawful, law: &Law, bindings: &HashMap<String, Node>, code: &str) -> Verdict {
	let mut items: Vec<Node> = lawful.functions.iter().map(|f| f.source.clone()).collect();
	items.push(substitute(&law.statement, bindings));
	let instance = Node::List(items, Bracket::None, Separator::Semicolon);
	match eval_parsed(instance, code) {
		Node::True => Verdict::Holds,
		Node::Number(Number::Int(n)) if n != 0 => Verdict::Holds,
		Node::False | Node::Number(Number::Int(0)) => Verdict::Violated(describe(bindings)),
		other => Verdict::Unknown(format!("law instance evaluated to {}", other.serialize().trim())),
	}
}

fn describe(bindings: &HashMap<String, Node>) -> String {
	let mut pairs: Vec<String> =
		bindings.iter().map(|(name, value)| format!("{}={}", name, value.serialize().trim())).collect();
	pairs.sort();
	format!("counterexample {}", pairs.join(" "))
}

// ==================== asserted ====================

fn is_closed(node: &Node, functions: &[FunctionDefinition]) -> bool {
	let mut closed = true;
	visit(node, &mut |inner| {
		if let Node::Symbol(name) = inner {
			if !functions.iter().any(|f| &f.name == name) {
				closed = false;
			}
		}
	});
	closed
}

/// Bind law variables by matching law calls `f(x, y)` against concrete program calls `f(3, 4)`.
fn observed_bindings(lawful: &Lawful, law: &Law) -> Vec<HashMap<String, Node>> {
	let mut patterns: Vec<(&str, Vec<String>)> = vec![];
	visit(&law.statement, &mut |node| {
		if let Some((name, args)) = call_parts(node) {
			let names: Vec<String> = args.iter().filter_map(|a| match a.drop_meta() {
				Node::Symbol(var) if law.variables.iter().any(|(v, _)| v == var) => Some(var.clone()),
				_ => None,
			}).collect();
			if !args.is_empty() && names.len() == args.len() {
				patterns.push((name, names));
			}
		}
	});
	let mut bindings = vec![];
	visit(&lawful.program, &mut |node| {
		if let Some((name, args)) = call_parts(node) {
			for (pattern, vars) in &patterns {
				let covers_all = law.variables.iter().all(|(v, _)| vars.contains(v));
				if *pattern == name && vars.len() == args.len() && covers_all && args.iter().all(|a| is_closed(a, &lawful.functions)) {
					bindings.push(vars.iter().cloned().zip(args.iter().cloned()).collect());
				}
			}
		}
	});
	bindings
}

/// Debug builds check every law at the concrete call sites of the program; a violation replaces the result.
pub fn assert_laws(lawful: &Lawful, code: &str) -> Option<Node> {
	if !cfg!(debug_assertions) {
		return None;
	}
	for law in &lawful.laws {
		for bindings in observed_bindings(lawful, law) {
			if let Verdict::Violated(why) = check_instance(lawful, law, &bindings, code) {
				return Some(Node::Error(Box::new(Node::Text(format!("{} violated: {}", law, why)))));
			}
		}
	}
	None
}

// ==================== tested ====================

/// Deterministic xorshift so failures reproduce without a rand dependency.
struct Generator(u64);

impl Generator {
	fn next(&mut self) -> u64 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		self.0
	}

	fn value(&mut self, kind: Kind, trial: usize) -> Node {
		let pick = |edges: usize| trial < edges;
		match kind {
			Kind::Float if pick(EDGE_FLOATS.len()) => Node::Number(Number::Float(EDGE_FLOATS[trial])),
			Kind::Float => {
				let unit = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
				Node::Number(Number::Float((unit * 2.0 - 1.0) * RANDOM_INT_RANGE as f64))
			}
			_ if pick(EDGE_INTS.len()) => Node::Number(Number::Int(EDGE_INTS[trial])),
			_ => Node::Number(Number::Int((self.next() % (2 * RANDOM_INT_RANGE as u64 + 1)) as i64 - RANDOM_INT_RANGE)),
		}
	}
}

pub fn property_test(lawful: &Lawful, law: &Law, trials: usize, code: &str) -> Verdict {
	let mut generator = Generator(RANDOM_SEED);
	for trial in 0..trials {
		let bindings: HashMap<String, Node> =
			law.variables.iter().map(|(name, kind)| (name.clone(), generator.value(*kind, trial))).collect();
		match check_instance(lawful, law, &bindings, code) {
			Verdict::Holds => {}
			other => return other,
		}
		if law.variables.is_empty() {
			break;
		}
	}
	Verdict::Holds
}

// ==================== verify ====================

/// Raise every law as far as it goes: property tests first (they find counterexamples), then Lean.
pub fn verify(code: &str) -> Vec<LawReport> {
	let lawful = separate_laws(crate::wasp_parser::WaspParser::parse(code));
	lawful
		.laws
		.iter()
		.map(|law| match property_test(&lawful, law, PROPERTY_TRIALS, code) {
			Verdict::Holds => match lean::prove(&lawful.functions, law) {
				Verdict::Holds => LawReport { law: law.clone(), assurance: Assurance::Proved, verdict: Verdict::Holds },
				verdict => LawReport { law: law.clone(), assurance: Assurance::Tested, verdict },
			},
			verdict => LawReport { law: law.clone(), assurance: Assurance::Stated, verdict },
		})
		.collect()
}
