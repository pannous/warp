# Cleanup (user card `cleanup`: major code cleanup and simplification)
Roadmap card: cleanup (board topic; its sub-cards are the areas below and their leftovers, notes/roadmaps.md).
Repo-wide leftovers: oversized files (cleanup-big-files), never-called functions over every area (cleanup-dead-scan),
fmt/clippy drift (cargo-fmt, clippy-reports), probe-script proliferation (cleanup-probe-scripts), stale notes
(cleanup-stale-notes).

Dead scan (cleanup-dead-scan, 2026-10-13, probes/cleanup/dead_scan.py): 15 never-used pub functions removed, unused in the default,
all-features and wasm32 builds alike. Removed: TypeManager len/is_empty/next_type_idx/user_type_indices_mut,
StringTable next_offset/table, ImportManager is_empty, Scope child, Real is_exact, Signature merge, Node get_op/from_json,
text_node/error_node/key_op. Kept: function.rs builders, set_hint_file and create_host_linker (in-file unit tests or
tests/ call them), Node last_item and extensions.rs public_function (the user's comments), get_meta and
run_wasm_gc_object (commented-out tests name them), the gc_traits reading API (GcReadable, GcObject get_struct…: the
documented rasm pattern, CLAUDE.md).

Big files (cleanup-big-files, 2026-10-13, probes/cleanup/split_module.py): every source file over 1500 lines split
into child modules along its own seams (section banners, cohesive groups of methods), one commit per file. No file
is over 1500 lines now. The tool moves line ranges into `x/child.rs`:
- It keeps paths and visibility: `use super::*` in the child, and a re-export in the parent as visible as the child's
  most visible item.
- Private items become pub(super) and struct fields pub(super); `super::` gains a level under x.rs. `use` lines stay
  in the parent.
- What it cannot know: an unused re-export (when only the child calls its items) and a `pub` item that exists only
  under a cfg (`#[cfg(feature = "native")] pub use raw_structs::run_raw_struct;`). Fix those by hand after
  `cargo check`.
Ranges are 1-based and inclusive. Start each at the doc comment of its first item and end it at the closing brace
of its last item: a range that stops one line short breaks the parse.

Probe scripts and stale notes (cleanup-probe-scripts, cleanup-stale-notes; recurring, the passes so far):
- 2026-10-13: the one-off agent tools of the shared-checkout era went (hunk selectors, unignore scripts, a bisect
  step) with the finished per-topic fixer prompts in probes/algo. Every other probe script is still named by a test,
  a note or a sibling probe.
- 2026-10-13: removed notes/float_truncation_survey.md (A5 done: notes/number_literals.md history keeps the rule) and
  notes/supervising_fixers.md (the 2026-10-02 fixer round; its rules live in notes/roles.md and notes/agents/).

Behaviour-preserving only: shared helpers instead of duplication, dead code and stale comments out, flatter control
flow, meaningful names, no test edits. One area per session, small themed branches through the Integrator.

Watch for closed lists (user, 2026-10-10: "We don't want specialists… we just want the general mechanism"): a
hard-coded list of words standing in for a mechanism breaks on every word it misses. Example: builtin_type_kind
(src/analyzer/checks.rs) knew int, float, number and text but not real, so `type(π) == real` failed with "undefined
variable: real" although type(π) prints real. Derive each such list from its source of truth (the type registry,
the library's exports, the operator table) instead of adding the missing word. Card cleanup-closed-lists; the
type-word lists become one table under warp-numbers (bare type names are type values).
List words (card derive-hard): the operand words (`1 == first xs` applies first) are every library word of one
argument by `arity` (runtime words, expanded words, lib/prelude.warp) that is no type word (type_tests registry), not
seven listed ones; the copy, draining and result-kind lists name one spelling each and match through
library_words::canonical_spelling. Still listed by hand: their members (properties no other table records),
broadcasting SCALAR_LIBRARY_WORDS, checks ORDER_WORDS.

## Before self-hosting: the plan (card cleanup-plan, 2026-10-13)
The user wants a proper Rust cleanup before self-hosting resumes (notes/decisions.md "self-hosting-postponed"). The
measure is how hard the code is to follow, and how hard it is to write its warp twin. Each step below is one card under
cleanup, a mergeable unit with no behaviour change. Emitter-touching steps are checked with
probes/cleanup/same_wasm.sh. Ranked by value per effort:

| # | card | what | why it matters | effort |
|---|------|------|----------------|--------|
| 1 | cleanup-scoped-state | one `scoped(&KEY, value, run)` helper for the 12 replace → run → restore triples (pipeline.rs GRANTED, COMPONENT_FUNCTIONS, SERVER_VALUES, SERVING_PORT, with_flag; diagnostic.rs ×3, util, units, modules, meta, host/native) | one idiom instead of twelve copies | S |
| 2 | cleanup-compile-mode | the 9 bool statics in pipeline.rs (FOR_A_PAGE, FOR_DEV, RENDERS_ITSELF, FOR_WAGI, PRINTS_RESULT, PRERENDERING, FOR_ANY_HOST, FOR_HOSTING, FOR_TESTS) and their for_x/is_x pairs as one `CompileMode` struct in one thread-local. The emitter reads it only through wasm_emitter/config.rs, not via `pipeline::is_for_*` in function_builder, mod.rs, module_finish and component_adapters | a twin passes one mode value; the emitter stops reaching into the pipeline | S–M |
| 3 | cleanup-parser-words | the parser imports from lowering and analyzer: SWITCH_WORDS, EMIT_WORDS/ALIASES, ON_WORD, IMPLICIT_PARAMETER, FILE_CLASS_MARK, VARIANTS_MARK, leading_name, operator_method, `call`, children_rewritten. Move these words and node helpers into leaf modules (lowering/words.rs → src/words.rs, node helpers to node/), so the parser depends only on node, words and the type registry | the parser is the first thing self-hosting rewrites; its dependencies are what the twin has to carry along | S |
| 4 | cleanup-parser-for-loops | the for-loop desugarings out of the parser (statements.rs + mod.rs unit_iteration, type_filter_header): type-filtered `for int in xs` (and the FilterLoop meta the list emitter downcasts), `for chars in text` (byte_at/map/chars), walked_text_units, `for … where`. The parser emits plain `for v in iter body`; lowering/control/for_loop.rs does the rest | the biggest semantic rewrite left in the parser (warp-dispatch moved `times` already) | M |
| 5 | cleanup-parser-element-wise | `xs .* 2` is built into a map lambda in the parser (mod.rs SpecialInfix::ElementWise → analyzer::element_wise); keep the operator node and lower it in broadcasting.rs | same as 4, small | S |
| 6 | cleanup-analyzer-lowering | analyzer/declaration_lowering.rs (1393 lines) and the analyzer's lower_* passes (lower_inline_unions, lower_list_times, lower_negated_calls, lower_kebab_members) move to lowering/; the analyzer keeps analysis | "analyzer" today means both; one place for passes | S–M |
| 7 | cleanup-pass-phases | SOURCE_PASSES (118) and MEANING_PASSES (31) as named phase slices (sugar, markup/web, declarations, signals, functions, dispatch) joined in the same order, each pass named one way: today pipeline.rs mixes `crate::serve::lower` (root re-exports from lib.rs) with `crate::lowering::index_positions::lower` | the order becomes readable; the twin's pipeline can copy the phases one at a time | S |
| 8 | cleanup-src-folders | the 73 top-level src/*.rs grouped by area: web/ (web, web_server, web_sockets, webview, site, dev_server, deploy, route_split, page_tests, headless, fetches), graphics/ (paint, paint_window, gpu, gpu_canvas, text_raster, shader_holes), sound/ (sound, sound_engine, midi). Pure moves; the paths are updated with fastmod | src/ reads as a map of the project | S |
| 9 | cleanup-kernel-names | linear_arrays writes a kernel's body into its function name and parses it back in the emitter (kernel_of; analyzer/imports.rs matches the prefix). Replace that with a side table keyed by name; also traps.rs's parse of a detail text | the emitter stops calling the parser | M |

Done, all nine (warp-cleanup, 2026-10-13). What came out of them:
- 4: the parser keeps `for int in xs`, `for (it>2) in xs`, `for chars in text` as written. lowering/control/for_loop/headers.rs
  is the first pass of lower_for_emission and lower_module_source, and it re-derives what the parser used to know:
  declared types (analyzer::declared_classes), functions (function_definitions), and text variables (post-order). One
  visible change: a filter loop's got-it warning points at the loop header instead of the loop's end.
- 7: SOURCE_PHASES = SUGAR, DECLARATION, MODULE (it starts with modules::resolve), SIGNAL and FUNCTION passes. A used
  module's source runs MODULE_SOURCE_PHASES. Passes are named by their real path (`crate::lowering::serve::lower`); the
  root re-exports remain for other callers.
- 8: web/mod.rs is the browser compiler, the old web.rs; sound/mod.rs is the old sound.rs; graphics/ is new. The
  include_str! paths in the moved files are one level deeper.
- 9: linear_arrays::kernel_name records the kernel's parameter and body in the thread-local FLOAT_KERNELS (lowering and
  emission share the compiling thread). A missing kernel panics instead of being skipped. traps.rs unquotes a detail
  by undoing serialize's escapes.

Not worth it (checked):
- **Merging lowering passes.** About 150 walks, but each is a small, separately testable rewrite. Merging would couple their order, and warp-markup's experience is that such merges cost more than they save. The few passes that run twice (phrase_calls, reflection, getters) say why in pipeline.rs.
- **children_rewritten vs Node::map_children.** The difference is documented: map_children also enters class bodies. cleanup-rewrite-tails (warp-markup) is folding the remaining hand-written tails.
- **The emitter's repeated instruction runs.** 77 overlapping 8-line windows, mostly in list_runtime.rs, linear_arrays.rs, node_arithmetic.rs and big_int; same verdict as cleanup-emitter-loops.
- Closed earlier: cleanup-const-evaluators, cleanup-gc-object, cleanup-emitter-loops. cleanup-ffi-headers done later the same day.

## Global state (card cleanup-global-state, 2026-10-13)
src has 66 thread-locals (`thread_local!`). Nearly all are private to their module behind accessor functions. They are
kept as thread-locals: the passes stay `fn(Node) -> Node` (supervisor), and grouping them gives the twin one context
object instead. How each one lives: **scoped** = `util::scoped` sets it for one run and restores it; **set** = written
per compile and never reset; **sink** = filled during a compile and taken at its end; **pass** = set at the start of
one pass and read only within it; **counter** = a fresh-name counter that is never reset.

A. Compile context, to become one `CompileContext` beside `CompileMode` (card cleanup-compile-context)

| static | module | lives | holds |
|---|---|---|---|
| COMPILE_MODE | pipeline | scoped | the nine compile flags (cleanup-compile-mode) |
| GRANTED | pipeline | scoped | capabilities eval grants |
| COMPONENT_FUNCTIONS | pipeline | scoped | a component world's imports and exports |
| SERVING_PORT, SERVER_VALUES | pipeline | scoped | the port and values of a served page |
| SOURCE | diagnostic | set | the source text, for excerpts; also read by generators, generator_consumers, fetch_signals |
| WARNING_MODE | diagnostic | set / scoped | how warnings are reported |
| COMMENTS_AS_META | diagnostic | scoped | strict pragma: comments kept as meta |
| HINT_MODE, STYLE | normalize | set | hint mode, style preferences (STYLE is read by markup, accessibility, style_rules, transitions, glued_attributes) |
| HINTS_MUTED | normalize | scoped | hints off for an inner compile |
| HINT_POSITION | normalize | set | the node a hint points at |
| PARSED_FILE | meta | scoped | the file being parsed, for positions |
| PROGRAM_FILE, MODULE_FILES | modules | set | the program file and the module files it used |
| SANDBOXED | sandbox | scoped | sandboxed eval (beside the process-wide PROCESS_SANDBOXED) |
| SHADOWED_UNITS | units | scoped | unit names the program defines |
| RESULT_UNITS | static_units | sink | the result's units, taken by the runner |
| FLOAT_KERNELS | linear_arrays | set | kernel bodies from lowering to the emitter (cleanup-kernel-names) |
| DECLARED_TYPES | named_arguments | pass | declared types, read by the same pass: a pass parameter |
| BOOL_FUNCTIONS | analyzer/booleans | set | functions known to return a bool |
| WRITTEN_PROBLEMS | accessibility | pass | accessibility problems the source states itself |

Done (2026-10-14): all of A are fields of src/compiler/compile_context.rs `CompileContext`, one thread-local; `compile_context::scoped(field, value, run)` replaces the per-static `util::scoped` calls and pipeline's with_flag / set_mode_flag, `read` and `change` the `.with(|cell| …)` accessors, and single-caller accessors (hint_line, module_files, is_module_definition, comments_as_meta, serving_port, server_value, sandbox::confined) are gone. Not done: a reset of the `set` fields per compile, since an inner compile (a module, the renderer) would wipe the outer one's kernels and module definitions; that needs a scoped context per compile, not a reset.

B. Compile output, to become one `Diagnostics` sink taken once per compile (card cleanup-diagnostic-sink)

| static | module | lives |
|---|---|---|
| COMPILE_WARNINGS, ERROR_DIAGNOSTICS, ASSUMPTIONS | diagnostic | sink (mem::take) |
| SAID, SAID_HINTS, HINT_CAPTURES, NOTES_SHOWN | diagnostic | sink / set |
| SHOWN_HINTS | normalize | sink (cleared) |
| MODULE_DEFINITIONS, EARLY_CLASS_MODULES | modules | sink (cleared), also read by modules/loader |

Done: diagnostic.rs's `Diagnostics` (one DIAGNOSTICS thread-local) holds said, warnings, errors, assumptions, the
hint captures and SHOWN_HINTS (said_before moved to diagnostic.rs). Kept apart: NOTES_SHOWN, which with_acknowledger
scopes together with the acknowledger statics of F; MODULE_DEFINITIONS and EARLY_CLASS_MODULES, loader state that
lives beside PROGRAM_FILE and MODULE_FILES of A (a candidate for the compile context, not for the output).

C. Fresh-name counters, never reset, so a name depends on what compiled before (card cleanup-fresh-names): RACES
(declarations/tasks), ITEM_LOOPS (switch). Done: one numbering per outermost compile (nodes::fresh_number); its
counter and compile depth are CompileContext fields since 2026-10-14, as is gpu_maps' PROGRAM_FUNCTIONS (scoped).

D. Caches, kept: REMEMBERED_ANALYSES (analysis_memo), BLOCK_MODULES and COMPILED_BLOCKS (pipeline), LOADED
(components), FETCHED (web).

E. Run-time host state, kept (one per running instance): LISTENERS (channels), CONNECTIONS and ROWS_READ (database),
BEFORE_AT and VOICE (sound), FETCHES and POSTED (web/fetches),
CONNECTIONS and LISTENERS (web_sockets), ON_LISTENING and REQUEST_LIMIT (web_server), HOOK, MODULE, RENDERED and
REPORT (web), BLOCK_VALUES and NATIVE_PAGE_PATH (host/native), PROGRAM_ARGUMENTS and UNFILED_STORES (std_adapters),
IN_TASK_WORKER (tasks), CURRENT_RULES (calendar), FAMILY_LINKERS (wasm_reader), FUEL_OVERRIDE (util),
RUNTIME_WARNINGS (diagnostic).

F. User acknowledgements, kept (the user's got-it answers): ACKNOWLEDGED_TOPICS, ACKNOWLEDGEMENTS_FILE, ACKNOWLEDGER
(diagnostic).

G. Re-entrancy guard, kept: READING_PRELUDE (std_modules) stops the prelude's own parse from waiting for the
OnceLock it fills; it belongs to that process-wide table, not to a compile.

Status 2026-10-14: 39 thread-locals left, down from 69: of compile state only CONTEXT and DIAGNOSTICS, the rest is D–G
and NOTES_SHOWN. Process-wide `static`s, checked: caches of outside facts (ffi STRUCT_TYPES and LOADED_LIBRARIES,
gc_traits REGISTRY of name sections, module_cache IN_MEMORY, components COMPILED, SQLITE), locks serializing
outside tools (NATIVE_CALL_TURN, MODEL_LOCK, FETCHING, BUILDING, PROVIDING), the one terminal's output settings
(SHOWN_FROM, ANY_HINT_PRINTED, SHOWN_SOURCE: a program may compile on another thread), the process's devices (graphics,
sound, VIEWER) and PROCESS_SANDBOXED, FILED. None is compile state.
A scoped context per compile: closed as not worth it (2026-10-14). Of CompileContext's `set` fields, the ones a compile
fills are already scoped or reset at their pass (module_definitions, early_class_modules, module_files, declared_types,
bool_functions, written_problems, result_units, shadowed_units, server_values). Only float_kernels accumulates, keyed by
the kernel's own text, so a later compile finds the same entry. A fresh context per compile would have to copy mode,
grants, source and positions into inner compiles (modules, the renderer): plumbing lines, no leak fixed. The 26
`thread_local!`s left are host and run-time state, caches and the diagnostics sink.

## src/analyzer/, src/warp_parser/, src/node/ (card cleanup-analyzer, session warp-types)

Done (branches cleanup-dead, cleanup-arith, cleanup-names, cleanup-parser, cleanup-modes, cleanup-guards, cleanup-lookahead, cleanup-atoms, cleanup-errors,
cleanup-groups, cleanup-filter):
- dead code: Scope types, Separator::from_char, Node::key_with_op, get_meta_data, type_definition, commented-out code
- `Node::symbol_name` / `Node::is_symbol` replace analyzer's is_word and ~40 hand-written Symbol matches
- node/arithmetic.rs: `keeping_left_meta` + `scalar_operands!` macro (317 → 106 lines)
- checks.rs check_type_errors_inner: flat match arms
- parser: `Mark`/`mark()`/`rewind()` replace 26 position-tuple snapshots; `take_chars`, `code_words`,
  `at_group_end`, `for_iterable_and_body_word`, one `skip_blanks`
- json_xml.rs: `xml_attribute`, `insert_json_entry`, `json_object`
- imports.rs: `functions_of_operator` / `functions_of_call` tables instead of if chains
- inference.rs infer_list_type: one `head` symbol, unused closure arity gone
- expressions.rs continue_expr: `parse_right_operand`, `combined`
- xml.rs parse_xml_tag: `parse_attribute_value`, `parse_closing_tag`, `xml_element`
- atoms.rs parse_symbol_with_suffix: other languages' forms in `foreign_form`; `in_code()`
- variables.rs collect_variables_inner: one `visit` closure, `bind_destructured`, `bind_assigned`
- lists.rs parse_list_with_separators: `with_definition_block`, `parse_separator`
- `Node::single_or_list` for the one-item-or-list choice
- (cleanup-json-nodes) `foreign::node_of` is the single JSON-to-Node conversion; headless page-event details reuse it,
  so nested arrays, codepoints, big integers, ordered objects and number handling no longer diverge.
- if-let guards (stable on our toolchain, already used in src/lowering/) replace ~30 arms that computed their match
  twice: `x if f(x).is_some() => f(x).expect("guarded")` and `matches!(…)` + `let … else { unreachable!() }`
- inference.rs infer_type: `conversion_kind`; checks.rs list_type_name: `nested_type_word`, `LIST_OF_PREFIX`;
  literals.rs parse_string: `quoted_text`
- scanning.rs `symbol_after`, `take_while_char`, `advance_by` (from atoms.rs) for parse_atom and neighbours;
  parse_type_declaration_body: `parse_comma_names`, one type-parameter parse
- `or_return_error!` (warp_parser/mod.rs) for `match r { Ok(x) => x, Err(message) => return error(&message) }`
- `lowering::nodes::call(word, args)` for the parser's ~16 hand-built `Node::List([Symbol(word), …], Round, None)`;
  `for_in_loop` for the four desugared for loops; group_by_separators: loosest separator via max()
- `type_filtered_body` for the two `for T in xs` filters; parse_number_value: `float_node`, hex via
  `take_while_char`; infer_list_type: one head-word block (edition 2021: no let-chains, only if-let guards)

Left (longest functions, candidates for splitting):
- DONE (cleanup-long-functions) atoms.rs parse_symbol_with_suffix (~160 lines → ~80): the function-name suffix
  (read_function_name_suffix) and the keyword forms (keyword_form) are their own functions, in the same order
- atoms.rs parse_atom (~140) and parse_type_declaration_body (~120): long but flat now
- declaration_lowering.rs lower_declarations_among (~110): a long but now flat rewrite dispatch
- two `matches!` + `unreachable!("guarded")` arms stay where the arm moves the scrutinee the guard borrows
  (parameter_copies, `x as number = 9`)
- lookahead.rs peek_operator (123): a flat operator table, left as is (keywords interleave with the glyph lengths)
- node/comparison.rs eq: left alone, it carries the user's comments

Outside this area: the private `is_word` / `symbol_name` / `is_symbol` copies are gone (lowering's in
cleanup-lowering; law/type_model.rs, modules.rs and time.rs in cleanup-word-copies, warp-functions); word_slices'
`is_word(node, words)` takes a list of words and stays. The parameter_name copies (soft_keywords, named_arguments,
go_blocks, parameter_shapes, law/type_model) stay: each has its own rules (which keys, recursion, String or Node).
## cleanup-host (warp-hosting): top-level src/*.rs, src/ffi/, src/gc_traits/
How dead code was found: every `fn` name in the area counted across src, tests, crates and web
(`grep -rwo`); a name seen only at its definition has no caller. `#[test]` functions inside src files and trait
methods (wasm-encoder `Reencode`'s `parse_*_section`, winit's `resumed` / `window_event`) are no dead code.
Exact copies: `npx jscpd --min-lines 4 --min-tokens 40 --format rust` over a copy of the area (it skips ignored
folders, so run it inside the copy); exact duplication is only ~0.1% here, the rest is duplication in meaning.
Done:
- never-called functions removed (function.rs builder leftovers, wasm_reader's unused readers and FromVal trait,
  TypeDef::from_node with its second extract_fields, ffi_parser get_signature / get_library_signatures, …)
- ffi: the libc overrides are one table (LIBC_SIGNATURES)
- `Op::holds(ordering)` replaces the comparison tables of real.rs and time.rs
- GcString's two array readers share array_text; wasm_name_resolver drops the type indices and `shared` flag it
  never read
- main.rs: one `fail(message)` for the CLI's 16 "say it and exit 1" pairs; host.rs: the never-imported
  host.get_result_ptr / get_result_len linker entries removed
- `Node::key(name, value)` replaces hand-built `Node::Key(Box::new(Node::Symbol(…)), Op::Colon, …)` (foreign,
  headless, host, meta_section, shader_holes, web_server)
- site.rs: read_program / program_stem shared by deploy, dev_server and host's served_site; page_html_text for the
  "gave no text" check; items_of reused
- ffi/link.rs: the hand-linked libc functions read memory through memory_bytes / parse_c_text / text_pair
- gc_traits: GcObject getters through with_field; numeric GcReadable impls by one macro
- wasm_reader run_main_with_tasks; `wasm_emitter::bracket_of_info` (bracket_info read back) for wasm_reader and web.rs
- modules.rs registered_package / package_version_tags / make_package_directory; tasks.rs zero_results;
  wasm_modules map_children_or_failure; Local::param via Local::new; type_kinds written_name; wasm_optimizer
  unique_temp_file
- `Node::as_items()`: a list's items, none of ø, else the node as the one item, also in class_methods and
  component_worlds (branch cleanup-items); warp_parser/mod.rs:204 differs (no drop_meta); site.rs items_of keeps ø
  as an item on purpose
- `Node::parts()`: a list's items or a key's two sides, borrowed (units.rs, generators.rs)
- wisp_parser: the typed s-expression nodes through finish_one / finish_key / finish_constant and one call_of
- wisp emitter: one emit_form for its `(word part …)` forms; gc_traits memory_text reads a $String for GcString and
  the debug formatter; gc_struct!/wasm_struct! field arms take an optional rest
- tasks::task_failure for the TaskFailure constructions, host module_memory for "the module exports no memory"
- compile_time.rs: the Stop, fail and answer_of of real.rs and units.rs (units/static_units.rs keeps its own Stop: it
  has a third case, Recursive)
- ffi/link.rs create_ffi_wrapper: the 18 typed arms are one link_typed! each (NativeArgument / NativeResult convert
  the values, CBool a C bool)
- host.rs keeps the words and the parts both builds use; its native-only half (wasmtime host functions, memory
  helpers, the linker) is src/host/native.rs, gated once at `mod native` and re-exported (`pub use native::*`)

Left (bigger, needs care):
- DONE (cleanup-ffi-headers) One C header parser: ffi/header.rs (`extract_function_signature`, multi-line
  declarations, pointer classes). ffi_parser.rs keeps its public API (parse_declaration, parse_header_file, header
  search) as an adapter over it; its own parser and cached table are gone. get_ffi_signatures holds only what warp links
  by hand (LIBC_SIGNATURES, libm f64, ln, host and linear words); every other C function is as its header declares it.
  Defaults (supervisor-approved): pointers are handles (i32), not raw i64; `char *` is a text; `char **` parameters
  are left out (`strtol("ff", 16)`); a long double function is refused by name ("sqrtl is a C function using long
  double, which wasm has no type for"). No existing tests/ffi test changed value. Pins: tests/ffi/test_ffi_one_parser.rs.
- (cleanup-const-evaluators) real.rs and time.rs are two small constant evaluators with their own Value types and the same Assign/Define arm;
  sharing a scope/evaluator skeleton would remove more, but they differ in errors (Stop vs String).
  Closed as not worth doing (2026-10-13): what they share is the `name = value` arm (4 lines) and the literals; a
  generic skeleton (a trait over Value, error and the unknown-name case) costs more than that, and each Assign arm
  also does its own thing (real.rs a declared `x: real`, time.rs getters `t := now`).
- src/extensions.rs is hard-linked with ~/dev/script/rust/extensions.rs (src/extensions.rs.hardlink is the
  hardlink-guard's marker): edit it only knowing it changes that file too.
- (cleanup-gc-object) Two GcObject types: wasm_reader::GcObject (node/mod.rs, re-exported by lib.rs) and gc_traits::GcObject (tests'
  ergonomic reader). One of them could wrap the other; the public API of both is pinned by tests/wasm.
  Closed as not worth doing (2026-10-13): they read different things. wasm_reader's is a Node view over a Val that may
  be null (`value()` of the last cell) with kind/tag/text; gc_traits' is a typed reader of a non-null struct (field
  names, FromVal/ToVal, set_field). Wrapping would share only the store and the field read (~10 lines) and give the
  Node view a null case of its own.
- done (cleanup-ffi-f32): ffi/link.rs has one wrapper for every C function of numbers and texts; the 21 typed
  signatures (`link_typed!`, NativeArgument/NativeResult, the signature keys) are gone, since card ffi-mixed passes a
  float in the low half of its register. tests/ffi/test_ffi_signatures.rs calls each signature but raylib's
  window-only ones (InitWindow, DrawCircle, DrawRectangle, DrawText); a C bool result is read from its low byte.
- (cleanup-html-escape) markup.rs escapes `<pre>` text without `>`, site.rs `escaped` with it: one escaper would change the error page.

## src/lowering/ (card cleanup-lowering, session warp-class)
Shared node forms live in src/lowering/nodes.rs: `call(f, args)`, `key(a, op, b)`, `assign`, `statement_list`,
`is_type_word`, `parameter_name`, `is_call_head`, `spaced_statement`, `children_rewritten`. Words are asked with
Node::symbol_name / Node::is_symbol. Bulk rewrites were done by small Python scripts (balanced-bracket parser +
`cargo check --message-format short` for the leftovers); a local fn or variable named `call` / `key` shadows the
import (E0255 / E0618), so those got telling names (suffix_call, applied, called_function, key_function, word_of).
Duplicates are found with probes/cleanup/duplicate_windows.py and by counting `fn` names defined in several files.

Done:
- ~30 local copies of call / is_word / symbol / symbol_name / key / assign / statements / is_type_word → nodes.rs
- spelled-out `Node::List(vec![Node::Symbol(f), …], Round, None)` and `Node::Key(Box::new…)` → call / key (75 files)
- `Node::Symbol(x.to_string())` / `Node::Text(x.to_string())` → symbol(x) / text(x) (51 files)
- parameter_name (3 identical copies), word lookups (phrase_calls, type_name_matching, reflection), is_call_head
  (memoization, type_name_matching), spaced_statement (phrase_calls + type_name_matching shared 4 lines)
- the Key / List / Meta recursion tail spelled out in 27 passes → `children_rewritten(node, rewrite)`
- class lookups: class_methods `with_value_classes`, `declared_classes`; `grouped_parameters` (class_methods +
  declarations) → nodes.rs
- calls whose head was already `symbol(f)` → `call(f, …)`
- `analyzer::function_context(node)` replaces Context::new + extract_user_functions in 11 passes;
  `named_assignment` (component_state + lambdas) and list_element_checks' own assign → nodes.rs
- word constants defined in 3+ passes (FOR / IN / ON / OF / FROM / COUNT / MAP / SUM / RETURN / GLOBAL_WORD) →
  src/words.rs (was src/lowering/words.rs); a constant with its own doc comment stayed in its pass
- identical bodies (found by comparing fn bodies with parameters renamed): `block`, `if_then`, `if_then_else` moved
  from variable_signals to nodes.rs and replace ~12 spelled-out `key(key(ø, If, c), Then, b)`; `is_block`,
  `is_colon_pair`, `is_binding`, `is_function_keyword` (2 copies each) → nodes.rs; two `statements`/`entries` → Node::as_items
- `match x.drop_meta() { Node::Symbol(n) => Some(n.clone()), _ => None }` (16 sites) → `x.symbol_name().map(String::from)`;
  `matches!(x.drop_meta(), Node::Symbol(w) if w == WORD)` (43 sites) → `x.is_symbol(WORD)`
- the `Cell<usize>` counters of 15 passes (get then set, set then get, replace) → `nodes::Counter` with
  `next_number()`; a pass counting from 1 starts it `Counter::starting_at(1)`, so every name stays as it was; then the
  `usize` makers of lazy_ranges, min_max, list_element_checks and variable_signals (`+= 1` then use) the same way
- (card cleanup-phrase-words) the parser's groups of a phrase undone (5 copies: list_phrases, scoped_handlers,
  event_signals, tasks, transitions) → `nodes::phrase_words`; code with placeholders (`parse` then `law::substitute`
  over a HashMap built by hand, 20 sites and 6 local helpers) → `nodes::filled(code, [("X", x)])`. Net −107 lines.
  Not folded: word_slices::flat_words and phrase_calls::spaced_words (they also split `to` / alphabetic operators),
  library_words::instantiate_template (names its temporaries apart), test_blocks::template (parses as a generated file)

Left (each changes behaviour or needs care):
- (cleanup-rewrite-tails) Node::map_children also enters class bodies (Node::Type); children_rewritten does not. 45 tails folded by
  probes/cleanup/fold_rewrite_tails.py: the generic Meta arm and `other => other`, with Key and List each either
  generic (folded) or taken by an unguarded arm of their own. The ~20 left differ: no List or Key arm at all (folding
  would make them recurse), `?` inside (folding.rs), a Meta arm doing more than recursing — each a case-by-case look.
  Done (2026-10-13): 8 more folded by hand (real.rs, interpolation.rs, normalize.rs as_written, user_functions.rs
  rename_nested_calls; and a Meta arm in front of `other.map_children` with the same rewrite, which is map_children's
  own Meta arm: time.rs, comprehensions.rs, generator_consumers.rs, atoms.rs). The ~45 Meta arms left belong to
  walks down one path (the statement under its comments, the leftmost operand), not to rewrites of every child.
- DONE (cleanup-statements-of) the plain copies (an unbracketed `;`/newline sequence, else the one node) are one
  Node::sequence_statements: late_binding/folding, run_time_blocks, serve, generator_consumers, law/type_model,
  modules. The ones below keep their own rules on purpose.
- statements_of (class_methods, event_signals, late_binding), statements (component_worlds, test_blocks),
  is_statement_list (generators, result_word, variable_signals), is_return (closures, generators,
  type_name_matching): same name, different bracket/separator rules each, so one shared version changes which forms
  match. The `for v in …` variable is one nodes::for_in_variable (run_time_blocks, signal_values with its five-word
  form, for_loop); library_words keeps its own (`for each v in` too, any word's name).
- DONE (cleanup-definition-view) One view of a function definition: lowering/nodes.rs `function_head(statement)` →
  `FunctionHead { head, name, written_parameters, form, result, body, written_body, block_bracket }`, forms
  `Operator(:= | =)`, `Keyword(: | -> | := | =)`, `KeywordBlock` (`def f(x) {…}`), `KeywordSpaced` (`def (f x) {…}`);
  `parameters()`, `plain_operator()`, `is_glued_head()`. Read through it: law/type_model/forms.rs, overloads,
  nonlocal_cells, printable, generators, component_state, units/static_units, definition_cases, signal_values; each
  keeps its own filter (which forms, result type allowed, glued head), so no site reads a form it did not read before.
  Follow-up (definition-view-ports): the Lean export's `function_definition` returns the FunctionHead (no
  result_type / tuple wrappers, Exporter::function and nested lift_from take it), units.rs reads parameter clashes
  through it (`plain_operator`). Left on purpose: memoization `definition_parts` (type_tests) also takes `x := 3` and
  rebuilds the same form (compacted: keyword and meta wrappers share `wrapped`);
  declarations.rs `function_definition` builds the Key form; law.rs `function_definition` goes through the analyzer;
  lambdas / nested_functions / reflection `definition` build or print one. Narrowed (nonsense forms only): signal_values
  no longer reads `def {f x}`, `def (f x) y {…}` (four words) or a head whose first item is not a word.
  Candidates (forms rejected by accident, no test pins them): the Lean export skips `def f(x) := body`, untyped
  `def f(x): body` and `def (f x) {…}`; three is_call_head functions mean different things (nodes: round + word,
  traits: round + word without parameters, variable_signals: round only).
- (cleanup-traits-alias, 2026-10-14) Of src/lowering/traits* (2572 lines) only about 700 are trait code; the rest is
  class dispatch (generics.rs multiple dispatch, dispatch.rs `<`/`==`/sort on instances, instance_types.rs static
  shapes, likeness.rs `like`, op·T witnesses that class methods and overloads use too; notes on card
  methods-as-dispatch). The element check of `xs: Comparable list` is now Dispatch::admit's (a trait list is a
  declared parameter type like a class), dropping trait_constraints / unmet_constraint (−33). Foreign signatures and
  the element check stay (interviewer: rust-forms-welcome, list element checks kept). A mixin rewrite of traits saves
  ~40 lines and loses free-function default methods.
- soft_keywords::parameter_name accepts only `:` / `=` keys and does not recurse, unlike nodes::parameter_name;
  go_blocks / named_arguments / parameter_shapes have their own parameter_name returning String or Node.
- (cleanup-big-files) class_methods.rs (~1920 lines): child modules (`use super::*`, items pub(super)) class_methods/operators.rs
  (operators on instances, specialized calls), inheritance.rs (extends, mixins, super), other_languages.rs (impl, Go
  methods, Ruby .new, witness methods); further parts only if a cohesive group shows up (generic classes?).
  destructurings / positional_fields already share with_value_classes.

### The signal family: one design (card cleanup-signal-family, design only, 2026-10-13)
Nine passes implement "run this body when something happens", about 6.0k lines: event_signals (1161: `on stop {…}`,
`raise`/`emit`, page events), variable_signals (971: `on change x`, `on set x`, `once`/`whenever`, `after f`),
signal_values (1321: listeners that outlive a function, cells, shared polling), system_signals (597: timers, files,
frames, keys, channel messages), scoped_handlers (422: `on ask {…} in {…}`), component_state (765), fetch_signals (194),
stored_values (280), element_events (286). Each repeats three things:
1. Reading the listener. Each pass has its own reader of `on <trigger> <body>`, ~430 lines in 32 functions
   (event_signals::handler/named_handler/listed_events, variable_signals::listener_parts/subject_and_body/
   trailing_block/ungrouped, signal_values::named_listener/without_leading_word, system_signals::timer/file_watch/
   message_listener/event_listener/words_and_body/one_shot, scoped_handlers::scoped_handler/operand_phrase/…). Most of
   their length undoes the groups the parser made of the phrase (`(on stop) (the (machine {…}))`).
   Design: one reader `listeners::read(statement) -> Option<Listener { word: On|Once|Whenever|Before|After, name:
   Option<String> /* h = on … */, trigger: Vec<Node> /* flat words */, body: Node, scope: Option<Node> /* in {…} */ }>`
   in a new src/lowering/web/listeners.rs, and each pass asks it and matches on the trigger words. Estimate −300 lines,
   low risk (behaviour kept; the forms each reader accepts are the union, a few odd spellings may start or stop
   matching → cleanup-edge ignores).
2. Calling the handler. Host-triggered events each have their own registration: `signal_every(id, ms)`,
   `signal_watch(id, file)`, `signal_frame(id, ms)`, `channel_listen(id, name)`, page events through
   `on·click·node`, system events by name. Design: every handler is `on·<event>·<id>` made by
   event_signals::function_with_globals (already shared by 10 passes) and registered by ONE host word
   `signal_listen(kind, id, argument)`; the runtime keeps one table. Estimate −150 in src/, −100 in
   crates/warp-runtime, medium risk (the playground worker.js has to follow).
3. Change listeners twice. variable_signals inserts a check after every write of a watched variable (static);
   signal_values turns escaping variables into cells with listener closures (runtime). Design: drop the static
   path, every watched variable becomes a cell; `once`/`whenever` become cell listeners on the variables their
   condition reads (the derived `:=` case already works this way). Estimate −600 to −800 lines (most of
   variable_signals' write tracking: written_variable, statement_write, checks, inlined, the `while x-->0`
   expression rewrite, P111's signal·check functions). Cost: every read of a watched variable is `cell_get(x)`; a
   watched loop counter gets slower. Decision for the user (Interviewer) before building: lines vs speed of watched
   variables.
Order: 1, then 2, then 3 after the decision. Together about −1.2k lines of the 6.0k; the rest is real behaviour
(effects, polling across tasks, components) that no merge removes.

### Foreign spellings (welcome_forms.rs, asked 2026-10-13, nothing removed until the user answers)
Lines per form incl. helpers; no sample uses any; tests = files using it (they get cleanup-edge ignores if dropped).
generic `<T>` ~106 (5 tests) · OCaml let-in ~65 · Rust `let mut` ~18 · Kotlin/Ruby `when` chains ~66 · C++ lambda
`[](int x){…}` ~44 · C++ `auto x =` ~14 · Elixir `def f(x), do:` ~23 · `Enum.map`/`List.map` ~44 · LINQ ~57 ·
R/PHP `sapply`/`array_map` ~34 · R `c(1,2)` ~57 · R `<- function` ~24 · OCaml `fun x ->` ~26 · Ruby `lambda {}`/`proc`/
`.call` ~43 · `xs.sum { |x| }`/`sumOf` ~27 · Ruby `.to_s`/`.upcase` ~33 · Python `lambda x:` ~30 (5 tests) · variadic
lambdas ~23 · JS `({a, b}) =>` ~38. All ≈ 770 lines. Kept (samples use them): match `=>` arms, `loop {}`, `function(){}`.

Done (branch foreign-spellings, user decision 2026-10-13, src+lib net ≈ −274): dropped OCaml let-in and `fun x ->`,
Elixir `def …, do:`/defp, C++ `[](…){…}` lambdas (tests ignored `cleanup-edge: foreign-spellings`); R and PHP words
moved to lib/R.warp, lib/PHP.warp (`use R`, `use PHP`); LINQ, Ruby `.to_s`/`.upcase`, `sumOf` and module-qualified
`Enum.map`/`List.map`/`Seq.sum` are one-line rows of METHOD_ALIASES / ITERATION_MODULES (a program's own names are
never aliased); C++ `auto x` notes and becomes `var x`; every `when` shape gives one warning naming the word meant.
Variadic lambdas stay on starred_lambda: rest parameters are packed at compile time per named signature (variadic.rs),
so a lambda needs a definition to get one.

## src/wasm_emitter/ (card cleanup-emitter, session warp-fixer)

How a change is checked: behaviour-preserving emitter changes emit the same bytes. Build warp before and after in the
SAME worktree (`cargo build --offline --features native --bin warp` and the copy out of the shared target dir in one
command; `warp --version` names the worktree and commit it was built from, check it: another session's build can
replace the binary between two commands), then run
`probes/cleanup/same_wasm.sh <before> <after> $(find probes samples -name '*.warp')`: `same`, `unstable`,
`DIFFERENT`, `no module`. A binary finds lib/ (`use os`) through its worktree's CARGO_MANIFEST_DIR, so binaries of two
worktrees can differ on module programs for reasons outside the change. macOS kills a binary that `cp` overwrote in
place (exit 137, every program "no module"): `rm -f` the destination before copying. Builds are reproducible since
card emitter-deterministic (generator classes and tables were spliced in HashMap order; tests/wasm/
test_reproducible_builds.rs), so `unstable` should no longer appear.

Done (branches emitter-late-functions, emitter-two-pass, emitter-dead-code, emitter-duplicates, emitter-comments,
emitter-builders, emitter-equality, emitter-runs, emitter-deterministic, emitter-logic, emitter-loops):
- never-called functions and stale comments out; runtime functions through runtime_function / exported_function
- shared helpers for repeated code: emit_while, emit_text_argument_bounds, push_in_place, emit_global_store_declared,
  emit_offset_locals_test, emit_list_cell_function (list_at / list_node_at), emit_nth_cell_data, emit_try_table,
  emit_growable_list_types, string_fields, compile_time_string, emit_local_step (i++ of arithmetic.rs's two paths),
  emit_defined_user_function_call, import_function (host, WASI and FFI imports), emit_numeric_logical,
  emit_cells_from_last (the two as_node(list)), emit_cell_walk (nine list walks: null test, body, step to the rest)
- runs of single `f.instruction(&I::…)` lines as one `Self::emit_list(f, &[…])` per statement group:
  `probes/cleanup/merge_instruction_runs.py <file.rs>…` (rewrites in place; a run splits after statement-ending
  instructions and after `return; end`), done in every emitter file
- instruction arrays in rows, one logical step per row like WAT, nested blocks one tab deeper, kept by
  `#[rustfmt::skip]` on the statement (card emit-rows, user decision 2026-10-14: rustfmt puts each `I::…` on its own
  line): `probes/cleanup/emit_rows.py [--write] files…`, idempotent; `--check` proves only whitespace changed against
  HEAD. Emitter -3757 lines. New emitter code: write the rows by hand or run the script before sending a branch
