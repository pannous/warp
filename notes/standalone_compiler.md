# The standalone compiler: one warp program, source in, wasm out

User decision standalone-compiler (2026-10-20): "we are no longer interested in Rust. Everything we try directly with
warp. No more conversion to and fro. Of course, we need the Rust compiler to compile our compiler. But then the
compiler should be completely independent."

Today (src/compiler/self_hosted.rs, the pure lane): three stage programs (passes.warp `stage`, analyzer.warp
`emitter_kinds`, emitter.warp `typed_wasm_module`), each its own module in a worker thread; Rust writes each request
as JSON into a file, the stage reads it with `json(read(args#1))`, rebuilds nodes with walk.warp `node_of`, answers
with `view_of`, Rust turns the view JSON back into a Node (node_views.rs) and serializes it for the next stage. Rust
also makes two of the emitter's inputs: the runtime words (runtime.warp lowered by the *Rust* pipeline, as a view) and
the host words (src/host.rs HOST_WORD_SIGNATURES as JSON).

## The design

### Entry point: lib/compiler/compiler.warp
An ordinary warp program, nothing compiler-special about how it runs:

```warp
use "passes.warp"
use "analyzer.warp"
use "emitter.warp"
use "host_words.warp"
use file

# the wasm module of the source, as bytes; a form outside the core throws "outside the core: …"
compile(source: text, folder: text): list := {
	program = lowered_program(source, folder)
	module = typed_wasm_module(program, runtime_words, host_words, emitter_kinds(program))
	if module is text { throw module }
	module
}

# the runtime words, lowered by the same passes as a program, once per run of the compiler
runtime_words = lowered_program(read(library_folder + "/compiler/runtime.warp"), library_folder)

# warp compiler.warp <source file> <wasm file>
write_bytes(args#2, compile(read(args#1), folder_of(args#1)))
```

The stages hand each other plain warp values, the ones they already compute inside:
| stage | in | out |
|---|---|---|
| passes.warp `lowered_program` (parser.warp, uses.warp + prelude words, every pure pass) | source text, folder | the program, a Node |
| analyzer.warp `emitter_kinds` | the Node | a map {functions, variables, elements, value} |
| emitter.warp `typed_wasm_module` | Node, runtime Node, host map, kinds map | list of bytes, or the refusal text |

Gone from the pure lane: every JSON text, `view_of`/`node_of` between stages, `bridged`/handled views,
self_hosted.rs `pure_bytes`, `runtime_words`, `runtime_source`, `host_words`, `view_of_json`, the Stage workers and
input files for this lane. `node_of`/`view_of` stay only for the twin lanes (front, emit, WARP_SELF_ANALYSIS), which
compare with Rust stages; they retire with those lanes (decision self-hosting: twins go once both sides are warp).

### What Rust still does (the edges)
1. Bootstrap: compile lib/compiler/compiler.warp once with the Rust pipeline, kept in the program cache under the
   key self_hosted.rs `stage_key` already makes (binary, lib/ contents, WARP_ settings). A cold compile takes minutes,
   a warm start a fraction of a second, as the stage programs do today.
