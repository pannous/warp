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
pub mod type_model;

use crate::analyzer::extract_user_functions;
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
/// Keep machine-word boundaries in the generated corpus even though Warp Int promotes on overflow.
const EDGE_INTS: [i64; 9] = [0, 1, -1, 2, -2, i64::MAX, i64::MIN, 3_037_000_500, -3_037_000_500];
const EDGE_FLOATS: [f64; 5] = [0.0, 1.0, -1.0, 0.5, -2.5];
/// The wasm trap of a recursion deeper than the stack
const STACK_EXHAUSTED: &str = "call stack exhausted";

#[derive(Clone, Debug, PartialEq)]
pub struct Law {
	pub name: Option<String>,
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
		match &self.name {
			Some(name) => write!(f, "law {name}: {}", self.statement.serialize().trim()),
			None => write!(f, "law {}", self.statement.serialize().trim()),
		}
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

struct LawDeclaration {
	name: Option<String>,
	statement: Node,
	variables: Vec<(String, Kind)>,
}

fn quantified_variable(node: &Node) -> Option<(String, Kind)> {
	let Node::List(parts, _, _) = node.drop_meta() else {
		return None;
	};
	let [for_word, variable, type_block] = parts.as_slice() else {
		return None;
	};
	if !for_word.is_symbol("for") {
		return None;
	}
	let Node::List(types, Bracket::Curly, _) = type_block.drop_meta() else {
		return None;
	};
	let [type_name] = types.as_slice() else { return None };
	Some((variable.name(), kind_of_type_name(&type_name.name())))
}

fn quantified_law(node: &Node) -> Option<LawDeclaration> {
	let Node::Key(name, Op::Colon, body) = node.drop_meta() else {
		return None;
	};
	let items = top_level_items(body);
	let (statement, quantifiers) = items.split_last()?;
	let variables = quantifiers
		.iter()
		.map(quantified_variable)
		.collect::<Option<Vec<_>>>()?;
	(!variables.is_empty()).then(|| LawDeclaration {
		name: Some(name.name()),
		statement: statement.clone(),
		variables,
	})
}

fn law_declaration(node: &Node) -> Option<LawDeclaration> {
	match node.drop_meta() {
		// the word itself: Node's loose == takes `true` for any non-empty symbol, `[true, 2]` is no law
		Node::List(items, _, _)
			if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == LAW_KEYWORD) =>
		{
			quantified_law(&items[1]).or_else(|| {
				Some(LawDeclaration {
					name: None,
					statement: items[1].clone(),
					variables: vec![],
				})
			})
		}
		_ => None,
	}
}

pub(crate) fn top_level_items(node: &Node) -> Vec<Node> {
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
	if !items.iter().any(|item| law_declaration(item).is_some()) {
		return Lawful {
			program: parsed,
			laws: vec![],
			functions: vec![],
		};
	}
	let (law_items, program_items): (Vec<Node>, Vec<Node>) =
		items.into_iter().partition(|item| law_declaration(item).is_some());
	let functions: Vec<FunctionDefinition> = program_items.iter().filter_map(function_definition).collect();
	let laws = law_items
		.iter()
		.filter_map(law_declaration)
		.map(|declaration| make_law(declaration, &functions))
		.collect();
	Lawful {
		program: rebuild_block(program_items, &parsed),
		laws,
		functions,
	}
}

pub fn extract_laws(parsed: &Node) -> Vec<Law> {
	separate_laws(parsed.clone()).laws
}

/// Reuses the analyzer's recognizers, then reads parameter kinds from the declaration.
pub(crate) fn function_definition(item: &Node) -> Option<FunctionDefinition> {
	let mut context = Context::new();
	extract_user_functions(&mut context, item);
	let def = context.user_functions.into_values().next()?;
	let kinds = declared_parameter_kinds(item);
	let parameters = def
		.params
		.iter()
		.map(|param| (param.name.clone(), kinds.get(&param.name).copied().unwrap_or(Kind::Int)))
		.collect();
	Some(FunctionDefinition {
		name: def.name,
		parameters,
		body: *def.body,
		source: item.clone(),
	})
}

