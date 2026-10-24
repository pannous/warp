# `warp` wrapper over the self-hosted compiler: investigation (2026-10-22, read-only)

Checkout /workspace/cow/warp at bf03df01c (main). No repo file changed, nothing committed. Probe files are in
/workspace/cow/wrapper-probe/ (outside the repo): a hard link of target/debug/warp, the Mac's compiler.wasm copied
over, outputs. M = measured this session, I = inferred from code/notes.

## Findings

### 1. The current binary and its CLI
- Built from src/main.rs (`[[bin]] warp`, Cargo.toml, `required-features = ["native"]`), package warp-lang 1.4.1. Box:
  target/debug/warp (debug, 253 MB, built 17:12 CEST from bf03df01c) (M); nothing in ~/.cargo/bin or /home/box/bin; not
  on PATH (M). Mac: /Users/me/dev/bin/warp = release build of bf03df01c, 47 MB, with compiler.wasm + compiler-self.wasm
  beside it (both "of 9fc81b9241"), plus warp-<commit> symlinks to main-builds/, warp-debug, warp-runtime,
  warp-page-runtime (M).
- Dispatch is one if-chain in src/main.rs `run_command` (l.243–506). Commands: app, format/lint/beautify/fix, verify,
  types, fetch, tool, paint, serve, deploy, dev, data, compile/build/link (--wasm --aot --exe --site --page --wit
  --component --wagi), `test --case`, `test [--self] [--each] [folder]` (pure_tests.rs: already runs through
  `warp pure`), `test a.warp…`, rebuild [pure|self], **pure / self <file|code>** (l.402–413), `[run] f.warp`,
  .wat/.wast, `[run] f.wasm|.cwasm [-- args]`, eval, lower [--json], parse, repl/console, help [module|--markdown],
  version, else: eval the argument string. Flags (src/cli/flags.rs): --fuel/fuel, fast, --strict[=], --no-hints,
  --warnings=, --no-ask, --sandbox, `--` separates program args. Env: WARP_LIB, WARP_HINTS, WARP_NO_WINDOW,
  WARP_EXECUTABLE, WARP_FUEL, WARP_TIME_LIMIT, WARP_SELF_HOSTED, WARP_EMIT_REFUSALS, WARP_COMPILER_WARNINGS, … (M, grep).
- Usage counts (grep of `warp <cmd>`): scripts/: pure 9, self 9, test 7, rebuild 3, run 2, build 1
  (self-check.sh hard-codes /Users/me/dev/bin/warp and `cp`s it); tests/queue.sh: test 2; web/playground: dev 8,
  serve 5, build --site 4, app 4, deploy 3, fetch 2; notes/: pure 41, self 33, test 22, build 22, run 20, rebuild 17;
  probes/: pure 17, rebuild 9, run 8, compile --wasm 6, lower --json 4, eval 4 (M). AGENTS.md/CLAUDE.md: none real.

