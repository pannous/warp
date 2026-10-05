//! Blocks that can never run (notes/runtime_eval.md step 0, user 2026-10-05: "give a compiler warning if the code
//! contains unresolved symbol arithmetic"): `x : a+b` where the program defines a and b nowhere. Names inside a block
//! resolve where `!` runs it (wiki/charged.md §5), so a name that appears nowhere else in the program can never resolve:
//! the block is a typo, or symbolic data that wants `data a+b`.
//! A block known only at run time runs through the host (run_block, notes/runtime_eval.md step 2).

use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const UNRESOLVED_TOPIC: &str = "unresolved-block";
/// The spec's word for `x!!`: run the block fully
const INTERPRET: &str = "interpret";

pub fn warn_unresolved(program: Node) -> Node {
	let mut warnings: Vec<Ask> = vec![];
	visit_statements(&program, &mut |statement| {
		let Some((name, block)) = crate::blocks::uncharged(statement) else { return };
		let unresolved: Vec<String> = arithmetic_operands(&block).into_iter()
			.filter(|operand| occurrences(&program, operand) == occurrences(&block, operand))
			.collect();
		if !unresolved.is_empty() {
			warnings.push(unresolved_warning(&name, &block, &unresolved).at_node(statement));
		}
	});
	for warning in warnings {
		if let Err(error) = ask(&warning) {
			return error;
		}
	}
	program
}

fn unresolved_warning(name: &str, block: &Node, unresolved: &[String]) -> Ask {
	let written = crate::normalize::operand_text(block);
	let names = unresolved.join(", ");
	let question = format!("{name} keeps {written} over {names}, defined nowhere in the program: {name}! can never run it");
	let readings = vec![reading("keeping the block", &format!("{name} = data {written}"))];
	Ask::new(UNRESOLVED_TOPIC, question, readings, Fallback::Warning).written(&format!("{name} : {written}"))
}

/// Every statement of the program and of its nested statement lists
fn visit_statements(program: &Node, visit: &mut dyn FnMut(&Node)) {
	match program.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.iter().for_each(|item| visit_statements(item, visit)),
		Node::Key(_, _, right) => {
			visit(program);
			visit_statements(right, visit);
		}
		other => visit(other),
	}
}

/// The names an arithmetic or comparison operator of the block applies to, each once
fn arithmetic_operands(block: &Node) -> Vec<String> {
	let mut names: Vec<String> = vec![];
	block.visit(&mut |node| {
		if let Node::Key(left, op, right) = node {
			if op.is_arithmetic() || op.is_comparison() {
				for operand in [left, right] {
					if let Node::Symbol(name) = operand.drop_meta() {
						if !names.contains(name) {
							names.push(name.clone());
						}
					}
				}
			}
		}
	});
	names.sort();
	names
}

fn occurrences(node: &Node, name: &str) -> usize {
	let mut count = 0;
	node.visit(&mut |part| count += matches!(part, Node::Symbol(word) if word == name) as usize);
	count
}

/// `interpret e` (wiki/charged.md §5: `x!!` or `interpret x`): of a block bound by `x : e` it is `x!!`, which blocks.rs
/// runs where it is written; of any other value it runs the block at run time through the host,
/// `run_block(e, "a b", [a, b], data [definitions])`: the main-level variables assigned before it (a snapshot the
/// block reads) and the program's function definitions (the block may call them)
pub fn lower_interpret(program: Node) -> Node {
	lower_statements(program, &mentions_interpret, &|site: &Site, node: Node| site.interpreted(node))
}

/// `x!` that blocks.rs did not run (wiki/charged.md §5, user decision P73: `!` forces, the kind decides): a block known
/// only at run time runs through the host. That is `e!` of any expression but a name or a field (`xs#2!`, `f(x)!`),
/// and `x!` of a name that holds an element of a list of data (`xs = [data a+1, …]; y = xs#2; y!`); any other name
/// stays mutation.rs's unwrap, which keeps `x!` of an optional pure. The host gives a value that is no code back as it is.
pub fn lower_run_time_bangs(program: Node) -> Node {
	let holders = data_holders(&program);
	lower_statements(program, &has_bang, &|site: &Site, node: Node| site.forced(node, &holders))
}

