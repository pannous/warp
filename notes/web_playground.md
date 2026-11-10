# Browser playground (web/playground)

Roadmap card: playground (board topic).

Run it: `web/playground/build.sh && python3 -m http.server 8000` in the repository root, then open
http://localhost:8000/web/playground/ (`?example=<name>` picks a tour example or a samples/ file).
Sweep: `scripts/samples-web.sh [<tree>] [samples/….warp…]` (headless agent-browser; test_in_browser.py --samples)
runs every samples/**/*.warp in the page as its own file (`playground.evaluate(code, file)`, so `use` and file reads
resolve beside it) and compares what the page printed with what the native run printed (scripts/samples.sh,
data/samples/out; the main checkout's when the tree has none). test/samples-web.txt (tracked): ok, DIFF, FAIL, TIME
(the page gave no answer: it stops a run after 30 s, ?run_seconds, as samples.sh does natively), ran (the native run failed: nothing to compare), WIN (needs a C window, not run);
data/samples-web/report.txt each difference and error. One page, one sample at a time: ~2 minutes for all.

## The tour (examples.js)
In the order of the guide's chapters: small examples with one idea each and an obvious result first (hello, numbers,
text, lists, data, conditions, loops), then functions, signals, events, markup, WebAssembly. Each entry is
`{value, printed?, wait?, code}`, its first code line a plain `//` caption.
`web/playground/test_in_browser.py --examples [name…]` (after build.sh) shows each in the page and compares value and
printed text; pages.yml runs it before deploying. The example menu is grouped by guide chapter (guide.js
groupExamplesByChapter: each chapter's Examples line), the samples no chapter links under "more samples".

## Page layout (index.html, playground.css)
guide | workspace (toolbar, editor, output stacked); ≤900px everything stacks and the guide starts closed. Two drag
handles (playground.js dragToResize) set the guide's width and the editor's height, remembered in localStorage
(warp-playground-sizes), double-click resets, arrow keys when focused. The developer things (native install, ⤓ wasm,
build switch, silenced hints) sit in the header's ⋯ menu; the run time is a small note in the output's corner.

- The assistant (assistant.js, lib/agent.warp): the API key field in the ⋯ menu, completion (Ctrl-Space), Ask ✦ chat.
- Completion as you type (completion.js, like the Sublime packages Warp and Uniscript): words from the
  second letter (the words of the whole code first, then keywords, then examples and samples, by use), the standard modules after
  `use` (keywords.js `modules`, written by keywords.py), uniscript entities after `\:` and `<:` (entities.tsv, a copy of
  core/entities.tsv fetched on first use) which become their character; a typed `<:name>` becomes it at `>`.
  Tab and Enter take the chosen one (the first unless Up/Down chose another); Enter after a word that is whole
  already (a keyword, a name of the code) is a new line. Tab after a word with no list open lists its completions from
  the first letter, or takes the only one; Claude's gray continuation goes before both. Claude reads the whole program
  (up to 20000 characters around the cursor) and also continues before closing brackets (`dir(m‸)`).
  The tour's "(completion with Tab and Enter)" check runs the keys in CI.
- Keyboard shortcuts (shortcuts.js SHORTCUTS): one table of CodeMirror key names, the editor's extraKeys.
  Cmd/Ctrl-Enter runs; Cmd-/ (Ctrl-/ elsewhere) toggles `// ` on the selected lines or the cursor's line (after their
  shallowest indentation; all commented: uncommented), one undo step, the selection kept. Cmd-Ctrl-Up/Down
  (Alt-Up/Down elsewhere) move those lines one up or down with the selection.
- ⛶: a small button over the paintings' corner (index.html #painted, playground.js toggleFullScreen) puts them full
  screen: the last painting only, as large as fits at its own aspect ratio (CSS --aspect set by showPaintings, so the
  pointer mapping of clickDetail stays exact), black around it; Esc or ⛶ again leaves. Its click is no click of the
  program's `on click`.

