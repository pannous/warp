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
f64), narrowing is `x as u8`, which cuts the bits (masked or sign-spread). `%` is C's remainder (rem_s).

## Structs
`struct Header { len: u32; tag: u8; total: i64 }` (or `class`) in a primitive file is a C layout: each field at a
multiple of its width, the size a multiple of the widest. `Header(…)` (positional or named) takes the size, rounded to
8, from a bump arena (global 0, from address 16; no memory.grow yet), stores the fields and gives the address; a struct
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

## Not yet (cards)
- f32 in a primitive function of a GC program (no ValueKind with an f32 wasm type there): refused, card
  f32-primitive-refused.
- (card primitive-class-gc) `@primitive class` in a GC program; lists, texts as (pointer, length), `use` of other primitive files, globals,
  `continue`, memory.grow, printing (milestone 2: WASI or libc imports).
- A primitive file's refusal of a struct field has no line: declarations reach the passes as opaque views.
