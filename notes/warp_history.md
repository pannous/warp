# How warp reached self-build, and what wap can take from it

Archaeology of the old compiler (core/, then lib/compiler/), 2026-11-18, for the wap self-build (milestone 2,
notes/backbone.md). Read from git history only; no code changed.

## Verdict

warp went from its first warp-written compiler part (7d190f1699, lexer.warp, 2026-10-13) to compiling itself and
running Hello (3676dfd372, 2026-10-21 14:30) in **8 days**, and to a fixpoint and a Rust-free rebuild (dc4114afaf,
2026-10-26) in **13 days**, with a fleet landing 500–1000 commits a day. It got there fast for three reasons, and
paid for two of them afterwards:

1. **A complete working compiler was always behind it.** Every stage (parser, passes, analyzer, emitter) was swapped
   in on its own, behind the Rust pipeline, and a program the warp stage could not handle fell back to Rust
   *per program* (`outside the core: …`). The warp parts never had to be complete to be measured.
2. **Untyped values were legal.** The emitter boxed any value whose kind it did not know as a `$Node` and checked it
   at run time (`node_truthy`, `list_of_node`, `int_admitted`). The biggest family blocking the compiler's own
   source ("a value known only at run time", 37 functions) went away that way, without any whole-program typing.
3. **The refusal unit was the function, not the file.** The bootstrap-gap survey compiled on past each refused
   function, recorded its first refused form, and ranked the families (262 → 185 refused functions in a day).

The price came after: the compiler's words lived in one namespace, two typing mechanisms (the emitter's own guesses
and analyzer.warp's kinds) had to agree, and pass order did the job that declared stage languages do in wap. From
10-29 to today 6643 commits followed, 2242 of them fixes; the fix-to-feature ratio rose from 1.5 to 2.7 a week, and
core + extensions grew from 21k to 50k lines.

**For wap:** it is one day past Hello (b5a5029b3d, 11-17). The census types no file yet because typing runs per
file, so one missing preload word (`line_of`) stops 15 of 21 files. Typing and emitting **per function**, with a
refusal ending only that function, is the cheapest speed-up on hand. A boxed `any` with run-time checks, kept as a
debt in notes/backbone.md, is the second. Module scoping and overload choice have to be designed up front, because
those are where warp patched the longest.

## 1. The route (10-13 → 10-26)

Background: warp already had a full language and a mature Rust compiler (since 2024; at 10-12 parser 7.4k Rust
lines, analyzer 6.6k, emitter 23k, lowering ~59k; old notes/self_hosting.md "Size of the job").

| date | step | evidence |
|---|---|---|
| 10-13 | self-hosting postponed for a Rust cleanup, but the scaffolding starts: differential "twin" harness, first twin `line_widths` | 1637d47653, d9af8cbe21 |
| 10-13 | lexer.warp matches its Rust twin on every source; parser.warp from atoms to 994 of 1366 sources parsing as Rust does; serializer.warp | 7d190f1699, 54c0147c77, 6be3b155ac, c752e31c2f |
| 10-14..16 | Rust cleanup (emitter split into folders, twins merged) | def6c077a7, def10d6d5f |
| 10-17 | self-hosting resumed; readiness survey: no macros first | 0d60168c11; passes.warp 4529c1ae84, emitter.warp 68ba41c4d9, analyzer.warp 2aa4f76efa, runtime.warp cf809c61b1, all created that day |
| 10-17 | first suite count through the warp front end: 4529 / 6421 (70.5%) | ceb59ae72e |
| 10-18 | WARP_SELF_HOSTED runs emitter.warp; a program outside its core goes to the Rust emitter | fc0f93e894 |
| 10-19 | bootstrap-gap survey: the refused functions of lib/compiler, ranked by family | b2619c2017, a5182dc4f4 |
| 10-19 12:50 | **first pure run**: parser.warp → warp passes → analyzer.warp → emitter.warp, no Rust pass between | b37c31bf7c, 18ed0c9983 |
| 10-19 | `pure-only`: no Rust behind it, a refusal is the program's error | d76bb0b537 |
| 10-20 | compiler.warp, all stages as one warp program; the 11 top-level words two files both defined renamed apart | d5f7d40d3e, e7a6b65699 |
| 10-20 | `warp pure`: compiler.warp compiled by the Rust-built stages, run on a program | 5abc5f5c09 |
| 10-21 14:30 | **compiler.warp compiled by itself runs Hello** | 3676dfd372 |
| 10-21..22 | the self-compiled compiler on plain wasmtime with WASI alone; self lane parity work | 1b48191157, f4a535c8cb |
| 10-26 13:10 | **fixpoint** (stage 3 == stage 4): three small fixes, 49 lines | d242ac4fb3 |
| 10-26 13:16 | a seed (a self-built compiler-self.wasm) replaces Rust: `warp rebuild` = seed → A → B | ae3d21c8a8, dc4114afaf |
| 10-26 16:24 | Rust compiler deleted from the harness | 475d65f8c7 |
| 10-26 21:48 | user: no fixpoint needed, compiling itself once more is enough | 8a96f086c4, e4b1292f01 |
| 10-29 | Rust harness gone | d115331edf |