- `probes/cleanup/duplicate_windows.py` lists repeated windows of normalised lines (the duplicates left to share)

Left:
- (cleanup-emitter-loops) ~70 other hand-written `Block, Loop, <exit test>, BrIf(1), …, Br(0), End, End` loops: counters, byte walks and
  cell walks whose step differs from emit_cell_walk (RefAsNonNull before the rest field, a position bump after it,
  or the loop opened and closed by two closures as in library_ops.rs join and slice). A general
  `loop_until(f, exit, |f| body)` would save ~1 line per loop; each conversion checked with same_wasm.sh.
  Closed as not worth doing (2026-10-13): most of the ~90 loops sit inside one flat instruction slice, often with an
  inner loop or an exit two levels out (`BrIf(2)`, text_grapheme_back); a closure helper cuts each slice into before,
  exit, body and after pieces, which costs as many lines as the Block/Loop/Br/End lines it saves and adds nesting
- DONE (cleanup-saved-state) user_function_calls.rs compile_user_function_body: the ~15 per-function emitter fields are one
  FunctionState, swapped in at the start (swap_function_state) and back at the end; the fields that need the body's
  scope (typed variables, scratch locals, loop values) are set once it is known, as main does

## Coverage survey (warp-cleanup, 2026-10-13, at 5899b83d1)

