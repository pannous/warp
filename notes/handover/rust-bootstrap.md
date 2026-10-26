# rust-bootstrap: handover (Worker Bootstrap, 2026-10-26)

Goal (user 2026-10-26 12:58): the Rust compiler moves to its own `rust-bootstrap` branch; main is self-hosted.
PLAN CHANGE (user 2026-10-26 ~14:25, via the Integrator): Rust is NOT cut on main. The Rust-free result grows on the
long-lived branch `pure`: main minus the Rust compiler, the minimal Rust harness kept, `warp rebuild` starting from
seed/compiler-self.wasm (on main since 8f0d4049e). Step branches (rust-bootstrap-self, rust-bootstrap-parse, the
src/ cut …) merge into `pure`; the card is readied as `todo ready rust-bootstrap pure`. origin/main is merged into
`pure` regularly (git merge, never rebase, never force-push). When everything works, `pure` becomes main. The
fixpoint is reached (stage3 == stage4 with fixpoint fixes 1+2, /workspace/cow/fixpoint-progress.md).

## Steps and branches
1. `rust-bootstrap-seed`: seed/compiler-self.wasm (stage4q, 1572586 B, sha256 3197e7ff…dc4e), seed/README.md,
   scripts/refresh-seed.sh (seed → A → B (→ C), the seed replaced only at the fixpoint; `--check` replaces nothing).
   Warp/data only. .gitignore un-ignores `/seed/compiler-self.wasm`.
2. `rust-bootstrap-self` (on top of 1): src/compiler/standalone.rs builds both compilers without any Rust-compiled
   stage: `warp rebuild pure` = the seed compiles compiler.warp → compiler.wasm; `warp rebuild self` = compiler.wasm
   (when built from these compiler inputs, else the seed, with a note) compiles compiler.warp → compiler-self.wasm.
   `warp rebuild` = both, in that order: seed → A (compiler.wasm) → B (compiler-self.wasm). `warp pure`, `warp self`,
   `warp test` (pure_tests.rs, which runs compiler.wasm) therefore run self-built compilers only. WARP_SEED names
   another seed file. The seed is looked up beside lib/ (`<lib>/../seed/compiler-self.wasm`), else in the installed
   folder (`<bin>/seed/compiler-self.wasm`; installs must ship it).
3. Cut (Integrator): create branch `rust-bootstrap` from main at the commit where step 2 has landed; then a deletion
   branch removes the Rust compiler from main (list below).

## Survey of src/ (origin/main 46c441c440)
Where Rust still compiles warp code:
- `warp rebuild self` (before step 2): standalone::build(SelfCompiled) → pure_compiled → self_hosted::pure_bytes:
  the stage programs PASSES (passes.warp), ANALYSIS (analyzer.warp), EMIT (emitter.warp) are compiled by
  pipeline::compile (self_hosted::compiled_stages) and run per request. Replaced in step 2.
- `warp rebuild pure` (before step 2): pipeline::compile(compiler.warp). Replaced in step 2 (the seed builds it).
- Plain `warp <file>`, `warp run`, `warp eval`, `warp lower`, `warp parse`, the repl/console, `warp verify`,
  `warp test <file.warp>` (run_tests_of), serve/app/webview: main.rs eval() → pipeline (warp_parser + lowering +
  analyzer + wasm_emitter). Next to move: make them run `warp self` (standalone::run with SelfCompiled), keeping the
  CLI shape.
- WARP_SELF_HOSTED=front|emit|pure|pure-only lanes (self_hosted.rs): Rust compiles the stage programs. Obsolete once
  the self lane is the only one; delete with the Rust compiler.
- Deploy (host/host_parts.rs deployable), component builder (modules/component_builder.rs), std module lowering
  (modules/loader.rs, lowered_modules.rs, implicit_modules.rs, rewrites.rs, wasm_modules/program_uses.rs):
  pipeline / warp_parser. Need compiler.warp equivalents (build flags for page/worker/component) before deletion.
- Host words that parse data at run time: host/native.rs channel_next (warp_parser::parse of a message), json/
  formats readers. These need the data-notation reader, which stays (or moves to crates/) — it is the data format,
  not the compiler.
- pure_tests.rs (`warp test [folder]`) finds the cases with lowering::test_blocks (Rust parser) and runs each with
  `warp test --case` → standalone::run_code: the runs are self lane; the case discovery still uses Rust's parser.

Harness to keep (the minimal Rust host):
- run/ (wasmtime_runner, wasm_reader, module_cache, program_cache), crates/warp-runtime (engine, GC, profile, system
  values), host.rs + host/ (host words, sandbox, channels, database, paint), ffi/, gc_traits/ (reading GC results
  back), node/ (Node, serialization for printing results), errors/ (diagnostic printing, crash cards), modules/
  std_adapters.rs (args, env, kept values), modules/program_file.rs (library_folder, installed_folder), cli/,
  main.rs, compiler/standalone.rs, compiler/pure_tests.rs, graphics/, sound/, web/ (dev server, fetches, sockets),
  time/, numbers/ (as the host needs them), formats/ (data readers), the data-notation part of warp_parser.

