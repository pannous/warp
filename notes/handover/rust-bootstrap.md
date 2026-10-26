# rust-bootstrap: handover (Worker Bootstrap, 2026-10-26)

Goal (user 2026-10-26 12:58): the Rust compiler moves to its own `rust-bootstrap` branch; main is self-hosted. The
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

## Timing (Mac, load ~9-14)
- seed → compiler.warp (one self build): see progress lines; the fixpoint run took 1015-1255 s per stage.
- Old lane: `warp rebuild self` (Rust-compiled stages) ~507 s.

## Open
- fixpoint-fixes (analyzer.warp binding, parser.warp min-int, walk.warp names_made) landing on main: then refresh the
  seed with scripts/refresh-seed.sh on a tree at that main and commit the new seed (the current seed is 46c441c440 +
  fixes 1+2, and compiles origin/main's lib; a compiler built from main's lib WITHOUT fix 1 miscompiles globals' kinds,
  so its own stage B fails: refresh only after the fixes are on main).
