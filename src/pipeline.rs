//! The compile pipeline: parse → the lowering passes (`SOURCE_PASSES`, `MEANING_PASSES`) → analysis and diagnostics →
//! the WASM GC emitter (wasm_emitter) → run. `eval`, `compile` and `lower` are the entry points; wasm_emitter
//! re-exports them.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::{emit_module, find_struct_instantiation, WasmGcEmitter};
use crate::warp_parser::WarpParser;
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
	if let Some(outcome) = crate::page_tests::answer(code) {
		return outcome;
	}
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
	let lawful = crate::law::separate_laws(crate::diagnostic::in_source_mode(code, || WarpParser::parse(code)));
	match crate::law::assert_laws(&lawful, code) {
		Some(violation) => Err(violation),
		None => Ok(lawful.program),
	}
}

/// Evaluate foreign data with an empty capability set: a program that would call any host, WASI or FFI
/// function is refused before it is compiled. To only read data, use `parse_data`, which evaluates nothing.
pub fn eval_untrusted(code: &str) -> Node {
	crate::diagnostic::begin_program();
	let program = WarpParser::parse(code);
	match crate::effects::EffectReport::of(&program).denied(&crate::effects::Capability::GRANTED_UNTRUSTED) {
		Some((external, _)) => crate::node::error(&format!("untrusted code has no capabilities but pure libm, refusing to call {external}")),
		None => with_granted(&crate::effects::Capability::GRANTED_UNTRUSTED, || eval_parsed(program, code)),
	}
}

thread_local! {
	/// What the program being compiled may call: eval's grant, or the untrusted one (checked again after lowering,
	/// which brings calls of its own: `use python math` → foreign_call)
	static GRANTED: std::cell::Cell<&'static [crate::effects::Capability]> = const { std::cell::Cell::new(&crate::effects::Capability::GRANTED_BY_EVAL) };
}

/// `run` with the program granted only `granted`
fn with_granted<T>(granted: &'static [crate::effects::Capability], run: impl FnOnce() -> T) -> T {
	let before = GRANTED.with(|current| current.replace(granted));
	let result = run();
	GRANTED.with(|current| current.set(before));
	result
}