### 2. The self-hosted pipeline today
- Sources: lib/compiler/*.warp (parser 184 KB, emitter 172 KB, analyzer, passes, runtime/…); entry
  lib/compiler/compiler.warp: `compiler.warp <source> <wasm> [<folder> [<lib>]]`, source in, wasm bytes written to
  args#2, refusal raised ("outside the core: …") (M). Design: notes/standalone_compiler.md, notes/self_hosting.md,
  notes/self_hosting_dropped.md.
- Built by `warp rebuild [pure|self]` (src/compiler/standalone.rs): compiler.wasm = compiler.warp compiled by the Rust
  pipeline; compiler-self.wasm = compiled by itself through the pure lane. Written next to the binary (or
  <lib>/../data/ when WARP_LIB names another lib), stamped with custom section `warp.build`; never built on demand,
  missing = error (standalone.rs l.143–153, 251–273). No build.rs step. Box: both missing (M, `warp version`).
- Invoked today only in-process by the Rust host: standalone.rs `compiled_in` sets args, runs the module with
  `pipeline::run_module` under `kept_by(sha(compiler))`, reads the temp wasm (l.108–126). Also runnable as
  `warp compiler.wasm -- src out.wasm folder lib` (M, below).
- compiler.wasm (Mac copy, 1.7 MB) imports: host.fetch, fetch_within, read, warn, run, std_io + wasi fd_write (M);
  1747 exports incl. `main`, `compile`, GC runtime helpers; uses wasm GC + exceptions. std_io carries `use os`
  (args, env, kept, keep) and `use file` (write_bytes …), implemented by src/host/native.rs `std_call` →
  src/modules/std_adapters.rs (Rust). The program wasm it emits for hello: 1656 bytes, **0 imports**, exports main,
  memory (M).

### 3. End to end (box, debug binary, Mac's compiler.wasm; bash `time`, single runs, no hyperfine)
| run | first | warm |
|---|---|---|
| `warp run samples/hello/hello.warp` (Rust) | 2.97 s | 0.043–0.051 s |
| `warp pure samples/hello/hello.warp` | 14.3 s (keeps lowered runtime words) | 0.24–0.31 s |
| `warp compiler.wasm -- hello.warp h.wasm dir lib` | 8.2 s | 0.25 s |
| `warp h.wasm` | – | 0.017–0.022 s |
| `warp version` | | 0.029 s |
All print "Hello 🌍2026" (M). Parity on 12 random samples/: 1 identical (99-bottles), 9 refused "outside the core"
(class instance, divisors, match, j-i interpolation, `as binary`, `on`, `n times`, map, FFI), 2 failed with
"no message for the diagnostic (parse_error …) in lib/compiler/errors.data" although errors.data has parse_error (a
pure-lane diagnostics bug; lib/compiler unchanged since 9fc81b, so not version skew) (M). `use os; args` refused (M).
Errors carry file:line:col and the source line (M). Exit code 1 on refusal (M).

### 4. Runtimes
- wasmtime CLI: not on box (M); Mac wasmtime 49.0.2: `unknown import host::fetch` (M); notes/standalone_compiler.md
  l.195ff has the WASI plan (card standalone, branch wasi-runtime).
- node: box v20.19.2, Mac v26.12.0 (M); Mac has WebAssembly.Tag (M); Node 20 likely needs --experimental-wasm-gc flags (I). web/playground/std.js + host-files.js implement std_io for the
  browser (IndexedDB files), not for Node (I).
- crates/warp-runtime: no Cranelift (runs only carried .cwasm), no std_io/read/warn (M, host_words.rs). No cargo
  feature strips parser/compiler from the `warp` crate (only native, validate, ffi) (M).

### 5. Gaps of the pure path vs Rust CLI
Language coverage (most samples refused); REPL; `warp <code>` value printing works via run_code but eval/lower/parse
are Rust-only; os.args; FFI/foreign/GPU/pages/serve; compile/build --exe/--site/--wit/…; WASI-only hosting; diagnostics
message lookup bug; `use` of std modules only partly (uses.warp). Running the produced module still needs the Rust
runner (Node GC values back to text, WASI fd_write, fuel/time limit).

### 6. Ownership
Wrapper needs no change in lib/compiler, emitter, folds, aliases/signatures, error positions, standalone.rs or WASI.
Board (Mac `todo list` in ~/dev/angles/warp; from ~ it fails "not a git repository"): related open cards pure-self,
pure-big-int-power, self-showcase, user-cloud, topics self-hosted / self-hosted-2; notes name cards standalone
(reopened 2026-10-21, WASI), warp-pure, installed-compilers, compiler-wasm, under-pure-2, cloud-lane-narrow. No
wrapper/launcher card found.

## Options
A. Shell launcher (recommended). `warp` script: `*.warp`, `run *.warp`, inline code → `warp-rust pure …` (or `self`
   with WARP_COMPILER=self); `compile --wasm f.warp` → `warp-rust <compiler.wasm> -- f f.wasm dir lib`; everything else
   (wasm, serve, dev, app, repl, lower, parse, rebuild, version, help, test…) exec's warp-rust unchanged. Zero Rust,
   reuses existing `warp pure`. Cons: Rust binary still hosts; 0.25 s warm vs 0.05 s; renaming the install breaks
   self-check.sh's `cp` and main's binary refresh, which installs over ~/dev/bin/warp.
B. Node host (`warp.mjs`): instantiate compiler.wasm with Node implementations of std_io(os,file), read, warn, fd_write;
   run output likewise. Zero Rust, Rust-free runtime; but a second host to keep in step with native.rs/std_adapters,
   GC Node reading/printing to port from web/playground host.js. Medium-large.
C. Wait for card standalone (WASI compiler.wasm) and wrap `wasmtime run -W gc=y,function-references=y,exceptions=y
   --dir . compiler.wasm`. Truly Rust-free; blocked on emitter/WASI owners; printing a Node result still needs a host.

## Plan (A)
1. packaging/bin/warp (new, bash): resolve RUST=${WARP_RUST:-$(dirname $0)/warp-rust}; route as above; WARP_PIPELINE=rust
   forces the old path. Refusal: loud by default (decision: no silent fallback); WARP_FALLBACK=rust reruns via Rust and
   prints one notice line.
2. Install: binary → ~/dev/bin/warp-rust beside compiler*.wasm (standalone.rs finds them by current_exe); script →
   ~/dev/bin/warp. Integrator's refresh must install the binary as warp-rust; self-check.sh main_warp → warp-rust.
3. notes/pure_wrapper.md: routing table, env, fallbacks.
Phase 1 coverage: `warp f.warp [-- args]`, `warp run f.warp`, `warp <code>`, `warp compile --wasm f.warp`; test
already pure.
Tests (outcome only, scripts/cloud-test.sh or a shell probe probes/wrapper/check.sh): hello prints "Hello 🌍2026";
refused sample exits 1 with "outside the core"; WARP_PIPELINE=rust matches `warp-rust run`; `warp x.wasm`,
`warp version`, `warp serve --help`-style passthrough unchanged; compile --wasm writes a runnable module.
Risks/questions: which compiler by default (pure vs self)? silent fallback allowed? 0.2 s extra startup accepted?
install rename vs self-check.sh/refresh/homebrew/pypi packaging; cold first run 8–14 s (box) after each rebuild;
eval/REPL stay Rust; the errors.data lookup bug.
Card titles: "warp launcher: `warp f.warp` runs through compiler.wasm (wrapper script)", "pure diagnostics:
parse_error finds no message in errors.data", "install binary as warp-rust beside compiler.wasm".
