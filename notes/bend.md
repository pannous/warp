# Bend, HVM and what warp can take from them (warp-bend, 2026-10-11)

User request: "just import everything they've done and make it ours". Repos cloned (shallow) into
`scratch/bend/{Bend,HVM,HVM2,HVM3,HVM4,Kind}`.

## TL;DR

- **Bend 2 (Sept 2026, v2.0.36 on 2026-10-07) no longer runs on interaction nets.** It is a new language: Python-shaped
  syntax, an affine dependent type theory (BendTT), laws + proofs, compiled to one C file (CPU threads, Metal, CUDA)
  or to JavaScript. Its own runtime paper says: "Readers of the author's earlier runtimes may expect interaction nets
  here; there are none." Bend 1 programs and HVM do not carry over. Repo moved to github.com/bendlang/bend.
- The "kernel" today means two different things: **BendRT** (the C runtime, no GC, fork-join "cube" scheduler) and
  **BendTT** (the proof kernel, `bend2/bendtt.lean`, with a Lean consistency proof).
- The interaction-net line (HVM1 → HVM2 → HVM3 → HVM4) is a research runtime now separate from Bend. HVM4 (newest,
  C, May 2026) **has no license file**: we may read it, not copy it.
- For warp the valuable parts are the **ideas**, not the code: Bend's parallel call `a b = f(x) g(y)`, the
  LAWS/PROOF file convention, and proofs as defs. A second backend or a ported kernel is a poor trade (below).

## 1. What Bend and HVM are now

### Bend 2 (`scratch/bend/Bend`, TypeScript on Bun + C/JS runtimes + Lean kernel)

| piece | file | lines |
| --- | --- | --- |
| parser, checker, evaluator | `bend2/bend.ts` | 3.9 k |
| compiler to C (host+device) and JS | `bend2/comp.ts` | 6.4 k |
| CLI | `bend2/main.ts`, `safe.ts` | 2.5 k |
| proof kernel + consistency proof | `bend2/bendtt.lean` | 3.9 k |
| standard library | `bend2/base.bend` | 3.1 k |
| effects (IO) | `bend2/effs/*.c|*.js` (print, file, tcp/udp, env, now, audio, channels) | |

Language (guide/GUIDE.md):
- Pure, functional, everything annotated (nothing inferred). Types are terms (`Type : Type`), dependent types.
- **Quantities**: `-x` erased, `x` affine (used at most once), `+x` reusable (only for `Data` kinds: no lambdas,
  arrays, handles). The checker splits code into *live* (runs, must be affine and terminate) and *dead* (types,
  proofs: may be inconsistent). `bendtt.lean` proves no accepted def has type `Empty` and live code halts.
- Numbers: Base has only `U32`, `F32` and `Nat` (`0n`, `1n+p`): no 64-bit ints or floats, no signed ints; `Char`,
  `String`, lists, arrays, ADTs, closures,
  monads (`do IO<…>`), templates (`~f` compile-time arguments), modules, apps (a `tick` state fold per frame), shaders.
- **Parallelism**: exactly two marks. The *parallel let* `a b = f(x) g(y)` runs the calls in parallel (promise:
  independent and balanced); `f!(x)` runs the call and every parallel call inside it on the GPU. The JS target runs
  everything sequentially.
- **Laws**: `law name: for x: T  {lhs == rhs : T}`; the proof is a def of the same name returning that equality
  (`{==}` reflexivity, `%e : P` rewrite, a recursive call = induction hypothesis). No tactics. Project convention:
  `LAWS.bend` (human writes, AI must not touch) and `PROOF.bend` (AI writes); `bend PROOF.bend` prints
  `ALL PROOFS CHECK` / `SOME PROOFS FAIL`; `--verdict` re-checks with the Lean-verified kernel.
- Bend 1's `fold` / `bend` keywords (structural fold, tree generation with `fork`) are gone in Bend 2.

Runtime, BendRT (paper/BendRT.pdf, source `bend2/docs/BendRT/main.typ`):
- One 64-bit word per term, one heap shared by all cores and the GPU, **no GC**: a `match` frees the node it consumes
  (affinity says it is the only owner); only `+` values get reference counts (*share inference*); read-only
  arguments are lent without counts (*borrow inference*).
- No C call stack: each def is a *segment* of one worklist function; every call is a tail, a cut (continuation `f$k`)
  or a fork (joiner `f$j`). Lambdas are lifted to defs over their captures.
