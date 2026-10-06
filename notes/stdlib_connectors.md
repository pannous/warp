# Connectors to other ecosystems ("hijack the stdlib of others")

User, 2026-10-05: "write connectors to different libraries… FFI mechanism, but also Python, JavaScript, Rust and native
libraries, maybe a shallow wrapper or a general mechanism… We want lots of functionality soon… a mode that allows
access to anything." Card hijack-stdlib. Survey of what exists, options per ecosystem, recommendation, first step.

## What exists (main 1e0dea73f)
- **C via FFI** (src/ffi.rs, ffi_parser.rs): `import f from "lib"` / `use lib` reads the library's headers
  (`<lib>.h`, `<lib>/<lib>.h` in the SDK and /opt/homebrew/include), parses the declarations, opens `lib<lib>.dylib/.so`
  (libloading) and links one generic wasmtime wrapper per signature shape (i32/i64/f32/f64/pointer params). Texts cross
  into C as NUL-terminated copies in linear memory. Works for libm, libc (string.h, stdlib.h, stdio.h) and SDL2.
  Gaps probed:
  - `char*` results are refused, so `getenv` is "undefined function".
  - The header name is guessed from the library name, so zlib (`z` ↔ `zlib.h`) is never found and `zlibVersion()`
    returns the bare name.
  - `ctype.h` isn't searched, so `toupper` is unknown.
  - sqlite3 panics (a loud error is owed).
  - Struct and out-pointer APIs (`sqlite3_open(&db)`) need handles.
- **Host words** (src/host.rs): Rust functions the module imports, Node in / Node out. `run_block` reads Nodes with
  `wasm_reader::node_in` and builds its result with `tasks::Builders` (TaskValue: ø, Int, Float, Char, Text, Symbol,
  List, Key, Closure, Exact). That is a general bridge for any value: every connector below can use it.
- **Capabilities** (src/effects.rs): Host, Wasi, Ffi, Libm, Sql, Process. `eval` grants Host+Wasi+Ffi+Libm,
  `eval_untrusted` only Libm. Process (`exec sh "…"`) and Sql are granted by no host yet: there is no "allow anything"
  mode.
- **Packages** (notes/packages.md): `use name` = local `.wasp`, then builtin/FFI library, then the git registry
  packages.wasp. Package tools are prebuilt wasm32-wasip1 commands.
- **AOT** (notes/aot.md): `warp build --exe` links only print, libm and the host words without compiler; a program
  importing FFI libraries is refused at build time. The browser build gets the host word names only.

## Values crossing
Wasp data is a superset of JSON (Node::to_json / Node::from_json exist), so every dynamic runtime speaks JSON:
lists ↔ arrays, `{a:1}` ↔ objects, texts, numbers, booleans (1/0), ø ↔ null. Objects without a JSON form (a numpy
array, a file handle) come back as their text (`repr`) for now. Handles (proxies that stay in the foreign runtime) are
step 3.

## Options per ecosystem
| ecosystem | mechanism | values | effort | reach |
|---|---|---|---|---|
| C / native | existing header FFI; fix the gaps (header aliases zlib/ctype, `char*` results as text, loud errors instead of panics, opaque pointers as Int handles) | numbers, texts in and out, pointers as handles | S each gap | libc, libm, zlib, sqlite, curl, SDL… anything with a header |
| Python | a persistent `python3` subprocess speaking JSON lines (one per warp process, started on first use); `use python "math"` → `math.sqrt(2)` | JSON; repr text for the rest | **S (first step)** | all of PyPI that is installed: numpy, requests, sympy… |
| Python (browser) | `use python` in the playground: worker.js loads Pyodide 0.29.5 from jsdelivr before the run (a host call cannot wait), host.js pythonCall answers foreign_call with the same bridge (web/playground/foreign_python.py, shared with the native child). Pyodide 314.x fails importScripts in a classic worker (2026-10-06), hence 0.29 | JSON, $int for big integers, handles stay in Pyodide | done | the stdlib and Pyodide's packages |
| JavaScript (native) | the same loop under `node`/`deno`/`bun` | JSON | S once Python exists (same protocol) | npm |
| JavaScript (browser) | the JS host calls `globalThis[module][function](...args)` directly | JS values ↔ Nodes in host.js | S | DOM, Math, fetch, anything on the page |
| Rust crates | no ABI to call into dynamically. Options: (a) a crate compiled into warp as host words (what host.rs does, per crate, by hand); (b) a crate built as a wasm32-wasip1 component and linked as a second module (the package-tool path, needs cross-module GC types); (c) a crate exposing `extern "C"` and used through the C FFI | (a) Nodes, (c) C types | M per crate (a); L general (b) | curated |
| Embedded CPython (pyo3) | in-process, faster calls, true object handles | Python objects | M-L, adds libpython to warp's link, not offline-registry friendly | same as subprocess |
| WASI components / WIT | the long-term general answer: any language compiled to a component with a WIT interface; warp already parses WIT | WIT types ↔ Nodes | L (component model is turned off in the engines today) | everything that targets components |

