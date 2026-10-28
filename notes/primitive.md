# Primitive mode (`@primitive`)

Decision primitive-annotation, milestone 1 of notes/roadmap_native_harness.md. The code is
core/extensions/primitive.warp, an emitter of plain MVP wasm of its own: no GC types, no runtime words, no exceptions.

## Two places
- **A primitive file**: its first line (after a `#!` line) is `@primitive`. The whole file is lowered by the usual
  passes without the prelude (its words are GC code) and built by primitive.warp: each definition an exported function
  under its written name, the other statements `main`, plus `memory` and the arena global. wasm2c and cc make it a
  native program: test/native/test_native.sh (samples/native/fib.warp, structs.warp). `warp file` runs it too; the
  harness prints main's raw number.
- **A primitive function in a GC program**: `@primitive sq(x: i32): i32 := x * x`. plain_node notes the names an
  extension's attribute marks (walk.warp read_attributes, marked_names) before it drops the layer;
  function_compiled hands such a function to primitive_compiled. It becomes two functions: `sq·primitive`, the MVP
  code, which other primitive code calls directly, and `sq`, the boundary GC code calls: ints come in as i64 and
  floats as f64, cut to the primitive types, the result widened back (bool as Bool). A primitive function calling
  GC code is refused (`the call h(…), not a primitive function`). test/native/test_primitive.pairs.warp.

## Types
The FFI types of a foreign line (c_imports.warp c_type_words) plus the narrow ones of D16: i8/u8/byte/bool,
i16/u16, i32/u32/char/pointer, i64/u64/int/long, f32, f64/float/double/number. `int` is warp's 64-bit int and so is an
untyped int literal; a literal takes the width its expected type asks for (`x: i32 = 5`); 32 bits are spelled `i32`
(decision primitive-int-width). Widening is implicit (i32 → i64, int → float, f32 →
f64), narrowing is `x as u8`, which cuts the bits (masked or sign-spread). `%` is C's remainder (rem_s). A pointer
plus or minus any integer is a pointer, as in C (`p + i * 8`). `handle` is a C pointer outside the module's memory, an
i64 id. A top-level name given a number literal once (`STDERR = 2`) is a constant: every function and main read it as
that literal, so it takes the type its place wants.

## Structs
`struct Header { len: u32; tag: u8; total: i64 }` (or `class`) in a primitive file is a C layout: each field at a
multiple of its width, the size a multiple of the widest. `Header(…)` (positional or named) takes the size, rounded to
8, from a bump arena (global 0, from address 16; the memory grows when the arena passes its end), stores the fields and gives the address; a struct
value is that pointer, `h.len` loads, `h.len = …` stores.

