//! Blocks (wiki/charged.md sections 2, 4, 5; notes/blocks.md). Stage 1: `x : e` of a computed expression binds x to the
//! uncharged block e. `x!` and `x!!` run it where they are written (the block is a constant, so it is inlined there and its
//! names resolve at the `!`), a bare `x` is the block as data (`x : 1+2; x` shows 1+2), and `x` where a value is needed
//! (`x + 1`) is a type error with the fix. A literal or a plain word after `:` is simply that value (`age: 3`), types and
//! objects keep their meaning (`x : int`, `person: {…}`). A computed `:` gets a got-it warning where it is written.
//! `x = …` ends the block.
//! Stage 2: `x = {statements}` is a block too (no warning: braces say so), an object's computed `key: value` entry is an
//! uncharged block (`o.s1!` runs it, `o.s1 + 1` is the type error, the object holds it as data), and `obj!` of an object
//! literal runs it as code: each `key: value` is the call `key(value)` (`help!`). `key := value` entries are untouched (P71).
//! Stage 3: `x = code e` / `x = block e` are blocks, `y = data e; y!` runs with a got-it warning ("running data as code"),
//! and a function with a `block` parameter (`when_not(c, body:block) := if not c { body! }`) is expanded at every call:
//! its other parameters bound to fresh names, `body!` the argument's code, so names resolve where the call is written.
//! Any `!` left (known only at run time) is mutation.rs's: a name unwraps, any other expression is a loud error.

use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::mutation::bang_target;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

/// The data prefix a bare block name becomes: the block as it is written (list_emitter emit_quoted)
const DATA_WORD: &str = "data";
/// The got-it topic of `x : a+b`, which keeps a block where a value may have been meant
const BLOCK_TOPIC: &str = "uncharged-block";
const IT: &str = "it";
/// Prefixes of a block: `code e`, `block e` (synonyms, wiki/charged.md section 4)
const BLOCK_WORDS: [&str; 2] = ["code", "block"];
/// The got-it topic of `!` on data
const DATA_RUN_TOPIC: &str = "data-as-code";
/// `body·m0`: a parameter of an expanded block function, bound once
const EXPANSION_SEPARATOR: &str = "·m";

pub fn lower(program: Node) -> Node {
	let mut functions = HashMap::new();
	program.visit(&mut |node| {
		if let Some((name, function)) = block_function(node) {
			functions.insert(name, function);
		}
	});
	let mut lowering = Blocks { blocks: HashMap::new(), objects: HashMap::new(), data: HashMap::new(), functions, expansions: 0 };
	lowering.statement(program)
}

/// A function with a `block` parameter: its parameters (name, whether a block) and body
#[derive(Clone)]
struct BlockFunction {
	parameters: Vec<(String, bool)>,
	body: Node,
}

/// `f(c, body:block) := …`: a definition with at least one `block` (or `code`) parameter
fn block_function(node: &Node) -> Option<(String, BlockFunction)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
	let (name, parameters) = items.split_first()?;
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let parameters: Vec<(String, bool)> = parameters.iter().map(|parameter| match parameter.drop_meta() {
		Node::Symbol(name) => Some((name.clone(), false)),
		Node::Key(name, Op::Colon, kind) => Some((name.name(), matches!(kind.drop_meta(), Node::Symbol(word) if BLOCK_WORDS.contains(&word.as_str())))),
		_ => None,
	}).collect::<Option<_>>()?;
	parameters.iter().any(|(_, block)| *block).then(|| (name.clone(), BlockFunction { parameters, body: body.as_ref().clone() }))
}

/// The prefixed value `code e` / `data e`: the prefix and e
fn prefixed<'a>(value: &'a Node, words: &[&str]) -> Option<&'a Node> {
	let Node::List(items, Bracket::None, Separator::Space) = value.drop_meta() else { return None };
	match items.as_slice() {
		[prefix, expression] if matches!(prefix.drop_meta(), Node::Symbol(word) if words.contains(&word.as_str())) => Some(expression),
		_ => None,
	}
}

/// In a body: `param!` the argument's code (a group), a bare `param` the argument as data
fn substitute_block(node: Node, parameter: &str, argument: &Node) -> Node {
	if let Some((Node::Symbol(name), _)) = bang_target(&node) {
		if name == parameter {
			return Node::List(vec![argument.clone()], Bracket::Round, Separator::None);
		}
	}
	match node {
		Node::Symbol(name) if name == parameter => Node::List(vec![Node::Symbol(DATA_WORD.to_string()), argument.clone()], Bracket::None, Separator::Space),
		Node::Key(left, op, right) => Node::Key(Box::new(substitute_block(*left, parameter, argument)), op, Box::new(substitute_block(*right, parameter, argument))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| substitute_block(item, parameter, argument)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(substitute_block(*node, parameter, argument)), data },
		other => other,
	}
}