fn lower_statements(program: Node, applies: &dyn Fn(&Node) -> bool, lower: &dyn Fn(&Site, Node) -> Node) -> Node {
	let statements = match program.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => vec![program.clone()],
	};
	if !statements.iter().any(applies) {
		return program;
	}
	let all_variables = statements.iter().filter_map(assigned_name).fold(vec![], |mut names: Vec<String>, name| {
		if !names.contains(&name) {
			names.push(name);
		}
		names
	});
	let definitions = statements.iter().filter(|statement| defines_functions(statement)).cloned().collect();
	let mut site = Site { blocks: vec![], variables: vec![], all_variables, definitions };
	let mut lowered = vec![];
	for statement in statements {
		lowered.push(lower(&site, statement.clone()));
		site.follow(&statement);
	}
	match program {
		Node::List(_, bracket, separator) => Node::List(lowered, bracket, separator),
		Node::Meta { data, .. } => Node::Meta { node: Box::new(Node::List(lowered, Bracket::None, Separator::Newline)), data },
		_ => lowered.into_iter().next().unwrap_or(Node::Empty),
	}
}

/// What a run-time block sees where it runs
struct Site {
	/// the constant blocks bound so far (`x : e`)
	blocks: Vec<String>,
	/// the main-level variables assigned so far
	variables: Vec<String>,
	/// every main-level variable: a function body may run when all are bound (late binding reads their current values)
	all_variables: Vec<String>,
	/// the program's function definitions
	definitions: Vec<Node>,
}

/// `x = …` as a statement: x
fn assigned_name(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Key(target, Op::Assign, _) => match target.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

fn defines_functions(statement: &Node) -> bool {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, statement);
	!context.user_functions.is_empty() && crate::blocks::uncharged(statement).is_none()
}

/// The names whose kind is known only at run time, so that their `!` decides there (P73): assigned an element of a list
/// that holds data (`xs = [data a+1, data 2]; y = xs#2`), or what a function of the program gives (`y = g()`, `g()#1`)
fn data_holders(program: &Node) -> Vec<String> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, program);
	let mut lists: Vec<String> = vec![];
	let mut holders: Vec<String> = vec![];
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign, value) = node else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		match value.drop_meta() {
			Node::List(items, _, _) if items.iter().any(is_data) => lists.push(name.clone()),
			value if known_at_run_time(value, &lists, &context) => holders.push(name.clone()),
			_ => {}
		}
	});
	holders
}

/// An element of a list of data, or the result of a call of the program's own functions (or an element of it)
fn known_at_run_time(value: &Node, lists: &[String], context: &crate::context::Context) -> bool {
	match value.drop_meta() {
		Node::Key(list, Op::Hash, _) => matches!(list.drop_meta(), Node::Symbol(list) if lists.contains(list)) || known_at_run_time(list, lists, context),
		Node::List(items, _, Separator::None | Separator::Space) => {
			matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(function)) if context.user_functions.contains_key(function))
		}
		_ => false,
	}
}

/// The parameters of a definition (only those without a declared type: anything may come in, a block too)
fn parameters(definition: &Node, untyped_only: bool) -> Vec<String> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, definition);
	context.user_functions.values().flat_map(|function| function.params.iter())
		.filter(|param| !untyped_only || param.annotation.is_none())
		.map(|param| param.name.clone())
		.collect()
}

/// `upper x!`: the `!` mutates x (D2, mutation.rs), it forces nothing
fn is_mutating_call(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, separator) if items.len() == 2 && !matches!(separator, Separator::Semicolon | Separator::Newline)
		&& matches!(items[0].drop_meta(), Node::Symbol(_)) && crate::mutation::bang_target(&items[1]).is_some_and(|(inner, _)| matches!(inner, Node::Symbol(_))))
}

pub(crate) fn is_data(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == crate::blocks::DATA_WORD))
}

