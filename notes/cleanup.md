# Cleanup: rules, tools, verdicts, leftovers
Roadmap card: cleanup (board topic; sub-cards are the areas and their leftovers, notes/roadmaps.md).

## Rules
- Behaviour-preserving only: shared helpers instead of duplication, dead code and stale comments out, flatter control
  flow, meaningful names, no test edits. One area per session, small themed branches through the Integrator.
- The measure is how hard the code is to follow and how hard it is to write its warp twin (self-hosting).
- Watch for closed lists: a hard-coded list of words standing in for a mechanism breaks on every word it misses
  (builtin_type_kind once knew int/float/number/text but not real). Derive such a list from its source of truth (the
  type registry, the library's exports, the operator table) instead of adding the missing word. Example: the operand
  words (`1 == first xs`) are every one-argument library word by `arity` that is no type word; copy, draining and
  result-kind lists name one spelling each and match through library_words::canonical_spelling. Still listed by
  hand: broadcasting SCALAR_LIBRARY_WORDS, checks ORDER_WORDS.
- src/extensions.rs is hard-linked with ~/dev/script/rust/extensions.rs (src/extensions.rs.hardlink is the
  hardlink-guard's marker): editing it changes that file too. extensions/lists.rs is the user's style; leave it.

## Tools (probes/cleanup/)
- `dead_scan.py`: never-used pub functions across the default, all-features and wasm32 builds (keeps glob re-exports
  and those naming items used outside). Kept on purpose: function.rs builders, set_hint_file, create_host_linker
  (unit tests call them), Node last_item and extensions.rs public_function (the user's comments), get_meta,
  run_wasm_gc_object (named by commented-out tests), the gc_traits reading API (the documented rasm pattern).
  By hand: count every `fn` name across src, tests, crates and web (`grep -rwo`); a name seen only at its definition
  has no caller. `#[test]` functions in src and trait methods (`Reencode`'s `parse_*_section`, winit's `resumed`)
  are not dead.
- `split_module.py`: moves 1-based inclusive line ranges into `x/child.rs`, keeping paths and visibility (`use
  super::*` in the child, a re-export in the parent as visible as the child's most visible item; private items become
  pub(super)). Start each range at the doc comment of its first item, end it at the closing brace of its last item.
  Fix by hand after `cargo check`: an unused re-export, a `pub` item that exists only under a cfg. No source file is
  over 1500 lines.
- `duplicate_windows.py`: repeated windows of normalised lines. Exact copies: `npx jscpd --min-lines 4 --min-tokens 40
  --format rust` (run inside a copy: it skips ignored folders).
- `same_wasm.sh <before> <after> $(find probes samples -name '*.warp')`: emitter changes must emit the same bytes
  (`same`, `unstable`, `DIFFERENT`, `no module`). Build before and after in the same worktree and copy the binary out
  in the same command; check `warp --version` (another session's build can replace it). `rm -f` the destination
  before copying: macOS kills a binary `cp` overwrote in place (exit 137). Builds are reproducible
  (tests/wasm/test_reproducible_builds.rs).
- `merge_instruction_runs.py`, `emit_rows.py [--write|--check] files…`: emitter instruction arrays in rows, one logical
  step per row like WAT, kept by `#[rustfmt::skip]` on the statement. Write new emitter code that way or run the
  script before sending a branch.
- `fold_*.py`, `round_call_sites.py`, `fold_rewrite_tails.py`: re-runnable rewrites for lowering passes.
- `stacked_docs.py`: doc comments that drifted off their function (many false positives in lowering).
- `dead_note_paths.py --fix`: rewrites stale paths in notes along git's renames, leaving lines that say "was/gone".
- `feature_usage.py . <wiki>`: how often each lowering feature is used in samples, tour and wiki.
- `coverage_ranking.py <json> <lcov> <checkout>`: ranks never-run code after
  `WARP_TEST_TIME_LIMIT=2400 tests/queue.sh cargo llvm-cov --offline --no-report --tests --no-fail-fast` (the
  executable-building tests fail under instrumentation). Most never-run code is I/O no test drives (CLI, paint
  window, deploy, sound, downloads), not dead.
- Bulk rewrites: small Python scripts (balanced-bracket parser) + `cargo check --message-format short`. A local fn or
  variable named `call` / `key` shadows the nodes helpers (E0255 / E0618): give it a telling name.

## Where shared code lives
- lowering/nodes.rs: `call`, `key`, `assign`, `block`, `if_then(_else)`, `statement_list`, `is_type_word`,
  `parameter_name`, `is_call_head`, `spaced_statement`, `children_rewritten`, `word_headed`, `round_call`,
  `any_part`, `word_applied`, `head_symbol`, `phrase_words` (undo the parser's groups of a phrase),
  `filled(code, [("X", x)])` (code with placeholders), `Counter` (fresh names), `for_in_variable`,
  `function_head(statement) -> FunctionHead` (one view of every definition form).
- Node: `symbol_name`, `is_symbol(WORD)`, `as_items`, `parts`, `single_or_list`, `sequence_statements`, `key(name, value)`.
  `Node::map_children` also enters class bodies; `children_rewritten` does not.
- src/words.rs: word constants used by 3+ passes.
- `foreign::node_of`: the one JSON-to-Node conversion. ffi/header.rs: the one C header parser.
- wasm_emitter/function_builder.rs: `runtime_function`, `exported_function`, `emit_list`, `emit_while`, `emit_cell_walk`
  and friends. runtime_dependencies.rs: each runtime module declares `dependencies()`; a new runtime function is one
  line in its module's table.
- `extensions::utils::numbered_temporary_file` for temporary output files (WAVs, PNGs).

## Global state
- Compile state is two thread-locals: src/compiler/compile_context.rs `CompileContext` (mode flags, grants, component
  functions, serving port, source, hint/warning modes, files, units, kernels, fresh-name counter, …; use
  `compile_context::scoped(field, value, run)`, `read`, `change`) and diagnostic.rs `Diagnostics` (warnings, errors,
  assumptions, hints; taken once per compile).
- The other thread-locals are run-time host state (one per running instance), caches, user acknowledgements
  (ACKNOWLEDGED_TOPICS, NOTES_SHOWN) and READING_PRELUDE (re-entrancy guard of the prelude's OnceLock).
  Process-wide `static`s are caches of outside facts, locks serializing outside tools, terminal output settings and
  devices: none is compile state.
- A fresh context per compile was rejected: inner compiles (modules, the renderer) need the outer mode, grants and
  source; only float_kernels accumulates, keyed by the kernel's own text.

## Checked and not worth it
- Merging lowering passes (each is a small, separately testable rewrite; passes that run twice say why in pipeline.rs).
- A generic loop helper for the ~90 hand-written emitter Block/Loop loops (costs as many lines as it saves).
- One skeleton for the real.rs and time.rs constant evaluators; wrapping one GcObject in the other (they read
  different things).
- Library functions over closures instead of lambdas.rs' iteration templates (~3× slower, untyped results).
- One HTML escaper (markup.rs escapes `<pre>` without `>`, site.rs with it: changes the error page).
- The statements_of / is_statement_list / is_return / parameter_name copies differ in bracket and separator rules on
  purpose; sharing one changes which forms match.

## Leftovers
- Signal family: listeners are read by lowering/web/listeners.rs and host events registered by `signal_listen`. Left:
  making every watched variable a cell instead of variable_signals' static write checks (~−700 lines, slower watched
  loop counters: needs a decision first).
- warp_parser/mod.rs holds ~200 keyword constants: each belongs in its one user file.
- `matches!(x.drop_meta(), Node::List(_, Bracket::Curly, _))` is hand-written ~67× while nodes::is_block exists;
  `Node::is_block` is its home. USE_WORD is re-declared in 5 lowering files beside modules::USE_KEYWORD.
- warp_parser::mentions vs Node::mentions differ only by template holes: name them precisely.
- emit_kind_globals' name table (derive from Kind::ALL); one shared hex_digest (module_cache, program_cache,
  cargo_crates); list_emitter KNOWN_WORDS as a mechanism; `declarations::word` (~40 callers).
- Lean export skips `def f(x) := body`, untyped `def f(x): body` and `def (f x) {…}`; three is_call_head functions
  mean different things (nodes, traits, variable_signals).
- `round_to` is in lib/prelude.warp and lib/math.warp (dropping math's removes it from `dir(math)`).
- src/numbers/units.rs, units/static_units*, src/graphics/gpu_canvas.rs, paint_window.rs, src/numbers/real.rs got no
  final pass yet.
