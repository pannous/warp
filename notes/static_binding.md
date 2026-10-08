# Static compilation vs dynamic run-time binding (card static-binding, 2026-10-08)

Question (user): should std modules be linked at compile time or bound at run time, especially in the browser? "We
might just include the source files."

## How it is today: static, at the source level
- The 20 std modules (lib/*.warp) are embedded in the warp binary (modules.rs STD_MODULES, include_str!). They are
  embedded in the browser compiler warp.wasm too, so nothing is fetched.
- `use regex` (or an implicit use: a page brings markup, routes bring router) splices in **only the definitions the
  program reaches** (modules.rs with_needed_definitions). dead_functions.rs then drops every wasm function nothing
  reaches.
- Words warp cannot write itself go through host words: std_pure/std_io are implemented in Rust natively and in host.js
  in the browser. A site ships only the host parts its imports name (site.rs HOST_PARTS).
- Already bound at run time:
  - `import "x.wasm"` core modules: wasmtime links them natively, and the browser instantiates them lazily on first call;
  - components (jco, loaded on first call);
  - `use c`, through libc.wasm;
  - foreign_call (JS, Pyodide);
  - a site's per-route modules (wasm-split, app-route-N.wasm, instantiated against app.wasm's exports).

## The options
| | static (today) | std as one shared wasm module | ship sources, compile in the page |
|---|---|---|---|
| hello-world site | 12.5 KB gz app.wasm, only what is used | the whole std at least once (all of markup, list, text… ≈ every function) | warp.wasm 646 KB gz + lib sources 14 KB gz |
| types across the boundary | whole program: inference, specialisation, inlining, bool/int analysis | an export boundary: `any`/Node in and out, no specialisation | whole program (it compiles everything) |
| caching across pages / apps | none, each app.wasm carries its copy | the browser caches std.wasm once per origin | the compiler is cached once, programs are tiny |
| startup | instantiate one module | instantiate and link two (GC types must be identical rec groups to match) | compile in the page first (the playground's path: noticeable on phones) |
| version skew | impossible, one artefact | std.wasm and app.wasm must come from the same warp | the page's compiler is the version |
| eval / `run_block` / live editing | needs the compiler anyway (host-compiler.js) | same | free: the compiler is there |

## Recommendation (an undoable default)
1. **Keep static linking as the default** for built sites and native programs. Whole-program compilation is what makes
   warp's types, tree shaking and the 23 KB bundle budget work. A shared std module would trade that for a cache win
   that only multi-app origins see.
2. **Dynamic binding stays where code really is not known at build time**, and it already is: foreign modules,
   components, `use c`, route modules (lazy, but still from one build), and code typed at run time (`run_block`, the
   playground).
3. **"Just include the source files"** is the right model for the **playground and dev pages**, which already work
   this way: warp.wasm embeds lib/ and compiles in the browser. For deployed sites the compiler costs ~50× the hello
   world, so only a page that evaluates user code at run time should carry it. That page already does, through
   host-compiler.js, which is loaded only when the module imports run_block.
4. **A middle step, if cross-app caching ever matters:** content-addressed per-module chunks. Split std functions out
   with the same wasm-split machinery as routes (route_split.rs), named by hash, so two apps built by the same warp
   share files. This keeps whole-program types inside each app. It is not worth doing before a real multi-app user
   exists.

## Open, for the Interviewer only if the user wants to pursue it
- Should a site be able to opt into a shared std (`warp build --site --shared-std`)? Default: no.
