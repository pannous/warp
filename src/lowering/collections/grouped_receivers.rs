//! `(p).a` is `p.a` (card dump-text): parentheses around a variable before a dot only group it, so the dot reads the
//! field or method as it does without them. Before, `(p).a` stayed the data `a:1.a` and `(t).dump` of a text failed
//! at run time as an item print could not join. `(function f).dump` keeps its group: the function itself, not a call.
//! The parser gives the call `f()` the same form as the group `(f)`, so only a name the program binds to a value (an
//! assignment, a loop variable) ungroups; any other name stays a call: `f().b`, `today().year`, `point().x`, `mk().a`
//! of `mk = () => …`, and the functions of an imported module

use crate::lowering::keyword_rewrites::{Facts, KeywordRewrite};
use crate::lowering::nodes::key;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::program_names::{Bindings, Meaning};

/// The keyword rewrite of a dot, an entry of src/pipeline.rs SUGAR_PHRASES
pub const UNGROUPED: KeywordRewrite =
	KeywordRewrite { triggered_by: |word| word == Op::Dot.as_str(), rewrite: ungrouped, reads_bindings: true };

fn ungrouped(node: Node, facts: &mut Facts) -> Node {
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot, member) => match grouped_name(receiver) {
			Some(name) if name.symbol_name().is_some_and(|name| is_variable(facts.bindings(), name)) => {
				key(name.clone(), Op::Dot, member.as_ref().clone()).with_meta_of(&node)
			}
			_ => node,
		},
		_ => node,
	}
}

/// A name assigned a value that is no function, or a loop variable
fn is_variable(bindings: &Bindings, name: &str) -> bool {
	(bindings.binds_as(name, Meaning::Variable) || bindings.binds_as(name, Meaning::Loop))
		&& !bindings.binds_as(name, Meaning::Function)
}

/// `(p)`: the one name in parentheses, no function reference
fn grouped_name(receiver: &Node) -> Option<&Node> {
	match receiver.drop_meta() {
		Node::List(items, Bracket::Round, _)
			if items.len() == 1
				&& items[0].symbol_name().is_some()
				&& crate::closures::referenced_function(&items[0]).is_none() =>
		{
			Some(&items[0])
		}
		_ => None,
	}
}
