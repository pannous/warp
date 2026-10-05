# Run-time `!`: feasibility study (2026-10-05)

Question (wiki/charged.md §5 "Open"): `x!` on a quote whose value is known only at run time (`x : a+b` passed
around, data read at run time, a `block` parameter of a function called with different blocks), inside a compiled
WASM GC program. No implementation yet; this note decides the route.

**Verdict: feasible. Recommended: (b) compile the block at run time with the warp compiler the host already has,
instantiate it next to the program, share GC objects, memory and globals with it (no copying). An in-module
interpreter (a) is a second implementation of the language and is only worth it later as the fast path of a hybrid (c)
for small arithmetic blocks.** Effort to a usable native + browser version: about 10-15 agent-days (table at the end).

## Implemented (2026-10-05, branches run-block-0 … run-block-3)

The first version takes the *copying* road, not the shared-store one below: simpler, and the same on both hosts.

* `interpret e` (the spec's word for `x!!`; src/lowering/run_time_blocks.rs): of a constant block it is `x!!`
  (blocks.rs inlines it); of anything else it is the host word `run_block(e, "a b", [a, b])`, the main-level
  variables assigned before it as a snapshot (the block reads them, never writes them).
* The host (natively src/host.rs `run_block`, in the browser host.js → `web_eval_block` of a second compiler
  instance loaded on first use) reads the three Nodes, binds the names (`pipeline::eval_block`), compiles and runs the
  block in a module of its own, and builds the value back in the program through its constructors (tasks.rs
  `TaskValue`). A failure is the program's error "the block q+1 failed: undefined variable: q".
* The block is pure: no capability but libm (grants: P76, "Later"). Running it is the effect `Eval`.
* `warp compile` warns that a module calling run_block needs a warp host.
* Step 0: `x : a+b` over names defined nowhere warns that `x!` can never run it.

Not yet: leftover suffix `!` marks (blocks-3) forcing at run time per P73; the program's functions inside the block;
exact ratios, big Ints and pairs as results ("cannot hand back yet"); sharing GC objects instead of copying.

## 0. State today (measured with the CLI, debug build of 2026-10-03)

| program | result | spec (charged.md) |
|---|---|---|
| `x : a+b; x` (a, b undefined) | `undefined variable: a` | the quote `a+b` (§4) |
| `a=1;b=2; x : a+b; x!` | `x` (silently the symbol) | 3 (constant quote, §5) |
| `x : a+b; x + 1` | `undefined variable: a` | type error "x is a quote: x! or x = …" |

So not even the compile-time `!` of a constant quote exists; `x!` on a name falls through to the mutation marker of
the parser (`try_parse_evaluate_bang`) and returns the name. That silent result is a bug (todo.md). Run-time `!`
builds on compile-time `!`, which comes first.

**User, 2026-10-05: "What we can do right now is give a compiler warning if the code contains unresolved symbol
arithmetic."** This is step 0 below: when `:` becomes uncharged, a quote that does arithmetic (or calls) over names
that are defined nowhere in the program (`x : a+b` without any a, b) can never be run: not at compile time (names
resolve at the `!` site, there are none) and not at run time (not implemented). A got-it warning at the quote, "x
keeps a+b over the undefined names a, b: `x!` can not run it; define them or write `quote a+b` for symbolic data",
catches typos and the "I expected this to compute" case, and stays silent for literals and plain words (JSON-shaped
data, §4). Where `!` is applied to such a quote, it is the error of §5 already. Cost: half a day in analyzer.rs (it
already knows every defined name: `collect_variables`, `user_functions`).

## 1. The options

What exists that every option uses:

* **The compiler is a library, natively and in the browser.** `pipeline::compile` (src/pipeline.rs) is
  parse → `lower_for_emission` → `emit_module`; `CompiledModule` says which imports it needs. The CLI links it with
  wasmtime (feature `native`); web/playground runs the same crate compiled to wasm32 (warp.wasm: 1.59 MB, 545 KB
  gzipped, in a Worker, worker.js → `web_evaluate`), and the page instantiates the emitted modules through the
  `warp_host` imports (src/web.rs `page`, web/playground/host.js).
* **Compile cost is small**: `warp compile` (release) of `3+4`, `a=3;b=4;a*b+a` and recursive fib takes ≤ 10 ms
  wall time including process start; the modules are 7.4-8 KB (each carries its runtime functions).
* **Values already cross between instances**: tasks.rs (`go f(x)`) runs a function in a *new instance on another
  thread* and copies arguments and result as `TaskValue`, rebuilt through the module's exported constructors
  (`new_int`, `new_text`, `new_key`, `new_list`, `closure_rebuild`). `wasm_reader` / `gc_traits` read a `$Node` into
  a Rust `Node`; `reflection.rs` exports field readers for the browser.
* **A host `run` import exists as a stub**: host.rs links `host.run(wasm_ptr, wasm_len) -> i64` (runs a module that
  returns an i64, in a fresh store; "full GC object return requires more complex setup"). It is the place for
  `run_block`, but its current shape (bytes in linear memory, i64 out) is not what run-time `!` needs.
* **Capabilities**: `EffectReport::of(program).denied(grant)` (effects.rs) refuses a program before it is compiled;
  `eval` grants `[Host, Wasi, Ffi, Libm]`, `eval_untrusted` only `[Libm]`.

### (a) An interpreter over `$Node` inside the module

* **Rust compiled to wasm** does not work for the module itself: Rust targets linear memory and cannot hold or walk
  GC structs; every node would have to be serialised to bytes and back at each `!`. It is option (b) without the
  compiler, plus a second semantics.
* **Emitted as wasm GC by the emitter** (runtime functions via `function_builder::runtime_function`, like
  equality.rs or list_dispatch.rs): a `$Node` tree walker with an environment (`$NodeMap`, map_backend.rs), dispatch
  on the kind, and calls into the existing Node-level runtime functions (big_int, exact, equality, text_builtins).
  Feasible for a closed subset (numbers, texts, arithmetic, comparisons, `if`, field access, calls of program
  functions through Node-uniform adapters in a funcref table) in about 5-8 agent-days and maybe 10-30 KB per module.
* **But the language is not a closed subset.** Its meaning lives in 22 + 22 lowering passes, the analyzer and a
  40 k-line emitter (`src/*.rs`, src/lowering, src/wasm_emitter): units, reals, exact ratios, broadcasting, traits and
  dispatch, closures, comprehensions, tasks, welcoming forms, diagnostics. An interpreter would re-implement each of
  them, or say "not supported in a run-time block" for each. Every later language change would have to be made twice,
  and the two would drift; this is the main risk, not the start.
* Cost per `!`: no compilation, microseconds. Name resolution: an environment lookup per symbol at run time.
* Browser: works unchanged (it is just the module). No compiler needed at run time.

### (b) Compile the block at run time (recommended)

The program imports one host function, only when it contains a run-time `!` (like `needs_host` today):
`warp.run_block(block: (ref $Node), scope: (ref $Node)) -> (ref null $Node)`. The host

1. reads the block into a Rust `Node` (wasm_reader natively; the reflection exports in the browser, as reader.js
   does), or keeps a cache keyed by the block's structure;
2. compiles it with the scope of the `!` site (below) as declarations: one more `pipeline` entry,
   `compile_block(block, scope) -> CompiledModule`, so the block gets the whole language, the same diagnostics, the
   same effect check;
3. instantiates the child module **in the program's store**, linked to the program's exports: its memory, the text
   heap global (`TEXT_HEAP_EXPORT`), the big-number heap, the globals and the functions the block can see;
4. calls it and returns the result `$Node` itself.

**How values cross: by reference, no copying.** Every emitted type is its own singleton rec group
(type_manager.rs: `types.ty().struct_(…)`, no `rec`), so identical definitions in two modules are the same type to
the engine (iso-recursive canonicalisation). A `$Node` built by the child is a `$Node` of the program and vice versa.
Texts are `$String(ptr, len)` into linear memory, so the child must **import the program's memory and text heap**
instead of having its own; same for the `$Numbers` heap global that big Int handles index into. Closures are typed
function references and cross the same way.
*Probe* (probes/runtime_eval/, `node run.mjs` after `wasm-tools parse` of the two .wat): a program module that
owns memory, `$Node` and the global `a` calls `run_block`; the host instantiates a child that imports the memory and
`a`, computes `a+2` and returns a fresh `$Node`; the program `ref.cast`s it to its own `$i64box` and reads 42. V8
(node 26): works, 0.65 ms including the child's compilation. Natively not probed (needs a build): wasmtime documents
the same canonicalisation for modules of one `Engine`, and both instances must live in one `Store` (tasks.rs uses a
store per thread and therefore copies; a run-time block runs on the caller's thread and can share the store).

**User-defined types** (`person{name:text age:int}`) must be declared identically in the child, in the same field
order: the scope metadata carries their definitions and the child's TypeManager emits them first.

**Cost.** Size of the program module: one import, plus no dead-code elimination of the functions and globals a
`!` site can see (below). Startup: none until the first run-time `!`. Per distinct block: one compilation (a few ms
natively incl. cranelift; in the browser the compiler wasm plus V8's Liftoff, low ms), then a cached
`Instance`; per call of a cached block: one host call plus building the scope (O(visible locals)). The browser
playground has the compiler loaded already; a module compiled with `warp compile` and run elsewhere needs a host
that provides `warp.run_block` (the warp CLI natively; in a browser outside the playground, warp.wasm, 545 KB gzipped,
has to ship with it). A module without run-time `!` is unchanged.

**Re-entrancy is the main technical risk.** `run_block` is called while the compiler is on the stack natively
(`eval_program` → wasmtime → host closure → `compile_block`) and in the browser (`web_evaluate` → `warp_host.run` →
JS → back into warp.wasm). The pipeline keeps per-program thread-locals (`diagnostic::begin_program`, the acknowledger,
`take_warnings`, `modules::with_program_file`, web.rs `REPORT`): a nested compile must save and restore them (a
`diagnostic::nested_program(|| …)` scope), and no `RefCell` borrow may be held across the run. In the browser, a
second compiler instance in the worker is the fallback if re-entering proves fragile (memory, not correctness).

### (c) Hybrid

Precompiled paths for the common shapes, (b) as the fallback:

* **Constant quote** (the usual case, §5): compiled at compile time at the `!` site, no run-time machinery. Must
  come first anyway.
* **A `block` parameter called with constant blocks** (`unless(c, body:block)`): specialise the function per
  call site's block (function_values.rs already specialises function arguments at compile time) → constant case.
* **A cache of compiled blocks** keyed by the block's structure: a loop that runs the same run-time block 10 000
  times compiles it once.
* **Later, if profiling asks for it**: the small GC interpreter of (a) for pure arithmetic and comparison over the
  scope (the user's "symbol arithmetic"), falling back to (b) for every other node kind. Only worth it for many
  *distinct* small blocks (spreadsheet-like data), where compile time dominates.

## 2. Names, writes and effects at the `!` site

**Names resolve where `!` is written** (charged.md §5, not hygienic, lisp's eval). At compile time the `!` site's
scope is fully known: parameters, locals, `global`/`nonlocal` variables, the program's functions and types. The
compiler records it per site (types included, as metadata: a custom section `warp.scope`, or a `$Node` constant),
and at run time passes the *values* of the locals and parameters as the `scope` argument (a `$NodeMap` or a
`$Node` list of `name: value` keys, boxed); globals are imported by the child directly. A name the block uses that
is not in the scope is the loud error "undefined variable: b (in the block run at 12:3)" from the child's
compilation, returned as an error `$Node` (a trap with `trap_detail`, like any run-time error).

Lowering renames things (inlining.rs `swap·i·1`, closures lift lambdas, overloads mangle names): the scope maps
*source* names to what holds them; the wasm name section (CLAUDE.md: use all names) is the natural source.

**Writes.** Python's precedent: `exec` inside a function cannot rebind the function's locals. Recommended rule for the
first version: a run-time block **reads** the scope as a snapshot and **cannot assign** a local or parameter of the
`!` site (loud error: "the block assigns x, a local of f: return the value, or declare x global"); it may assign a
variable declared `global` (a wasm global the child imports as mutable) and call functions. Mutation through a
reference (`xs.add(1)` on a list in the scope) works, as it shares the object.

**Effects and capabilities.** The child is compiled by the same pipeline, so `EffectReport` checks it against the
grant. Which grant: a block built from program literals has the program's trust; data read at run time (fetch,
`parse`) is foreign. Recommended default: run-time `!` gets the grant of the program run (`eval`: all, `eval_untrusted`:
libm), and `run(x, grant: [])` / a `pure` declaration narrows it; a taint bit for run-time data is an open decision
(open-decisions question below). Fuel: the child runs in the program's store, so the fuel budget covers it.

## 3. Precomputed paths stay correct (charged.md §3)

A run-time block is an unknown call. What the compiler must keep:

| optimisation | invariant with run-time `!` | cost |
|---|---|---|
| folding constant free variables | a variable is "never reassigned" only if no run-time block can assign it: with the write rule above only `global` variables qualify, and they are already never folded (§3: a reassigned variable must be declared `global` and stays a read) | none beyond §3, *if* blocks can only write declared globals |
| folding constant calls | a function containing a run-time `!` is not [[pure]]: new effect `Eval` (or "unknown") in effects.rs, inherited by its callers; never folded | small: one effect bit |
| hoisting invariant parts | `!` is never hoisted; an expression after a `!` that reads a `global` the block may write is re-read | none |
| memoizing | a function with a run-time `!` (transitively) is never memoized unless the block is run with an empty grant and no global writes (then it is pure) | none |
| specialisation, inlining, dead-code elimination | every function, type and global visible at a run-time `!` site must stay in the module, with its generic signature, and be exported to the child; an inlined or fully specialised function keeps its generic copy | module size; only for programs that contain a run-time `!` |
| local variables in registers (unboxed i64/f64 locals) | boxed into the scope at each `!` call (snapshot) | O(visible locals) per call |
| closures capturing by value (D7) / late binding of globals | consistent: a block sees the current value of a global and the snapshot of the locals | none |

The whole list collapses to two rules for the analyzer: **(1) `!` on a non-constant quote is a call with the effect
`Eval` and the set of globals declared `global` in scope as possibly written; (2) everything visible at such a site is
kept and exported.** Both are local to effects.rs and the dead-code / inlining passes.

## 4. Recommendation and effort

| step | what | agent-days |
|---|---|---|
| 0 | warning: quote over undefined names ("unresolved symbol arithmetic", user 2026-10-05); fix the silent `x!` → `x` | 0.5-1 |
| 1 | compile-time `!` of constant quotes (§5 main case), quote type errors | 2-3 |
| 2 | **minimal run-time `!`, native**: `warp.run_block` import, `pipeline::compile_block(block, scope)`, nested diagnostics scope, child in the same store linked to memory / text heap / globals / functions, scope snapshot of locals, block cache, read-only rule | 4-6 |
| 3 | analyzer invariants: `Eval` effect, keep + export what a `!` site sees, write rule for `global` | 1-2 |
| 4 | browser: `run_block` in host.js, a re-entrant compiler export (or a second compiler instance) | 2-3 |
| 5 | grants: narrowing per `!`, foreign data | 1 |
| later | (c) GC interpreter for small arithmetic blocks, only if profiling asks | 5-8 |
| not recommended | (a) as the only route: a second implementation of the whole language | 20-40, then forever |

**Minimal first step (after 0 and 1):** natively, a program whose only run-time `!` is at top level, reading top-level
variables: `xs = [quote a+1, quote a*2]; a = 5; xs#2!` → 10. That exercises the import, the nested compile, sharing
`$Node` and memory across instances, globals, and the cache, without locals or writes. Test it as an `is!` test.

**Risks:** re-entrant compiler state (save/restore of thread-locals, both hosts); wasmtime's cross-instance GC type
canonicalisation not yet probed natively (V8 probed); user types must be re-declared identically; run-time blocks
from foreign data and their capabilities (security); no DCE for programs with run-time `!`; error positions (a block
compiled at run time reports positions inside the quote, plus the `!` site).

**Open decisions for the Interviewer:** (a) may a run-time block assign the `!` site's locals (recommended: no, only
declared globals); (b) the default grant of a run-time `!` and whether data read at run time is tainted (recommended:
the run's grant, narrowed explicitly; no taint yet); (c) a standalone `warp compile` module with run-time `!` outside
warp's hosts: ship warp.wasm with it, or refuse to compile (recommended: compile, and the missing import is a loud link
error naming `warp.run_block`).
