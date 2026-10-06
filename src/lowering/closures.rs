//! Real closures: a function as a run-time value. Where compile-time specialisation (function_values.rs) cannot know the
//! function, a lambda is lifted to a top-level function whose first parameters are the variables it captures (by value,
//! decision D7), and the lambda becomes `closure_new(lifted, captured…)`: at run time a $Node of Kind::Function holding a
//! $Closure struct (a typed function reference plus the captured values). Calling a variable that holds one, `f(x)`,
//! becomes `closure_call_1(f, x)`, a helper per arity that call_refs the closure's entry (wasm_emitter/closures.rs).
//! notes/closures.md describes the representation.

use crate::analyzer::extract_user_functions;
use crate::context::{Context, Param, UserFunctionDef};
use crate::lambdas::arrow_lambda;
use crate::library_words::collect_assigned_names;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::{HashMap, HashSet};

pub const CLOSURE_NEW: &str = "closure_new";
/// The mark of `&add` / `function add`: the function itself, not its call (user decision P82)
const FUNCTION_REFERENCE_MARK: &str = "function_reference";

/// `&add`, `function add`: the name, marked as the function itself; passes that ignore marks see the name
pub fn function_reference(name: String) -> Node {
	Node::Meta { node: Box::new(Node::Symbol(name)), data: Box::new(Node::key(FUNCTION_REFERENCE_MARK, Node::True)) }
}

/// The function `&add` / `function add` refers to
pub fn referenced_function(node: &Node) -> Option<String> {
	let Node::Meta { node: inner, data } = node else { return None };
	match data.as_ref() {
		Node::Key(key, _, _) if key.name() == FUNCTION_REFERENCE_MARK => match inner.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => referenced_function(inner),
	}
}
const CLOSURE_CALL_PREFIX: &str = "closure_call_";
const LIFTED_PREFIX: &str = "closure_lambda_";
const IMPLICIT_PARAMETER: &str = "it";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";

pub fn closure_call_name(arity: usize) -> String {
	format!("{CLOSURE_CALL_PREFIX}{arity}")
}

/// `closure_call_2` → 2
pub fn closure_call_arity(name: &str) -> Option<usize> {
	name.strip_prefix(CLOSURE_CALL_PREFIX)?.parse().ok()
}

fn call(name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

/// The call of the function value `function` with `arguments`
pub fn closure_call(function: Node, arguments: Vec<Node>) -> Node {
	call(&closure_call_name(arguments.len()), [vec![function], arguments].concat())
}

fn closure_new(target: &str, captured: Vec<Node>) -> Node {
	call(CLOSURE_NEW, [vec![Node::Symbol(target.to_string())], captured].concat())
}

/// `closure_new(target, captured…)`: the target and its captured values
pub fn as_closure_new(node: &Node) -> Option<(&str, &[Node])> {
	let Node::List(items, Bracket::Round, Separator::None) = node.drop_meta() else { return None };
	let [head, target, captured @ ..] = items.as_slice() else { return None };
	match (head.drop_meta(), target.drop_meta()) {
		(Node::Symbol(word), Node::Symbol(target)) if word == CLOSURE_NEW => Some((target, captured)),
		_ => None,
	}
}

/// Every closure made in the program: (target function, number of captured values), each once
pub fn closure_targets(program: &Node) -> Vec<(String, usize)> {
	let mut found: Vec<(String, usize)> = vec![];
	program.visit(&mut |node| {
		if let Some((target, captured)) = as_closure_new(node) {
			let entry = (target.to_string(), captured.len());
			if !found.contains(&entry) {
				found.push(entry);
			}
		}
	});
	found
}

/// The variable a closure call calls: `f` in `closure_call_1(f, x)`
pub fn called_closure(node: &Node) -> Option<&str> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [head, function, ..] = items.as_slice() else { return None };
	match (head.drop_meta(), function.drop_meta()) {
		(Node::Symbol(word), Node::Symbol(function)) if closure_call_arity(word).is_some() => Some(function),
		_ => None,
	}
}