What the test suite (which runs every sample too) never executes, by `cargo llvm-cov`: one queued run in a detached
worktree, `WARP_TEST_TIME_LIMIT=2400 tests/queue.sh cargo llvm-cov --offline --no-report --tests --no-fail-fast`,
then `cargo llvm-cov report --json` and `--lcov` into data/cleanup/, ranked by
`probes/cleanup/coverage_ranking.py <json> <lcov> <checkout>` (data/cleanup/coverage.txt). The executable-building
tests (wasm::test_executable_*, test_build_emits_wasm, runtime stub) fail under instrumentation; nothing else does.

- 6673 of the executable src lines never run: 534 functions never called (2454 lines), 4219 lines of untaken branches.
  The suite is thorough, so coverage alone frees a few hundred lines, not thousands.
- Most unexecuted code is I/O no test drives on purpose: main.rs CLI commands (418), the paint window (408), deploy,
  sound, package downloads, crash cards. Not dead.
- The real clusters, filed as cleanup cards: unrun emitter branches (key_emitter emit_power_op, values
  emit_float_value, list_emitter emit_floor_quotient, wasi emit_result_print), unused math (real.rs, reals.rs,
  numbers.rs), components.rs value_of/json_of, time.rs, gpu_maps.rs, node/ comparison and indexing, gc_traits'
  name resolver and wasm_reader's string readers (tested by test_gc_name_registry, so not removable without it).
