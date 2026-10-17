//! `f == g` on functions.
//!
//! Extensional equality of functions is undecidable in general (Rice), but decidable in useful fragments.
//! The compiler answers only what it can prove and never guesses:
//! 1. structure up to renaming: bodies with positional parameters (and `$self` for recursion) compare equal,
//!    the content hash Unison names its definitions by: `f(x):=x+1; g(y):=y+1; f==g` → true
//! 2. polynomials over exact numbers: expanded and collected into a sparse normal form:
//!    `(x+1)^2 == x^2+2*x+1` → true, `x*x == x+x` → false
//! 3. finite parameter types (bool): every input is enumerated
//! 4. everything else: `Error("undecidable: f == g …")`, with a counterexample if a quick property test
//!    (src/law.rs) finds one
//!
//! Function values only the run time knows (`xs#1 == xs#2`, `make(1) == make(2)`) compare by rule 1 alone: a closure's
//! kind carries the class of its target's written form (written_form, wasm_emitter/closures.rs function_kind), equal
//! classes compare the captured values; different forms are different functions there (card runtime-function)
use crate::extensions::numbers::Number;
use crate::extensions::reals::Rational;
use crate::law::{self, FunctionDefinition, Law, Lawful, Verdict};
use crate::node::{Bracket, Node};
use crate::operators::Op;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

/// Inputs tried before an undecidable comparison is reported without a counterexample.
const QUICK_TRIALS: usize = 16;
/// Largest exponent expanded into the polynomial normal form.
const MAX_POWER: u32 = 64;
/// Largest normal form built before giving up on the polynomial fragment.
const MAX_TERMS: usize = 4096;
/// Most bool parameters enumerated (2^10 inputs).
const MAX_ENUMERATED_PARAMETERS: usize = 10;
const SELF: &str = "$self";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Domain {
	Bool,
	Exact,
	Float,
	Other(String),
}

fn domain(type_name: Option<&String>) -> Domain {
	let Some(name) = type_name else { return Domain::Exact };
	match crate::type_tests::canonical_spec_word(name) {
		crate::analyzer::BOOL_TYPE => Domain::Bool,
		"float" => Domain::Float,
		exact if crate::type_tests::type_matches("int", exact) => Domain::Exact,
		other => Domain::Other(other.to_string()),
	}
}

struct Function {
	definition: FunctionDefinition,
	domains: Vec<Domain>,
}

impl Function {
	fn name(&self) -> &str {
		&self.definition.name
	}

	fn arity(&self) -> usize {
		self.definition.parameters.len()
	}

	fn parameter_names(&self) -> Vec<String> {
		self.definition.parameters.iter().map(|(name, _)| name.clone()).collect()
	}

	fn normal_body(&self) -> Node {
		normal_body(self.name(), &self.parameter_names(), &self.definition.body)
	}

	fn canonical_form(&self) -> String {
		format!("{:?} {:?}", self.domains, self.normal_body())
	}

	/// Content hash as in Unison: independent of the function's name and its parameter names.
	fn content_hash(&self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.canonical_form().hash(&mut hasher);
		hasher.finish()
	}
}

/// The body with parameters renamed to their positions and self-reference to `$self`.
fn normal_body(name: &str, parameters: &[String], body: &Node) -> Node {
	let mut names: HashMap<String, String> =
		parameters.iter().enumerate().map(|(i, parameter)| (parameter.clone(), format!("${i}"))).collect();
	names.insert(name.to_string(), SELF.to_string());
	canonical(body, &names)
}

/// A definition as dump shows it, without its own name and its parameters' names: the same text for two functions
/// written alike (`x => x + 1`, `y => y + 1`), what the run time compares function values by (card runtime-function)
pub fn written_form(name: &str, parameters: &[(String, Option<Node>)], body: &Node) -> String {
	let names: Vec<String> = parameters.iter().map(|(name, _)| name.clone()).collect();
	let types: Vec<String> =
		parameters.iter().map(|(_, annotation)| annotation.as_ref().map(Node::serialize).unwrap_or_default()).collect();
	format!("{types:?} {:?}", normal_body(name, &names, body))
}