/// The arities of the closure calls in the program
pub fn closure_call_arities(program: &Node) -> Vec<usize> {
	let mut found = vec![];
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				if let Some(arity) = closure_call_arity(name).filter(|arity| !found.contains(arity)) {
					found.push(arity);
				}
			}
		}
	});
	found
}

/// A value that cannot be a function where one is needed: `needer needs a function[ for parameter p], got 5 (an Int); fix: …`
pub fn needs_a_function(needer: &str, parameter: Option<&str>, value: &Node, fix: &str) -> Node {
	let kind = crate::analyzer::infer_type(value, &crate::analyzer::Scope::new());
	let parameter = parameter.map(|name| format!(" for parameter {name}")).unwrap_or_default();
	let message = format!("{needer} needs a function{parameter}, got {} ({}); fix: {fix}", value.serialize(), crate::analyzer::kind_with_article(kind));
	crate::diagnostic::Diagnostic::at(value, message).into_error()
}

/// A function value that a call cannot be specialised for: a variable, a lambda, a call that may return a function, an
/// element of a list (`fs#1`, the arguments of a task's wrapper)
pub fn may_be_function_value(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(_) | Node::Key(_, Op::Hash, _) => true,
		Node::List(items, Bracket::Round, Separator::None) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))),
		other => arrow_lambda(other).is_some(),
	}
}

pub fn lower(program: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &program);
	let mut variables = HashSet::new();
	collect_assigned_names(&program, &mut variables);
	// a loop variable (`for n in xs`, what `xs.map(n => …)` lowers to) is captured by value like any variable
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if let [keyword, variable, in_word, _, _] = items.as_slice() {
				let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(name) if name == word);
				if let (true, Node::Symbol(name)) = (is_word(keyword, FOR_WORD) && is_word(in_word, IN_WORD), variable.drop_meta()) {
					variables.insert(name.clone());
				}
			}
		}
	});
	let functions: HashSet<String> = context.user_functions.keys().cloned().collect();
	let mut values = FunctionValues { functions, returning: HashSet::new(), variables: HashSet::new(), lists: HashSet::new() };
	values.settle(&context, &program);
	let FunctionValues { functions, returning: returns_function, variables: function_variables, lists: function_lists } = values;
	let mut lifting = Lifting { functions, variables, function_variables, function_lists, returns_function, lifted: vec![], enclosing: vec![] };
	let program = lifting.walk(program, &HashSet::new());
	if lifting.lifted.is_empty() {
		return program;
	}
	Node::List([lifting.lifted, vec![program]].concat(), Bracket::Round, Separator::Semicolon)
}

/// The last statement of a body: its value
fn tail(body: &Node) -> &Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::Round, Separator::Semicolon | Separator::Newline) if !items.is_empty() => tail(&items[items.len() - 1]),
		Node::List(items, Bracket::Round | Bracket::Curly, _) if items.len() == 1 => tail(&items[0]),
		Node::List(items, _, _) if is_return(items) => tail(&items[1]),
		other => other,
	}
}

/// `function add` or `return function add`
fn gives_reference(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) if is_return(items) => referenced_function(&items[1]).is_some(),
		_ => referenced_function(node).is_some(),
	}
}

/// `return value`
fn is_return(items: &[Node]) -> bool {
	matches!(items, [word, _] if matches!(word.drop_meta(), Node::Symbol(name) if name == "return"))
}

/// The variable or field a node names: `s`, `s.fs` (a field lookup `s#("fs"+1)` after mutation.rs), `s.a.fs`
fn holder_path(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(object, Op::Dot, field) => Some(format!("{}.{}", holder_path(object)?, field_word(field)?)),
		Node::Key(object, Op::Hash, index) => Some(format!("{}.{}", holder_path(object)?, field_word(crate::wasp_parser::subscript_key(index)?)?)),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => holder_path(&items[0]),
		_ => None,
	}
}

fn field_word(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		_ => None,
	}
}

/// What may hold a function value, judged from the source: the functions returning one, the variables assigned one
struct FunctionValues {
	functions: HashSet<String>,
	returning: HashSet<String>,
	variables: HashSet<String>,
	/// Variables assigned a list of function values: `fs#2` is one
	lists: HashSet<String>,
}

