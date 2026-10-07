# Standard library (card stdlib-standard; leads: functions = modules in wasp, async = adapters)

User, 2026-10-07: the standard library is released for work. module-manager and package-manager stay parked.
Draft by functions (warp-64), adapters section by async (warp-f0). Defaults below are undoable; open questions at the end.

## 1. Principles
- **Nothing that works today breaks.** Every word in section 4 that works without `use` stays in the prelude.
  New words go into modules; a module word used without its `use` is the error naming the module
  ("zip is in module list: `use list`"), never a silent miss.
- **Written in wasp first.** A module is a `.wasp` file of ordinary definitions; a host word only where wasp cannot do it
  (files, clock, network, processes), an adapter only where another ecosystem is far better (regex engines, hashing,
  big formats). The same module then runs natively, in the browser and under AOT.
- **One name, aliases with notes** (alias rule): `len`, `size`, `count` are one word; other languages' names
  (`str.upper`, `Math.sqrt`, `Enum.map`) work with a got-it note naming wasp's word.
- **P165 soft keywords.** Module names and module words are soft: a program may name a local variable `text` or a
  parameter `count`; a top-level redefinition of a *prelude* word stays allowed as today (user function wins, e.g.
  `def sum(xs)`), unless the word is in `soft_keywords::SOFT_KEYWORDS`.
- **Capabilities.** Modules that touch the outside declare it (src/effects.rs): file → Wasi/Host, net → Host, os/process
  → Process. `eval_untrusted` gets math, text, list, map, json, random-with-seed only.

## 2. Module layout
Each module is `std/<name>.wasp` in the warp repository, embedded in the binary (`include_str!`), so `use <name>`
needs no files on disk and works in the browser.

| module | content | built as |
|---|---|---|
| math | abs sqrt floor ceil round round_to min max pow log ln exp sin cos tan, pi e tau, gcd lcm, clamp, sign, hypot | prelude words (emitter + libm via FFI natively, JS Math in the browser); new ones in wasp |
| text | upper lower trim split join replace starts_with ends_with chars ord chr, pad, repeat (`n times "a"`), format, is_digit is_alpha | prelude words; pad/format in wasp |
| list | first last count sum reverse sort map filter fold reduce each any all min max range, unique zip enumerate product mean median flatten chunk window take drop | prelude words; the new ones in wasp (loops over the GC list) |
| map | keys values entries get has without, merge, map_values | prelude words; merge in wasp |
| time | clock now sleep, today, duration words (`1 s`, `200 ms`), date parts, format | host words (clock, sleep) + wasp |
| random | random random_below, choice shuffle sample, seed | host words + wasp |
| io / file | read, write, append, exists, list_files, lines | host words (WASI natively, a virtual FS in the browser) |
| net / http | fetch, post, url parts | host word fetch (exists) + wasp |
| json | parse_json, to_json (wasp data is a JSON superset: `Node::from_json` / `to_json` exist) | host words |
| os / process | args env exit exec | host words (exit exists), Process capability |
| regex | matches find find_all replace_all | adapter (Rust regex as host word natively, JS RegExp in the browser) |
| hash | hash sha256 md5 crc32 | adapter (xxHash/zlib C modules exist, notes/wasm_modules.md) |

## 3. Prelude (global without `use`)
Everything that works today (section 4) plus the language forms (print, type, int/text/float/as, error/raise/try,
go/await, signals). Proposed additions to the prelude, since every program needs them: none yet; each new word starts
in its module and moves into the prelude only by decision.

## 4. Existing words and their module (probed on functions 6b84b42da, 2026-10-07)
Works today: math `abs sqrt floor ceil round min max pow log sin cos tan ln exp root`; list `len count size range sum
any all filter map fold reduce each first last reverse sort contains index_of slice copy`; text `upper lower trim
strip split join replace starts_with ends_with chars ord chr is_digit is_alpha is_alphanumeric codepoint`; map `keys
values entries get has contains without`; conversion `int text float list type as`; time `clock now sleep`, durations;
random `random random_below`; io `read`; net `fetch`; os `exit`; signals `every daily at whenever on emit send`.
Sources: src/lowering/library_words.rs (SYNONYMS, RUNTIME_WORDS, EXPANDED_WORDS), src/wasm_emitter/library_ops.rs
(LIBRARY_FUNCTIONS), text_builtins.rs (TEXT_BUILTINS), list_emitter.rs (BUILTIN_CALLS), src/real.rs (FUNCTIONS),
src/host.rs (HOST_WORDS), notes/library_words.md.

