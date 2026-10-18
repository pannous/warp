# Connectors to other ecosystems ("hijack the stdlib of others")

Roadmap card: connectors (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).
Goal: lots of functionality soon, through FFI, Python, JavaScript, Rust, WebAssembly and native libraries, with a mode
that allows access to anything.

## Overview
| ecosystem | how | where |
|---|---|---|
| C / native | header FFI: `import f from "lib"` / `use lib` reads the headers, dlopen, one wasmtime wrapper per signature shape | src/ffi/link.rs, ffi_parser.rs; handles: notes/ffi_handles.md |
| C compiled to wasm | `import lib.wasm` with a header beside it: texts, out-pointers, buffers cross; same results natively and in the browser | notes/wasm_modules.md "How to add a C library" |
| Python | `use python "math"`: one `python3 -u` child per process speaking JSON lines; in the playground Pyodide 0.29.5 | src/modules/foreign.rs, web/playground/foreign_python.py |
| JavaScript | `use js Math`: a `node` child natively, the page's globalThis in the browser | foreign.rs FOREIGN_JS_LOOP, host.js foreign_call |
| Rust crates | `use cargo regex`: a wrapper component generated from rustdoc JSON | notes/cargo_crates.md |
| WebAssembly components | `use lib.wasm` / `use wasm "lib.wasm" as lib` | src/modules/components.rs |

`use c` in the browser calls web/playground/lib/libc.wasm (wasi-libc built once); natively dlopen (the macOS SDK's
_string.h is read with its `_LIBC_CSTR` annotations dropped).

## Foreign calls (Python, JS)
- Lowering (src/lowering/foreign/foreign_modules.rs, the first source pass): `use python "math"` / `use python math` /
  `use python "os.path" as path` record the module and vanish; `math.sqrt(2)` → `foreign_call("python", "math",
  "sqrt", 1, [2])`, `math.pi` a read (`call` 0: an empty argument list arrives as ø). A word right after `.` is a
  member name (`math.pi` stays `pi`, no π).
- Host: `WARP_PYTHON` / `WARP_NODE` name another interpreter (bun works); a promise is awaited. An exception is an
  error with the foreign message. Results are built with tasks::Builders (host::VALUE_GIVING_WORDS) and held as Nodes
  (Kind::Empty), so arithmetic and comparisons decide at run time (node_order).
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

## WebAssembly components (`use wasm`)
- `use lib.wasm`, `use lib`, `use lib as l`, `use wasm "lib.wasm" as lib` (alias: the file's stem; a missing one is
  the error naming the components there; a relative path is next to the program, modules::beside_program). A Rust
  crate builds one with `cargo build --target wasm32-wasip2` and `wit_bindgen::generate!`
  (tests/fixtures/components/rust_demo); componentize-py and jco make them from Python and JS.
- `lib.f(x)` → foreign_call("wasm", path, "f", 1, [x]); components.rs loads each component once per path and process
  on an engine with the component model on (the programs' engines stay core-only), WASI p2 linked (stdio inherited,
  no files).
- Values by the declared WIT type: integers range-checked, floats, bool, char, string, list, tuple (list), record
  ({field: value}), enum (its name), variant (a name or {case: value}), option (ø or the value), flags (a list of
  names); the err of a result is a warp error. WIT kebab-case: `stats_of` finds `stats-of`. Results are plain warp
  values.
- Resources stay in the component behind a handle `{$handle: 1, type: "counter", text: "counter#1", component:
  path}`: `r.counter(5)` / `r.Counter(5)` call the constructor, `r.merged(a, b)` a static, `c.increment(2)` a method
  (a method call on a variable holding a component's result goes to the component unless it is a warp method);
  `c.drop()` runs the destructor (a dropped handle is refused; a WIT method named drop stays that method). Handles
  cross back as `own`/`borrow`; another resource type or component is refused.
- Operators of handles: `a + b` is `a.add(b)` (add, sub, mul, div, mod, pow, lt, gt, le, ge, eq, ne);
  components::result_kind tells at compile time which calls give resources, so plain results stay warp arithmetic.
  Native only (the browser build reads no component at compile time).
- Not yet: `warp build --exe` with components.

## Capabilities
Foreign calls (C, Python, JS, components) run under the Ffi capability: granted by `eval` and the CLI, refused to
`eval_untrusted`, whose grant also applies to the check after lowering (pipeline `GRANTED`). P88 "allow everything"
grants Process and Sql too. A component without preopened files is sandboxed: granting it to eval_untrusted is an open
question. `warp build --exe` runs `use python`/`use js` (the stub starts python3 or node from PATH, a missing
interpreter named loudly; crates/warp-runtime/src/foreign.rs); callbacks, components and C libraries stay refused
(notes/aot.md).
