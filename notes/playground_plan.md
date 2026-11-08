# Today's plan (user 2026-11-01): every sample runs in the browser playground

Goal: each samples/**/*.warp runs in web/playground with the output the native warp gives (test/samples.txt).
This supersedes decision playground-after-parity (the user: "the bigger local issues appear resolved").

## Where it stands
- web/playground still serves the Rust-era compiler (warp.wasm of 2026-10-21; build.sh builds src/ with cargo, gone).
- The native warp runs the self-hosted compiler warp.wasm (core/compiler.warp) through core/native/driver.warp, which
  answers its host words (54 `warp_wasmtime` imports, WASI, files, modules, FFI) over libc.
- Cards: wasm-playground-build, playground-compiles (both say: host warp.wasm in JavaScript; done by step 1).

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
   a sample that can't run in a browser (processes, C calls) says `# host: native: <why>`, which keeps it out of the menu and the page sweep.

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
## Tour gaps (2026-11-02: 19 of 52 tour examples pass in the self-hosted page, `test_in_browser.py --examples --firefox`; 2026-11-01: 13 of 57)
Failing on 2026-11-02: constants, lazy ranges, linear arrays, matching, arguments, polyglot calls, broadcasting,
closures, classes, properties, welcoming errors, signals, listeners, signals in lists, events, timers, system values,
tasks, channels, markup, element events, fine updates, components, cleanup, keyed list, form binding, styles, routes,
forms, transitions, wasm components, C libraries, kitchen sink. The table below is from 2026-11-01.
The page gives what the native warp gives, so these are native gaps. Repro: the example's code in
web/playground/examples.js, run with `warp <file>`. Each example is extracted with its name's spaces as `_`:
`node -e 'const vm=require("vm"),fs=require("fs");const c={self:{}};vm.createContext(c);vm.runInContext(fs.readFileSync("web/playground/examples.js","utf8")+";this.E=EXAMPLES",c);for(const[n,e]of Object.entries(c.E))try{fs.writeFileSync("data/web/examples/"+n.replace(/ /g,"_")+".warp",e.code)}catch{}'`

| example | native result | expected | known place |
|---|---|---|---|
| constants | `pi = 4; 2 * pi` gives 8 | Error "pi is a constant" (P130) | regression |
| on exit | prints `working`, no `bye` | `bye` at the end (P134) | regression |
| tasks | `late` before `first!` | `first!`, then `late` (a task runs beside) | ordering bug |
| linear arrays, matching | `[1, 4, 9, 16, 25]`, `["logo.png", "cat.jpg"]` | space form (shown-separator: no commas in the source) | show |
| arguments | unknown name greeting (2:39) | | |
| polyglot calls | no dot call plus.(…) | | |
| broadcasting | the operator * on list and list (2:16) | | |
| closures | a call of sorted_node with 2 arguments (5:18) | | |
| classes | unknown name super (3:51) | | |
| properties | a int written to a typed field fahrenheit (4:20) | | |
| listeners | unknown name whenever (3:9) | | |
| events | unknown name price (4:10) | | |
| timers, system values, channels, cleanup | unknown name on: `use keys` defines it | | implicit uses skip rule heads (prelude_words.warp word_index_text) |
| animation, frames, mouse | a text where a int is needed, lib color.warp:29 hex_text | | lib/color.warp |
| game of life | unknown name every (22:4) | | |
| routes, forms | unknown name route | | warp-speed, card server-routes |
| wasm components | the call counter(…) (3:5) | | |
| C libraries | a call of crc32 with 3 arguments (4:55) | | |
| signals, signals in lists | a list where a int is needed (3:9, 6:58) | | |
| lazy ranges | builds the range of 10^12 numbers: 4.3 GB in 12 s natively; in the page Chrome freezes under a memory cap before the 10 s stop, so a tour stalls there | `[999999999999 …]` at once: count, sum and # of a range need no items | range words (lazy) |
| kitchen sink | unknown name test (67:1) | | |

| markup | `div: {h2: {"Alice"} …}` | `div{h2:"Alice" …}`, the markup shown form | show |

## Guide gaps (2026-11-02: 60 of 83 guide.md snippets hold in the page, `test_in_browser.py --examples --firefox --guide`; 2026-11-01: 54 of 82)
The page gives what the native warp gives for all 28. Each snippet is guide.md at its line.

