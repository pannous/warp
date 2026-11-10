# Standard library

Standard functionality split into importable modules, `lib/<name>.warp`. User docs: wiki/standard-library.md.

## 1. Principles
- Nothing that works breaks: prelude words stay global. New words go into modules; a module word used without its
  `use` is found through the library index (`sha256("a")` needs no `use hash`), never a silent miss.
- Written in warp first: a module is a `.warp` file of ordinary definitions; a host word only where warp cannot do it
  (files, clock, network, processes), an adapter only where another ecosystem is far better. The same module then runs
  natively and in the browser.
- One name, aliases with notes: `len`, `size`, `count` are one word; other languages' names (`str.upper`,
  `Math.sqrt`, `JSON.parse`, `re.sub`) work with a got-it note naming warp's word.
- P165 soft keywords: module names and words are soft: a program may name a local `text` or a parameter `count`; a
  program's own definition of a word wins.

## 2. Modules
Each module is `lib/<name>.warp`, prebuilt by `warp rebuild` beside the compiler (core/passes/library.warp,
notes/library_zero_bake.md), so `use <name>` needs no files and works in the browser. Non-standard modules (netbase)
live in lib/extra/; the modules the native
driver links itself (native_host and its parts) in lib/host/, which the library word index skips, so none of their
words brings them to a program (decision host-folder, uses.warp module_folder/is_host_module, wiki standard-library
"Library folders"). lib/web.webidl is data
for extensions/js_globals/webidl.warp, not a module.

| module | content | backing |
|---|---|---|
| math | descriptive names forwarding to libm (square square_root cube_root power sine … hyperbolic_tangent hypotenuse ceiling remainder), gcd lcm clamp sign round_to factorial is_prime lerp is_even digits choose mod_pow percent to_radians isqrt divisors prime_factors to_base from_base | libm natively, JS Math in the browser; the rest warp |
| fast_math | sin cos exp pow atan2 as wasm polynomials, ~10× cheaper than libm (errors in its header); `use fast_math`, `warp fast <file>`, a leading line `fast`, or `fast f(x) := …` (fast scopes also wrap Ints as i64 and divide Ints as floats). Not `use fast`: fast is a float type alias | warp |
| text | repeat pad format lines capitalize center title slug truncate is_blank Cursor case conversions indent wrap … | warp |
| list | unique zip enumerate product mean chunk window median flat_map partition group_by tally scan pairwise … | warp |
| map | merge map_values invert pick from_pairs omit filter_values get_or find_key | warp |
| time | date_of weekday day_number today add_days days_between format_date format_time parse_date format_duration is_leap_year days_in_month | warp over `clock` |
| random | choice shuffle sample seed (xorshift64*, identical in both hosts) | host words + warp |
| file | write exists (prelude), append_file (`file.append`), list_files lines; read, write_bytes, read_bytes are prelude words | std_io |
| os | env, args (`warp run prog.warp a b` gives ["a" "b"]) | std_io |
| json | parse_json (`json.parse`), to_json | warp |
| hash | sha256 (hex), crc32 of a text's UTF-8 bytes | std_pure |
| regex | matches first_match find_all replace_all (`$1` groups) | std_pure |
| net | post(url, body); fetch is a host word | std_io |
| extrema | min max of two or more values or one list, the builtin's replacement in waiting (card min-max-lib; moves into the prelude once as fast) | warp |
| collections | Stack Queue Deque Set Counter; OrderedMap() is `{}` (OrderedDict, LinkedHashMap aliases) | warp classes over a list field |
| matrix, draw, markup, router, i18n, memory, sound, shell, units, agent, wagi | their own notes | warp |

Collections: other languages' class names (`alias OrderedDict => OrderedMap` in lib/collections.warp) and method
names (append appendleft popleft offer poll addFirst …, methods of the classes there) work; `len(s)`, `count(s)` of
an instance are its size method. A class the program declares itself wins.

## 3. Prelude and core
Three layers, each with one place in the source:
1. Core (compiler, core/): language forms, len/count, indexing, the runtime functions (core/emitter/runtime.warp,
   runtime/) and host words (core/analysis/host_words.warp).
2. Prelude (`lib/prelude.warp`): first last round_to replace is_digit is_alpha is_alphanumeric hex_text binary_text …
   Loaded when the program or a used std module mentions a word in any spelling (core/passes/prelude_words.warp:
   `isdigit`, `round(x, 2)` → round_to); its definitions are named `prelude·first`
   (emitter.warp prelude_prefix), so a program's own `first` never meets them.
3. Modules, brought by `use`. The type modules list, text and map are the exception: their words are standard
   functions of every program, `unique(xs)`, `unique xs` or `xs.unique()` without `use` (decision
   type-library-words); `use list` only for the qualified `list.zip`.

