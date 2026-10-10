# Cleanup (user card `cleanup`: major code cleanup and simplification)

Behaviour-preserving only: shared helpers instead of duplication, dead code and stale comments out, flatter control
flow, meaningful names, no test edits. One area per session, small themed branches through the Integrator.

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
- atoms.rs parse_symbol_with_suffix (~140 lines): still a keyword dispatch
- atoms.rs parse_atom (~140) and parse_type_declaration_body (~120): long but flat now
- declaration_lowering.rs lower_declarations_among (~110): a long but now flat rewrite dispatch
- two `matches!` + `unreachable!("guarded")` arms stay where the arm moves the scrutinee the guard borrows
  (parameter_copies, `x as number = 9`)
- lookahead.rs peek_operator (123): a flat operator table, left as is (keywords interleave with the glyph lengths)
- node/comparison.rs eq: left alone, it carries the user's comments

Outside this area (for whoever takes src/lowering/): private `is_word` / `symbol_name` / `is_symbol` copies in
lowering/comprehensions, for_loop, generators, lazy_ranges, stored_values, welcome_forms, inlining, soft_keywords,
tuples, and law/type_model.rs, modules.rs, time.rs can use `Node::is_symbol` / `Node::symbol_name`.
# Code cleanup (user card `cleanup`: "major code cleanup and simplification")
Behaviour-preserving only: shared helpers for duplication, dead code and stale comments out, flatter code, no test edits.
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
- `Node::as_items()`: a list's items, none of ø, else the node as the one item. Copies left for the lowering areas:
  lowering/class_methods.rs (2), component_worlds.rs, type_constructor.rs; warp_parser/mod.rs:204 differs (no
  drop_meta); site.rs items_of keeps ø as an item on purpose
- `Node::parts()`: a list's items or a key's two sides, borrowed (units.rs). Copies left for the lowering areas:
  lowering/class_methods.rs children_of, lowering/generators.rs parts
- wisp_parser: the typed s-expression nodes through finish_one / finish_key / finish_constant and one call_of
- wisp emitter: one emit_form for its `(word part …)` forms; gc_traits memory_text reads a $String for GcString and
  the debug formatter; gc_struct!/wasm_struct! field arms take an optional rest
- tasks::task_failure for the TaskFailure constructions, host module_memory for "the module exports no memory"
- compile_time.rs: the Stop, fail and answer_of of real.rs and units.rs (units/static_units.rs keeps its own Stop: it
  has a third case, Recursive)
- ffi/link.rs create_ffi_wrapper: the 18 typed arms are one link_typed! each (NativeArgument / NativeResult convert
  the values, CBool a C bool)

Left (bigger, needs care):
- Two C header parsers: ffi_parser.rs (`parse_declaration`, one line at a time, C types → Kind → ValType via
  kind_to_valtype) and ffi/header.rs (`extract_function_signature`, multi-line declarations, C types → ValType via
  map_c_type_to_valtype, pointer classes). get_ffi_signatures uses the first, get_signatures_from_headers the second,
  and get_ffi_signature mixes both. One parser would change some mappings (Kind::Pointer is i64 there, a pointer is
  i32 here), so it needs its own card and tests of both tables first. tests/ffi pins ffi_parser's public API.
- real.rs and time.rs are two small constant evaluators with their own Value types and the same Assign/Define arm;
  sharing a scope/evaluator skeleton would remove more, but they differ in errors (Stop vs String).
- src/extensions.rs is hard-linked with ~/dev/script/rust/extensions.rs (src/extensions.rs.hardlink is the
  hardlink-guard's marker): edit it only knowing it changes that file too.
- host.rs (~170 `#[cfg(feature = "native")]`-gated items) would read better split into host/native.rs, but it is the
  hottest file of the repo (dozens of commits a day): split it in a quiet hour, as its own branch, nothing else in it.
- Two GcObject types: wasm_reader::GcObject (node/mod.rs, re-exported by lib.rs) and gc_traits::GcObject (tests'
  ergonomic reader). One of them could wrap the other; the public API of both is pinned by tests/wasm.
- headless.rs node_of_json and foreign.rs node_of both turn JSON into Nodes; they differ in separator (None vs Space),
  arrays (ø vs list) and a non-integer number's fallback, so one replacing the other changes output.
- ffi/link.rs: the generic wrapper could take most typed signatures if it passed f32 arguments right (it passes
  f64s), which needs FFI tests for each signature first.
- markup.rs escapes `<pre>` text without `>`, site.rs `escaped` with it: one escaper would change the error page.

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

Left (each changes behaviour or needs care):
- Node::map_children also enters class bodies (Node::Type); children_rewritten does not. ~40 more passes spell out
  the tail with a custom List or Key arm in between or a guard; folding those in needs a case-by-case look.
- statements_of (class_methods, event_signals, late_binding), statements (component_worlds, test_blocks),
  is_statement_list (generators, result_word, variable_signals), is_return (closures, generators,
  type_name_matching), loop_variable(s) (library_words, run_time_blocks, signal_values, for_loop): same name,
  different bracket/separator rules each, so one shared version changes which forms match.
- soft_keywords::parameter_name accepts only `:` / `=` keys and does not recurse, unlike nodes::parameter_name;
  go_blocks / named_arguments / parameter_shapes have their own parameter_name returning String or Node.
- class_methods.rs (~2600 lines): destructurings / positional_fields share their class_of setup (431/444), two
  class-collecting visits (1848/2003); worth its own split into files.
- temporary-name makers (lazy_ranges, min_max, parallel, list_element_checks, named_arguments) each format their own
  prefix; one `Temporaries` counter type could serve them, names must stay byte-identical (tests pin some).

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
emitter-builders, emitter-equality, emitter-runs, emitter-deterministic):
- never-called functions and stale comments out; runtime functions through runtime_function / exported_function
- shared helpers for repeated code: emit_while, emit_text_argument_bounds, push_in_place, emit_global_store_declared,
  emit_offset_locals_test, emit_list_cell_function (list_at / list_node_at), emit_nth_cell_data, emit_try_table,
  emit_growable_list_types, string_fields, compile_time_string, emit_local_step (i++ of arithmetic.rs's two paths),
  emit_defined_user_function_call, import_function (host, WASI and FFI imports)
- runs of single `f.instruction(&I::…)` lines as one `Self::emit_list(f, &[…])` per statement group:
  `probes/cleanup/merge_instruction_runs.py <file.rs>…` (rewrites in place; a run splits after statement-ending
  instructions and after `return; end`), done in every emitter file
- `probes/cleanup/duplicate_windows.py` lists repeated windows of normalised lines (the duplicates left to share)

Left:
- list_ops.rs (~290 single instructions, the longest file): text count loops repeat; candidates for emit_while
- equality.rs: the return tails of the per-kind comparisons repeat
- user_function_calls.rs compile_user_function_body saves and restores ~15 emitter fields by hand, interleaved with
  computing the new values: one saved-state struct would halve it, but the order of the computations matters