thread_local! {
	/// whether the program compiled on this thread is for a page (`warp build --site`), where page events happen
	static FOR_A_PAGE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	/// whether it is for `warp dev`, whose page keeps the program's state across reloads
	static FOR_DEV: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	static RENDERS_ITSELF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	/// whether the program compiled now ends in the print of its value that compile_printing_result added
	static PRINTS_RESULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	/// whether the page is compiled to render its first HTML where the server's code is (site.rs, headless.rs)
	static PRERENDERING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	/// the values the page's calls of server functions gave its prerender, the shipped page's first values
	static SERVER_VALUES: std::cell::RefCell<Vec<Node>> = const { std::cell::RefCell::new(vec![]) };
	/// the port `warp serve` serves a program at that has no `serve PORT {…}` of its own (lowering/serve.rs)
	static SERVING_PORT: std::cell::Cell<Option<u16>> = const { std::cell::Cell::new(None) };
	/// whether it is a module for any host (`warp compile --wasm`): a browser's host reads its values too
	static FOR_ANY_HOST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
	/// whether it runs under `warp test`: its tests run and give its value (lowering/test_blocks.rs)
	static FOR_TESTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `run` with `flag` set on this thread
fn with_flag<T>(flag: &'static std::thread::LocalKey<std::cell::Cell<bool>>, run: impl FnOnce() -> T) -> T {
	let before = flag.with(|set| set.replace(true));
	let result = run();
	flag.with(|set| set.set(before));
	result
}

thread_local! {
	/// The functions the component compiled on this thread exports and imports (`warp build --component`)
	static COMPONENT_FUNCTIONS: std::cell::RefCell<crate::component_worlds::WorldFunctions> = std::cell::RefCell::new(Default::default());
}

/// `run` compiling a program as a component exporting and importing `functions`: the emitter adds the canonical-ABI
/// adapters of the exports and imports the imports from the core modules the world names
pub fn for_a_component<T>(functions: crate::component_worlds::WorldFunctions, run: impl FnOnce() -> T) -> T {
	let before = COMPONENT_FUNCTIONS.with(|current| current.replace(functions));
	let result = run();
	COMPONENT_FUNCTIONS.with(|current| *current.borrow_mut() = before);
	result
}

/// The functions the component being compiled exports, none for a program
pub fn component_exports() -> Vec<(String, crate::component_worlds::Signature)> {
	COMPONENT_FUNCTIONS.with(|current| current.borrow().exports.clone())
}

/// The functions the component being compiled imports, none for a program
pub fn component_imports() -> Vec<(String, crate::component_worlds::Signature)> {
	COMPONENT_FUNCTIONS.with(|current| current.borrow().imports.clone())
}

/// `run` compiling a program for a page: its page event handlers are expected, not warned about
pub fn for_a_page<T>(run: impl FnOnce() -> T) -> T {
	with_flag(&FOR_A_PAGE, run)
}

/// Whether the program compiled now is for a page: it exports the reflection getters the page's host reads values with
pub fn is_for_a_page() -> bool {
	FOR_A_PAGE.with(|page| page.get())
}

/// `run` compiling a module for any host (`warp compile --wasm`): it exports the reflection getters a host without GC
/// field access (the browser) reads values with, so a browser program importing it copies values in and out
pub fn for_any_host<T>(run: impl FnOnce() -> T) -> T {
	with_flag(&FOR_ANY_HOST, run)
}

pub fn is_for_any_host() -> bool {
	FOR_ANY_HOST.with(|any| any.get())
}

/// `run` compiling a page that calls its server functions directly, as the server does (lowering/serve.rs): for its
/// first HTML, rendered at build time
pub fn prerendering<T>(run: impl FnOnce() -> T) -> T {
	with_flag(&PRERENDERING, run)
}

pub fn is_prerendering() -> bool {
	PRERENDERING.with(|prerendering| prerendering.get())
}

/// `run` compiling the shipped page, whose calls of server functions start as `values`
pub fn with_server_values<T>(values: Vec<Node>, run: impl FnOnce() -> T) -> T {
	let before = SERVER_VALUES.with(|current| current.replace(values));
	let result = run();
	SERVER_VALUES.with(|current| current.replace(before));
	result
}

/// `run` compiling a program `warp serve` serves at the port: its server functions, its top-level `get`/`post` routes
/// and its page, without a `serve PORT {…}` statement
pub fn serving_at<T>(port: u16, run: impl FnOnce() -> T) -> T {
	let before = SERVING_PORT.with(|current| current.replace(Some(port)));
	let result = run();
	SERVING_PORT.with(|current| current.set(before));
	result
}

pub fn serving_port() -> Option<u16> {
	SERVING_PORT.with(|port| port.get())
}

/// The first value of the page's `index`th call of a server function, ø when none was rendered
pub fn server_value(index: usize) -> Node {
	SERVER_VALUES.with(|values| values.borrow().get(index).cloned().unwrap_or(Node::Empty))
}

/// `run` compiling a program that renders its own markup (the playground, web.rs renders_itself): it exports page·html
/// and page·render as a page does (lowering/page_html.rs); nothing else of a page changes (it may still serve)
pub fn rendering_itself<T>(run: impl FnOnce() -> T) -> T {
	with_flag(&RENDERS_ITSELF, run)
}

/// Whether the program compiled now exports its renderer: a page's, or one rendering itself
pub fn renders_itself() -> bool {
	is_for_a_page() || RENDERS_ITSELF.with(|renders| renders.get())
}

/// `run` compiling a program for `warp dev`: the main-level variables it changes are kept (lowering/stored_values.rs)
pub fn for_dev<T>(run: impl FnOnce() -> T) -> T {
	with_flag(&FOR_DEV, run)
}

pub fn is_for_dev() -> bool {
	FOR_DEV.with(|dev| dev.get())
}

/// `run` a program under `warp test`: its `test` lines and blocks run, its value is their summary (P209, P210)
pub fn for_tests<T>(run: impl FnOnce() -> T) -> T {
	with_flag(&FOR_TESTS, run)
}

pub fn is_for_tests() -> bool {
	FOR_TESTS.with(|tests| tests.get())
}

/// A compiled program and the host capabilities its imports need.
#[cfg_attr(not(feature = "native"), allow(dead_code))] // the browser host links every import itself
#[derive(Clone)]
pub struct CompiledModule {
	pub bytes: Vec<u8>,
	pub(crate) needs_host: bool,
	pub(crate) needs_wasi: bool,
	pub(crate) needs_ffi: bool,
}

/// The passes over the source forms, in order, each reading what the one before it left: definitions and sugar become
/// the forms every later pass knows (`def f(x) {…}` is `f(x) := {…}`), modules are resolved
const SOURCE_PASSES: [fn(Node) -> Node; 94] = [
	// `"a \(x) b"` → `"a " + text_form(x) + " b"` (interpolation.rs) first, so every pass reads the holes as code
	crate::interpolation::lower_program,
	crate::analyzer::lower_inline_unions,
	// `on ask {…} in {…}` before any pass reads `{…} in {…}` as membership or an emit as nothing
	crate::scoped_handlers::lower,
	// `component name {…}` declares a WIT world (`warp build --wit`) and does nothing at run time
	crate::component_worlds::lower,
	// `ch.send(v)` of `ch = channel()` before go_blocks renames ch in a go block and system_signals reads the send
	crate::channel_words::lower,
	// `input{type="text"}`: HTML's attribute form is the attribute (markup_tags.rs), before soft_keywords refuses `class = …`
	crate::markup_tags::lower_html_attributes,
	// P165: a hard keyword redefined, a soft one defined at the top level, before any pass gives the word its meaning
	crate::soft_keywords::lower,
	// `test C` and `test "name" { … }` run under `warp test` only, before library_words lowers their checks and tries
	crate::lowering::test_blocks::lower,
	// P179: `red is Color` of a variant without payload, before any pass lowers the type test
	crate::lowering::sum_variants::lower,
	// `global n = 5` in a function body is `global n; n = 5` before any pass reads its `global n`
	crate::late_binding::split_global_assignments,
	// `xs.keep only positive` is `xs where it > 0`, before lower_where reads it (list_phrases.rs)
	crate::list_phrases::lower,
	// `xs where it > 1` before welcome_forms reads its words and a function's `it` is read as its parameter
	crate::comprehensions::lower_where,
	// `go { … }` before any pass reads into the block (go_blocks.rs)
	crate::go_blocks::lower,
	// `$a` / `$1` references of data literals (references.rs) before a pass takes `{ parent=$1 }` for a Swift closure
	crate::references::lower,
	// `serve 8080 { get "/" {…} }`: route functions and the serving call (serve.rs), before any pass reads `get` as a call
	crate::serve::lower,
	// `.card { padding: 8px }` in a style block: the selector and the length as texts (style_rules.rs), before markup_tags
	// reads `p.note{…}` as a tag
	crate::style_rules::lower,
	// `label(for:pwd):"Password"` in a tag block is the tag label{for:pwd "Password"} (markup_tags.rs), before any pass
	// reads it as a call
	crate::markup_tags::lower,
	// `li{ transition: fade 200ms }`: words as data, the attribute data-warp-transition (transitions.rs), before any pass
	// reads fade as a variable
	crate::transitions::lower,
	// `dir(time)`: the names of the standard module (introspection.rs), before any pass reads time as a value
	crate::introspection::lower,
	// `f.effects`, `e.listeners`: the reflection words' dot forms (reflection.rs), before any pass reads them as fields
	crate::reflection::lower,
	// `root` is sqrt (word_operators.rs), before pipes.rs reads `2|square|root`
	crate::word_operators::lower,
	// `x | f` (pipes.rs) before any pass reads the or
	crate::pipes::lower,
	// the classes of used modules (`use shapes`, `use collections`) before class_methods lowers them with the program's
	// `class Foo;` takes in the definitions after it before any pass reads class bodies (wiki/type.md)
	crate::lowering::file_declarations::lower,
	// `60 mph` is `60 mi/h`, `q as km/h` converts to km/h, before any units pass reads them
	crate::units::lower_unit_words,
	// `5 m ± 1 cm; x * 2`: a run-time quantity with an interval amount (units.rs), before the units module is loaded for it
	crate::units::lower_run_time_tolerances,
	crate::modules::insert_module_classes,
	// `type Name = string` resolved in `name: Name` before class_methods reads the fields
	crate::type_aliases::lower,
	// `f(s:Shape)` of a trait takes any conforming instance: untyped before class_methods reads typed parameters
	crate::traits::lower_trait_parameters,
	// `people: [Person] = database.people`: the table's rows, inserts and field changes written through
	// (database_tables.rs), before class_methods lowers the class (its row id) and stored_values reads `database.people`
	crate::database_tables::lower,
	// `p.fields`, `p.methods`, `dir(p)`: the class layout (reflection.rs), before class_methods lowers the class bodies
	crate::reflection::lower_objects,
	// methods in a class body become functions over the class before any pass reads the body as fields
	crate::class_methods::lower,
	// `calc.exports` of a component (reflection.rs) before foreign_modules makes it a call into the component
	crate::reflection::lower_component_words,
	// first: `math.sqrt(2)` of `use python math` is no method call of the built-in word
	crate::foreign_modules::lower,
	// `x as text?` keeps ø, after foreign_modules types a nullable WebIDL result so
	crate::optional_casts::lower,
	crate::phrase_calls::lower,
	crate::std_aliases::lower, crate::class_methods::lower_json_classes, crate::welcome_forms::lower, crate::analyzer::lower_kebab_members, crate::number_keys::lower,
	// `{ a: 1, b: 2\n c: 3 }`: one row of fields (object_groups.rs)
	crate::object_groups::lower,
	// the used modules join the program before the signal passes: a listener of the program sees the writes of their functions
	crate::routes::lower, crate::page_html::use_markup, crate::modules::resolve,
	// `fourty_two.exports`, `dir(fourty_two)` of an imported core module, once resolve found its file (reflection.rs)
	crate::reflection::lower_module_words,
	crate::uncertain::lower_certainty, crate::units::lower_sleep_durations, crate::units::lower_quantity_comparisons, crate::stored_values::lower, crate::undo_history::lower, crate::declarations::lower_tasks, crate::system_values::name, crate::signal_values::poll_shared, crate::system_values::read, crate::shared_arrays::lower, crate::fetch_signals::lower, crate::system_signals::lower, crate::component_state::lower, crate::element_events::lower, crate::event_signals::lower, crate::page_html::lower, crate::signal_values::subscribe, crate::variable_signals::lower, crate::signal_values::lower, crate::declarations::lower_c_functions, crate::declarations::lower_bare_declarations, crate::declarations::lower_spaced_definitions, crate::lowering::number_words::lower, crate::parameter_shapes::lower, crate::ruby_blocks::lower, crate::declarations::lower_sized_arrays, crate::result_word::lower, crate::picked_calls::lower, crate::variadic::lower, crate::nonlocal_cells::lower_lambdas, crate::named_arguments::lower, crate::comprehensions::lower, crate::library_words::lower_function_methods, crate::tuples::lower, crate::run_time_blocks::warn_unresolved, crate::run_time_blocks::lower_interpret, crate::blocks::lower, crate::getters::lower, crate::run_time_blocks::lower_run_time_bangs, crate::mutation::warn_discarded, crate::mutation::lower, crate::nested_index::lower, crate::field_elements::lower, crate::host::lower_aliases,
	// again: the getters of the modules used, which lower_module_source leaves for here, and the program's reads of them
	crate::getters::lower,
	crate::type_name_matching::lower, crate::meta_entries::lower, crate::versions::lower_versions,
	crate::analyzer::lower_negated_calls,
];

/// The passes after the constant answers (time, units, reals), in order: types and traits, lambdas and closures, words
const MEANING_PASSES: [fn(Node) -> Node; 29] = [
	// first: the run-time item checks of declared lists see `names.add(v)` before any pass lowers the append
	crate::lowering::list_element_checks::lower,
	crate::lazy_ranges::lower, crate::declarations::resolve_tasks, crate::traits::lower_declarations, crate::type_tests::lower, crate::ambiguous_forms::lower, crate::analyzer::lower_list_times,
	// before any pass reads a call's arity: `square square 2` nests, `f(a)` and `f(a, b)` become `f·1`, `f·2`
	crate::broadcasting::lower_scalar_element_wise, crate::broadcasting::lower_prefix_calls, crate::overloads::lower_arity_overloads,
	crate::broadcasting::lower, crate::library_words::lower_count_in, crate::lambdas::lower, crate::function_values::lower, crate::closures::lower, crate::lambdas::lower_strict, crate::broadcasting::lower_several_arguments, crate::real::lower,
	crate::type_constructor::lower, crate::printable::lower, crate::overloads::lower, crate::traits::lower_conformances, crate::min_max::lower,
	crate::declarations::lower, crate::switch::lower, crate::phrase_words::lower, crate::library_words::lower,
	crate::traits::lower_dispatch, crate::memoization::lower,
];

/// A module's source after the passes the program ran before its `use` was resolved (those before modules::resolve):
/// its definitions join the program in the same forms (`[w for w in ws if …]` is lowered, not read as a list).
/// Not getters::lower: a getter `answer := 42` is lowered with the program, whose reads of it become calls.
pub(crate) fn lower_module_source(module: Node) -> Node {
	type Pass = fn(Node) -> Node;
	let (resolve, getters): (Pass, Pass) = (crate::modules::resolve, crate::getters::lower);
	let resolved_at = SOURCE_PASSES.iter().position(|pass| std::ptr::fn_addr_eq(*pass, resolve)).expect("modules::resolve is a source pass");
	SOURCE_PASSES[..resolved_at].iter().filter(|pass| !std::ptr::fn_addr_eq(**pass, getters)).fold(module, |module, pass| pass(module))
}

fn run_passes(node: Node, passes: &[fn(Node) -> Node]) -> Node {
	passes.iter().fold(node, |node, pass| pass(node))
}

/// Everything before code generation. `Err` is the final value of the program when it needs no module
/// (a constant answer, an error, a denied capability).
fn lower_for_emission(node: Node) -> Result<Node, Node> {
	use crate::effects::{without_constraints, EffectReport};

	if let Some(clash) = crate::analyzer::check_operator_word_functions(&node) {
		return Err(clash.into_error());
	}
	if let Some(misread) = crate::analyzer::check_upcast_fields(&node) {
		return Err(misread.into_error());
	}
	crate::diagnostic::report(&crate::accessibility::warnings(&node))?;
	crate::diagnostic::report(&crate::analyzer::source_warnings(&node))?;
	// emits and handler blocks as written: the passes lower them to calls, the named effects are read from them
	let as_written = node.clone();
	let node = run_passes(node, &SOURCE_PASSES);
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	if let Some(kind_change) = crate::analyzer::check_kind_changes(&node) {
		return Err(kind_change.into_error());
	}
	if let Some(unsaid) = crate::uncertain::check_orderings(&node) {
		return Err(unsaid.into_error());
	}
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
	// quantities in loops and branches: SI amounts at run time, the units checked here (static units, stage 1)
	let node = match crate::units::static_units::lower(&node) {
		Some(lowered) => lowered?,
		None => node,
	};
	if let Some(answer) = crate::real::answer(&node) {
		return Err(answer);
	}
	let node = run_passes(node, &MEANING_PASSES);
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	let effects = EffectReport::of(&node).with_events_of(&as_written);
	if let Some(answer) = effects.answer(&node) {
		return Err(answer);
	}
	let node = effects.answer_queries(node);
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	if let Some((name, capability)) = effects.denied(GRANTED.with(|granted| granted.get())) {
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
	compile_program(code, |program| program)
}

/// compile for a standalone executable (`warp build`): the program prints its value at the end, as `warp <file>`
/// shows it, since nobody reads the result of an executable
pub fn compile_printing_result(code: &str) -> Result<CompiledModule, Node> {
	PRINTS_RESULT.with(|prints| prints.set(false));
	// the routes first: a program ending with a route prints the page its path shows, not the route statement
	let compiled = compile_program(code, |program| printing_result(crate::routes::lower(program)));
	PRINTS_RESULT.with(|prints| prints.set(false));
	compiled
}

/// Whether the program compiled now ends in the print of its value that compile_printing_result added: that print
/// shows nothing for ø (a loop or a call that gives nothing), as `warp <file>` shows nothing; a print of the program's
/// own prints ø
pub fn prints_result() -> bool {
	PRINTS_RESULT.with(|prints| prints.get())
}

fn compile_program(code: &str, rewrite: fn(Node) -> Node) -> Result<CompiledModule, Node> {
	crate::diagnostic::begin_program();
	crate::diagnostic::in_program_mode(rewrite(lawful_program(code)?), |program| {
		let source = program.clone();
		let node = crate::folding::precompute(lower_for_emission(program)?);
		let reflected = crate::reflection::meta_entries(&source, &node, code);
		warn_about_run_time_blocks(&node)?;
		// the module's `warp.meta` section: a final quantity's unit, the program's functions and classes
		let entries: Vec<_> = crate::units::static_units::result_units_entry().into_iter().chain(reflected).collect();
		choose_module(&node).map(|module| CompiledModule { bytes: crate::meta_section::with_entries(module.bytes, entries), ..module })
	})
}

/// The program with its last statement `x` printed: `print(x)`; a declaration or a print stays as it is
fn printing_result(program: Node) -> Node {
	match program {
		Node::Meta { node, data } => Node::Meta { node: Box::new(printing_result(*node)), data },
		Node::List(mut statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			if let Some(last) = statements.pop() {
				statements.push(printing_result(last));
			}
			Node::List(statements, Bracket::None, separator)
		}
		Node::Empty => Node::Empty,
		statement if crate::modules::is_declaration(&statement) || crate::warp_parser::starts_print(&statement) => statement,
		value => {
			PRINTS_RESULT.with(|prints| prints.set(true));
			crate::warp_parser::print_call([value])
		}
	}
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

	// Fallback to standard Node encoding; a final quantity's unit travels in the module (`warp.meta`) and is read back
	match emit_module(&node) {
		Ok(module) => run_module(CompiledModule { bytes: crate::units::static_units::with_result_units(module.bytes), ..module }),
		Err(type_error) => type_error,
	}
}

/// The words of an exception (library_words lowers them to the call raise(X))
pub(crate) const RAISE_WORDS: [&str; 2] = ["raise", "throw"];

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
	let items = |list: &Node| match list.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	let block = spaced(block);
	let definitions: Vec<Node> = items(definitions).into_iter().map(spaced).collect();
	let mentioned = |name: &str| [&block].into_iter().chain(&definitions).any(|part| mentions(part, name));
	let bound: Vec<(String, Node)> = names.into_iter().zip(items(values)).filter(|(name, _)| mentioned(name)).collect();
	let program_of = |bindings: Vec<Node>| {
		let statements: Vec<Node> = bindings.into_iter().chain(definitions.iter().cloned()).chain([block.clone()]).collect();
		Node::List(statements, Bracket::None, Separator::Newline)
	};
	let as_written = program_of(bound.iter().map(|(name, value)| assignment(name, as_written(value.clone()))).collect());
	let result = match crate::effects::EffectReport::of(&as_written).denied(&crate::effects::Capability::GRANTED_UNTRUSTED) {
		Some((external, capability)) => crate::node::error(&format!(
			"a block run at run time is pure: {external} needs the {} capability, which it does not get", capability.name())),
		None => run_block_program(bound, program_of),
	};
	match result.drop_meta() {
		Node::Error(message) => Err(format!("the block {written} failed: {}", message.serialize().trim_matches('"'))),
		_ => Ok(result),
	}
}

fn assignment(name: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Assign, Box::new(value))
}

