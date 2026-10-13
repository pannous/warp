# Cleanup (user card `cleanup`: major code cleanup and simplification)
Roadmap card: cleanup (board topic; its sub-cards are the areas below and their leftovers, notes/roadmaps.md).
Repo-wide leftovers: oversized files (cleanup-big-files), never-called functions over every area (cleanup-dead-scan),
fmt/clippy drift (cargo-fmt, clippy-reports), probe-script proliferation (cleanup-probe-scripts), stale notes
(cleanup-stale-notes).

Dead scan (cleanup-dead-scan, 2026-10-13, probes/dead_scan.py): 15 never-used pub functions removed, unused in the default,
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
- (cleanup-long-functions) atoms.rs parse_symbol_with_suffix (~140 lines): still a keyword dispatch
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
- (cleanup-ffi-headers) Two C header parsers: ffi_parser.rs (`parse_declaration`, one line at a time, C types → Kind → ValType via
  kind_to_valtype) and ffi/header.rs (`extract_function_signature`, multi-line declarations, C types → ValType via
  map_c_type_to_valtype, pointer classes). get_ffi_signatures uses the first, get_signatures_from_headers the second,
  and get_ffi_signature mixes both. One parser would change some mappings (Kind::Pointer is i64 there, a pointer is
  i32 here), so it needs its own card and tests of both tables first. tests/ffi pins ffi_parser's public API.
  Measured 2026-10-13 on macOS (m, c, SDL2): ffi_parser's table has 514 functions, the header table 725; 309 differ
  and 11 are only in the first. The differences: FILE* and other pointers i64 there, i32 here; char* results a number
  there, a text (anyref) here; long double results anyref there, i32 here; a declaration with a trailing macro call
  (`strtod(…) __DARWIN_ALIAS(strtod)`) misread here (card ffi-header), `char **` parameters dropped here. One parser
  changes the signatures of hundreds of implicit calls (get_ffi_signature takes the ffi_parser table first), so it is
  a behaviour change with decisions per difference, not a cleanup: parked in Later.
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
  src/lowering/words.rs; a constant with its own doc comment stayed in its pass
- identical bodies (found by comparing fn bodies with parameters renamed): `block`, `if_then`, `if_then_else` moved
  from variable_signals to nodes.rs and replace ~12 spelled-out `key(key(ø, If, c), Then, b)`; `is_block`,
  `is_colon_pair`, `is_binding`, `is_function_keyword` (2 copies each) → nodes.rs; two `statements`/`entries` → Node::as_items
- `match x.drop_meta() { Node::Symbol(n) => Some(n.clone()), _ => None }` (16 sites) → `x.symbol_name().map(String::from)`;
  `matches!(x.drop_meta(), Node::Symbol(w) if w == WORD)` (43 sites) → `x.is_symbol(WORD)`
- the `Cell<usize>` counters of 15 passes (get then set, set then get, replace) → `nodes::Counter` with
  `next_number()`; a pass counting from 1 starts it `Counter::starting_at(1)`, so every name stays as it was; then the
  `usize` makers of lazy_ranges, min_max, list_element_checks and variable_signals (`+= 1` then use) the same way

Left (each changes behaviour or needs care):
- (cleanup-rewrite-tails) Node::map_children also enters class bodies (Node::Type); children_rewritten does not. 45 tails folded by
  probes/cleanup/fold_rewrite_tails.py: the generic Meta arm and `other => other`, with Key and List each either
  generic (folded) or taken by an unguarded arm of their own. The ~20 left differ: no List or Key arm at all (folding
  would make them recurse), `?` inside (folding.rs), a Meta arm doing more than recursing — each a case-by-case look.
  Done (2026-10-13): 8 more folded by hand (real.rs, interpolation.rs, normalize.rs as_written, user_functions.rs
  rename_nested_calls; and a Meta arm in front of `other.map_children` with the same rewrite, which is map_children's
  own Meta arm: time.rs, comprehensions.rs, generator_consumers.rs, atoms.rs). The ~45 Meta arms left belong to
  walks down one path (the statement under its comments, the leftmost operand), not to rewrites of every child.
- (cleanup-statements-of) statements_of (class_methods, event_signals, late_binding), statements (component_worlds, test_blocks),
  is_statement_list (generators, result_word, variable_signals), is_return (closures, generators,
  type_name_matching): same name, different bracket/separator rules each, so one shared version changes which forms
  match. The `for v in …` variable is one nodes::for_in_variable (run_time_blocks, signal_values with its five-word
  form, for_loop); library_words keeps its own (`for each v in` too, any word's name).
- soft_keywords::parameter_name accepts only `:` / `=` keys and does not recurse, unlike nodes::parameter_name;
  go_blocks / named_arguments / parameter_shapes have their own parameter_name returning String or Node.
- (cleanup-big-files) class_methods.rs (~1920 lines): child modules (`use super::*`, items pub(super)) class_methods/operators.rs
  (operators on instances, specialized calls), inheritance.rs (extends, mixins, super), other_languages.rs (impl, Go
  methods, Ruby .new, witness methods); further parts only if a cohesive group shows up (generic classes?).
  destructurings / positional_fields already share with_value_classes.

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
- `probes/cleanup/duplicate_windows.py` lists repeated windows of normalised lines (the duplicates left to share)

Left:
- (cleanup-emitter-loops) ~70 other hand-written `Block, Loop, <exit test>, BrIf(1), …, Br(0), End, End` loops: counters, byte walks and
  cell walks whose step differs from emit_cell_walk (RefAsNonNull before the rest field, a position bump after it,
  or the loop opened and closed by two closures as in library_ops.rs join and slice). A general
  `loop_until(f, exit, |f| body)` would save ~1 line per loop; each conversion checked with same_wasm.sh.
  Closed as not worth doing (2026-10-13): most of the ~90 loops sit inside one flat instruction slice, often with an
  inner loop or an exit two levels out (`BrIf(2)`, text_grapheme_back); a closure helper cuts each slice into before,
  exit, body and after pieces, which costs as many lines as the Block/Loop/Br/End lines it saves and adds nesting
- (cleanup-saved-state) user_function_calls.rs compile_user_function_body saves and restores ~15 emitter fields by hand, interleaved with
  computing the new values: one saved-state struct would halve it, but the order of the computations matters
