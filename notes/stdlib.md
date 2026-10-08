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
Each module is `lib/<name>.wasp` in the warp repository, embedded in the binary (`include_str!`), so `use <name>`
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
| collections | classes Stack Queue Deque Set Counter, OrderedMap; HashSet TreeSet frozenset ArrayDeque VecDeque deque as aliases | wasp classes over a list field (lib/collections.wasp, classes side warp-41) |

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
- `use math` resolves: a local `math.wasp` (the file wins) → the embedded `lib/math.wasp` → an FFI library (`m`) →
  the package registry. lib/math.wasp forwards to libm (P169), whose C names still work with a note; `use cmath` is
  libm alone.
- Names: after `use list`, `zip(a, b)` and `list.zip(a, b)` both work; a program's own `zip` wins (as for prelude words).
- A module word used without its `use`: the loud error naming the module, with the fix `use list`.

### Coverage (card std-word)
tests/modules/test_std_coverage.rs fails naming every lib/*.wasp word no test calls: a word is covered when a test
(tests/**/*.rs or .wasp) names it, or a covered word's body calls it. src/ doesn't count: its alias tables
(modules.rs `("acos", "arc_cosine")`) name words without calling them. A new lib word needs an is! test; the words
nothing else tested are in tests/modules/test_std_words.rs (native and browser).

## 6. Steps (functions)
Done (branch functions, 2026-10-07):
1. `lib/<name>.wasp` embedded in warp (modules.rs STD_MODULES; std/ until P194 merged it into lib/, non-standard
   modules like netbase live in lib/extra/); a local file of the same name wins, except warp's own lib/<name>.wasp,
   which is the embedded module itself (modules.rs is_embedded_std_file). The loader keeps a
   std module's definitions aside and gives the program only those it calls, and those they call
   (`with_needed_definitions`): an unused word would compile with parameters of no kind.
   `use list`: unique zip enumerate product mean take drop flatten (tests/modules/test_std_list.rs).
2. A std word without its `use`: "zip is in the standard module list: write `use list`" (ffi::undefined_function_message,
   also for the braceless call).
3. `use math` = libm (as before) + lib/math.wasp: gcd lcm clamp sign; `use text`: repeat pad_left pad_right
   (tests/modules/test_std_math_text.rs). pi, e, tau already exist as exact symbols.
4. Qualified `list.zip(…)`, `math.gcd(…)`, JS's `Math.sqrt(16)`: the bare word with a note (welcome_forms
   module_calls); a program's own variable `text`/`list` keeps its methods (tests/modules/test_std_qualified.rs).
5. `use random`: choice shuffle sample; `use map`: merge map_values (tests/modules/test_std_random.rs, test_std_map.rs).
   A std module's source shows no style hints (parsed under normalize::without_hints).
