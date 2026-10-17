//! `parse(text)` is the code the text holds, as data (card parse-word, notes/parse_word.md): the same Node `data (…)`
//! gives, so `parse("$a + 0") == data ($a + 0)`. A constant text folds here; any other text is parsed at run time by
//! the host word parse_code (card parse-runtime). A program's own `parse` stays its own, and so does a method
//! `x.parse(…)`. `eval(text)` runs that code where it is written: `interpret(parse(text))` (card eval).

use crate::host::PARSE_CODE;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::AS_CODE;

const PARSE_WORD: &str = "parse";
const EVAL_WORD: &str = "eval";
/// lowering/control/run_time_blocks.rs runs a block given as data
const INTERPRET_WORD: &str = "interpret";

pub fn lower(program: Node) -> Node {
	let program = where_free(program, EVAL_WORD, evaluated);
	where_free(program, PARSE_WORD, parsed)
}

/// The rewrite of `word` where the program leaves it free (notes/dispatch.md: shadowing is scoped)
fn where_free(program: Node, word: &str, rewrite: fn(Node) -> Node) -> Node {
	match mentions(&program, word) {
		true => crate::program_names::rewritten_where_free(program, word, &rewrite),
		false => program,
	}
}

fn mentions(program: &Node, word: &str) -> bool {
	let mut mentioned = false;
	program.visit(&mut |part| mentioned |= part.is_symbol(word));
	mentioned
}

/// `eval(t)` → `interpret(parse(t))`
fn evaluated(node: Node) -> Node {
	match word_argument(&node, EVAL_WORD).cloned() {
		Some(argument) => {
			let code = Node::List(vec![symbol(PARSE_WORD), evaluated(argument)], Bracket::None, Separator::Space);
			parens(vec![symbol(INTERPRET_WORD), parens(vec![code])])
		}
		None => crate::node::children_rewritten(node, evaluated),
	}
}

/// The code a text holds, or the first parse failure; never a file (`parse("a.warp")` is the code `a.warp`)
pub fn code_of(source: &str) -> Result<Node, String> {
	let code = crate::warp_parser::WarpParser::parse(source);
	match code.first_error().map(Node::drop_meta) {
		Some(Node::Error(failure)) => Err(failure.text_value().unwrap_or_else(|| failure.serialize())),
		_ => Ok(code),
	}
}

fn parsed(node: Node) -> Node {
	if let Node::Key(receiver, Op::Dot, method) = node {
		return Node::Key(Box::new(parsed(*receiver)), Op::Dot, method);
	}
	let Some(argument) = word_argument(&node, PARSE_WORD).cloned() else {
		return crate::node::children_rewritten(node, parsed);
	};
	// a text that fails to parse fails at run time, where `try` catches it
	match argument.text_value().and_then(|source| code_of(&source).ok()) {
		Some(code) => super::blocks::quoted_code(code),
		None => crate::node::call(AS_CODE, vec![parens(vec![symbol(PARSE_CODE), parsed(argument)])]),
	}
}

/// `parse "…"`, `parse(t)`, `parse t`: what is parsed, the same of eval
fn word_argument<'a>(node: &'a Node, word_name: &str) -> Option<&'a Node> {
	let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = node.drop_meta() else {
		return None;
	};
	let [word, argument] = items.as_slice() else {
		return None;
	};
	if !word.is_symbol(word_name) {
		return None;
	}
	Some(match argument.drop_meta() {
		Node::List(inner, Bracket::Round, _) if inner.len() == 1 => &inner[0],
		_ => argument,
	})
}
