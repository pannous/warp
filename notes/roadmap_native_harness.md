# Roadmap: the Rust harness written in warp (draft, 2026-10-28)

User 2026-10-28: "create the Rust harness with pure Warp. For that we would need primitive types … the compiler would
need a hint to only use pure types, no runtime, nothing, to create a pure binary … We might then compile with wasm2c."

## Earlier work (notes/host_from_warp_eval.md, 2026-10-23)
- Every native route failed on the *whole* warp module: wasm2c (wabt 1.0.42) has no GC and only legacy EH; WAMR has no
  standard EH; Binaryen has no GC→linear-memory lowering. The user then dropped those routes and kept the Rust host.
- What worked: driver.warp, warp-host's CLI written in warp (157 lines: run, compile, help, options, refusals, exit
  status) on os.compile_wasm / os.run_wasm / file words; tests/host/test_host.sh 86/86 against it (merge 285e1b7367).
  It left main with probes/ (584de16dc9); restore it with `git show 584de16dc9^:probes/host_from_warp/driver.warp`.
- The new idea gets around that blocker: wasm2c never has to understand GC, because the harness program contains no
  GC. Only warp.wasm (the compiler and user programs) keeps GC, and wasmtime still runs it.

## Measured today
`fib(n: int): int = …` built by `warp build` (699 B trimmed) still declares GC types (array/struct types, ref
globals), so wasm2c refuses it ("unexpected type form 0x5e"). Even pure-integer code needs a mode that emits plain
MVP wasm.

## Milestones
1. **`@primitive`** (core extension core/extensions/primitive.warp, user OK; decision primitive-annotation). Marked
   code is emitted as MVP wasm: i32/i64/f64 locals, bool as i32, linear memory, no GC types, no runtime words, no
   try_table/throw (errors are return values). The mark goes on
   - a function: `@primitive add(a: i32, b: i32): i32 := a + b`. GC code calls it with conversion at the boundary,
     like an FFI call; a primitive function calling GC code is refused;
   - a file, as its first line: every definition in it is primitive, and the file builds to MVP wasm;
   - a class: `@primitive class Header { len: u32; tag: u8 }` is a fixed linear-memory layout, like a C struct; `struct Header { len: u32; tag: u8 }` says
     the same.

   The types are the FFI types, usable everywhere (D16: i32, u8, …), with `pointer` added. Text is a (pointer,
   length) in memory, the existing `$String` layout of the host edge. Lists are fixed arrays in memory. A bump arena
   allocator is written in primitive warp. A form the mode cannot emit is refused loudly at its line. Done when an
   `@primitive` file with `fib 30` goes through wasm2c and cc, runs natively and prints the right value.
2. **Native build**: `warp build native <file>` (words, no flags) runs primitive wasm → wasm2c → `cc -O2` with
   wasm-rt. Imports from `'c'` bind straight to libc symbols through generated glue, and WASI fd_write/args/exit come
   from a ~50-line C shim or a direct libc import. Done when hello, arguments and file read/write all work.
3. **Driver**: revive driver.warp as an `@primitive` file. It loads warp.wasm and user modules through wasmtime's C API
   (libwasmtime + wasm.h, called by FFI like any C library) and links the host words that warp.wasm imports, as C
   callbacks. Done when `warp-native hello.warp` prints Hello. The same runner, with a program's precompiled module
   appended, is the executable `warp build` writes for GC programs (user 2026-10-28), replacing the Rust warp-runtime
   stub that executable-prints-value reconnects in the meantime.
4. **Results without gc_traits**: compiled programs print their own value (card rust-bootstrap, "a standalone
   executable needs the program compiled to print its value"), so the native host never reads GC objects back.
5. **Host words move to lib/**: the files, time, ffi, web, graphics and sound words in src/ (~20k lines) become warp
   libraries that call C libraries (libc, raylib/wgpu-native, CoreAudio) through FFI. The bulk of src/ stays in
   warp.wasm as ordinary GC warp, not in the primitive harness.
6. **Switch-over**: `warp test` (pure_tests.rs) runs on the native host, the native binary replaces ~/dev/bin/warp
   when the full suite matches, and src/ goes to a `rust-harness` snapshot branch (the same way as rust-bootstrap).

## Open
- The user decision of 2026-10-23 ("a fully native no-Rust/no-wasmtime host is not a goal") is revisited: wasmtime
  stays (through its C API), and Rust goes. Recorded as decision native-harness.
- Alternatives to wasm2c: w2c2 (MVP, faster C), or wasmtime's own AOT for the driver (the driver's .cwasm inside the
  warp-runtime stub; route (c) of the 2026-10-23 evaluation, days of work, but Rust stays).
