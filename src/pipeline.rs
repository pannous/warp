//! The compile pipeline: parse → the lowering passes (`SOURCE_PASSES`, `MEANING_PASSES`) → analysis and diagnostics →
//! the WASM GC emitter (wasm_emitter) → run. `eval`, `compile` and `lower` are the entry points; wasm_emitter
//! re-exports them.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::{emit_module, find_struct_instantiation, WasmGcEmitter};
use crate::wasp_parser::WaspParser;
#[cfg(feature = "native")]
use log::warn;

const INDEX_OUT_OF_RANGE: &str = "index out of range";
const UNWRAPPED_EMPTY: &str = "unwrapped ø";
const RANGE_END_HINT: &str = "hint: `..` excludes the end; `...` or `to` include it";
#[cfg(feature = "native")]
use crate::wasm_emitter::{failed_run, run_raw_struct};
#[cfg(feature = "native")]
use crate::wasm_reader::read_bytes;

/// The program written in `code`, or in the file `code` names: a file sees the definitions of its folder (D15)
pub fn eval(code: &str) -> Node {
	let names_a_file = !code.contains('\n') && (code.ends_with(".wasp") || code.ends_with(".warp"));
	match names_a_file.then(|| std::fs::read_to_string(code).ok()).flatten() {
		Some(source) => crate::modules::with_program_file(std::path::Path::new(code), || eval_source(&source)),
		None => eval_source(code),
	}
}

fn eval_source(code: &str) -> Node {
	crate::diagnostic::begin_program(); // only the guesses made for this program explain its errors
	match lawful_program(code) {
		Ok(program) => {
			let exclusive_range = has_exclusive_range(&program);
			explain_runtime_error(eval_parsed(program, code), exclusive_range)
		}
		Err(violation) => violation,
	}
}

fn has_exclusive_range(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Range, _) => true,
		Node::Key(left, _, right) => has_exclusive_range(left) || has_exclusive_range(right),
		Node::List(items, _, _) => items.iter().any(has_exclusive_range),
		_ => false,
	}
}

/// A runtime error fails far from its cause: it names the range hint and the defaults unanswered Asks took
fn explain_runtime_error(result: Node, exclusive_range: bool) -> Node {
	let assumptions = crate::diagnostic::take_assumptions();
	let Node::Error(reason) = &result else { return result };
	let Node::Text(message) = reason.drop_meta() else { return result };
	let mut lines = vec![message.clone()];
	if exclusive_range && message.starts_with(INDEX_OUT_OF_RANGE) {
		lines.push(RANGE_END_HINT.to_string());
	}
	let mut assumed: Vec<(String, Vec<String>)> = Vec::new();
	for assumption in assumptions {
		let guess = format!("{}; fix: {}", assumption.message, assumption.fix.unwrap_or_default());
		let position = format!("{}:{}", assumption.line, assumption.column);
		match assumed.iter_mut().find(|(known, _)| *known == guess) {
			Some((_, positions)) => positions.push(position),
			None => assumed.push((guess, vec![position])),
		}
	}
	lines.extend(assumed.into_iter().map(|(guess, positions)| format!("assumed at {}: {guess}", positions.join(", "))));
	match lines.len() {
		1 => result,
		_ => crate::node::error(&lines.join("\n  ")),
	}
}

/// Parse the source and check its laws; `Err` is the violation.
fn lawful_program(code: &str) -> Result<Node, Node> {
	let lawful = crate::law::separate_laws(crate::diagnostic::in_source_mode(code, || WaspParser::parse(code)));
	match crate::law::assert_laws(&lawful, code) {
		Some(violation) => Err(violation),
		None => Ok(lawful.program),
	}
}

/// Evaluate foreign data with an empty capability set: a program that would call any host, WASI or FFI
/// function is refused before it is compiled. To only read data, use `parse_data`, which evaluates nothing.
pub fn eval_untrusted(code: &str) -> Node {
	crate::diagnostic::begin_program();
	let program = WaspParser::parse(code);
	match crate::effects::EffectReport::of(&program).denied(&crate::effects::Capability::GRANTED_UNTRUSTED) {
		Some((external, _)) => crate::node::error(&format!("untrusted code has no capabilities but pure libm, refusing to call {external}")),
		None => eval_parsed(program, code),
	}
}

/// A compiled program and the host capabilities its imports need.
#[cfg_attr(not(feature = "native"), allow(dead_code))] // the browser host links every import itself
pub struct CompiledModule {
	pub bytes: Vec<u8>,
	pub(crate) needs_host: bool,
	pub(crate) needs_wasi: bool,
	pub(crate) needs_ffi: bool,
}

