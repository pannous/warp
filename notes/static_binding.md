# Static compilation vs dynamic run-time binding

Std modules are linked statically, at the source level; dynamic binding only where code is not known at build time.

## How it works
- The std modules (lib/*.warp) are embedded in the warp binary (modules.rs STD_MODULES, include_str!) and in the
  browser compiler warp.wasm, so nothing is fetched.
- `use regex` (or an implicit use: a page brings markup, routes bring router) splices in only the definitions the
  program reaches (modules.rs with_needed_definitions); dead_functions.rs then drops every wasm function nothing
  reaches.
- Words warp cannot write itself are host words: std_pure/std_io in Rust natively and in host.js in the browser. A
  site ships only the host parts its imports name (site.rs HOST_PARTS).
- Bound at run time: `import "x.wasm"` core modules (wasmtime natively, lazily on first call in the browser),
  components (jco, on first call), `use c` through libc.wasm, foreign_call (JS, Pyodide), a site's per-route modules
  (wasm-split, app-route-N.wasm against app.wasm's exports), and code typed at run time (`run_block`, the playground,
  through host-compiler.js, loaded only when the module imports run_block).

## Why static
| | static (default) | std as one shared wasm module | ship sources, compile in the page |
|---|---|---|---|
| hello-world site | 12.5 KB gz app.wasm, only what is used | the whole std at least once | warp.wasm 646 KB gz + lib sources 14 KB gz |
| types across the boundary | whole program: inference, specialisation, inlining | an export boundary: `any`/Node in and out | whole program |
| caching across pages / apps | none, each app.wasm carries its copy | std.wasm cached once per origin | the compiler cached once |
| startup | one module | two, linked (GC types must be identical rec groups) | compile in the page first (noticeable on phones) |
| version skew | impossible | std.wasm and app.wasm must come from the same warp | none |

Whole-program compilation is what makes warp's types, tree shaking and the bundle budget work; a shared std would
trade that for a cache win only multi-app origins see. The playground and dev pages already "just include the source
files" (warp.wasm embeds lib/ and compiles in the browser); for deployed sites the compiler costs ~50× the hello world.
If cross-app caching ever matters: content-addressed per-module chunks, std functions split out with the route
machinery (route_split.rs) and named by hash, so two apps built by the same warp share files. Not before a real
multi-app user exists; a `--shared-std` opt-in is not planned.
