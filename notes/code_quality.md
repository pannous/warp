# Code quality analysis (2026-10-03, origin/main ddb5dc4d)

Analysis only, nothing changed. Numbers come from scripts run over `src/` and `tests/`: line counts, a brace-matching
function-length and nesting scan, a `crate::` dependency graph, 6-line clone windows, `cargo check` with the crate-wide
`#![allow(dead_code, unused_imports)]` temporarily removed, and grep for unreferenced `pub` items.
"Mechanical" means fastmod/semgrep or a pure file move with re-exports; "Decision" means the user has to choose.

src: 88 files, 41 101 lines, 2 274 functions (82 over 50 lines, 35 over 100). tests: 225 files, 17 814 lines,
1 500 `#[test]`, 91 ignored.

## Status (2026-10-03, branch `code-quality`)

Done, behaviour-preserving (full suite, default features, green through the function builder; the rest checked by
`cargo check --all-targets --all-features`, the CLI and targeted runs of test_ffi, test_float_promotion, test_wasi,
test_host, test_wasm_reader, test_gc_struct; the Integrator's full run is the final check):
- #1 main.rs uses the library (`use warp::…`), no second compile of every module.
- #3 deleted `compiler/`, `run/wasmer_runner.rs`, `run/wasmedge_runner.rs`, `ast.rs`, `wasm_emitter/node_emitter.rs`,
  `extensions/_mod.rs`, `bin/test_op.rs`, the dead `run_wasm`, `test_func`, `register_import`, `ArithmeticWrap::None`;
  crates `parity-wasm`, `wasm-ast`, `regex` removed; `wat`, `once_cell` are dev-dependencies (std `LazyLock` in src).
- #6 `wasm_emitter/function_builder.rs` (`runtime_function`, `exported_function`, `emit_list`); list_ops, getters,
  `i64_pow` use it; the 8-arm `emit_constructor!` macro is one `emit_node_constructor` function.
- #7 `KIND_BITS`, `KIND_MASK`, `CURLY/SQUARE_BRACKET_INFO`, `CURLY/SQUARE_LIST_KIND` in type_kinds.rs; `BYTE`, `WORD`
  in `wasm_emitter/layout.rs`; WASI stdout writes share `emit_stdout_write`.
- #8 (part) `wasm_reader::run_main` serves plain, host, WASI and FFI runs; FFI and `.wat` results go through the
  generic `val_to_node`, which learned Float (FFI returned Empty for texts and lists before); CLI `run`/`run_wat` failures are error
  nodes; `util::fetch` delegates to `download` instead of a second panicking ureq call.
- #9 (part) the 15 libm bindings are a table (`LIBM_UNARY`, `LIBM_BINARY`); the two header parsers remain.
- #14 AGENTS.md architecture/build/test sections match the tree (CLAUDE.md untouched, see question 8).

#2 (user decision 2026-10-03, "Remove them"): the 21 C++ feature flags and their `#[cfg]` branches in src/main.rs,
tests/test_wasm.rs and tests/test_web.rs are gone (the default-build branch kept); `--all-features` is now `native` +
`optimizer` + `ffi`, the same program as plain `cargo test` plus the optimizer and FFI tests.

smarty.rs (user decision 2026-10-03): deleted with its asserts (`test_smart_types` in tests/numbers/test_angle.rs) and
tests/test_asts.rs, the only user of the `syn` dev-dependency, which is gone too.

Left, because they need a decision, edit tests, or would collide with the sessions editing the same files now:
#4/#5/#10 moves and splits (every open branch touches mod.rs, analyzer.rs,
wasp_parser.rs), #12, #13, #15–#20, smarty.rs and test_asts.rs, the extensions/ dead traits (extensions.rs says it is
linked from ~/dev/script/rust), the crate-wide `#![allow(dead_code, unused_imports)]`.
Observed once: a 4-thread run aborted in `test_struct_types::test_magic_object_mismatch` (should_panic) with
`assertion failed: a.comes_from_same_engine(b.engine())` while panicking; it passes alone (see TODO).

## Summary, ranked by value/effort

