//! Blocks compiled and run at run time (`eval_block`): tasks and the browser hand a block with its captured values
//! to a fresh module, cached by source.

use crate::errors::run_time::*;
use crate::lowering::nodes::children_rewritten;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::pipeline::*;
#[cfg(feature = "native")]
use crate::wasm_emitter::emit_module;

#[cfg(feature = "native")]
thread_local! {
	static BLOCK_MODULES: std::cell::RefCell<std::collections::HashMap<String, CompiledModule>> = std::cell::RefCell::new(std::collections::HashMap::new());
	static COMPILED_BLOCKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The block programs compiled on this thread, by their text; past BLOCK_MODULES_KEPT they start over
#[cfg(feature = "native")]
pub(crate) const BLOCK_MODULES_KEPT: usize = 256;

/// Run a block the program built at run time (the host's run_block, notes/runtime_eval.md): `names` (one text, the
/// names separated by spaces) are bound to `values` first, the values the program had where it ran the block, then
/// the program's function `definitions` follow. Only the names the block or the definitions mention are bound.
/// The block gets no capability but pure libm, wherever its code came from (user decision: pure by default)
/// `Err` is the message the run fails with, naming the block.
pub fn eval_block(block: Node, names: &Node, values: &Node, definitions: &Node) -> Result<Node, String> {
	let written = block.serialize().trim().to_string();
	if block.drop_meta() == &Node::Empty {
		return Err(UNWRAPPED_EMPTY.to_string()); // `x!` forces: ø is the optional's loud error (P73)
	}
	let names: Vec<String> = match names.drop_meta() {
		Node::Text(names) => names.split_whitespace().map(str::to_string).collect(),
		_ => vec![],
	};
	let block = spaced(block);
	let definitions: Vec<Node> = definitions.as_items().into_iter().map(spaced).collect();
	let mentioned = |name: &str| [&block].into_iter().chain(&definitions).any(|part| names_or_interpolates(part, name));
	let bound: Vec<(String, Node)> =
		names.into_iter().zip(values.as_items()).filter(|(name, _)| mentioned(name)).collect();
	let program_of = |bindings: Vec<Node>| {
		let statements: Vec<Node> =
			bindings.into_iter().chain(definitions.iter().cloned()).chain([block.clone()]).collect();
		Node::List(statements, Bracket::None, Separator::Newline)
	};
	let as_written =
		program_of(bound.iter().map(|(name, value)| assignment(name, as_written(value.clone()))).collect());
	let result = match crate::analyzer::effects::EffectReport::of(&as_written)
		.denied(&crate::analyzer::effects::Capability::GRANTED_UNTRUSTED)
	{
		Some((external, capability)) => crate::node::error(&format!(
			"a block run at run time is pure: {external} needs the {} capability, which it does not get",
			capability.name()
		)),
		None => run_block_program(bound, program_of),
	};
	match result.drop_meta() {
		Node::Error(message) => Err(format!("the block {written} failed: {}", message.drop_meta())),
		_ => Ok(result),
	}
}

pub(crate) fn assignment(name: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Assign, Box::new(value))
}

/// A value written into the block's program: a number, text or character as itself, anything else as data
pub(crate) fn as_written(value: Node) -> Node {
	match value.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::Empty => value,
		_ => Node::List(
			vec![Node::Symbol(crate::lowering::blocks::DATA_WORD.to_string()), value],
			Bracket::None,
			Separator::Space,
		),
	}
}

/// Does `node` mention `name`: as a word, or inside a text (an interpolation)
pub(crate) fn names_or_interpolates(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| {
		found |= match part {
			Node::Symbol(word) => word == name,
			Node::Text(text) => text.contains(name),
			_ => false,
		}
	});
	found
}

/// Natively the block is compiled as a function of the numbers it reads: each is `block·value(i) as int` (or `float`),
/// exact ratios and big Ints included,
/// which the host gives from this run's values, so the same block with other numbers is the same program, compiled
/// once per thread (BLOCK_MODULES) and machine code once per machine (run/module_cache.rs). Other values are written in
#[cfg(feature = "native")]
pub(crate) fn run_block_program(bound: Vec<(String, Node)>, program_of: impl Fn(Vec<Node>) -> Node) -> Node {
	use crate::extensions::numbers::Number;
	use crate::host::tasks::TaskValue;
	let mut numbers = vec![];
	let bindings = bound
		.into_iter()
		.map(|(name, value)| {
			// an exact ratio (`0.5` is 1/2) or big Int is an Int to the compiler: a handle the host composes (tasks.rs Builders)
			let (number, type_word) = match value.drop_meta() {
				Node::Number(Number::Float(x)) => (TaskValue::Float(*x), "float"),
				Node::Number(Number::Int(_) | Number::BigInt(_) | Number::Quotient(..) | Number::BigQuotient(_)) => {
					match TaskValue::of(&value) {
						Ok(number) => (number, "int"),
						Err(_) => return assignment(&name, as_written(value)),
					}
				}
				_ => return assignment(&name, as_written(value)),
			};
			let read = Node::List(
				vec![
					Node::Symbol(crate::host::BLOCK_VALUE.to_string()),
					Node::Number(Number::Int(numbers.len() as i64)),
				],
				Bracket::Round,
				Separator::None,
			);
			numbers.push(number);
			assignment(&name, Node::Key(Box::new(read), Op::As, Box::new(Node::Symbol(type_word.to_string()))))
		})
		.collect();
	let program = program_of(bindings);
	crate::host::with_block_values(numbers, || crate::errors::diagnostic::in_program_mode(program, run_cached_block))
}

#[cfg(not(feature = "native"))]
pub(crate) fn run_block_program(bound: Vec<(String, Node)>, program_of: impl Fn(Vec<Node>) -> Node) -> Node {
	eval_parsed(program_of(bound.into_iter().map(|(name, value)| assignment(&name, as_written(value))).collect()), "")
}

/// How many block programs this thread compiled (the cache's misses)
#[cfg(feature = "native")]
pub fn compiled_blocks() -> usize {
	COMPILED_BLOCKS.with(std::cell::Cell::get)
}

#[cfg(feature = "native")]
pub(crate) fn run_cached_block(program: Node) -> Node {
	let key = program.serialize();
	let module = match BLOCK_MODULES.with(|modules| modules.borrow().get(&key).cloned()) {
		Some(module) => module,
		None => {
			COMPILED_BLOCKS.with(|count| count.set(count.get() + 1));
			let module = match lower_for_emission(program).map(|node| emit_module(&node)) {
				Ok(Ok(module)) => module,
				Ok(Err(type_error)) => return type_error,
				Err(final_value) => return final_value,
			};
			BLOCK_MODULES.with(|modules| {
				let mut modules = modules.borrow_mut();
				if modules.len() >= BLOCK_MODULES_KEPT {
					modules.clear();
				}
				modules.insert(key, module.clone());
			});
			module
		}
	};
	run_module(module)
}

/// A phrase read back from a Node of the program (`data 2*3`, `print x`) has lost its spaces: an unbracketed list is a
/// spaced phrase again, as the parser made it
pub(crate) fn spaced(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::None) => {
			Node::List(items.into_iter().map(spaced).collect(), Bracket::None, Separator::Space)
		}
		other => children_rewritten(other, spaced),
	}
}

/// A block's value the host has no constructor for in the running module yet
pub fn cannot_hand_back(value: &Node) -> String {
	format!("the block gave {}, a value run_block cannot hand back yet", value.serialize().trim())
}