impl FunctionValues {
	/// A lambda, a function name, a variable holding a function, a call returning one (a called closure may), or a choice between them
	fn is_function_value(&self, node: &Node) -> bool {
		match node.drop_meta() {
			field if is_field(field) => holder_path(field).is_some_and(|path| self.variables.contains(&path)),
			Node::Symbol(name) => self.functions.contains(name) || self.variables.contains(name),
			Node::Key(choice, Op::Else, otherwise) => self.is_function_value(otherwise) || self.is_function_value(choice),
			Node::Key(_, Op::Then, chosen) => self.is_function_value(chosen),
			Node::Key(list, Op::Hash, _) => self.is_function_list(list),
			Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.returning.contains(name) || self.variables.contains(name)) => true,
			Node::List(items, _, _) if matches!(items.as_slice(), [marker, _, _, ..] if matches!(marker.drop_meta(), Node::Symbol(word) if word == crate::wasp_parser::TRY_MARKER)) => {
				self.is_function_value(&items[1]) || self.is_function_value(&items[2])
			}
			// a block is worth its last statement: `{fallback}`
			block @ Node::List(..) if !std::ptr::eq(tail(block), block) => self.is_function_value(tail(block)),
			other => arrow_lambda(other).is_some() || as_closure_new(other).is_some(),
		}
	}

	/// A list of function values: a literal holding one, another such list, one with more appended (`out + [f]`, what
	/// `xs.map(n => (x => x + n))` builds), a signal's listeners, or a block that ends in one
	fn is_function_list(&self, node: &Node) -> bool {
		match node.drop_meta() {
			field if is_field(field) => holder_path(field).is_some_and(|path| self.lists.contains(&path)),
			Node::List(items, Bracket::Square, _) => items.iter().any(|item| self.is_function_value(item)),
			Node::Symbol(name) => self.lists.contains(name),
			Node::Key(left, Op::Add, right) => self.is_function_list(left) || self.is_function_list(right),
			// a signal's listeners (`listeners of x`, lowering/signal_values.rs)
			Node::List(items, Bracket::Round, _) if items.first().is_some_and(|word| word.drop_meta().name() == crate::wasm_emitter::cells::SIGNAL_LISTENERS) => true,
			block @ Node::List(..) if !std::ptr::eq(tail(block), block) => self.is_function_list(tail(block)),
			_ => false,
		}
	}

	/// `for f in fs {…}` over a list of function values: f holds one
	fn loop_variable_over_functions<'a>(&self, node: &'a Node) -> Option<&'a str> {
		let Node::List(items, _, _) = node else { return None };
		let [keyword, variable, in_word, list, _body] = items.as_slice() else { return None };
		let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(name) if name == word);
		match variable.drop_meta() {
			Node::Symbol(variable) if is_word(keyword, FOR_WORD) && is_word(in_word, IN_WORD) && self.is_function_list(list) => Some(variable),
			_ => None,
		}
	}

	fn settle(&mut self, context: &Context, program: &Node) {
		loop {
			let returning: HashSet<String> = context.user_functions.values().filter(|function| self.is_function_value(tail(&function.body))).map(|function| function.name.clone()).collect();
			let (mut variables, mut lists) = (HashSet::new(), HashSet::new());
			program.visit(&mut |node| {
				if let Some(variable) = self.loop_variable_over_functions(node) {
					variables.insert(variable.to_string());
				}
				// `out.add(f)`, what `xs.map(n => (x => x + n))` builds: out holds functions
				if let Node::Key(list, Op::Dot, call) = node {
					if let (Node::Symbol(name), Node::List(items, _, _)) = (list.drop_meta(), call.drop_meta()) {
						if let [method, value] = items.as_slice() {
							if APPEND_METHODS.contains(&method.drop_meta().name().as_str()) && self.is_function_value(value) {
								lists.insert(name.clone());
							}
						}
					}
				}
				// `fs += [f]` appends as `fs = fs + [f]` does
				let Node::Key(target, op @ (Op::Assign | Op::Define | Op::AddAssign), value) = node else { return };
				let Some(name) = holder_path(target) else { return };
				if self.functions.contains(&name) {
					return;
				}
				if *op != Op::AddAssign {
					self.record_holder(&name, value, &mut variables, &mut lists);
				} else if self.is_function_list(value) {
					lists.insert(name);
				}
			});
			if returning == self.returning && variables == self.variables && lists == self.lists {
				return;
			}
			self.returning = returning;
			self.variables = variables;
			self.lists = lists;
		}
	}
}

