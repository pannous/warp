# Extension and approval review, 2026-11-22

Read-only review of the 24 hours before 2026-11-22 17:00, user request.

Scope:
- **wap** (pannous/wap): origin/main d16c11b019..e99063f (275 commits), plus the open branches touched today.
- **warp** (this repo): 25 commits.

Counts are `git diff --numstat` lines. "Size" is wap's size ceiling in nodes (test/wap ratchet).

## Totals

| repo | layer | + | − | net |
|---|---|---|---|---|
| wap | wap/core | 499 | 246 | **+253** |
| wap | wap/kernel | 0 | 0 | 0 |
| wap | wap/sugar | 33 | 14 | +19 |
| wap | wap/extensions | 828 | 413 | +415 |
| wap | wap/types (library) | 378 | 57 | +321 |
| wap | wap/compiler.wap | 34 | 8 | +26 |
| warp | extensions/ | 1 | 1 | 0 (VERSION bump) |
| warp | core/ | 0 | 0 | 0 |

- **warp:** nothing to review in extensions or core. The fleet is on wap only.
- **wap's size ceiling** went from about 110.7k to 117,250 nodes (+5.9%) in one day. It was raised about 20 times, each raise citing an interviewer OK.
  - No single raise was large, so no single OK looked large. Nobody looked at the sum.
  - The ceiling exists to make growth visible. Raising it on every landing turns it into a log, not a limit.

## 1. Extensions and core growth, judged

Ranked by impact. "Shape" is the more elegant form I would ask for.

### 1.1 Native memory model: a core switch for one target (core ≈ +80, types +190, extensions +100)

**Commits:**
- native-lists 676b1c2
- memory_texts 611d8a0
- native-records 835a359 / merge ed0b496
- fold f39a3b6
- C crossings 965fea2, 9827917, 50b888c
- out-pointer WIP fd60634, 35e8110

**Facts:**
- `language.values_in_memory` (core/language.wap:28) is branched on eight times across four core files:
  - emit/values.wap:39, 60, 185, 187, 193
  - emit/places.wap:28, 34
  - analyze/data.wap:36, 94
  - emit/emitter.wap, emit/wasm.wap
- analyze/data.wap:15-19 hardcodes the library words `reserved`, `extended`, `record_reserved` and `field_stored`.
- Two DEBT notes already admit the missing mechanism (data.wap:46, places.wap:12 `FIELD_BYTES` twin of `ITEM_BYTES`).
- **Verdict:** a special case for one target inside a semi-frozen core. It works, but it should have been a hook.

**Duplication:**
- memory_lists.wap and memory_texts.wap (74 lines each) share idioms, not lines:
  - three address casts: memory_lists:17, :73 and memory_texts:15
  - two byte-copy loops: memory_lists:39 and memory_texts:47
- The shared allocator `memory_taken` sits in the texts file.
- types/c.wap and host.wap branch on a flag `texts_in_memory()` (c.wap:26, 52, 98; host.wap:49), when the two swappable files could supply behaviour instead.
- extensions/c/foreign.wap spreads each C type over four parallel rule tables (`c_kept`, `c_argument`, `c_type`, `c_back`).

**Shape:**
1. **Hook table, not switch.** Core asks the language for "the literal builder of a kind" and "the layout of a type word". GC is the default; extensions/native registers the memory builders and I32 layouts. Removes all eight branches and the four names, and pays both DEBTs.
2. **One memory sequence** `[count][room][block]` parameterised by item width (1 for bytes, 8 for items), in a small types/memory.wap:
   - A text is a byte sequence plus NUL.
   - A record is a fixed-room sequence.
   - One cast, one copy loop.
3. **Behaviour, not a flag.** Both text files define `fresh_bytes` and `c_view`, and the `if texts_in_memory()` branches go.
4. **One rule per C type** in foreign.wap, giving its whole crossing.

### 1.2 Library linking: three lookup orders and silent fallbacks (core +36 +20 +13, extensions ≈ +260)

**library-compiled-once-2 (75f342b):**
- `is_library` compares a file name with TYPES_FILE (extensions/wasm/library.wap:45) and is passed into core emit (compiler.wap:72).
- Core emit gained `tag_module` and `tag_shared` just to share the library's raise tag (emit/emitter.wap:57-61).
- **Shape:** every used module shares its raise tag the same way, so nothing names the library.