## The "access to anything" mode
- Foreign calls (C, Python, JS) run arbitrary native code: they are the same trust as FFI. They sit under the Ffi
  capability: granted by `eval` and the CLI, refused to `eval_untrusted`.
- A CLI mode granting Process and Sql too (`warp --allow all file.wasp`, or `--allow process,sql`) is the missing piece
  for `exec`/`execute`. Queued as a question to warp-08.
- AOT: `warp build --exe` keeps refusing programs with foreign imports, naming them (as for FFI libraries). The stub
  could link `foreign_call` later (std::process only, no Cranelift).

## Recommendation
1. **Python bridge now** (S, biggest reach per line of code): one host word `foreign_call(runtime, module, member,
   arguments)`. `use python "math"` (or `use python math`) names a module, `math.sqrt(2)` is a call, `math.pi` a read.
   The same protocol serves node/deno next (`use js "lodash"`), and the browser host maps runtime "js" to globalThis.
2. **C gaps** (S each): header aliases (z→zlib.h, ctype.h in libc), `char*` results as text, a loud error instead of
   the sqlite panic, opaque pointers as Int handles.
3. **Handles**: a foreign object without JSON form stays in its runtime behind an id (`np.array([1,2])` → handle 7,
   `h.sum()` → a call on handle 7), freed when the program ends.
4. **Components/WIT** when the component model is turned on again: the general, typed, sandboxable connector.

## Step 1 as implemented (branch hijack-stdlib)
- Lowering (src/lowering/foreign_modules.rs, the first source pass, before `x.word(…)` becomes a method call of a
  built-in word): `use python "math"` / `use python math` / `use python "os.path" as path` record the module and
  vanish; `math.sqrt(2)` → `foreign_call("python", "math", "sqrt", [2])`, `math.pi` →
  `foreign_call("python", "math", "pi", ø)` (ø: a read, a list: a call).
- Parser: a word right after `.` is a member name: `math.sqrt`, `math.pi` stay `sqrt`, `pi` (no √ operator, no π).
- Host (src/foreign.rs, native only): one `python3 -u` child per warp process (`WARP_PYTHON` names another), its loop
  script embedded; request and answer one JSON line each; an exception is an error with the Python message
  (`AttributeError: module 'math' has no attribute 'nope'`). The result is built with tasks::Builders like run_block's
  (host::VALUE_GIVING_WORDS: the module exports its constructors and exact builders).
- Values: exact rationals cross as floats (`2.5` is 5/2 in warp), integers beyond 64 bits come back exact
  (`{"$int": "digits"}`: `math.factorial(25)` → 15511210043330985984000000), booleans 1/0, None ø, dicts objects,
  tuples/sets lists, numpy arrays via tolist(), anything else its repr text.
- Capability: foreign_call is a host word linked like an FFI import, so it needs Ffi: eval grants it, eval_untrusted
  refuses it. eval_untrusted used to check only the parsed program, before lowering added calls of its own; its grant
  now also applies to the check after lowering (pipeline `GRANTED`).
- AOT (warp-ec, 2026-10-05): `warp build --exe` names foreign_call as a missing import and refuses the program; the stub
  could link it later (std::process only) but an exe silently needing python3 would surprise.
- Next: the C gaps (card ffi-gaps); handles (card foreign-handles).

## Step 2: `use js` (branch use-js)
- `use js Math` / `use js "path"`: natively a `node --no-warnings -e` child running FOREIGN_JS_LOOP (`WARP_NODE` names
  another node-compatible runtime, e.g. bun): a global (`Math`, `JSON`, `crypto`) or a module it requires or imports;
  a method is applied to its object; a promise is awaited; BigInts beyond 64 bits cross as `{"$int": …}`.