impl FunctionValues {
	/// The variable or field `path` assigned `value`: a function value, a list of them, or an object whose fields hold them
	/// (`s = {fs: [x => x * 2]}`: s.fs is a list of function values)
	fn record_holder(&self, path: &str, value: &Node, variables: &mut HashSet<String>, lists: &mut HashSet<String>) {
		if self.is_function_value(value) {
			variables.insert(path.to_string());
		}
		if self.is_function_list(value) {
			lists.insert(path.to_string());
		}
		for (field, value) in crate::library_words::object_entries(value).unwrap_or_default() {
			self.record_holder(&format!("{path}.{field}"), &value, variables, lists);
		}
	}
}

/// A field of a variable: `s.f`, `s#("f"+1)`
fn is_field(node: &Node) -> bool {
	holder_path(node).is_some_and(|path| path.contains('.'))
}

/// Methods that append one value to a list variable (analyzer APPEND_METHODS)
const APPEND_METHODS: [&str; 3] = ["add", "append", "push"];

/// The parameter names of a definition `name(params) := body`, or `it` for `name := body` with `it`
fn definition_parameters(node: &Node, functions: &HashSet<String>) -> Option<Vec<String>> {
	let Node::Key(head, Op::Define | Op::Assign, _) = node.drop_meta() else { return None };
	match head.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) => {
			let (name, params) = items.split_first()?;
			let Node::Symbol(name) = name.drop_meta() else { return None };
			functions.contains(name).then(|| params.iter().filter_map(parameter_name).collect())
		}
		Node::Symbol(name) if functions.contains(name) => Some(vec![IMPLICIT_PARAMETER.to_string()]),
		_ => None,
	}
}

fn parameter_name(param: &Node) -> Option<String> {
	match param.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, _, _) => parameter_name(name),
		_ => None,
	}
}

struct Lifting {
	functions: HashSet<String>,
	variables: HashSet<String>,
	/// Variables assigned a function value: `f(x)` calls the closure they hold
	function_variables: HashSet<String>,
	/// Functions whose body ends in a function value: `make_adder(2)(5)` calls their result
	returns_function: HashSet<String>,
	/// Variables assigned a list of function values: `(fs#2)(5)` and `fs#2(5)` call an item
	function_lists: HashSet<String>,
	lifted: Vec<Node>,
	/// The functions whose bodies the walk is in, innermost last, by their full names (`outer·inner`)
	enclosing: Vec<String>,
}

impl Lifting {
	/// A function name as written in the current body: the nested `outer·inner` before a top-level `inner`
	fn function_named(&self, name: &str) -> Option<String> {
		let nested = self.enclosing.iter().rev().map(|outer| crate::analyzer::qualify_nested_name(Some(outer), name));
		nested.chain(std::iter::once(name.to_string())).find(|candidate| self.functions.contains(candidate))
	}

	fn is_variable(&self, name: &str, bound: &HashSet<String>) -> bool {
		bound.contains(name) || (self.variables.contains(name) && !self.functions.contains(name))
	}

	/// A parameter, or a variable assigned a function value
	fn may_hold_function(&self, name: &str, bound: &HashSet<String>) -> bool {
		bound.contains(name) || (self.function_variables.contains(name) && !self.functions.contains(name))
	}

