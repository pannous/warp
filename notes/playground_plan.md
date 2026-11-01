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
## Tour gaps (2026-11-01: 13 of 57 tour checks pass on the self-hosted page)
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
| lazy ranges | runs past 20 s | `[999999999999 …]` at once | |
| kitchen sink | unknown name test (67:1) | | |

| markup | `div: {h2: {"Alice"} …}` | `div{h2:"Alice" …}`, the markup shown form | show |

## Guide gaps (2026-11-01: 54 of 82 guide.md snippets hold in the page; `test_in_browser.py --examples --firefox --guide`)
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

Markup DOM examples (element events, fine updates, components, keyed list, form binding, styles, transitions) are
warp-web-samples' page work. Still open: lists (`[#fruits, fruits#1]` writes a comma, so native's `[3, "apple"]` may be
right) and welcoming errors (native's new sentence carries no position).

- Samples that fail natively fail identically in the page. These are core or language gaps, reported to the supervisor:
  a linked storage.wasm can't call type(instance); the word index misses rule heads such as `stored`; plus `where`
  with bare fields, `stop`, `load`, and `go` in an expression.

## Rules for the two playground workers (user: no core changes, delegate them)
- They change only lib/, web/, samples/, test/, scripts/, notes/: never core/.
- When a sample needs a core change: send warp-supervisor one line (sample, error, the core place if known, a repro),
  park that sample, and go on with the next. The Supervisor hands it to a core worker.
- Load: one background test or sweep run each; the browser sweep runs a few samples at a time.
