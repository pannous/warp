# Wasm features: what warp uses, wants, and skips

Decisions and directions from the user (2026-10-03). Runtime support for exceptions: notes/wasm_exceptions.md.

## On by default
- GC, reference types, typed function references, bulk memory, exception handling (`try`).
- Extended constant expressions (Wasm 3.0): "should just be default". wasmtime and wasmparser enable them by default,
  wasm-opt gets `--enable-extended-const` (BINARYEN_FEATURES in src/wasm_optimizer.rs). The emitter does not use them
  yet: a global with an arithmetic initialiser (`global x = 2*21`) could be a constant expression instead of a store in main.

## In progress (workers spawned 2026-10-03, branches of the same name)
- typed-lists: homogeneous Int/Float lists as GC arrays behind one list-operation dispatch layer (notes/typed_lists.md).
- closures: first-class functions as a GC struct of a typed function reference plus captured values (notes/closures.md).
- multi-value: real multi-value results instead of globals/scratch workarounds (notes/multi_value.md).

## Not wanted: SIMD
We are not interested in wasm SIMD (128-bit or relaxed). Vector work goes the direct vector/GPU path: list operations
above a size threshold switch to a host-native or GPU backend behind the typed-lists dispatch seam, without the program
noticing. The flexible-vectors proposal (length-agnostic vectors) never got far, so this is our own mechanism: the
backend choice lives in warp's list-operation table, not in wasm instructions.

## Ideas, not started
- Tail calls (`return_call`): constant stack for recursion.
- `i31ref` for small Ints: no i64box allocation.
- Branch hints: mark the runtime-error branches unlikely.
- Multi-memory: see below.
- Wide arithmetic (`i64.mul_wide`, `i64.add128`): limb arithmetic of the unbounded Int; still a proposal.
- Stack switching: generators, coroutines, async (D2 `!`); still a proposal.

## Multi-memory ideas
- Separate the constant string table (read-only data) from the runtime text heap, so a stray write can never corrupt literals.
- A memory per FFI library: raylib/SDL buffers in their own memory, shared with the host, isolated from wasp texts.
- A GPU staging memory: typed-array payloads copied into one memory that the host maps straight to a GPU buffer.
- Scratch memory for big-int limbs or temporary texts, dropped wholesale after a call.