**compile-library-programs (7a7402e, e99063f, landed today):**
- `library_file` (library.wap:23-26) takes a `types.wasm` beside the compiler whenever one exists, without reading or hashing the sources.
- Edit wap/types/*.wap in a tree with an old types.wasm and every compile quietly uses the old library.
- The stamp check that used to say "is of other source" is gone.
- The comment at compiler.wap:62 ("where its stamp fits") is now false.
- This is exactly the silent compensation the user forbids.
- **Shape:** embed types.wasm in the compiler as `WARM_STATE := embedded("warm.packed")` already does. One mechanism, no loose file to trust, no drift possible.

**wap.compiler section (3294b66, 718e709, 09cc56d):**
- Each module records the absolute folder of its compiler (library.wap:57), so output depends on the build path.
- When that folder moves, linking quietly falls back (extensions/native/linking.wap:26-29).
- The compile-time lookup (core/analyze/declarations.wap:86-98: importer's folder first, then lib/) and the link-time lookup (linking.wap:22-32: compiler's folder first) search in **opposite orders**. 09cc56d flipped only the link-time one.
- `"wap.compiler"` is spelled twice (library.wap:19, linking.wap:9).
- **Shape:** resolve once at compile time and write the resolved path as the import's module name. The linker then only needs "a name with a slash is a path", and `recorded_folder`, `leb_at` and `leb_end` go (about 40 lines). One lookup rule.

**module-init (a079ae9, 806b4e4):**
- `warp/start` is right.
- The `Prim(shown, …)` marker exists only so the emitter can cut off the shown value (core/emit/functions.wap:17-19, analyze/calls.wap:12).
- **Shape:** compile the top level into warp/start and let `main` add the show. No marker, no split.

**finished() (3460857, extensions only):**
- A second lifecycle hook, found by a magic export name only the native host knows.
- `finisher_count < FINISHERS` silently drops the 17th module's hook (extensions/native/run.wap:207).
- **Shape:** `warp/finish` beside `warp/start`, one naming rule, and refuse loudly past the cap (or no cap).

**export-from-library (ca23c65):** core +20/−4. The `export` keyword stripping (`unexported`/`export_stripped` in core/analyze/declarations.wap) could be a sugar rule that tags the definition, leaving core only the EXPORTS fact.

**use-subfolders (a60bdff):** core +13. Needed, since `module_named` is core. `import`/`require` as aliases (operators.wap:160) is fine.

**Engine-settings cache hash (5dc91d9, 0378b9d) and memory/time caps (86f0b59):** clean, extension only. No objection.

**Minor:** `BASE_EXPORTS := 3` (run.wap:244) is a magic count that the library's extra export `raised` already breaks in spirit.

### 1.3 Core language changes (wap/core, ≈ +130 of the +253)

| item | core ± | verdict | shape |
|---|---|---|---|
| spaced-is-two (35bba1d, 25121a6, b2bde9b) | +51/−28 | **guess, special case**: see 2.1 | one reading; refuse where `w` is a function or rule head |
| measure-group (b36edc6, 086483a, 32f840d, 4f5475d) | +12/−8 | needed, but patched three times: `word_measures` (core/parse/operators.wap:230) now excludes operators, rule words and holes, and core/format.wap:120 repeats the test | units declare their words as postfix operators of the measure row; no exception list, `5 apples` stops measuring by accident |
| retyping (ac06c29, 920a99b) | +36/−10 | user decision; heavy: restarts typing for the whole program per round (analyze/types.wap:190) | narrow the rerun to the retyped name's users |
| field-default (adba601…07473f9) | +18/−8, sugar +16/−10 | needed, but a second path beside parameters' DEFAULT fact (analyze/shapes.wap:51, 55) | one rule "a missing type comes from the default", shared by parameters and fields |
| settled-instance (d6bed40) | +16/−5 | fix in patch shape: a `settled_calls` side table rewriting DEFINED_AT (analyze/letters.wap:59, 92) | a deferred call owns its instance and takes DEFINED_AT once when it settles |
| list-separators (bd20c93) | +3/−2 | special case by bracket: `bracket == Bracket.Square or` (core/rules/matcher.wap:53) | separators never decide data-list matching, for any bracket |
| expected-case (43ab9c2) | +5/−1 | repeated: two `expected_case` calls (analyze/types.wap:432, 437) | one call before the branch |
| none-list (883c081) | +3/−1 | same rule twice (analyze/data.wap:91, types.wap:381) | "ø alone is an optional of an unknown" in one place (`common_type`) |
| map-base (8aaa42d) | +4/−3 | needed; the `layout_of` if-chain (emit/values.wap:62) grows one arm per kind | a kind → base table |
| void-continuation (977bfb1, 40fdd43) | +23/−21 | general (reads the rules' inner phrase words, no then/else named) | ok |
| enum-case-as-type (89d2012) | +16/−12 | needed, a net simplification | ok |
| reader-names-rows, typed-template-forms/-names | small | general sibling fixes; heads.wap +37 is a move out of stages.wap | ok |
| push→add, prefix-root √, host-io, wap-units, simplify-27 | core 0–1 | library, sugar or alias | ok; but `rule (public $head := $body)` (sugar/definitions.wap:21) is the third copy of a modifier rule beside lines 20 and 25: make public/const/global one rule |

### 1.4 Open branches

| branch | core | note |
|---|---|---|
| native-plain-enum | emit +23/−8 | Native enums as i32 tags: a general fix, fine. But see the next row. |
| native-plain-enum and keyboard-shortcuts-arrows | 0 (each adds its own extensions/native/editing.wap) | **Two branches each wrote their own console line editor** (252 vs 263 lines, 186 diff lines apart). keyboard-shortcuts-arrows (40e2230) first turned the edit enum into ints "as a native build keeps no enum's case": degrading the input to suit a tool. native-plain-enum then fixed the tool and turned it back. One must be dropped before either lands, and the int workaround must not land. |
| any-values | analyze/dispatch.wap +70 | Run-time dispatch for `any`: the 11-21 generic-fallback decision, the right place. |
| general-overload-mechanism | +10/−9 | Small. |
| one-memory | +9/−3, awaiting OK | Would make 1.2's tag and travelling-words parameters unnecessary. **Recommend: OK it, then delete the tag-sharing special case.** |

## 2. The interviewer's approvals

Today about 30 OKs are cited in wap merge messages and ceiling commits. Most are small sibling fixes and were right to wave through:
- typed-template-forms/-names
- reader-names-rows
- void-continuation
- enum-case-as-type
- simplify-27
- wap-units
- the engine-settings hash

The pattern problem is that the OK is given **per landing, counted in nodes**, and nothing checks the shape of the change:
- None of the OKs below asked "could this be a hook or a library function", though AGENTS.md makes that the first question.
- Several OKs approved the line count of a WIP ("1190 approved", then +3, +3 more for lines that keep a file under 300).

The ones I would have refused or sent back:

### 2.1 spaced-is-two (b2bde9b "(warp-interviewer)", merge 831195e): refuse

- The user decided (notes/decisions.md:40): "`style {…}` is two items [style, {…}] with a did-you-mean; glued `style{…}` is the call."
- What landed adds "**unless a typed rule takes the call**" (core/rules/typed.wap:50-56, `parted_rewritten`). The same text is now a call or a list depending on which rules are loaded, and a function-backed word reads differently from a rule-backed one.
- That is two readings decided silently, against "refuse, never guess".
- It is also a change to a user decision that the interviewer signed alone, while the merge message presents it as "user decisions 11-22".
- **Should have been:** exactly the user's rule, or the question back to the user with the cost.

### 2.2 library-compiled-once-2 / compile-library-programs (wap#42 step 2): send back

- The user OK'd the goal (compile the library once). The interviewer then OK'd a shape that:
  - names the library in core emit (`is_library`, tag parameters);
  - trusts a loose types.wasm without a check (1.2).
- **Should have asked for:** the embedded-module shape the warm state already uses. Or the one-memory branch first, which removes the tag special case.

### 2.3 native-lists / native-records (OK 11-21, 11-22): send back for a hook

- Eight core branches on one flag, and four library word names hardcoded in core analysis.
- The DEBT notes prove the authors knew the hook was missing. An OK with a DEBT is a loan the core never repays under a semi-frozen rule.
- **Should have asked for:** the hook table (1.1 shape 1) as step 1, then the memory layout in extensions/native.

### 2.4 list-separators (OK 11-22, +7): refuse as worded

- `bracket == Bracket.Square` is a special case for one sibling.
- The general rule (separators never decide data matching) is the same size.

### 2.5 measure-group (OK 11-21, "3 core lines + a table row"): fine at first, should have pushed back on the patches

- The two follow-ups (holes, rule words excluded) were each OK'd as small.
- Together they make an exception list in the reader, copied into format.wap.
- Units declaring their words is the general form.

### 2.6 finished() and module-init: no OK cited for finished()

- finished() is extension-only, so it needed no core OK. But it adds a second lifecycle convention with a silent cap.
- module-init's OK let the `shown` marker into core.
- **Should have asked for:** one `warp/start` and `warp/finish` pair.

### 2.7 Size ceiling raises: process

- Every raise was OK'd separately ("+7", "+122", "+236", "+1190"), and nobody looked at the +5.9% day.
- **Suggest:** the interviewer approves a raise only with a node budget per day or per card. Each raise names what the change could have been as a hook or library function and why not.

## Ranked findings

1. A stale library is used silently: compile-library-programs trusts any types.wasm beside the compiler without a source check. Embed it like warm.packed.
2. spaced-is-two landed with a reading the user did not decide ("unless a typed rule takes it"). It is a silent two-way guess, signed by the interviewer alone.
3. The native memory model is a core switch: eight `values_in_memory` branches plus four hardcoded library names in core analysis. It needs a hook table.
4. Two open branches each wrote their own console line editor (editing.wap), one of them with an enum→int workaround. Drop one before landing.
5. Linking has two lookup orders (compile time vs link time) and absolute build paths baked into modules. Resolve once at compile time.
6. Size ceiling +5.9% in a day through about 20 separate OKs. The ceiling became a log, not a limit.
7. Smaller special cases: list-separators by bracket; the measure-group exception list copied into format; a second default-typing path for fields; the settled-instance side table.
8. Copies: expected_case ×2, the none-list rule ×2, the modifier rules ×3 in sugar/definitions.wap, the address cast ×3, "wap.compiler" ×2.
9. Silent caps: the finished() hooks past 16; `BASE_EXPORTS := 3`.
10. Good: void-continuation, enum-case-as-type, simplify-27, reader-names-rows and the engine-settings hash are general and well placed. The warp repo had no extension or core growth.
