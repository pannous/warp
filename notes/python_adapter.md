# Python from warp (card ffi-python-adapter, topic ffi-libraries-roadmap)

Warp programs import and call Python modules: warp-host loads libpython and calls its C API, the glue is pure warp
(lib/native/python.warp). Nothing Python-specific is in Rust. [M] measured on the box (Linux x86-64, Debian trixie,
Python 3.13.5, numpy 2.x from python3-numpy), [I] inferred. Times CEST.

## Use

```warp
use python
math = python_import("math")
print python_call(math, "sqrt", [16])                               // 4
print python_call("hello", "upper", [])                             // HELLO
print python_call(python_import("json"), "dumps", [{name: "Alice", age: 30}])   // {"name": "Alice", "age": 30}
print python_call_with(json, "dumps", [data], {indent: 2})          // keyword arguments
print python_get(math, "pi")                                        // 3.141592653589793
print python_eval("sorted([3, 1, 2])")                              // [1 2 3]
python_exec("def square(x): return x * x")                          // statements in __main__
print python_text(python_call(python_import("fractions"), "Fraction", [3, 4]))  // 3/4
print try python_call(math, "sqrt", [-1]) else "caught"             // caught
```

| word | gives |
|---|---|
| `python_import(name)` | the module, a reference |
| `python_call(target, name, arguments)` | `target.name(*arguments)` as a warp value |
| `python_call_with(target, name, arguments, keywords)` | the same with keyword arguments (a map) |
| `python_get(target, name)` | the attribute as a warp value |
| `python_eval(expression)` / `python_exec(code)` | `eval` / `exec` in `__main__`'s namespace |
| `python_text(target)` | `str(target)` |
| `python_release(reference)` | drops the reference's Python object |
| `python_object(value)` / `python_value(handle)` | the low-level conversions, a new PyObject handle / a warp value |

- A target is a reference or any warp value: a warp value becomes its Python twin first (`"hello"` a str, so its
  methods work on texts, lists, maps).
- Values cross by value: ø ↔ None, yes/no ↔ True/False, Int ↔ int (64 bits), float ↔ float, text ↔ str, list ↔ list
  (tuple → list), map ↔ dict. Subclasses of float, int and str (numpy.float64, IntEnum) come back as their value.
- Any other object comes back as a reference, the map `{python: <handle>, type: "<type name>"}`. It goes back to
  Python as that object, as a target or an argument. References are not released by themselves (python_release).
- A Python exception is a warp Error raised by `raise`: `python ValueError: math domain error`,
  `python ModuleNotFoundError: No module named 'no_such_module'`. try catches it. Unhandled, warp-host prints it and
  exits 1.

Design decision: functions over the module (`python_call(math, "sqrt", [16])`), not `math.sqrt(16)`. Warp resolves
calls at compile time; a dynamic `m.sqrt` would need a dispatch hook in the compiler (core, owned by others).
`import sqrt from python "math"` could later be compile-time sugar for `python_call(python_import("math"), "sqrt", …)`.

## How it works

- lib/native/python.warp has foreign lines `PyImport_ImportModule(name: text): handle from 'python'` (33 of them,
  lib/c.warp's types), as lib/native/sqlite3.warp. `use python` declares them (card pure-c-imports: the pure compiler
  makes each an import of the module `python` and describes it in the `warp.c_functions` section).