- Bigger deletions have to come from generalizing what does run: near-twin lowering passes and runtime emitters.

## Feature overlap in src/lowering/ (warp-cleanup, 2026-10-13)

What each lowering feature costs and who uses it, so the user can decide which are worth their code (decisions
cleanup-means-fewer-lines, cleanup-may-drop-edge-cases). Usage by `probes/cleanup/feature_usage.py . <wiki>`
(approximate syntax regexes; data/cleanup/feature_usage.txt), the low rows checked by hand. Foreign spellings and the
signal family are warp-list-pattern's and left out. Samples = files in samples/, tour = web/playground/examples.js.

| feature (files) | example | lines | samples | tour | wiki | overlap |
|---|---|---|---|---|---|---|
| compile-time lambdas + function values (lambdas, function_values) | `xs.map(x => x*2)`, `apply(f, x) := f(x)` | 1939 | many | 2 | 43 | run-time closures (closures.rs, 1170) run the same programs; the compile-time path is a speed-up |
| traits (traits, traits/dispatch, traits/instance_types) | `trait shape{area}`, `compare(a:person, b:person) := …` | 1901 | 1 | 0 | 1 | instance `<`/`==` via compare witnesses rides on it; declared traits barely used |
| database tables, ORM (database_tables/) | `people: [Person] = database.people` | 1889 | 4 | 0 | 0 | none; big and recent (orm cards) |
| gpu maps (gpu_maps) | `ys = xs.map(x => x*2) @gpu` | 1306 | 1 | 0 | 0 | shader blocks (WGSL) and data parallelism do GPU/parallel work |
| shared arrays (shared_arrays) | `shared xs = int[n]` | 1180 | 3 | 0 | 8 | channels and atomics share between tasks; KEPT (decision cleanup-cuts): `shared` for any atomic variable is the general form for later |
| web server (serve) | `serve 8080 { get "/api" { users } }` | 1143 | 0–2 | 1 | 6 | none in warp; routes.rs is the page side |
| generators (generators, generator_objects, generator_consumers) | `count_to(n) := { … yield i … }` | 1114 | 0 | 0 | 4 | lazy ranges; a go task writing a channel |
| reflection dot words (reflection) | `f.effects` | 886 | 2 | 0 | 5 | the word forms `effects of f` |
| return-type overloads (overloads) | `parse(t) -> int` beside `parse(t) -> float` | 827 | 0 | 0 | 12 | `as int` conversions |
| DONE: parked natural phrases | `move piece from a to b`, `first 3 of xs` | 787 removed | 2 | 0 | 3 | plain calls `move(piece, a, b)`, `xs#(1…3)` |
| multiple dispatch by class (traits/generics) | `collide(a:Ship, b:Rock)` | 529 | 1 | 0 | 2 | methods on the class; traits' witness dispatch |
| type-name parameters (type_name_matching) | `fib int i = …`, `foo of int = it+it` | 442 | ~9 | 0 | 8 | `fib(i:int) := …` |
| scoped handlers (scoped_handlers) | `on ask { 42 } in { compute() }` | 433 | 1 | 0 | 1 | try/catch, raise and event handlers |
| tuple returns (tuples) | `return a, b`; `x, y = f()` | 387 | 1 | 0 | 4 | returning a list `[a, b]` and list destructuring |
| list element checks (list_element_checks) | `xs: [int] = [1, "a"]` is an error | 359 | 0 | 0 | 0 | the analyzer's type checks |
| instance destructuring (class_methods/destructuring) | `{x, y} = p`, `Point{x, y: b}` patterns | 322 | 0 | 0 | 0 | map destructuring and match patterns |
| nonlocal cells (nonlocal_cells) | `nonlocal y` | 281 | 0 | 0 | 4 | closures already capture |
| named patterns + definitions by cases (named_patterns, definition_cases) | `pattern Pair = [_, _]`; `fib(0) := 0; fib(n) := …` | 405 | 0 | 0 | 1 | match arms and guards |
| object-shaped parameters (parameter_shapes) | `f(p{name, title?}) := …` | 230 | 0 | 0 | 1 | named arguments |
| ruby blocks (blocks.rs lower_trailing_blocks) | `each(xs) { |x| … }` with `yield v` | 215 | 0 | 1 | 1 | lambdas passed as arguments |
| version literals (versions) | `v1.2.3`, `version 1.2.3` | 210 | 0 | 0 | 4 | texts `"1.2.3"` compared by parts |

