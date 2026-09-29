# Task: TODO sweep (2026-09-29) — supervisor board

Baseline: ./test.sh → 611 passed, 0 failed, 132 ignored. Issue #2 closed (server file restored).
The supervisor session owns this board and assigns ONE task per agent at a time; agents do not edit the board.
Status: open | assigned <session> | review | done <commit> | parked (reason)

## High — ignore "next" / known wrong results
| id | task | status |
|---|---|---|
| A1 | tests/test_law.rs:52 `:=` functions compile `x:float` params as Int, `half(1.0) == 0` | done 768e8a87 (bundled into C2's commit by mistake)
| A2 | tests/probe_footguns.rs:129 `(x*x) as i64` panics inside a function body | done a0b29543 (ignore was stale, already fixed earlier) |
| A4 | footguns: `1/4+1/4` → 0 (sum typed Int) — verify first | done e5934b38 (already fixed; fixed float locals in function bodies instead) |
| B4 | footguns: `f := it*10; 1 + f 3` → 31; `x=[1 2 3]; x[3]` returns program text — verify first | done 872063f4 0b795bf9 (stale, already fixed; duplicate tests sent back) |

## Medium
| id | task | status |
|---|---|---|
| B1 | tests/test_functions.rs:139/145/151 `:=` vs newline precedence | done f4b601c0 (already worked, newline tests added) |
| B2 | tests/test_angle.rs:35 function application without parens precedence | done 872063f4 (already worked) |
| B3 | tests/test_string.rs:221 statement sequences `'hello';(1 2 3 4);10` → 10 | done de18c3bb — heuristic rule, needs user confirmation (1;(2 3);4 → 4 vs wiki/list.md:103) |
| A3 | tests/test_math.rs:99, :287 ignore "soon" (mixed-type variables, automatic casting) | done 0768b809 (ignores were stale) |
| C1 | src/main.rs:208 compile-only CLI path writing the .wasm | done bc52e71c 1d260145 (after one review round) |
| A5 | float local read in an exact context truncates silently (emit_truncated_float) — loud would break existing tests | survey fd9dad15 (notes/float_truncation_survey.md): 4 silent wrong results; rule implementation → warp-f6: step 1 40ac69a3, step 2 506e6b94, step 3 a8f17bdf, pow fd4557dc, step 4 bab7a22b — done; A5b ccaf3da9 ecc7847c; A5c: floor in function body 9be02d9b; bit/logical ops 7a8bd840, survey outcome 73e6203a — done; shifts don't exist (decision 14a) |
| A6 | stale #[ignore]s: A2/A3 showed ignored tests that already pass — run all ignored tests, un-ignore the passing ones | done c519d7fe..bfcd81bd (10 un-ignored, 657/0/123); A6b: none newly passing; 67 ignored tests still fail (probes/ignored_run.log) |
| A7 | notes/footguns.md plain-bugs list stale (6 of 7 fixed) → pointers to regression tests | done 9a436c80 |
| C2 | src/node.rs:412 `Text("TODO: …")` placeholder → real implementation or loud error | done 768e8a87 (Node::todo has no callers) |

## Low
| id | task | status |
|---|---|---|
| C3 | tests/test_node.rs:7 Node `remove`; tests/test_node_operators.rs `Meta` not exported | Meta: not applicable (old API gone, comment kept); remove: done e88399a6 (verified by warp-numeric) |
| B5 | semicolon: block literals ({},(),[]) are values keeping all items; running a block (root, function body, if/while) yields its last item (wiki/list.md); replaces de18c3bb heuristic. Step 1: survey conflicting tests | user decided: block literals are values, running a block yields last item; test_lists.rs:205 may change → done ff9b2d62 b2f931d1 |
| B6 | bare name of a function with parameters (`sq:=it*2;sq`) → silent 0, should be loud | done 7e2e746c 417f3fed |
| B7 | test_angle.rs test_switch / test_switch_evaluation: object key lookup {a:1 b:2}[a] | done 82bd757f 72c51ed5 |
| A8 | test_math.rs superscript powers 3⁴, vulgar fractions ⅓9 | done d184af4f |
| C6 | test_meta.rs 5 ignored failing tests (Meta API) | test_parent_context done ba38aea1; @ attributes 74383853 553b398c, IndexMut 1e82fd43; tag form c1b29ed6, meta serialization 5202803b — all 5 done (696/1 expected/114), IndexMut meta, meta serialization → warp-library |
| B8 | test_lists.rs 6 ignored failing array tests | parked: tests contradict recorded decisions (size in bytes, checked index assignment); typed-array declaration + while value need user |
| B9 | `while cond: body;rest` colon body swallows `;`; compiler panic (unwrap undefined variable) → error value | part 1 done 1ca29159 (while-colon → Do); while(cond) body abaf6712 (test_wasm_while2 un-ignored); part 2 restored after wipe, patch probes/b9_part2.patch, waits for user OK to replace should_panic test probe_footguns.rs:56 |
| B10 | `i--` lexes as a kebab-case symbol (test_wasm_while2) | done 2bc61ab1 |
| B11 | `for i in 1..3: i` unimplemented (returns unevaluated) | done 99d752df (for → while lowering, src/for_loop.rs) |
| A9 | `3²+1`, `x²+1` → 'cannot extract a numeric value' (postfix Square/Cube in arithmetic) | done 1a815cfc |
| A10 | juxtaposition `3x` → 3*x (wiki/number.md), today a list; `1½` → list | survey done: 0 regressions; decisions: 1/2x grouping, ordinals 2nd, 2i/2e, spaced 2 km |
| A11 | triage remaining ignored test_math tests (units, sin, primitive types) | f2cda334 (type words double/long, typed params); rest needs design: units+±+ranges, data-as-scope kebab keys, test_sin exact float eq (test defect), C-style decl blocks, use <file> modules |
| C7 | triage ignored test_wit (nested separator groups) and test_wast tests | done eb31f69d 2318c301 aa52fedd (test_wit_parse); test_wast needs polish-notation decision |
| A12 | unify missing_functions and missing math imports emission rerun into one mechanism | assigned warp-f6 |
| C8 | empty {} blocks dropped (`x i {}` loses the block) | done 207e4dfd, $main eval test 16594fbd |
| A13 | triage ignored test_operators (1), test_todo (4) | done: all design-blocked (notes/open_decisions.md) |
| A14 | triage ignored test_wasm.rs, in slices | slice 1: a84384e0 (16 un-ignored) 7cccd61f (global x); slice 2: globals 8e726057+cd0aebda (named_data_sections: exit(0) in test, decision); slice 3 2c39f1a3 (test_globals, math_operators_runtime) — closed; rest blocked (design/host/test defects/B9(2)); (a) global modifiers 12a70b5d (warp-parser); B13 export declarations → warp-parser |
| B12 | colon body parsed at `:` precedence: `while c: i+=2`, `if c: x+=1` crash; remove for_loop re-attach workaround | done d9989daf (+ repairs 4add7843 e027886f) |
| C9 | `foo:=it#1;foo [1 2 3]` traps: `it` holding a list can't be indexed | done de780e15 (param kind from body indexing) |
| C10 | untyped param silently coerces a list to Int (`foo(x):=x;foo([1 2 3])` → 3) → loud error / argument kind | part 1 08b3f97b; part 2 (all call sites agree else loud) 2a8d7793 — done (its read-tree race reverted d3eb186b, restored a115c2e0) |
| A16 | regression: `download https://…` returns unevaluated list (worked at A14 slice 1) — bisect | no regression: download never existed, slice-1 PASS was vacuous under --all-features; 281abe1a re-ignores test_math_operators_runtime |
| B14 | inline block/`# ` comments break expressions | done d3eb186b a115c2e0; test_comments2 expects C++ length of `y=0` (decision) |
| A15 | survey: unresolved call in code position | done 3ca4b09c (0 test changes with `name(` rule), decision #7 |
| C4 | tests/test_string.rs:143/150 string operator overloads (`"a".s() + 2`) | parked: user doesn't need it, commented lines stay TODO |
| C5 | src/meta.rs:9 conditional compilation | done edd78b86 (LineInfo.line debug-only) |
| L1 | test_web.rs:43 `$b.ok` emitAttributeSetter; :71 Externref kind | parked: needs a real webview host (wry?) + Externref Kind → user |
| L2 | test_types.rs ignored type-system/generics tests | parked (large design work) |

## Rules for everyone
- COMMIT ONLY THROUGH A PRIVATE INDEX, never the shared one (2026-09-29: d9989daf committed a stale shared index and
  silently reverted 7cccd61f; repaired in 4add7843 / e027886f):
    old=$(git rev-parse HEAD)   # FIRST, so a concurrent commit makes update-ref fail instead of reverting it
    export GIT_INDEX_FILE=$PWD/probes/<name>.index; git read-tree $old; git apply --cached --3way <patch>
    git diff --cached --stat $old; new=$(git commit-tree $(git write-tree) -p $old -m msg)
    git update-ref refs/heads/main $new $old && git push; unset GIT_INDEX_FILE
  Build blobs by 3-way merging onto CURRENT HEAD (git apply --cached --3way), never whole files from an older export
  (8e726057 reverted de780e15 that way; repaired 75da373b, cd0aebda).
  then verify a clean export (git archive HEAD) with your own CARGO_TARGET_DIR and `cargo test --offline --all-features --no-fail-fast`
  (test.sh uses --all-features; cfg(feature) test bodies differ between modes). Probe ignored tests in both modes before un-ignoring. Never rm anything (user rule): reuse/overwrite your export dir.
- Read CLAUDE.md first. Run ./test.sh before and after each fix; a fix is done only when the formerly ignored/commented test passes.
- NEVER modify or delete existing tests. You MAY remove an `#[ignore…]` attribute or uncomment a line marked TODO once it passes;
  replace that TODO word with DONE, keep the rest of the text identical.
- New tests go into tests/ (one per feature), experiments into probes/. No mock tests.
- Code style: no duplication (extract functions for repeated >2 lines), meaningful names, no trivial comments,
  constants at top of file, no ad-hoc special cases — find the general rule. Remove debug prints when done.
- Git: stage only your own files by explicit path, never `git add -A`/`commit -a`; `git pull --rebase` before push;
  never reset/stash/revert others' work, never cargo clean. Conventional commits (fix:, feature(minor):, refactor:, test:),
  no AI attribution/co-author lines. Commit + push each fix separately, then run `cargo fix --allow-dirty` and commit again if it changed something.
- Commit test_results.txt only if your change caused its diff.
- Shared checkout: edit only with the Edit tool, never scripts that rewrite whole files (2026-09-29: a script truncated
  wasm_emitter/mod.rs and wiped another agent's uncommitted B9(2) hunks). Work held uncommitted for a decision → also save a patch in probes/.
- After each commit verify HEAD compiles AND its new tests pass on their own (a test can pass in the shared tree only thanks to
  someone's uncommitted hunks: bab7a22b vs B9(2)). Verify HEAD compiles on its own (git archive HEAD | tar -x -C probes/head_check; cargo build --offline --tests)
  (2026-09-29: abaf6712 committed a call without its fn definition, main did not build until b8f9f79b).
- If a break comes from another agent's in-progress edit, don't fix it — note it and continue.
- If something can't be solved in reasonable time, record the difficulty in notes/footguns.md or a GitHub issue (gh) and move on.
- When done: append a short summary per item (fixed / left open + why) to the bottom of this file under "## Results", commit, push.

## Results