/// The passes over the source forms, in order, each reading what the one before it left: definitions and sugar become
/// the forms every later pass knows (`def f(x) {…}` is `f(x) := {…}`), modules are resolved
const SOURCE_PASSES: [fn(Node) -> Node; 26] = [
	crate::phrase_calls::lower,
	crate::welcome_forms::lower, crate::number_keys::lower,
	crate::declarations::lower_tasks, crate::shared_arrays::lower, crate::variable_signals::lower, crate::declarations::lower_c_functions, crate::declarations::lower_bare_declarations, crate::declarations::lower_spaced_definitions, crate::result_word::lower, crate::picked_calls::lower, crate::named_arguments::lower, crate::comprehensions::lower, crate::library_words::lower_function_methods,
	crate::tuples::lower, crate::run_time_blocks::warn_unresolved, crate::run_time_blocks::lower_interpret, crate::blocks::lower, crate::run_time_blocks::lower_run_time_bangs, crate::mutation::lower, crate::host::lower_aliases, crate::modules::resolve,
	crate::type_name_matching::lower, crate::meta_entries::lower, crate::versions::lower_versions,
	crate::analyzer::lower_negated_calls,
];

/// The passes after the constant answers (time, units, reals), in order: types and traits, lambdas and closures, words
const MEANING_PASSES: [fn(Node) -> Node; 22] = [
	crate::declarations::resolve_tasks, crate::traits::lower_declarations, crate::type_tests::lower, crate::ambiguous_forms::lower, crate::analyzer::lower_list_times,
	crate::broadcasting::lower, crate::library_words::lower_count_in, crate::lambdas::lower, crate::function_values::lower, crate::closures::lower, crate::lambdas::lower_strict, crate::real::lower,
	crate::type_constructor::lower, crate::printable::lower, crate::overloads::lower, crate::traits::lower_conformances, crate::min_max::lower,
	crate::declarations::lower, crate::switch::lower, crate::phrase_words::lower, crate::library_words::lower,
	crate::traits::lower_dispatch,
];

fn run_passes(node: Node, passes: &[fn(Node) -> Node]) -> Node {
	passes.iter().fold(node, |node, pass| pass(node))
}

/// Everything before code generation. `Err` is the final value of the program when it needs no module
/// (a constant answer, an error, a denied capability).
fn lower_for_emission(node: Node) -> Result<Node, Node> {
	use crate::effects::{without_constraints, Capability, EffectReport};

	let node = run_passes(node, &SOURCE_PASSES);
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	if let Some(kind_change) = crate::analyzer::check_kind_changes(&node) {
		return Err(kind_change.into_error());
	}
	let node = crate::interpolation::lower(crate::injection::lower_templates(node)?);
	let node = crate::function_equality::decide_comparisons(node);
	if let Node::Error(_) = node {
		return Err(node);
	}
	if let Some(answer) = crate::time::answer(&node) {
		return Err(answer);
	}
	if let Some(answer) = crate::units::answer(&node) {
		return Err(answer);
	}
	if let Some(answer) = crate::real::answer(&node) {
		return Err(answer);
	}
	let node = run_passes(node, &MEANING_PASSES);
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	let effects = EffectReport::of(&node);
	if let Some(answer) = effects.answer(&node) {
		return Err(answer);
	}
	if let Some((name, capability)) = effects.denied(&Capability::GRANTED_BY_EVAL) {
		return Err(crate::node::error(&format!(
			"capability denied: {name} needs the {} capability, which eval does not grant", capability.name())));
	}
	let node = crate::analyzer::resolve_main_variable_assignments(crate::late_binding::lower(without_constraints(node))?)?;
	if let Some(error) = crate::analyzer::diagnose(&node) {
		return Err(error);
	}
	crate::diagnostic::report(&crate::analyzer::lint(&node))?;
	Ok(crate::analyzer::indexed_parameter_copies(crate::inlining::lower(crate::analyzer::lower_declarations(crate::analyzer::resolve_data_scope(node)))))
}

/// The program as the emitter sees it after every lowering pass (`warp lower <code>`); `Err` is its final value
/// when it needs no module.
pub fn lower(code: &str) -> Result<Node, Node> {
	crate::diagnostic::begin_program();
	crate::diagnostic::in_program_mode(lawful_program(code)?, lower_for_emission)
}

/// Compile source text to a wasm module without running it. `Err` carries the error, or the constant
/// answer of a program that needs no module.
pub fn compile(code: &str) -> Result<CompiledModule, Node> {
	crate::diagnostic::begin_program();
	crate::diagnostic::in_program_mode(lawful_program(code)?, |program| {
		let node = crate::folding::precompute(lower_for_emission(program)?);
		warn_about_run_time_blocks(&node)?;
		choose_module(&node)
	})
}

/// A compiled module that runs blocks at run time needs a host with a warp compiler: say so where it is made
fn warn_about_run_time_blocks(node: &Node) -> Result<(), Node> {
	if !crate::effects::EffectReport::of(node).calls_external(crate::host::RUN_BLOCK) {
		return Ok(());
	}
	crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(node, format!(
		"this module runs blocks known only at run time: its host must provide host.{} (the warp CLI, or web/playground/host.js, which loads warp.wasm on first use)",
		crate::host::RUN_BLOCK))])
}

