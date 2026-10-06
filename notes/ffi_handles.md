# C pointers as handles (card ffi-handles)

A C pointer never reaches wasp as an address. It crosses as a **handle id** into the run's table (`ffi::CHandles` in
`HostState`): id n is the n-th pointer the run got, the same pointer keeps its id, **0 is NULL**. The table belongs to
one run and goes with it: ids never outlive the run, nothing is freed in C by warp (call `sqlite3_close`, `fclose`).

```wasp
use sqlite3
db = sqlite3_open(":memory:")                       // sqlite3 **ppDb is the result
stmt = sqlite3_prepare_v2(db, "select 1+2, 'wasp'", -1)
sqlite3_step(stmt)                                  // 100, SQLITE_ROW
sqlite3_column_int(stmt, 0)                         // 3
sqlite3_column_text(stmt, 1)                        // "wasp"
sqlite3_finalize(stmt); sqlite3_close(db)
```

How a C type crosses (`ffi::pointer_kind`):
- `char *`, `const unsigned char *`: a text (a parameter its NUL-terminated copy in linear memory, a result copied
  into a wasp text, NULL ø).
- a pointer to a struct: a handle. A struct is `struct X *` or a type a header of the library declares as one
  (`typedef struct sqlite3 sqlite3;`, `typedef struct __sFILE {…} FILE;`). All headers of a library are read before
  its signatures are classified, since stdio.h uses the FILE that _stdio.h declares.
- any other single pointer (`void *`, `int *`, zlib's `Bytef *`): linear memory as before; as a result a handle.
- `T **` is an out-pointer: left out of the wasp call. The first one becomes the result (a handle); NULL there is a
  loud error naming the C status (`sqlite3_prepare_v2 gave no sqlite3_stmt (C status 1)`). Further out-pointers
  (`const char **pzTail`) receive NULL.
- An id the run never handed out is a loud error (`sqlite3_step: 42 is no C handle of this run`).

All functions crossing pointers share one wrapper (`create_pointer_wrapper`): handle parameters, an out-pointer, a text
or handle result. Passing 0 (NULL) where C does not accept it crashes as it would in C (`fclose(0)`).

Open: Linux declares FILE in bits/types/FILE.h, which is not read, so `FILE *` stays memory there; `char **` as the
only out-pointer (asprintf) gives a handle, not a text.
