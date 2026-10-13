//! The compile pipeline: parse → the lowering passes (`SOURCE_PHASES`, `MEANING_PASSES`) → analysis and diagnostics →
//! the WASM GC emitter (wasm_emitter) → run. `eval`, `compile` and `lower` are the entry points; wasm_emitter
//! re-exports them.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::WarpParser;
use crate::wasm_emitter::{emit_module, find_struct_instantiation, WasmGcEmitter};
#[cfg(feature = "native")]
use log::warn;

const INDEX_OUT_OF_RANGE: &str = "index out of range";
const UNWRAPPED_EMPTY: &str = "unwrapped ø";
const RANGE_END_HINT: &str = "hint: `..` excludes the end; `...` or `to` include it";
use crate::lowering::nodes::children_rewritten;
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
	if let Some(outcome) = crate::web::page_tests::answer(code) {
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
	let Node::Text(message) = reason.drop_meta() else {
		return result;
	};
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
	lines.extend(
		assumed
			.into_iter()
			.map(|(guess, positions)| format!("assumed at {}: {guess}", positions.join(", "))),
	);
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
		Some((external, _)) => crate::node::error(&format!(
			"untrusted code has no capabilities but pure libm, refusing to call {external}"
		)),
		None => with_granted(&crate::effects::Capability::GRANTED_UNTRUSTED, || {
			eval_parsed(program, code)
		}),
	}
}

thread_local! {
	/// What the program being compiled may call: eval's grant, or the untrusted one (checked again after lowering,
	/// which brings calls of its own: `use python math` → foreign_call)
	static GRANTED: std::cell::Cell<&'static [crate::effects::Capability]> = const { std::cell::Cell::new(&crate::effects::Capability::GRANTED_BY_EVAL) };
}

/// `run` with the program granted only what a sandboxed program gets (`warp --sandbox`, Capability::GRANTED_SANDBOXED)
pub fn sandboxed<T>(run: impl FnOnce() -> T) -> T {
	with_granted(&crate::effects::Capability::GRANTED_SANDBOXED, || {
		crate::sandbox::confined(run)
	})
}

/// `run` with the program granted only `granted`
fn with_granted<T>(granted: &'static [crate::effects::Capability], run: impl FnOnce() -> T) -> T {
	crate::util::scoped(&GRANTED, granted, run)
}

/// What the program compiled on this thread is compiled for: set by the `for_*` wrappers around a compile, read by the
/// lowering passes and (through wasm_emitter/config.rs) the emitter
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompileMode {
	/// for a page (`warp build --site`), where page events happen
	pub for_a_page: bool,
	/// for `warp dev`, whose page keeps the program's state across reloads
	pub for_dev: bool,
	/// a program that renders its own markup (rendering_itself)
	pub rendering_itself: bool,
	/// for WAGI (`warp build --wagi`): its routes answer the one request in its environment
	pub for_wagi: bool,
	/// the program ends in the print of its value that compile_printing_result added: that print shows nothing for ø (a
	/// loop or a call that gives nothing), as `warp <file>` shows nothing; a print of the program's own prints ø
	pub prints_result: bool,
	/// the page is compiled to render its first HTML where the server's code is (site.rs, headless.rs)
	pub prerendering: bool,
	/// a module for any host (`warp compile --wasm`): a browser's host reads its values too
	pub for_any_host: bool,
	/// hosted (`warp deploy`): a program without routes gets them (lowering/serve.rs with_default_routes)
	pub for_hosting: bool,
	/// it runs under `warp test`: its tests run and give its value (lowering/test_blocks.rs)
	pub for_tests: bool,
}

impl CompileMode {
	/// a plain run: none of the flags
	pub const PLAIN: CompileMode = CompileMode {
		for_a_page: false,
		for_dev: false,
		rendering_itself: false,
		for_wagi: false,
		prints_result: false,
		prerendering: false,
		for_any_host: false,
		for_hosting: false,
		for_tests: false,
	};

	/// Whether the program exports its renderer: a page's, or one rendering itself
	pub fn renders_itself(&self) -> bool {
		self.for_a_page || self.rendering_itself
	}
}

thread_local! {
	static COMPILE_MODE: std::cell::Cell<CompileMode> = const { std::cell::Cell::new(CompileMode::PLAIN) };
	/// the values the page's calls of server functions gave its prerender, the shipped page's first values
	static SERVER_VALUES: std::cell::RefCell<Vec<Node>> = const { std::cell::RefCell::new(vec![]) };
	/// the port `warp serve` serves a program at that has no `serve PORT {…}` of its own (lowering/serve.rs)
	static SERVING_PORT: std::cell::Cell<Option<u16>> = const { std::cell::Cell::new(None) };
}