	fn walk(&mut self, node: Node, bound: &HashSet<String>) -> Node {
		if let Some(params) = definition_parameters(&node, &self.functions) {
			let Node::Key(head, op, body) = node.drop_meta().clone() else { unreachable!("a definition") };
			let inner: HashSet<String> = bound.iter().cloned().chain(params).collect();
			let name = match head.drop_meta() {
				Node::List(items, _, _) => items.first().map(|name| name.drop_meta().name()).unwrap_or_default(),
				other => other.name(),
			};
			let full_name = self.function_named(&name).unwrap_or(name);
			self.enclosing.push(full_name);
			let body = self.walk(*body, &inner);
			let body = self.function_value(body, &inner);
			self.enclosing.pop();
			return Node::Key(head, op, Box::new(body));
		}
		if let Some(lambda) = arrow_lambda(&node) {
			return self.lift(lambda.params, lambda.body, bound);
		}
		// `data e` is the code as written: an object's function entry `f: x => x * a`, whose function blocks.rs defined
		if is_data(&node) {
			return node;
		}
		match node {
			Node::Key(target, op @ (Op::Assign | Op::Define), value) if matches!(target.drop_meta(), Node::Symbol(_)) => {
				let value = self.walk(*value, bound);
				// P83: `g = add` is no alias, add needs its arguments (`g = function add` is one)
				let bare_function = matches!(value.drop_meta(), Node::Symbol(name) if !bound.contains(name) && self.function_named(name).is_some()) && crate::closures::referenced_function(&value).is_none();
				let value = if bare_function { value } else { self.function_value(value, bound) };
				Node::Key(target, op, Box::new(value))
			}
			Node::Key(list, Op::Hash, index) if self.indexed_call(&list, &index).is_some() => {
				let (position, arguments) = self.indexed_call(&list, &index).expect("guarded");
				let arguments = arguments.into_iter().map(|argument| self.walk(argument, bound)).collect();
				closure_call(Node::Key(list, Op::Hash, Box::new(position)), arguments)
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.walk(*left, bound)), op, Box::new(self.walk(*right, bound))),
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.walk(item, bound)).collect();
				self.called_value(items, bracket, separator, bound)
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.walk(*node, bound)), data },
			other => other,
		}
	}

	/// `f(x)` with a variable `f`, `make_adder(2)(5)`, and function names given as arguments or list items
	fn called_value(&mut self, items: Vec<Node>, bracket: Bracket, separator: Separator, bound: &HashSet<String>) -> Node {
		let head_name = match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => Some(name.clone()),
			_ => None,
		};
		let is_call = bracket == Bracket::Round && separator == Separator::None;
		if let (true, Some(name)) = (is_call, &head_name) {
			let arguments: Vec<Node> = items[1..].iter().map(|argument| self.function_value(argument.clone(), bound)).collect();
			if self.may_hold_function(name, bound) {
				return closure_call(items[0].clone(), arguments);
			}
			if self.functions.contains(name) {
				return Node::List([vec![items[0].clone()], arguments].concat(), bracket, separator);
			}
		}
		// `make_adder(1)(2)`, `add(1)(2)(3)`: each group calls the closure the call before returned
		if let ([head, groups @ ..], Bracket::None, Separator::Space) = (items.as_slice(), &bracket, &separator) {
			let argument_lists: Option<Vec<Vec<Node>>> = groups.iter().map(|group| match group.drop_meta() {
				Node::List(arguments, Bracket::Round, _) => Some(arguments.clone()),
				_ => None,
			}).collect();
			if let (true, Some(argument_lists)) = (!groups.is_empty() && self.returns_closure(head, bound), argument_lists) {
				let function = match head.drop_meta() {
					Node::List(inner, Bracket::Round, _) if matches!(inner.as_slice(), [Node::Key(_, Op::Hash, _)]) => inner[0].clone(),
					_ => head.clone(),
				};
				return argument_lists.into_iter().fold(function, closure_call);
			}
		}
		if bracket == Bracket::Square {
			let items = items.into_iter().map(|item| self.function_value(item, bound)).collect();
			return Node::List(items, bracket, separator);
		}
		Node::List(items, bracket, separator)
	}

	/// A call of a function that returns a function value, or of a closure (which may), or `(fs#2)`: an item of a list
	/// of function values
	fn returns_closure(&self, node: &Node, bound: &HashSet<String>) -> bool {
		let Node::List(items, Bracket::Round, _) = node.drop_meta() else { return false };
		match items.first().map(Node::drop_meta) {
			Some(Node::Key(list, Op::Hash, _)) if items.len() == 1 => self.is_function_list(list),
			Some(Node::Symbol(name)) => self.returns_function.contains(name) || self.may_hold_function(name, bound) || closure_call_arity(name).is_some(),
			_ => false,
		}
	}

	fn is_function_list(&self, list: &Node) -> bool {
		holder_path(list).is_some_and(|path| self.function_lists.contains(&path))
	}

	/// `fs#2(5)` parses as `fs#(2*(5))`: of a list of function values it is the call of item 2 with (5)
	fn indexed_call(&self, list: &Node, index: &Node) -> Option<(Node, Vec<Node>)> {
		if !self.is_function_list(list) {
			return None;
		}
		let Node::Key(position, Op::Mul, group) = index.drop_meta() else { return None };
		match (position.drop_meta(), group.drop_meta()) {
			(Node::Number(_), Node::List(arguments, Bracket::Round, _)) => Some((position.as_ref().clone(), arguments.clone())),
			_ => None,
		}
	}

	/// A function name where a value is expected is the closure of that function
	fn function_value(&mut self, node: Node, bound: &HashSet<String>) -> Node {
		if let Some(target) = referenced_function(&node).and_then(|name| self.function_named(&name)) {
			return closure_new(&target, vec![]);
		}
		match node {
			Node::Symbol(ref name) if !bound.contains(name) && self.function_named(name).is_some() => closure_new(&self.function_named(name).expect("guarded"), vec![]),
			Node::Key(choice, op @ (Op::Then | Op::Else), chosen) => {
				let choice = if op == Op::Else { self.function_value(*choice, bound) } else { *choice };
				Node::Key(Box::new(choice), op, Box::new(self.function_value(*chosen, bound)))
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.function_value(*node, bound)), data },
			// `return function add`
			Node::List(mut items, bracket, separator) if is_return(&items) && gives_reference(&items[1]) => {
				let value = items.pop().expect("a returned value");
				items.push(self.function_value(value, bound));
				Node::List(items, bracket, separator)
			}
			// `{def add(t){…}; function add}`: a body ending in a function reference gives the function (a bare `add` there
			// needs its arguments, P82)
			Node::List(mut items, bracket @ (Bracket::Curly | Bracket::Round), separator) if items.last().is_some_and(gives_reference) => {
				let last = items.pop().expect("not empty");
				items.push(self.function_value(last, bound));
				Node::List(items, bracket, separator)
			}
			other => other,
		}
	}

	/// The lambda as the top-level function `closure_lambda_n(captured…, params…)`, and its closure
	fn lift(&mut self, params: Vec<String>, body: Node, bound: &HashSet<String>) -> Node {
		let inner: HashSet<String> = bound.iter().cloned().chain(params.iter().cloned()).collect();
		let body = self.walk(body, &inner);
		let body = self.function_value(body, &inner);
		let captured = self.captured(&body, &params, bound);
		let name = format!("{LIFTED_PREFIX}{}", self.lifted.len() + 1);
		let all_params = captured.iter().chain(&params).map(|param| Node::Symbol(param.clone()));
		let head = Node::List([vec![Node::Symbol(name.clone())], all_params.collect()].concat(), Bracket::Round, Separator::None);
		self.lifted.push(Node::Key(Box::new(head), Op::Define, Box::new(body)));
		self.functions.insert(name.clone());
		closure_new(&name, captured.into_iter().map(Node::Symbol).collect())
	}

	/// The variables of the enclosing scopes the body reads, in order of first use
	fn captured(&self, body: &Node, params: &[String], bound: &HashSet<String>) -> Vec<String> {
		let mut captured: Vec<String> = vec![];
		body.visit(&mut |node| {
			if let Node::Symbol(name) = node {
				if !params.contains(name) && self.is_variable(name, bound) && !captured.contains(name) {
					captured.push(name.clone());
				}
			}
		});
		captured
	}
}