fn declared_parameter_kinds(item: &Node) -> HashMap<String, Kind> {
	declared_parameter_types(item)
		.into_iter()
		.map(|(name, type_name)| (name, kind_of_type_name(&type_name)))
		.collect()
}

/// Parameter name → declared type name, for parameters written `name:type`.
pub(crate) fn declared_parameter_types(item: &Node) -> HashMap<String, String> {
	let mut types = HashMap::new();
	if let Node::Key(signature, Op::Assign | Op::Define, _) = item.drop_meta() {
		if let Node::List(params, _, _) = signature.drop_meta() {
			for param in params.iter().skip(1) {
				if let Node::Key(name, Op::Colon, type_name) = param.drop_meta() {
					types.insert(name.name(), type_name.name());
				}
			}
		}
	}
	types
}

fn kind_of_type_name(name: &str) -> Kind {
	crate::analyzer::builtin_type_kind(name)
		.filter(|kind| kind.is_float())
		.unwrap_or(Kind::Int)
}

fn make_law(declaration: LawDeclaration, functions: &[FunctionDefinition]) -> Law {
	let LawDeclaration {
		name,
		statement,
		mut variables,
	} = declaration;
	let known = |name: &str| functions.iter().find(|f| f.name == name);
	let mut function = String::new();
	statement.visit(&mut |node| {
		if let Some((name, args)) = node.call_parts() {
			if let Some(definition) = known(name) {
				if function.is_empty() {
					function = name.to_string();
				}
				for (arg, (_, kind)) in args.iter().zip(&definition.parameters) {
					arg.visit(&mut |inner| {
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
	statement.visit(&mut |node| {
		if let Node::Symbol(var) = node {
			if known(var).is_none() && !variables.iter().any(|(v, _)| v == var) {
				variables.push((var.clone(), Kind::Int));
			}
		}
	});
	Law {
		name,
		function,
		statement,
		variables,
	}
}

pub fn substitute(node: &Node, bindings: &HashMap<String, Node>) -> Node {
	match node {
		Node::Symbol(name) => bindings.get(name).cloned().unwrap_or_else(|| node.clone()),
		Node::Key(left, op, right) => Node::Key(
			Box::new(substitute(left, bindings)),
			*op,
			Box::new(substitute(right, bindings)),
		),
		Node::List(items, bracket, separator) => Node::List(
			items.iter().map(|item| substitute(item, bindings)).collect(),
			bracket.clone(),
			separator.clone(),
		),
		Node::Meta { node: inner, data } => Node::Meta {
			node: Box::new(substitute(inner, bindings)),
			data: data.clone(),
		},
		_ => node.clone(),
	}
}

/// Evaluate one instance of a law through the full parse → wasm → Node machinery. `law a => b` holds where a does not,
/// without running b (`n >= 0 => sum_to(n) …` never calls sum_to of a negative n)
pub(crate) fn check_instance(lawful: &Lawful, law: &Law, bindings: &HashMap<String, Node>, code: &str) -> Verdict {
	let statement = substitute(&law.statement, bindings);
	if let Node::Key(condition, Op::FatArrow | Op::Arrow, conclusion) = statement.drop_meta() {
		return match instance_verdict(lawful, condition, bindings, code) {
			Verdict::Holds => instance_verdict(lawful, conclusion, bindings, code),
			Verdict::Violated(_) => Verdict::Holds,
			unknown => unknown,
		};
	}
	instance_verdict(lawful, &statement, bindings, code)
}

fn instance_verdict(lawful: &Lawful, statement: &Node, bindings: &HashMap<String, Node>, code: &str) -> Verdict {
	let mut items: Vec<Node> = lawful.functions.iter().map(|f| f.source.clone()).collect();
	items.push(statement.clone());
	let instance = Node::List(items, Bracket::None, Separator::Semicolon);
	match eval_parsed(instance, code) {
		Node::True => Verdict::Holds,
		Node::Number(Number::Int(n)) if n != 0 => Verdict::Holds,
		Node::False | Node::Number(Number::Int(0)) => Verdict::Violated(describe(bindings)),
		other => Verdict::Unknown(format!("law instance evaluated to {}", other.serialize().trim())),
	}
}

fn describe(bindings: &HashMap<String, Node>) -> String {
	let mut pairs: Vec<String> = bindings
		.iter()
		.map(|(name, value)| format!("{}={}", name, value.serialize().trim()))
		.collect();
	pairs.sort();
	format!("counterexample {}", pairs.join(" "))
}

// ==================== asserted ====================

fn is_closed(node: &Node, functions: &[FunctionDefinition]) -> bool {
	let mut closed = true;
	node.visit(&mut |inner| {
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
	law.statement.visit(&mut |node| {
		if let Some((name, args)) = node.call_parts() {
			let names: Vec<String> = args
				.iter()
				.filter_map(|a| match a.drop_meta() {
					Node::Symbol(var) if law.variables.iter().any(|(v, _)| v == var) => Some(var.clone()),
					_ => None,
				})
				.collect();
			if !args.is_empty() && names.len() == args.len() {
				patterns.push((name, names));
			}
		}
	});
	let mut bindings = vec![];
	lawful.program.visit(&mut |node| {
		if let Some((name, args)) = node.call_parts() {
			for (pattern, vars) in &patterns {
				let covers_all = law.variables.iter().all(|(v, _)| vars.contains(v));
				if *pattern == name
					&& vars.len() == args.len()
					&& covers_all && args.iter().all(|a| is_closed(a, &lawful.functions))
				{
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
			_ => Node::Number(Number::Int(
				(self.next() % (2 * RANDOM_INT_RANGE as u64 + 1)) as i64 - RANDOM_INT_RANGE,
			)),
		}
	}
}

/// An instance recursing deeper than the wasm stack (`sum_to(i64::MAX)`) is skipped: no verdict, Lean decides
pub fn property_test(lawful: &Lawful, law: &Law, trials: usize, code: &str) -> Verdict {
	let mut generator = Generator(RANDOM_SEED);
	let (mut run, mut too_deep) = (0, 0);
	for trial in 0..trials {
		let bindings: HashMap<String, Node> = law
			.variables
			.iter()
			.map(|(name, kind)| (name.clone(), generator.value(*kind, trial)))
			.collect();
		run += 1;
		match check_instance(lawful, law, &bindings, code) {
			Verdict::Holds => {}
			Verdict::Unknown(why) if why.contains(STACK_EXHAUSTED) => too_deep += 1,
			other => return other,
		}
		if law.variables.is_empty() {
			break;
		}
	}
	match too_deep == run {
		true => Verdict::Unknown(format!("every instance recursed too deep to run ({STACK_EXHAUSTED})")),
		false => Verdict::Holds,
	}
}

// ==================== verify ====================

/// Raise every law as far as it goes: property tests first (they find counterexamples), then Lean, with the laws
/// proved before as lemmas
pub fn verify(code: &str) -> Vec<LawReport> {
	let lawful = separate_laws(crate::warp_parser::WarpParser::parse(code));
	let mut proved: Vec<Law> = vec![];
	let mut reports = vec![];
	for law in &lawful.laws {
		let report = match property_test(&lawful, law, PROPERTY_TRIALS, code) {
			Verdict::Holds => match lean::prove_using(&lawful.functions, law, &proved) {
				Verdict::Holds => LawReport {
					law: law.clone(),
					assurance: Assurance::Proved,
					verdict: Verdict::Holds,
				},
				verdict => LawReport {
					law: law.clone(),
					assurance: Assurance::Tested,
					verdict,
				},
			},
			verdict => LawReport {
				law: law.clone(),
				assurance: Assurance::Stated,
				verdict,
			},
		};
		if report.assurance == Assurance::Proved {
			proved.push(law.clone());
		}
		reports.push(report);
	}
	reports
}
