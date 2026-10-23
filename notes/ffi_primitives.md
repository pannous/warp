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
| void | | no result |

## C memory reads (card ffi-sqlite3)

lib/c_memory.warp, answered by warp-host's std_io (crates/warp-runtime/src/warp_host/c_memory.rs):

- `c_bytes(address, count)`: the `count` bytes at a C address, a list of 0 to 255 ([] for NULL). For a blob, a
  buffer: `c_bytes(sqlite3_column_blob(s, 0), sqlite3_column_bytes(s, 0))`.
- `c_text(address)`: the NUL-terminated text at a C address, ø for NULL (a `char *` held as i64, an out text).

The address must come from C and still be valid. Only warp-host answers them (not warp-rust's lanes).

## Library lookup (crates/warp-runtime/src/c_calls.rs library_paths)

A path as written; else WARP_LIBRARY_PATH's folders, then the crates cache's `$WARP_CRATES_CACHE/{lib,native}`
(default ~/.cache/warp/crates), then the system's folders; Linux sonames: the plain `lib<name>.so.N` first, then
version-named ones (libSDL2-2.0.so.0, libpython3.13.so.1), highest version first; macOS frameworks.

## Not yet

- Writes to C memory, malloc'd out-parameters other than the first handle (`char **errmsg` of sqlite3_exec is passed
  0), callbacks, structs by value (wasmtime's StructIn/StructOut lines exist in c_signatures.rs but c_imports.warp
  doesn't declare them yet).
- A NULL out-pointer fails the run; it is not a catchable Error yet.
