//! The compile pipeline: parse → the lowering passes (`SOURCE_PHASES`, `MEANING_PASSES`) → analysis and diagnostics →
//! the WASM GC emitter (wasm_emitter) → run. `eval`, `compile` and `lower` are the entry points; wasm_emitter
//! re-exports them.

use crate::compiler::compile_context;
pub use crate::compiler::compile_mode::*;
use crate::node::{Bracket, Node, Separator};
use crate::warp_parser::WarpParser;
use crate::wasm_emitter::{emit_module, find_struct_instantiation, WasmGcEmitter};
#[cfg(feature = "native")]
use log::warn;

use crate::errors::run_time::{explain_runtime_error, has_exclusive_range};
#[cfg(feature = "native")]
use crate::run::wasm_reader::read_bytes;
#[cfg(feature = "native")]
use crate::wasm_emitter::{failed_run, run_raw_struct};

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
	crate::errors::diagnostic::begin_program(); // only the guesses made for this program explain its errors
	match lawful_program(code) {
		Ok(program) => {
			let exclusive_range = has_exclusive_range(&program);
			let result = crate::graphics::shader_errors::relocated(eval_parsed(program, code), code);
			explain_runtime_error(result, exclusive_range)
		}
		Err(violation) => violation,
	}
}

/// Parse the source and check its laws; `Err` is the violation.
fn lawful_program(code: &str) -> Result<Node, Node> {
	let unit_lines = crate::numbers::units::program_unit_lines(code);
	crate::compiler::compile_context::change(|context| context.program_unit_lines = unit_lines);
	let parse = || crate::compiler::self_hosted::parse(code).unwrap_or_else(|| WarpParser::parse(code));
	let parsed = crate::errors::diagnostic::in_source_mode(code, parse);
	let lawful = crate::law::separate_laws(crate::numbers::units::without_unit_lines(parsed)?);
	match crate::law::assert_laws(&lawful, code) {
		Some(violation) => Err(violation),
		None => Ok(lawful.program),
	}
}