fn canonical(node: &Node, names: &HashMap<String, String>) -> Node {
	match node.drop_meta() {
		Node::Symbol(name) => Node::Symbol(names.get(name).cloned().unwrap_or_else(|| name.clone())),
		Node::List(items, Bracket::Round | Bracket::Curly | Bracket::None, _) if items.len() == 1 => {
			canonical(&items[0], names)
		}
		Node::List(items, bracket, separator) => {
			Node::List(items.iter().map(|item| canonical(item, names)).collect(), bracket.clone(), separator.clone())
		}
		Node::Key(left, op, right) => {
			Node::Key(Box::new(canonical(left, names)), *op, Box::new(canonical(right, names)))
		}
		other => other.clone(),
	}
}

/// Replace every comparison of two named functions by its decided value.
/// An undecidable comparison makes the whole program its error.
pub fn decide_comparisons(node: Node) -> Node {
	let compared = compared_names(&node);
	if compared.is_empty() {
		return node;
	}
	// analysing every definition is costly (a fresh analysis each): first only those naming a compared word
	let words: Vec<&str> = compared.iter().flat_map(|(left, right)| [left.as_str(), right.as_str()]).collect();
	let candidates = collect_functions_where(&node, |item| item.mentions_any(&words));
	let is_function = |name: &str| candidates.iter().any(|function| function.name() == name);
	if !compared.iter().any(|(left, right)| is_function(left) && is_function(right)) {
		return node;
	}
	let functions = collect_functions_where(&node, |_| true);
	let mut failure = None;
	let decided = rewrite(node, &functions, &[], &mut failure);
	failure.unwrap_or(decided)
}

/// The names compared in `a == b` and `a != b`
fn compared_names(node: &Node) -> Vec<(String, String)> {
	let mut compared = vec![];
	node.visit(&mut |inner| {
		if let Node::Key(left, Op::Eq | Op::Ne, right) = inner {
			if let (Node::Symbol(left), Node::Symbol(right)) = (left.drop_meta(), right.drop_meta()) {
				compared.push((left.clone(), right.clone()));
			}
		}
	});
	compared
}

/// The functions defined by the top-level items `wanted` picks
fn collect_functions_where(node: &Node, wanted: impl Fn(&Node) -> bool) -> Vec<Function> {
	law::top_level_items(node)
		.iter()
		.filter(|item| wanted(item))
		.filter_map(|item| {
			let definition = law::function_definition(item)?;
			let types = law::declared_parameter_types(item);
			let domains = definition.parameters.iter().map(|(name, _)| domain(types.get(name))).collect();
			Some(Function { definition, domains })
		})
		.collect()
}

fn named<'a>(node: &Node, functions: &'a [Function], shadowed: &[String]) -> Option<&'a Function> {
	match node.drop_meta() {
		Node::Symbol(name) if !shadowed.contains(name) => {
			functions.iter().find(|function| function.name() == name.as_str())
		}
		_ => None,
	}
}

/// The names a function definition's parameters shadow in its body (`f(name) := name == text`), with the outer ones
fn shadowed_within(node: &Node, shadowed: &[String]) -> Option<Vec<String>> {
	if !matches!(node.drop_meta(), Node::Key(_, Op::Define | Op::Assign, _)) {
		return None;
	}
	let definition = law::function_definition(node)?;
	Some(shadowed.iter().cloned().chain(definition.parameters.into_iter().map(|(name, _)| name)).collect())
}

fn rewrite(node: Node, functions: &[Function], shadowed: &[String], failure: &mut Option<Node>) -> Node {
	let inner_shadowed = shadowed_within(&node, shadowed);
	let shadowed = inner_shadowed.as_deref().unwrap_or(shadowed);
	match node {
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) => {
			if let (Some(f), Some(g)) = (named(&left, functions, shadowed), named(&right, functions, shadowed)) {
				return match decide(f, g, functions) {
					Ok(equal) => {
						if equal == matches!(op, Op::Eq) {
							Node::True
						} else {
							Node::False
						}
					}
					Err(error) => {
						failure.get_or_insert_with(|| error.clone());
						error
					}
				};
			}
			Node::Key(
				Box::new(rewrite(*left, functions, shadowed, failure)),
				op,
				Box::new(rewrite(*right, functions, shadowed, failure)),
			)
		}
		Node::Key(left, op, right) => Node::Key(
			Box::new(rewrite(*left, functions, shadowed, failure)),
			op,
			Box::new(rewrite(*right, functions, shadowed, failure)),
		),
		Node::List(items, bracket, separator) => Node::List(
			items.into_iter().map(|item| rewrite(item, functions, shadowed, failure)).collect(),
			bracket,
			separator,
		),
		Node::Meta { node: inner, data } => {
			Node::Meta { node: Box::new(rewrite(*inner, functions, shadowed, failure)), data }
		}
		other => other,
	}
}