/// What the program compiled now is compiled for
pub fn compile_mode() -> CompileMode {
	COMPILE_MODE.with(std::cell::Cell::get)
}

fn set_mode_flag(flag: fn(&mut CompileMode) -> &mut bool, value: bool) -> bool {
	COMPILE_MODE.with(|current| {
		let mut mode = current.get();
		let before = std::mem::replace(flag(&mut mode), value);
		current.set(mode);
		before
	})
}

/// `run` with one flag of the compile mode set, then that flag as it was (the others keep what `run` set)
fn with_flag<T>(flag: fn(&mut CompileMode) -> &mut bool, run: impl FnOnce() -> T) -> T {
	struct Restore(fn(&mut CompileMode) -> &mut bool, bool);
	impl Drop for Restore {
		fn drop(&mut self) {
			set_mode_flag(self.0, self.1);
		}
	}
	let _restore = Restore(flag, set_mode_flag(flag, true));
	run()
}

thread_local! {
	/// The functions the component compiled on this thread exports and imports (`warp build --component`)
	static COMPONENT_FUNCTIONS: std::cell::RefCell<crate::lowering::component_worlds::WorldFunctions> = std::cell::RefCell::new(Default::default());
}

/// `run` compiling a program as a component exporting and importing `functions`: the emitter adds the canonical-ABI
/// adapters of the exports and imports the imports from the core modules the world names
pub fn for_a_component<T>(functions: crate::lowering::component_worlds::WorldFunctions, run: impl FnOnce() -> T) -> T {
	crate::util::scoped(&COMPONENT_FUNCTIONS, functions, run)
}

/// The functions the component being compiled exports, none for a program
pub fn component_exports() -> Vec<(String, crate::lowering::component_worlds::Signature)> {
	COMPONENT_FUNCTIONS.with(|current| current.borrow().exports.clone())
}

/// The functions the component being compiled imports, none for a program
pub fn component_imports() -> Vec<(String, crate::lowering::component_worlds::Signature)> {
	COMPONENT_FUNCTIONS.with(|current| current.borrow().imports.clone())
}

/// `run` compiling a program for a page: its page event handlers are expected, not warned about; it exports the
/// reflection getters the page's host reads values with
pub fn for_a_page<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.for_a_page, run)
}

/// `run` compiling a program for WAGI (Spin, Akamai Functions): lowering/serve.rs ends it in the answer of its routes
/// to the request (lib/wagi.warp), and it exports `_start`
pub fn for_wagi<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.for_wagi, run)
}

/// `run` compiling a module for any host (`warp compile --wasm`): it exports the reflection getters a host without GC
/// field access (the browser) reads values with, so a browser program importing it copies values in and out
pub fn for_any_host<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.for_any_host, run)
}

/// `run` compiling a hosted program (`warp deploy`): without routes of its own, its value is the page at / and each of
/// its functions a route
pub fn for_hosting<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.for_hosting, run)
}

/// `run` compiling a page that calls its server functions directly, as the server does (lowering/serve.rs): for its
/// first HTML, rendered at build time
pub fn prerendering<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.prerendering, run)
}

/// `run` compiling the shipped page, whose calls of server functions start as `values`
pub fn with_server_values<T>(values: Vec<Node>, run: impl FnOnce() -> T) -> T {
	crate::util::scoped(&SERVER_VALUES, values, run)
}

/// `run` compiling a program `warp serve` serves at the port: its server functions, its top-level `get`/`post` routes
/// and its page, without a `serve PORT {…}` statement
pub fn serving_at<T>(port: u16, run: impl FnOnce() -> T) -> T {
	crate::util::scoped(&SERVING_PORT, Some(port), run)
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
	with_flag(|mode| &mut mode.rendering_itself, run)
}

/// `run` compiling a program for `warp dev`: the main-level variables it changes are kept (lowering/stored_values.rs)
pub fn for_dev<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.for_dev, run)
}

/// `run` a program under `warp test`: its `test` lines and blocks run, its value is their summary (P209, P210)
pub fn for_tests<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|mode| &mut mode.for_tests, run)
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

type Pass = fn(Node) -> Node;

