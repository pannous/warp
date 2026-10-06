# Importing WebAssembly core modules

`import name` (also `use`, `require`, `include`) finds `name.wasp`/`name.warp` first; failing those `name.wasm` or its
text form `name.wat`, in the module search directories (src/modules.rs `use_wasm_module`). An explicit path works too:
`import "lib/x.wasm"`, `use "x.wat"`.

```wasp
import tests/fixtures/wasm/fourty_two   // exports ft (global i64 42), twice(i64), half(f64), add32(i32, i32), tick()
twice(ft) + half(5.0)                   // 86.5
```

- Exports with number types (i32, i64, f32, f64) become FFI imports (`FfiSignature`, library = the module's absolute
  path): the emitted program imports `(import "/abs/fourty_two.wat" "twice" (func (param i64) (result i64)))`, calls
  convert arguments and results like C calls (an i32 result is an Int, f32/f64 a Float). Other exports (references,
  v128, memories, tables) are not callable from warp yet.
- An exported global reads as its value: `ft` becomes the call `ft()` (wasm_modules::rewrite_uses), linked to a getter.
  Assigning it is a loud error (like P130's `pi = 4`): "ft is a global of an imported module; fix: another name".
- An export qualified by the module's file stem, `fourty_two.twice(21)`, `fourty_two.ft`, is the same import under the
  key `fourty_two.twice` (wasm_modules::rewrite_uses): it reaches an export named like a warp builtin,
  `fourty_two.double(21)`, which the bare `double(21)` (the float cast) does not.
- Linking (src/wasm_modules.rs `link`, native): every import from a module path is a host function that instantiates
  the module in the run's store at its first call (HostState::wasm_modules), so its state (globals, memory) lasts for
  the run: `tick(); tick()` counts on.
- Not yet: modules that import something themselves (refused loudly at the first call: link WASI or warp's host into
  them), the browser host (tests/modules/test_wasm_modules.rs is native-only), assigning mutable globals, names from the module's name
  section (parameter names for named arguments), the value of `import m` alone (ø now; tests/wasm/test_wasm.rs
  test_import_wasm expects 42 = its main, an open question).
- WebAssembly components (`use wasm "lib.wasm" as lib`, `lib.f(x)`) are the other road: notes/stdlib_connectors.md.