## The language guide (guide.md, guide.js)
The left pane of the page (P188 one page): web/playground/guide.md, chapters `## Title` from
easy to advanced, one open at a time (anchor `#<title-in-kebab-case>`). A fence ```` ```warp => <value> ```` is a snippet
with a `try ▶` button (window.playground.runCode) and the value the page shows for it; an `Examples: a, "b c"; samples:
x, y` line links tour examples and samples/ files, and choosing one of those shows a toolbar back-link "guide: <chapter>".
`test_in_browser.py --examples --guide` checks every fence's value and printed lines in the page. Write snippets in canonical style (`^`, `\(name)`, `xs#1`, `if … then … else`): the page shows hints otherwise.
Explanatory prose for a snippet belongs immediately before its fence, while the fence keeps the executable expected value.
Markup values show as warp data there (`p{class:"note" "hello"}`), not as HTML like the CLI.

## Architecture
- The compiler itself runs in the browser: build.sh copies the installed warp.wasm and its lib/ into warp-lib/ (slimmed
  by wasm-opt to the exports the page calls); driver.js plays core/native/driver.warp: warp.wasm compiles the program
  over WASI (wasi.js: the served repository read with synchronous GETs, writes in an overlay), then the module it wrote
  runs with the program's host words (host.js) and the same WASI.
- worker.js runs the compiler AND the compiled programs in a Web Worker: synchronous `new WebAssembly.Module` is
  limited to 4 KB on the main thread, and host calls (fetch, read) are synchronous XHR, allowed only in workers.
  A stuck program: the page terminates the worker after 10 s and starts a new one.
- Worker start: worker.js streams warp.wasm and posts its progress (`#loading` beside #status: "downloading the
  compiler: N MB so far", "compiling warp.wasm (N bytes)", "instantiating warp.wasm", "; no progress for 30 s" after
  LOADING_STALL_MS); #status stays exactly "running…" meanwhile (test_in_browser.py and probes wait on it).
  playground.js rejects workerReady on the Worker's error event, restarts a start silent for STALLED_START_MS (60 s,
  every message restarts the clock) once with a console warning naming its stage, and a second silent start fails the
  run naming the stage.
- Reading a GC result: JavaScript cannot read struct fields, so a host word reads values through the module's exports
  (values.js, below).
- Traps: the JS error's stack names the wasm functions (name section), plus the `trap_detail` global.
- Ambiguities: a warning names its explicit form; each reading is an "I meant: …" button that rewrites the code at the
  warning and runs it again; "got it" silences a warning's topic in this browser until "show again" (playground.js).
- Program imports mirror the native warp's host words: host.js and its host-*.js parts, wasi fd_write → printed output;
  any other import throws "X is not available in the browser" when called.
