//! A block known only at run time runs through the host (run_block, notes/runtime_eval.md step 2).

use super::nodes::{call, for_in_variable};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

/// The spec's word for `x!!`: run the block fully
const INTERPRET: &str = "interpret";

/// `interpret e` (wiki/charged.md §5: `x!!` or `interpret x`): of a block bound by `x : e` it is `x!!`, which blocks.rs
/// runs where it is written; of any other value it runs the block at run time through the host,
/// `run_block(e, "a b", [a, b], data [definitions])`: the main-level variables assigned before it (a snapshot the
/// block reads) and the program's function definitions (the block may call them)
pub fn lower_interpret(program: Node) -> Node {
	lower_statements(program, &|node: &Node| node.mentions(INTERPRET), &|site: &Site, node: Node| {
		site.interpreted(node)
	})
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
	let statements = program.sequence_statements().to_vec();
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
		// LineInfo (and similar) may wrap a whole program; a bang mark must stay on the expression
		// (`ø!` has no LineInfo, so the mutated Meta is outermost — do not promote it onto a list)
		Node::Meta { data, .. } if crate::mutation::bang_target(&program).is_none() => {
			Node::Meta { node: Box::new(Node::List(lowered, Bracket::None, Separator::Newline)), data }
		}
		_ => lowered.into_iter().next().unwrap_or(Node::Empty),
	}
}

/// What a run-time block sees where it runs
#[derive(Clone)]
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
		Node::Key(target, Op::Assign, _) => target.symbol_name().map(String::from),
		_ => None,
	}
}

fn defines_functions(statement: &Node) -> bool {
	let context = crate::analyzer::function_definitions(statement);
	!context.user_functions.is_empty() && crate::blocks::uncharged(statement).is_none()
}

/// The names whose kind is known only at run time, so that their `!` decides there (P73): assigned an element of a list
/// that holds data (`xs = [data a+1, data 2]; y = xs#2`), or what a function of the program gives (`y = g()`, `g()#1`)
fn data_holders(program: &Node) -> Vec<String> {
	let mut context = crate::compiler::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, program);
	let mut lists: Vec<String> = vec![];
	let mut holders: Vec<String> = vec![];
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign, value) = node else {
			return;
		};
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
fn known_at_run_time(value: &Node, lists: &[String], context: &crate::compiler::context::Context) -> bool {
	match value.drop_meta() {
		Node::Key(list, Op::Hash, _) => {
			matches!(list.drop_meta(), Node::Symbol(list) if lists.contains(list))
				|| known_at_run_time(list, lists, context)
		}
		Node::List(items, _, Separator::None | Separator::Space) => items
			.first()
			.and_then(Node::symbol_name)
			.is_some_and(|function| context.user_functions.contains_key(function)),
		_ => false,
	}
}

/// The parameters of a definition (only those without a declared type: anything may come in, a block too)
fn parameters(definition: &Node, untyped_only: bool) -> Vec<String> {
	let mut context = crate::compiler::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, definition);
	context
		.user_functions
		.values()
		.flat_map(|function| function.params.iter())
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
	node.is_call_of(crate::blocks::DATA_WORD)
}

/// A `!` mark this pass runs at run time: on an expression but a name or a field, or on a name holding data.
/// Empty (`ø!`) stays mutation.rs's unwrap (P73 force of an optional).
fn run_time_bang(node: &Node, holders: &[String]) -> Option<Node> {
	let (inner, _) = crate::mutation::bang_target(node)?;
	match &inner {
		Node::Symbol(name) => holders.contains(name).then_some(inner),
		Node::Key(_, Op::Dot, _) => None,
		Node::Empty => None,
		// `now!`: the run reads the clock where it stands, a block could not hand the time back (card data-run-now)
		_ if crate::time::is_time_call(&inner) => None,
		_ => Some(inner),
	}
}

/// Is there a `!` mark anywhere (Node::visit looks through the meta wrappers that carry the marks)
fn has_bang(node: &Node) -> bool {
	crate::mutation::bang_target(node).is_some()
		|| match node {
			Node::Meta { node, .. } => has_bang(node),
			Node::Key(left, _, right) => has_bang(left) || has_bang(right),
			Node::List(items, _, _) => items.iter().any(has_bang),
			_ => false,
		}
}

/// `interpret e` / `interpret(e)`: e; `interpret(data n+k)`: the phrase `data n+k`
fn interpret_argument(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() >= 2 && items[0].is_symbol(INTERPRET) => Some(match items.len() {
			2 => items[1].clone(),
			_ => Node::List(items[1..].to_vec(), Bracket::None, Separator::Space),
		}),
		_ => None,
	}
}

impl Site {
	/// After a statement: the block or variable it binds
	fn follow(&mut self, statement: &Node) {
		if let Some((name, _)) = crate::blocks::uncharged(statement) {
			self.blocks.push(name);
			return;
		}
		if let Some(name) = assigned_name(statement) {
			self.bind(name);
		}
	}

	fn bind(&mut self, name: String) {
		self.blocks.retain(|block| *block != name);
		if !self.variables.contains(&name) {
			self.variables.push(name);
		}
	}

	/// `run_block(e, "a b", [a, b], data [definitions])`
	fn run_block(&self, block: Node) -> Node {
		let names = Node::Text(self.variables.join(" "));
		let values = Node::List(
			self.variables.iter().map(|name| Node::Symbol(name.clone())).collect(),
			Bracket::Square,
			Separator::Comma,
		);
		let definitions =
			crate::blocks::as_data(Node::List(self.definitions.clone(), Bracket::Square, Separator::Comma));
		call(crate::host::RUN_BLOCK, vec![block, names, values, definitions])
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
		let variables = [
			parameters.clone(),
			self.all_variables.iter().filter(|name| !parameters.contains(name)).cloned().collect(),
		]
		.concat();
		Site {
			blocks: self.blocks.clone(),
			variables,
			all_variables: self.all_variables.clone(),
			definitions: self.definitions.clone(),
		}
	}

	/// The parts of `node` lowered where they are: a statement of a body after the variables the statements before it
	/// assign, the body of `for i in …` with the loop's variable
	fn each_part(&self, node: Node, lower: &dyn Fn(&Site, Node) -> Node) -> Node {
		match node {
			Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
				let mut site = self.clone();
				let items = items
					.into_iter()
					.map(|item| {
						let lowered = lower(&site, item.clone());
						site.follow(&item);
						lowered
					})
					.collect();
				Node::List(items, bracket, separator)
			}
			Node::List(items, bracket, separator) if for_in_variable(&items).is_some() => {
				let mut site = self.clone();
				site.bind(for_in_variable(&items).unwrap_or_default());
				Node::List(items.into_iter().map(|item| lower(&site, item)).collect(), bracket, separator)
			}
			other => other.map_children(|child| lower(self, child)),
		}
	}
}