/// Evaluate foreign data with an empty capability set: a program that would call any host, WASI or FFI
/// function is refused before it is compiled. To only read data, use `parse_data`, which evaluates nothing.
pub fn eval_untrusted(code: &str) -> Node {
	crate::errors::diagnostic::begin_program();
	let program = WarpParser::parse(code);
	match crate::analyzer::effects::EffectReport::of(&program)
		.denied(&crate::analyzer::effects::Capability::GRANTED_UNTRUSTED)
	{
		Some((external, _)) => crate::node::error(&format!(
			"untrusted code has no capabilities but pure libm, refusing to call {external}"
		)),
		None => with_granted(&crate::analyzer::effects::Capability::GRANTED_UNTRUSTED, || eval_parsed(program, code)),
	}
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
const SOURCE_PHASES: [&[Pass]; 5] = [SUGAR_PASSES, DECLARATION_PASSES, MODULE_PASSES, SIGNAL_PASSES, FUNCTION_PASSES];
/// The phases a used module's source runs before it joins the program (lower_module_source)
const MODULE_SOURCE_PHASES: [&[Pass]; 2] = [SUGAR_PASSES, DECLARATION_PASSES];

/// The written forms as the parser keeps them: sugar, markup and web words, blocks, before any pass reads a declaration
pub const SUGAR_PASSES: &[Pass] = &[
	// `"a \(x) b"` → `"a " + text_form(x) + " b"` (interpolation.rs) first, so every pass reads the holes as code
	crate::text::interpolation::lower_program,
	// `pattern => body` and `_ => v` arms of a match as `pattern: body` and `default: v`, the one form later passes read
	crate::lowering::welcome_forms::lower_match_arms,
	// a bare `return` as the phrase `return ø` (welcome_forms.rs), before any pass reads a return
	crate::lowering::welcome_forms::lower_bare_returns,
	// `x(p:P) -> int := …` is `x(p:P): int := …` (welcome_forms.rs) before any pass reads a definition's head
	crate::lowering::welcome_forms::lower_arrow_result_types,
	// `a of b` is `b.a` (of_lookups.rs), before any pass reads a lookup
	crate::lowering::of_lookups::lower,
	// `(p).a` is `p.a` (grouped_receivers.rs), before any pass reads a lookup
	crate::lowering::grouped_receivers::lower,
	// `window.width = 800`, `canvas.height`, a bare width (module_properties.rs): getters and setters, before any pass
	// reads a field's write; resolve brings draw for them
	crate::lowering::module_properties::lower,
	// `n times {…}`, `n times [x]`, `n times "ab"` as the parser keeps them: a loop, a filled list, a repeated text
	crate::lowering::times_phrases::lower,
	// `xs .* 2` as the parser keeps it: the operator applied to each element (broadcasting.rs)
	crate::lowering::broadcasting::lower_element_wise_operators,
	// a shader's `$name` holes (shader_holes.rs) as `name: name` in the values map of each paint of it
	crate::graphics::shader_holes::lower,
	// `Person as text := it.name` and `p as text` (conversions.rs) before any pass reads `as` as a cast
	crate::lowering::conversions::lower,
	// `cbrt(x: float): float from 'm'` (foreign_lines.rs) as the import it declares, before any pass reads it as a call
	crate::lowering::foreign_lines::lower,
	crate::lowering::inline_unions::lower_inline_unions,
	// `component name {…}` declares a WIT world (`warp build --wit`) and does nothing at run time
	crate::lowering::component_worlds::lower,
	// `ch.send(v)` of `ch = channel()` before go_blocks renames ch in a go block and system_signals reads the send
	crate::lowering::channel_words::lower,
	// `input{type="text"}`: HTML's attribute form is the attribute (markup_tags.rs), before soft_keywords refuses `class = …`
	// `form post "/todos" { … }` is `form{ method:"post" action:"/todos" … }` (markup_tags.rs), before serve reads its
	// `post "/todos" { … }` as a route
	crate::lowering::markup_tags::lower_form_routes,
	// `a{ href="/about" "About" }`, `Person{ class = "a" }`: `=` in a data block is an entry (tag_entries.rs), before
	// soft_keywords reads `class = …` as a definition
	crate::lowering::tag_entries::lower,
	// P165: a hard keyword redefined, a soft one defined at the top level, before any pass gives the word its meaning
	crate::lowering::soft_keywords::lower,
	// `add_pair([a, b]) := a + b`, `sum(Point{x, y}) := …`: a pattern parameter is a match on it (parameter_patterns.rs)
	crate::lowering::parameter_patterns::lower,
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
	crate::analyzer::match_exhaustive::check,
	// `global n = 5` in a function body is `global n; n = 5` before any pass reads its `global n`
	crate::lowering::late_binding::split_global_assignments,
	// a hosted program without routes: its value the page at /, each function a route (card make-deploy)
	crate::lowering::serve::with_default_routes,
	// a run without a server: each `get "/path" {…}` is also the page at its path (cards square-int, recognized-browser)
	crate::lowering::serve::with_page_routes,
	// `post "/todos/:id" { todos where it.id == id }`: the path's parameters bound before lower_where reads the variables
	crate::lowering::serve::bind_path_parameters,
	// `Pair = [_, _]` names a pattern, its uses are the pattern before lower_where reads `it is Pair` (card patterns-values)
	crate::lowering::named_patterns::lower,
	// `x is {host: h}`: h is the unit hour, a weak warning while it is still the word (card pattern-unit-name)
	crate::analyzer::unit_names::check_unit_names,
	// `tree find {type: "image", src}` and `tree replace p with q`: each a recursive walker of its own (tree_walks.rs)
	crate::lowering::tree_walks::lower,
	// `data($a + 1)` builds the code with a's value before any pass renames a, once tree walks are match arms (quasi_quote.rs)
	crate::lowering::quasi_quote::lower,
	// `xs where it > 1` before welcome_forms reads its words and a function's `it` is read as its parameter
	crate::lowering::comprehensions::lower_where,
	// `go { … }` before any pass reads into the block (go_blocks.rs)
	crate::lowering::music_words::lower_live_loops,
	crate::lowering::go_blocks::lower,
	// `$a` / `$1` references of data literals (references.rs) before a pass takes `{ parent=$1 }` for a Swift closure
	crate::lowering::references::lower,
	// `serve 8080 { get "/" {…} }`: route functions and the serving call (serve.rs), before any pass reads `get` as a call
	crate::lowering::serve::lower,
	// `xs map f`: an iteration word between a list and its function is the method call (lambdas.rs)
	crate::lowering::lambdas::lower_infix_iterations,
	// `xs.map(.floor)`: a leading-dot word is the lambda on its member (lambdas.rs), once no selector is left
	crate::lowering::lambdas::lower_member_lambdas,
	// `label(for:pwd):"Password"` in a tag block is the tag label{for:pwd "Password"} (markup_tags.rs), before any pass
	// reads it as a call
	crate::lowering::markup_tags::lower,
	// `li{ transition: fade 200ms }`: words as data, the attribute data-warp-transition (transitions.rs), before any pass
	// reads fade as a variable
	crate::lowering::transitions::lower,
	// `xs.insert 4` is `xs.insert(4)` (member_phrases.rs), after markup_tags read `p.note "hi"` as an element
	crate::lowering::member_phrases::lower,
	// `dir(time)`: the names of the standard module (introspection.rs), before any pass reads time as a value
	crate::lowering::introspection::lower,
	// `f.effects`, `e.listeners`: the reflection words' dot forms (reflection.rs), before any pass reads them as fields
	// `help(sound)`, `help(midi)`: a standard module's index, a word's head and comment (word_help.rs), before reflection.rs
	// answers help(sound) with the signature of the word sound
	// a function's leading text and the comment above it: its doc (doc_strings.rs), before reflection.rs reads `f.doc`
	crate::lowering::doc_strings::lower,
	crate::lowering::word_help::lower,
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
	// `class int` without a body is the int type (class_references.rs), before file_declarations takes `class Foo;`
	crate::lowering::class_references::lower,
	crate::lowering::file_declarations::lower,
	// `density: dpi` keeps the word dpi for messages and fixes (unit_fields.rs)
	crate::numbers::units::static_units::remember_field_unit_words,
	// `60 mph` is `60 mi/h`, `q as km/h` converts to km/h, before any units pass reads them
	crate::numbers::units::lower_unit_words,
	// `class Part{length: m ± 1 mm}`: `Part(5 m)` holds `5 m ± 1 mm` (unit_fields.rs), a tolerance the next pass reads
	crate::numbers::units::static_units::lower_field_tolerances,
	// `5 m ± 1 cm; x * 2`: a run-time quantity with an interval amount (units.rs), before the units module is loaded for it
	crate::numbers::units::lower_run_time_tolerances,
	crate::modules::insert_module_classes,
	// `type Name = string` resolved in `name: Name` before class_methods reads the fields
	crate::lowering::type_aliases::lower,
	// `f(t: type)` takes the type value itself (type_parameters.rs), before class_methods reads typed parameters
	crate::lowering::type_parameters::lower,
	// `f(s:Shape)` of a trait takes any conforming instance: untyped before class_methods reads typed parameters
	crate::lowering::traits::lower_trait_parameters,
	// `stored people: [Person]`: the table's rows, inserts and field changes written through
	// (database_tables.rs), before class_methods lowers the class (its row id) and stored_values reads `database.people`
	crate::lowering::database_tables::lower,
	// `p.fields`, `p.methods`, `dir(p)`: the class layout (reflection.rs), before class_methods lowers the class bodies
	crate::lowering::reflection::lower_objects,
	// `y certainly < x` is `certainly(y < x)` before class_methods compares the amounts of run-time quantities
	crate::numbers::uncertain::lower_certainty,
	// methods in a class body become functions over the class before any pass reads the body as fields
	// `for x in c` of an object with next() (for_loop.rs) before class_methods lowers the next() calls
	crate::lowering::for_loop::lower_iterators,
	crate::lowering::class_methods::lower,
	// `calc.exports` of a component (reflection.rs) before foreign_modules makes it a call into the component
	crate::lowering::reflection::lower_component_words,
	// page ids as names: `who` is document.getElementById("who"), `$who` its value, with `use js document`, which foreign_modules lowers
	crate::lowering::page_names::lower,
	// first: `math.sqrt(2)` of `use python math` is no method call of the built-in word
	crate::lowering::foreign_modules::lower,
	// `x as text?` keeps ø, after foreign_modules types a nullable WebIDL result so
	crate::lowering::optional_casts::lower,
	// `std::cmp::max(a, b)`, `String::from("hi")` (rust_paths.rs), before std_aliases reads a module call
	crate::lowering::rust_paths::lower,
	crate::lowering::std_aliases::lower,
	crate::lowering::class_methods::lower_json_calls,
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
	// `random(1, 6)`, `random int`: the prelude words they call, which resolve then loads, and only those
	crate::host::lower_random_forms,
	// `count(x) := 7`: the program's count·any for the values the library's count is not for (typed_library_words.rs)
	crate::lowering::typed_library_words::lower,
	crate::modules::resolve,
	// `fourty_two.exports`, `dir(fourty_two)` of an imported core module, once resolve found its file (reflection.rs)
	crate::lowering::reflection::lower_module_words,
	// again: `dir(is_square)`, `is_square.params` of a used module's function, which joined the program in resolve
	crate::lowering::reflection::lower,
	// `help(trick)`, `function(trick).dump` of a name known at run time: a table of the used modules' words (word_help.rs)
	crate::lowering::word_help::lower_named_words,
];

/// Stored values, system values, signals and events, pages
const SIGNAL_PASSES: &[Pass] = &[
	// `divides(3, 12)`: the call form of a declared word operator, its own or a used module's
	crate::lowering::word_operators::lower_call_forms,
	crate::numbers::units::lower_sleep_durations,
	// `now > 17h`, `now >= October`, `now ≈ 2026-10-14`: that part of now (partial_comparisons.rs), before units fold `time > 17h`
	crate::time::partial_comparisons::lower,
	crate::numbers::units::lower_quantity_comparisons,
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
	// `play 440Hz for: 0.5s` of `play(tone, for duration)`: `play(440Hz, duration: 0.5s)` (preposition_calls.rs), before any
	// pass reads the head's `for duration` or the call's words
	crate::lowering::preposition_calls::lower,
	crate::lowering::declarations::lower_c_functions,
	crate::lowering::declarations::lower_bare_declarations,
	crate::lowering::declarations::lower_spaced_definitions,
	crate::lowering::number_words::lower,
	crate::lowering::parameter_shapes::lower,
	crate::lowering::blocks::lower_trailing_blocks,
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
	crate::analyzer::unresolved_blocks::warn_unresolved,
	crate::lowering::run_time_blocks::lower_interpret,
	crate::lowering::blocks::lower,
	// again for the `data e` that parse_word and blocks made (lower_for_emission unseals them)
	crate::lowering::blocks::seal_quoted,
	crate::lowering::getters::lower,
	crate::lowering::run_time_blocks::lower_run_time_bangs,
	crate::analyzer::discarded_values::warn_discarded,
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
	crate::numbers::uncertain::lower_interval_parameters,
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
	crate::lowering::filled_lists::lower_list_times,
	// before any pass reads a call's arity: `square square 2` nests, `f(a)` and `f(a, b)` become `f·1`, `f·2`
	crate::lowering::broadcasting::lower_scalar_element_wise,
	crate::lowering::broadcasting::lower_prefix_calls,
	crate::lowering::overloads::lower_arity_overloads,
	crate::lowering::broadcasting::lower,
	crate::lowering::library_words::lower_count_in,
	// `room(signal)` of an instance whose type defines `call`, before the closure passes read it as a function value's call
	crate::lowering::traits::lower_instance_calls,
	// `each(ink, n)` of a function each calling its parameter: `each·ink(n)`, which calls ink by name (specialising.rs)
	crate::lowering::specialising::lower,
	crate::lowering::lambdas::lower,
	crate::lowering::function_values::lower,
	crate::lowering::closures::lower,
	// `x = a`, `a + 1` of a function a without parameters: its call (bare_calls.rs), once aliases and closures are named
	crate::lowering::bare_calls::lower,
	crate::lowering::lambdas::lower_strict,
	crate::lowering::broadcasting::lower_several_arguments,
	crate::numbers::real::lower,
	crate::lowering::type_constructor::lower,
	crate::lowering::printable::lower,
	crate::lowering::overloads::lower,
	crate::lowering::traits::lower_conformances,
	crate::lowering::min_max::lower,
	crate::lowering::declarations::lower,
	crate::lowering::switch::lower,
	crate::lowering::variadic::lower_list_spreads,
	crate::lowering::phrase_words::lower,
	crate::lowering::library_words::lower,
	// `count x.bytes` is `byte_size(x)`, after library_words wrote x.bytes as the prelude call
	crate::lowering::library_words::lower_byte_counts,
	// `xs[xs.index_of(x)]`: a position meeting brackets, an index meeting `#` (index_positions.rs), once index_of is named
	crate::lowering::index_positions::lower,
	crate::lowering::number_keys::lower_key_words,
	crate::lowering::traits::lower_dispatch,
	crate::lowering::memoization::lower,
];

/// A module's source after the passes the program ran before its `use` was resolved (MODULE_SOURCE_PHASES):
/// its definitions join the program in the same forms (`[w for w in ws if …]` is lowered, not read as a list).
/// Not the program's own passes: getters::lower (a getter `answer := 42` is lowered with the program, whose reads of it
/// become calls) serve::with_default_routes (a module's functions are no routes of a hosted program) and serve::with_page_routes.
pub(crate) fn lower_module_source(module: Node) -> Node {
	let program_only: [Pass; 3] = [
		crate::lowering::getters::lower,
		crate::lowering::serve::with_default_routes,
		crate::lowering::serve::with_page_routes,
	];
	MODULE_SOURCE_PHASES
		.iter()
		.flat_map(|phase| phase.iter())
		.enumerate()
		.filter(|(_, pass)| !program_only.iter().any(|own| std::ptr::fn_addr_eq(**pass, *own)))
		// `style(sheet: data)` of lib/markup.warp: its data parameters typed any, as a program's (typed_block_arguments.rs)
		.fold(
			crate::lowering::typed_block_arguments::lower(crate::lowering::for_loop::headers::lower(module)),
			|module, (index, pass)| {
				crate::compiler::phase_times::timed("module source passes", || {
					crate::compiler::phase_times::timed_as(|| format!("module source pass {index}"), || pass(module))
				})
			},
		)
}

/// The program just before and just after the source pass `pass`, the earlier passes run as compiling runs them: what
/// a pass's warp twin gets and must give (card twin-harness, tests/common/twins.rs pass_differential). None for a
/// program refused before the passes, or a pass that is no source pass.
pub fn around_source_pass(code: &str, pass: Pass) -> Option<(Node, Node)> {
	around_source_pass_reporting(code, pass).map(|(before, after, _)| (before, after))
}

/// `around_source_pass` with the hints and warnings the pass itself says, every one, printing none (card carries-line:
/// a twin reports them too)
pub fn around_source_pass_reporting(
	code: &str,
	pass: Pass,
) -> Option<(Node, Node, Vec<crate::errors::diagnostic::Diagnostic>)> {
	use crate::errors::diagnostic::{clear_shown_hints, every_hint_of, take_warnings};
	crate::errors::diagnostic::begin_program();
	let program = lawful_program(code).ok()?;
	crate::errors::diagnostic::in_program_mode(program, |program| {
		crate::lowering::nodes::numbering_fresh_names(|| {
			let program = crate::lowering::for_loop::headers::lower(program);
			let program = crate::lowering::data_items::lower(crate::lowering::blocks::seal_quoted(program));
			let passes: Vec<Pass> = SOURCE_PHASES.iter().flat_map(|phase| phase.iter().copied()).collect();
			let index = passes.iter().position(|each| std::ptr::fn_addr_eq(*each, pass))?;
			let before = run_passes(program, &passes[..index]);
			clear_shown_hints();
			take_warnings();
			let (after, mut reports) = every_hint_of(|| pass(before.clone()));
			reports.extend(take_warnings());
			Some((before, after, reports))
		})
	})
}

fn run_passes(node: Node, passes: &[Pass]) -> Node {
	passes.iter().fold(node, |node, pass| pass(node))
}

/// `run_passes` with each pass's warp twin in its place when self-hosted (WARP_SELF_HOSTED, self_hosted.rs)
fn run_program_passes(node: Node, (phase, passes): (usize, &&[Pass])) -> Node {
	passes.iter().enumerate().fold(node, |node, (index, pass)| {
		crate::compiler::phase_times::timed_as(
			|| format!("program pass {phase}.{index}"),
			|| crate::compiler::self_hosted::pass(*pass, &node).unwrap_or_else(|| pass(node)),
		)
	})
}

/// Everything before code generation. `Err` is the final value of the program when it needs no module
/// (a constant answer, an error, a denied capability).
pub(crate) fn lower_for_emission(node: Node) -> Result<Node, Node> {
	use crate::lowering::blocks::unsealed;
	crate::compiler::phase_times::timed("lowering, all", || {
		crate::lowering::nodes::numbering_fresh_names(|| lower_with_sealed_quotes(node))
	})
	.map(unsealed)
	.map_err(unsealed)
}

fn lower_with_sealed_quotes(node: Node) -> Result<Node, Node> {
	use crate::analyzer::effects::{without_constraints, EffectReport};
	// the loop headers the parser keeps as written (`for int in xs`), given their meaning before any check reads the loops
	let node = crate::lowering::for_loop::headers::lower(node);
	// `shape(a + b)` of `shape(code: data)`: the argument quoted, `run { n + 1 }` of `run(f: function)` its lambda
	// (typed_block_arguments.rs), then sealed with every `data e`
	let node = crate::lowering::typed_block_arguments::lower(node);
	// `data e` as written: no sugar pass rewrites inside e
	let node = crate::lowering::blocks::seal_quoted(node);
	// the list items as written, before passes build lists of their own
	let node = crate::lowering::data_items::lower(node);

	if let Some(clash) = crate::analyzer::check_operator_word_functions(&node) {
		return Err(clash.into_error());
	}
	if let Some(misread) = crate::analyzer::check_upcast_fields(&node) {
		return Err(misread.into_error());
	}
	if let Some(unfillable) = crate::analyzer::self_typed_fields::check_self_typed_fields(&node) {
		return Err(unfillable.into_error());
	}
	crate::errors::diagnostic::report(&crate::web::accessibility::warnings(&node))?;
	crate::errors::diagnostic::report(&crate::analyzer::source_warnings(&node))?;
	// emits and handler blocks as written: the passes lower them to calls, the named effects are read from them
	let as_written = node.clone();
	let node = crate::compiler::phase_times::timed("program source passes", || {
		SOURCE_PHASES.iter().enumerate().fold(node, run_program_passes)
	});
	let node = without_located_errors(node)?;
	if let Some(kind_change) = crate::analyzer::check_kind_changes(&node) {
		return Err(kind_change.into_error());
	}
	if let Some(unsaid) = crate::numbers::uncertain::check_orderings(&node) {
		return Err(unsaid.into_error());
	}
	let node = crate::compiler::function_equality::decide_comparisons(node);
	if let Node::Error(_) = node {
		return Err(node);
	}
	let node = crate::time::lower(node);
	// quantities in loops and branches: SI amounts at run time, the units checked here (static units, stage 1)
	let node = match crate::numbers::units::static_units::lower(&node) {
		Some(lowered) => lowered?,
		None => node,
	};
	if let Some(answer) = crate::numbers::real::answer(&node) {
		return Err(crate::numbers::units::static_units::with_answer_units(answer));
	}
	let node = without_located_errors(crate::compiler::phase_times::timed("meaning passes", || {
		run_passes(node, MEANING_PASSES)
	}))?;
	let effects = EffectReport::of(&node).with_events_of(&as_written);
	if let Some(answer) = effects.answer(&node) {
		return Err(answer);
	}
	let node = without_located_errors(effects.answer_queries(node))?;
	if let Some((name, capability)) = effects.denied(compile_context::read(|context| context.granted)) {
		return Err(crate::node::error(&if crate::host::sandbox::is_sandboxed() {
			crate::host::sandbox::denial(&name, capability.name())
		} else {
			format!("capability denied: {name} needs the {} capability, which eval does not grant", capability.name())
		}));
	}
	let node = crate::analyzer::resolve_main_variable_assignments(crate::lowering::late_binding::lower(
		without_constraints(node),
	)?)?;
	if let Some(error) = crate::compiler::phase_times::timed("diagnose", || crate::analyzer::diagnose(&node)) {
		return Err(error);
	}
	// the standard modules' code warns no program using it (nearest's exact float stop)
	let mut warnings = crate::compiler::phase_times::timed("lint", || crate::analyzer::lint(&node));
	warnings.retain(|warning| !warning.file.as_deref().is_some_and(crate::modules::is_std_source));
	crate::errors::diagnostic::report(&warnings)?;
	Ok(crate::lowering::declaration_forms::indexed_parameter_copies(crate::lowering::inlining::lower(
		crate::lowering::for_loop::lifted_walked_literals(crate::lowering::declaration_forms::lower_declarations(
			crate::lowering::kebab_names::resolve_data_scope(node),
		)),
	)))
}

/// The node, or the first error a pass left in it at a source position
fn without_located_errors(node: Node) -> Result<Node, Node> {
	match crate::errors::diagnostic::first_located_error(&node) {
		Some(error) => Err(error),
		None => Ok(node),
	}
}

/// The program as the emitter sees it after every lowering pass (`warp lower <code>`); `Err` is its final value
/// when it needs no module.
pub fn lower(code: &str) -> Result<Node, Node> {
	crate::errors::diagnostic::begin_program();
	crate::errors::diagnostic::in_program_mode(lawful_program(code)?, lower_for_emission)
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

/// `compile` again when code generation called prelude words the program never named, so resolve did not load them
/// (`n times [x]` calls filled_list): the second time resolve loads them as if called (card lib-from-runtime). The
/// first compile already hinted the program
fn loading_compiler_words<T>(compile: impl Fn() -> Result<T, Node>) -> Result<T, Node> {
	use crate::compiler::compile_context::{change, scoped};
	let compiled = compile();
	let missing = change(|context| std::mem::take(&mut context.missing_compiler_words));
	if compiled.is_ok() || missing.is_empty() {
		return compiled;
	}
	// an implied_words scan that missed: the program compiles twice (RUST_LOG=debug names the words to scan for)
	log::debug!("compiling again for the prelude words {missing:?}");
	crate::normalize::without_hints(|| scoped(|context| &mut context.compiler_words, missing, compile))
}

fn compile_program(code: &str, rewrite: fn(Node) -> Node) -> Result<CompiledModule, Node> {
	crate::errors::diagnostic::begin_program();
	crate::errors::diagnostic::in_program_mode(rewrite(lawful_program(code)?), |program| {
		loading_compiler_words(|| compile_lowered(program.clone(), code))
	})
}

fn compile_lowered(program: Node, code: &str) -> Result<CompiledModule, Node> {
	let source = program.clone();
	let node = crate::lowering::folding::precompute(crate::lowering::folding::refuse_result_misfits(
		lower_for_emission(program)?,
	)?);
	let reflected = crate::lowering::reflection::meta_entries(&source, &node, code);
	warn_about_run_time_blocks(&node)?;
	// the module's `warp.meta` section: a final quantity's unit, the program's functions and classes
	let entries: Vec<_> =
		crate::numbers::units::static_units::result_units_entry().into_iter().chain(reflected).collect();
	choose_module(&node).map(|module| CompiledModule {
		bytes: crate::formats::meta_section::with_entries(module.bytes, entries),
		..module
	})
}

/// The program with its last statement `x` printed: `print(x)`; a declaration or a print stays as it is
fn printing_result(program: Node) -> Node {
	match program {
		meta @ Node::Meta { .. } => meta.under_meta(printing_result),
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
	if !crate::analyzer::effects::EffectReport::of(node).calls_external(crate::host::RUN_BLOCK) {
		return Ok(());
	}
	crate::errors::diagnostic::report(&[crate::errors::diagnostic::Diagnostic::at(node, format!(
		"this module runs blocks known only at run time: its host must provide host.{} (the warp CLI, or web/playground/host.js, which loads warp.wasm on first use)",
		crate::host::RUN_BLOCK)).weak()])
}

/// `TypeName:{field:value, ...}` compiles to a raw struct module, everything else to the standard Node encoding; an
/// instance with unit fields (`Run(5 km)`) too, whose units only the standard module carries
fn raw_struct_module(node: &Node) -> Option<Vec<u8>> {
	if crate::numbers::units::static_units::has_result_units() {
		return None;
	}
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
pub fn eval_parsed(node: Node, code: &str) -> Node {
	crate::errors::diagnostic::in_program_mode(node, |node| eval_program(node, code))
}

fn eval_program(node: Node, code: &str) -> Node {
	#[cfg(feature = "native")]
	if let Some(module) = crate::run::program_cache::kept(code) {
		return checked_markup(run_module(module));
	}
	match module_to_run(node, code) {
		Ok(module) => checked_markup(run_module(module)),
		Err(value) => value,
	}
}

/// The module the program compiles to (kept for its next run, src/run/program_cache.rs), or its value when it needs
/// none: a constant answer, a raw struct's, a type error
fn module_to_run(node: Node, code: &str) -> Result<CompiledModule, Node> {
	let compile = || {
		loading_compiler_words(|| {
			let node = lower_for_emission(node.clone())?;
			#[cfg(feature = "native")]
			if let Some(wasm_bytes) = raw_struct_module(&node) {
				match run_raw_struct(&wasm_bytes) {
					Ok(result) => return Err(result),
					Err(e) => warn!("raw struct eval failed: {}", e),
				}
			}
			// Fallback to standard Node encoding; a final quantity's unit travels in the module (`warp.meta`) and is read back
			emit_module(&node).map(|module| CompiledModule {
				bytes: crate::numbers::units::static_units::with_result_units(module.bytes),
				..module
			})
		})
	};
	#[cfg(feature = "native")]
	return crate::run::program_cache::keeping(code, compile);
	#[cfg(not(feature = "native"))]
	{
		let _ = code; // the browser keeps no module between runs
		compile()
	}
}

/// A run's value; markup it built is checked for accessibility as it came out (card web-a11y-runtime)
fn checked_markup(result: Node) -> Node {
	if crate::web::markup::is_markup(&result) {
		crate::web::accessibility::check_rendered(&result);
	}
	result
}

/// Run a compiled program with the linker its imports need
#[cfg(feature = "native")]
pub fn run_module(CompiledModule { bytes, needs_host, needs_wasi, needs_ffi }: CompiledModule) -> Node {
	use crate::run::wasm_reader::{read_bytes_with_imports, Imports};
	let result = crate::compiler::phase_times::timed("run (with wasmtime compile)", || {
		if needs_ffi || needs_wasi || needs_host {
			read_bytes_with_imports(&bytes, Imports { host: needs_host, wasi: needs_wasi, ffi: needs_ffi })
		} else {
			read_bytes(&bytes)
		}
	});
	result.unwrap_or_else(failed_run)
}

/// Without wasmtime the embedding host runs the program (the browser playground: web.rs); a final quantity's unit is
/// read back from the module's `warp.meta` section (entry units), as wasm_reader does natively
#[cfg(not(feature = "native"))]
pub fn run_module(module: CompiledModule) -> Node {
	crate::numbers::units::static_units::with_module_units(&module.bytes, crate::web::run_in_host(&module.bytes))
}
