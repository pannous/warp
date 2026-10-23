# Runtime

This folder holds the runtime words that the pure Warp compiler provides to compiled programs.
Each word is compiled into a module only on its first call, so a word nobody calls costs nothing.

> **Guideline:** once the compiler is fully functional, new functionality normally goes into
> `extended/`, not core. Core grows only for what every program needs.

## Core and extended

The split was approved by the user on 2026-10-22 (decision commit c23bd8b476, card runtime-split).

**Core** lives in this folder. It is what every program, or the compiler itself, needs:
instructions, intrinsics, walks, text, text_words, characters, case and case_tables, show,
float printing, arithmetic, equality, ordering, types, conversions, text_numbers, lists,
library_words, maps, fields, nodes, errors and builders.

**Extended** lives in `extended/` and covers optional features:
exact, exact_rounding, big_ints, pow and pow_tables, durations, times and wasi.

Units, uncertain, math and similar modules are not part of the runtime. They stay ordinary
library modules in `lib/`.

## Rules

- `runtime.warp` uses only core files.
- Core never uses extended. A grep check in self-check enforces this.
- An extended file is loaded only by the word that needs it: for example, exact loads big_ints,
  and float `^` loads pow.
- The emitter declares extended GC types such as `$Limbs` and `$BigInt` only on first use, so a
  hello world contains nothing from extended.
- Intrinsics that cost nothing unless called may stay in core.

## Status

The move into `extended/` is pending card runtime-split, which follows emitter-split. Until then,
some extended files may still sit in this folder.