- Scheduler: the *cube*, a 128×128 grid of rings, forks along rows (grow), drains along columns (work); no locks, no
  work stealing; the program must split work evenly. M4 Max: sequential 0.8–1.5× hand-written C, 16 threads 9–12×,
  integrated GPU up to 67×.

### HVM line (interaction calculus / combinators)

| repo | what | language | license | last commit |
| --- | --- | --- | --- | --- |
| HVM (HVM1) | lazy interaction-net runtime, 2022 | Rust | MIT, © 2023 Victor Maia | 2024-02 |
| HVM2 | strict interaction-combinator evaluator, C + CUDA, `hvm` crate 2.0.22; Bend 1's backend | Rust/C/CUDA | Apache-2.0 | 2024-08 |
| HVM3 | lazy/optimal Interaction Calculus runtime | Haskell + C | MIT, © 2024 Victor Taelin | 2026-01 |
| HVM4 | Interaction Calculus runtime, "before launch", one `src/hvm.c` (6.4 k lines), docs per interaction rule (`docs/hvm/interactions/*.md`: app_lam, dup_lam, dup_sup, …) | C | **none** | 2026-05 |
| Kind | proof language on HVM3 | Haskell | MIT, © 2024 Victor Taelin | 2024-12 |
| Bend (Bend 1, Rust, in history / old crate `bend-lang` 0.2.x) and Bend 2 | | Rust → TS | Apache-2.0, © 2026 HigherOrderCO | 2026-10-10 |

HVM model: a program is a graph of nodes (λ, application, duplicator/superposition, eraser, numbers, constructors);
evaluation rewrites active pairs by a fixed table of local rules (beta = APP-LAM, DUP-LAM, DUP-SUP annihilation or
commutation, …). Every rule is local and confluent, so all redexes can fire in parallel and sharing is optimal
(no duplicated work for shared lambdas). HVM2 numbers are 24-bit (u24/i24/f24); strings and IO were thin. The cost
was interpretation overhead, which is why Bend 2 abandoned it.

## 2. Licenses: what "make it ours" requires