Well used (no cut proposed): library words 1909, tasks 1473, broadcasting 1199, blocks 955, switch/match 882, late
binding 737, for loops 725, constant folding 670, lazy ranges 654, comprehensions 631, type tests 596, named
arguments 574, markup tags 398, type constructors 370.

## cleanup-drop-lambdas (2026-10-14, supervisor: stop at step 1)
- Done: function_values.rs no longer specialises a function per function argument (`apply__double`); every call passes
  a closure, a capture-free one made once (a global, wasm_emitter/closures.rs). 10M `apply(y => y*2+1, k)`: 0.25 s →
  0.42 s. Side fixes: `f(*args)` of a closure spreads by count (variadic::run_time_spread), `f:(Int)->Int` parameters
  are functions, a called returned closure may return a function (Z combinator).
- Kept: the iteration templates of lambdas.rs (map filter each fold find any all reduce sort) stay the inlining rule
  for literal lambdas. Library functions over closures measured 0.22 s → 0.45 s on 1M map/filter/sum (~3× without
  compile time) and give untyped (any) results, for about −140 lines: not worth it.
- Benchmarks: probes/cleanup/lambdas/bench_*.warp
## `a of b` is `b.a` (card cleanup-of-is-dot, 2026-10-14)
One SUGAR pass, lowering/collections/of_lookups.rs, right after interpolation, rewrites every `a of b` to `b.a` (`first 3 of xs` →
`xs.first(3)`, `byte count of t` → `t.bytes`, `name of people with age > 20` → the field of the filtered list); every
later pass reads only the dot form. Gone: the special `of` arms in library_words, objects, broadcasting, lazy_reads,
reflection, comprehensions' call-argument block, and signal_values' token scanner. Not lookups: a collection type
phrase `list of int` and a spaced definition `square of a number = …`. Measured: about −50 net src lines, not the
card's −700: reflection's fields/methods/dir code is feature code, not `of` handling. New forms that work now:
`params of f`, `a of xs` (a field of every element), `name of p + "b"`.
Trap found: the old signal_values reflected_lists rebuilt every list and unwrapped one-item unbracketed lists on the
way; `func add<T>(_ a: T, …)` relied on it, so the new walk keeps that unwrap explicitly.