/// A value written into the block's program: a number, text or character as itself, anything else as data
fn as_written(value: Node) -> Node {
	match value.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::Empty => value,
		_ => Node::List(vec![Node::Symbol(crate::blocks::DATA_WORD.to_string()), value], Bracket::None, Separator::Space),
	}
}

/// Does `node` mention `name`: as a word, or inside a text (an interpolation)
fn mentions(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= match part {
		Node::Symbol(word) => word == name,
		Node::Text(text) => text.contains(name),
		_ => false,
	});
	found
}

/// Natively the block is compiled as a function of the numbers it reads: each is `block·value(i) as int` (or `float`),
/// exact ratios and big Ints included,
/// which the host gives from this run's values, so the same block with other numbers is the same program, compiled
/// once per thread (BLOCK_MODULES) and machine code once per machine (run/module_cache.rs). Other values are written in
#[cfg(feature = "native")]
fn run_block_program(bound: Vec<(String, Node)>, program_of: impl Fn(Vec<Node>) -> Node) -> Node {
	use crate::extensions::numbers::Number;
	use crate::tasks::TaskValue;
	let mut numbers = vec![];
	let bindings = bound.into_iter().map(|(name, value)| {
		// an exact ratio (`0.5` is 1/2) or big Int is an Int to the compiler: a handle the host composes (tasks.rs Builders)
		let (number, type_word) = match value.drop_meta() {
			Node::Number(Number::Float(x)) => (TaskValue::Float(*x), "float"),
			Node::Number(Number::Int(_) | Number::BigInt(_) | Number::Quotient(..) | Number::BigQuotient(_)) => match TaskValue::of(&value) {
				Ok(number) => (number, "int"),
				Err(_) => return assignment(&name, as_written(value)),
			},
			_ => return assignment(&name, as_written(value)),
		};
		let read = Node::List(vec![Node::Symbol(crate::host::BLOCK_VALUE.to_string()), Node::Number(Number::Int(numbers.len() as i64))], Bracket::Round, Separator::None);
		numbers.push(number);
		assignment(&name, Node::Key(Box::new(read), Op::As, Box::new(Node::Symbol(type_word.to_string()))))
	}).collect();
	let program = program_of(bindings);
	crate::host::with_block_values(numbers, || crate::diagnostic::in_program_mode(program, run_cached_block))
}

