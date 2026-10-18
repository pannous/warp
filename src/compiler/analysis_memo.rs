//! One analysis per program state (user decision P91, notes/build_speed.md): `extract_user_functions` of a fresh
//! Context is remembered by a fingerprint of the whole tree, positions and comments included, so the many lowering
//! passes (and every EffectReport::of) that analyse a program the pass before them did not change get the same answer
//! without recomputing it. A changed tree has another fingerprint and is analysed anew: the answer is always the one a
//! fresh analysis gives. An analysis that says something (a warning, a hint) is not remembered, so a repeat still says
//! it. `WARP_ANALYSIS_CACHE=off` turns it off (A/B measurements).

use crate::compiler::context::{Context, Param, UserFunctionDef};
use crate::ffi::FfiSignature;
use crate::node::Node;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

const SWITCH_VARIABLE: &str = "WARP_ANALYSIS_CACHE";
/// Program states remembered per thread: the passes of one compile alternate between few of them
const REMEMBERED: usize = 256;
/// Definitions remembered per thread: the prelude's and the used modules' words of the programs it compiles
const REMEMBERED_DEFINITIONS: usize = 4096;

/// What extract_user_functions writes into a fresh Context
#[derive(Clone)]
pub struct Analysis {
	user_functions: std::collections::BTreeMap<String, UserFunctionDef>,
	ffi_imports: HashMap<String, FfiSignature>,
	field_kinds: HashMap<String, crate::type_kinds::Kind>,
	enclosing_functions: HashMap<String, String>,
	parameter_conflicts: Vec<String>,
	closure_targets: Vec<(String, usize)>,
	closure_variable_targets: crate::analyzer::ClosureTargets,
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
	static REMEMBERED_FUNCTIONS: RefCell<HashMap<u64, UserFunctionDef>> = RefCell::new(HashMap::new());
}

/// `analyse(context, program)` into a fresh `context`, or what it gave for the same tree before
pub fn analysed(context: &mut Context, program: &Node, analyse: impl FnOnce(&mut Context, &Node)) {
	let key = (is_fresh(context) && std::env::var(SWITCH_VARIABLE).as_deref() != Ok("off"))
		.then(|| fingerprint(program))
		.flatten();
	let Some(key) = key else {
		return analyse(context, program);
	};
	if let Some(analysis) = REMEMBERED_ANALYSES.with(|remembered| {
		remembered.borrow().iter().find(|(known, _)| *known == key).map(|(_, analysis)| analysis.clone())
	}) {
		return analysis.into(context);
	}
	// what the analysis needs from the runtime is no part of a remembered answer: such an analysis is not remembered
	let (required, said) = (context.required_functions.clone(), crate::errors::diagnostic::said());
	analyse(context, program);
	// an analysis that needed runtime functions or said something (a warning, a hint) is no pure answer: not remembered
	if context.required_functions != required || crate::errors::diagnostic::said() != said {
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

/// `analyse()`'s answer for one definition (user_function: its parameters' and result's kinds from its body), or what it
/// gave for the same definition before: the passes analyse each program state anew, the prelude's words in it unchanged
/// (card prelude-cache-runs). The key adds the bool functions so far, the one part of the compile the analysis reads.
pub fn analysed_function(
	name: &str,
	params: &[Param],
	body: &Node,
	analyse: impl FnOnce() -> UserFunctionDef,
) -> UserFunctionDef {
	let key =
		(std::env::var(SWITCH_VARIABLE).as_deref() != Ok("off")).then(|| function_fingerprint(name, params, body));
	let Some(key) = key.flatten() else { return analyse() };
	if let Some(function) = REMEMBERED_FUNCTIONS.with_borrow(|remembered| remembered.get(&key).cloned()) {
		return function;
	}
	let said = crate::errors::diagnostic::said();
	let function = analyse();
	if crate::errors::diagnostic::said() == said {
		REMEMBERED_FUNCTIONS.with_borrow_mut(|remembered| {
			if remembered.len() == REMEMBERED_DEFINITIONS {
				remembered.clear();
			}
			remembered.insert(key, function.clone());
		});
	}
	function
}

fn function_fingerprint(name: &str, params: &[Param], body: &Node) -> Option<u64> {
	let mut hasher = DefaultHasher::new();
	name.hash(&mut hasher);
	let mut bool_functions: Vec<String> =
		crate::compiler::compile_context::read(|context| context.bool_functions.iter().cloned().collect());
	bool_functions.sort();
	bool_functions.hash(&mut hasher);
	// the presence too: `b: 2` read as an annotation must not meet `b = 2`, the default it lowers to
	let hash_optional = |node: &Option<Node>, hasher: &mut DefaultHasher| {
		node.is_some().hash(hasher);
		node.as_ref().is_none_or(|node| hash_node(node, hasher))
	};
	let params_hashed = params.iter().all(|param| {
		(&param.name, param.used_as).hash(&mut hasher);
		hash_optional(&param.annotation, &mut hasher) && hash_optional(&param.default, &mut hasher)
	});
	(params_hashed && hash_node(body, &mut hasher)).then(|| hasher.finish())
}

/// A Context nothing was put in yet: the analysis depends only on the program
fn is_fresh(context: &Context) -> bool {
	context.user_functions.is_empty()
		&& context.ffi_imports.is_empty()
		&& context.field_kinds.is_empty()
		&& context.enclosing_functions.is_empty()
		&& context.parameter_conflicts.is_empty()
		&& context.closure_targets.is_empty()
		&& context.closure_variable_targets.is_empty()
		&& context.type_registry.types().is_empty()
		&& context.user_globals.is_empty()
		&& context.declared_globals.is_empty()
		&& context.captures.is_empty()
		&& context.capture_bindings.is_empty()
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
			hash_number(number, hasher);
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
			op.hash(hasher);
			hash_node(left, hasher) && hash_node(right, hasher)
		}
		Node::List(items, bracket, separator) => {
			crate::wasm_emitter::bracket_info(bracket).hash(hasher);
			if let crate::node::Bracket::Other(open, close) = bracket {
				(open, close).hash(hasher);
			}
			std::mem::discriminant(separator).hash(hasher);
			items.len().hash(hasher);
			items.iter().all(|item| hash_node(item, hasher))
		}
		Node::Meta { node, data } => hash_node(node, hasher) && hash_node(data, hasher),
		Node::Type { name, body } => hash_node(name, hasher) && hash_node(body, hasher),
		// a position (Meta data): the only Rust data a parsed program holds; any other makes the tree unremembered
		Node::Data(dada) => match dada.downcast_ref::<crate::compiler::meta::LineInfo>() {
			Some(position) => {
				(position.line_nr, position.column, &position.file, position.after_number).hash(hasher);
				true
			}
			None => false,
		},
	}
}

/// The common numbers without formatting them: the fingerprint walks every node of each program state
fn hash_number(number: &crate::extensions::numbers::Number, hasher: &mut DefaultHasher) {
	use crate::extensions::numbers::Number;
	std::mem::discriminant(number).hash(hasher);
	match number {
		Number::Int(int) => int.hash(hasher),
		Number::Float(float) => float.to_bits().hash(hasher),
		other => format!("{other:?}").hash(hasher),
	}
}
