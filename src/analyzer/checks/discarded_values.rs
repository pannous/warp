//! Values a statement drops (wiki/mutable.md): a copy word's result (`uppercase x`), the copy a mutating call also
//! makes (`y = x.upper!`), an `error "…"` nobody raises (card error-if)

use crate::errors::diagnostic::Diagnostic;
use crate::lowering::mutation::is_marked;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::ERROR_WORD;

/// Pure words whose result is a changed copy of their argument: dropping it changes nothing. Any spelling of them
/// (`uppercase`, `strip`, `sorted`) is one (library_words::canonical_spelling)
const COPY_WORDS: [&str; 7] = ["upper", "lower", "trim", "reverse", "sort", "replace", "capitalize"];

/// wiki/mutable.md: a statement dropping a copy word's result (`uppercase x`), and an assignment of a mutating call
/// (`y = x.upper!`, which changes x too), warn with the forms meant; so does a dropped `error "…"` (card error-if)
pub fn warn_discarded(program: Node) -> Node {
	let mut warnings = vec![];
	program.visit(&mut |node| match node.drop_meta() {
		Node::List(statements, _, Separator::Semicolon | Separator::Newline) => {
			for statement in &statements[..statements.len().saturating_sub(1)] {
				if let Some((word, variable)) = copy_call(statement) {
					let message = format!("unused value of {word} {variable}: assign it (y = {word} {variable}) or write {variable}.{word}! to change {variable}");
					warnings.push(Diagnostic::at(statement, message).offer(format!("change {variable}"), format!("{word} {variable}"), format!("{variable}.{word}!")));
				}
				let mut dropped = vec![];
				discarded_errors(statement, &mut dropped);
				warnings.extend(dropped.into_iter().map(discarded_error_warning));
			}
		}
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Some(variable) = mutating_call(value) {
				let target = crate::normalize::operand_text(target);
				warnings.push(Diagnostic::at(node, format!("{target} = …! changes {variable} too: assign the copy without `!`, or mutate {variable} first and then write {target} = {variable}")));
			}
		}
		_ => {}
	});
	match crate::errors::diagnostic::report(&warnings) {
		Err(error) => error,
		Ok(()) => program,
	}
}

/// The `error "…"` calls whose value a dropped statement drops: the statement itself, the branches of an `if`, the
/// last statement of a block or sequence
fn discarded_errors<'a>(statement: &'a Node, dropped: &mut Vec<&'a Node>) {
	match statement.drop_meta() {
		Node::List(items, _, Separator::Space | Separator::None)
			if items.len() == 2 && items[0].is_symbol(ERROR_WORD) =>
		{
			dropped.push(statement)
		}
		Node::Key(_, Op::Then, branch) => discarded_errors(branch, dropped),
		Node::Key(then, Op::Else, otherwise) => {
			discarded_errors(then, dropped);
			discarded_errors(otherwise, dropped);
		}
		Node::List(items, Bracket::Curly, _) | Node::List(items, _, Separator::Semicolon | Separator::Newline) => {
			items.iter().last().into_iter().for_each(|last| discarded_errors(last, dropped))
		}
		_ => {}
	}
}

/// `error "?"` only makes an error value; `fail "?"` raises it
fn discarded_error_warning(call: &Node) -> Diagnostic {
	let written = crate::normalize::operand_text(call);
	let raised = written.replacen(ERROR_WORD, crate::errors::run_time::FAIL_WORD, 1);
	let message = format!("`{written}` makes an error value nothing uses: `{raised}` raises it");
	Diagnostic::at(call, message).offer("raise the error", written, raised)
}

/// A copy word called on a variable, as written (`upper x`, `upper(x)`, `x.upper`, `x.upper()`), and whether a `!`
/// marks it as mutating (`upper x!`, `x.upper!`)
fn copy_word_call(call: &Node) -> Option<(&str, &str, bool)> {
	let (word, variable) = match call.drop_meta() {
		Node::Key(receiver, Op::Dot, method) => (method.as_ref(), receiver.as_ref()),
		Node::List(items, _, _) if items.len() == 2 => (&items[0], &items[1]),
		_ => return None,
	};
	let called = match word.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].drop_meta(),
		other => other,
	};
	match (called, variable.drop_meta()) {
		(Node::Symbol(called), Node::Symbol(name))
			if COPY_WORDS.contains(&crate::library_words::canonical_spelling(called)) =>
		{
			Some((called, name, is_marked(call) || is_marked(variable)))
		}
		_ => None,
	}
}

fn copy_call(call: &Node) -> Option<(&str, &str)> {
	copy_word_call(call).and_then(|(word, variable, mutating)| (!mutating).then_some((word, variable)))
}

/// The variable a mutating call assigns: `x.upper!` and `upper x!` as marked, `x = x.upper` as the parser wrote `upper(x)!`
fn mutating_call(value: &Node) -> Option<&str> {
	match value.drop_meta() {
		Node::Key(variable, Op::Assign, call) => match (variable.drop_meta(), copy_call(call)) {
			(Node::Symbol(name), Some((_, called))) if name == called => Some(name),
			_ => None,
		},
		_ => copy_word_call(value).and_then(|(_, variable, mutating)| mutating.then_some(variable)),
	}
}
