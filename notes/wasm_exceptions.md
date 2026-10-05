# Wasm exception handling (`try X else Y`)

`try` compiles to the Wasm 3.0 exception proposal (try_table / throw / tag section), see src/wasm_emitter/try_guard.rs.
Only `throw` is catchable: engine traps (ref.cast failure, null, stack overflow, fuel) are not, so runtime errors must be
named error functions that throw (todo.md lists the remaining raw trap sites).

## Runtime support, checked 2026-10-03 with a tiny try_table module (probe: throw 7, catch it, return 7)

| runtime | result | notes |
|---|---|---|
| wasmtime 49 (crate and CLI) | ✓ | on by default; the old CLI 40 in ~/.wasmtime needed `-W exceptions=y` (upgraded to 49.0.2) |
| wasmer 7.5 (Cranelift default) | ✓ | Cranelift support since Wasmer 7, LLVM before; open bug wasmerio/wasmer#7037: a host-thrown exception is not caught by try_table under Cranelift |
| wasmedge 0.17.2 (interpreter) | ✓ | AOT/JIT compile try_table only from 0.18.0-alpha.1 (#5168); 0.18 drops the legacy try/catch encoding |
| browsers | ✓ | Chrome 137+, Firefox 131+; Node ≥ 22.19 by default |
| wasm3, wasmi, wazero | ✗ | no exceptions (and no GC, so they cannot run warp output anyway) |

## Tools
- binaryen `wasm-opt` needs `--enable-exception-handling` (BINARYEN_FEATURES in src/wasm_optimizer.rs), else it rejects the module.
- wabt 1.0.42 `wasm2wat --enable-all` prints try_table; `wasm-tools print` always does.

## Engine limits under `try` (guarded_call, 2026-10-05)
A stack overflow is no wasm exception: wasmtime's Trap::StackOverflow (the browser's RangeError) unwinds the whole call
out of wasm, past every try_table. So `try f(args) else Y` of a user function f calls f through the host
(declarations::guarded_calls → `guarded_call("f·node", [args])`, host.rs / host.js): the host calls f's node wrapper
(the one tasks use) in the same instance; a stack overflow inside comes back as the Error "call stack exhausted"
(built with the exported error_of), which try's is_error path catches. An Int, Float or Text result is converted back
(`as int`, …). A wasm exception thrown inside (raise, a named runtime error) stays pending in the store and the host's
Err rethrows it into the caller, so try catches it as before.
- State after a caught overflow: nothing is rolled back. Globals and memory the aborted call wrote keep what it wrote
  (tests/control/test_try_stack_overflow.rs counts the calls in a global); the instance keeps working.
- Only a direct call of a user function is guarded; any other guarded expression keeps the old behaviour (a stack
  overflow ends the run). Running out of fuel is never caught: it is the runaway guard and leaves no fuel for a fallback.
