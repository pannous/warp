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

## 6. First steps (functions)
1. `std/` folder, the embedded loader in modules.rs, `use list` for a first wasp-only module: unique, zip, enumerate,
   product, mean, flatten, take, drop (tests/modules/test_std_list.rs).
2. The "word is in module X" error for words of a known module.
3. text: pad, format; math: gcd, lcm, clamp, pi/e/tau constants.
4. Host modules with async: file (write, exists, append), os (env, args), json.

## 7. Adapters (async, warp-f0)
To be written by warp-f0: per ecosystem (C/wasm FFI, JS, Python, Rust components) how a module's word is backed, and
which modules (regex, hash, json, http) use which adapter natively and in the browser. See notes/stdlib_connectors.md.

## Open questions (to warp-e9, defaults in force)
- Q1 `use math` = std module re-exporting libm (default) vs. keep `use math` as the raw C library and name the std
  module differently.
- Q2 Module words qualified only (`list.zip`) vs. both qualified and bare after `use` (default: both).
- Q3 Does any new word go straight into the prelude (candidates: zip, enumerate, unique, write, exists)? Default: no.
