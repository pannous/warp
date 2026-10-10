# Cleanup (user card `cleanup`: major code cleanup and simplification)

Behaviour-preserving only: shared helpers instead of duplication, dead code and stale comments out, flatter control
flow, meaningful names, no test edits. One area per session, small themed branches through the Integrator.

## src/analyzer/, src/warp_parser/, src/node/ (card cleanup-analyzer, session warp-types)

Done (branches cleanup-dead, cleanup-arith, cleanup-names, cleanup-parser, cleanup-modes, cleanup-guards, cleanup-lookahead, cleanup-atoms, cleanup-errors,
cleanup-groups):
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
- ffi/link.rs create_ffi_wrapper: ~20 hand-typed arms (`"II_I"`, `"IIP_V"`, …) of the same shape; a macro would
  shorten them, and the generic wrapper could take most of them if it passed f32 arguments right (it passes f64s),
  which needs FFI tests for each signature first.
- markup.rs escapes `<pre>` text without `>`, site.rs `escaped` with it: one escaper would change the error page.