/// The passes over the source forms, in order, each reading what the one before it left: definitions and sugar become
/// the forms every later pass knows (`def f(x) {…}` is `f(x) := {…}`), modules are resolved; one phase after the other
const SOURCE_PHASES: [&[Pass]; 5] = [
	SUGAR_PASSES,
	DECLARATION_PASSES,
	MODULE_PASSES,
	SIGNAL_PASSES,
	FUNCTION_PASSES,
];
/// The phases a used module's source runs before it joins the program (lower_module_source)
const MODULE_SOURCE_PHASES: [&[Pass]; 2] = [SUGAR_PASSES, DECLARATION_PASSES];

/// The written forms as the parser keeps them: sugar, markup and web words, blocks, before any pass reads a declaration
const SUGAR_PASSES: &[Pass] = &[
	// `"a \(x) b"` → `"a " + text_form(x) + " b"` (interpolation.rs) first, so every pass reads the holes as code
	crate::interpolation::lower_program,
	// `n times {…}`, `n times [x]`, `n times "ab"` as the parser keeps them: a loop, a filled list, a repeated text
	crate::lowering::times_phrases::lower,
	// `xs .* 2` as the parser keeps it: the operator applied to each element (broadcasting.rs)
	crate::lowering::broadcasting::lower_element_wise_operators,
	// a shader's `$name` holes (shader_holes.rs) as `name: name` in the values map of each paint of it
	crate::graphics::shader_holes::lower,
	crate::lowering::declaration_forms::lower_inline_unions,
	// `on ask {…} in {…}` before any pass reads `{…} in {…}` as membership or an emit as nothing
	crate::lowering::scoped_handlers::lower,
	// `component name {…}` declares a WIT world (`warp build --wit`) and does nothing at run time
	crate::lowering::component_worlds::lower,
	// `ch.send(v)` of `ch = channel()` before go_blocks renames ch in a go block and system_signals reads the send
	crate::lowering::channel_words::lower,
	// `input{type="text"}`: HTML's attribute form is the attribute (markup_tags.rs), before soft_keywords refuses `class = …`
	// `form post "/todos" { … }` is `form{ method:"post" action:"/todos" … }` (markup_tags.rs), before serve reads its
	// `post "/todos" { … }` as a route
	crate::lowering::markup_tags::lower_form_routes,
	crate::lowering::markup_tags::lower_html_attributes,
	// P165: a hard keyword redefined, a soft one defined at the top level, before any pass gives the word its meaning
	crate::lowering::soft_keywords::lower,
	// `add_pair([a, b]) := a + b`, `sum(Point{x, y}) := …`: a pattern parameter is a match on it (parameter_patterns.rs)
	crate::lowering::parameter_patterns::lower,
	// `fib(0) := 0; fib(n) := …`: a second definition of one shape is an error with a match as its fix
	crate::lowering::definition_cases::lower,
	// `p: position`, `i: index` (index_positions.rs) checked as ints, before any pass reads a declaration
	crate::lowering::index_positions::mark_declarations,
	// `int age = 30` is `age: int = 30` before the passes that read declarations, the signal passes among them
	crate::lowering::type_name_matching::lower_prefixed_declarations,
	// `test C` and `test "name" { … }` run under `warp test` only, before library_words lowers their checks and tries
	crate::lowering::test_blocks::lower,
	// card plain-enum: `enum Coin { Penny, Dime }` is the sum type of its cases, `Coin.Dime` is Dime, before sum_variants
	crate::lowering::plain_enums::lower,
	// P179: `red is Color` of a variant without payload, before any pass lowers the type test
	crate::lowering::sum_variants::lower,
	// card match-exhaustive: a match on a declared sum type or enum that misses a case, while its arms are as written
	crate::lowering::match_exhaustive::check,
	// `global n = 5` in a function body is `global n; n = 5` before any pass reads its `global n`
	crate::lowering::late_binding::split_global_assignments,
	// `xs from #2 to #4`, `xs up to second`: a slice by position (word_slices.rs), before a pass reads `to` as a range
	crate::lowering::word_slices::lower,
	// `xs.keep only positive` is `xs where it > 0`, before lower_where reads it (list_phrases.rs)
	crate::lowering::list_phrases::lower,
	// a hosted program without routes: its value the page at /, each function a route (card make-deploy)
	crate::lowering::serve::with_default_routes,
	// `post "/todos/:id" { todos where it.id == id }`: the path's parameters bound before lower_where reads the variables
	crate::lowering::serve::bind_path_parameters,
	// `Pair = [_, _]` names a pattern, its uses are the pattern before lower_where reads `it is Pair` (card patterns-values)
	crate::lowering::named_patterns::lower,
	// `tree find {type: "image", src}`: a recursive walker of its own (tree_find.rs), lowered like written code
	crate::lowering::tree_find::lower,
	// `xs where it > 1` before welcome_forms reads its words and a function's `it` is read as its parameter
	crate::lowering::comprehensions::lower_where,
	// `go { … }` before any pass reads into the block (go_blocks.rs)
	crate::lowering::music_words::lower_live_loops,
	crate::lowering::go_blocks::lower,
	// `$a` / `$1` references of data literals (references.rs) before a pass takes `{ parent=$1 }` for a Swift closure
	crate::lowering::references::lower,
	// `serve 8080 { get "/" {…} }`: route functions and the serving call (serve.rs), before any pass reads `get` as a call
	crate::lowering::serve::lower,
	// `.card { padding: 8px }` in a style block: the selector and the length as texts (style_rules.rs), before markup_tags
	// reads `p.note{…}` as a tag
	crate::lowering::style_rules::lower,
	// `label{ for: name "Name" }`: the value ends before the content after it (glued_attributes.rs), before the options
	// of a select are read
	crate::lowering::glued_attributes::lower,
	// `select{ bind: fruit; ["apple" "pear"] }`: its options spelled out (select_options.rs), before markup_tags makes the
	// loop over a list variable's options a comprehension and any pass reads the bind
	crate::lowering::select_options::lower,
	// `label(for:pwd):"Password"` in a tag block is the tag label{for:pwd "Password"} (markup_tags.rs), before any pass
	// reads it as a call
	crate::lowering::markup_tags::lower,
	// `li{ transition: fade 200ms }`: words as data, the attribute data-warp-transition (transitions.rs), before any pass
	// reads fade as a variable
	crate::lowering::transitions::lower,
	// `dir(time)`: the names of the standard module (introspection.rs), before any pass reads time as a value
	crate::lowering::introspection::lower,
	// `f.effects`, `e.listeners`: the reflection words' dot forms (reflection.rs), before any pass reads them as fields
	crate::lowering::reflection::lower,
	// `root` is sqrt (word_operators.rs), before pipes.rs reads `2|square|root`
	crate::lowering::word_operators::lower,
	// `x | f` (pipes.rs) before any pass reads the or
	crate::lowering::pipes::lower,
];

