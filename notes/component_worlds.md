# Component worlds (`warp build --wit`, `warp build --component`)

A program declares the component it is with `component name { import …: {…} export …: interface }`
(samples/wasm_interop.warp, tests/wasm/test_component_worlds.rs). At run time the declaration does nothing.

- `src/lowering/component_worlds.rs` parses the declaration into a `World` (WIT names, kebab-case) and writes its WIT
  (`world_wit`, what `warp build --wit` writes). Types: the scalars of `WIT_TYPES` and `string`.
- `src/component_builder.rs` (`warp build --component`): compiles the program under `pipeline::for_a_component(world.functions())`,
  embeds the WIT and encodes the component with wit-component (which validates it).
- Exports (`src/wasm_emitter/component_adapters.rs`): each exported function is the program's `export def` of that
  name, reached through an adapter `api#add` lifting/lowering the canonical ABI (s32 widened, a string as pointer and
  length, a text result in a return area); `cabi_realloc` allocates on the text heap.
- Imports: `component_worlds::lower` rewrites `host.time()` into a call of `host.time`; the analyzer adds every
  imported function to `ffi_imports` (core import module `host`, field `time`, `imported_signatures`), so all FFI call
  paths resolve it; `emit_import_call` lowers the arguments (a text as pointer and length) and lifts a text result
  from a static return area in the module's data. `imported_kind` gives the call's kind to inference.
- Running: `use name.wasm` loads a component (src/components.rs); warp serves a component's `host` import
  (`serve_host_imports`: print, time in ms since 1970, read a file's text); any other import traps when called,
  naming itself (`unknown import: host#upper`).
- Name conflict: warp's own core host module is also called `host` (crate::host::HOST_LIBRARY). A component program
  that needs warp's own host words (fetch, run…) fails to encode, loudly (its core imports are not in the world).
