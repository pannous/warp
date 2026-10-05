//! Blocks, stage 1 (wiki/charged.md sections 2, 4, 5): `x : e` of a computed expression binds x to the uncharged block e.
//! `x!` and `x!!` run it where they are written (the block is a constant, so it is inlined there and its names resolve at
//! the `!`), a bare `x` is the block as data (`x : 1+2; x` shows 1+2), and `x` where a value is needed (`x + 1`) is a type
//! error with the fix. A literal or a plain word after `:` is simply that value (`age: 3`), types and objects keep their
//! meaning (`x : int`, `person: {…}`). A computed `:` gets a got-it warning where it is written. `x = …` ends the block.

use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::mutation::bang_of;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

/// The data prefix a bare block name becomes: the block as it is written (list_emitter emit_quoted)
const DATA_WORD: &str = "data";
/// The got-it topic of `x : a+b`, which keeps a block where a value may have been meant
const BLOCK_TOPIC: &str = "uncharged-block";

pub fn lower(program: Node) -> Node {
	let mut lowering = Blocks { blocks: HashMap::new() };
	lowering.statement(program)
}

struct Blocks {
	/// The blocks in scope: name → the expression
	blocks: HashMap<String, Node>,
}

/// `x : a+b` of a computed expression: the name and the block
pub(crate) fn uncharged(node: &Node) -> Option<(String, Node)> {
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

fn type_error(name: &str, block: &Node) -> Node {
	let written = crate::normalize::operand_text(block);
	crate::node::error(&format!("{name} is a block ({written}), no value: run it with {name}!, or write {name} = {written} for its value"))
}

impl Blocks {
	/// A statement: only here `x : e` binds a block (a branch `c : x`, a case `1: 10` or an entry is no statement)
	fn statement(&mut self, node: Node) -> Node {
		if let Some((name, block)) = uncharged(&node) {
			let written = crate::normalize::operand_text(&block);
			let question = Ask::new(BLOCK_TOPIC, format!("{name} keeps the block {written}, it runs only at {name}!; write {name} = {written} for its value"),
				vec![reading("a block", &format!("{name} : {written}")), reading("its value", &format!("{name} = {written}"))], Fallback::Warning)
				.written(&format!("{name} : {written}")).at_node(&node);
			if let Err(error) = ask(&question) {
				return error;
			}
			let block = self.rewrite(block);
			self.blocks.insert(name, block);
			return Node::Empty;
		}
		self.rewrite(node)
	}

	fn rewrite(&mut self, node: Node) -> Node {
		if let Some((name, _)) = bang_of(&node) {
			if let Some(block) = self.blocks.get(&name) {
				return Node::List(vec![block.clone()], Bracket::Round, Separator::None);
			}
			return node;
		}
		match node {
			Node::Symbol(name) if self.blocks.contains_key(&name) => {
				Node::List(vec![Node::Symbol(DATA_WORD.to_string()), self.blocks[&name].clone()], Bracket::None, Separator::Space)
			}
			// `x = …` ends the block named x
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let value = self.rewrite(*value);
				if let Node::Symbol(name) = target.drop_meta() {
					self.blocks.remove(name);
				}
				Node::Key(target, op, Box::new(value))
			}
			// a block where a value is needed
			Node::Key(left, op, right) if op.is_arithmetic() || op.is_comparison() => {
				for operand in [&left, &right] {
					if let Node::Symbol(name) = operand.drop_meta() {
						if let Some(block) = self.blocks.get(name) {
							return type_error(name, block);
						}
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