#[cfg(not(feature = "native"))]
fn run_block_program(bound: Vec<(String, Node)>, program_of: impl Fn(Vec<Node>) -> Node) -> Node {
	eval_parsed(program_of(bound.into_iter().map(|(name, value)| assignment(&name, as_written(value))).collect()), "")
}

/// The block programs compiled on this thread, by their text; past BLOCK_MODULES_KEPT they start over
#[cfg(feature = "native")]
const BLOCK_MODULES_KEPT: usize = 256;
#[cfg(feature = "native")]
thread_local! {
	static BLOCK_MODULES: std::cell::RefCell<std::collections::HashMap<String, CompiledModule>> = std::cell::RefCell::new(std::collections::HashMap::new());
	static COMPILED_BLOCKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many block programs this thread compiled (the cache's misses)
#[cfg(feature = "native")]
pub fn compiled_blocks() -> usize {
	COMPILED_BLOCKS.with(std::cell::Cell::get)
}

#[cfg(feature = "native")]
fn run_cached_block(program: Node) -> Node {
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
fn spaced(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::None) => Node::List(items.into_iter().map(spaced).collect(), Bracket::None, Separator::Space),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(spaced).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(spaced(*left)), op, Box::new(spaced(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(spaced(*node)), data },
		other => other,
	}
}

/// A block's value the host has no constructor for in the running module yet
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

/// Without wasmtime the embedding host runs the program (the browser playground: web.rs); a final quantity's unit is
/// read back from the module's `warp.meta` section (entry units), as wasm_reader does natively
#[cfg(not(feature = "native"))]
pub(crate) fn run_module(module: CompiledModule) -> Node {
	crate::units::static_units::with_module_units(&module.bytes, crate::web::run_in_host(&module.bytes))
}

/// The run used up its fuel: it probably does not terminate, or needs a larger budget
pub fn out_of_fuel(steps: u64) -> Node {
	crate::node::error(&format!(
		"out of fuel after {steps} steps: the program may not terminate (raise the budget with {}=<steps> or --fuel <steps>)",
		crate::util::FUEL_VARIABLE))
}