Missing (probe gives "undefined function"): `unique zip enumerate product mean write exists env args hash regex
matches format pad json parse`; `today` and `args` read as symbols; `exec sh "…"` is undefined outside its capability.

## 5. How a module is loaded
- `use math` resolves: a local `math.wasp` (the file wins) → the embedded `std/math.wasp` → an FFI library (`m`) →
  the package registry. Today `use math` goes to libm directly (modules.rs `is_builtin_library`); std/math.wasp
  re-exports libm's functions, so nothing changes for existing programs.
- Names: after `use list`, `zip(a, b)` and `list.zip(a, b)` both work; a program's own `zip` wins (as for prelude words).
- A module word used without its `use`: the loud error naming the module, with the fix `use list`.

## 6. Steps (functions)
Done (branch functions, 2026-10-07):
1. `std/<name>.wasp` embedded in warp (modules.rs STD_MODULES); a local file of the same name wins. The loader keeps a
   std module's definitions aside and gives the program only those it calls, and those they call
   (`with_needed_definitions`): an unused word would compile with parameters of no kind.
   `use list`: unique zip enumerate product mean take drop flatten (tests/modules/test_std_list.rs).
2. A std word without its `use`: "zip is in the standard module list: write `use list`" (ffi::undefined_function_message,
   also for the braceless call).
3. `use math` = libm (as before) + std/math.wasp: gcd lcm clamp sign; `use text`: repeat pad_left pad_right
   (tests/modules/test_std_math_text.rs). pi, e, tau already exist as exact symbols.
4. Qualified `list.zip(…)`, `math.gcd(…)`, JS's `Math.sqrt(16)`: the bare word with a note (welcome_forms
   module_calls); a program's own variable `text`/`list` keeps its methods (tests/modules/test_std_qualified.rs).
5. `use random`: choice shuffle sample; `use map`: merge map_values (tests/modules/test_std_random.rs, test_std_map.rs).
   A std module's source shows no style hints (parsed under normalize::without_hints).
Bugs met (cards over-keys, inside-loop, index-hint): `for k in keys(m)` / `m[k]` in a loop over a one-entry map
parameter; std/map.wasp uses `ks = keys(m)` and `m.get(k)` until they are fixed.
6. `use time`: date_of(ms) {year month day}, weekday(ms) (ISO, Monday 1), day_number(ms), today()
   (tests/modules/test_std_time.rs). Fixed on the way: `{year:1970 month:1}` read `1970 month` as a duration
   (card key-unit), and the map parameter bug above (cards over-keys, inside-loop: no list copy for a map parameter).
Next:
7. Done: list chunk window median; time add_days days_between, format_date (ISO 2026-10-07), format_time (UTC
   13:05:09), two_digits; text format("{} has {} items", ["cart", 3]). Fixed on the way: `"" + 7` passed to a counted
   parameter made it a list; the elements of split and chars had no kind (`p[0] + 3` added numbers).
   Then: text words lines capitalize center. Fixed on the way: a module's source now gets the program's early
   passes (pipeline::lower_module_source; a comprehension in a module was read as a list), its getters lowered with
   the program (a second getters pass after modules::resolve); a parameter guessed a list takes text when the calls
   pass only texts (pad_right(pad_left(…))).
   map: invert pick from_pairs; time: parse_date("2026-10-07"), days_from_date(y, m, d).
8. Host modules (async, warp-f0): json (done on std-json), hash, regex, file, os, net — through std_pure/std_io.

## 7. Adapters (async, warp-f0)
How a module word is backed when wasp alone cannot do it. All six mechanisms exist (notes/stdlib_connectors.md,
notes/wasm_modules.md); the question per module is which one ships with warp.

