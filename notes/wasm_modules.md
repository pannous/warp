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
  P140: assigning a mutable global (`level = 5`, `level += 2`, qualified `fourty_two.level = 5`) sets it in the module (the setter import `set level`);
  an immutable one is the loud error of P130's `pi = 4`: "ft is an immutable global of an imported module; fix: another name".
- An export qualified by the module's file stem, `fourty_two.twice(21)`, `fourty_two.ft`, is the same import under the
  key `fourty_two.twice` (wasm_modules::rewrite_uses): it reaches an export named like a warp builtin,
  `fourty_two.double(21)`. P141: the bare `double(21)` is the loud error "double is ambiguous: fourty_two.double(21) for
  the export, 21 as float for the cast" (an export a cast or type word would take).
- P139: `import`, `use`, `require` of a module only declare (their value is ø); `include m` also runs m's `main` (or
  `_start`) and is its value: `include fourty_two` is 42.
- Linking (src/wasm_modules.rs `link`, native): every import from a module path is a host function that instantiates
  the module in the run's store at its first call (HostState::wasm_modules), so its state (globals, memory) lasts for
  the run: `tick(); tick()` counts on.
- A module's own imports are linked like a .wasm file's (`Imports::EVERY`): WASI (`fd_write`), warp's host words
  (`host.random_below`), C libraries, and other modules by path (`(import "tests/fixtures/wasm/counter.wasm" "count_up" …)`,
  relative to the working directory); one instance per file however it is named (fixture greeter.wat).
- Browser (web/playground/host.js `moduleImports`): an import from a module path instantiates the module's binary
  (fetched from the served repository) once per run at the first call, its own imports linked by `programImports`; a
  global reads through its getter, `set g` sets it. WAT text needs the native build, so each fixture .wat has its
  binary .wasm next to it (found first; rebuild with the `wasm-tools parse` line at the top of the .wat).
- Parameter names come from the module's name section (`(param $from i64)`, `$0` … without one): a call names them,
  `fourty_two.minus(amount: 2, from: 10)`, `minus(amount=2, from=10)`, mixed with positional ones in order; an unknown
  name or a wrong count is the error "fourty_two.minus(from, amount) has no parameter by" / "… takes 2 arguments, got 1"
  (wasm_modules ordered_arguments; a bare positional call is not checked, the module's validation catches it).
- A C library compiled to wasm (fixture shout.c → shout.wasm, `clang --target=wasm32 -nostdlib -fno-builtin`): the header
  beside it (shout.h) gives its exports C types (wasm_modules with_header_types, the C FFI's header parser): a `char *`
  parameter copies the text into a block of the module's exported `malloc`, a `char *` result reads the NUL-terminated
  text from the module's memory (NULL is ø) and is typed Text (`t = shout("ab"); letters(t)`); header parameter names
  serve named arguments where the module has no name section. Native only: host.js cannot see an import's result type
  (card wasm-texts-browser). Plan: notes/stdlib_connectors.md "Status 2026-10-06".
- Not yet: `help m.f` (warp has no help word yet). tests/wasm/test_wasm.rs test_import_wasm pins P139 (P144).
- WebAssembly components (`use wasm "lib.wasm" as lib`, `lib.f(x)`) are the other road: notes/stdlib_connectors.md.
