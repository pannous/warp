# Browser playground (web/playground)

Run it: `web/playground/build.sh && python3 -m http.server 8000` in the repository root, then open
http://localhost:8000/web/playground/ (`?example=<name>` picks a tour example or a samples/ file).
Probe: `probes/web_playground.py [sample…]` (headless agent-browser; compares every sample's value with the CLI's).

## The tour (examples.js)
In the order of the guide's chapters (card playground-redesign, 2026-10-07): small examples with one idea each and an
obvious result first (hello, numbers, text, lists, data, conditions, loops), then functions, signals, events, markup,
WebAssembly. Each entry is `{value, printed?, wait?, code}`, its first code line a plain `//` caption.
`web/playground/test_in_browser.py --examples [name…]` (after build.sh) shows each in the page and compares value and
printed text; pages.yml runs it before deploying. The example menu is grouped by guide chapter (guide.js
groupExamplesByChapter: each chapter's Examples line), the samples no chapter links under "more samples".

## Page layout (index.html, playground.css)
guide | workspace (toolbar, editor, output stacked); ≤900px everything stacks and the guide starts closed. Two drag
handles (playground.js dragToResize) set the guide's width and the editor's height, remembered in localStorage
(warp-playground-sizes), double-click resets, arrow keys when focused. The developer things (native install, ⤓ wasm,
build switch, silenced hints) sit in the header's ⋯ menu; the run time is a small note in the output's corner.

## The language guide (guide.md, guide.js)
The left pane of the page (cards "core feature", "doc-example"; P188 one page): web/playground/guide.md, chapters `## Title` from
easy to advanced, one open at a time (anchor `#<title-in-kebab-case>`). A fence ```` ```wasp => <value> ```` is a snippet
with a `try ▶` button (window.playground.runCode) and the value the page shows for it; an `Examples: a, "b c"; samples:
x, y` line links tour examples and samples/ files, and choosing one of those shows a toolbar back-link "guide: <chapter>".
tests/web/test_guide.rs checks every fence's value through warp::web::evaluate (the page's own report) and every linked
example. Write snippets in canonical style (`^`, `\(name)`, `xs#1`, `if … then … else`): the page shows hints otherwise.
Markup values show as wasp data there (`p{class:"note" "hello"}`), not as HTML like the CLI.

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
(exit 101 on a failure). `WARP_BROWSER_TEST_WORKERS` (default 2) and `WARP_BROWSER_TEST_PORT` (default: a free port per run; a taken one fails naming its PID) tune it.
  `WARP_BROWSER_TEST_PER_WORKER` (tests.js TESTS_PER_WORKER, 100) is how many tests a worker runs before it is replaced.
- Wasm memory (card browser-memory, 2026-10-06): Chrome holds ~124 live Wasm memories per page, all Workers together,
  and an isolate that runs out collects only its own dead instances (probes/wasm_memory_limit.html), so another
  worker's garbage failed plain tests ("WebAssembly.Instance(): Out of memory"). Besides the replacement every 100
  tests, a test that runs out makes the page replace every worker (terminating an isolate frees its memories at once),
  re-send the tests in flight and run it once more; the runner prints "Wasm memory ran out N times …" naming them.
  Proof: 4 workers without replacement (WARP_BROWSER_TEST_PER_WORKER=100000) failed 2 tests that way before, none after.
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
- Ignored in the browser build only (user decision P79, `#[cfg_attr(not(feature = "native"), ignore = "browser: …")]`):
  git clones / tags of packages (test_package_pin 3), Lean (test_law 2), threads (test_eval_state 1).
- Made to run in the browser (P79, 2026-10-05):
  - downloads: tests/common `serve` is test_in_browser.py's `/__stub__?status=…&body=…` there (URL in WARP_HTTP_STUB)
  - scratch files: tests/common `scratch_directory` is below /tmp, an overlay-only preopen of wasi.js (TMPDIR), since
    std::env::temp_dir and std::process::id panic on WASI; web.rs fetch_text reads module files through the WASI file
    system first in the tests build, so modules a test writes are found
  - directory walks: wasi.js asks `<dir>/?listing`, which test_in_browser.py lists even when the directory holds an
    index.html (web/uniscript did: its page's links read as entries, `use project` failed)
- Need packages/ cloned by a native run first (a fresh worktree has none): test_packages::a_package_is_fetched_once_into_packages,
  test_text_bytes::read_loads_a_file_as_bytes
- libc in the browser (P147, 2026-10-06): host.js `c` calls web/playground/lib/libc.wasm, wasi-libc built by
  lib/build_libc.sh (21 KB, deployed with the page: pages.yml SITE_FILES `lib`), one instance per worker. Its exports
  have the native FFI's types (lib/libc.c wraps strlen, strncmp, strspn, strcspn, atol, labs to 64 bits), so numbers
  pass through; texts cross by the program's custom section warp.c_calls (src/wasm_modules.rs c_calls: `t` a
  NUL-terminated text, `l` the length after strcmp's text, `n` a number; result `t` a text): copied into libc.wasm's
  malloc and freed after the call, a char * result read back (NULL is ø). Functions: lib/libc.h; anything else of libc
  throws "c.X is not available in the browser". The hand-written shims of 2026-10-04 are gone.
- Samples: tests/programs/test_samples_run_cleanly.rs runs every sample but raylib/SDL in both; 2026-10-04 all 64 give
  the same result natively and in the browser
- Ideas: should_panic needs panic=unwind (nightly -Zbuild-std with wasm exception handling); git/lean/process tests could
  get a host import that asks the static server to run them, which defeats the point of the browser run.

## Tasks on Workers (2026-10-04, notes/threads.md step 5)
- host.js startTask: a task runs on a Worker of a pool (task-worker.js) when the page is cross-origin isolated, else at
  once in a fresh instance. The pool is made by worker.js / test-worker.js at start (prepareTaskPool): a Worker made
  while a program runs would never start. test_in_browser.py sends COOP/COEP; index.html registers
  coi-serviceworker.js, which adds them on GitHub Pages (one reload, guarded by sessionStorage "isolating").
- card coi-headless (2026-10-08): the reload does happen, on warp.pannous.com and a plain local server, in a fresh
  profile of headless Chrome (agent-browser) and of WebKit 26 (Playwright, probes/webkit_samples.mjs, 3 of 3). Without
  isolation (service worker blocked: ISOLATED=no in that probe, a private window) tasks take turns: `tasks` prints out
  of order, samples/threads.wasp and async.wasp hang. No fallback can make them parallel without shared memory (the
  program blocks on a task's result with Atomics.wait; only JSPI could suspend instead, not in Safari), so the run says
  why: host-tasks.js TASKS_INLINE as a warning, and the page's 10 s timeout names it (worker message "tasks inline").

## Published: https://warp.pannous.com/ (user request 2026-10-03)
- .github/workflows/pages.yml ("Playground") builds warp.wasm + samples.js with web/playground/build.sh on a push to
  main touching src/, web/playground/, samples/ or the manifest (or `gh workflow run Playground -R pannous/warp`) and
  deploys only the playground files plus CNAME to GitHub Pages. Repository Pages settings: build type workflow, custom
  domain warp.pannous.com, HTTPS enforced. DNS: CNAME warp → pannous.github.io at the registrar (orderbox), added by the
  user; *.pannous.com otherwise points to the pannous.com server.
- The tests page is not published (it needs test_in_browser.py's endpoints and the 38 MB test binary).
- C headers in the browser tests: WARP_INCLUDE=/include, served by name as /__include__/<header> from INCLUDE_DIRS.

## A clean console, checked (card console-errors, user 2026-10-08)
- The red lines were the compiler's searches: `use math` asks ., include, lib, lib/extra, src, source, samples for
  math.wasp/.warp before the embedded module, a C header search asks lib/host.h…; every miss was a 404 in the console
  (Firefox shows worker XHRs; ~46 per sample). Now build.sh writes served-files.js (the server's files: git ls-files
  locally, `build.sh served _site` for the deploy) and host-files.js getSync answers an unlisted path "missing" without
  asking. Also: an inline 🐝 favicon (favicon.ico 404), CodeMirror fixedGutter off (Firefox's scroll-linked warning).
- index.html reloaded under coi-serviceworker.js before the worker was active: the page came back unisolated and the
  `isolating` flag stopped retries (headless Chrome: no shared memory, tasks/mouse/kitchen sink failed live). It now
  waits for serviceWorker.ready; an isolated page clears the flag (a hard reload bypasses the worker).
- Gate: `test_in_browser.py --examples [--site D | --url U]` runs the tour and every sample and fails on any console
  error/warning/failed request of the page and its Workers. agent-browser's `console`/`network` see only the page, so
  console_watch.mjs attaches over CDP (`agent-browser get cdp-url`) to every target: Runtime + Log + Network (Chrome's
  Log omits a worker's failed request). pages.yml runs it on _site before deploy and on the live page after (job verify).
- Not ours: `TypeError … shiftKey … inject.js` in the user's Firefox is a browser extension's content script.
- Open: the gate in headless Firefox (card console-errors-step).

## Modules and packages in the browser (2026-10-04)
The compiler reads files through the page: `warp_host.fetch(address)` / `take_fetched` (web.rs `read_bytes`, cached
per address; `read_text`, `file_exists` on top, the file system outside the page), a path of the served repository or a
URL: module sources, C modules (`import tests/fixtures/wasm/zlib`: zlib.wasm and the zlib.h beside it), C headers. The
page's own files go by `page:` (web.rs PAGE_PREFIX, host.js resolves it against the worker's URL, since the page is
web/playground/ locally but the site root when deployed): `use c` reads page:lib/libc.h (ffi_parser PAGE_INCLUDE), no
WARP_INCLUDE (2026-10-06, card use-import; before, the page had no headers: `strlen("hello")` gave 0). The deploy
copies the tour's C module to _site/tests/fixtures/wasm/ (pages.yml), tour example "C libraries". A registered package (packages.wasp) is read from its
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

## Foreign runtimes in the page (host.js registerForeignRuntime)
- foreign_call (src/foreign.rs natively) dispatches by runtime name to `registerForeignRuntime(name, {call, prepare})`:
  `call(module, member, arguments, hooks)` is synchronous and gives a plain value (arguments null: a read);
  `prepare(code)` is optional and asynchronous, awaited by worker.js (`prepareForeignRuntimes`) before a run, for a
  runtime that must load first (Pyodide). The test worker cannot prepare (it never sees the code), so runtimes the
  browser tests use load synchronously. Registered: `js` (the page's globals, host.js), `wasm` (components.js).
- `use lib.wasm` / `use wasm "lib.wasm"` (components.js): build.sh components runs `jco transpile --instantiation sync` on every
  tests/fixtures/components/*.wasm and wraps the result into components/<name>.js, a classic script with the core
  modules as base64 (`registerComponent`); the first call importScripts it, found by the file's name alone (the page has
  one flat folder of components). WASI p2 is a small shim: output goes to the program's print, no input, no environment.
  test_in_browser.py runs build.sh components before serving; pages.yml installs jco and ships components/.
- jco's JavaScript values are turned into the native JSON forms by the WIT types, which build.sh embeds as the
  signatures of the exports (`wasm-tools component wit --json`): camelCase ↔ the WIT's kebab-case names, `{tag, val}` →
  `{case: payload}`, flags objects → lists of names, a char → `{$char: c}`, a Codepoint (P94, as natively), a thrown
  result `err` → the error, class instances → handles with ids per component, the argument count with the parameter
  names. Still jco's own: the enum check's message ("\"triangle\" is not one of the cases of shape"), and a variant
  argument (an object) is not converted.
  tests/ffi/test_components_anywhere.rs runs in both hosts.

## paint: a canvas in the page (2026-10-06, issue #15)
`paint(pixels, width, height)` is a host word (src/host.rs PAINT): host.js reads the pixel list and hands it to the
worker's hooks.paint, the page draws one canvas per call under the output (playground.js showPaintings: nonzero/true
is ink, 0 paper). Natively it writes a grayscale PNG to <temp>/warp-paint/paint.png (src/paint.rs, flate2 + crc32fast), prints its path and opens it on a terminal. samples/circle.wasp is the issue's demo as
written (one loop moving x and y together, so it paints only a short diagonal), samples/filled_circle.wasp the filled
circle with two loops.

### Wasm memory limit across Workers (card browser-test, 2026-10-06)
Chrome holds ~124 live Wasm memories per page, all its Workers together (V8's sandbox: each 32-bit memory reserves
~8 GB of a 1 TB cage, whatever its size or declared maximum; Node without the sandbox ~16300). An isolate whose
allocation fails collects its own dead instances and retries, never another isolate's: a task Worker failed with
"Out of memory: Cannot allocate Wasm memory for new instance" while the test worker (or the program's worker) held
dead instances. Measured with probes/wasm_memory_limit.html (Chrome) and probes/wasm_memory_limit.sh (Node).
Fix: host.js finishedTask runs a task its Worker could not instantiate (`unstarted`) inline, in the starting isolate,
whose failed allocation frees its own garbage. Still possible: the other test worker's garbage filling the page
(TESTS_PER_WORKER recycling bounds it). Not usable: `--js-flags=--expose-gc` through agent-browser `--args` (the tab
ends on about:blank), a declared memory maximum (still ~8 GB reserved).

## First markup run (card guide-warmup, 2026-10-08)
- The first markup program a worker compiles pays a one-time cost (the markup renderer, lib/markup.wasp's to_html):
  live, a fresh page measured 470–560 ms in headless Chrome and 0.6–4.4 s in WebKit 26, later markup runs ~80–120 ms,
  other programs none. After a run is shown the page asks its worker once to warm up (playground.js warmWorker →
  worker.js warmUp: `p{ "" }` evaluated with the worker's messages silenced, skipped over a live run). With it the
  first markup run was ~80 ms in Chrome and ~97 ms in WebKit (local build). Measured with a Playwright probe timing
  window.playground.evaluate in a fresh context (probes/*.mjs are not tracked; agent-browser eval works the same).
