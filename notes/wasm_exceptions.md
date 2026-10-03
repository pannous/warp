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