/// The helpers `closure_call_n(f, a1…an)` the program calls, as user functions: a closure and n values of any kind (Data)
pub fn register_closure_calls(context: &mut Context, program: &Node) {
	context.closure_targets = closure_targets(program);
	context.closure_variable_targets = settle_closure_variable_targets(context, program);
	for arity in closure_call_arities(program) {
		let function = Param { used_as: Some(Kind::Function), ..Param::untyped("function") };
		let values = (1..=arity).map(|index| Param { used_as: Some(Kind::Data), ..Param::untyped(&format!("value{index}")) });
		let name = closure_call_name(arity);
		let params = std::iter::once(function).chain(values).collect();
		context.user_functions.insert(name.clone(), UserFunctionDef { name, params, body: Box::new(Node::Empty), return_kind: Kind::Data, tuple_kinds: vec![], func_index: None });
	}
}

/// Join return kinds; Data when they differ or the set is empty
fn join_return_kinds(kinds: impl IntoIterator<Item = Kind>) -> Kind {
	let mut kinds = kinds.into_iter();
	let first = kinds.next().unwrap_or(Kind::Data);
	if kinds.all(|kind| kind == first) { first } else { Kind::Data }
}

/// The closures a call of `arity` values may call
pub fn targets_of_arity(context: &Context, arity: usize) -> impl Iterator<Item = &UserFunctionDef> {
	context.closure_targets.iter().filter_map(move |(target, captured)| context.user_functions.get(target).filter(|function| function.params.len() == captured + arity))
}

