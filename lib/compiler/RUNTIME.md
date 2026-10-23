# Runtime

The runtime words are what the pure Warp compiler gives compiled programs. Each word is compiled into a module only
on its first call, so a word nobody calls costs nothing. This file sits beside the runtime folders, not in them:
every entry of `lib/compiler/runtime/` must run alone as warp (tests/functions/test_bodiless_runtime_words.rs
`each_runtime_part_runs_alone`).

> **Guideline:** once the compiler is fully functional, new functionality normally goes into
> `extended/`, not core. Core grows only for what every program needs.

(`extended/` is now `lib/compiler/runtime_extended/`.)

## Core and extended

The split was approved by the user on 2026-10-22 (decision commit c23bd8b476, card runtime-split).

**Core** is `runtime.warp` and the parts its `use` lines name in `lib/compiler/runtime/`. It is what every program,
or the compiler itself, needs: instructions, intrinsics, walks, text, text_words, characters, case and case_tables,
show, float printing, arithmetic, equality, ordering, types, conversions, text_numbers, lists, library_words, maps,
fields, nodes, errors and builders, plus bridge (below).

**Extended** is `runtime_extended.warp` and the parts it names in `lib/compiler/runtime_extended/`, for optional
features: exact, exact_rounding, pow and pow_tables, durations, times and wasi, plus the hooks' bodies (below).
`runtime/durations.warp` is a link to `runtime_extended/durations.warp`, kept while
tests/numbers/test_duration_words.rs reads the file at its old path.
`runtime_extended/time_arithmetic.warp` (calendar arithmetic, card time-arithmetic-library) is an extended part the root
does not load yet: its words work on plain integers (form and position, month/day/nanosecond counts), some of its names
(time_form, day_nanos, hour_nanos, sign_of) clash with times' and durations', and node_add, node_sub and node_mul do
not call it yet. `lib/compiler/time_arithmetic.warp` links to it while tests/parser/test_self_time_arithmetic.rs reads
the file at its old path.

Both roots are loaded, core first: by `compiler.warp` (`RUNTIME_ROOTS`) and by `src/compiler/self_hosted.rs`
(`EXTENDED_RUNTIME_WORDS`). Loading only joins the source; the emitter still compiles a word only on its first call.

Units, uncertain, math and similar modules are not part of the runtime. They stay ordinary library modules in `lib/`.

## The bridge

Core's generic words hand a value of an extended type, an exact ratio or a time, to one of 7 hooks that
`runtime/bridge.warp` declares by their typed heads only (`… from runtime`, decision bodiless-imports):

| hook | caller | what it does |
|---|---|---|
| `extended_text` | show | a ratio (3.5, 1/3) or a time as text |
| `extended_order` | ordering, equality | -1, 0 or 1 of two Int nodes, one a ratio |
| `extended_sum` | arithmetic | the sum of two Int nodes, one a ratio |
| `extended_float` | conversions | a ratio as a float |
| `extended_truncated` | conversions | a ratio truncated toward 0 |
| `extended_ratio` | text_numbers | "3/4" read as n/d in lowest terms |
| `extended_zeros` | floats | a text of zero digits |

`runtime_extended.warp` gives their bodies itself, calling exact's and times' words: the Rust lane refuses a word
declared in one used file and defined in another, so the bodies are the root's own words, and the root runs alone. Plain ints never reach a hook, so a
hello world calls nothing from extended. In the Rust lane a core part runs alone with its hooks trapping if called.

## Rules

- `runtime.warp` uses only core files.
- Core never uses extended and never names an extended word: it calls the bridge's hooks.
  `scripts/self-check.sh` checks both before taking a slot: no `^use .*extended` line in `runtime.warp` or a plain
  file of `runtime/`, and no call there to a word that a `runtime_extended/*.warp` file defines.
- An extended part is only ever loaded by `runtime_extended.warp`; its words compile only when called, for example
  float `^` compiles pow's.
- The emitter declares each GC type only on first use, so a hello world contains none of extended's.
- Intrinsics that cost nothing unless called may stay in core.
