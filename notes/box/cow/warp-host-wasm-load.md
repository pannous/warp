# warp-host-wasm-load: loading other WebAssembly modules and components from Warp (Phase 1)

Card warp-host-wasm-load (Soon), branch warp-host-wasm-load from origin/warp-host-run-wasm (1aec4ee9f).

## What exists

### The old Rust pipeline (src/, read-only)
- Core modules, notes/wasm_modules.md, src/modules/wasm_modules/:
  - `import fourty_two` (or `use`, `require`, `include`) finds name.warp first, then name.wasm or name.wat. An
    explicit path works too: `import "lib/x.wasm"`.
  - Exports with number types become FFI imports whose import module is the module's path:
    `(import "/abs/fourty_two.wat" "twice" (func (param i64) (result i64)))`. A call converts like a C call.
  - Exported globals read as values (a getter; `set g` is the setter). `fourty_two.twice(21)` is qualified.
    Named arguments come from the name section. C-in-wasm with a header (shout.h) copies `char *` through the module's
    malloc.
  - Linking (linking.rs): every import from a module path is a host function that instantiates the module in the
    run's store at its first call. There is one instance per file per run, and its state lasts the run. The module's
    own imports are linked the same way (WASI, host words, C, other modules by path relative to the cwd).
  - Browser: host.js `moduleImports` does the same.
- Components, src/modules/components.rs and notes/stdlib_connectors.md:
  - `use lib.wasm; lib.f(x)` (long form `use wasm "lib.wasm" as lib`) becomes
    foreign_call("wasm", path, "f", 1, [x]).
  - Each component gets an engine of its own with the component model on (programs' engines are core-only) and
    WASI p2.
  - Values cross by the WIT type through JSON. Records are objects, enums names, variants `{case: payload}`, options
    ø or the value, and a result's err is a warp error. Kebab-case names are found from snake_case. Resources stay
    behind handles.
  - Fixture: tests/fixtures/components/rust_demo.wasm (90 KB, wasm32-wasip2, imports WASI p2).
- The host words: `host.run(ptr, len) -> i64` (src/host/native.rs) passes bytes in linear memory.

### lib/ (pure)
- lib/os.warp: run_wasm(bytes) and compile_wasm(code) through std_io (card warp-host-run-wasm). A whole module runs
  and gives its main's value, with no handle to an instance.
- lib/wasmtime.warp: `use wasmtime`, wasmtime's C API called as a C library (Homebrew libwasmtime), with run(bytes)
  only.
- lib/c.warp: `strlen(s: text): i64 from 'c'`, the foreign declaration lines that card pure-c-imports (branch
  pure-c-imports) teaches the pure emitter to turn into `(import "c" "strlen" …)`.
- lib/compiler/uses.warp: the pure lane follows `use "x.warp"` and keeps std `use os`. Nothing pure handles a
  .wasm `use` or import yet.
- compiler-self reads std modules from the lib/ it was built from, so a new lib/wasm.warp needs a `warp rebuild self`.

### warp-host (crates/warp-runtime/src/warp_host)
- run_wasm (fresh store, own thread), compile_wasm, std_io, and ffi.rs. An import module that isn't host or WASI is
  dlopen'ed as a C library, and a module named "x.wasm" would be tried as a C library and fail.
- The engine has GC, function references and exceptions, with the component model turned off
  (engine.rs deterministic_config). wasmtime's `component-model` feature is already compiled in through the feature
  `compiler`, so it costs no bytes of wasmtime, only our code.

## Design

### Warp API (lib/wasm.warp, `use wasm`)
    m = load_wasm("x.wasm")          // a core module or a component, by its header; or its bytes (read_bytes, compile_wasm)
    call(m, "twice", [21])           // 42; also m.call("twice", [21]) where the compiler resolves it as call(m, …)
    exports(m)                       // the export names, with WIT/core types later
    wasm_call("x.wasm", "twice", [21])  // by path, the module loaded once per run
- The handle is a warp map `{wasm: 1, path: "x.wasm", kind: "module"|"component"}` whose id indexes the run's loaded
  instances, as C handles index CHandles.
- `m.f(args)`, a call through the handle, needs compiler support (a method call on a value holding a wasm handle,
  or `use "x.wasm" as m` lowering `m.f(x)` to call(m, "f", [x]), as the Rust foreign_modules pass does). That's a
  follow-up for the sugar owners.

### Core modules: one store
- A loaded core module is instantiated in the program's own store with the same linker (WASI, host words, std_io, C,
  and its own .wasm imports). One instance per path per run.
