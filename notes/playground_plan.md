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

## Rules for the two playground workers (user: no core changes, delegate them)
- They change only lib/, web/, samples/, test/, scripts/, notes/: never core/.
- When a sample needs a core change: send warp-supervisor one line (sample, error, the core place if known, a repro),
  park that sample, and go on with the next. The Supervisor hands it to a core worker.
- Load: one background test or sweep run each; the browser sweep runs a few samples at a time.
