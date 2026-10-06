//! Named arguments (user decision P37): `f(b=1, a=5)` and `f(b:1, a:5)` pass a by name, and a name that is no parameter
//! but a free variable of the body sets it: `f y := y*y+v; f(y=2, v=3)` is 7, `fun={x*y}; fun(x:2 y:3)` is 6. Such a
//! variable becomes an extra parameter of the function; a call that does not name it passes the variable of that name
//! where it is called, as the body read it before. A parameter is never taken from a same-named variable: a missing
//! argument stays an error.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

/// `x?`, `int?`: optional
const OPTIONAL_MARK: char = '?';
const MAYBE_WORD: &str = "maybe";

struct Function {
	parameters: Vec<String>,
	/// the default of each parameter `a=1`, if any
	defaults: Vec<Option<Node>>,
	/// free variables of the body some call names, appended as parameters in their order of first use in the body
	extras: Vec<String>,
}

pub fn lower(node: Node) -> Node {
	let node = default_forms(node);
	let mut functions = definitions(&node);
	if functions.is_empty() {
		return node;
	}
	DECLARED_TYPES.with(|types| *types.borrow_mut() = declared_types(&node));
	let mut unknown = None;
	collect_extras(&node, &mut functions, &mut unknown);
	if let Some(error) = unknown {
		return error;
	}
	if !functions.values().any(|function| !function.extras.is_empty()) && !has_named_call(&node, &functions) {
		return node;
	}
	Rewrite { functions }.node(node)
}

/// Defaults written as other languages do: Ruby's keyword parameter `def f(a, b: 2)` (a literal after the colon is no
/// type but the default, `b=2`) and the optional `x?`, `x: int?` (TypeScript, Swift, Kotlin), `maybe x`, `maybe int x`,
/// `x: maybe int` (P125: the same as `x=ø`, a missing argument is ø)
fn default_forms(node: Node) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) => {
			let head = with_parameters(*head, &|parameters| maybe_parameters(parameters).into_iter().map(literal_default).collect());
			Node::Key(Box::new(head), op, Box::new(default_forms(*body)))
		}
		other => other.map_children(default_forms),
	}
}

fn optional(name: Node) -> Node {
	Node::Key(Box::new(name), Op::Assign, Box::new(Node::Empty))
}

fn is_maybe(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == MAYBE_WORD)
}

/// `maybe x`, `maybe int x`, `x: maybe int` among the parameters (also spaced in one item): `x=ø`
fn maybe_parameters(parameters: Vec<Node>) -> Vec<Node> {
	let mut forms = vec![];
	let mut items = parameters.into_iter().peekable();
	while let Some(item) = items.next() {
		match item.drop_meta() {
			Node::List(words, Bracket::None | Bracket::Round, Separator::Space) if words.iter().any(is_maybe) || words.first().is_some_and(is_maybe_annotated) => {
				forms.extend(maybe_parameters(words.clone()));
			}
			_ if is_maybe(&item) => {
				let mut name = items.next();
				if name.as_ref().is_some_and(|word| crate::analyzer::type_word_kind(&word.drop_meta().name()).is_some()) && items.peek().is_some() {
					name = items.next(); // `maybe int x`
				}
				match name {
					Some(name) => forms.push(optional(Node::Symbol(parameter_name(&name)))),
					None => forms.push(item),
				}
			}
			Node::Key(name, Op::Colon, _) if is_maybe_annotated(&item) => {
				items.next_if(|word| matches!(word.drop_meta(), Node::Symbol(_))); // the type after `maybe`
				forms.push(optional(name.as_ref().clone()));
			}
			_ => forms.push(item),
		}
	}
	forms
}

/// `x: maybe` (the type word follows as the next item)
fn is_maybe_annotated(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, annotation) if is_maybe(annotation))
}

