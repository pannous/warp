//! Definitions by cases (card definition-cases, user decision definition-cases): `fib(0) := 0; fib(n) := …` is not
//! joined into one function. A second definition of the same shape in one block (same name, same parameter count, same
//! parameter types) is the error "fib is defined twice", whose fix for cases is the one definition with a match; so is a
//! literal where a parameter's name belongs (`f(0) := 1`). Overloads by parameter count stay (test_arity_overloads.rs);
//! definitions with a typed parameter are left to overload resolution, which also tells them apart by the result type
//! of their bodies (test_return_type_dispatch.rs).

use super::nodes::is_call_head;
use crate::diagnostic::{written_statement, Diagnostic};
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};

/// The subject a fix matches on when no definition names its parameter
const SUBJECT: &str = "n";
/// The arm a fix adds for the values no case names
const OTHER_ARM: &str = "_: …";

pub fn lower(program: Node) -> Node {
	first_problem(&program).unwrap_or(program)
}

/// A definition with a call head `f(…)`: its name, parameters and body, borrowed from the statement
struct Definition<'a> {
	name: String,
	parameters: Vec<&'a Node>,
	body: &'a Node,
	statement: &'a Node,
}

impl Definition<'_> {
	/// The name and parameter count: what overloads of untyped definitions differ by
	fn shape(&self) -> (&str, usize) {
		(&self.name, self.parameters.len())
	}

	fn is_typed(&self) -> bool {
		self.parameters
			.iter()
			.any(|parameter| matches!(parameter.drop_meta(), Node::Key(_, Op::Colon, _)))
	}

	fn literal_parameter(&self) -> Option<&Node> {
		self.parameters.iter().copied().find(|parameter| is_literal(parameter))
	}
}

fn first_problem(node: &Node) -> Option<Node> {
	if let Some(definition) = definition(node) {
		if let Some(literal) = definition.literal_parameter() {
			let message = format!(
				"a parameter of {} is a name, not {}",
				definition.name,
				literal.serialize()
			);
			return Some(Diagnostic::at(node, message).fix(joined(&[&definition])).into_error());
		}
	}
	if let Node::List(items, Bracket::None | Bracket::Curly, separator) = node.drop_meta() {
		if let Some(twice) = separator.separates_statements().then(|| defined_twice(items)).flatten() {
			return Some(twice);
		}
	}
	node.children().into_iter().find_map(|child| first_problem(&child))
}

/// The second definition of a shape among the statements of one block, as the error: definitions by cases are fixed by
/// one definition with a match, other ones by another name
fn defined_twice(statements: &[Node]) -> Option<Node> {
	let definitions: Vec<Definition> = statements
		.iter()
		.filter_map(definition)
		.filter(|definition| !definition.is_typed())
		.collect();
	let (_, later) = definitions.iter().enumerate().find(|(index, later)| {
		definitions[..*index]
			.iter()
			.any(|earlier| earlier.shape() == later.shape())
	})?;
	let cases: Vec<&Definition> = definitions
		.iter()
		.filter(|other| other.shape() == later.shape())
		.collect();
	let diagnostic = Diagnostic::at(later.statement, format!("{} is defined twice", later.name));
	Some(
		if cases.iter().any(|case| case.literal_parameter().is_some()) {
			diagnostic
				.message(format!("{} is defined twice; join its cases in one match", later.name))
				.fix(joined(&cases))
		} else {
			diagnostic.message(format!(
				"{} is defined twice; give one of them another name",
				later.name
			))
		}
		.into_error(),
	)
}

/// `f(x) := body`, `f(x) = body`, `fun f(x): body` and `def f(x) { body }`
fn definition(statement: &Node) -> Option<Definition<'_>> {
	let (head, body, grouped) = match statement.drop_meta() {
		Node::Key(head, Op::Define | Op::Assign, body) => (head.as_ref(), body.as_ref(), false),
		Node::List(items, _, _) if items.len() == 2 && is_function_keyword(&items[0].drop_meta().name()) => {
			match items[1].drop_meta() {
				Node::Key(head, Op::Colon | Op::Define, body) => (head.as_ref(), body.as_ref(), false),
				Node::List(parts, Bracket::Round, _) if parts.len() == 2 => (&parts[0], &parts[1], true),
				_ => return None,
			}
		}
		_ => return None,
	};
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else {
		return None;
	};
	if !is_call_head(head) {
		return None;
	}
	Some(Definition {
		name: items[0].name(),
		parameters: items[1..]
			.iter()
			.flat_map(|parameter| listed_parameters(parameter, grouped))
			.collect(),
		body,
		statement,
	})
}

/// `a, b` written as one comma list in the head, or the parameter group `(a, b)` of `def f(a, b) {…}`: its items
fn listed_parameters(parameter: &Node, grouped: bool) -> Vec<&Node> {
	match parameter.drop_meta() {
		Node::List(items, Bracket::None, Separator::Colon) => items.iter().collect(),
		Node::List(items, Bracket::Round, _) if grouped => {
			items.iter().flat_map(|item| listed_parameters(item, false)).collect()
		}
		// `f()`: no parameter
		Node::Empty => vec![],
		_ => vec![parameter],
	}
}

fn is_literal(parameter: &Node) -> bool {
	matches!(
		parameter.drop_meta(),
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty
	)
}

/// The one definition the cases make: `fib(n) := match n { 0: 0; _: … }`; with several parameters, the advice only
fn joined(cases: &[&Definition]) -> String {
	let first = cases[0];
	if first.parameters.len() != 1 {
		return format!("one definition of {} with a match on its parameters", first.name);
	}
	let general = cases.iter().find(|case| !is_literal(case.parameters[0]));
	let subject = general.map_or(SUBJECT.to_string(), |case| case.parameters[0].name());
	let mut arms: Vec<String> = cases
		.iter()
		.map(|case| {
			let parameter = case.parameters[0];
			let pattern = if is_literal(parameter) {
				parameter.serialize()
			} else {
				"_".into()
			};
			let body = written_statement(case.body).unwrap_or_else(|| case.body.serialize());
			format!("{pattern}: {body}")
		})
		.collect();
	if general.is_none() {
		arms.push(OTHER_ARM.into());
	}
	format!("{}({subject}) := match {subject} {{ {} }}", first.name, arms.join("; "))
}