2. Input: the program file's path is `args#1`. Inline code (`warp "1 + 2"`, is! tests) has no file: Rust writes it to
   an input file in the cache folder (as the stage workers do now) and passes its folder as the program's folder
   (`args#3`, else the source file's folder).
3. Output: Rust reads back the wasm file `args#2` names: bytes, no conversion. A refusal is the compiler program's
   error (its message "outside the core: …") and the program's error, recorded in WARP_EMIT_REFUSALS: no Rust
   compile behind the stages (supervisor, 2026-10-20: the silent fallback hid gaps).
4. Run: the produced module through `run_module` with WASI and the host imports it names (`imports_from`), plus the
   fuel mark as today.

The same file runs from the command line with no switch at all: `warp lib/compiler/compiler.warp app.warp app.wasm`,
then `warp app.wasm`. That is the independence the decision asks for; the Rust lane is only a convenience around it.

### WARP_SELF_HOSTED=pure (alias pure-only)
pure and pure-only become one lane with no fallback; `pure-only` stays as an alias with a note.
pipeline::eval_source → self_hosted::pure_module(code) → (new, small) `standalone::compiled(code)`: kept compiler
module, input file, run it, read the wasm file. `pure_bytes` and its three stage round trips go. The ladder
(probes/self_hosting/refusals.sh --pure) must count at least as many right programs as before the switch.

## What has to change first (found by probes/self_hosting/one_compiler.warp, runtime_lowered.warp)
1. Name clashes across stages: today each stage is its own program, so their helpers never met. In one program they
   are errors ("is_block(any) is declared in both shapes.warp and analyzer.warp"). Cross-stage pairs:
   analyzer×emitter: declared_kind, held_kind, new_scope, is_block (also shapes.warp); analyzer×walk: kind_of;
   analyzer×scope: parameters_of; analyzer×serializer: is_empty; analyzer×multiple_assignments: statements;
   emitter×parser: expression, global_declaration, statements_of; emitter×node_matches: chosen;
   emitter×definitions: definition; emitter×for_loops: is_entry; emitter×clauses: joined.
   Decided (supervisor, 2026-10-20): the general rule wiki/use.md already promises between a program and a module
   ("the module's own code keeps calling its own play"), file and file alike: a used file's own code calls its own
   word; the error only when a file that sees both calls the word itself. No renames (card file-own-words).
2. Runtime words lowered by warp: runtime.warp through `lowered_program` (folder lib/, so uses.warp splices its
   `use case_tables`; since card runtime-split its parts are `use "runtime/<part>.warp"` beside it, so the folder is
   lib/compiler and uses.warp must also find a std module name in lib/: card uses-std-modules) runs (cold 2 min), with warnings that `Round`, `Add`, `Define` name nothing (enum variants
   written bare in runtime.warp: warp-compiler-enums). Whether emitter.warp emits the same from it as from Rust's
   lowering is the next probe. Cost: lowered once per compiler run; if that is slow, the general fix is lowering
   only the runtime words a program reaches (as the prelude words are, card prelude-for-pure), never a cached JSON.
3. Host words as warp data: lib/compiler/host_words.warp, `{draw_shape: [[int, float, float, float, float, int],
   []], …}`, the one source of truth; build.rs generates src/host.rs HOST_WORD_SIGNATURES from it (the host is an
   edge, its table is not).
4. `library_folder`, `folder_of`: words the compiler needs from os/file (the folder of lib/, a path's folder);
   library_folder from WARP_LIB or the compiler file's own folder.

## Steps
1. This note (approved by warp-supervisor 2026-10-20).
2. Clashes: rule or renames (1 above).
3. compiler.warp + host_words.warp; probe it from the command line on the pure ladder.
4. self_hosted.rs: pure/pure-only run the compiler module; pure_bytes, JSON setups go; ladder counts ≥ before.
5. build.rs: HOST_WORD_SIGNATURES from host_words.warp.

## Progress (2026-10-20, branch standalone)
- Step 2 done by renames (user: rename for now): 14 emitter/analyzer helpers, and emitter.warp `length` → `length_code`
  (a text's `.length` in parser.warp called it). The general rule stays card file-own-words.
- Composing the stages found six Rust scope bugs of one kind, all fixed with tests in
  tests/scope/test_bound_names_shadow_functions.rs (card local-variable-named): a parameter, local or loop variable
  named like a function elsewhere was taken for that function in list heads, type inference, a returned local
  (closure analysis), lambda captures, prefix calls (`call#1`) and nested lambdas. Plus: a text's member looked up by
  name (`line.length` beside a map with a key length) was typed a text.
- Step 3 written: lib/compiler/compiler.warp and host_words.warp. Not yet through: the next probe needs the scope fixes
  in the shared build. Probe: `cd scratch/sc && WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp ../../lib/compiler/compiler.warp app.warp app.wasm`.
- Blocked on compile time (card analyzer-settle): compiler.warp does not finish compiling (>30 min CPU); even
  `use passes+emitter; print(1)` takes >45 min, while each stage alone compiles in ~30 s. A 23-min sample of the
  whole warp-debug run (probes/sample_inclusive.py) put 98% of it in one caller: run_time_blocks::data_holders ran
  the whole kind inference (flow::settle, its main-program walk restarting thousands of times) only to learn the
  program's function names. Branch analyzer-settle reads them with function_definitions (no inference), law.rs's
  function_definition too. Still open: why one inference of that pass state takes minutes.

## Coordination
- warp-self-analyzer (pure lane, branch prelude-for-pure): compiler.warp only composes their stages; their passes,
  prelude words and emitter growth flow in unchanged. Step 4 touches self_hosted.rs `pure_module`, which they also
  edit: merge after prelude-for-pure lands.
- warp-compiler-enums: the bare `Round`/`Add`/`Define` symbols in the warp-lowered runtime; walk.warp's `Op`,
  `Bracket`, `Separator` enums are no longer crossed through text views in this lane.