## The prelude
lib/primitive.warp is a primitive file's prelude (its words reached as the GC prelude's are; the GC library's typed
lines are left out, so they make no variants of the file's definitions): WASI's calls as foreign lines (imports of
`wasi_snapshot_preview1`), `print`, `write`, `text_of(n)`, `arg_count()`, `arg(i)` (the program's path at 0),
`read_file(path)` (the empty text when it does not open), `write_file(path, content)`, `exit(code)`, and `byte_at`,
`word_at`, `long_at` (memory read and written as a struct's field). A text is the address of its (address, length) pair,
the $String layout and a WASI iovec at once; its literals are data in front of the arena. `allocate(bytes)` takes
memory from the arena. A function declared `: nothing` gives no value.

## Native build
`warp build native app.warp` (core/commands.warp build_native; Rust only routes the word) compiles the primitive file,
makes it C with wasm2c (module "program"), writes app.native.main.c (primitive.warp native_main: the C calls' glue, instantiate, run_main printing main's value by its type) and
compiles both with cc -O2, wasm2c's runtime (beside the wasm2c on the PATH) and lib/native/wasi.c: libc answering
fd_write, fd_read, fd_seek, fd_close, path_open (paths of the current folder), args_sizes_get, args_get, proc_exit,
and main (instantiate and run_main are main.c's). The executable is `app` beside the file; the intermediate files are removed. Samples: samples/native/ hello,
arguments, files, fib, structs; test/native/test_native.sh. Under wasmtime (`warp app.warp`) the files sample finds
no opened folder, so it runs natively only.

## C calls
`import (getenv, strlen) from "c"` (or `from "m"`) in a primitive file takes the lines of lib/c.warp and lib/math.warp
as a GC program does (c_imports.warp c_catalog, c_import; primitive.warp c_imported), each an import of the module
`c` or `m`; the file's own foreign lines (`f(x: pointer): i32 from 'c'`) are read with primitive types. In a native
build, primitive.warp native_main writes each such import's C function for wasm2c (c_function_glue): it finds the C
function by name in the process (lib/native/foreign.c native_function, dlsym; a library beside libc and libm is
linked with -l) and passes the values as C takes them: a text a NUL-terminated copy (freed after the call), a text
result copied into the arena (exported as global `arena`), `pointer` an address of the module's memory, `handle` a C
pointer as an i64 id (fopen's FILE *). out, bytes, length and owned_text are refused at the import's line. Sample:
samples/native/c_calls.warp. Under wasmtime, warp's own host answers `c` imports with its GC conventions, so a
primitive file calling C runs natively only.

## Driver
lib/native/driver.warp is the native runner (roadmap_native_harness.md milestone 3): a primitive file calling
wasmtime's C API through its own foreign lines (`wasmtime_module_new(engine: handle, wasm: pointer, size: i64, module:
pointer): handle from "wasmtime"`; the structs the calls fill are arena bytes read with long_at/byte_at, native memory
copied in with memcpy). `warp build native lib/native/driver.warp` makes `driver`:
- `driver app.warp args…`: warp.wasm ($WARP_LIB, else beside the driver) compiles the file as the native warp has it
  compile (the file, a module in $TMPDIR, its folder, lib/, runtime.wasm), then the module runs with the arguments.
- `driver app.wasm args…` runs a module; a driver with a module appended (`<module><length u64 LE>WRPMwasm`, padded to
  16 bytes, at the end or where a later Mach-O signature begins) runs that one with all its arguments.
`warp build app.warp` of a GC program writes that executable (core/commands.warp write_carrying: __LINKEDIT grown over
the module, then signed): about 100 KB beside the module. The driver is built once per lib/ into lib/native/built/
(again when driver.warp changes), in the same run as the program: every compile starts with a fresh Module
(emitter.warp fresh_module, card module-inheritance). The Rust warp-runtime stub is no longer written.
Tests: test/native/test_executables.sh.
A module gets WASI (stdin/out/err and the environment inherited, "." and "/" preopened), GC, typed references and
exceptions, wasmtime's machine-code cache. Its `_start` runs (a GC program prints its own value), else `main`, whose
value is printed by its kind; an error's or trap's message goes to stderr with status 1, the module's exit status is
kept. Host words: std_io is a C callback (lib/native/foreign.c warp_host_callback, the values through the driver's
arena to its exported host_word), which reads its arguments through the module's export shown_at (the node as print
shows it, in linear memory) and builds answers with new_text/new_empty/error_of: every word is an Error saying
the driver lacks it (the compiler calls none since kept values were retired). A C function the module's
warp.c_functions section describes (one tab-separated line each, crates/warp-runtime c_signatures.rs) is the same
callback, its data the line and the function (foreign.c library_function: lib<name>.dylib/.so, libc and libm the
process's): numbers go in registers (foreign.c native_call, up to 8 integers and 8 floats), a text NUL-terminated
through shown_at, a handle id through foreign.c's handle table, the first out-pointer to a slot that becomes the
result; a text result is written at the module's text_heap and made by new_text. Bytes, structs and a failed call stop
the run with one line naming the function. Every other host word is a trap naming it. The executable's path is _NSGetExecutablePath's (macOS; Linux needs /proc/self/exe).
warp.wasm has the shown_at export once the seed that builds it has it. Tests: test/native/test_native.sh "by the
driver".

## Not yet (cards)
- f32 in a primitive function of a GC program (no ValueKind with an f32 wasm type there): refused, card
  f32-primitive-refused.
- (card primitive-class-gc) `@primitive class` in a GC program; lists, texts as (pointer, length), `use` of other primitive files,
  `continue`, globals beyond number constants.
- A primitive file's refusal of a struct field has no line: declarations reach the passes as opaque views.