- warp-host resolves the module `python` with dlopen and each function with dlsym (crates/warp-runtime/src/warp_host/ffi.rs,
  c_calls.rs). A `PyObject *` crosses as a handle (an id in the run's CHandles, 0 is NULL); `const char *` results
  (PyUnicode_AsUTF8) are copied into the module as warp texts; texts go in as NUL-terminated copies.
- The glue, all warp:
  - python_start: Py_InitializeEx(0) once (Py_IsInitialized), no signal handlers; sys.stdout gets line buffering so
    Python's own print lines show up (unflushed otherwise: nothing calls Py_Finalize at exit).
  - Arguments: PyTuple_New + PyTuple_SetItem (steals), PyObject_Call(callable, tuple, kwargs or NULL).
  - Conversion back: the type's name (PyObject_Type + `__name__`), then PyLong_AsLongLong, PyFloat_AsDouble,
    PyUnicode_AsUTF8, PyObject_IsTrue, PySequence_GetItem, PyDict_Items; PyObject_IsInstance for subclasses.
  - Refcounts: every new reference the glue makes is Py_DecRef'd; borrowed ones are not.
  - Errors: a NULL result (and PyErr_Occurred after PyLong_AsLongLong) → PyErr_GetRaisedException (3.12+), its type
    name and str → `raise`. PyErr_Fetch is not used: its three `PyObject **` out-pointers don't fit the `out` type
    (only the first out-pointer is filled, the others get NULL).
  - None: Py_GetConstant(0), new in 3.13. So the adapter needs Python ≥ 3.13 (3.12 would need None another way).
- lib-dynload extension modules (_contextvars, _decimal, numpy's _multiarray_umath) are not linked to libpython: they
  need the C API among the process's global symbols. warp-host's dlopen is RTLD_LOCAL, so the glue calls libc's
  `dlopen("libpython3.13.so.1.0", RTLD_NOW|RTLD_GLOBAL|RTLD_NOLOAD)` right after Py_InitializeEx, the soname built from
  `sys.version`. dlopen is a foreign line `from 'python'`: dlsym on libpython's handle finds it in libpython's
  dependency libc. [M] Before that, `fractions` failed: `undefined symbol: PyContextVar_Type`.

## Finding libpython

- Linux: the package libpython3.X (Debian/Ubuntu; python3 alone ships a static interpreter, no .so) gives
  /usr/lib/x86_64-linux-gnu/libpython3.13.so.1.0 and .so.1; python3-dev adds libpython3.13.so. On this box it had
  to be installed: `apt install libpython3.13`.
- warp-host: the generic rule `version_named_sonames` (c_calls.rs, this branch): a library named without the version
  its file carries, `python` → the newest `libpython<version>.so…` of the library folders and WARP_LIBRARY_PATH
  (`lua` → liblua5.4.so.0). Highest version first, then the shortest name. Without the rule a folder in
  WARP_LIBRARY_PATH with a `libpython.so` symlink works too.
- macOS [I, untested]: Homebrew's is
  /opt/homebrew/opt/python@3.x/Frameworks/Python.framework/Versions/3.x/Python; python.org's
  /Library/Frameworks/Python.framework/Versions/3.x/Python. Neither is found by the name `python` yet: library_paths
  would need `/opt/homebrew/opt/python3/Frameworks/Python.framework/Python` and
  `/Library/Frameworks/Python.framework/Python` for `python` (or WARP_LIBRARY_PATH with a libpython.dylib symlink).
  The RTLD_GLOBAL step is a no-op there (the soname differs; dlopen gives NULL, ignored); macOS extension modules
  resolve the C API with `-undefined dynamic_lookup` from the flat namespace, which may still need it global.
- Windows: not handled.

## Measured (box, 2026-10-23 00:40–01:05 CEST, box load 8–50)

warp-host of this branch (`cargo build --profile host -p warp-runtime --features host --bin warp-host`, 20m53s under
load), warp.wasm of the pure-c-imports worktree (`--compiler full`), no WARP_LIBRARY_PATH: `python` found by
version_named_sonames. Each sample ~8.8 s wall, the compile included.

samples/python/python_modules.warp (`--compiler full`):
```
4
HELLO
[1, "two", 3.5, true, null]
{"name": "Alice", "age": 30}
3.141592653589793
[1 2 3]
caught the ValueError
[['a' 3] ['n' 2]]
3/4
144
```
samples/python/numpy_matrix.warp: `[[7 10] [15 22]]` and `-2.0000000000000004`.

tests/host/test_python.sh: 15/15 passed with this branch's host (2m53s); 15/15 before with pure-c-imports' host and a
`libpython.so` symlink in WARP_LIBRARY_PATH (6m34s, heavier load). warp-self.wasm was not built here, so the
default `--compiler self` lane is untested [I: it shares core/emitter/c_imports.warp].

## Open

- Depends on card pure-c-imports (C imports on the pure lane, merged into this branch from its tip 773fd6ee8, not
  yet on origin when merged): without it every Python call is refused `outside the core`.
- CHandles never shrinks and finds a pointer's id by a linear scan: converting big lists is quadratic, and every
  short-lived PyObject pointer stays in the table for the run. A generic fix: a map pointer → id, or releasing ids.
- Int beyond 64 bits: PyLong_AsLongLong overflows → `python OverflowError`. Big ints could cross as text (str/int).
- bytes, set, numpy arrays come back as references (call `tolist`, `decode`).
- No callbacks (Python calling warp functions): would need a generic C callback primitive in warp-host.
- The GIL: the program's thread initializes Python and holds the GIL; calls from other threads are not handled.
- `print` of a reference shows `{python:37 type:"Counter"}`; python_text gives its str.

## After merging origin/pure-c-imports 57f273b59 (2026-10-23 ~03:00 CEST)

One version_named_sonames (theirs: plain sonames before version-named ones). warp-host rebuilt (1m24s), compilers of
the pure-c-imports worktree at 57f273b59. [M] tests/host/test_python.sh: 15/15 with `--compiler full` (2m21s) and
15/15 with `--compiler self` (57s), so the self lane works too. LD_DEBUG=files: `python` opens
/lib/x86_64-linux-gnu/libpython3.13.so.1 after the plain names libpython.so, .so.0 and .so.6 (also in the crates cache)
miss.