| # | Finding | Files | Sev | Effort | Proposal | Kind |
|---|---------|-------|-----|--------|----------|------|
| 1 | `main.rs` redeclares all 45 modules with `mod x;`: the whole compiler is compiled twice (lib + bin), 107 bin warnings | src/main.rs | High | S | `main.rs` uses `warp::…`, drop its `mod` list and the "modules also need to be used in main.rs" comments in lib.rs | Mechanical |
| 2 | 21 stale C++ feature flags (`WASM`, `WASMEDGE`, `RUNTIME_ONLY`, `release`, `wasm`, …); agents run tests with `--all-features`, which flips 53 `#[cfg]` branches (35 in tests/test_wasm.rs, 15 in main.rs, 3 in test_web.rs) | Cargo.toml, src/main.rs, tests/test_wasm.rs, tests/web/test_web.rs | High | S | Keep `native`, `optimizer`, `ffi`; delete the rest and their `cfg` branches (keep the branch that runs today) | Mechanical + test edit (needs OK) |
| 3 | Dead files and dead dependencies: `compiler/` (wasm_reader declared but unused, parity_wasm_reader not even declared), `run/wasmer_runner.rs`, `run/wasmedge_runner.rs`, `ast.rs`, `wasm_emitter/node_emitter.rs`, `extensions/_mod.rs`, `smarty.rs` (self-described OBSOLETE C++ ABI), `bin/test_op.rs`; crates `parity-wasm`, `wasm-ast`, `regex` used by nothing, `syn` (dev) used only by a test of syn itself | 10 files, ~1 000 lines, 4 crates | High | S | Delete (list in §4) | Mechanical; smarty/test_asts need test OK |
| 4 | The compile pipeline (`lower_for_emission`, `eval*`, `compile`, `run_module`, ~330 lines) lives in `wasm_emitter/mod.rs`; 25 lowering passes are flat `src/*.rs` files, composed by a 7-deep nested call | src/wasm_emitter/mod.rs, 22 src/*.rs | High | M | `src/pipeline.rs` (driver) + `src/lowering/` with an ordered `PASSES` table (§1) | Mechanical moves; pass order unchanged |
| 5 | One 42-module dependency cycle (every module except leaves); `wasm_emitter` fans out to 41 modules, `node` (the core type) depends on parser, wasm_reader, gc_traits; `extensions` depends on analyzer and emitter | whole crate | Med | M | Follows from #4, #8, #9: node becomes a leaf, emitter stops importing passes, test macros leave `extensions` | Design |
| 6 | Function boilerplate repeated 18×: type + `functions.function` + `Function::new` + `End` + `code.function` + `register_func`; the helper `runtime_function` already exists but hides in big_int.rs | wasm_emitter/{mod,constructors,list_ops}.rs | Med | S | Move `runtime_function`, `emit_list` to `wasm_emitter/function_builder.rs`, use them everywhere | Mechanical-ish |
| 7 | Constants redefined per file: `KIND_MASK` ×6, `BYTE: MemArg` ×5, `KIND_SHIFT`/`SQUARE_BRACKET_INFO` ×2–3, plus 14 inline `MemArg {…}` | wasm_emitter/*.rs, web.rs | Med | S | One `wasm_emitter/layout.rs` (or in type_kinds.rs next to `Kind`) | Mechanical |
| 8 | Four near-identical "instantiate and call main" functions, engine/linker setup in 6 files, three HTTP getters (`util::fetch` panics, `extensions::utils::download` returns "", `host::fetch` returns Result) | wasm_reader.rs, run/, host.rs, util.rs, extensions/utils.rs, wasm_emitter/mod.rs | Med | M | `src/runtime/` with one `run_main<S>(bytes, state, link)`; one `net::get(url, timeout) -> Result` | Mechanical-ish |
| 9 | Two C-header parsers and five C-type mappers; `link_ffi_functions` is 298 lines of 20 hand-written copies of the same closure | ffi.rs, ffi_parser.rs | Med | M | One parser in `ffi/header.rs`, one `c_type_kind`; a `(lib, name, arity)` table with `link_f64_unary/binary` | Mechanical-ish |
| 10 | Giant files: wasm_emitter/mod.rs 3 832, wasp_parser.rs 3 491, analyzer.rs 3 139, node.rs 2 469, gc_traits.rs 1 730, ffi.rs 1 531 | 6 files | Med | L | Splits in §2 | Mechanical moves |
| 11 | Giant functions: `emit_numeric_value` 336, `link_ffi_functions` 298, `emit_exact_functions` 283, `emit_list_ops` 256, `continue_expr` 233, `emit_cast` 229, `main` 202 | §2 | Med | M–L | Dispatch tables / one function per case | Refactor |
| 12 | Every lowering pass re-runs `extract_user_functions` over the whole program (12 call sites) and re-implements the Node recursion (`Lowering::expand` in 5 files, 41 `Meta` re-wrap arms); `node::map_node` exists unused | lowering passes | Med | M | `Node::map_bottom_up(f)` + one `Program` context computed once per pipeline run | Design |
| 13 | `panic!` on user-reachable paths despite decision #1: node arithmetic `Cannot add …` (4), `IndexMut` (3), wisp_parser (13), `extensions/numbers.rs` (6), `FunctionRegistry::get`, `todo!` in node.rs (3) and main.rs (2) | §5 | Med | M | Error values (`Node::Error`) or `Result`; see notes/panic_sweep.md for the pattern | Mechanical per site |
| 14 | AGENTS.md describes files that no longer exist (`src/emitter.rs`, `src/wasm_gc_emitter.rs`, `docs/…`, `tests/node_test.rs` …, `examples/`, `vendor/`, wasmer/wasmedge runners); CLAUDE.md is a diverged copy (16 differing lines) | AGENTS.md, CLAUDE.md | Med | S | Rewrite the architecture section from §1; CLAUDE.md → symlink to AGENTS.md | Decision (user owns both) |
| 15 | tests/: 225 flat files; `probe_*.rs` (7) inside the suite, tests/probes/*.rs not compiled, tests/notes/ is notes, tests/test_utils.rs a dead module cache (11 warnings), 250 duplicated assertion lines (65 inside test_wasm.rs alone) | tests/ | Low | M | Topic subdirectories, move orphans out (§6) | Decision (tests are append-only) |
| 16 | Two instruction styles: `func.instruction(&Instruction::X)` (657 sites, mod.rs/constructors/wasi/ffi_emitter) vs `use Instruction as I` (1 241 sites) | wasm_emitter/*.rs | Low | S | fastmod to the `I::` style, then `emit_list(func, &[…])` for straight-line runs | Mechanical |
| 17 | Root clutter: `goo`, `conversation.md` (92 KB chat dump), `node.wat`, `test.wasp`, dangling `wasp -> target/debug/wasp`, `warp -> target/release/warp` (no target/ in repo), `build_debug.sh`, `add_ignore_to_failing_tests.sh`, `nextest.sh` beside `test.sh` | / | Low | S | Delete or move to notes/OLD, probes/ | Decision |
| 18 | 279 commented-out code lines (59 extensions/lists.rs, 39 strings.rs, 34 ast.rs, 32 node.rs, wasmedge 13), 178 trivial "// Get x / // Create y" comments (55 in wasm_emitter/mod.rs) | §4, §5 | Low | S | Delete | Mechanical |
| 19 | 517 numeric literals outside `const` (calendar 86, strings 67 Unicode ranges, operators 53 precedences, text_unicode 51, big_int 43), local slots by number (`LocalGet(5)` ×16) | §5 | Low | M | Named consts at file top; named scratch locals | Mechanical per file |
| 20 | Cryptic names: `laste`, `typ`, `todow`, `tee`, `Dada`, `peq!`, `s!`/`strings!`/`Strings!`, `fetch(p0)`, `smarty32`, `float_data28`; vulgar comments in Cargo.toml/extensions.rs | §5 | Low | S | Renames (§5) | Mechanical (semgrep) |

## 1. Module structure

### What src/ looks like
Six directories (`wasm_emitter/` 21 files, `extensions/` 6, `run/` 4, `compiler/` 3, `law/` 1, `time/` 1, `bin/` 1) and
**50 flat files** in `src/`. The flat files are four different kinds of thing mixed together:

| kind | files |
|------|-------|
| syntax | wasp_parser, wisp_parser, operators, node, meta, ast (dead), interpolation (half parse, half lowering) |
| lowering passes (Node → Node, called from `lower_for_emission`) | mutation, modules, type_name_matching, meta_entries, versions, injection, interpolation, function_equality, type_tests, ambiguous_forms, lambdas, function_values, real (`lower`), type_constructor, min_max, declarations, switch, phrase_words, library_words, for_loop, plus `host::lower_aliases` and `analyzer::{lower_negated_calls, lower_list_times, lower_declarations, resolve_data_scope, resolve_main_variable_assignments}` |
| analysis | analyzer, context, local, function, type_kinds, effects, diagnostic, normalize, law/, units, time/, real (`answer`), fixed_width |
| native runtime | host, ffi, ffi_parser, gc_traits, wasm_reader, run/, util (engine config), package_tools, wasm_optimizer |
| misc | extensions/ (Rust std helpers **and** the test macros `is!`/`eq!`/`skip!`), smarty (dead), web, main, lib |

Names that don't say what's inside: `util.rs` (wasmtime engine and fuel, plus a panicking `fetch`), `smarty.rs`,
`extensions.rs` (test macros), `context.rs` (the analyzer's function table), `meta.rs` (`Dada`/`LineInfo`),
`host.rs` (host imports *and* a lowering pass *and* HTTP), `type_tests.rs` (sounds like tests, is the `x is int` pass),
`library_words.rs` (a 573-line lowering pass), `compiler/` (contains no compiler).

### Proposed layout (moves only, pass order and behaviour unchanged)

```
src/
  lib.rs                 re-exports only
  main.rs                CLI only, `use warp::…` (finding #1)
  pipeline.rs            from wasm_emitter/mod.rs 3499–3812: compile, eval, eval_untrusted, eval_parsed,
                         lower_for_emission → lowering::run, run_module, out_of_fuel, explain_runtime_error
  syntax/                node/ (split, §2), wasp_parser/ (split, §2), wisp_parser.rs, operators.rs, meta.rs
  lowering/
    mod.rs               `pub const PASSES: &[(&str, fn(Node) -> Result<Node, Node>)]` in today's order;
                         `run(node)` folds them and stops at the first Error (replaces the 7-deep nested call
                         and the four repeated `first_error()` checks)
    aliases.rs           ← host::lower_aliases, renamed_heads
    negated_calls.rs     ← analyzer::lower_negated_calls, negate_calls, applicable_function_names
    list_times.rs        ← analyzer::lower_list_times … typed_array_declaration (analyzer 1942–2209)
    declarations.rs      ← analyzer::lower_declarations(_among), of_type_declaration + today's declarations.rs (enum)
    data_scope.rs        ← analyzer::resolve_data_scope, kebab_ambiguities, resolve_blocks
    mutation.rs modules.rs type_name_matching.rs meta_entries.rs versions.rs injection.rs interpolation.rs
    function_equality.rs type_tests.rs → type_test.rs, ambiguous_forms.rs lambdas.rs function_values.rs
    type_constructor.rs min_max.rs switch.rs phrase_words.rs library_words.rs for_loop.rs
  analysis/
    infer.rs             ← analyzer 99–352 (infer_type, arithmetic_kind, written_kind, branches_kind)
    scope.rs             ← analyzer 353–838 (collect_variables, Scope, captured_variables)
    user_functions.rs    ← analyzer 949–1092, 2222–2700 (collect/extract functions, params, return kinds)
    checks.rs            ← analyzer 839–948, 1228–1747 (type errors, lint, diagnose, null use, constants)
    counting.rs          ← analyzer 2704–2839 (counting words/units)
    type_words.rs        ← analyzer 1068–1227 (builtin_type_kind, list_type_name, number_type_word)
    context.rs local.rs function.rs type_kinds.rs effects.rs diagnostic.rs normalize.rs → hints.rs, law/
  values/                compile-time value domains: real.rs, units.rs, time/, fixed_width.rs
  wasm_emitter/          codegen only; `analyze_required_functions` and `extract_ffi_imports` move here
                         (they decide what the emitter emits; today they make analyzer ↔ emitter a 2-cycle)
  runtime/               (cfg native) engine.rs ← util.rs, run.rs ← run/wasmtime_runner + wasm_reader::read_bytes*,
                         host.rs, ffi/ (mod, header, link), gc/ (gc_traits, GcObject), package_tools.rs, optimizer.rs
  web.rs
  extensions/            std helpers only; the test macros move to tests/common (or `src/testing.rs`, cfg test)
```

`analyzer.rs` today hosts four of the 25 passes plus inference, scope, checks and FFI import extraction, which is why it
is in 8 of the 26 mutual dependencies.

### Dependency graph (crate:: references, top-level modules)
- One strongly connected component of **42** modules: practically every module can reach every other.
- 26 direct 2-cycles, e.g. `analyzer ↔ wasm_emitter`, `analyzer ↔ wasp_parser`, `node ↔ wasp_parser`,
  `node ↔ wasm_reader`, `node ↔ gc_traits`, `node ↔ normalize`, `extensions ↔ wasm_emitter`, `extensions ↔ analyzer`.
- Fan-out: wasm_emitter 41, analyzer 18, wasp_parser 15, library_words 11. Fan-in: node 42, operators 32, extensions 19.
- Root causes, each fixed by one move:
  1. the pipeline in `wasm_emitter/mod.rs` → `pipeline.rs` removes ~25 of the emitter's 41 edges;
  2. `Node::from_gc_object` / `read_list_from_gc` (node.rs 303–453) → `runtime/gc/` removes node → wasm_reader/gc_traits;
  3. `node.rs` calling `wasp_parser::parse` (`From<&str>`-style helpers) → `syntax/` side;
  4. the runtime-function names the analyzer reads (`VALUES_EQUAL`, `INT_RUNTIME`, `IS_TRUTHY`, `TEXT_FORM`, …) →
     a leaf `wasm_emitter/runtime_names.rs` (no imports), or move their users into the emitter (see above);
  5. test macros out of `extensions` (they call `wasm_emitter::eval`, `analyzer`, `wasp_parser`).

## 2. Size

### Largest 20 files (lines)
| lines | file | lines | file |
|------:|------|------:|------|
| 3832 | wasm_emitter/mod.rs | 741 | effects.rs |
| 3491 | wasp_parser.rs | 684 | extensions/reals.rs |
| 3139 | analyzer.rs | 605 | real.rs |
| 2469 | node.rs | 575 | wasm_emitter/equality.rs |
| 1730 | gc_traits.rs | 573 | library_words.rs |
| 1531 | ffi.rs | 561 | wasm_emitter/exact.rs |
| 1396 | wasm_emitter/list_ops.rs | 558 | function.rs |
| 1213 | wasm_emitter/big_int.rs | 557 | wasm_reader.rs |
| 946 | wisp_parser.rs | 547 | wasm_emitter/library_ops.rs |
| 903 | normalize.rs | 535 | type_kinds.rs |
| 783 | time/calendar.rs | 770 | modules.rs |

### Longest 30 functions (lines, max brace depth)
| len | function | where | depth |
|----:|----------|-------|------:|
| 336 | emit_numeric_value | wasm_emitter/mod.rs:2539 | 7 |
| 298 | link_ffi_functions | ffi.rs:540 | 6 |
| 283 | emit_exact_functions | wasm_emitter/exact.rs:113 | 4 |
| 256 | emit_list_ops | wasm_emitter/list_ops.rs:20 | 3 |
| 233 | continue_expr | wasp_parser.rs:1648 | 6 |
| 229 | emit_cast | wasm_emitter/mod.rs:2291 | 6 |
| 202 | main | main.rs:81 | 4 |
| 187 | emit_list_node | wasm_emitter/list_emitter.rs:72 | 5 |
| 185 | parse_symbol_with_suffix | wasp_parser.rs:1398 | 6 |
| 181 | create_ffi_wrapper | ffi.rs:1081 | 5 |
| 177 | infer_type | analyzer.rs:133 | 9 |
| 174 | emit_raw_struct | wasm_emitter/mod.rs:3321 | 4 |
| 174 | emit_float_value | wasm_emitter/mod.rs:2900 | 6 |
| 172 | parse_xml_tag | wasp_parser.rs:2915 | 5 |
| 166 | emit_values_equal | wasm_emitter/equality.rs:311 | 5 |
| 155 | emit_magnitude_functions | wasm_emitter/big_int.rs:699 | 4 |
| 155 | emit_int_functions | wasm_emitter/big_int.rs:1058 | 3 |
| 139 | emit_key_node | wasm_emitter/key_emitter.rs:12 | 5 |
| 139 | analyze_required_functions | analyzer.rs:2840 | 7 |
| 135 | emit_signed_functions | wasm_emitter/big_int.rs:856 | 4 |
| 132 | emit_text_units | wasm_emitter/list_ops.rs:335 | 2 |
| 131 | parse_ffi_signatures | ffi.rs:368 | 3 |
| 121 | from_gc_object | node.rs:303 | 6 |
| 119 | emit_map_words | wasm_emitter/list_ops.rs:1277 | 5 |
| 118 | to_json_value | node.rs:1199 | 11 |
| 117 | emit_exact_text | wasm_emitter/exact.rs:420 | 3 |
| 116 | parse_declaration | ffi_parser.rs:90 | 5 |
| 115 | lower_declarations_among | analyzer.rs:1754 | 6 |
| 112 | run_wat | run/wasmtime_runner.rs:52 | 10 |
| 111 | emit_with_at_functions | wasm_emitter/list_ops.rs:894 | 3 |

Deepest nesting: `format_gc_val` 13 (gc_traits.rs:250), `to_xml` 12 (node.rs:1105), `val_to_node` 12
(wasm_reader.rs:357), `to_json_value` 11, `run_wat` 10, `extract_ffi_imports` 10, `infer_type` 9.

The runtime emitters (`emit_exact_functions`, `emit_*_functions` in big_int, `emit_list_ops`) are long because they are
lists of `runtime_function(...)` calls; that is acceptable, but each block should become its own `fn emit_<name>` so
the list function reads as a table of contents (as `emit_text_units` already is).

### Proposed splits
- **wasm_emitter/mod.rs (3 832)** →
  `mod.rs` (struct, `new`, setters, `emit`, `finish`, names: ~700) ·
  `user_functions.rs` (312–560: closures, signatures, bodies, calls) ·
  `arithmetic.rs` (1278–1735: arithmetic, compound assign, inc/dec, float ops, truthy logic) ·
  `globals.rs` (1806–1971) · `control_flow.rs` (1972–2189: ternary, if, while) ·
  `casts.rs` (2190–2521, `emit_cast` split per target type) ·
  `numeric_value.rs` (2539–3076: `emit_numeric_value`, `emit_float_value`, `emit_integer_builtin`) ·
  `raw_struct.rs` (3321–3565) · `names.rs` (3201–3298) · driver → `src/pipeline.rs` (3566–3812).
- **wasp_parser.rs (3 491)** → `wasp_parser/mod.rs` (struct, entry points, `parse_expr`, `continue_expr`) ·
  `lexing.rs` (506–910: chars, whitespace, comments) · `operators.rs` (911–1100, user operators 396–492) ·
  `atoms.rs` (1112–1600: atoms, symbols, suffixes) · `control.rs` (1883–2300: if/else/while/for/guards) ·
  `literals.rs` (2423–2830: strings, holes, numbers) · `xml.rs` (786–865, 2915–3087) ·
  `lists.rs` (3088–3311: brackets, separators, grouping) · `rewrites.rs` (3312–end: free functions).
  `continue_expr` (233): one `try_parse_*` per infix family, called from a table.
- **analyzer.rs (3 139)** → see §1 `analysis/` and `lowering/`; nothing should remain in a file called analyzer.
- **node.rs (2 469)** → `node/mod.rs` (enum + accessors, ~600) · `node/index.rs` (Index/IndexMut, 504–683) ·
  `node/constructors.rs` (684–760 + free `int`/`text`/… helpers) · `node/attributes.rs` (737–896) ·
  `node/serialize.rs` (940–1030, `to_json*`, `to_xml`, `from_json`) · `node/equality.rs` (PartialEq impls 1479–1780) ·
  `node/arithmetic.rs` (Add/Sub/Mul/Div 1990–2260, one `binary_number_op` instead of four copies of the
  `(left, left_meta) = match self { Meta … }` prologue) · `from_gc_object` → runtime/gc.
- **gc_traits.rs (1 730)** → `runtime/gc/{convert.rs (FromVal/ToVal), object.rs (GcObject), macros.rs (gc_struct!,
  wasm_struct!, obj!), registry.rs}`; decide whether the two GC reading APIs both stay (§4).
- **ffi.rs + ffi_parser.rs (1 861)** → `runtime/ffi/{signatures.rs, header.rs (one parser), link.rs, dynamic.rs}` (§3).
- **main.rs `main` (202)** → one `fn` per command (`compile`, `run`, `tool`, `repl`, `eval`) and a command table.

## 3. Duplication

| what | where | shared helper |
|------|-------|---------------|
| function emission boilerplate (type, `functions.function`, `Function::new`, `End`, `code.function`, `register_func`, export) | 8× constructors.rs macro arms, 6× mod.rs (446, 926, 973, 999, 1072, 1151), 4× list_ops.rs (62, 92, 178, 267) | existing `runtime_function` (big_int.rs:585) → `wasm_emitter/function_builder.rs`, plus `exported_function` variant |
| `emit_list(func, &[I…])` | big_int.rs:681, used from other files via `Self::` | same file as above |
| `KIND_MASK = 0xFF` | web.rs, text_unicode, equality, library_ops, text_builtins, list_ops | `type_kinds::KIND_MASK` (Kind encoding is defined there) |
| `KIND_SHIFT = 8`, `SQUARE_BRACKET_INFO = 1` | text_unicode, library_ops, list_ops | `type_kinds::{KIND_SHIFT, Bracket::info()}` |
| `BYTE`/`WORD`/`MEMORY`/`IOVEC: MemArg` | text_unicode, library_ops, text_builtins, list_ops, exact, equality, wasi_emitter + 14 inline literals | `wasm_emitter/layout.rs::{BYTE, WORD}` |
| UTF-8 lead-byte tests (0xC0/0xE0/0xF0) | list_ops.rs 345–360, text_unicode.rs 27–96 | `text_unicode::emit_sequence_length` |
| wasi iovec store sequence (3 copies of 20 lines) | wasi_emitter.rs 42, 149, 185 | `emit_iovec(ptr, len)` |
| instantiate + call `main` + trap detail | wasm_reader.rs `read_bytes_gc`, `read_bytes_with_host`, `read_bytes_with_wasi`, `read_bytes_with_ffi`, `run_wasm_gc_object`; host.rs `run_wasm_simple`; wasm_emitter `run_raw_struct`; run/wasmtime_runner `run_wasm`, `run_wat` | `runtime::run_main<S>(bytes, state: S, link: impl FnOnce(&mut Linker<S>)) -> Result<Node>` |
| HTTP GET | `util::fetch` (unwrap → panic), `extensions::utils::download` ("" on error), `download_within`, `host::fetch` | one `net::get(url, timeout) -> Result<String, String>` |
| C declaration parser | `ffi::extract_function_signature` (106 lines) + `ffi::parse_header_file`; `ffi_parser::parse_declaration` (116) + `ffi_parser::parse_header_file` | keep ffi_parser's, delete ffi.rs's |
| C type → wasm type | `ffi::map_c_type_to_valtype`, `c_type_to_param_type`, `c_type_to_ret_type`, `c_type_to_wasm_valtype`, `ffi_parser::parse_c_type` | one `c_type_kind(&str) -> Kind` + `Kind::valtype()` |
| 20 hand-written libm/libc linker closures | `ffi::link_ffi_functions` (298 lines) | `const LIBM: &[(&str, F64Op)]` + `link_f64_unary/binary`; check whether the generic `link_dynamic_library` already covers them and drop the table |
| `ordering` → comparison op match | real.rs:583, time.rs:244 | `operators::Op::holds_for(Ordering)` |
| FFI return kind lookup | analyzer.rs 178 and 213 (same 8 lines) | `ffi_result_kind(name)` |
| `Assign/Define` target match | analyzer.rs:2396, library_words.rs:199 | `assignment_parts(node)` |
| node arithmetic prologue (Meta unwrap) | node.rs 1993, 2073, 2155, 2233 | `binary_number_op(self, other, op)` |
| user-function discovery per pass | `extract_user_functions(&mut Context::new(), &node)` in 11 files, 12 calls | compute once in `pipeline` and pass a `&Program` to passes that need it |
| tree recursion per pass | `Lowering::expand` in switch, library_words, phrase_words, lambdas, min_max; 41 `Meta { node, data } =>` re-wrap arms | `Node::map_bottom_up(&mut impl FnMut(Node) -> Node)` (node.rs `map_node` is an unused draft of it) |
| user-function call emitters | `emit_user_function_call`, `_float`, `_numeric`, `_inner` (mod.rs 461–552) | one call with a result-kind parameter |
| `while` loop emitters | `emit_while_loop`, `emit_while_loop_value`, `emit_while_loop_impl`; ternary/if `_numeric` twins (1972–2108) | one function with `wrap_result` / result-kind parameters (pattern already half there) |

## 4. Dead or outdated code

Detected with `cargo check --lib --bins` after removing `#![allow(dead_code, unused_imports)]` from lib.rs, main.rs,
node.rs and `#![allow(unused)]` from smarty.rs (lib: 88 warnings, 49 unused imports): the crate-wide allow hides all of
it. Proposal: drop the crate-wide allows once the items below are gone, so new dead code shows up.

Whole files (never referenced, or only by a test of themselves):
- `src/compiler/` — `wasm_reader.rs` declared, used nowhere; `parity_wasm_reader.rs` not even declared (dead file); the
  only user of the `parity-wasm` crate.
- `src/run/wasmer_runner.rs` (9 lines, all commented), `src/run/wasmedge_runner.rs` (31 lines, all comments).
- `src/ast.rs` (103): 7 structs never constructed, `walk` unused.
- `src/wasm_emitter/node_emitter.rs` (100): `EmitContext`, `NodeEmitter` never used.
- `src/extensions/_mod.rs` (8): not a module (extensions.rs is).
- `src/smarty.rs` (221): its own header says "OBSOLETE"; used only by `tests/numbers/test_angle.rs` (decision: delete with test?).
- `src/bin/test_op.rs` (58): a debugging printer, built as a second binary.
- `tests/test_asts.rs`: tests the `syn` crate, the only reason for the heavy `syn` v3 "full" dev-dependency.
- `tests/probes/*.rs` (2 files): not in tests/main.rs, never compiled.
- `src/wasm_optimizer.rs` is `#![cfg(feature = "optimizer")]`; check whether anyone builds with `optimizer`.

Unused dependencies: `parity-wasm`, `wasm-ast` (one unused `use wasm_ast::instruction`, comment "shitty vaporware"),
`regex` (one unused import in node.rs); `wat` is used only by tests → `[dev-dependencies]`; `once_cell` (2 uses) →
`std::sync::LazyLock`. Cargo.toml also carries ~25 commented-out dependency lines.

Unused items (selection; 42 `pub` items referenced nowhere, 20 used only by tests):
`function.rs` (`with_modifier`, `add_return`, `param_valtypes`, `return_valtypes`, `find_variant`, `get_by_index`,
`code_functions`), `node.rs` (`get_meta_data`, `to_json_compact`, `from_json`, `from_char`, `text_node`,
`type_definition`, `map_node`), `gc_traits.rs` (`from_struct`, `get_int`, `get_str`, `clone_store`, `clone_instance`,
`get_nested`), `analyzer::Scope::{define_param, define_type}`, `operators::{is_suffix, is_right_assoc}`,
`ffi_parser::{get_signature, get_library_signatures}`, `type_kinds::{is_pointer, from_node, set_wasm_type_idx}`,
`wasm_emitter::{register_import, ArithmeticWrap::None}`, `text_unicode::KIND_MASK`, `ffi::strlen`,
`run/wasmtime_runner::test_func`; in extensions: traits `Indexed`, `Filter`, `FromRange`, `StringVecExtensions`,
`Adds`, `VecExtensions`, `ArrayExtensions`, `CharExtensions`, `IntegerExtensions`, `PartialEqStr/Char/Num`,
`FileExtensions`, enum `List`, struct `WasmString`, fns `print`, `prints`, `assert_throws`, `todow`, `tee`,
`print_list`, `in_ranges`, `is_control`, `grapheme_*`, `download`, `write_wasm`, const `NO_NETWORK`;
smarty.rs 5 unused variables; tests/test_utils.rs: the whole module cache (11 warnings).

Two GC reading APIs: `GcObject` (lib.rs calls it "Legacy … for backward compatibility", 74 src / 28 test uses) and the
`gc_traits` traits (`GcStructWrapper`, `GcReadable`: 0 test uses). Decide which one stays.

Commented-out code: 279 lines, 30 blocks of ≥ 3 lines; biggest in extensions/lists.rs (59), extensions/strings.rs (39),
ast.rs (34), node.rs (32), extensions/numbers.rs (14), extensions.rs (13), run/wasmedge_runner.rs (13).

Outdated docs: AGENTS.md describes `src/emitter.rs`, `src/wasm_gc_emitter.rs`, `docs/wasm-gc-reading-guide.md`,
`tests/{node,wasp_parser,wasm_gc_emitter,wasm_reader}_test.rs`, `cargo run --example …` (no examples/), "vendored
dependencies (see vendor/)" (no vendor/), and wasmer/wasmedge backends. notes/probes_layout.md still says builds go to
`/opt/cargo/warp-<topic>` (contradicts notes/build_speed.md, shared target). notes/build_speed.md §5 mentions wasmer and
wasmedge as heavy deps; they are not dependencies any more. `main.rs` has `todo!("linking files needs compilation with
WABT_MERGE")` behind a C++ flag.

## 5. Rule violations

**Panics where decision #1 wants error values** (notes/panic_sweep.md covered emitter/analyzer/parser; these remain):
- node.rs: `Add/Sub/Mul/Div for Node` panic `Cannot add {:?} and {:?}` (2010, 2092, 2170, 2249); `IndexMut` panics on
  missing keys/out of bounds (630–667); `remove` (120); `todo!` in `class`, `typ`, `add` (78, 82, 194).
- wasp_parser.rs:2902 `Invalid bracket`; function.rs:446 `FunctionRegistry::get` panics; wasm_emitter/mod.rs:303
  `func_index` (kept on purpose, see panic_sweep), :3170 `finish` (test API).
- wisp_parser.rs: 13 `panic!("expected …")` + 37 unwrap/expect.
- extensions/numbers.rs: `panic!("unsupported types")` ×4, `not an integer`, `unimplemented!()` for Complex.
- util.rs `fetch`: two `unwrap()` on network I/O.
- main.rs: two `todo!` reachable from the CLI (`link`).
Totals in src: 35 `panic!`, 61 `.unwrap()`, 76 `.expect(`, 16 `unreachable!`, 6 `todo!`, 2 `unimplemented!`.

**Magic numbers** (517 literals outside `const`, ignoring 0–4, 8, 10): time/calendar.rs 86 (days, months, 60),
extensions/strings.rs 67 (Unicode ranges 0x0670 …, should be a named `const` table), operators.rs 53 (precedence
numbers inline in a match: make it a `const PRECEDENCE: &[(Op, u8)]` table at the top), text_unicode.rs 51 and
list_ops.rs 37 (UTF-8 masks; text_unicode already names some), big_int.rs 43 (local slots), smarty.rs 38.
`LocalGet(5..19)` with bare numbers 16×: name the scratch locals (`let carry = 5;` or a `Locals` enum).

**Trivial comments**: 178 comments of the form "// Get x", "// Create y", "// Emit z" restating the next line
(wasm_emitter/mod.rs 55, wasp_parser.rs 25, ffi.rs 14, list_emitter.rs 12, gc_traits.rs 10). Also the `// ====`
banner comments in ffi.rs that a module split replaces.

**Names**: `Node::laste` → `last`, `Node::typ` → `type_name`, `todow` → delete, `tee` → delete, `Dada` →
`DataValue`, `peq!` → `parses_to!`, `s!`/`strings!`/`Strings!` (two macros differing in case) → `texts!`,
`util::fetch(p0)` → `net::get(url)`, `smarty32`/`float_data28` → delete, `wis!` → `wisp!`, `extensions::_mod.rs`,
`meta.rs` → `node/metadata.rs`. Unprofessional comments: Cargo.toml line 2 and extensions.rs "fucking …",
Cargo.toml "shitty vaporware".

**Configuration at the top**: mostly followed in newer files (big_int, text_unicode, library_ops, main.rs); violated
by constants defined mid-file (list_ops.rs 1014–1016 `SQUARE_BRACKET_INFO`, `KIND_MASK`).

## 6. Tests

- One integration crate (tests/main.rs, 225 modules): good, done 2026-10-03.
- Flat directory of 225 files. Proposal (needs OK, tests are append-only): group by topic as module directories, e.g.
  `tests/syntax/` (parser, position, comments, glyphs, superscripts, …), `tests/types/`, `tests/text/`, `tests/lists/`,
  `tests/control/` (loops, switch, try, if), `tests/functions/`, `tests/units_time/`, `tests/modules/` (use, packages,
  folder scope), `tests/welcoming/` (15 `test_welcoming_*`), `tests/algo/` (6 `test_algo_*`), `tests/runtime/` (wasm,
  wasi, host, ffi, gc_struct, reader). A pure `git mv` + `mod` lines, no test body changes.
- Not tests in tests/: `probe_*.rs` ×7 (probe_footguns.rs 1 088 lines runs in the suite), `tests/probes/*.rs` (dead),
  `tests/notes/` (session notes → notes/OLD), `tests/test_utils.rs` (dead cache, 11 warnings), `tests/test_todo.rs` /
  `test_node_todo.rs` (todo lists as tests).
- Oversized: test_wasm.rs 1 781 (a port of the C++ test_wasm.cpp, 35 feature `cfg` branches, 65 assertion lines
  repeated inside the file), probe_footguns.rs 1 088, test_ffi.rs 621, test_types.rs 602, test_xml.rs 537.
- Duplicated assertions: 250 identical `is!/eq!/assert_eq!` lines appear more than once (e.g. `is!("42", 42)` ×6);
  between files mostly test_wasm.rs ↔ test_angle.rs / test_global_modifiers.rs, probe_type.rs ↔ test_todo.rs,
  probe_footguns.rs ↔ test_size_count.rs. When a probe's case is promoted to a test, delete it from the probe.
- Overlapping names to merge when grouped: test_wasm_emitter / test_emitter (optimizer-gated) / test_wasm;
  test_operators / test_node_operators; test_switch_* ×3; test_units_* ×3; test_text_* ×7; test_web / test_web_playground;
  test_wit / test_wit_types; test_min_max / test_min_max_lists.
- Unit tests inside src: wisp_parser (18), function (8), ffi (7), ffi_parser (7), normalize (3), strings (2): fine,
  but wisp_parser has no integration test at all and nothing outside lib.rs uses it (decision: keep the Wisp format?).
- The test macros (`is!`, `eq!`, `skip!`, `check!`, `put!`) are `#[macro_export]` in src/extensions.rs, so the library
  ships test helpers and `extensions` depends on the whole compiler. Move them to tests/common/mod.rs.
- probes/: 3.2 MB, mostly the fixer prompt templates and per-topic snippet dirs from 2026-10-02 (algo 2.2 MB, ask,
  break, elements, listexpr, listparams, maps, parse, print, slices, globals, …). Once those topics are merged, keep the
  `.wasp` snippets that became tests' inputs and delete the rest; `stage_own_hunks.py`, `unignore.py`,
  `commit_unignored.py`, `bisect_step.sh` are one-off agent tools (move to ~/dev/bin if reused).

## 7. Build hygiene

- **Biggest hot spot: main.rs compiles the whole crate a second time** (finding #1). Fixing it roughly halves warp's own
  compile for `cargo build` / `cargo test` (the bin and the bin's test harness each recompile 41 k lines).
- `src/bin/test_op.rs` is a second binary: delete or move to probes/.
- Stale features (finding #2): `APPLE EMSCRIPTEN GRAFIX INCLUDE_MERGER LINUX MICRO MULTI_VALUE MY_WASM RUNTIME_ONLY SDL std
  TRACE WABT_MERGE WAMR WASI WASM WASMEDGE WASMTIME WEBAPP release wasm` are empty C++ leftovers. With `--all-features`
  (the agents' documented command) `RUNTIME_ONLY`, `WASM`, `WASMEDGE`, `release`, `wasm` switch test branches and main.rs
  behaviour, so "--all-features" and plain `cargo test` test different programs.
- Dependencies to drop: `parity-wasm`, `wasm-ast`, `regex`, dev `syn` (syn v3 "full" + "extra-traits" is one of the
  slower crates); `wat` → dev-dependency. Cargo.lock has 245 packages.
- No `vendor/` exists although Cargo.toml and AGENTS.md describe vendored offline builds; `--offline` works from the
  registry cache. Fix the docs, or vendor for real (decision).
- DONE 2026-10-04: `crate-type = ["cdylib", "rlib"]` is now `["rlib"]`; the playground build asks for the cdylib itself
  (`cargo rustc --crate-type cdylib`, web/playground/build.sh), so copies no longer share one unhashed `libwarp.rlib`.
- `[package.metadata.clippy] warn = false` does nothing (not a clippy key); remove.
- `#![allow(dead_code, unused_imports)]` crate-wide (lib.rs, main.rs, node.rs) + `#![allow(unused)]` (smarty.rs) hide 88
  lib warnings; remove after §4.
- Test-binary count is already 1 (good). For editing feedback, `cargo check --tests` (no codegen) is documented in
  build_speed.md.

## Safe mechanical refactors vs decisions

Mechanical (behaviour-preserving, fastmod/semgrep or `git mv` + re-exports; run the full suite once after each):
#1 main.rs uses the lib · #3 delete dead files/deps (except smarty/test_asts) · #6 function_builder · #7 shared constants ·
#16 instruction style · #18 commented-out code and trivial comments · #20 renames · the file splits in §2 and the
`lowering/`, `analysis/`, `runtime/` moves in §1 (with `pub use` shims in lib.rs so tests keep compiling) ·
`wat` to dev-dependencies · remove crate-wide allows (after the deletions).

Needs a design decision: #2 removing features also edits tests · #4/#5 pipeline as a `PASSES` table (error handling
contract `fn(Node) -> Result<Node, Node>`) · #8 runtime API · #9 hard-coded libm table vs dynamic FFI only · #12 shared
`Program` context for passes · #13 panic → error contract for `Node` arithmetic/indexing (operators on `Node` return
`Node::Error`?) · #14 AGENTS.md/CLAUDE.md · #15 test regrouping · GcObject vs gc_traits · the Wisp format.

## Open questions for the user

1. May the stale C++ feature flags go, including their `#[cfg]` branches in tests/test_wasm.rs and tests/web/test_web.rs
   (keeping the branch that runs today)?
2. Delete `smarty.rs` with its asserts in tests/numbers/test_angle.rs, and `tests/test_asts.rs` (a test of `syn`)?
3. Regroup tests/ into topic subdirectories (pure moves), and move `probe_*.rs` out of the suite?
4. Keep the Wisp format (wisp_parser.rs, 946 lines, no integration test, no user)?
5. Which GC reading API stays: `GcObject` or the `gc_traits` wrappers?
6. Keep the 20 hand-written libm/libc bindings in ffi.rs, or rely on the generic header-driven FFI only?
7. `Node` operators (`+ - * /`, `IndexMut`): return `Node::Error` instead of panicking?
8. CLAUDE.md: replace with a symlink to AGENTS.md, and may AGENTS.md's architecture section be rewritten from §1?
9. Root clutter (`goo`, `conversation.md`, `node.wat`, `test.wasp`, dangling `wasp`/`warp` links, `build_debug.sh`,
   `add_ignore_to_failing_tests.sh`, `nextest.sh`): delete or move to notes/OLD?
10. DONE (user: "can we gate it"): no split needed, the cdylib is gated to web/playground/build.sh.