/// Declarations: files, units, type aliases, traits, classes and their methods, foreign modules, the used modules' words
const DECLARATION_PASSES: &[Pass] = &[
	// the classes of used modules (`use shapes`, `use collections`) before class_methods lowers them with the program's
	// `class Foo;` takes in the definitions after it before any pass reads class bodies (wiki/type.md)
	crate::lowering::file_declarations::lower,
	// `60 mph` is `60 mi/h`, `q as km/h` converts to km/h, before any units pass reads them
	crate::units::lower_unit_words,
	// `class Part{length: m ± 1 mm}`: `Part(5 m)` holds `5 m ± 1 mm` (unit_fields.rs), a tolerance the next pass reads
	crate::units::static_units::lower_field_tolerances,
	// `5 m ± 1 cm; x * 2`: a run-time quantity with an interval amount (units.rs), before the units module is loaded for it
	crate::units::lower_run_time_tolerances,
	crate::modules::insert_module_classes,
	// `type Name = string` resolved in `name: Name` before class_methods reads the fields
	crate::lowering::type_aliases::lower,
	// `f(s:Shape)` of a trait takes any conforming instance: untyped before class_methods reads typed parameters
	crate::lowering::traits::lower_trait_parameters,
	// `people: [Person] = database.people`: the table's rows, inserts and field changes written through
	// (database_tables.rs), before class_methods lowers the class (its row id) and stored_values reads `database.people`
	crate::lowering::database_tables::lower,
	// `p.fields`, `p.methods`, `dir(p)`: the class layout (reflection.rs), before class_methods lowers the class bodies
	crate::lowering::reflection::lower_objects,
	// `y certainly < x` is `certainly(y < x)` before class_methods compares the amounts of run-time quantities
	crate::uncertain::lower_certainty,
	// methods in a class body become functions over the class before any pass reads the body as fields
	// `for x in c` of an object with next() (generators.rs) before class_methods lowers the next() calls
	crate::lowering::generators::lower_iterators,
	crate::lowering::class_methods::lower,
	// `calc.exports` of a component (reflection.rs) before foreign_modules makes it a call into the component
	crate::lowering::reflection::lower_component_words,
	// first: `math.sqrt(2)` of `use python math` is no method call of the built-in word
	crate::lowering::foreign_modules::lower,
	// `x as text?` keeps ø, after foreign_modules types a nullable WebIDL result so
	crate::lowering::optional_casts::lower,
	crate::lowering::phrase_calls::lower,
	crate::lowering::std_aliases::lower,
	crate::lowering::class_methods::lower_json_classes,
	crate::lowering::welcome_forms::lower,
	crate::lowering::kebab_names::lower_kebab_members,
	crate::lowering::number_keys::lower,
	// `{ a: 1, b: 2\n c: 3 }`: one row of fields (object_groups.rs)
	crate::lowering::object_groups::lower,
	// `{a:1}` becomes the map of its one entry where a map is needed (one_entry_maps.rs), before a pass lowers m.pop(k)
	crate::lowering::one_entry_maps::lower,
	// the used modules join the program before the signal passes: a listener of the program sees the writes of their functions
	// note names (`play F#4`) while the program is its own, before the sound module joins it
	crate::lowering::music_words::lower,
	crate::lowering::routes::lower,
	crate::lowering::page_html::use_markup,
];