- **Apache-2.0 (Bend 1+2, HVM2):** copying or porting code (including translating TS/C to Rust or warp) is allowed,
  also commercially, under these conditions: ship a copy of the license; keep the copyright line
  (`Copyright 2026 HigherOrderCO` for Bend, the HVM2 authors for HVM2); mark modified files as changed ("ported from
  Bend bend2/comp.ts, modified"); there is no upstream NOTICE file today, so no NOTICE text is owed, but if one appears
  in a version we copy, its text must be carried. Patent clause: contributors grant a patent license for their
  contributions; it terminates for us if warp's owner sues anyone claiming the work infringes a patent. No trademark
  grant: we can't call anything "Bend" or "HVM" beyond factual attribution.
- **MIT (HVM1, HVM3, Kind):** keep the copyright + permission notice in copies or substantial portions. No patent grant.
- **HVM4: no license = all rights reserved.** Reading it and re-implementing the published interaction rules (math,
  from papers and Lafont 1997) is fine; copying `hvm.c` or its docs is not. Ask HOC for a license before any port.
- **Ideas, syntax and semantics are not copyrightable**: adopting the parallel let, laws/proof file convention or
  quantity marks needs no attribution legally; we credit Bend in the wiki anyway.
- Mechanics if code is ever copied: a `THIRD_PARTY/bend/LICENSE` (Apache text) + `THIRD_PARTY/README.md` listing
  file → origin commit, and a header comment in each ported file. Note: the papers' text is not under the code license.

## 3. Adoption options against warp's architecture

warp: Node AST → lowering passes → WASM GC module, run by wasmtime or the browser. Values are GC objects; real threads
(notes/threads.md) are one instance per task with copied arguments, because GC refs cannot cross threads.

| option | effort | gain | blockers | verdict |
| --- | --- | --- | --- | --- |
| A. warp → HVM2 as second backend | L (months) | GPU/many-core speedup only for pure, recursive, balanced code (tree sums, divide-and-conquer, search) | HVM2 is frozen (2024) and 24-bit numbers; no warp strings, maps, floats64, big ints, units, IO, FFI, drawing; warp is dynamic/mutable; Bend itself left it | no |
| A'. warp → Bend 2 source (or its C) as backend | L | BendRT speed + Metal/CUDA for the pure subset | Bend 2 has only U32/F32/Nat numbers, requires full annotations, affine values, termination; warp's mutation, dynamic kinds and effects don't map; a dependency on bun + clang | no, maybe a later export of pure functions |
| B. port HVM kernel to WASM as a warp runtime | L–XL | interaction nets live in *linear memory*, which wasm threads can share (unlike GC objects), so it would give shared-memory parallelism in wasmtime | HVM4 unlicensed; HVM2's speed comes from CUDA/C atomics; browser threads need COOP/COEP; a second value model beside $Node; numbers 24-bit | no |
| C. adopt BendRT ideas in warp's runtime | M, piecewise | worklist/no-stack calls (deep recursion without stack overflow), fork-join scheduler for `go` | WASM GC already frees for us, so ownership-freeing gains little; the cube needs shared memory, which GC objects lack | partly: the fork-join shape, not the allocator |
| D. adopt Bend's language laws as warp semantics/lowering | S–M per item | parallel divide-and-conquer with existing threads; laws that AI agents must prove | none hard | **yes** |
| E. reuse parser/type pieces | — | — | Bend's parser/checker are TS for a different (indentation, annotated) syntax; warp has its own parser; BendTT's ideas (quantities, live/dead split) are what transfers | ideas only |

Details for D (what to take):
1. **Parallel let**: `a b = f(x) g(y)` → warp lowering to `ja = go f(x); b = g(y); a = await ja` (the second call runs
   on the current thread, as BendRT does). It reuses lower_tasks/resolve_tasks. Programs that get faster natively:
   recursive tree sums, fib-style splits, merge sort, n-queens, ray tracing per half image, mandelbrot rows. Cost: one
   instance + copy per fork, so it needs a depth cutoff (fork only the top log2(cores) levels, run deeper levels as
   plain calls; BendRT's "a fork met while draining runs as a plain call"). Browser: Workers only with cross-origin
   isolation; otherwise sequential, like Bend's JS target. Bigger win later: shared typed arrays (threads.md step 6).
2. **LAWS/PROOF convention**: warp has `law` already (notes/laws.md: stated → asserted → tested → proved via Lean).
   Take Bend's `for x: T` quantifier form as an alias, a `LAWS.warp` file the AI must not edit, and `warp verify` as
   the gate with Bend's wording ("all proofs check / some proofs fail"). Done (card bend-style, 2026-10-11): named
   laws accept one `for x: T` line per variable, the root `LAWS.warp` is the human-owned contract, and the gate exits
   successfully only when every law is proved. Warp checks generated Lean directly, so it needs no generated
   `PROOF.warp`. Proofs-as-defs (equality, rewrite, induction by recursive call) remain separate.
3. **Live vs dead / quantities**: types and laws are never run; an erased mark `-x`. Low priority for a dynamic
   language; worth noting for the semantic IR (laws.md "Next").
4. **Interaction rules as rewrite laws**: the HVM rule table (APP-LAM, DUP-SUP, …) is a fine test oracle for an
   optimizer's beta/sharing rewrites, but warp has no graph IR that needs it now.

What NOT to take: interaction-net evaluation (Bend itself dropped it), GPU codegen (warp's GPU path is notes/gpu.md),
the affine-only discipline (it would break warp's mutation and aliasing-free copies that already work).

## 4. Cards

First (one card): **parallel let** `a b = f(x) g(y)` lowered onto go/await with a fork-depth cutoff, plus
`samples/tasks/parallel_let.warp` (tree sum / pow2 like Bend's README) and a timing probe native vs sequential.
Done (card lower-parallel, 2026-10-11), notes/go_blocks.md "Parallel let".

After it:
- Done: `for x: T` law quantifier alias + `LAWS.warp` convention + `warp verify` gate wording (card bend-style).
- Proofs as warp defs for recursive laws (equality type, rewrite, induction by recursive call), checked by the Lean
  export; this closes the "recursive functions stay Tested" gap in notes/laws.md.
- Export pure, annotated warp functions + laws to `LAWS.bend` so `bend PROOF.bend --verdict` can check them
  (cross-check of warp's Lean verdicts; optional, needs bend installed).
- Fork-join scheduler for `go` with a fixed pool (BendRT cube idea: no stealing, balanced splits) instead of a thread
  per task.
- Wiki credit: a "Prior art: Bend/HVM" section in the laws and threads pages.

Longer term, an own proof kernel absorbing Lean's and Bend's ideas: notes/own_prover.md.

Sources: github.com/HigherOrderCO/Bend (→ bendlang/bend) README, guide/GUIDE.md, CHANGELOG.md, paper BendRT.pdf /
BendTT.pdf; HVM2 README + paper; HVM4 README and docs; Taelin on X (2025-11) on Bend2/HVM4 plans; LICENSE files of
each repo.
