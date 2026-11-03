# Runtime

The runtime words are what the pure Warp compiler gives compiled programs. Each word is compiled into a module only
on its first call, so a word nobody calls costs nothing. This file sits beside the runtime folder, not in it:
every entry of `core/runtime/` must run alone as warp (for its folder `extended/` warp says it is a folder, no
program).

> **Guideline:** once the compiler is fully functional, new functionality normally goes into
> `extended/`, not core. Core grows only for what every program needs.

(`extended/` is `core/runtime/extended/`: no underscore compound folder names.)

## Core and extended

The split was approved by the user on 2026-10-22 (decision commit c23bd8b476, card runtime-split).

**Core** is `runtime.warp` and the parts its `use` lines name in `core/runtime/`. It is what every program,
or the compiler itself, needs: instructions, intrinsics, walks, text, text_words, characters, case and case_tables,
show, float printing, arithmetic, equality, ordering, types, conversions, text_numbers, lists, library_words, maps,
fields, nodes, errors and builders, plus bridge (below).

**Extended** is `runtime/extended.warp` and the parts it names in `core/runtime/extended/`, for optional
features: durations, times and wasi, plus the bodies of their hooks (below).

**Extensions' runtimes** are each extension's runtime root, named by its row in the extension table: big numbers
(`extensions/big_numbers.warp`: big_ints, exact, big_ratios, exact_rounding, pow and pow_tables in
`extensions/big_numbers/`, card big-numbers-extension) gives ints past 64 bits, exact ratios and float pow, and the
bodies of their hooks. Its parts `use` the core's files as lib/ links them (`../../compiler/runtime/…`).
Calendar arithmetic is not part of the runtime: `lib/time_arithmetic.warp` (card time-arithmetic-library) is a class
library beside `lib/units.warp`, over plain integers (form and position, month/day/nanosecond counts). The compiler
never loads it: its Span's plus would send every `+` the compiler compiles through operator dispatch, and some of its
names (time_form, day_nanos, hour_nanos, sign_of) clash with times' and durations'. Nothing in the compiler uses it.

The roots are loaded core first, then extended, then each extension's: by `compiler.warp` (`RUNTIME_ROOTS`), each
`use` path from the folder of the file it is in (compiler.warp `root_text` prefixes a root's folder to its `use`
paths, as it lowers all roots in core). Loading only joins the source; the emitter still compiles a word only on its first call.

Units, uncertain, math and similar modules are not part of the runtime. They stay ordinary library modules in `lib/`.

## The bridge

Core's generic words hand a value of an extended type, an exact ratio, a big int or a time, to one of the hooks that
`runtime/bridge.warp` declares by their typed heads only (`… from runtime`, decision bodiless-imports):

| hook | caller | what it does |
|---|---|---|
| hook | caller | body in |
|---|---|---|
| `extended_number_text` | show | big numbers: a ratio (3.5, 1/3) or a big int as text |
| `extended_text` | show | extended: a time as text |
| `extended_order` | ordering, equality | big numbers: -1, 0 or 1 of two Int nodes, one a ratio |
| `extended_sum`, `_difference`, `_product` | arithmetic | big numbers: of two Int nodes, one a ratio, or past 64 bits |
| `extended_float`, `extended_truncated` | conversions | big numbers: a ratio as a float, truncated toward 0 |
| `extended_wrapped` | conversions | big numbers: a big int's low 64 bits |
| `extended_ratio` | text_numbers | big numbers: "3/4" read as n/d in lowest terms |
| `extended_warned` | maps | extended: a warning on stderr |

Each body is its root's own word, not a used file's. Plain ints never reach a hook, so a hello world calls nothing
from extended or an extension.

## Rules

- `runtime.warp` uses only core files.
- Core never uses extended and never names an extended word: it calls the bridge's hooks.
  `scripts/self-check.sh` checks both before taking a slot: no `^use .*extended` line in `runtime.warp` or a plain
  file of `runtime/` (`extended.warp` aside), and no call there to a word that a `runtime/extended/*.warp` file or an
  extension's runtime part defines.
- An extended part is only ever loaded by its root; its words compile only when called, for example
  float `^` compiles pow's.
- The emitter declares each GC type only on first use, so a hello world contains none of extended's.
- Intrinsics that cost nothing unless called may stay in core.