/// The used modules join the program (modules::resolve), then the passes that read their words again
const MODULE_PASSES: &[Pass] = &[
	crate::modules::resolve,
	// again: a used module's phrases, `play 440Hz for 0.5s` of lib/sound.warp's `to play x for d:`
	crate::lowering::phrase_calls::lower,
	// `fourty_two.exports`, `dir(fourty_two)` of an imported core module, once resolve found its file (reflection.rs)
	crate::lowering::reflection::lower_module_words,
	// again: `dir(is_square)`, `is_square.params` of a used module's function, which joined the program in resolve
	crate::lowering::reflection::lower,
];

/// Stored values, system values, signals and events, pages
const SIGNAL_PASSES: &[Pass] = &[
	crate::units::lower_sleep_durations,
	crate::units::lower_quantity_comparisons,
	crate::lowering::stored_values::lower,
	crate::lowering::mutation::lower_deletes,
	crate::lowering::undo_history::lower,
	crate::lowering::declarations::lower_tasks,
	crate::lowering::reflection::lower_unknown_dir,
	// `held_key == LEFT`: the key words, before system_values reads `held_key`
	crate::lowering::key_names::lower_held_comparisons,
	crate::lowering::system_values::name,
	crate::lowering::signal_values::poll_shared,
	crate::lowering::system_values::read,
	crate::lowering::shared_arrays::lower,
	crate::lowering::fetch_signals::lower,
	// `on key { if key == ESC … }`: the pressed key and the key words, before system_signals reads the key handlers
	crate::lowering::key_names::lower,
	crate::lowering::system_signals::lower,
	crate::lowering::element_events::lower_binds,
	crate::lowering::component_state::lower,
	crate::lowering::element_events::lower,
	crate::lowering::event_signals::lower,
	crate::lowering::page_html::lower,
	crate::lowering::signal_values::subscribe,
	crate::lowering::variable_signals::lower,
	crate::lowering::signal_values::lower,
];

/// Functions and their calls: declarations, parameters, blocks, mutation, getters
const FUNCTION_PASSES: &[Pass] = &[
	crate::lowering::declarations::lower_c_functions,
	crate::lowering::declarations::lower_bare_declarations,
	crate::lowering::declarations::lower_spaced_definitions,
	crate::lowering::number_words::lower,
	crate::lowering::parameter_shapes::lower,
	crate::lowering::ruby_blocks::lower,
	crate::lowering::generators::lower,
	crate::lowering::declarations::lower_sized_arrays,
	crate::lowering::result_word::lower,
	crate::lowering::picked_calls::lower,
	crate::lowering::variadic::lower,
	crate::lowering::nonlocal_cells::lower_lambdas,
	crate::lowering::named_arguments::lower,
	crate::lowering::comprehensions::lower,
	crate::lowering::library_words::lower_function_methods,
	crate::lowering::tuples::lower,
	// `parse("n * 1")` of a constant text is `data (n * 1)` (card parse-word), before blocks.rs reads data
	crate::lowering::parse_word::lower,
	crate::lowering::run_time_blocks::warn_unresolved,
	crate::lowering::run_time_blocks::lower_interpret,
	crate::lowering::blocks::lower,
	crate::lowering::getters::lower,
	crate::lowering::run_time_blocks::lower_run_time_bangs,
	crate::lowering::mutation::warn_discarded,
	crate::lowering::mutation::lower,
	crate::lowering::nested_index::lower,
	crate::lowering::field_elements::lower,
	crate::host::lower_aliases,
	// again: the getters of the modules used, which lower_module_source leaves for here, and the program's reads of them
	crate::lowering::getters::lower,
	crate::lowering::type_name_matching::lower,
	crate::lowering::meta_entries::lower,
	crate::lowering::versions::lower_versions,
	crate::lowering::negated_calls::lower_negated_calls,
	// `lo(a:number) := a.low; lo(5 ± 1)`: a number parameter of a program with ± values holds one (uncertain.rs)
	crate::uncertain::lower_interval_parameters,
];