/// `Ok` only when proven; `Err` is the undecidable error.
fn decide(f: &Function, g: &Function, functions: &[Function]) -> Result<bool, Node> {
	if f.name() == g.name() {
		return Ok(true);
	}
	// Functions of different arity or parameter types have different domains.
	if f.arity() != g.arity() || f.domains != g.domains {
		return Ok(false);
	}
	if f.content_hash() == g.content_hash() && f.canonical_form() == g.canonical_form() {
		return Ok(true);
	}
	if f.domains.iter().all(|domain| *domain == Domain::Exact) {
		let names = f.parameter_names();
		let body_g = rename_parameters(g, &names);
		if let (Some(p), Some(q)) = (polynomial(&f.definition.body, &names), polynomial(&body_g, &names)) {
			return Ok(p == q);
		}
	}
	if f.arity() > 0 && f.domains.iter().all(|domain| *domain == Domain::Bool) && f.arity() <= MAX_ENUMERATED_PARAMETERS
	{
		return enumerate(f, g, functions);
	}
	Err(undecidable(f, g, functions))
}

fn rename_parameters(g: &Function, names: &[String]) -> Node {
	let renaming: HashMap<String, Node> =
		g.parameter_names().into_iter().zip(names.iter().map(|name| Node::Symbol(name.clone()))).collect();
	law::substitute(&g.definition.body, &renaming)
}

fn comparison_law(f: &Function, g: &Function) -> Law {
	let mut names = f.parameter_names();
	for (i, name) in names.iter_mut().enumerate() {
		if name == f.name() || name == g.name() {
			*name = format!("p{i}");
		}
	}
	let arguments = names.join(", ");
	let statement =
		crate::warp_parser::parse_generated(&format!("{}({arguments}) == {}({arguments})", f.name(), g.name()));
	let variables = names.into_iter().zip(f.definition.parameters.iter().map(|(_, kind)| *kind)).collect();
	Law { name: None, function: f.name().to_string(), statement, variables }
}

fn lawful(functions: &[Function]) -> Lawful {
	Lawful {
		program: Node::Empty,
		laws: vec![],
		functions: functions.iter().map(|function| function.definition.clone()).collect(),
	}
}

fn enumerate(f: &Function, g: &Function, functions: &[Function]) -> Result<bool, Node> {
	let law = comparison_law(f, g);
	let lawful = lawful(functions);
	for bits in 0..(1usize << f.arity()) {
		let bindings: HashMap<String, Node> = law
			.variables
			.iter()
			.enumerate()
			.map(|(i, (name, _))| (name.clone(), if (bits >> i) & 1 == 1 { Node::True } else { Node::False }))
			.collect();
		match law::check_instance(&lawful, &law, &bindings, "") {
			Verdict::Holds => {}
			Verdict::Violated(_) => return Ok(false),
			Verdict::Unknown(why) => {
				return Err(crate::node::error(&format!("undecidable: {} == {}: {why}", f.name(), g.name())))
			}
		}
	}
	Ok(true)
}

fn undecidable(f: &Function, g: &Function, functions: &[Function]) -> Node {
	let head = format!(
		"undecidable: {} == {} (not equal up to renaming, not polynomials over exact numbers, not over a finite domain)",
		f.name(),
		g.name()
	);
	let testable = f.arity() > 0 && f.domains.iter().all(|domain| matches!(domain, Domain::Exact | Domain::Float));
	if !testable {
		return crate::node::error(&head);
	}
	let law = comparison_law(f, g);
	let message = match law::property_test(&lawful(functions), &law, QUICK_TRIALS, "") {
		Verdict::Violated(counterexample) => format!("{head}; they differ: {counterexample}"),
		Verdict::Holds => format!("{head}; {QUICK_TRIALS} generated inputs agree, state a law to test more"),
		Verdict::Unknown(why) => format!("{head}; {why}"),
	};
	crate::node::error(&message)
}

// ==================== polynomial normal form ====================

/// The exact value of a number: none for floats (not exact), NaN and infinities (not in a ring)
fn exact_ratio(number: &Number) -> Option<Rational> {
	match number {
		Number::Quotient(_, 0) => None,
		Number::Int(_) | Number::Quotient(..) | Number::BigQuotient(_) | Number::BigInt(_) => {
			Some(number.to_rational())
		}
		_ => None,
	}
}