fn literal_default(parameter: Node) -> Node {
	match parameter {
		Node::Key(name, Op::Colon, value) if matches!(value.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_)) => Node::Key(name, Op::Assign, value),
		// the value may be ø or of the type: it is held boxed, as any value
		Node::Key(name, Op::Colon, type_name) if type_name.drop_meta().name().ends_with(OPTIONAL_MARK) => optional(*name),
		Node::Symbol(name) if name.len() > 1 && name.ends_with(OPTIONAL_MARK) => optional(Node::Symbol(name.trim_end_matches(OPTIONAL_MARK).to_string())),
		Node::Meta { node, data } => Node::Meta { node: Box::new(literal_default(*node)), data },
		other => other,
	}
}

/// `f(a, b) := body` and the block value `fun = {body}`, by name: their parameters
fn definitions(node: &Node) -> HashMap<String, Function> {
	let mut functions = HashMap::new();
	node.visit(&mut |part| if let Node::Key(head, Op::Define | Op::Assign, _) = part { match untyped_head(head).drop_meta() {
 			Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
 				let parameters = items[1..].iter().map(parameter_name).collect();
 				let defaults = items[1..].iter().map(parameter_default).collect();
 				functions.insert(items[0].name(), Function { parameters, defaults, extras: vec![] });
 			}
 			_ => {}
 		} });
	node.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		if let (Node::Symbol(name), Node::List(_, Bracket::Curly, _)) = (target.drop_meta(), value.drop_meta()) {
			functions.entry(name.clone()).or_insert(Function { parameters: vec![], defaults: vec![], extras: vec![] });
		}
	});
	functions
}

/// `a`, `a:int`, `a=1`: the parameter's name
/// `x`, `x: int`, `x = 1`, and a function-typed `f: (Int) -> Int`: the name
fn parameter_name(parameter: &Node) -> String {
	match parameter.drop_meta() {
		Node::Key(name, _, _) => parameter_name(name),
		other => other.name(),
	}
}

/// `f(x)`: a name with its parameters
fn is_function_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))))
}

/// `f(x): T`, `f(x) as T`: the head `f(x)` without its result type
fn untyped_head(head: &Node) -> &Node {
	match head.drop_meta() {
		Node::Key(inner, Op::Colon | Op::As, _) if is_function_head(inner) => inner,
		_ => head,
	}
}

/// The head with its parameters rewritten, its result type kept
fn with_parameters(head: Node, rewrite: &impl Fn(Vec<Node>) -> Vec<Node>) -> Node {
	match head.drop_meta() {
		Node::Key(inner, op @ (Op::Colon | Op::As), result) if is_function_head(inner) => {
			Node::Key(Box::new(with_parameters(inner.as_ref().clone(), rewrite)), *op, result.clone())
		}
		Node::List(items, Bracket::Round, separator) if is_function_head(&head) => {
			let (name, parameters) = items.split_first().expect("a name");
			Node::List(std::iter::once(name.clone()).chain(rewrite(parameters.to_vec())).collect(), Bracket::Round, separator.clone())
		}
		_ => head,
	}
}

fn parameter_default(parameter: &Node) -> Option<Node> {
	match parameter.drop_meta() {
		Node::Key(_, Op::Assign, default) => Some(default.as_ref().clone()),
		_ => None,
	}
}

thread_local! {
	/// The program's declared types, while this pass runs: `pic{width:5}` constructs and `s:shape` declares, neither
	/// names an argument
	static DECLARED_TYPES: std::cell::RefCell<std::collections::HashSet<String>> = Default::default();
}

fn declared_types(node: &Node) -> std::collections::HashSet<String> {
	let mut types = std::collections::HashSet::new();
	node.visit(&mut |part| if let Node::Type { name, .. } = part { types.insert(name.name()); });
	types
}

fn is_type(name: &str) -> bool {
	crate::analyzer::type_word_kind(name).is_some() || DECLARED_TYPES.with(|types| types.borrow().contains(name))
}