/// A closure call returns what all closures of its arity return; Data (any Node) when they differ
pub fn closure_call_kind(name: &str, context: &Context, kinds: &HashMap<String, Kind>) -> Option<Kind> {
	let arity = closure_call_arity(name)?;
	Some(join_return_kinds(targets_of_arity(context, arity).map(|function| kinds.get(&function.name).copied().unwrap_or(function.return_kind))))
}

/// Result kind of `closure_call_n(callee, …)`: join only the targets `callee` may hold when known; else arity-wide
pub fn closure_call_site_kind(callee: Option<&str>, arity: usize, context: &Context, kinds: &HashMap<String, Kind>) -> Kind {
	if let Some(variable) = callee {
		if let Some(targets) = context.closure_variable_targets.get(variable) {
			if !targets.is_empty() {
				let returned = targets.iter().filter_map(|target| {
					let function = context.user_functions.get(target)?;
					Some(kinds.get(target).copied().unwrap_or(function.return_kind))
				});
				return join_return_kinds(returned);
			}
		}
	}
	closure_call_kind(&closure_call_name(arity), context, kinds).unwrap_or(Kind::Data)
}

/// `closure_new` target names a function value expression may evaluate to
fn closure_targets_of_value(node: &Node, held: &HashMap<String, HashSet<String>>, returning: &HashMap<String, HashSet<String>>) -> HashSet<String> {
	match node.drop_meta() {
		Node::Symbol(name) => held.get(name).cloned().unwrap_or_else(|| returning.get(name).cloned().unwrap_or_default()),
		other if as_closure_new(other).is_some() => {
			let (target, _) = as_closure_new(other).expect("guarded");
			HashSet::from([target.to_string()])
		}
		Node::Key(choice, Op::Else, otherwise) => {
			let mut targets = closure_targets_of_value(choice, held, returning);
			targets.extend(closure_targets_of_value(otherwise, held, returning));
			targets
		}
		Node::Key(_, Op::Then, chosen) => closure_targets_of_value(chosen, held, returning),
		Node::List(items, Bracket::Round, Separator::None) => {
			let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else {
				return if items.len() == 1 { closure_targets_of_value(&items[0], held, returning) } else { HashSet::new() };
			};
			if closure_call_arity(name).is_some() {
				let callee = items.get(1).map(|node| closure_targets_of_value(node, held, returning)).unwrap_or_default();
				return callee.into_iter().flat_map(|target| returning.get(&target).cloned().unwrap_or_default()).collect();
			}
			if name == CLOSURE_NEW {
				return as_closure_new(node).map(|(target, _)| HashSet::from([target.to_string()])).unwrap_or_default();
			}
			returning.get(name).cloned().unwrap_or_default()
		}
		block @ Node::List(..) if !std::ptr::eq(tail(block), block) => closure_targets_of_value(tail(block), held, returning),
		Node::Key(list, Op::Hash, _) => match list.drop_meta() {
			Node::Symbol(name) => held.get(name).cloned().unwrap_or_default(),
			Node::List(items, Bracket::Square, _) => items.iter().flat_map(|item| closure_targets_of_value(item, held, returning)).collect(),
			_ => HashSet::new(),
		},
		Node::List(items, Bracket::Square, _) => items.iter().flat_map(|item| closure_targets_of_value(item, held, returning)).collect(),
		_ => HashSet::new(),
	}
}