- A clean console: build.sh writes served-files.js (the server's files: git ls-files locally, `build.sh served _site`
  for the deploy) and host-files.js getSync answers an unlisted path "missing" without asking, so the compiler's module
  and header searches cause no 404s. Inline 🐝 favicon, CodeMirror fixedGutter off (Firefox's scroll-linked warning).
- From the C++ version (~/wasp/docs, wasp.pannous.com): CodeMirror 5 + simple mode (web/playground/codemirror/), the
  warp syntax mode (rewritten with word boundaries and the compiler's comment rules: every `//` starts a comment,
  including `//note` without a space), example menu + `?example=`,
  run-while-typing, download wasm. Not taken: the C++ linear-memory node ABI, binaryen wasm→wat, deploy-to-lambda.
- The first markup program a worker compiles pays a one-time cost (lib/markup.warp's to_html): after a run is shown
  the page asks its worker once to warm up (playground.js warmWorker → worker.js warmUp: `p{ "" }` evaluated with the
  worker's messages silenced, skipped over a live run).
  It waits for 5 s without a key or click (WARM_UP_QUIET_MS): sent right after the first run, it held the worker for
  1–4 s and a Hello clicked meanwhile waited behind it (card compiler-cache-compiled).
- The compiler's machine code is cached by the browser, not by the page: compileStreaming of the versioned warp.wasm
  (?build=, host.js ofThisBuild) keeps it with the HTTP-cached response. IndexedDB cannot hold a WebAssembly.Module
  (Chrome and Firefox removed its serialization). Measured on the live site, headless Chrome: Hello at 7.5 s on a
  first visit, 2.3 s after (1.1 s with the quiet warm-up); playground.state().stages shows where a start's time goes.
- Stopping a run (an animation the next run stops, Stop, a sound handover) ends its worker; a new worker loads and warms
  the compiler first, so Hello after a shader took 0.8–1.2 s in Firefox (the user: 4 s after five shaders). A run that
  animates starts a standby worker (playground.js prepareStandby), which the stop takes at once: Hello 0.33 s
  (card playground-progressive-slowdown). Chrome and Firefox showed no growth over many runs otherwise.

## Values of self-hosted modules (values.js)
A module of the self-hosted compiler has no reflect_* exports: a host word reads its value arguments as driver.warp
does natively, through exports (values.js): guestText (shown_at, packed address·2^32+length), isGuestText,
guestCount/guestItem/guestInts (item_count, item_at, int_item_at), guestBytes; guestPlain reads any value from its
shown text (maps, lists, texts, yes/no, ø, numbers, ratios; a list too long to show item by item). Built back with
the constructors of core/emitter/host.warp host_builders (newGuestText, newGuestList from the last item on,
newGuestMap by new_spelled_key/new_spelled_list, newGuestFloats, newGuestError, newGuestValue of plain JS).
Every host word reads and builds so. Ported: std.js stdCall (every std adapter), paint, save_picture, text_coverage,
sound_samples, gpu_*. Known cost: newGuestList is O(n²) (built_list copies its rest) and floats are read by shown
text, so a million-item GPU list stops the run (samples/acceleration/gpu_million; bulk exports asked of core).
lib/sound.warp's device words are host words (sound_played, sound_file_played, sound_file_decoded, midi_sent,
midi_ports): host-sound.js soundWords plays samples on the run's voice, plays a file, decodes and resamples; natively
lib/host/native_host/sound.warp answers them.
Sweep caveats: samples that print a measured duration as "N ms" compare masked; gpu/gpu_visualizer and
gpu/warp_compiled_wgsl_shader end with a while loop whose printed value is a measured time (5.001 vs 5.123): DIFF.

## Cross-origin isolation and tasks on Workers (notes/threads.md step 5)
- The task Workers (host-tasks.js's pool of task-worker.js) need the page cross-origin isolated (shared memory).
  test_in_browser.py sends COOP/COEP; index.html registers coi-serviceworker.js, which adds
  them on GitHub Pages (one reload, guarded by sessionStorage "isolating"; it waits for serviceWorker.ready, an isolated
  page clears the flag). index.html's `pageStarts` resolves only when no reload is coming (isolated, no service
  workers, already tried, registration failed, or 5 s passed), and playground.js initializes then (site.js
  siteStarts likewise), so no Worker starts before the reload.
- The page makes the task Workers (task-workers.js serveTaskWorkers): Firefox's nested `new Worker` waits for the page's thread and stalled starts. The program's Worker
  asks {taskWorker: {id, url}}, gets a MessagePort to the new Worker, and asks {endTaskWorker: id} to stop one; the pool
  holds the ports. A check that leaves a page runs a program on it first (a run waits for taskPoolReady).
- Wasm memories: Chrome holds ~124 live Wasm memories per page, all Workers together (V8's sandbox reserves ~8 GB of a
  1 TB cage per 32-bit memory), and an isolate collects only its own dead instances.
- Safari: a growable SharedArrayBuffer reserves its maxByteLength (TASK_RESULT_LIMIT, 256 MB) and an idle Worker never
  collects it (~226 then "Out of memory"), so every answer buffer (task, fetch, askPage, GPU job) comes from a free list
  (host-tasks.js takeAnswerBuffer) and goes back once its answer is read. Tour checks GPU_JOBS (400 GPU jobs) and
  MANY_TASKS (400 tasks).

## Published: https://warp.pannous.com/
- .github/workflows/pages.yml ("Playground") builds warp.wasm + samples.js with web/playground/build.sh on a push to
  main touching core/, lib/, web/playground/, samples/ or scripts/ci-compilers.sh, hourly (or `gh workflow run Playground -R pannous/warp`) and
  deploys only the playground files plus CNAME to GitHub Pages. Repository Pages settings: build type workflow, custom
  domain warp.pannous.com, HTTPS enforced. DNS: CNAME warp → pannous.github.io at the registrar (orderbox);
  *.pannous.com otherwise points to the pannous.com server.
- Gate: `test_in_browser.py --examples [--site D | --url U]` runs the tour and every sample and fails on any console
  error/warning/failed request of the page and its Workers. agent-browser's `console`/`network` see only the page, so
  console_watch.mjs attaches over CDP (`agent-browser get cdp-url`) to every target: Runtime + Log + Network.
  `--firefox` drives headless Firefox over WebDriver BiDi (firefox_driver.mjs: log.entryAdded +
  network.responseCompleted, workers included; a fresh profile per run; on macOS started with `open -n -g`, since a
  terminal's child may not read Firefox's profile folder). On a timeout firefox_driver.mjs reports the page's state
  (address, readyState, isolated, service-worker controlled, #status) and its console. `--safari` drives Safari through
  safaridriver (no console) on a macos-26 runner (job tour-safari; its Safari has navigator.gpu but no adapter).
  pages.yml runs Chrome and Firefox on _site before deploy and on the live page after (job verify). The runner has no
  GPU (Chrome gets SwiftShader's software WebGPU adapter through AGENT_BROWSER_ARGS) and slow starts
  (`?slow_start=2000` reproduces them locally). A change that may behave differently there:
  `gh workflow run Playground -R pannous/warp --ref <branch>` first.
- Not ours: `TypeError … shiftKey … inject.js` in a user's Firefox is a browser extension's content script.

## test/ in the browser: `scripts/tests-web.sh` (test_in_browser.py --tests)
Every case of test/ runs in the page and is judged as the native `warp test` judges it. The self-hosted compiler
parses run-time code through parse_code, and effects and spectrum are warp in lib/sound.warp.
- test_in_browser.py serves `/__stub__?status=…&body=…&delay=…` (a POST there echoes what it got, as httpbin.org/post) for
  fetch tests, `/__include__/<header>` (WARP_INCLUDE=/include) for C headers, and `<dir>/?listing` for wasi.js's walks.
- CI's Chrome has SwiftShader's software WebGPU adapter, whose sin and exp are off by up to 5.2e-5 (within WGSL's 2^-11).
- libc in the browser (P147): host-foreign.js calls web/playground/lib/libc.wasm, wasi-libc built by lib/build_libc.sh
  (zlib likewise as lib/z.wasm)
  (deployed with the page: pages.yml SITE_FILES `lib`), one instance per worker. Its exports have the native FFI's
  types (lib/libc.c wraps strlen, strncmp, strspn, strcspn, atol, labs, strtol to 64 bits), so numbers pass through;
  texts cross by the program's warp.c_functions section: copied into libc.wasm's malloc and freed after the call, a
  char * result read back (NULL is ø). Anything else of libc throws "c.X is not available in the browser".

## Modules and packages in the browser
The compiler reads files over WASI (wasi.js: synchronous GETs of the served repository): module sources, C modules
(`import test/fixtures/wasm/zlib`: its .wasm and .warp), C headers. The page's own files go by `page:`
(host-files.js PAGE_PREFIX, resolved against the worker's URL, since the page is web/playground/ locally but the
site root when deployed). build.sh copies the files the tour's examples read to their repository paths, tour example
"C libraries". Packages (packages/*/package.warp, linked by scripts/link-packages.sh) are served as files; build.sh
pack_package_libraries copies each package's C library to web/playground/lib/<name>.wasm, so `use uniscript` calls
uniscript_c.wasm (wiki/use.md). A program's `read(path)` of a URL fetches its bytes as they are (no newline added,
like the native read).

## Foreign runtimes in the page (host.js registerForeignRuntime)
- foreign_call dispatches by runtime name to `registerForeignRuntime(name, {call, prepare})`:
  `call(module, member, arguments, hooks)` is synchronous and gives a plain value (arguments null: a read);
  `prepare(code)` is optional and asynchronous, awaited by worker.js (`prepareForeignRuntimes`) before a run, for a
  runtime that must load first (Pyodide). Registered: `js` (the page's globals, host-foreign.js), `python` (Pyodide,
  worker.js gives its prepare), `wasm` (components.js).
- `use lib.wasm` / `use wasm "lib.wasm"` (components.js): a component transpiled by jco into components/<name>.js, a
  classic script with the core modules as base64 (`registerComponent`); the first call importScripts it. build.sh no
  longer makes components/ (its `components` step went), so no component loads in the page today.
- jco's JavaScript values are turned into the native JSON forms by the WIT types, which build.sh embeds as the
  signatures of the exports (`wasm-tools component wit --json`): camelCase ↔ the WIT's kebab-case names, `{tag, val}` →
  `{case: payload}`, flags objects → lists of names, a char → `{$char: c}`, a Codepoint (P94, as natively), a thrown
  result `err` → the error, class instances → handles with ids per component, the argument count with the parameter
  names. Still jco's own: the enum check's message ("\"triangle\" is not one of the cases of shape"), and a variant
  argument (an object) is not converted.
- A promise is refused (host-foreign.js PROMISE_REFUSAL: "gives a promise, which a program in the page cannot wait
  for"); natively node awaits it. worker.js handles messages one at a time (`inTurn`).

## paint: a canvas in the page
`paint(pixels, width, height)` is a host word: host.js reads the pixel list and hands it to the
worker's hooks.paint, the page draws one canvas per call under the output (playground.js showPaintings: nonzero/true
is ink, 0 paper); a painting the size of the last replaces it at once.
- Natively it writes a PNG to <temp>/warp-paint/paint.png (core/native/driver.warp `painted`) and names it on stderr;
  the native host module shows paintings in a window instead (lib/host/native_host/window.warp, never under
  WARP_NO_WINDOW or CI).
- samples/advanced/circle.warp paints a short diagonal (one loop moving x and y together), samples/advanced/filled_circle.warp
  the filled circle with two loops.
- Prints show as they arrive, like paintings: `print "Watch it turn blue."` before a slow paint is read while it
  renders. Only an animation (after its first sleep) holds a text frame back until the next one begins (playground.js
  printedChunk).
- Frames: only what changed. The program's Worker shades the pixels into RGBA words straight from the low and high
  32 bits of each value (host-media.js paintFrame) and keeps the last frame: of a painting the size of the one before it
  posts (transfers) only the rectangle that changed. The page (canvas.js wholePainting) writes that rectangle into the
  whole painting it keeps as an ImageData, and draws the rectangles of a screen frame joined, once per
  requestAnimationFrame; a new canvas is drawn whole at once.
- Fingers: a touch is the pointer. Every painting canvas has `touch-action: none` (canvas.js paintingElement), else iPad
  Safari takes a finger for scrolling and sends pointercancel right after pointerdown; pointercancel releases
  mouse_down like pointerup; of several fingers only the first one down steers mouse_x/mouse_y. The Chrome tour drags a
  real touch over samples/graphics/finger_paint (touch_drag.mjs over the DevTools protocol, test_in_browser.py
  fingers_wrong).
- Tour example "shader mouse": a fragment shader under the pointer, the inputs as shader holes
  (extensions/shader_blocks.warp): `$mouse` (a list [mouse_x, mouse_y] → vec2f), `$mouse_down` (0/1) and `$seconds`
  (from clock()), read at each paint. The ⛶ box (.painted) is as wide as its paintings.

## The default canvas is the output pane (decision: defaults, no errors)
System values `view_width`, `view_height`: playground.js viewSize tells the room below the output pane's text (CSS
pixels, retold on resize and as each run starts) with the other system values; host.js falls back to 640×480
(VIEW_DEFAULTS) on a page without a pane, as natively (core/native/driver.warp NATIVE_VIEW_WIDTH). lib/draw.warp sizes
its default canvas from them, so `use draw; show()` fills the pane.
The room is measured from the pane's width and the window's bottom, never from the paintings (measured around them the
pane grew with each view-sized painting). What follows the paintings to the page's end (diagnostics list, pane padding
and border, body padding; roomBelowPaintings) is left free, and `.painted` is top-aligned, so a view-sized painting
never overflows into a scrollbar that narrows the next view (tour check VIEW_STEADY).
