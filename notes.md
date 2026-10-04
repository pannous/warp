
⏺ warp binary: 12.3 MiB (7.1 MiB .text)
  ┌────────┬─────────┬─────────────────────────────┐
  │ % Text │  Size   │            Crate            │
  ├────────┼─────────┼─────────────────────────────┤
  │ 35.2%  │ 2.5 MiB │ cranelift_codegen           │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.7%  │ 777 KiB │ wast                        │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.4%  │ 756 KiB │ wasmparser                  │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.4%  │ 755 KiB │ wasmtime_internal_cranelift │
  ├────────┼─────────┼─────────────────────────────┤
  │ 10.0%  │ 723 KiB │ std                         │
  ├────────┼─────────┼─────────────────────────────┤
  │ 8.3%   │ 599 KiB │ wasmtime                    │
  ├────────┼─────────┼─────────────────────────────┤
  │ 2.2%   │ 159 KiB │ wasmtime_environ            │
  ├────────┼─────────┼─────────────────────────────┤
  │ 2.2%   │ 158 KiB │ rustyline                   │
  ├────────┼─────────┼─────────────────────────────┤
  │ 1.8%   │ 128 KiB │ warp (your code)            │
  ├────────┼─────────┼─────────────────────────────┤
  │ 1.7%   │ 125 KiB │ wasm_encoder                │
  └────────┴─────────┴─────────────────────────────┘
  ~77% of the binary is wasmtime/cranelift JIT compiler infrastructure. Your code is only 128 KiB (1.8%). rustyline (new REPL dependency) adds 158 KiB.
## Night 2026-10-04 (raw finds)
- `warp lower <code>` prints the program after all lowering passes: the fastest way to see what the emitter gets.
- `tests/queue.sh -- filter` builds the default features: tests/ffi/* need `tests/queue.sh --all-features -- filter`
  (test.sh uses --all-features). A colored compile error also slips past `grep ^error`: strip ANSI first.
- The parser gives `c()` and `(c)` the same node; closures read a one-item round list of a function variable as a call.
- `def f(x) {…}` stayed a `def` list until the emitter; declarations::keyword_definition now makes it `f(x) := …` first.
- Programs that print and return a list read back as ø: val_to_node only knew atoms (now wasm_reader::node_of).
- ~/.gitignore's `*_LOCAL*` matches case-insensitively on macOS: tests/control/test_branch_assigned_locals.rs was silently ignored (git add refused, mod.rs committed without it); renamed to …_variables.rs. Avoid "_locals" in tracked file names. (night 2026-10-04)
- Night 2026-10-04 bug hunt by probing idioms (`warp eval` one-liners, probes/night/*.wasp): most finds were silent wrong values, not crashes — the raw-struct module shortcut returning an instance for `f(pt(…))`, dropped arguments (`log(100,10)`, `sin(0,5)`, `f(1,2)` of f(x)), str() of a call printing its source, `#m#1` as `(#m)#1`, `w#1.upper()` as `w#(1.upper())`. Probing small realistic programs (word frequency, bank account, matrix transpose) found more than reading code. Probe one-liners first, then fix the shared rule (arity check, position fallback, node_add concat), not the one case.
- macOS ~/.gitignore `*_LOCAL*` is case-insensitive: a tracked file named `…_locals.rs` is silently ignored by `git add`.
- A debug warpgate build serves its UI through rust-embed from warpgate-web/dist at run time: a CI job that only downloads the binary must also get dist (test_gzip "Manifest not found").
- Tried: one shared wasmtime Engine (LazyLock) instead of one per run: no measurable test speedup (lists:: 15 s either way), reverted. The per-test cost is the debug-build Cranelift compile of each module, not engine setup. (night 2026-10-04)