Bugs met (cards over-keys, inside-loop, index-hint): `for k in keys(m)` / `m[k]` in a loop over a one-entry map
parameter; lib/map.wasp uses `ks = keys(m)` and `m.get(k)` until they are fixed.
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
   list: flat_map partition max_by min_by group_by ({"1": [1 3] "0": [2 4]}, keys are texts) tally. Fixed on the
   way: m.get(k) had no value kind (a list value + [x] was 'int + list'); `xs where it > 1` filters (was silently
   nothing). Built in, no module: sort_by any all find first last index_of; libm now also links asin acos atan atan2
   sinh cosh tanh hypot log2 trunc log1p expm1 (they compiled to their last argument). Still missing in math: hypot
   etc. in the glibc fallback table (card call-name).
   list: zip_with sum_by count_by rotate interleave dedupe; text: title slug truncate; time: format_duration(ms)
   ("1h 30m 30s", "250ms"; plain milliseconds: units don't reach functions yet); map: omit filter_values. Fixed on
   the way: a slice of a Node is a Node (capitalize(w) for the elements of words(t) was 'text + list'); a
   comprehension's list is a `let` local (one in a module and one in the program asked "new local or main-level?").
   math: round_to factorial is_prime lerp; list: scan take_while drop_while argmax argmin (from 0, like xs[i]; 1-based like index_of asked at warp-e9);
   map: entries; text: is_blank; random: seed(n) (host word random_seed: xorshift64* natively and in host.js, per run).
   math: is_even is_odd digits choose mod_pow; list: pairwise split_at cartesian mode variance stdev; text: is_upper
   is_lower remove_prefix remove_suffix count_of is_palindrome; map: get_or; time: is_leap_year days_in_month. Fixed on
   the way: a lambda reading the loop variable given to a function (mode's count_by(xs, y => y == x)) was specialised
   into a function reading the variable before the loop; loop variables now make it a closure
   (closures::captured_variables_of).
   list: index_where last_n fill minmax; math: percent to_radians to_degrees isqrt is_square divisors prime_factors
   to_base from_base; text: word_count snake_case kebab_case camel_case indent is_numeric between wrap; map: find_key.
   A std module's word can call another module's words since §8 (std-module-uses-module); older ones use loops.
   Met: a name `end` after else is Ruby's block end (card end-variable); `none(xs, f)` is the null (card none-call).
   P171: write and exists are prelude words (modules PRELUDE_WORDS: only their definitions come along, a program's own
   word wins); P183: a "file://…" text loads the whole file module; file.append(path, text) is the qualified-only word
   (welcome_forms QUALIFIED_WORDS → append_file), bare append stays the list method.
   P169/P191: `use math` adds descriptive names forwarding to C (square square_root cube_root power exponential
   natural_log binary_log decimal_log logarithm sine … hyperbolic_tangent angle hypotenuse ceiling whole_part
   remainder); the C names still work with a note (modules STD_ALIASES, positioned at the nearest positioned node);
   `use cmath` is the raw C library. A constant fractional exponent (`x ^ (1/3)`) is a float
   power (analyzer inference constant_value); one held in a variable still traps. sqrt/cbrt have no alias: they are the operators √ ∛ (∛ of a run-time value calls libm cbrt, Math.cbrt in the browser).
8. Host modules (async, warp-f0): json (done on std-json), hash, regex, file, os, net — through std_pure/std_io.

Collections (classes, branch classes-36): `use collections` = lib/collections.wasp, classes over a list field:
Stack push pop peek size, Queue enqueue dequeue peek size, Deque push_back push_front pop_back pop_front size,
Set(xs) add has remove size, Counter(xs) add get most_common (tests/modules/test_std_collections.rs). Module only,
not prelude (prelude question queued with warp-e9). A used module's classes go in before class_methods
(modules::insert_module_classes, its own source pass), since modules::resolve runs after class_methods; the loader
leaves those modules' classes out (EARLY_CLASS_MODULES). Same for a file module's classes (`use shapes`, all its
classes; a std module's only those the program names); a class the program declares itself wins. A module used only
inside another module still loads its classes late (their methods unlowered). `new Set(xs)`, `collections.Counter(xs)` and the foreign class names (STD_CLASS_ALIASES) work with a
note. Other languages' method names (append appendleft popleft offer poll addFirst pollLast contains delete shift …,
class_methods METHOD_ALIASES) are the class's methods with a note, on a class not defining that name; `len(s)`,
`count(s)`, `s.len()` of an instance are its size method. OrderedMap() is `{}` (wasp maps keep insertion order),
OrderedDict and LinkedHashMap its aliases (modules STD_ALIASES). Not yet: `from collections import Counter` (no
`from … import` form at all).

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
library compiled to wasm once, embedded in warp like lib/*.wasp). C, E and F need a library, python3 or node on the
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
defines its words over them, `parse_json(text) := std_pure("json", "parse", [text])` (lib/json.wasp); natively
src/std_adapters.rs answers by (module, member), in the browser host.js STD_ADAPTERS. Values cross as for the foreign
runtimes (foreign.rs json_of / node_of, host.js plainOfTree / treeOfPlain). Their results are any Node (analyzer
ANY_VALUE_WORDS). The wrappers' parameters are annotated `any` (`to_json(value:any)`): an unannotated parameter fed
only by such values would default to an int ("not an int" for `parse_json(post(…))`).
Adding a word: one match arm in std_adapters.rs, one function in STD_ADAPTERS, one line in lib/<module>.wasp, a test
run natively and in the browser.

### First adapter steps (async)
1. Done: json (A): `use json` brings parse_json and to_json, native and browser (tests/modules/test_std_json.rs).
1b. Done: file and os (A, std_io): `use file` brings write, append_file, exists, list_files, lines (read stays a
   prelude word); `use os` brings env. Natively the file system (paths as read resolves them) and the environment; in
   the browser host.js keeps written files in memory while the page is open (read sees them first, then the served
   repository) and env is ø (tests/modules/test_std_file.rs). Names: `append_file`, since `append` is the list
   method `xs.append(v)` a program using `use file` still needs (question Q6). `use os; args` (card std-args): the words
   after the program file, `warp run prog.wasp a b` gives ["a" "b"] (main.rs program_file → std_adapters
   set_program_arguments), [] without any and in the browser; without `use os`, `args` is still the symbol.
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

## Decisions (user, 2026-10-07; the former open questions Q1-Q6)
- P169/P191: `use math` is wasp's math module with descriptive names (square, sine, cube_root …), forwarding to C until
  wasp has its own; `use cmath` is the raw C library.
- P170: module words work bare and qualified (`zip(a, b)`, `list.zip(a, b)`).
- P171: new words stay in their modules, except `write` and `exists` (global); a file URL loads the file module.
- P183: std modules are backed only by wasp, host words or C compiled to wasm; regex is the common subset of Rust regex
  and JS RegExp with a loud error for the rest; the file module's append is `file.append(path, text)`.

## 8. Standard: inventory and plan (card stdlib-standard, functions warp-da, 2026-10-08)
Goal (user): standard functionality split into importable modules. Steps 1-7 made the modules; this section is where
every word lives today and what still moves.

### Inventory: where a word is defined
| place | what | count |
|---|---|---|
| src/lowering/library_words.rs SYNONYMS | other names of built-in words (size → len …) | 25 |
| … RUNTIME_WORDS | words lowered to runtime calls | 17 |
| … EXPANDED_WORDS | wasp templates kept as Rust strings: list round_to first last sum replace is_digit is_alpha unwrap is_alphanumeric | 10 |
| src/wasm_emitter/library_ops.rs LIBRARY_FUNCTIONS | runtime functions in wasm: keys values entries contains index_of get without ord chars field_with reverse sort upper lower split join slice | 17 |
| src/wasm_emitter/text_builtins.rs TEXT_BUILTINS | text runtime functions | 21 |
| src/wasm_emitter/list_emitter.rs BUILTIN_CALLS | list runtime calls | 18 |
| src/real.rs FUNCTIONS | exact-real functions | 6 |
| src/host.rs HOST_WORDS | host imports (print, clock, fetch, paint, clipboard, notify, tasks, foreign_call, gpu, std_pure/std_io …) | 55 |
| src/modules.rs STD_MODULES | embedded lib/*.wasp: memory net collections hash regex file json os list math text random map time matrix draw markup router i18n | 19 |
| … PRELUDE_WORDS | module words global without `use`: file: write exists | 2 |
| … STD_ALIASES, lowering/std_aliases.rs, welcome_forms QUALIFIED_WORDS | foreign and qualified names (OrderedDict, JSON.parse, file.append …) | 27 + 21 + 1 |
| … IMPLICIT_MODULES | modules a program needs without `use`: file (a file URL), markup (a page), router (routes), regex (a route's regular expression); markup.rs reads markup's lists at compile time | 4 |
Words per module: math 45, list 38, text 26, markup 19, matrix 15, time 14, map 10, router 9, the rest fewer.
lib/web.webidl is data for web_idl.rs, not a module. The playground has no prelude of its own (warp-70).

Probed (functions2, 2026-10-08): `use list` and `import list` work; `use list, text` and `use list text` load only
list; `from list import zip` has no form; `list.zip(…)` without `use` is the loud error (P171, fine); `use os; args`
is the symbol args (no CLI way to pass them); a std module's `use list` worked, but leaked list's words to the program.

### Target
- Three layers, each with one place in the source:
  1. **Core** (compiler, Rust/wasm runtime): words that need the emitter: the language forms, len/count, indexing, the
     runtime functions of library_ops / text_builtins / list_emitter, host words. They stay; their names are listed
     in one generated index (below), not moved.
  2. **Prelude** (`lib/prelude.wasp`, done, card std-prelude-module): first last round_to replace is_digit is_alpha
     is_alphanumeric, formerly EXPANDED_WORDS templates (list, sum, unwrap stay templates: the emitter dispatches on
     them). Loaded when the program or a used std module mentions a word in any spelling (modules::use_prelude,
     library_words::words_spelled_by: `isdigit`, `round(x, 2)` → round_to); its definitions are named `prelude·first`
     (modules::prelude_name) and library_words writes the calls so, so a program's own `first` or a local named
     `first` never meets them. A pass after modules::resolve must not add a prelude word (undo_history indexes
     instead of calling last). PRELUDE_WORDS (file's write, exists) stay where they are. Fixed on the way: a module
     word inside an interpolation (`"\(zip(a, b))"`, `"\(x.round(2))"`) was missed by the loader
     (modules::template_holes); `declared_name` of a list `[a, b]` named b.
  3. **Modules** (`lib/<name>.wasp`): everything else, brought by `use`.
- **One implicit-use mechanism** (done, card std-implicit): modules.rs IMPLICIT_MODULES, one table "module → when the
  program needs it" (file URL → file, a page → markup, routes → router, a route's regular expression → regex), and
  PRELUDE_WORDS (write/exists → file); page_html.rs and routes.rs insert no `use` text
  (tests/modules/test_std_implicit_use.rs).
- **use forms** (done, cards std-use, std-import): `use a, b` and `use a b` load each; `from list import zip, unique`
  brings only those words (the others stay the loud error); `import list as l` later with the module manager.
- **Modules calling modules** (done, card std-module-uses-module): a std module may `use` another (lib/text.wasp
  `use list`, camel_case calls drop). A module only std modules use is hidden from the program: modules.rs
  Loader::hidden_apart renames its words `lib·drop` in the std definitions, so the program's bare `drop` is the
  error "drop is in the standard module list: write `use list`" and a program's own `drop` is its own
  (tests/modules/test_std_module_uses_module.rs). A file module's `use` counts as the program's.
- **Discoverability**: `help list` (or `words list`) prints a module's words with their first comment line; the
  same index generates wiki/standard-library.md, so docs never drift from lib/.

### Cards (column Next)
std-use-several, std-from-import, std-module-uses-module, std-prelude-module, std-implicit-use, std-args,
std-module-docs, std-word-tests (each module word called once natively and in the browser).
Order: use-several and from-import first (small, independent); module-uses-module before prelude-module (the prelude
calls list words); implicit-use after prelude-module.