/// `b=1` or `b:1` in a call: the name and the value. Not `pic:{…}` (an instance) nor `s:shape` (a parameter
/// declaration in a signature)
pub(crate) fn named_argument(argument: &Node) -> Option<(String, &Node)> {
	match argument.drop_meta() {
		Node::Key(name, op @ (Op::Assign | Op::Colon), value) => match name.drop_meta() {
			Node::Symbol(_) if *op == Op::Colon && is_type(&name.name()) => None,
			Node::Symbol(_) if *op == Op::Colon && matches!(value.drop_meta(), Node::Symbol(type_name) if is_type(type_name)) => None,
			// `pic{width:5}` (parsed as `pic:{…}`) is an ad-hoc instance: a named object argument is written `opts={…}`;
			// a block that is a function is the argument (Swift `using: { $0 * 3 }`)
			Node::Symbol(_) if *op == Op::Colon && matches!(value.drop_meta(), Node::List(_, crate::node::Bracket::Curly, _)) && !is_block_function(value) => None,
			Node::Symbol(name) => Some((name.clone(), value)),
			_ => None,
		},
		_ => None,
	}
}

/// `{ $0 * 3 }`, `{ x in x * 3 }`, `{ it * 3 }`, `{ x -> x * 3 }`
fn is_block_function(value: &Node) -> bool {
	crate::lambdas::arrow_lambda(value).is_some() || crate::lambdas::block_as_arrow(value).is_some()
}

fn call_of<'a>(node: &'a Node, functions: &HashMap<String, Function>) -> Option<(String, &'a [Node])> {
	let Node::List(items, Bracket::Round, _) = node.drop_meta() else { return None };
	let Node::Symbol(name) = items.first()?.drop_meta() else { return None };
	functions.contains_key(name).then(|| (name.clone(), &items[1..]))
}

fn has_named_call(node: &Node, functions: &HashMap<String, Function>) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= call_of(part, functions).is_some_and(|(_, arguments)| arguments.iter().any(|a| named_argument(a).is_some())));
	found
}

/// The bodies, by function name
fn bodies(node: &Node) -> HashMap<String, Node> {
	let mut bodies = HashMap::new();
	node.visit(&mut |part| if let Node::Key(head, Op::Define | Op::Assign, body) = part {
		match untyped_head(head).drop_meta() {
			Node::List(items, Bracket::Round, _) if !items.is_empty() => { bodies.insert(items[0].name(), body.as_ref().clone()); }
			Node::Symbol(name) if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => { bodies.insert(name.clone(), body.as_ref().clone()); }
			_ => {}
		}
	});
	bodies
}

/// Every named argument that is no parameter must be a free variable of the body: it becomes an extra parameter
fn collect_extras(node: &Node, functions: &mut HashMap<String, Function>, unknown: &mut Option<Node>) {
	let bodies = bodies(node);
	let mut named: Vec<(String, String, Node)> = vec![];
	node.visit(&mut |part| if let Some((name, arguments)) = call_of(part, functions) {
		named.extend(arguments.iter().filter_map(named_argument).map(|(argument, _)| (name.clone(), argument, part.clone())));
	});
	for (name, argument, call) in named {
		let function = functions.get_mut(&name).expect("a call of a known function");
		if function.parameters.contains(&argument) || function.extras.contains(&argument) {
			continue;
		}
		let used = bodies.get(&name).map(symbols_in_order).unwrap_or_default();
		if !used.contains(&argument) {
			unknown.get_or_insert_with(|| Diagnostic::at(&call, format!("{name} has no parameter {argument}")).into_error());
			continue;
		}
		function.extras.push(argument);
		function.extras.sort_by_key(|extra| used.iter().position(|symbol| symbol == extra));
	}
}

fn symbols_in_order(body: &Node) -> Vec<String> {
	let mut symbols = vec![];
	body.visit(&mut |part| if let Node::Symbol(name) = part {
		if !symbols.contains(name) {
			symbols.push(name.clone());
		}
	});
	symbols
}

struct Rewrite {
	functions: HashMap<String, Function>,
}

