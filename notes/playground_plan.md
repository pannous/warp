# Today's plan (user 2026-11-01): every sample runs in the browser playground

Goal: each samples/**/*.warp runs in web/playground with the output the native warp gives (test/samples.txt).
This supersedes decision playground-after-parity (the user: "the bigger local issues appear resolved").

## Where it stands
- web/playground still serves the Rust-era compiler (warp.wasm of 2026-10-21; build.sh builds src/ with cargo, gone).
- The native warp runs the self-hosted compiler warp.wasm (core/compiler.warp) through core/native/driver.warp, which
  answers its host words (54 `warp_wasmtime` imports, WASI, files, modules, FFI) over libc.
- Cards: wasm-playground-build, playground-compiles (both say: host warp.wasm in JavaScript).

## Steps
1. Host the self-hosted compiler in the page: warp.wasm (the installed one, beside ~/dev/bin/warp) compiles in a
   worker; JS answers the host words driver.warp answers natively (files from the served repo/lib, `use` of lib
   modules, WASI, time, random). build.sh: no cargo, it copies the installed compilers and lib/. Check:
   hello.warp prints `Hello 🌍2026` in the page.
2. A headless sweep, scripts/samples-web.sh (agent-browser, beside scripts/samples.sh): every sample in the page,
   its output against the native run; writes test/samples-web.txt (status and sample, one a line, tracked).
3. Fix the gaps by host area, lib/ and web/playground/ first:
   - warp-web-host: compiler hosting, files, modules, tasks, timers, events, storage, servers, foreign/python.
   - warp-web-samples: the sweep, then graphics (canvas, gpu), sound, media, markup.
   excluded_samples.txt keeps only samples that can't run in a browser (raylib/SDL windows, processes), each with why.

## Host areas, state (warp-web-host, 2026-11-01)
- Step 1 done: web/playground/driver.js is driver.warp's twin (warp.wasm over wasi.js; the compiler reads the
  tree's lib/ through WARP_LIB; build.sh overlays it on the installed lib).
- Servers: a program with routes serves the page's requests (lib/web.warp page_serve under WARP_SERVE=page,
  host-routes.js web.request/answer over shared memory; the address bar, links and forms are the client).
  `route "…"`, `server def` and `form post` are warp-speed's (card server-routes).
- Storage: host-sqlite.js answers lib/native/sqlite3.warp with SQLite's own wasm build, without its JS glue
  (instantiated synchronously, env/WASI stubs; its own VFS has no files, so a file database opens in memory, is
  deserialized from the page's files and serialized back on close and at the end of the run). Any C library in a wasm
  module goes through host-foreign.js wasmLibraryImports and typedExports (arguments fitted to the wasm signature).
- FFI and Python: callC follows driver.warp c_called (warp.c_functions section; only the first out-pointer gets a
  slot); `use python` runs in Pyodide through python_capi.py, the C API by handle.
- Samples that fail natively fail identically in the page. These are core or language gaps, reported to the supervisor:
  a linked storage.wasm can't call type(instance); the word index misses rule heads such as `stored`; plus `where`
  with bare fields, `stop`, `load`, and `go` in an expression.

## Rules for the two playground workers (user: no core changes, delegate them)
- They change only lib/, web/, samples/, test/, scripts/, notes/: never core/.
- When a sample needs a core change: send warp-supervisor one line (sample, error, the core place if known, a repro),
  park that sample, and go on with the next. The Supervisor hands it to a core worker.
- Load: one background test or sweep run each; the browser sweep runs a few samples at a time.