- Values by the export's wasm type:
  - i32/i64 take an Int (a Bool as 0/1); f32/f64 take a Float or an Int. Results come back as Int or Float, several
    results as a list, none as ø.
  - Reference parameters and results are the program's own GC refs, passed as they are, not copied. Since modules
    share the store and the engine canonicalizes identical rec groups, a warp module's $Node *is* the caller's
    $Node: GC refs between our own warp modules cross for free. A result ref that isn't a node is refused (an Error),
    so the caller never gets a struct it would trap on.
  - Texts to plain (non-GC) core modules: not in this slice. Later, a copy into the module's malloc/cabi_realloc as
    the Rust C-in-wasm path does.

### Static imports: an import module naming a .wasm file
- `(import "lib/x.wasm" "f" (func …))` (or .wat): warp-host instantiates x.wasm in the same store (recursively
  linked, one instance per file, a cycle is an error) and links its export. This is the old Rust convention: the path
  is relative to the cwd, then to the importing module's folder.
- A bare import module `name` with a `name.wasm` beside the program is the module too, checked before C libraries.
- Compiler side: this is the same import the pure-c-imports emitter makes from `twice(x: i64): i64 from 'x.wasm'`
  (lib/c.warp's line shape). So once that branch lands, a declared signature over a .wasm needs no further compiler
  work. `import x.wasm` with types read from the module at compile time is a follow-up (the compiler must read the
  module's type section).
- The wasm-linker card (static linking in WASP) can merge such imports at build time later. The import convention
  stays the same.

### Components: own engine, WIT types
- A separate engine with the component model on (programs' engines stay core-only, like Rust's components.rs) and a
  store per component kept in the run's Host.
- Values by the declared WIT type, Node to component::Val and back:
  - bool, s8–s64, u8–u64 (range-checked), f32/f64, char (codepoint), string (text)
  - list (list), tuple (list), record (map `{field: value}`, kebab-case from snake_case)
  - option (ø or the value), result (ok value; err becomes an Error), enum (name), variant (name or {case: value}),
    flags (list of names)
  - Resources are refused in this slice.
- WASI p2 imports: wasmtime-wasi is built with p1 only, and p2 costs size. In this slice a component's unknown
  imports are traps (instantiation still works when they're only functions). Follow-up: feature p2.
- use-component-registry (load by package name) can map `wasi:foo/bar@1` to a path and then call load_wasm.

## Smallest useful slice (this branch)
1. Host: wasm.load / wasm.call / wasm.exports through std_io, core modules in the program's store. Host only.
2. Host: imports from a module named x.wasm/x.wat linked to that module's exports. Host only, testable with
   hand-written WAT like the C tests.
3. Host: components with scalars, strings, lists, records, options, results, enums.
4. lib/wasm.warp: the words (lib only; `use wasm` needs a rebuild of compiler-self).
Compiler support needed later: `m.f(x)` sugar, `import x.wasm` with types read at compile time, and emitting
foreign `from 'x.wasm'` imports (pure-c-imports).

## Status (2026-10-23 ~01:55 UTC+2, branch tip see git)
- Done: core modules (load_wasm/wasm_call/wasm_exports), import resolution of .wasm/.wat module names, components with
  WIT scalars/strings/lists/records/options/results/enums/variants/flags; run_wasm(bytes, args); run_wasm's uncaught
  Warp Error passed through vs a trap.
- Size (profile host): 6,510,696 → 6,674,280 (core) → 7,351,304 B (with components), +841 KB.
- Tests: host suite 64 cases; 64/64 green with `use wasm`/`run_wasm(b, a)` inlined as std_io definitions; 51/64 with the
  shipped words because no compiler-self could be rebuilt from this lib/ (warp rebuild OOM-killed 3× on the loaded
  box). Launcher 22/22.
- Needed: `warp rebuild self` on this branch (Mac slot), then rerun tests/host/test_host.sh.