/// The passes after the constant answers (time, units, reals), in order: types and traits, lambdas and closures, words
const MEANING_PASSES: &[Pass] = &[
	// first: the run-time item checks of declared lists see `names.add(v)` before any pass lowers the append
	crate::lowering::list_element_checks::lower,
	crate::lowering::lazy_ranges::lower,
	crate::lowering::declarations::resolve_tasks,
	crate::lowering::traits::lower_declarations,
	crate::lowering::type_tests::lower,
	crate::lowering::ambiguous_forms::lower,
	crate::lowering::declaration_forms::lower_list_times,
	// before any pass reads a call's arity: `square square 2` nests, `f(a)` and `f(a, b)` become `f·1`, `f·2`
	crate::lowering::broadcasting::lower_scalar_element_wise,
	crate::lowering::broadcasting::lower_prefix_calls,
	crate::lowering::overloads::lower_arity_overloads,
	crate::lowering::broadcasting::lower,
	crate::lowering::library_words::lower_count_in,
	crate::lowering::lambdas::lower,
	crate::lowering::function_values::lower,
	crate::lowering::closures::lower,
	crate::lowering::lambdas::lower_strict,
	crate::lowering::broadcasting::lower_several_arguments,
	crate::real::lower,
	crate::lowering::type_constructor::lower,
	crate::lowering::printable::lower,
	crate::lowering::overloads::lower,
	crate::lowering::traits::lower_conformances,
	crate::lowering::min_max::lower,
	crate::lowering::declarations::lower,
	crate::lowering::switch::lower,
	crate::lowering::phrase_words::lower,
	crate::lowering::library_words::lower,
	// `xs[xs.index_of(x)]`: a position meeting brackets, an index meeting `#` (index_positions.rs), once index_of is named
	crate::lowering::index_positions::lower,
	crate::lowering::number_keys::lower_key_words,
	crate::lowering::traits::lower_dispatch,
	crate::lowering::memoization::lower,
];

/// A module's source after the passes the program ran before its `use` was resolved (MODULE_SOURCE_PHASES):
/// its definitions join the program in the same forms (`[w for w in ws if …]` is lowered, not read as a list).
/// Not the program's own passes: getters::lower (a getter `answer := 42` is lowered with the program, whose reads of it
/// become calls) and serve::with_default_routes (a module's functions are no routes of a hosted program).
pub(crate) fn lower_module_source(module: Node) -> Node {
	let program_only: [Pass; 2] = [
		crate::lowering::getters::lower,
		crate::lowering::serve::with_default_routes,
	];
	MODULE_SOURCE_PHASES
		.iter()
		.flat_map(|phase| phase.iter())
		.filter(|pass| !program_only.iter().any(|own| std::ptr::fn_addr_eq(**pass, *own)))
		.fold(crate::lowering::for_loop::headers::lower(module), |module, pass| {
			pass(module)
		})
}

fn run_passes(node: Node, passes: &[Pass]) -> Node {
	passes.iter().fold(node, |node, pass| pass(node))
}