### The adapters and where they run
| adapter | native | browser | `warp build --exe` | eval_untrusted | values |
|---|---|---|---|---|---|
| A host word (Rust in warp, src/host.rs; JS twin in web/playground/host.js) | yes | yes, if host.js has the twin | only the stub's words (print, libm, sleep, random, random_below, clock); others refused, named | per capability | Nodes both ways |
| B C library compiled to wasm (`use zlib` → zlib.wasm, types from its header, notes/wasm_modules.md) | yes | yes, same module | not yet (the stub links no second module) | sandboxed, a candidate | numbers, texts, byte buffers, out-pointers |
| C native C FFI (dlopen + headers) | yes | no (host.js shims a few libc words) | refused | refused (Ffi) | numbers, texts, handles |
| D WIT component (`use wasm "lib.wasm"`) | yes | yes (jco, build.sh components) | refused | sandboxed, a later question | WIT types ↔ Nodes, resources as handles |
| E Python (`use python`) | python3 child | Pyodide | refused | refused (Ffi) | JSON, handles |
| F JavaScript (`use js`) | node child | the page's globalThis | refused | refused (Ffi) | JSON, handles |

Rule for the standard library (default): **a std module must work in every host without anything installed.** So it
uses wasp, A (a Rust crate already in warp's dependencies natively, the browser's built-in API in host.js) or B (a C
library compiled to wasm once, embedded in warp like std/*.wasp). C, E and F need a library, python3 or node on the
machine: they stay the user's `use python numpy`, `use js lodash`, `use sqlite3`, never a std module's backing. D is
for Rust crates without a host word (a crate built to a component) once a std module needs one.

Where the two hosts' engines differ (Rust regex vs JS RegExp), the module defines the common subset; a feature only one
engine has is a loud error in both, never a different result.

### Per module
| module | backing natively | backing in the browser | notes |
|---|---|---|---|
| regex | A: Rust `regex` crate (new dependency, ~1 MB in the compiler; the compiler's browser build carries it only if the compiler needs regexes itself) | A: JS RegExp | common subset: no look-around or backreferences (regex lacks them), named groups `(?<n>…)` in both; matches find find_all replace_all split |
| hash | B: sha256/md5 from a small C file compiled to wasm; crc32/adler32 from zlib.wasm and xxh64 from xxhash.wasm (both exist as fixtures) | B: the same modules | one implementation, byte-identical results; Rust `sha2` (in Cargo.toml, optional) would be A natively but needs a JS twin (crypto.subtle is async: a host call cannot wait) |
| json | A: serde_json via Node::to_json / Node::from_json | A: JSON.parse / JSON.stringify with host.js treeOfPlain / plainOfTree | wasp data is a JSON superset, so parse_json gives Nodes; to_json of a non-JSON value (a closure) is a loud error |
| net / http | A: `fetch` exists (ureq); post, headers, status as more host words | A: fetch exists; synchronous XHR in the worker (async fetch cannot be awaited by a host call) | Host capability; the browser obeys CORS, a refused request is the error naming it |
| time (dates, format) | wasp over `clock` (A), the calendar arithmetic in wasp | the same | time zones: A natively (the OS database) vs Intl in the browser, deferred |
| random | A: random, random_below exist (stub too) | A: Math.random twin exists | seed: wasp PRNG (xorshift) over a seed word, so seeded runs are identical in both hosts |
| io / file | A: WASI natively | A: host.js virtual file system (in-memory, per run) | Wasi capability |
| os / process | A: args env exit; exec under Process | A: args empty, env empty, exec the error naming the native host | Process capability (P88 allow-everything mode) |
| compress (zlib) | B: zlib.wasm (compress, uncompress, crc32 already round-trip) | B: the same | a candidate std module once its buffers read as byte lists |
| math | libm (stub "m") natively, JS Math in the browser (today) | — | gcd lcm clamp sign in wasp; no adapter needed |
| text, list, map | wasp | wasp | no adapter |

### How an adapter word is built (A)
Two host words carry every adapter: `std_pure(module, member, arguments)` for words without effects (json, hash,
regex: capability like libm's) and `std_io(…)` for words that touch the outside (file, os, net: Host, IO). A std module
defines its words over them, `parse_json(text) := std_pure("json", "parse", [text])` (std/json.wasp); natively
src/std_adapters.rs answers by (module, member), in the browser host.js STD_ADAPTERS. Values cross as for the foreign
runtimes (foreign.rs json_of / node_of, host.js plainOfTree / treeOfPlain). Their results are any Node (analyzer
ANY_VALUE_WORDS). The wrappers' parameters are annotated `any` (`to_json(value:any)`): an unannotated parameter fed
only by such values would default to an int ("not an int" for `parse_json(post(…))`).
Adding a word: one match arm in std_adapters.rs, one function in STD_ADAPTERS, one line in std/<module>.wasp, a test
run natively and in the browser.

### First adapter steps (async)
1. Done: json (A): `use json` brings parse_json and to_json, native and browser (tests/modules/test_std_json.rs).
1b. Done: file and os (A, std_io): `use file` brings write, append_file, exists, list_files, lines (read stays a
   prelude word); `use os` brings env. Natively the file system (paths as read resolves them) and the environment; in
   the browser host.js keeps written files in memory while the page is open (read sees them first, then the served
   repository) and env is ø (tests/modules/test_std_file.rs). Names: `append_file`, since `append` is the list
   method `xs.append(v)` a program using `use file` still needs (question Q6). args waits for a CLI way to pass them.
2. Done as A instead of B: hash: `use hash` brings sha256 (lowercase hex) and crc32 (a number) of a text's UTF-8
   bytes: sha2 and crc32fast natively (both already warp dependencies), a synchronous JS twin in host.js (crypto.subtle
   is asynchronous, a host call cannot wait); same values in both hosts (tests/modules/test_std_hash.rs). B (C modules
   compiled to wasm) stays the way for xxhash, compression and other libraries without a Rust/JS pair.
3. Done: regex (A): `use regex` brings matches, first_match, find_all, replace_all (`$1` groups in the replacement);
   Rust's regex natively, JS RegExp (flag u) in the browser; look-around and backreferences are the error "… is not
   in wasp's regex (one engine lacks it)" in both (tests/modules/test_std_regex.rs). `first_match`, since `find` is
   the list word find(xs, predicate).
3b. Done: net (A, std_io): `use net` brings post(url, body), the body sent as UTF-8 text, the answer's text (ureq
   natively, a synchronous XMLHttpRequest in the browser; tests/modules/test_std_net.rs against httpbin.org).
3c. Done: other ecosystems' names (src/lowering/std_aliases.rs, the source pass before welcome_forms): `JSON.parse` /
   `json.loads` → parse_json, `JSON.stringify` / `json.dumps` → to_json, `re.findall(p, t)` → find_all(t, p),
   `re.sub(p, r, t)` → replace_all(t, p, r), `fs.readFileSync` → read, `fs.writeFileSync` / `appendFileSync` /
   `existsSync` → write / append_file / exists, `os.getenv` → env, `process.exit` → exit: the got-it note names wasp's
   word, and the alias brings its module as `use json` would. A program that names the module (`re = 3`) or imports
   the real one (`use python "json"`) keeps it (tests/modules/test_std_aliases.rs). A name not in the table goes
   through the foreign bridges (notes/stdlib_connectors.md).
4. Later: the AOT stub linking B modules (they need no compiler), then hash and compress work in executables.

## Open questions (to warp-e9, defaults in force)
- Q1 `use math` = std module re-exporting libm (default) vs. keep `use math` as the raw C library and name the std
  module differently.
- Q2 Module words qualified only (`list.zip`) vs. both qualified and bare after `use` (default: both).
- Q3 Does any new word go straight into the prelude (candidates: zip, enumerate, unique, write, exists)? Default: no.
- Q4 (adapters) A std module works in every host with nothing installed, so it is backed only by wasp, host words or C
     compiled to wasm, never by python3/node/a system library (default) vs. allowing std modules that need them.
- Q5 (adapters) Where Rust regex and JS RegExp differ, the regex module is their common subset with a loud error for
     the rest (default) vs. one regex engine compiled to wasm for both hosts (identical, but ~300 KB more per page).
- Q6 (adapters) file words: `append_file(path, text)` (default; `append` stays the list method) vs. a qualified
     `file.append(path, text)` only, once qualified access exists (Q2).