## Final pass (final-repo, 2026-10-16): dead scan, probes, notes, playground JS, lib/
- Dead scan (probes/cleanup/dead_scan.py, now keeping glob re-exports and those naming items used outside, which made
  97 false hits under src/law and the CLI's users): across default, all-features and wasm32 only the kept items of
  the first pass are left (gc_traits reading API, last_item, set_hint_file) plus `Exact::standard_part`
  (src/extensions/reals.rs; removed on branch final-media). Two re-exports nothing read went.
- Probes: the code-cleanup tools live in probes/cleanup/ (dead_scan, similar_names, duplicate_constants,
  drop_function, call_to_method, move_module_joins); the shared-checkout hunk selector and the early Lean probes
  went; the reusable checks nobody listed are in notes/probes_layout.md. Still unreferenced by notes, tests, probes
  or board try lines: 132 one-off .warp inputs (list: data/cleanup/unref_probes2.txt); kept, they cost nothing.
- Notes: `probes/cleanup/dead_note_paths.py --fix` follows git's renames (and x.rs → x/mod.rs) to rewrite stale
  paths, leaving lines that name the old path on purpose ("was", "gone", "removed"). 98 dead paths remain, nearly
  all of deleted files in history notes (done.md, code_quality.md's audit table, cloud_tasks.md).
- Playground JS (jscpd: 0.2% exact duplication): gpu_compute's two routes share one compute pass and the buffer
  helpers; worker.js reuses host-compiler's passedBytes.
- lib/: `round_to` is defined in lib/prelude.warp and lib/math.warp alike; dropping math's would take it out of
  `dir(math)`, so it stays (card). lib/compiler's lexer `is_blank` is prelude's `is_space`, kept as the Rust twin's name.
## Final pass (final-emitter, 2026-10-16): src/wasm_emitter/, src/run/, src/type_kinds.rs
Behaviour-preserving, +352 −587 lines. Byte check: every probe and sample compiles to the same wasm as before
(probes/cleanup/same_wasm.sh), except the intended superset below and embedded doc text naming moved sample paths.
- Runtime function dependencies (src/wasm_emitter/runtime_dependencies.rs): each module declares `dependencies()`
  (`Dependency`, `any`/`all`), `require_dependencies()` closes them to a fixed point. The old ordered if chain missed
  `function_text` when `list_join` was required late; vector_shader, zero_argument_shader, math_signatures and
  warp_compiled_wgsl_shader now also emit it. A new runtime function = one line in its module's table.
- `Kind::of_field` / `Kind::ALL` replace tag comparisons in wasm_reader; RawFieldValue conversions by one macro;
  unused `AstKind` gone.
- WasmGcEmitter, TypeManager, ImportManager derive Default (−120 lines of initializers); one `emit_struct_types`.
- Emitter words via `words::GLOBAL_WORD` / `RETURN_WORD` / `host::FETCH_WORD`; BUILTIN_CALLS no longer repeats
  NODE_ARGUMENT_CALLS.
Left for later (cards): emit_kind_globals' hand-written name table (derive from Kind::ALL); module_cache and
program_cache each hash to hex (one shared hex_digest, also modules/cargo_crates); host/native.rs and
host/tasks/values.rs could match Kind::of_field (host area); list_emitter KNOWN_WORDS as a mechanism.

## Final pass (final-host, 2026-10-16)
Area: src/host/, src/ffi/, src/gc_traits/, src/modules/, top-level src/*.rs (not extensions.rs). Net −410 lines.
- ffi: one `NativeCall` wrapper for every C function; aliases interned once (resolve_library_alias leaked a string
  per call); header signatures in one collect; header.rs comments match the code
- host: `exported_memory` (one lookup of a caller's memory for host words, gpu, tasks, foreign calls, module linking),
  `answer_for_text`, one `HostNode` type; database.rs's SQLite API as one `sqlite_api!` table
- gc_traits/wasm_name_resolver.rs matches on wasmparser's own types (the Parsed* mirrors gone, −245)
- modules: one program_loader, `qualified_call`, `instance_memory`; rustdoc's `Conversion::list_of/option_of/borrowed`
  and its `kebab` = components' `kebab_case` + validity check
- pipeline `without_located_errors`, law `add_free_variables`
Left on purpose:
- needed_definitions `with_statements_first` / `with_statements_in_front` differ in Meta handling and separator
- main.rs's long if/else command dispatch: not touched
- gc_traits ObjFieldValue / obj! and the GcObject reading API: the documented rasm pattern
## Final pass (final-analyzer)
Done on branch final-analyzer (behaviour-preserving): unused Node helpers out, definitions/imports/return-kind helpers,
one Pratt try_parse chain, BOOL_TYPES / NONE_WORDS read from the type registry, stray doc lines back on their
functions (probes/cleanup/stacked_docs.py finds them), `\:name` entity escape and `Index<&str>` flattened.
Found, not done:
- warp_parser/mod.rs holds ~200 keyword constants for every submodule: each belongs in its one user file
  (left: conflicts with every open parser branch).
- `matches!(x.drop_meta(), Node::List(_, Bracket::Curly, _))` is hand-written ~67× repo-wide while
  lowering::nodes::is_block exists; `Node::is_block` in node/ is its home.
- USE_WORD "use" is re-declared in 5 lowering files beside modules::USE_KEYWORD.
- warp_parser::mentions (enters template holes) and Node::mentions differ only by the holes: name them precisely.
- lookahead.rs group_holds_comma and comparison_follows_and repeat a quote/depth scan; a shared scanner needs the
  two-glyph operator skip, so it was left.
- analyzer/imports.rs extract_declared_ffi_imports: the nested `(use X from Y) …` arm also adds X as a library
  (kept). The `import: from lib` arm that declared a function named "import" went (import-lib, Interviewer).
## Final pass (final-media): src/sound/, src/graphics/, numbers and units (2026-10-16)
Done (behaviour-preserving, no test edits):
- sound: one `Chain::process_changing` runs a chain whose effects change at sample indices, for save sound
  (effects::processed) and the live buses (sound_engine Bus::process) alike; `Comb::gap`; one per-channel maker in
  Stage::new; the non-macOS engine stubs are one `mod output`; `paced_when_heard` (rest and sounded); `added_into` for
  render's two mix additions; one FORMATS list (players, probes, the "not a sound file" message); `signal_pcm` shared
  with audio_files (its `scaled` was the same); adapters' `whole_of`; midi receive flattened.
- `extensions::utils::numbered_temporary_file`: sound's WAVs and paint's PNGs (`<stem>.<ext>`, then `<stem>-N.<ext>`
  in the temporary folder) share one writer.
- paint: `shown()` holds the window attempt of paint, paint_rgba and paint_shader; `rgba_pixels`.
- gpu: `validated()` pops a validation scope for checked_values, render_texture and read_backs; `readback_of` and
  `image_texture` make every readback buffer and image texture; shader_holes::mentions (a name not continued by a
  name character) serves the alias hints and gpu's reads_value.
- numbers: the overflow trap names and the ± run-time function names are derived from the type or word by
  `width!` / `interval_word!` (concat!, so they stay &'static str for the emitter) instead of spelled twice; one
  generic `deserialize_leaked`; Div's ratio arm by is_rational; Monomial::times extracts square and cube roots in one
  loop; Interval::truncated; Exact::standard_part removed (st(x) is gone, user 2026-10-14).
Left, because open branches change these files (2026-10-16): src/numbers/units.rs, units/static_units*,
units/definitions.rs and lib/units.warp (fix-field-unit, fix-live-loops, fix-unit-hint, program-units,
runtime-quantities, display-names); src/graphics/gpu_canvas.rs and paint_window.rs (draw-shader-warp, paper-shape);
src/numbers/real.rs (why-sqrt). A later pass should look at them once those land.
Seen, not worth it or not mine:
- gpu.rs with_builtin_values matches the built-in hole names a second time beside shader_holes::BUILTIN_HOLES; a
  table of (name, value) needs the frame's locals, so the match stays.
- audio_files CONVERTERS and sound/mod.rs Command (players, probes) are two "first command the system has that knows
  the format" lookups with different shapes (arguments around the input and output paths vs before the file).
- extensions/lists.rs: `find2` has one caller (node/indexing.rs), `Filter` for Vec/Node none found in src; the file
  is the user's extension style, left for the user.
- Number's Display and exact_decimal each call terminating_decimal; uncertain.rs unsaid_ordering and overlap_warning
  spell the three readings each (a shared helper came out longer after rustfmt, so it was dropped).