/// Everything before code generation. `Err` is the final value of the program when it needs no module
/// (a constant answer, an error, a denied capability).
fn lower_for_emission(node: Node) -> Result<Node, Node> {
	use crate::effects::{without_constraints, EffectReport};
	// the loop headers the parser keeps as written (`for int in xs`), given their meaning before any check reads the loops
	let node = crate::lowering::for_loop::headers::lower(node);

	if let Some(clash) = crate::analyzer::check_operator_word_functions(&node) {
		return Err(clash.into_error());
	}
	if let Some(clash) = crate::units::check_unit_parameter_clashes(&node) {
		return Err(clash.into_error());
	}
	if let Some(misread) = crate::analyzer::check_upcast_fields(&node) {
		return Err(misread.into_error());
	}
	if let Some(unfillable) = crate::lowering::class_methods::check_self_typed_fields(&node) {
		return Err(unfillable.into_error());
	}
	crate::diagnostic::report(&crate::accessibility::warnings(&node))?;
	crate::diagnostic::report(&crate::analyzer::source_warnings(&node))?;
	// emits and handler blocks as written: the passes lower them to calls, the named effects are read from them
	let as_written = node.clone();
	let node = SOURCE_PHASES.iter().fold(node, |node, phase| run_passes(node, phase));
	if let Some(error) = crate::diagnostic::first_located_error(&node) {
		return Err(error);
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
	let node = crate::time::lower(node);
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
	let node = run_passes(node, MEANING_PASSES);
	if let Some(error) = crate::diagnostic::first_located_error(&node) {
		return Err(error);
	}
	let effects = EffectReport::of(&node).with_events_of(&as_written);
	if let Some(answer) = effects.answer(&node) {
		return Err(answer);
	}
	let node = effects.answer_queries(node);
	if let Some(error) = crate::diagnostic::first_located_error(&node) {
		return Err(error);
	}
	if let Some((name, capability)) = effects.denied(GRANTED.with(|granted| granted.get())) {
		return Err(crate::node::error(&if crate::sandbox::is_sandboxed() {
			crate::sandbox::denial(&name, capability.name())
		} else {
			format!(
				"capability denied: {name} needs the {} capability, which eval does not grant",
				capability.name()
			)
		}));
	}
	let node = crate::analyzer::resolve_main_variable_assignments(crate::lowering::late_binding::lower(
		without_constraints(node),
	)?)?;
	if let Some(error) = crate::analyzer::diagnose(&node) {
		return Err(error);
	}
	crate::diagnostic::report(&crate::analyzer::lint(&node))?;
	Ok(crate::lowering::declaration_forms::indexed_parameter_copies(
		crate::lowering::inlining::lower(crate::lowering::declaration_forms::lower_declarations(
			crate::lowering::kebab_names::resolve_data_scope(node),
		)),
	))
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
	set_mode_flag(|mode| &mut mode.prints_result, false);
	// the routes first: a program ending with a route prints the page its path shows, not the route statement
	let compiled = compile_program(code, |program| printing_result(crate::lowering::routes::lower(program)));
	set_mode_flag(|mode| &mut mode.prints_result, false);
	compiled
}

fn compile_program(code: &str, rewrite: fn(Node) -> Node) -> Result<CompiledModule, Node> {
	crate::diagnostic::begin_program();
	crate::diagnostic::in_program_mode(rewrite(lawful_program(code)?), |program| {
		let source = program.clone();
		let node = crate::lowering::folding::precompute(crate::lowering::folding::refuse_result_misfits(
			lower_for_emission(program)?,
		)?);
		let reflected = crate::lowering::reflection::meta_entries(&source, &node, code);
		warn_about_run_time_blocks(&node)?;
		// the module's `warp.meta` section: a final quantity's unit, the program's functions and classes
		let entries: Vec<_> = crate::units::static_units::result_units_entry()
			.into_iter()
			.chain(reflected)
			.collect();
		choose_module(&node).map(|module| CompiledModule {
			bytes: crate::meta_section::with_entries(module.bytes, entries),
			..module
		})
	})
}

/// The program with its last statement `x` printed: `print(x)`; a declaration or a print stays as it is
fn printing_result(program: Node) -> Node {
	match program {
		Node::Meta { node, data } => Node::Meta {
			node: Box::new(printing_result(*node)),
			data,
		},
		Node::List(mut statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			if let Some(last) = statements.pop() {
				statements.push(printing_result(last));
			}
			Node::List(statements, Bracket::None, separator)
		}
		Node::Empty => Node::Empty,
		statement if crate::modules::is_declaration(&statement) || crate::warp_parser::starts_print(&statement) => {
			statement
		}
		value => {
			set_mode_flag(|mode| &mut mode.prints_result, true);
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
		crate::host::RUN_BLOCK)).weak()])
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
		Some(bytes) => Ok(CompiledModule {
			bytes,
			needs_host: false,
			needs_wasi: false,
			needs_ffi: false,
		}),
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
		Ok(module) => checked_markup(run_module(CompiledModule {
			bytes: crate::units::static_units::with_result_units(module.bytes),
			..module
		})),
		Err(type_error) => type_error,
	}
}