/// Which `closure_new` targets each variable may hold (assignment / if-else / calls that return a function)
fn settle_closure_variable_targets(context: &Context, program: &Node) -> HashMap<String, HashSet<String>> {
	let mut returning: HashMap<String, HashSet<String>> = context.user_functions.iter()
		.map(|(name, function)| (name.clone(), closure_targets_of_value(tail(&function.body), &HashMap::new(), &HashMap::new())))
		.filter(|(_, targets)| !targets.is_empty())
		.collect();
	let mut held: HashMap<String, HashSet<String>> = HashMap::new();
	let bodies: Vec<&Node> = std::iter::once(program).chain(context.user_functions.values().map(|function| function.body.as_ref())).collect();
	for _ in 0..=context.user_functions.len() + bodies.len() {
		let mut next = held.clone();
		for body in &bodies {
			body.visit(&mut |node| {
				if let Node::List(items, _, _) = node {
					if let [keyword, variable, in_word, list, _] = items.as_slice() {
						let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(name) if name == word);
						if is_word(keyword, FOR_WORD) && is_word(in_word, IN_WORD) {
							if let Node::Symbol(name) = variable.drop_meta() {
								let targets = closure_targets_of_value(list, &held, &returning);
								if !targets.is_empty() {
									next.entry(name.clone()).or_default().extend(targets);
								}
							}
						}
					}
				}
				let Node::Key(target, Op::Assign | Op::Define, value) = node else { return };
				let Node::Symbol(name) = target.drop_meta() else { return };
				let targets = closure_targets_of_value(value, &held, &returning);
				if !targets.is_empty() {
					next.entry(name.clone()).or_default().extend(targets);
				}
			});
		}
		let mut returning_now = returning.clone();
		for (name, function) in &context.user_functions {
			let targets = closure_targets_of_value(tail(&function.body), &next, &returning_now);
			if !targets.is_empty() {
				returning_now.insert(name.clone(), targets);
			}
		}
		if next == held && returning_now == returning {
			return next;
		}
		held = next;
		returning = returning_now;
	}
	held
}

/// The fast path of closure calls: when every closure of an arity has one signature of numbers (Ints and Floats, the
/// same for all), its `closure_call_n` passes and returns them as i64 / f64 and the entries take them as they are, no
/// Node box per argument and result (wasm_emitter/closures.rs `typed_entry`)
pub fn type_closure_calls(context: &mut Context) {
	let helpers: Vec<(String, usize)> = context.user_functions.keys().filter_map(|name| closure_call_arity(name).map(|arity| (name.clone(), arity))).collect();
	for (name, arity) in helpers {
		let signatures: Vec<Vec<Kind>> = targets_of_arity(context, arity).map(|function| {
			let captured = function.params.len() - arity;
			function.params[captured..].iter().map(crate::analyzer::param_kind).chain(std::iter::once(function.return_kind)).collect()
		}).collect();
		let Some(signature) = signatures.first().cloned() else { continue };
		let numbers = signature.iter().all(|kind| matches!(kind, Kind::Int | Kind::Float));
		if numbers && signatures.iter().all(|other| *other == signature) {
			let helper = context.user_functions.get_mut(&name).expect("a closure call helper");
			helper.params.iter_mut().skip(1).zip(&signature).for_each(|(param, kind)| param.used_as = Some(*kind));
			helper.return_kind = signature[arity];
		}
	}
}

/// Is `closure_call_n` on the fast path: its values are numbers (type_closure_calls)
pub fn is_typed_closure_call(function: &UserFunctionDef) -> bool {
	closure_call_arity(&function.name).is_some() && function.params.len() > 1
		&& function.params[1..].iter().all(|param| matches!(param.used_as, Some(Kind::Int | Kind::Float)))
}

/// `data e` (blocks.rs)
fn is_data(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && items[0].drop_meta().name() == crate::blocks::DATA_WORD)
}
