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

### `warp pure <program.warp> [arguments…]` (card warp-pure, user 2026-10-20: "put everything in the binary")
src/compiler/standalone.rs: compiler.warp's module, the build file compiler.wasm (decision compiler-build: a build
output, no cache), runs as `compiler.warp <source> <wasm> <folder> <lib>`; the module it writes runs with the program's
arguments. The Rust pipeline compiles compiler.warp, always (user 2026-10-21: "pure always is Rust's compiler.warp and
`warp self` always uses the self-hosted, self-compiled"). `warp self <program.warp>` runs the program through
compiler.warp compiled by the pure lane, itself, the build file compiler-self.wasm; a refusal of that compile is a loud
error, never a fallback to the Rust-compiled one. `warp pure` switches to the self-compiled compiler once more tests
pass with it (the count self vs Rust-compiled goes to the supervisor first). It is also the gate's
pure-chain check (`warp pure samples/hello/hello.warp` → "Hello 🌍2026").
Build files (decision compiler-build, card compiler-wasm): compiler.wasm and compiler-self.wasm lie next to the binary
for the lib/ they record (custom section `warp.build`: lib/'s path, its fingerprint, the binary's WARP_SOURCE from
build.rs); a run with any other lib/ (WARP_LIB, a worktree) uses <lib>/../data/. A missing file is built at its first
run; an existing one only by `warp rebuild [pure|self]`, never because lib/ changed: a run on one built from another
lib/ or warp build prints one line naming `warp rebuild`. main's binary refresh runs `warp rebuild pure` with the new
binary before installing it next to ~/dev/bin/warp. probes/speed/compiler_build.sh checks the whole cycle.
Cold start (card pure-cold-start, empty module cache): the pure lane's try at compiler.warp (passes stage compiled ~44
s, analysis ~6 s, emit ~21 s, then their runs) and Rust's compile of it (~90 s) ran one after the other: 166 s. Now
Rust compiles it in a thread while the pure lane tries, and the analysis and emit workers start with the passes one:
89 s real, 214 s user, 2.8 GB max RSS. The cache files' mtimes are the cheapest timeline of a cold run.
Warm start (card pure-warm-start): 2.9 s, of which ~97% was compiler.warp lowering runtime.warp again at every run
(WARP_PROFILE: 590 of 613 samples). compiler.warp now keeps them, `kept(name) ?? keep(name, value)` of use os (a
value in the module cache under stage_key: binary + lib/): warm hello 0.07 s, a new program ~0.1–0.2 s. The kept
Node crosses to Rust and back (postcard); 38 samples give the same results with and without it.
For compiler.warp to compile as one program the 11 cross-file clashes were renamed (user decision rename-clashes;
list on card file-own-words, to undo once that rule lands).

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
  function_definition too. That inference never ended: before declarations::lower a definition with a result type
  in its head (`f(x): list := …`) was walked as main code, and locals of one name in many stage functions (kept,
  found) turned each other between a list and a Node forever. Fixed on the same branch (function_definition_body
  reads such heads; a Node widening stays one). Lldb recipe for such hunts: data/settle/ in the standalone worktree.

## Coordination
- warp-self-analyzer (pure lane, branch prelude-for-pure): compiler.warp only composes their stages; their passes,
  prelude words and emitter growth flow in unchanged. Step 4 touches self_hosted.rs `pure_module`, which they also
  edit: merge after prelude-for-pure lands.
- warp-compiler-enums: the bare `Round`/`Add`/`Define` symbols in the warp-lowered runtime; walk.warp's `Op`,
  `Bracket`, `Separator` enums are no longer crossed through text views in this lane.

## warp.wasm under plain wasmtime (card standalone, reopened 2026-10-21, user: "build in the runtime itself.
## wasmtime warp.wasm should behave like the native binary")
Today `wasmtime run -W gc=y,function-references=y,exceptions=y --dir . compiler.wasm hello.warp h.wasm` stops at the
first host import. Imports found (2026-10-21):
- compiler.wasm (Rust-compiled): host.fetch, fetch_within, read, warn, run, std_io + wasi fd_write.
- compiler-self.wasm (self-compiled): all 69 host words + m.pow: emitter.warp imports a host word whenever the
  program `mentions` its name, and compiler.warp includes host_words.warp, whose map keys name every one.
What each is for in compiler.warp: std_io carries `use os` (args, env, kept, keep) and `use file` (write_bytes,
exists …); read is the prelude's file/URL read (fetch/fetch_within its URL half); warn the compile's warnings; run
probably a mention only.

Plan, one import at a time, each a WASI-backed runtime part in both emitters (Rust wasm_emitter for the bootstrap
compiler.wasm, emitter.warp for compiler-self.wasm):
1. Imports only for words called: a map key or text naming a host word imports nothing (emitter.warp `mentions`,
   and the Rust side's equivalent). Takes compiler-self.wasm from 69 host imports to the handful it calls.
2. `_start`: a WASI command export calling main, so `wasmtime warp.wasm …` needs no `--invoke`.
3. os.args, os.env: args_sizes_get/args_get, environ (emit_wasi_environment exists).
4. read / file.read_bytes / exists: path_open on the preopened dirs (fd_prestat_get), fd_read, fd_close; a URL under
   plain WASI is a loud error (no network in preview1).
5. file.write_bytes / write: path_open with O_CREAT|O_TRUNC, fd_write.
6. warn: fd_write to stderr (fd 2).
7. kept/keep: ø / the value under WASI (no module cache); a cache file later.
8. Outcome test: `wasmtime … warp.wasm hello.warp hello.wasm` writes a module that prints Hello.
Open question (warp-interviewer): running the compiled program inside the same wasmtime process needs either the
host (instantiate is no WASI call) or an evaluator in warp; default until answered: warp.wasm compiles to a .wasm and
prints how to run it.