/// `TypeName:{field:value, ...}` compiles to a raw struct module, everything else to the standard Node encoding.
fn raw_struct_module(node: &Node) -> Option<Vec<u8>> {
	let mut type_registry = crate::type_kinds::TypeRegistry::new();
	crate::analyzer::collect_all_types(&mut type_registry, node);
	let (type_def, field_values) = find_struct_instantiation(&type_registry, node)?;
	Some(WasmGcEmitter::emit_raw_struct(&type_def, &field_values))
}

fn choose_module(node: &Node) -> Result<CompiledModule, Node> {
	match raw_struct_module(node) {
		Some(bytes) => Ok(CompiledModule { bytes, needs_host: false, needs_wasi: false, needs_ffi: false }),
		None => emit_module(node),
	}
}

/// Compile and run an already parsed program; imports follow its resolved effects.
pub fn eval_parsed(node: Node, _code: &str) -> Node {
	crate::diagnostic::in_program_mode(node, eval_program)
}

fn eval_program(node: Node) -> Node {
	let node = match lower_for_emission(node) {
		Ok(node) => node,
		Err(final_value) => return final_value,
	};

	#[cfg(feature = "native")]
	if let Some(wasm_bytes) = raw_struct_module(&node) {
		match run_raw_struct(&wasm_bytes) {
			Ok(result) => return result,
			Err(e) => warn!("raw struct eval failed: {}", e),
		}
	}

	// Fallback to standard Node encoding
	match emit_module(&node) {
		Ok(module) => run_module(module),
		Err(type_error) => type_error,
	}
}

/// The words of an exception (library_words lowers them to the call raise(X))
pub(crate) const RAISE_WORDS: [&str; 2] = ["raise", "throw"];

/// Run a block the program built at run time (the host's run_block, notes/runtime_eval.md): `names` (one text, the
/// names separated by spaces) are bound to `values` first, the values the program had where it ran the block, then
/// the program's function `definitions` follow. The
/// block gets no capability but pure libm, wherever its code came from (user decision: pure by default)
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
	let items = |list: &Node| match list.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	let values = items(values);
	let as_written = |value: Node| match value.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::Empty => value,
		_ => Node::List(vec![Node::Symbol(crate::blocks::DATA_WORD.to_string()), value], Bracket::None, Separator::Space),
	};
	let mut statements: Vec<Node> = names.into_iter().zip(values)
		.map(|(name, value)| Node::Key(Box::new(Node::Symbol(name)), Op::Assign, Box::new(as_written(value))))
		.collect();
	statements.extend(items(definitions));
	statements.push(block);
	let program = Node::List(statements, Bracket::None, Separator::Newline);
	let result = match crate::effects::EffectReport::of(&program).denied(&crate::effects::Capability::GRANTED_UNTRUSTED) {
		Some((external, capability)) => crate::node::error(&format!(
			"a block run at run time is pure: {external} needs the {} capability, which it does not get", capability.name())),
		None => eval_parsed(program, ""),
	};
	match result.drop_meta() {
		Node::Error(message) => Err(format!("the block {written} failed: {}", message.serialize().trim_matches('"'))),
		_ => Ok(result),
	}
}

/// A block's value the host has no constructor for in the running module yet (a pair)
pub fn cannot_hand_back(value: &Node) -> String {
	format!("the block gave {}, a value run_block cannot hand back yet", value.serialize().trim())
}

/// The message of `error("…")`, or of `raise …`, which always fails the run
pub(crate) fn returned_error_message(value: &Node) -> Option<&Node> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == "error" || word == crate::wasm_emitter::text_builtins::RAISE) => Some(&items[1]),
		// `raise X` / `throw X` before library_words makes them the call raise(X)
		Node::List(items, Bracket::None, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if RAISE_WORDS.contains(&word.as_str())) => Some(&items[1]),
		_ => None,
	}
}

/// Run a compiled program with the linker its imports need
#[cfg(feature = "native")]
pub(crate) fn run_module(CompiledModule { bytes, needs_host, needs_wasi, needs_ffi }: CompiledModule) -> Node {
	use crate::wasm_reader::{read_bytes_with_imports, Imports};
	let result = if needs_ffi || needs_wasi || needs_host {
		read_bytes_with_imports(&bytes, Imports { host: needs_host, wasi: needs_wasi, ffi: needs_ffi })
	} else {
		read_bytes(&bytes)
	};
	result.unwrap_or_else(failed_run)
}

/// Without wasmtime the embedding host runs the program (the browser playground: web.rs)
#[cfg(not(feature = "native"))]
pub(crate) fn run_module(module: CompiledModule) -> Node {
	crate::web::run_in_host(&module.bytes)
}

/// The run used up its fuel: it probably does not terminate, or needs a larger budget
pub fn out_of_fuel(steps: u64) -> Node {
	crate::node::error(&format!(
		"out of fuel after {steps} steps: the program may not terminate (raise the budget with {}=<steps> or --fuel <steps>)",
		crate::util::FUEL_VARIABLE))
}
