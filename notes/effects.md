# Effects guide

Code: `src/effects.rs` · Tests: `tests/test_effects.rs` · Design: DESIGN.md "Effects", "Effects as enforced capabilities"

## Model
- Closed set `Effect { State, Allocation, IO, FFI, Async, Unsafe }`, `EffectSet` is a u8 bitset, `Pure` = empty set.
- Trusted signatures: `TRUSTED_EXTERNALS` (fetch → Host/IO, puts/puti/putl/putf → Wasi/IO, fd_write → Wasi/IO+Unsafe).
  FFI functions declared by `import f from lib` / `use lib` get `FFI` (via `analyzer::extract_ffi_imports`).
- Resolution reuses `analyzer::extract_user_functions`; every symbol naming a user function or external
  counts as a call. Nested definitions and `import`/`use` statements are skipped. Top-level code is the
  pseudo-function `main`, so defining an IO function is not IO; calling it is.
- Effects = union over callees, iterated to a fixpoint (recursion safe).
- Only `State` (the `global` keyword) is inferred beyond calls; Allocation/Async are not inferred yet.

## API
- `EffectReport::of(&node)` → `effects_of(name)`, `entry_effects()`, `needs(Capability)`, `call_chain(f, effect)`, `violations`.
- `effects::effects_of(code, "f")` convenience.
- Source query: last statement `effects of f` evaluates to `Pure`, `IO`, or `(IO FFI)`.

## Constraints
`f(x) := … ! Pure` / `! IO, FFI` (the comma form parses as `[(def ! IO), FFI]` and is re-joined).
Violation → `Error("effect violation at L:C: f is declared ! Pure but performs IO via f → log → puts")`.
`! Effects` on a non-function (e.g. `answer := 42 ! Pure`) is a loud "misplaced effect constraint" error.
`without_constraints` strips them before emission.

## Imports follow effects
`WasmGcEmitter::emit_for_node` calls `derive_imports_from_effects`: WASI/host imports only if such a call
resolves, FFI imports pruned to called functions (`use m; 3` imports nothing). Explicit `set_*_imports(true)`
still wins. `eval_parsed` picks the linker from `emitter.imports(Capability)`. The old `code.contains("puts ")`
heuristics are gone.

Consequence: WASI no longer changes `main`'s type. `main` always returns a Node, WASI call results are boxed
with `new_int`, `read_bytes_with_wasi` returns a Node (shared `val_to_node` in wasm_reader.rs).
Before, `puts('ok')` (no space) silently skipped printing; now it really prints.

## Known gaps / next
- `puts x` / `puti x` where `x` is a *function parameter* panics "Undefined variable" (WASI emitter looks up
  main's scope). Pre-existing, now reachable more often.
- WASI output written with no trailing newline glues onto libtest's `test … ok` line, so test.sh can miss
  such tests (e.g. test_list_index_after_puts passes but isn't counted).
- Once `src/semantic/` exists: store `EffectSet` on `Expr`/`FunctionDecl`, spans from the IR, and let the
  import manager emit per-function imports (fd_write only) instead of per-capability groups.