struct Blocks {
	/// The blocks in scope: a name (`x`) or a field (`o.s1`) → the expression
	blocks: HashMap<String, Node>,
	/// Object literals in scope, by name: their entries, to run the object as code (`help!`)
	objects: HashMap<String, Vec<(Node, Node)>>,
	/// Names holding `data e`: e, run only with a warning
	data: HashMap<String, Node>,
	/// Functions with a `block` parameter, expanded at every call
	functions: HashMap<String, BlockFunction>,
	expansions: usize,
}

/// `x : a+b` of a computed expression: the name and the block
fn uncharged(node: &Node) -> Option<(String, Node)> {
	let Node::Key(target, Op::Colon, value) = node.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	is_computed(value).then(|| (name.clone(), value.as_ref().clone()))
}

/// Code that computes: an operation or a call, not a literal, a word, a type or an object/list literal
fn is_computed(value: &Node) -> bool {
	match value.drop_meta() {
		// a type expression declares: `int[100]`, `100 * int`, `3 * char`
		Node::Key(left, op, right) => {
			let is_type = |side: &Node| matches!(side.drop_meta(), Node::Symbol(word)
				if crate::analyzer::type_word_kind(word).is_some() || crate::analyzer::plural_element_type(word).is_some());
			!is_type(left) && !is_type(right) && !matches!(op, Op::Colon | Op::Assign | Op::Define)
		}
		// a call `f(x)`; `100 int` (a typed array) and other spaced words stay as they are
		Node::List(items, Bracket::Round, Separator::None) => items.len() > 1,
		_ => false,
	}
}

/// The entries of an object literal `{k: v, k = v}`: every item names a key
fn object_entries(value: &Node) -> Option<&Vec<Node>> {
	let Node::List(items, Bracket::Curly, _) = value.drop_meta() else { return None };
	let is_entry = |item: &Node| matches!(item.drop_meta(), Node::Key(key, Op::Colon | Op::Assign | Op::Define, _) if matches!(key.drop_meta(), Node::Symbol(_)));
	(!items.is_empty() && items.iter().all(is_entry)).then_some(items)
}

/// `{1+2}`, `{a = 1; a*2}` assigned: statements in braces (several, or one computed expression), no object, no data
/// list (`{1 2}`) and no lambda (`{it*2}`, `{x => x+1}`)
fn statement_block(value: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Curly, separator) = value.drop_meta() else { return None };
	let statements = matches!(separator, Separator::Semicolon | Separator::Newline) && items.len() > 1;
	let lambda = items.iter().any(|item| matches!(item.drop_meta(), Node::Key(_, Op::Arrow | Op::FatArrow, _)));
	if items.is_empty() || object_entries(value).is_some() || crate::wasp_parser::mentions(value, IT) || lambda {
		return None;
	}
	if !statements && !matches!(items.as_slice(), [single] if is_computed(single)) {
		return None;
	}
	Some(match items.as_slice() {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::Round, separator.clone()),
	})
}

/// Expansions of block functions in one program: a function expanding itself without end stops here
const MAX_EXPANSIONS: usize = 1000;

/// `node` with its innermost value replaced, keeping the meta information around it (`@unit("cm") {x:1}`)
fn with_inner(node: &Node, inner: Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_inner(node, inner)), data: data.clone() },
		_ => inner,
	}
}

/// `x` or `o.s1`: the key of a block
fn block_key(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(object, Op::Dot, field) => match (object.drop_meta(), field.drop_meta()) {
			(Node::Symbol(object), Node::Symbol(field)) => Some(format!("{object}.{field}")),
			_ => None,
		},
		_ => None,
	}
}

fn type_error(name: &str, block: &Node) -> Node {
	let written = crate::normalize::operand_text(block);
	crate::node::error(&format!("{name} is a block ({written}), no value: run it with {name}!, or write {name} = {written} for its value"))
}

fn call(function: &Node, argument: Node) -> Node {
	Node::List(vec![function.clone(), argument], Bracket::Round, Separator::None)
}