/// A whole exponent small enough to expand
fn small_exponent(ratio: &Rational) -> Option<u32> {
	if !ratio.is_integer() {
		return None;
	}
	ratio.numerator.to_u32().filter(|power| *power <= MAX_POWER)
}

/// Sparse polynomial: exponent vector (one entry per parameter) → nonzero coefficient.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Polynomial {
	terms: BTreeMap<Vec<u32>, Rational>,
	arity: usize,
}

impl Polynomial {
	fn constant(value: Rational, arity: usize) -> Polynomial {
		let mut terms = BTreeMap::new();
		if !value.is_zero() {
			terms.insert(vec![0; arity], value);
		}
		Polynomial { terms, arity }
	}

	fn variable(index: usize, arity: usize) -> Polynomial {
		let mut exponents = vec![0; arity];
		exponents[index] = 1;
		Polynomial { terms: BTreeMap::from([(exponents, Rational::integer(BigInt::one()))]), arity }
	}

	fn as_constant(&self) -> Option<Rational> {
		match self.terms.len() {
			0 => Some(Rational::integer(BigInt::zero())),
			1 => self.terms.get(&vec![0; self.arity]).cloned(),
			_ => None,
		}
	}

	fn add_term(terms: &mut BTreeMap<Vec<u32>, Rational>, exponents: Vec<u32>, coefficient: Rational) {
		let sum = match terms.get(&exponents) {
			Some(existing) => existing.add(&coefficient),
			None => coefficient,
		};
		if sum.is_zero() {
			terms.remove(&exponents);
		} else {
			terms.insert(exponents, sum);
		}
	}

	fn add(&self, other: &Polynomial) -> Polynomial {
		let mut terms = self.terms.clone();
		for (exponents, coefficient) in &other.terms {
			Self::add_term(&mut terms, exponents.clone(), coefficient.clone());
		}
		Polynomial { terms, arity: self.arity }
	}

	fn scale(&self, factor: &Rational) -> Polynomial {
		if factor.is_zero() {
			return Polynomial::constant(factor.clone(), self.arity);
		}
		let terms =
			self.terms.iter().map(|(exponents, coefficient)| (exponents.clone(), coefficient.mul(factor))).collect();
		Polynomial { terms, arity: self.arity }
	}

	fn neg(&self) -> Polynomial {
		self.scale(&Rational::integer(-BigInt::one()))
	}

	fn mul(&self, other: &Polynomial) -> Option<Polynomial> {
		let mut terms = BTreeMap::new();
		for (a, x) in &self.terms {
			for (b, y) in &other.terms {
				let exponents = a.iter().zip(b).map(|(i, j)| i + j).collect();
				Self::add_term(&mut terms, exponents, x.mul(y));
			}
			if terms.len() > MAX_TERMS {
				return None;
			}
		}
		Some(Polynomial { terms, arity: self.arity })
	}

	fn pow(&self, power: u32) -> Option<Polynomial> {
		let mut result = Polynomial::constant(Rational::integer(BigInt::one()), self.arity);
		for _ in 0..power {
			result = result.mul(self)?;
		}
		Some(result)
	}
}

/// The normal form of an arithmetic body over the given parameters, None outside the polynomial fragment.
fn polynomial(node: &Node, parameters: &[String]) -> Option<Polynomial> {
	let arity = parameters.len();
	let of = |inner: &Node| polynomial(inner, parameters);
	match node.drop_meta() {
		Node::Number(number) => Some(Polynomial::constant(exact_ratio(number)?, arity)),
		Node::Symbol(name) => Some(Polynomial::variable(parameters.iter().position(|p| p == name)?, arity)),
		Node::List(items, Bracket::Round | Bracket::Curly | Bracket::None, _) if items.len() == 1 => of(&items[0]),
		Node::Key(left, op, right) => match (left.drop_meta(), op, right.drop_meta()) {
			(Node::Empty, Op::Neg, _) => Some(of(right)?.neg()),
			(_, Op::Square, Node::Empty) => of(left)?.pow(2),
			(_, Op::Cube, Node::Empty) => of(left)?.pow(3),
			_ => {
				let (a, b) = (of(left)?, of(right)?);
				match op {
					Op::Add => Some(a.add(&b)),
					Op::Sub => Some(a.add(&b.neg())),
					Op::Mul => a.mul(&b),
					Op::Div => Some(a.scale(&b.as_constant()?.inverse()?)),
					Op::Pow => a.pow(small_exponent(&b.as_constant()?)?),
					_ => None,
				}
			}
		},
		_ => None,
	}
}
