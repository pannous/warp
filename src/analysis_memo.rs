//! One analysis per program state (user decision P91, notes/build_speed.md): `extract_user_functions` of a fresh
//! Context is remembered by a fingerprint of the whole tree, positions and comments included, so the many lowering
//! passes (and every EffectReport::of) that analyse a program the pass before them did not change get the same answer
//! without recomputing it. A changed tree has another fingerprint and is analysed anew: the answer is always the one a
//! fresh analysis gives. An analysis that says something (a warning, a hint) is not remembered, so a repeat still says
//! it. `WARP_ANALYSIS_CACHE=off` turns it off (A/B measurements).

use crate::context::{Context, UserFunctionDef};
use crate::ffi::FfiSignature;
use crate::node::Node;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

const SWITCH_VARIABLE: &str = "WARP_ANALYSIS_CACHE";
/// Program states remembered per thread: the passes of one compile alternate between few of them
const REMEMBERED: usize = 256;

/// What extract_user_functions writes into a fresh Context
#[derive(Clone)]
pub struct Analysis {
	user_functions: HashMap<String, UserFunctionDef>,
	ffi_imports: HashMap<String, FfiSignature>,
	field_kinds: HashMap<String, crate::type_kinds::Kind>,
	enclosing_functions: HashMap<String, String>,
	parameter_conflicts: Vec<String>,
	closure_targets: Vec<(String, usize)>,
	closure_variable_targets: HashMap<String, HashSet<String>>,
}

impl Analysis {
	fn of(context: &Context) -> Analysis {
		Analysis {
			user_functions: context.user_functions.clone(),
			ffi_imports: context.ffi_imports.clone(),
			field_kinds: context.field_kinds.clone(),
			enclosing_functions: context.enclosing_functions.clone(),
			parameter_conflicts: context.parameter_conflicts.clone(),
			closure_targets: context.closure_targets.clone(),
			closure_variable_targets: context.closure_variable_targets.clone(),
		}
	}

	fn into(self, context: &mut Context) {
		context.user_functions = self.user_functions;
		context.ffi_imports = self.ffi_imports;
		context.field_kinds = self.field_kinds;
		context.enclosing_functions = self.enclosing_functions;
		context.parameter_conflicts = self.parameter_conflicts;
		context.closure_targets = self.closure_targets;
		context.closure_variable_targets = self.closure_variable_targets;
	}
}

thread_local! {
	static REMEMBERED_ANALYSES: RefCell<Vec<(u64, Analysis)>> = const { RefCell::new(Vec::new()) };
}

/// `analyse(context, program)` into a fresh `context`, or what it gave for the same tree before
pub fn analysed(context: &mut Context, program: &Node, analyse: impl FnOnce(&mut Context, &Node)) {
	let key = (is_fresh(context) && std::env::var(SWITCH_VARIABLE).as_deref() != Ok("off")).then(|| fingerprint(program)).flatten();
	let Some(key) = key else { return analyse(context, program) };
	if let Some(analysis) = REMEMBERED_ANALYSES.with(|remembered| remembered.borrow().iter().find(|(known, _)| *known == key).map(|(_, analysis)| analysis.clone())) {
		return analysis.into(context);
	}
	// what the analysis needs from the runtime is no part of a remembered answer: such an analysis is not remembered
	let (required, said) = (context.required_functions.clone(), crate::diagnostic::said());
	analyse(context, program);
	// an analysis that needed runtime functions or said something (a warning, a hint) is no pure answer: not remembered
	if context.required_functions != required || crate::diagnostic::said() != said {
		return;
	}
	REMEMBERED_ANALYSES.with(|remembered| {
		let mut remembered = remembered.borrow_mut();
		if remembered.len() == REMEMBERED {
			remembered.remove(0);
		}
		remembered.push((key, Analysis::of(context)));
	});
}

/// A Context nothing was put in yet: the analysis depends only on the program
fn is_fresh(context: &Context) -> bool {
	context.user_functions.is_empty() && context.ffi_imports.is_empty() && context.field_kinds.is_empty() && context.enclosing_functions.is_empty()
		&& context.parameter_conflicts.is_empty() && context.closure_targets.is_empty() && context.closure_variable_targets.is_empty()
		&& context.type_registry.types().is_empty() && context.user_globals.is_empty() && context.declared_globals.is_empty()
		&& context.captures.is_empty() && context.capture_bindings.is_empty()
}

/// A hash of the whole tree, metadata included; none for a tree holding opaque Rust data (Node::Data)
fn fingerprint(program: &Node) -> Option<u64> {
	let mut hasher = DefaultHasher::new();
	hash_node(program, &mut hasher).then(|| hasher.finish())
}

fn hash_node(node: &Node, hasher: &mut DefaultHasher) -> bool {
	std::mem::discriminant(node).hash(hasher);
	match node {
		Node::True | Node::False | Node::Empty => true,
		Node::Number(number) => {
			format!("{number:?}").hash(hasher);
			true
		}
		Node::Char(character) => {
			character.hash(hasher);
			true
		}
		Node::Text(text) | Node::Symbol(text) => {
			text.hash(hasher);
			true
		}
		Node::Error(inner) => hash_node(inner, hasher),
		Node::Key(left, op, right) => {
			format!("{op:?}").hash(hasher);
			hash_node(left, hasher) && hash_node(right, hasher)
		}
		Node::List(items, bracket, separator) => {
			crate::wasm_emitter::bracket_info(bracket).hash(hasher);
			if let crate::node::Bracket::Other(open, close) = bracket {
				(open, close).hash(hasher);
			}
			format!("{separator:?}").hash(hasher);
			items.len().hash(hasher);
			items.iter().all(|item| hash_node(item, hasher))
		}
		Node::Meta { node, data } => hash_node(node, hasher) && hash_node(data, hasher),
		Node::Type { name, body } => hash_node(name, hasher) && hash_node(body, hasher),
		// a position (Meta data): the only Rust data a parsed program holds; any other makes the tree unremembered
		Node::Data(dada) => match dada.downcast_ref::<crate::meta::LineInfo>() {
			Some(position) => {
				format!("{position:?}").hash(hasher);
				true
			}
			None => false,
		},
	}
}
