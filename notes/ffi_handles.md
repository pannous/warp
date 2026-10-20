# C pointers as handles

A C pointer never reaches warp as an address. It crosses as a **handle id** into the run's table (`ffi::CHandles` in
`HostState`): id n is the n-th pointer the run got, the same pointer keeps its id, **0 is NULL**. The table belongs to
one run and goes with it: ids never outlive the run, nothing is freed in C by warp (call `sqlite3_close`, `fclose`).

```warp
use sqlite3
db = sqlite3_open(":memory:")                       // sqlite3 **ppDb is the result
stmt = sqlite3_prepare_v2(db, "select 1+2, 'warp'", -1)
sqlite3_step(stmt)                                  // 100, SQLITE_ROW
sqlite3_column_int(stmt, 0)                         // 3
sqlite3_column_text(stmt, 1)                        // "warp"
sqlite3_finalize(stmt); sqlite3_close(db)
```

How a C type crosses (`ffi::pointer_kind`):
- `char *`, `const unsigned char *`: a text (a parameter its NUL-terminated copy in linear memory, a result copied
  into a warp text, NULL ø).
- a pointer to a struct: a handle. A struct is `struct X *` or a type a header of the library declares as one
  (`typedef struct sqlite3 sqlite3;`, `typedef struct __sFILE {…} FILE;`). All headers of a library are read before
  its signatures are classified, since stdio.h uses the FILE that _stdio.h declares.
- any other single pointer (`void *`, `int *`, zlib's `Bytef *`): linear memory; as a result a handle.
- `T **` is an out-pointer: left out of the warp call. The first one becomes the result (a handle); NULL there is a
  loud error naming the C status (`sqlite3_prepare_v2 gave no sqlite3_stmt (C status 1)`). Further out-pointers
  (`const char **pzTail`) receive NULL.
- An id the run never handed out is a loud error (`sqlite3_step: 42 is no C handle of this run`).

All functions crossing pointers share one wrapper (`create_pointer_wrapper`): handle parameters, an out-pointer, a text
or handle result. Passing 0 (NULL) where C does not accept it crashes as it would in C (`fclose(0)`).

FILE and DIR are structs everywhere (ffi STANDARD_STRUCT_TYPES): Linux declares FILE in bits/types/FILE.h, which is
not read. A `char **` out-pointer
(strtod's endptr) is no handle out-pointer: left out of the warp call as any out-pointer, but the C
function's own result stays the result (`strtod('2.5')` is 2.5, `strtol('ff', 16)` 255). Open: asprintf's text,
written to its `char **`, is not read back.

## Structs as maps, failure reports (card wasmtime-ffi)

Headers are read through their macros (src/ffi/macros.rs: function-like macros expanded with `##`, words some branch
defines empty dropped, C keywords never) and with the headers they include from their own include root, includes
first (ffi_parser `with_own_includes`): wasm.h declares `wasm_engine_new` through `WASM_DECLARE_OWN(engine)`.
A function-pointer parameter (`void (*finalizer)(void *)`) crosses as a `void *`.

A struct or union a header defines with a body whose fields all resolve (src/ffi/struct_layouts.rs; C11 anonymous
members included) has a layout (crates/warp-runtime/src/c_structs.rs, LP64 offsets): a pointer to it crosses as a
warp **map** (numbers, pointers as handle ids, arrays as lists, nested structs and unions as maps; a union read back
holds every member).
- `const T *`: a map written into a copy (a handle id passes its pointer); `const T *, size_t n`: a list of maps, n
  filled in.
- a trailing non-const `T *` of a function that reports its success apart (an error-shaped result, a bool result, an
  error out-pointer): an **output**, left out of the call, the call's result; `T *, size_t n` gives a list of n, n
  passed by warp. Other non-const struct pointers (raylib's `Image *image`, in and out) stay handles.
- error-shaped types (`error`, `trap`, `status`, `exception` in the name, `is_error_type`): a `wasmtime_error_t *`
  result or a `wasm_trap_t **` out-pointer that is not NULL fails the call loudly (`wasmtime_module_new failed (C
  result 0x…)`), as does false from a bool function with an output. Open: the message text (wasmtime_error_message).

```warp
use wasmtime                        // lib/wasmtime.warp and the C library libwasmtime
instance = wasmtime_linker_instantiate(linker, context, module)   // wasmtime_instance_t *: {store_id: 1, __private: 0}
main = wasmtime_instance_export_get(context, instance, "main")    // wasmtime_extern_t *, false fails
results = wasmtime_func_call(context, main.of.func, [], 1)       // wasmtime_val_t *results, size_t nresults
results#1.of.i64
```
A standard module named after an installed C library brings both (`use wasmtime`, as `use math` brings libm).
Trap: a second wasmtime in the process (Homebrew's beside warp's) installs its own signal handlers on its first
memory, and warp's next trap becomes a SIGILL: lib/wasmtime.warp turns `signals_based_traps` off.