Rust compiler to delete after the cut (only once the commands above run on the self lane):
- warp_parser/ (all but the data reader the host needs), lowering/ and the flat pass files re-exported in lib.rs,
  analyzer/, wasm_emitter/, pipeline.rs, normalize/, operators/ (if compile-only), law.rs/law/ (verify: needs a warp
  port), compiler/{self_hosted.rs stage lanes, analysis_memo, compile_context, compile_mode, compile_time, context,
  dead_functions, function*, local, meta, node_views, phase_times, run_time_blocks}, words.rs, type_kinds.rs (if the
  host's read-back doesn't need it), text/interpolation etc.
- Tests under tests/ that drive the Rust pipeline (cargo tests) go with it to the rust-bootstrap branch; warp tests
  (`warp test`) stay.

## Smoke results
- Step 1 (seed): seed compiled compiler.warp of 46c441c440 → stage A, 1603 s at load 30-70; A ran hello (40 s cold).
- Step 2 (rust-bootstrap-self at f8bbe974fc = main b65a545429 + seed + step 2), branch binary, Mac:
  `warp rebuild pure` (seed → compiler.wasm, 1577271 B) 1300 s, `warp pure` hello ✓ (1 s), 3*7 = 21 ✓;
  `warp rebuild self` (compiler.wasm → compiler-self.wasm, 1577053 B) 1047 s, `warp self` hello ✓, 21 ✓.
  Old Rust lane for comparison (box): `warp rebuild pure` 189 s.

## Timing (Mac, load ~9-14)
- seed → compiler.warp (one self build): see progress lines; the fixpoint run took 1015-1255 s per stage.
- Old lane: `warp rebuild self` (Rust-compiled stages) ~507 s.

## Harness uses of the Rust parser (user decision 2026-10-26: they call the self-hosted compiler in wasm; no
## separate Rust data-notation reader)
- src/host/native.rs:944 channel_next: a message's text read into a value → `standalone::value_of_text`, which runs
  the text through compiler.wasm (`warp pure`'s compiler) and gives its value (text that is no value stays the text).
  Branch rust-bootstrap-parse (step 3a).
- src/ffi/mod.rs:133 is_plain_name: a local check (letters, digits, `_`). Step 3a.
- src/cli/console.rs:15 (does a REPL line carry state: assignment, `let`/`var`/`const`/`global`, statements of those)
  and src/cli/outputs.rs:71 (`warp build --wit`, component_worlds::world_wit of the parsed program): need the parsed
  program as a Node. Plan: compiler.warp gets a view mode (an output path ending in `.json` writes parser.warp's view
  instead of wasm), and the harness turns it into a Node with compiler/node_views.rs (kept in the harness). `--wit`
  is a compile feature: with the Rust lowering gone, component_worlds moves into warp (later).
- Also still parsing with Rust, not in the decision's list: compiler/pure_tests.rs:104/200 (`warp test` finds the
  cases with warp_parser + lowering::test_blocks: the same view mode serves it), formats/meta_section.rs:32
  (parse_data of a module's meta section), main.rs:343/467 (`warp data`/`warp parse` printing). The rest of the
  warp_parser users (numbers/units, time, web, law, formatting, normalize, modules loader) are the Rust compiler and
  leave with it; std-library lowering for plain `warp <file>` is later work (user 2026-10-26) and no blocker.

## Cut plan (on branch `pure`, not main: see PLAN CHANGE above; the steps below land on `pure`)
1. Land rust-bootstrap-seed (Ready, express) and rust-bootstrap-self (step 2): from then on `warp rebuild`, `warp
   pure`, `warp self`, `warp test` touch no Rust-compiled stage.
2. Integrator creates branch `rust-bootstrap` at that main commit (the last main with the whole Rust compiler; it
   builds the seed's ancestors and can always recompile compiler.warp via the old pure lane).
3. Step 3a (rust-bootstrap-parse): the four harness parser uses above call the wasm compiler.
4. Step 3b: plain `warp <file>` / `eval` / `run` / repl run through standalone::run (compiler.wasm) instead of
   pipeline::eval; programs using std modules the self lane can't load yet keep failing loudly (later work).
5. Step 4 (deletion branch, after 3a/3b): remove warp_parser (all), lowering/, analyzer/, wasm_emitter/,
   pipeline.rs, normalize/, the flat passes, compiler/{self_hosted, analysis_memo, compile_*, context,
   dead_functions, function*, local, meta, phase_times, run_time_blocks}, law*, numbers/units, the WARP_SELF_HOSTED
   lanes, and the cargo tests that drive them (they live on in rust-bootstrap). Keep the harness list above.

## Open
- fixpoint-fixes (analyzer.warp binding, parser.warp min-int, walk.warp names_made) landing on main: then refresh the
  seed with scripts/refresh-seed.sh on a tree at that main and commit the new seed (the current seed is 46c441c440 +
  fixes 1+2, and compiles origin/main's lib; a compiler built from main's lib WITHOUT fix 1 miscompiles globals' kinds,
  so its own stage B fails: refresh only after the fixes are on main).