impl Blocks {
	/// The got-it warning of a computed `:` where it is written: `name` keeps a block where a value may have been meant
	fn warn_uncharged(&self, name: &str, block: &Node, at: &Node) -> Result<(), Node> {
		let written = crate::normalize::operand_text(block);
		let question = Ask::new(BLOCK_TOPIC, format!("{name} keeps the block {written}, it runs only at {name}!; write {name} = {written} for its value"),
			vec![reading("a block", &format!("{name} : {written}")), reading("its value", &format!("{name} = {written}"))], Fallback::Warning)
			.written(&format!("{name} : {written}")).at_node(at);
		ask(&question).map(|_| ())
	}

	/// A statement: only here `x : e` binds a block (a branch `c : x`, a case `1: 10` or an entry is no statement)
	fn statement(&mut self, node: Node) -> Node {
		if let Some((name, block)) = uncharged(&node) {
			if let Err(error) = self.warn_uncharged(&name, &block, &node) {
				return error;
			}
			let block = self.rewrite(block);
			self.blocks.insert(name, block);
			return Node::Empty;
		}
		// a function with a `block` parameter exists only in its expansions
		if block_function(&node).is_some() {
			return Node::Empty;
		}
		if let Node::Key(target, Op::Assign, value) = node.drop_meta() {
			if let Node::Symbol(name) = target.drop_meta() {
				// `x = code 1+2`, `x = block e`: a block
				if let Some(block) = prefixed(value, &BLOCK_WORDS) {
					let block = self.rewrite(block.clone());
					self.forget(name);
					self.blocks.insert(name.clone(), block);
					return Node::Empty;
				}
				// `y = data 1+2`: data, which `y!` runs with a warning
				if let Some(data) = prefixed(value, &[DATA_WORD]) {
					let data = data.clone();
					let name = name.clone();
					let lowered = self.rewrite(node);
					self.data.insert(name, data);
					return lowered;
				}
				// `x = {1+2}`: statements in braces are a block
				if let Some(block) = statement_block(value) {
					let block = self.rewrite(block);
					self.forget(name);
					self.blocks.insert(name.clone(), block);
					return Node::Empty;
				}
				// a plain object stays as it is; its entries can run as code (`help!`)
				let rewritten = |entry: &Node| uncharged(entry).is_some() || matches!(entry.drop_meta(), Node::Key(_, Op::Assign, _));
				if let Some(entries) = object_entries(value).filter(|entries| !entries.iter().any(rewritten)) {
					let pairs: Vec<(Node, Node)> = entries.iter().filter_map(|entry| match entry.drop_meta() {
						Node::Key(key, Op::Colon, value) => Some((key.as_ref().clone(), value.as_ref().clone())),
						_ => None,
					}).collect();
					let name = name.clone();
					let lowered = self.rewrite(node);
					self.objects.insert(name, pairs);
					return lowered;
				}
				// `o = {s1: a+b, …}`: computed entries are blocks the object holds as data, `s3 = …` entries values
				let rewritten = |entry: &Node| uncharged(entry).is_some() || matches!(entry.drop_meta(), Node::Key(_, Op::Assign, _));
				if let Some(entries) = object_entries(value).filter(|entries| entries.iter().any(rewritten)) {
					let name = name.clone();
					self.forget(&name);
					let mut lowered = vec![];
					for entry in entries.clone() {
						lowered.push(match uncharged(&entry) {
							Some((field, block)) => {
								if let Err(error) = self.warn_uncharged(&field, &block, &entry) {
									return error;
								}
								let block = self.rewrite(block);
								self.blocks.insert(format!("{name}.{field}"), block.clone());
								let data = Node::List(vec![Node::Symbol(DATA_WORD.to_string()), block], Bracket::None, Separator::Space);
								Node::Key(Box::new(Node::Symbol(field)), Op::Colon, Box::new(data))
							}
							None => match entry.drop_meta() {
								// `s3 = a+b`: a value entry, evaluated now
								Node::Key(field, Op::Assign, value) => {
									let value = self.rewrite(value.as_ref().clone());
									Node::Key(field.clone(), Op::Colon, Box::new(Node::List(vec![value], Bracket::Round, Separator::None)))
								}
								_ => self.rewrite(entry),
							},
						});
					}
					let pairs = lowered.iter().filter_map(|entry| match entry.drop_meta() {
						Node::Key(key, Op::Colon, value) => Some((key.as_ref().clone(), value.as_ref().clone())),
						_ => None,
					}).collect();
					self.objects.insert(name.clone(), pairs);
					let separator = match value.drop_meta() { Node::List(_, _, separator) => separator.clone(), _ => Separator::Space };
					return Node::Key(target.clone(), Op::Assign, Box::new(with_inner(value, Node::List(lowered, Bracket::Curly, separator))));
				}
			}
		}
		self.rewrite(node)
	}