/// A run's value; markup it built is checked for accessibility as it came out (card web-a11y-runtime)
fn checked_markup(result: Node) -> Node {
	if crate::markup::is_markup(&result) {
		crate::accessibility::check_rendered(&result);
	}
	result
}

/// The words of an exception (library_words lowers them to the call raise(X)), unless the program defines one
pub(crate) const RAISE_WORDS: [&str; 4] = ["raise", "throw", "fail", "panic"];

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
	let mentioned = |name: &str| {
		[&block]
			.into_iter()
			.chain(&definitions)
			.any(|part| mentions(part, name))
	};
	let bound: Vec<(String, Node)> = names
		.into_iter()
		.zip(values.as_items())
		.filter(|(name, _)| mentioned(name))
		.collect();
	let program_of = |bindings: Vec<Node>| {
		let statements: Vec<Node> = bindings
			.into_iter()
			.chain(definitions.iter().cloned())
			.chain([block.clone()])
			.collect();
		Node::List(statements, Bracket::None, Separator::Newline)
	};
	let as_written = program_of(
		bound
			.iter()
			.map(|(name, value)| assignment(name, as_written(value.clone())))
			.collect(),
	);
	let result =
		match crate::effects::EffectReport::of(&as_written).denied(&crate::effects::Capability::GRANTED_UNTRUSTED) {
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

fn assignment(name: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Assign, Box::new(value))
}

/// A value written into the block's program: a number, text or character as itself, anything else as data
fn as_written(value: Node) -> Node {
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
fn mentions(node: &Node, name: &str) -> bool {
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
fn run_block_program(bound: Vec<(String, Node)>, program_of: impl Fn(Vec<Node>) -> Node) -> Node {
	use crate::extensions::numbers::Number;
	use crate::tasks::TaskValue;
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
			assignment(
				&name,
				Node::Key(Box::new(read), Op::As, Box::new(Node::Symbol(type_word.to_string()))),
			)
		})
		.collect();
	let program = program_of(bindings);
	crate::host::with_block_values(numbers, || {
		crate::diagnostic::in_program_mode(program, run_cached_block)
	})
}

#[cfg(not(feature = "native"))]
fn run_block_program(bound: Vec<(String, Node)>, program_of: impl Fn(Vec<Node>) -> Node) -> Node {
	eval_parsed(
		program_of(
			bound
				.into_iter()
				.map(|(name, value)| assignment(&name, as_written(value)))
				.collect(),
		),
		"",
	)
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
		Node::List(items, Bracket::None, Separator::None) => {
			Node::List(items.into_iter().map(spaced).collect(), Bracket::None, Separator::Space)
		}
		other => children_rewritten(other, spaced),
	}
}

/// A block's value the host has no constructor for in the running module yet
pub fn cannot_hand_back(value: &Node) -> String {
	format!(
		"the block gave {}, a value run_block cannot hand back yet",
		value.serialize().trim()
	)
}

/// The message of `error "…"`, or of `raise …`, which always fails the run
pub(crate) fn returned_error_message(value: &Node) -> Option<&Node> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _)
			if items.len() == 2
				&& matches!(items[0].drop_meta(), Node::Symbol(word) if word == "error" || word == crate::wasm_emitter::text_builtins::RAISE) =>
		{
			Some(&items[1])
		}
		// `raise X` / `throw X` before library_words makes them the call raise(X)
		Node::List(items, Bracket::None, _)
			if items.len() == 2
				&& matches!(items[0].drop_meta(), Node::Symbol(word) if RAISE_WORDS.contains(&word.as_str())) =>
		{
			Some(&items[1])
		}
		_ => None,
	}
}

/// Run a compiled program with the linker its imports need
#[cfg(feature = "native")]
pub(crate) fn run_module(
	CompiledModule {
		bytes,
		needs_host,
		needs_wasi,
		needs_ffi,
	}: CompiledModule,
) -> Node {
	use crate::wasm_reader::{read_bytes_with_imports, Imports};
	let result = if needs_ffi || needs_wasi || needs_host {
		read_bytes_with_imports(
			&bytes,
			Imports {
				host: needs_host,
				wasi: needs_wasi,
				ffi: needs_ffi,
			},
		)
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
