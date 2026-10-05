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
- Next: `use js "…"` with node/deno on the same protocol and globalThis in the browser host; the C gaps; handles.