- Browser (web/playground/host.js foreign_call): runtime "js" is the page's own globalThis, synchronously; Python and npm
  modules are a loud error there ("runs only in the native host"). Trees ↔ plain values: plainOfTree, treeOfPlain.
- foreign_call(runtime, module, member, call, arguments): `call` (1/0) says a call from a read, since an empty
  argument list arrives as ø.
- Results are held as Nodes (Kind::Empty, like map values): arithmetic and text joining decide at run time, and an
  ordering comparison of a held Node goes through node_order (`time.time() > 0`, also cells).


## Step 3: handles (branch foreign-handles)
- A value without a JSON form stays in its runtime: Python's loop and node's loop keep a table (id → object), the page's
  host.js too (foreignHandles). It crosses as `{$handle: 7, type: "date", text: "datetime.date(2020, 1, 2)"}`, prints
  as that record, and is the object again when it comes back (an argument, or the receiver in the module position).
- Lowering (foreign_modules.rs `Foreign`): a variable assigned a foreign call holds that runtime's value
  (`d = datetime.date(2020, 1, 2); d.isoformat()`); a chained call asks the runtime of its receiver
  (`np.array([1, 2, 3]).sum()`). Reassigning the variable ends that.
- Python numbers stay numbers: integral types (numpy ints) as Ints, inexact reals (numpy floats) as Floats; exact
  rationals (Fraction) and Decimals are handles. JS: Dates, Maps, class instances, functions are handles.
- Shared rules with C's handles (warp-2d, card ffi-handles, table in src/ffi.rs): ids never addresses, one table per
  runtime, freed at the end of the run, no user-visible free yet.
- Operators (branch foreign-operators): an operator with a foreign operand forwards to its runtime's `operator` module:
  `a * 2` → operator.mul(a, 2), `-a` → neg, `a#2` → getitem(a, 1) (1-based to 0-based), `#a` / `count a` / `len(a)` →
  len, `for x in a` iterates operator.list(a). Python: its operator module plus len and list; node and host.js: an
  object with the same names. `(a * 2).sum()` chains through the group.

## Step 3: `use wasm` — WebAssembly components (card wit-components, branch wit-components)
- `use wasm "lib.wasm" as lib` (alias by default the file's stem): a WebAssembly component, the general answer for Rust
  crates and every language that compiles to components. A Rust crate builds one directly: `cargo build --target
  wasm32-wasip2` with `wit_bindgen::generate!` exporting a WIT world (tests/fixtures/components/rust_demo: fib, words,
  a record, an enum, a result; 84 KB). componentize-py and jco make them from Python and JS.
- `lib.f(x)` → foreign_call("wasm", path, "f", 1, [x]); src/components.rs loads each component once per path and
  process on an engine of its own with the component model on (the programs' engines stay core-only), WASI p2 linked
  for its imports (stdio inherited, no files). A relative path is next to the program's file (modules::beside_program).
- Values by the WIT type the function declares: integers range-checked into s8…u64, floats, bool, char (a one-character
  text), string, list, tuple (list), record ({field: value}), enum (its name as text), variant (a name or {case: value}),
  option (ø or the value), flags (a list of names). Results back: the same as JSON → Nodes; the err of a result is a
  warp error with its message. WIT names are kebab-case: `stats_of` finds `stats-of`.
- A component's results are plain values: `s = lib.stats_of(t); s.words` reads the record's field, `n * 2` is warp's
  arithmetic, `ws.reverse()` warp's method (foreign_modules does not forward operators of wasm values).
- Resources (card wit-resources): a resource a component gives stays in it behind a handle
  `{$handle: 1, type: "counter", text: "counter#1", component: path}` (shared handle rules: ids, one table per
  component, kept until the process ends). `r.counter(5)` / `r.Counter(5)` call `[constructor]counter`, `r.merged(a, b)`
  a `[static]counter.merged`, `c.increment(2)` the `[method]counter.increment` with the handle as self: a method call
  on a variable assigned a component's result (or on such a call) goes to the component unless it is a warp method
  (library words, list mutation, text builtins). Handles cross back as `own`/`borrow` arguments, also in lists;
  a handle of another resource type or component is refused.
- Not yet: dropping a resource before the process ends, a component's exports as operators, components in the browser (host.js says it runs only in the native host; jco could transpile them),
  `warp build --exe` (refused: foreign_call is a missing import). Capability: Ffi like the other foreign runtimes, though
  a component without preopened files is sandboxed: granting it to eval_untrusted is a later question.