**The order of the stages:** front first (lexer, parser, serializer, matched against Rust on every source), then
passes, analyzer and emitter together (created on one day, 10-17), the analyzer handing kinds to the emitter
(`emitter_kinds`), then one composing program (compiler.warp), then the self-compile, then the fixpoint and the seed.
The bootstrap waited only for "13–15 cover the compiler's own subset" (old notes/self_hosting.md "Next phase").

**The subset:** formally, the full language (decisions compiler-high-level, no-primitive-rewrites: "the lane grows
to the language, never the other way round"). In practice the compiler's own source was adjusted wherever it
tripped the young stages. Examples: 11 clashing top-level names renamed (e7a6b65699), a local `module` renamed
`made` (5191556c3b), `asserted` → `tested_as` (8a3d7fc6ec), `" " * n` replaced by a loop because the seed refused
char * int (39764917d1), constants of runtime/wasi.warp as functions (5e2384e9c6). The old notes listed these traps
("Traps writing passes in warp": a local named like a global function silently meant the function; a word ending in
`{…}` needed `def`; `[]` was ø).

**The tricks that made it reachable:**
- *Rust as a per-stage and per-program fallback:* lanes `front`, `emit`, `pure`, `pure-only` (WARP_SELF_HOSTED);
  any stage could be warp or Rust; stage programs were compiled by Rust and cached (old self_hosting.md "The suite
  through the warp pipeline").
- *Twins:* a warp part compared with its Rust original over ~1300 sources (tests/common/twins.rs); scaffolding
  only, dropped once both sides were warp, and abandoned for passes on 10-19 (decision ignore-rust: judge by results).
- *Loose typing in the emitter:* int/float/bool unboxed where known, everything else a boxed `$Node` with run-time
  truth, list and text unboxing and admission checks (d54957ead8, 959869e961, 6717847d15). A parameter took its
  annotation, else the kind of its first call's argument (old self_hosting.md "Emitter core").
- *A minimal emitter that refused loudly:* "a form enters emitter.warp's core only when it compiles right; unsure →
  refuse (Rust takes it)" (old self_hosting.md "Back-end focus"); runtime words in warp over ~25 one-instruction
  intrinsics.
- *A seed:* a self-built compiler checked in once the fixpoint held, so the rebuild needed no Rust (ae3d21c8a8).

**How progress was measured:** the suite count per lane (4529/6421 on 10-17); the pure-lane ladder of simplest-first
programs ("21 ran / 17 right → 31 / 30", refusals.sh); the bootstrap-gap survey of refused compiler functions by
family with an owner per family (a5182dc4f4); then a per-file survey over lib/compiler (data/self_hosting/,
notes/parser_compiles.md: parser.warp pure-only 160 → 37 → 0 refusals, f40fe787c0); the pure chain checked on every
push (d384d07370).

## 2. The architecture at dc4114afaf against wap's plan

Size at dc4114afaf: lib/compiler 20,756 warp lines in 114 files (+ entities.tsv): top level 15,791 (parser.warp
4,429, rewrites.warp 1,524, emitter.warp 1,272, analyzer.warp 998, lexer.warp 461, then ~60 pass files of 100–400
lines), emitter/ 1,790, runtime/ 2,189 + runtime/extended/ 973. Rust: still 524 .rs files in src/ (the harness).
Today: core/ 33,423 (passes 10,830, emitter 6,296, native 4,575, parsing 4,366, runtime 4,040, analysis 2,389),
extensions/ 17,348, lib/ 27,577. wap today: 8,149 lines.

| | warp at dc4114afaf | wap's plan (notes/wap.md, notes/backbone.md) |
|---|---|---|
| reading | parser.warp 4.4k lines, keyword knowledge inside, some sugar already moved to a table pass (sugar.warp) | a pure notation reader, no keyword (core/parse/), operators by a table |
| lowering | ~60 passes, ordered by hand; each sees whatever the previous one left | three rule stages with declared input and output languages; a form outside the language is refused naming the stage |
| names | one namespace over all compiler files and used modules; a clash replaced a word silently | one binding analysis (core/analyze/scope.wap) writing facts |
| typing | two mechanisms: analyzer.warp `emitter_kinds` and the emitter's own first-call inference; boxed `$Node` for anything unknown | one bidirectional typing stage; monomorphised Define per argument-type tuple; refusal where no type is found |
| emitter | per-form refusal "outside the core"; runtime words in warp over intrinsics | one case per kernel form, any other node refused; values.wap reuses the same layout |
| facts | positions lost across stage bridges (JSON views), `["at", line, col, view]` wrappers | facts in Meta, carried by one walker (core/stage.wap) |

Cleaner at dc4114afaf than people remember: the value layout (typed inside, boxed at the edges), runtime words as
plain warp over intrinsics, WASI-only output, the single composing program compiler.warp. wap reuses the first two
as they are.

Already messy: the 4.4k-line parser; pass order as an unwritten contract; two typing mechanisms; the global
namespace; the JSON view bridges between stages, which dropped positions (old self_hosting.md: "position-checking
tests fail by design").

## 3. What changed after (10-29 → 11-18)

6,643 non-merge commits; 2,242 `fix:`. By week:

| week | commits | fix | feature | fix/feature |
|---|---|---|---|---|
| 10-29..11-02 | 1,464 | 403 | 264 | 1.53 |
| 11-03..11-09 | 2,548 | 928 | 362 | 2.56 |
| 11-10..11-16 | 1,956 | 758 | 279 | 2.72 |
| 11-17.. (wap starts) | 675 | 153 | 240 | 0.64 |

core + extensions: 21.2k (10-26) → 30.4k (10-29) → 34.4k (11-03) → 43.6k (11-08) → 49.3k (11-13) → 50.4k (11-17).
Each feature cost more fixes as time went on, and the code grew 2.4× in three weeks.

Fix commits by mechanism (subject grep, overlapping): typing/kinds 197, rule conditions/guards 138, scope/binding
106, overloads/arity 104, sugar/rewrite rules 94, name clashes 30, pass order 14. The seed is named in 62 commits:
a feature the seed lacked meant a workaround or a two-step landing.

The chains that show patching:
- **scope and shadowing:** a rule's conditions not seeing a lambda's locals (5f63e9a5f7, 11-02); annotations
  meeting words bound as values (7c261df764, 11-03); a local named like a charged value (7286163d84, 11-08);
  host-module locals writing another file's global (30f49cc4e6, 11-12); `;` lines not shadowing module words
  (d779d8a0e5, 11-14); a used module's variable as a local (dca67a110b) and a program's class shadowing the
  preload's (362247b8f3), both 11-18. One rule is missing behind all of them: which binding a name means.
- **overload choice:** a program's operator joining a module's instead of replacing it (6b296786b5); a typed
  overload read across a comparison because `arity_overloads` ran before `prefix_calls` (728e995703); a typed
  parameter's kind not used in the choice (6cbacedb6f); `interpret_scope` having to run before `arity_overloads`
  (85a9419447); a user limit of "three lib lines or none" for text overloads of floor/ceil/round (4762e6ea9a).
  Overloads were resolved by passes before the types were known.
- **rule conditions:** guards placed in one more spot each time (117c260048, ff4c6a839c, a3f0eecdf5).
- **emitter typing:** e.g. a declared result versus a first reading (3ed3c76022, 10-21), a widened local
  (29da48730a), number/optional parameters held as nodes after the analyzer grew annotations (5af1a597c9), a declared
  bool trapping on 0 (b5092fb3d6). Two typing mechanisms meant every new annotation needed both to agree.
- **seed lag:** a lambda compiled by an old seed trapping (be14b0308c), ÷ not canonical because the seed could not
  read it (569571a744), a Unit class waiting until the seed read `$u is Unit` (aab691bb02), the GPU code avoiding a
  core word the seed lacked (db9410dbe8).

## 4. Lessons for wap

Ranked by how much they speed up milestone 2.

1. **Type and emit per function; a refusal ends that function only.** warp's survey compiled on past each refused
   function (b2619c2017) and ranked families by how many functions they blocked; families were cleared by the
   dozen (a5182dc4f4: 262 → 185 in a day). wap's census types per file, so `line_of` missing from the preload stops
   15 of 21 files (notes/wap.md "Road to self-build"), and the gauge hides everything behind that word. Typing a
   function whose callee is missing as refused, and typing its siblings anyway, turns the census into a ranked list
   at once.
2. **Allow a boxed `any` with run-time checks for the self-build, as a debt.** warp's top bootstrap family was
   untyped values (37 functions), cleared by boxing and run-time admission (d54957ead8, 959869e961). That gives a
   right value or a loud trap, never a guess, so it is no breach of "refuse, never guess". wap already has `any`
   (core/stage.wap, used in core/analyze/types.wap:46). The cost is known from warp: 197 later typing fixes when
   static kinds and boxed values had to meet. Keep it to one mechanism (types.wap decides boxed or not; the emitter
   never infers), which avoids warp's double-mechanism mess. Whether to do it at all is a question for the user
   (sent to warp-interviewer).
3. **Design module scoping and shadowing now.** warp's longest patch chain (section 3) and its self-build traps (a
   local named like a global meant the global; e7a6b65699 renamed 11 clashes just to compile itself) come from
   having no binding rule for module vs program vs preload vs local. notes/backbone.md "Open" says a name two
   modules define is read as overloads of one name and `m.f` is not built. That is warp's clash rule again; decide
   the precedence (local > parameter > program > used module > preload, with a loud clash between two modules)
   before wap's own 21 files depend on it.
4. **Choose overloads in typing, never in a pass.** warp chose arity overloads before the types and before phrases
   were read, and patched the order (728e995703, 85a9419447). wap's plan already does it right (types.wap and
   calls.wap choose by argument then expected type); keep every rule stage from choosing.
5. **Don't chase the byte fixpoint early, but keep it on the list.** Once warp compiled itself (10-21), the
   fixpoint took three fixes in 49 lines (d242ac4fb3), and the user then dropped it as a requirement (8a96f086c4).
   wap's milestone 2.3 (wap2 == wap3) is cheap once 2.1 and 2.2 hold; put no work into it before.
6. **Keep the compiler's own source small, and measure it.** Every feature warp's source used had to exist in the
   seed first (62 seed commits, e.g. 569571a744, 39764917d1). wap's rule "preload/sugar/ is exactly what wap's own
   source uses" (notes/wap.md) is the right guard; extend it so the census also fails when wap's source starts
   using a spelling the current wap1 cannot compile.
7. **Measure compile time from day one.** warp's loop was dominated by compile time: a cold compile took 2–5 min,
   a fixpoint stage 1015–1255 s (rust-bootstrap handover, a824f4e4db), and seed refreshes went 737 → 216 s only
   later (ca97f10a9b, dca5687125). wap's "under a minute" target (milestone 2.5) belongs on the census line now.
8. **A fallback compiler per program is what made breadth cheap; wap has one already.** warp's Rust lanes needed
   JSON view bridges that lost positions and cost weeks. wap does not need such lanes: the frozen warp compiles wap,
   and the census runs wap's stages file by file. Building bridges between warp's and wap's stages would cost more
   than it saves; don't.

Designed up front in wap already, and confirmed by warp's history: declared stage languages (in place of pass order),
facts in Meta carried by one walker (in place of the view bridges that dropped positions), one typing stage (in place
of two), the reader without keywords (in place of the 4.4k-line parser).