	/// A name given a new value: its blocks and object entries are gone
	fn forget(&mut self, name: &str) {
		self.blocks.remove(name);
		self.objects.remove(name);
		self.data.remove(name);
		let prefix = format!("{name}.");
		self.blocks.retain(|key, _| !key.starts_with(&prefix));
	}

	/// `x!`, `o.s1!`, `help!`: the block inlined where it runs, or the object run as code (each `key: value` the call
	/// `key(value)`); None for anything else (mutation.rs decides)
	fn run(&self, target: &Node) -> Option<Node> {
		let key = block_key(target)?;
		if let Some(block) = self.blocks.get(&key) {
			return Some(Node::List(vec![block.clone()], Bracket::Round, Separator::None));
		}
		if let Some(data) = self.data.get(&key) {
			let written = crate::normalize::operand_text(data);
			let question = Ask::new(DATA_RUN_TOPIC, format!("running data as code: {key} is data ({written})"),
				vec![reading("run it", &format!("{key}!")), reading("a block", &format!("{key} = code {written}"))], Fallback::Warning).written(&format!("{key}!"));
			if let Err(error) = ask(&question) {
				return Some(error);
			}
			return Some(Node::List(vec![data.clone()], Bracket::Round, Separator::None));
		}
		let entries = self.objects.get(&key)?;
		let calls = entries.iter().map(|(function, argument)| call(function, argument.clone())).collect();
		Some(Node::List(calls, Bracket::Round, Separator::Semicolon))
	}

	/// `f(a, b)` of a function with a `block` parameter: its body with the block arguments' code and the other parameters bound
	fn expand(&mut self, items: &[Node]) -> Option<Node> {
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else { return None };
		let function = self.functions.get(name)?.clone();
		if function.parameters.len() != arguments.len() || self.expansions > MAX_EXPANSIONS {
			return None;
		}
		let number = self.expansions;
		self.expansions += 1;
		let mut statements = vec![];
		let mut body = function.body;
		for ((parameter, is_block), argument) in function.parameters.iter().zip(arguments) {
			if *is_block {
				body = substitute_block(body, parameter, argument);
			} else {
				let bound = Node::Symbol(format!("{parameter}{EXPANSION_SEPARATOR}{number}"));
				statements.push(Node::Key(Box::new(bound.clone()), Op::Assign, Box::new(self.rewrite(argument.clone()))));
				body = crate::library_words::substitute(body, parameter, &bound);
			}
		}
		statements.push(self.rewrite(body));
		Some(Node::List(statements, Bracket::Round, Separator::Semicolon))
	}

	fn rewrite(&mut self, node: Node) -> Node {
		if let Some((target, _)) = bang_target(&node) {
			return self.run(&target).unwrap_or(node);
		}
		if let Node::List(items, Bracket::Round, Separator::None) = node.drop_meta() {
			if let Some(expanded) = self.expand(items) {
				return expanded;
			}
		}
		// `o.s1!`: the parser marks the field it was reading (as for `x.upper!`)
		if let Node::Key(object, Op::Dot, field) = node.drop_meta() {
			if let Some((field, _)) = bang_target(field) {
				if let Some(run) = self.run(&Node::Key(object.clone(), Op::Dot, Box::new(field))) {
					return run;
				}
			}
		}
		match node {
			Node::Symbol(name) if self.blocks.contains_key(&name) => {
				Node::List(vec![Node::Symbol(DATA_WORD.to_string()), self.blocks[&name].clone()], Bracket::None, Separator::Space)
			}
			// `x = …` ends the block named x
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let value = self.rewrite(*value);
				if let Node::Symbol(name) = target.drop_meta() {
					self.forget(name);
				}
				Node::Key(target, op, Box::new(value))
			}
			// a block where a value is needed
			Node::Key(left, op, right) if op.is_arithmetic() || op.is_comparison() => {
				for operand in [&left, &right] {
					if let Some(block) = block_key(operand).and_then(|key| self.blocks.get(&key).map(|block| (key, block))) {
						return type_error(&block.0, block.1);
					}
				}
				Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right)))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
				Node::List(items.into_iter().map(|item| self.statement(item)).collect(), bracket, separator)
			}
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}
}