/// A `!` mark this pass runs at run time: on an expression but a name or a field, or on a name holding data
fn run_time_bang(node: &Node, holders: &[String]) -> Option<Node> {
	let (inner, _) = crate::mutation::bang_target(node)?;
	match &inner {
		Node::Symbol(name) => holders.contains(name).then_some(inner),
		Node::Key(_, Op::Dot, _) => None,
		_ => Some(inner),
	}
}

/// Is there a `!` mark anywhere (Node::visit looks through the meta wrappers that carry the marks)
fn has_bang(node: &Node) -> bool {
	crate::mutation::bang_target(node).is_some() || match node {
		Node::Meta { node, .. } => has_bang(node),
		Node::Key(left, _, right) => has_bang(left) || has_bang(right),
		Node::List(items, _, _) => items.iter().any(has_bang),
		_ => false,
	}
}

fn mentions_interpret(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(word) if word == INTERPRET));
	found
}

/// `interpret e` / `interpret(e)`: e; `interpret(data n+k)`: the phrase `data n+k`
fn interpret_argument(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == INTERPRET) => Some(match items.len() {
			2 => items[1].clone(),
			_ => Node::List(items[1..].to_vec(), Bracket::None, Separator::Space),
		}),
		_ => None,
	}
}

fn as_data(node: Node) -> Node {
	Node::List(vec![Node::Symbol(crate::blocks::DATA_WORD.to_string()), node], Bracket::None, Separator::Space)
}

impl Site {
	/// After a statement: the block or variable it binds
	fn follow(&mut self, statement: &Node) {
		if let Some((name, _)) = crate::blocks::uncharged(statement) {
			self.blocks.push(name);
			return;
		}
		if let Some(name) = assigned_name(statement) {
			self.blocks.retain(|block| *block != name);
			if !self.variables.contains(&name) {
				self.variables.push(name);
			}
		}
	}

	/// `run_block(e, "a b", [a, b], data [definitions])`
	fn run_block(&self, block: Node) -> Node {
		let names = Node::Text(self.variables.join(" "));
		let values = Node::List(self.variables.iter().map(|name| Node::Symbol(name.clone())).collect(), Bracket::Square, Separator::Colon);
		let definitions = as_data(Node::List(self.definitions.clone(), Bracket::Square, Separator::Colon));
		Node::List(vec![Node::Symbol(crate::host::RUN_BLOCK.to_string()), block, names, values, definitions], Bracket::Round, Separator::None)
	}

	fn interpreted(&self, node: Node) -> Node {
		if let Some(argument) = interpret_argument(&node) {
			let argument = self.interpreted(argument);
			return match argument.drop_meta() {
				Node::Symbol(name) if self.blocks.contains(name) => crate::mutation::marked_fully(argument),
				_ => self.run_block(argument),
			};
		}
		if defines_functions(&node) {
			return self.in_body(&node).each_part(node, &|site, part| site.interpreted(part));
		}
		self.each_part(node, &|site, part| site.interpreted(part))
	}

	fn forced(&self, node: Node, holders: &[String]) -> Node {
		if is_mutating_call(&node) {
			return node;
		}
		if let Some(block) = run_time_bang(&node, holders) {
			return self.run_block(self.forced(block, holders));
		}
		if defines_functions(&node) {
			let with_parameters = [holders.to_vec(), parameters(&node, true)].concat();
			return self.in_body(&node).each_part(node, &|site, part| site.forced(part, &with_parameters));
		}
		self.each_part(node, &|site, part| site.forced(part, holders))
	}

	/// Inside a function body its parameters and every main-level variable are visible (late binding)
	fn in_body(&self, definition: &Node) -> Site {
		let parameters = parameters(definition, false);
		let variables = [parameters.clone(), self.all_variables.iter().filter(|name| !parameters.contains(name)).cloned().collect()].concat();
		Site { blocks: self.blocks.clone(), variables, all_variables: self.all_variables.clone(), definitions: self.definitions.clone() }
	}

	fn each_part(&self, node: Node, lower: &dyn Fn(&Site, Node) -> Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| lower(self, item)).collect(), bracket, separator),
			Node::Key(left, op, right) => Node::Key(Box::new(lower(self, *left)), op, Box::new(lower(self, *right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(lower(self, *node)), data },
			other => other,
		}
	}
}