/// `fun = {x*y}` called by name: `fun` is also a function keyword, so the function made of the block is `fun·block`
const KEYWORD_FUNCTION_SUFFIX: &str = "·block";

/// The name a function made of a block value is defined and called by
fn callable_name(name: &str) -> String {
	match crate::operators::is_function_keyword(name) {
		true => format!("{name}{KEYWORD_FUNCTION_SUFFIX}"),
		false => name.to_string(),
	}
}

impl Rewrite {
	fn node(&self, node: Node) -> Node {
		match node {
			Node::Key(head, op @ (Op::Define | Op::Assign), body) => self.definition(*head, op, *body),
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.node(item)).collect();
				let list = Node::List(items, bracket, separator);
				match call_of(&list, &self.functions) {
					Some((name, arguments)) => self.call(&list, &name, arguments).unwrap_or_else(|error| error),
					None => list,
				}
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.node(*left)), op, Box::new(self.node(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.node(*node)), data },
			other => other,
		}
	}

	/// The extras join the parameters; `fun = {body}` named in a call becomes `fun(extras) := body`
	fn definition(&self, head: Node, op: Op, body: Node) -> Node {
		// `f(x): T := body`: the head keeps its result type
		if let Node::Key(inner, result_op @ (Op::Colon | Op::As), result) = head.drop_meta() {
			if is_function_head(inner) {
				let Node::Key(inner, _, body) = self.definition(inner.as_ref().clone(), op, body) else { unreachable!("a definition") };
				return Node::Key(Box::new(Node::Key(inner, *result_op, result.clone())), op, body);
			}
		}
		let body = self.node(body);
		match head.drop_meta() {
			Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
				let extras = self.functions.get(&items[0].name()).map(|function| function.extras.clone()).unwrap_or_default();
				let items = items.iter().cloned().chain(extras.into_iter().map(Node::Symbol)).collect();
				Node::Key(Box::new(Node::List(items, Bracket::Round, separator.clone())), op, Box::new(body))
			}
			Node::Symbol(name) if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => match self.functions.get(name) {
				Some(function) if !function.extras.is_empty() => {
					let items = std::iter::once(Node::Symbol(callable_name(name))).chain(function.extras.iter().cloned().map(Node::Symbol)).collect();
					Node::Key(Box::new(Node::List(items, Bracket::Round, Separator::None)), Op::Define, Box::new(body))
				}
				_ => Node::Key(Box::new(head), op, Box::new(body)),
			},
			_ => Node::Key(Box::new(head), op, Box::new(body)),
		}
	}

	/// The arguments in parameter order: positional ones fill the parameters not named, extras not named pass the
	/// variable of their name
	fn call(&self, call: &Node, name: &str, arguments: &[Node]) -> Result<Node, Node> {
		let function = &self.functions[name];
		if function.extras.is_empty() && !arguments.iter().any(|argument| named_argument(argument).is_some()) {
			return Ok(call.clone());
		}
		let named: HashMap<String, &Node> = arguments.iter().filter_map(named_argument).collect();
		let mut positional = arguments.iter().filter(|argument| named_argument(argument).is_none());
		let callee = if function.parameters.is_empty() && !function.extras.is_empty() { callable_name(name) } else { name.to_string() };
		let mut ordered = vec![Node::Symbol(callee)];
		for (parameter, default) in function.parameters.iter().zip(&function.defaults) {
			match named.get(parameter).copied().or_else(|| positional.next()).or(default.as_ref()) {
				Some(value) => ordered.push(value.clone()),
				None => return Err(Diagnostic::at(call, format!("{name} needs a value for parameter {parameter}")).into_error()),
			}
		}
		ordered.extend(positional.cloned());
		ordered.extend(function.extras.iter().map(|extra| named.get(extra).map_or_else(|| Node::Symbol(extra.clone()), |value| (*value).clone())));
		Ok(Node::List(ordered, Bracket::Round, Separator::None))
	}
}
