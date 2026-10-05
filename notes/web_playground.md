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

## The test suite in the browser: `cargo browser-test [filter]`
One configuration (.cargo/config.toml): the alias builds tests/main.rs for wasm32-wasip1 with `--no-default-features`,
and the wasm32-wasip1 runner web/playground/test_in_browser.py serves the repository root plus the binary, opens
web/playground/tests.html in headless Chrome (agent-browser, session warp-browser-tests) and prints a libtest summary
(exit 101 on a failure). `WARP_BROWSER_TEST_WORKERS` (default 2) and `WARP_BROWSER_TEST_PORT` (8733) tune it.
- tests.js lists the tests with libtest's own `--list` (filters, `--ignored`, `--include-ignored` pass through) and runs
  every test in a fresh instance of the compiled binary on test-worker.js workers: wasm panics abort the instance.
- The binary is the whole compiler: `is!`/`eval` compile in wasm and run the program through warp_host (host.js),
  the same path as the playground. wasi.js: args, clocks, random, stdout/stderr, a file system whose "." is the served
  repository (directories via http.server's listing page) with an in-memory overlay for writes, per test.
- First full run (2026-10-03, 2 workers): 1140 passed, 18 failed, 75 ignored (70 #[ignore] + 5 skipped) in ~19 s.
- Blocks known only at run time (`!`, `interpret`, host.js run_block) compile with the test binary itself: it exports
  web_eval_block like warp.wasm, and test-worker.js hands host.js `BLOCK_COMPILER`, which makes a compiler instance of
  it per test. Before (2026-10-05) run_block looked for web/playground/warp.wasm, which a checkout only has after
  build.sh, and test_run_time_blocks + test_block_parameters failed (10 tests).

### What cannot run in the browser, and why
- Not compiled (`#[cfg(feature = "native")]` on their mod lines in tests/main.rs): wasmtime APIs (test_gc_struct,
  test_person_struct, test_struct_types, test_text_getters, test_wasm_reader, test_wasm_emitter, test_utils,
  test_compile_only, test_host, test_footguns (one wasmtime test in it)), network/ureq (test_web), the package tool
  runner (test_package_tools, test_uniscript).
- Skipped by libtest itself: the 5 `#[should_panic]` tests (test_node_operators type mismatches,
  test_newline_precedence::newline_form_is_really_evaluated): with panic=abort libtest ignores them.
- Fail at run time, inherent to a sandbox without processes, temp dir, git or network:
  - spawn the warp binary: test_download (2), test_ffi_warning_once, test_warning_mode::the_strict_flag_turns_warnings_into_errors
  - std::env::temp_dir panics on WASI: test_use_modules (4)
  - lean: test_law::test_law_proved_by_lean, test_law_overflow_promotion_proved_by_lean
  - git clones / tags of packages: test_package_pin (4), test_packages::use_loads_the_module_of_a_package, test_versions (3)
  - threads: test_eval_state::the_hint_mode_of_one_thread_is_not_another_threads; a directory walk below the project
    root: test_use_scopes::use_project_sees_every_file_below_the_project_root (2026-10-04: 1437 passed, 17 failed)
- libc in the browser (host.js `c`, 2026-10-04): rand, srand, abs, labs, strlen, strcmp, strncmp, atoi, atol, atof;
  the shims follow src/ffi.rs signatures: i64 results (size_t, long) are BigInts, strcmp/strncmp get (pointer, length)
  pairs, the others C strings read up to their zero byte (tests/ffi/test_libc_results.rs); anything else of libc still
  throws "c.X is not available in the browser"
- Samples: tests/programs/test_samples_run_cleanly.rs runs every sample but raylib/SDL in both; 2026-10-04 all 64 give
  the same result natively and in the browser
- Ideas: should_panic needs panic=unwind (nightly -Zbuild-std with wasm exception handling); git/lean/process tests could
  get a host import that asks the static server to run them, which defeats the point of the browser run.

## Tasks on Workers (2026-10-04, notes/threads.md step 5)
- host.js startTask: a task runs on a Worker of a pool (task-worker.js) when the page is cross-origin isolated, else at
  once in a fresh instance. The pool is made by worker.js / test-worker.js at start (prepareTaskPool): a Worker made
  while a program runs would never start. test_in_browser.py sends COOP/COEP; index.html registers
  coi-serviceworker.js, which adds them on GitHub Pages (one reload, guarded by sessionStorage "isolating").

## Published: https://warp.pannous.com/ (user request 2026-10-03)
- .github/workflows/pages.yml ("Playground") builds warp.wasm + samples.js with web/playground/build.sh on a push to
  main touching src/, web/playground/, samples/ or the manifest (or `gh workflow run Playground -R pannous/warp`) and
  deploys only the playground files plus CNAME to GitHub Pages. Repository Pages settings: build type workflow, custom
  domain warp.pannous.com, HTTPS enforced. DNS: CNAME warp → pannous.github.io at the registrar (orderbox), added by the
  user; *.pannous.com otherwise points to the pannous.com server.
- The tests page is not published (it needs test_in_browser.py's endpoints and the 38 MB test binary).
- C headers in the browser tests: WARP_INCLUDE=/include, served by name as /__include__/<header> from INCLUDE_DIRS.

## Modules and packages in the browser (2026-10-04)
The compiler reads module files through the page: `warp_host.fetch(address)` / `take_fetched` (web.rs `fetch_text`,
cached per address), a path of the served repository or a URL. A registered package (packages.wasp) is read from its
GitHub raw files at the pinned tag (`raw.githubusercontent.com/<owner>/<repo>/v<version>/…`, served to any page), so
`use uniscript` works; a program's `read(path)` of a URL (the package's data/entities.idx) fetches its bytes as they
are (host.js `readBytes`, no newline added, like the native read). Browser suite: 1452 passed, 15 failed (inherent);
night 2026-10-04 end: 1497 passed, the same 15 failed (downloads, module files, package pins, lean, threads, strict flag).

## Feature parity (2026-10-05, fixer)
Every feature area of 2026-10-04/05 runs in the browser build with its native tests unchanged:
`tests/queue.sh cargo browser-test -- test_unit test_units test_counting_units test_hyperreals test_structural_patterns
test_filter_loops test_type_word_filter_loops test_job_lists test_return_type_dispatch test_type_dispatch
test_trait_runtime_dispatch test_operator_declarations test_operator_precedence test_superscript_operator_declarations
test_mutating_bang test_unwrap test_failed_word test_raise test_try_catch_except test_try_stack_overflow
test_uniscript_entities` → 114 of 114 (natively 115: untrusted_code_may_guard_a_call is native-only). Areas: units
(literals, conversion, products, composites, comparisons, at run time), hyperreals, structural patterns, filter loops,
job lists (three one-second jobs ~1 s with cross-origin isolation), dispatch (parameter, return type, trait runtime),
custom operators, `x!` / failed / raise / try-catch / a caught stack overflow (guarded_call in host.js), entities.
No gaps found. A test that reads a compiled module natively (wasm_reader) must be `#[cfg(feature = "native")]`, else
the browser build of the tests does not compile (test_precomputed, fixed here; test_ffi_warning_once and
the_strict_flag_turns_warnings_into_errors before).
