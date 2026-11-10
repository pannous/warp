# Connectors to other ecosystems ("hijack the stdlib of others")

Roadmap card: connectors (board topic; its sub-cards are the steps and leftovers below).
Goal: lots of functionality soon, through FFI, Python, JavaScript, Rust, WebAssembly and native libraries, with a mode
that allows access to anything.

## Overview
| ecosystem | how | where |
|---|---|---|
| C / native | header FFI: `import f from "lib"` / `use lib` reads the headers, dlopen, one wasmtime wrapper per signature shape | extensions/c_imports/imports.warp, core/native/driver.warp, lib/native/foreign.c; handles: wiki/c.md |
| C compiled to wasm | `import lib.wasm` with a header beside it: texts, out-pointers, buffers cross; same results natively and in the browser | notes/wasm_modules.md "How to add a C library" |
| Python | natively libpython through `use python` (python_import, python_call); in the playground Pyodide 0.29.5 | lib/native/python.warp, web/playground/foreign_python.py |
| JavaScript | `use js Math`: the webview natively, the page's globalThis in the browser | extensions/js_globals.warp, lib/host/native_host/webview.warp, host-foreign.js foreignCall |
| Rust crates | `use cargo regex`: a WebAssembly module of the crate with its warp.words section | scripts/warp-crate, lib/crates/, notes/crate_adapter.md |
| WebAssembly components | `load_wasm(./lib.wasm)`, `wasm_call(m, "f", [x])` | lib/wasm.warp |

`use c` in the browser calls web/playground/lib/libc.wasm (wasi-libc built once); natively dlopen (the macOS SDK's
_string.h is read with its `_LIBC_CSTR` annotations dropped).

## Foreign calls (Python, JS)
- Lowering: `use js Math` lowers in extensions/js_globals.warp, `Math.max(1, 5)` → `foreign_call("js", "Math", "max",
  1, [1, 5])`, `Math.PI` a read (`call` 0). The same form for `use python "math"` (`math.sqrt(2)`, `use python
  "os.path" as path`) is not lowered by the self-hosted compiler yet (test/ffi/test_foreign_handles.pairs.warp red).
- Values: JSON both ways. Exact rationals cross as floats; integers beyond 64 bits exact (`{"$int": "digits"}`);
  booleans yes/no; None ø; dicts objects; tuples/sets lists; numpy arrays via tolist(); integral numpy types as Ints,
  numpy floats as Floats.
- Handles: a value without a JSON form (a date, a Fraction, a Decimal, a JS Map or class instance, a function) stays
  in its runtime's table and crosses as `{$handle: 7, type: "date", text: "datetime.date(2020, 1, 2)"}`; it is the
  object again when it comes back. A variable assigned a foreign call holds that runtime's value (`d =
  datetime.date(2020, 1, 2); d.isoformat()`), chained calls ask the receiver's runtime (`np.array([1, 2,
  3]).sum()`). Rules shared with C's handles: ids never addresses, one table per runtime, freed at the end of the run.
- Operators with a foreign operand forward to its runtime's `operator` module: `a * 2` → operator.mul, `-a` → neg,
  `a#2` → getitem(a, 1), `#a` / `count a` / `len(a)` → len, `for x in a` iterates operator.list(a).
- Browser: runtime "js" is the page's globalThis, synchronously (plainOfTree, treeOfPlain); Python through Pyodide,
  loaded by worker.js before the run (a host call cannot wait); npm modules are a loud error there.

## WebAssembly components
- lib/wasm.warp: `m = load_wasm(./lib.wasm)` loads a core module or a component (by its header, WIT types) once per
  run, `wasm_call(m, "f", [x])` calls an export; the native warp serves it, the browser doesn't yet. `use lib.wasm`
  of a component is not lowered by the self-hosted compiler yet (test/ffi/test_component_short_use.warp red).
- A Rust crate builds one with `cargo build --target wasm32-wasip2` and `wit_bindgen::generate!`
  (test/fixtures/components/rust_demo); componentize-py and jco make them from Python and JS.
- Values by the declared WIT type: integers range-checked, floats, bool, char, string, list, tuple (list), record
  ({field: value}), enum (its name), variant (a name or {case: value}), option (ø or the value), flags (a list of
  names); the err of a result is a warp error. WIT kebab-case: `stats_of` finds `stats-of`. Results are plain warp
  values.