| guide.md line | snippet | native | the guide |
|---|---|---|---|
| 302 313 338 344 352 360 382 394 | `[1 2 3]`, `sort [3 1 2]`, `{name: "Ada" age: 36}` … | `[1, 2, 3]`, `{name: "Ada", age: 36}` | space form (shown-separator) |
| 684 690 696 702 | `p{ "hello" }`, `ul{ li all fruits }` | `p: {"hello"}`, `ul: {li all ["apple", "pear"]}` | `p:"hello"`, `ul{li:"apple" li:"pear"}` |
| 605 611 | `1 / 0`, `raise "boom"` | a failure "divide by zero at …:1:1" | the value `Error("divide by zero at 1:1")` |
| 617 | `if f(-1) failed` of `error "neg"` | 2 | 1: failed sees the error |
| 524 | `type(3)` | `"nat"` | `class int` |
| 97 | `2h + 30min` | unknown name min: `use extrema` defines it | 150min |
| 289 | `match day { Saturday \| Sunday => …}` | unknown name Sunday | "weekend" |
| 319 | `[Alf, Bob]` | unknown name Alf | 2 |
| 332 | `sum 1 2 3` | unknown name sum | 6 |
| 367 504 | `sort(xs, {it.length})`, `sort(xs, {$0 > $1})` | a call of sorted_node with 2 arguments | sorted |
| 375 | `square := it * it; square [1 2 3]` | the operator * on list and list | [1 4 9] |
| 562 569 | `local["name"]`, `database.score` | unknown name local / database | the value |
| 576 | `people where age > 18` | unknown name age | "Bo" |
| 587 | `write "f" "milk"; read "f"` | unknown name read | "milk" |
| 799 | `law square(-x) == square(x)` | unknown name law | 9 |

Markup DOM examples (element events, fine updates, components, keyed list, form binding, styles, transitions):
warp-web-host's page work now, on lib/markup.warp's two exports (card markup-keeps): page_render(value) is the HTML,
handlers numbered from 1 per render as data-warp-<event>="n"; page_event(n, detail) runs one. driver.js already
renders and handles through them (pageEventHandled). State 2026-11-02: the tour's markup examples say `use web` (a
used library stays explicit; whether a handler brings markup in by itself is a morning question) and put a comma
beside each handler; the examples check compares a markup page's rendered HTML (test_in_browser.py show_example).
Holding: markup, element events, keyed list, routes, the computed text between children. Native gaps, warp-markup's:
fine updates/styles (markup-done-selector, live style maps), components (def-counter), cleanup (on mount/cleanup),
form binding (bind:), transitions (`transition:` as style); warp-speed's: forms (urlencoded body fields, 303 after a
POST, which the page already follows). A serving program's run is reported with its first page.
Card remove-rust (verified by `test_in_browser.py --examples --firefox`): the Rust-era paths go:
host.js page·value/page·render/page·hole(s)/on·<event>·node (renderedHtml, pageHoles, holePatches, pageEvents,
runPageEvent, listeningRun), worker.js PAGE_VALUE and showHoles, host-compiler.js page_event, markup.js
data-warp-instance (done c76e12a0e0). Then blockcompiler-entries: the block compiler, effects/spectrum entries and the
Rust test binary's runner gone; built sites on driver.js (`warp build --site`, notes/web_framework.md "Built sites").
Still open: lists (`[#fruits, fruits#1]` writes a comma, so native's `[3, "apple"]` may be
right) and welcoming errors (native's new sentence carries no position).

- Samples that fail natively fail identically in the page. These are core or language gaps, reported to the supervisor:
  a linked storage.wasm can't call type(instance); the word index misses rule heads such as `stored`; plus `where`
  with bare fields, `stop`, `load`, and `go` in an expression.

## Rules for the two playground workers (user: no core changes, delegate them)
- They change only lib/, web/, samples/, test/, scripts/, notes/: never core/.
- When a sample needs a core change: send warp-supervisor one line (sample, error, the core place if known, a repro),
  park that sample, and go on with the next. The Supervisor hands it to a core worker.
- Load: one background test or sweep run each; the browser sweep runs a few samples at a time.

## test/ in the page (scripts/tests-web.sh, night decision N6)
- `scripts/tests-web.sh <tree> [paths]` lists the cases with the page's compiler (test-list), runs each in the page
  (`playground.evaluate(code, file, {asData: true})`: WARP_VALUE_AS_DATA, no hints, no WARP_SERVE/WARP_PROGRAM, as
  core/native/tests.warp's `warp run <code>`), lets test-verdicts judge them and sorts them against the stored baseline:
  ok, PAGE (fails only in the page), EXP (its section says native only), BOTH, NONE. A full run writes test/tests-web.txt.
- Never merge during a run: test-verdicts refuses a tree that changed since the listing.
- The stored baseline lags main: before calling a PAGE case a page gap, run its file natively (`warp test <file>`).
- A section the browser cannot run says so in its heading, `# name — native only: why` (raylib, SDL2, os.command,
  PATH, /dev/null, /etc/hosts, libm's last digit): a line of its own would move the lines the verdicts are keyed by.
- The page compiles files; native `warp test` runs code inline. Separately linked programs differ (card program-file):
  sleep with a unit, color/math modules, factorial.signature, patterns. Not hidden in the runner: users get files too.
- 2026-11-02: 8988 of 11998 hold, 69 PAGE: 31 native regressions on main, 6 fixed since (emit key), 22 program-file,
  8 zlib (N18), module_directory (the native inline run's folder), sound_seconds (a morning question).