## 4. Implicit modules
core/extension.warp implicit_modules_of, "module → when the program needs it" (`warp uses <file>` lists them): file
URL → file (P183), a page → markup, routes → router, a route's regular expression → regex; play, beep, stop_sound,
wait_sound → sound (P171). No `use` text is inserted.

## 5. How a module is loaded
- `use math` resolves: a local `math.warp` (the file wins, except warp's own lib/<name>.warp) → `lib/math.warp` → an
  FFI library (`m`) → the package registry. `use cmath` is libm alone (P169/P191).
- `use a, b` and `use a b` load each; `from list import zip, unique` brings only those words; `import list` works.
- Names: after `use math`, `gcd(a, b)` and `math.gcd(a, b)` both work (P170); a program's own `gcd` wins. A program
  variable `text`/`list` keeps its methods.
- The loader keeps a std module's definitions aside and gives the program only those it calls, and those they call
  (core/passes/prelude_words.warp): an unused word would compile with parameters of no kind.
- Modules calling modules: a std module may `use` another (lib/text.warp `use list`). A file module's `use` counts as
  the program's.

### Coverage (card std-word)
A new lib word needs an is! test.

## 6. Known gaps
- sqrt/cbrt have no alias: they are the operators √ ∛.
- `none(xs, f)` is the null (card none-call); a name `end` after else is Ruby's block end (card end-variable).
- Other ecosystems' names below are not in the self-hosted compiler yet: `JSON.parse(t)` is "no function parse".

## 7. Adapters
How a module word is backed when warp alone cannot do it (notes/stdlib_connectors.md, notes/wasm_modules.md).

| adapter | native | browser | values |
|---|---|---|---|
| A host word (core/native/driver.warp host_word; JS twin in web/playground/host.js) | yes | yes, if host.js has the twin | Nodes both ways |
| B C library compiled to wasm (`use zlib` → zlib.wasm, types from its header) | yes | yes, same module | numbers, texts, byte buffers, out-pointers |
| C native C FFI (dlopen + headers) | yes | no | numbers, texts, handles |
| D wasm module or component (lib/wasm.warp load_wasm) | yes | not yet | WIT types ↔ Nodes |
| E Python (`use python`) | libpython (lib/native/python.warp) | Pyodide | values, references |
| F JavaScript (`use js`) | the webview | the page's globalThis | JSON, handles |

Rule (P183): a std module works in every host without anything installed, so it uses warp, A (libc or what the
native warp links, the browser's built-in API in host.js) or B (a C library compiled to wasm once).
C, E and F need a library, python3 or a webview on the machine: they stay the user's `use python numpy`, never a std
module's backing. Where the two hosts' engines differ (POSIX regex vs JS RegExp), the module defines the common subset;
a feature only one engine has is a loud error in both ("… is not in warp's regex (one engine lacks it)": look-around,
backreferences), never a different result.

Per module: json is warp in both hosts (lib/json.warp); hash sha256 in the native warp / a
synchronous JS twin (crypto.subtle is async, a host call cannot wait); regex libc regcomp / RegExp (flag u); net curl /
synchronous XMLHttpRequest (the browser obeys CORS); file WASI natively, in the browser in memory and on a page in
IndexedDB (store `files` of database `warp`, read back before the first run; read sees them first, then the served
repository); os env is ø and args [] in the browser, exec the error naming the native host. B stays the way for
xxhash, compression and other libraries without a native/JS pair; zlib.wasm is a candidate std module.

### How an adapter word is built (A)
Two host words carry every adapter: `std_pure(module, member, arguments)` for words without effects (hash,
regex) and `std_io(…)` for words that touch the outside (file, os, net). A std module
defines its words over them, `public first_match(subject: any, pattern: any): any := std_pure("regex", "first", [subject, pattern])` (lib/regex.warp); natively
core/native/driver.warp host_word answers by (module, member), in the browser host.js STD_ADAPTERS; a member the
host lacks is the Error "the host word <module>.<member> is not in warp yet". Their results are any Node
(core/analysis/host_words.warp); the wrappers' parameters are annotated `any`: an unannotated
parameter fed only by such values would default to an int.
Adding a word: one branch in driver.warp host_word, one function in STD_ADAPTERS, one line in lib/<module>.warp, a
test run natively and in the browser.

Other ecosystems' names (not ported yet, section 6): `JSON.parse` / `json.loads`,
`JSON.stringify` / `json.dumps`, `re.findall(p, t)` → find_all(t, p), `re.sub(p, r, t)` → replace_all(t, p, r),
`fs.readFileSync` → read, `fs.writeFileSync` / `appendFileSync` / `existsSync`, `os.getenv` → env, `process.exit` →
exit: the note names warp's word, and the alias brings its module. A program that names the module (`re = 3`) or
imports the real one (`use python "json"`) keeps it.
