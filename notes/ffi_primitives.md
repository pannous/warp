# Generic FFI primitives (topic ffi-libraries-roadmap)

What the pure pipeline and warp-host give every C library, so a library is pure warp over plain C calls
(lib/native/<library>.warp) and nothing of it is in Rust. For Warp Crates, Warp Python and the next libraries.

## Foreign-line types (lib/compiler/c_imports.warp, card pure-c-imports)

`name(p: type, …): type from 'library'` in a program or lib/native/<library>.warp; `use <library>`, `import f from
'library'` or `use crate "x@1"` (library rs_x) bring them. Each becomes an import `library.name` and a line of the
module's custom section `warp.c_functions` (crates/warp-runtime/src/c_signatures.rs, nine tab-separated fields).

| type | C | crosses as |
|---|---|---|
| i32, int, char / i64, long, size_t | int / int64_t | numbers |
| f64, float, double / f32 | double / float | numbers |
| bool | bool | its low byte |
| text | `const char *` | a NUL-terminated copy; a result read back as a text, NULL ø |
| bytes + length | `const void *` + size | a text's bytes or a list of 0 to 255, NULs and all; `length` takes no argument |
| handle | any pointer | an id into the run's handle table, 0 NULL |
| out | `T **` | no argument; the first one's pointer is a handle result, NULL fails the run naming it |
| i64 as a pointer | `const void *` result | the raw address, for c_bytes / c_text below |
| buffer | `void *dest` | the argument is its capacity; the host allocates that many zeroed bytes (card ffi-z) |
| written | `uLongf *destLen`, `size_t *` | no argument; a slot holding the buffer's capacity in, the written length out |
| bytes (result) | see below | a list of 0 to 255, NULs and all; ø for a NULL pointer |
| void | | no result |

## C memory reads (card ffi-sqlite3)

lib/c_memory.warp, answered by warp-host's std_io (crates/warp-runtime/src/warp_host/c_memory.rs):

- `c_bytes(address, count)`: the `count` bytes at a C address, a list of 0 to 255 ([] for NULL). For a blob, a
  buffer: `c_bytes(sqlite3_column_blob(s, 0), sqlite3_column_bytes(s, 0))`.
- `c_text(address)`: the NUL-terminated text at a C address, ø for NULL (a `char *` held as i64, an out text).

The address must come from C and still be valid. Only warp-host answers them (not warp-rust's lanes).

## Bytes results in one call (card ffi-z)

A `bytes` result turns one C call into a byte list, the C convention either way:

- **Caller-sized buffer**, an int status: `compress(dest: buffer, destLen: written, source: bytes, sourceLen:
  length): bytes from 'z'` is called `compress(capacity, data)`. The host allocates `capacity` bytes, passes the
  slot with the capacity, and answers `dest[..*destLen]`. The C int result is the status: non-zero fails the run with
  "compress failed (C status -5)" (failure `nonzero`).
- **Returned pointer** with a written length, for a crate shim keeping its buffer valid (thread-local, static):
  `rs_x_encode(data: bytes, size: length, out_size: written): bytes from 'rs_x'` answers `*out_size` bytes at the
  returned pointer, copied before the next call; NULL is ø. The slot starts at 0 and nothing is freed.

In `warp.c_functions` these are plain lines: `buffer` crosses as `i64` and `written` as `length`; result `bytes`,
failure `nonzero` with a buffer. warp-host reads the two indexes back from the parameter names
(c_signatures.rs `ByteOutput::of_line`), so warp-rust's src/ffi still parses the same lines.

## C memory writes (card ffi-z)

For structs passed by pointer (z_stream): `calloc`/`free` declared from 'c' in the library's native file, then

- `c_write(address, data)`: copies a text's bytes or a byte list to the address, answers the count.
- `c_load(address, size)` / `c_store(address, size, value)`: an unsigned little-endian 1, 2, 4 or 8-byte integer
  at the address, e.g. a field at its offset (`c_store(stream + Z_AVAIL_IN, 4, #data)`).

NULL fails the run. Offsets are the platform's (z.warp's are LP64's, Linux and macOS).

## errno and libc (card ffi-libc)

- `c_errno()` (lib/c_memory.warp, std_io `c.errno`): errno as the last C call left it. c_calls.rs reads it right
  after each described call, before the host's own work can change it; it is per thread. Meaningful only after a
  call that reported a failure (-1, NULL): `unlink(path) == -1` then `strerror(c_errno())`.
- libc is plain foreign lines in lib/c.warp, warp-rust's hand-linked eight (c_signatures.rs HAND_LINKED_LIBC: abs
  strlen atoi atol atof strcmp strncmp rand) included. warp-host links those Rust shims (texts as linear-memory
  pointers) only for a module without a warp.c_functions section, a warp-rust one; a pure module's C calls are all
  described and dlsym'd. The shims stay while frozen src/ffi (mod.rs, link.rs) and standalone.rs use them.
  tests/host/test_ffi_libc.sh.

## Variadic functions: not supported

printf, snprintf, sscanf, open(path, flags, mode), fcntl, ioctl and other `...` functions have no foreign line.
A described call passes its arguments the way the platform passes fixed ones. That happens to work for int and
pointer varargs on x86-64 Linux (they go in the same registers; only %al, the count of vector registers, is
unset), but not for doubles there, and not at all on arm64 macOS, where every variadic argument goes on the stack.
So a declared `snprintf(buffer: i64, size: i64, format: text, x: i32): i32` would print garbage on a Mac.
Use a fixed-argument function instead (strtol, strtod, strerror; open's 3-argument form is fixed on Linux only), or
format in warp. A real primitive would need a `...` type in foreign lines and libffi's ffi_prep_cif_var (or
hand-written per-ABI calls) in c_calls.rs.

## Library lookup (crates/warp-runtime/src/c_calls.rs library_paths)

A path as written; else WARP_LIBRARY_PATH's folders, then the crates cache's `$WARP_CRATES_CACHE/{lib,native}`
(default ~/.cache/warp/crates), then the system's folders; Linux sonames: the plain `lib<name>.so.N` first, then
version-named ones (libSDL2-2.0.so.0, libpython3.13.so.1), highest version first; macOS frameworks.

## Not yet

- Malloc'd out-parameters other than the first handle (`char **errmsg` of sqlite3_exec is passed
  0), callbacks, structs by value (wasmtime's StructIn/StructOut lines exist in c_signatures.rs but c_imports.warp
  doesn't declare them yet).
- A NULL out-pointer or a non-zero status fails the run; it is not a catchable Error yet.
- A struct-layout primitive (fields by name instead of offsets).
