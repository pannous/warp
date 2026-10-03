# Browser playground (web/playground)

Run it: `web/playground/build.sh && python3 -m http.server 8000` in the repository root, then open
http://localhost:8000/web/playground/ (`?example=<name>` picks a tour example or a samples/ file).
Probe: `probes/web_playground.py [sample…]` (headless agent-browser; compares every sample's value with the CLI's).

## Architecture
- The compiler itself runs in the browser: `cargo rustc --lib --crate-type cdylib --target wasm32-unknown-unknown
  --no-default-features --release` (build.sh, 8 MB stack, wasm-opt -Oz → ~1.7 MB). Plain C ABI, no wasm-bindgen:
  exports `web_alloc/web_free/web_evaluate/web_report`, imports `warp_host.run/take/now_ms/panicked`.
- `native` (default feature) carries wasmtime, wasmtime-wasi, ureq, rustyline, libloading. Without it `eval` is the same
  pipeline (laws, lowering, Asks, explain_runtime_error); only `run_module` differs: src/web.rs `run_in_host` gives the
  bytes to the page and reads back a JSON outcome.
- worker.js runs the compiler AND the compiled programs in a Web Worker: synchronous `new WebAssembly.Module` is
  limited to 4 KB on the main thread, and host calls (fetch, read) are synchronous XHR, allowed only in workers.
  A stuck program: the page terminates the worker after 10 s and starts a new one.
- Reading a GC result: JavaScript cannot read struct fields, so modules built without `native` export 13 `reflect_*`
  getters (src/wasm_emitter/reflection.rs, EmitterConfig.emit_reflection). reader.js walks the result into
  `{kind, data, chain}` (chain = the value-field cells, flat, so long lists need no recursion), web::node_from_tree
  mirrors Node::from_gc_object. Floats travel as text when JSON cannot hold them (NaN, ±Infinity, -0).
- Traps: the JS error's stack names the wasm functions (name section), plus the `trap_detail` global read through the
  reflection getters → wasm_emitter::trap_error, the same reading wasmtime's trace gets (failed_run).
- Asks: web::evaluate runs with a PageAsker: answers the page sent are taken (and hinted, as on the terminal), every
  other Ask is reported with its readings and falls back. Clicking a reading stores `topic = explicit form` in
  localStorage and runs again. educate_once notes get a "got it" button (`ack:<topic> = acknowledged`).
- Program imports mirror the wasmtime runner: host.fetch/fetch_within/read/warn (read fetches relative to the repo
  root), wasi fd_write → printed output, ffi module "m" → Math; any other import throws "X is not available in the
  browser" when called.

## From the C++ version (~/wasp/docs, wasp.pannous.com)
- Reused: CodeMirror 5 + simple mode (copied to web/playground/codemirror/), the wasp syntax mode (rewritten with word
  boundaries and warp's comment rules), example menu + `?example=` URL parameter, run-while-typing, download wasm.
- Not taken: the C++ runtime's linear-memory node ABI (string/array headers, smartResult), binaryen wasm→wat,
  deploy-to-lambda, the canvas/DOM externref bridge (`$canvas.getContext`) — candidates for later host imports.

## Found on the way
- `return x` was typed through a bogus libm "function" named `return` (the header parser read math.h inline bodies);
  the browser has no headers and typed it as a list. Fixed in analyzer infer_type + ffi.rs.
- Copies of warp share target/debug/warp: a probe must copy its binary right after building it.
