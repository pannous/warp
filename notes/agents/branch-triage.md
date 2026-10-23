# Branch triage (2026-10-24)

Unmerged branches on origin after the merged-branch cleanup, measured against origin/main at a8a824e44.
Checks: `git cherry`, subject match against `git log origin/main`, `git merge-tree --write-tree` against main
(squash equivalence), blob identity, the board (`gh project item-list`, read once: 400 of 1402 items), and main's
merge messages. Nothing was deleted.

Counts: still-needed 81, superseded 15, stale junk 36 (total 132).

## Still needed

| branch | last commit | reason |
|---|---|---|
| `arrow-result-entry` | 2026-10-22 | 2 unique commits within 7 days: chore: probe twin_walk: arrow definitions and of-phrases in  |
| `backup/classops` | 2026-10-23 | 27 lines not in origin/classops |
| `backup/compiler-to-core-prep` | 2026-10-23 | 8 lines not in origin/compiler-to-core-prep |
| `backup/integrate-host` | 2026-10-23 | 12 unique commits, no origin/integrate-host |
| `backup/libgap-as-conversions` | 2026-10-23 | 3 lines not in origin/libgap-as-conversions |
| `backup/libgap-as-units` | 2026-10-23 | 4 lines not in origin/libgap-as-units |
| `backup/pure-big-int-power` | 2026-10-22 | 681 lines not in origin/pure-big-int-power |
| `backup/pure-class-methods-3` | 2026-10-23 | 108 lines not in origin/pure-class-methods-3 |
| `backup/pure-class-methods-3b` | 2026-10-23 | 3 lines not in origin/pure-class-methods-3b |
| `backup/pure-pipes` | 2026-10-23 | 1 unique commits within 7 days: wip: box backup of pure-pipes (2026-10-23 16:10 CEST) |
| `cast-list` | 2026-10-21 | 4 unique commits within 7 days: refactor: conversion_kind's list arm is names_list_type's (r |
| `classops` | 2026-10-23 | 12 unique commits within 7 days: fix: the operator table class_methods reads is a map of its  |
| `code-empty` | 2026-10-21 | 1 unique commits within 7 days: wip: {} in code as the empty map, EMPTY_MAP_KIND (card code- |
| `compiler-cache-keys` | 2026-10-23 | 1 unique commits within 7 days: fix(compiler): a build is judged, and the stages are kept, b |
| `compiler-library-source` | 2026-10-23 | 2 unique commits within 7 days: test(host): an alias added to WARP_LIB's prelude.warp works  |
| `compiler-rebuild-policy` | 2026-10-23 | 22 unique commits within 7 days: Merge remote-tracking branch 'origin/warp-host-wasm-load' in |
| `compiler-to-core-prep` | 2026-10-23 | 2 unique commits within 7 days: refactor(layout): move lib/compiler to core and lib/sugar to |
| `compiling-lib` | 2026-10-22 | 1 unique commits within 7 days: Merge remote-tracking branch 'origin/main' into compiling-li |
| `engine-right-to-left-runs` | 2026-10-23 | 2 unique commits within 7 days: Merge remote-tracking branch 'origin/main' into engine-right |
| `feat/ffi-rust-crate-adapter` | 2026-10-23 | 14 unique commits within 7 days: feat(crates): bytes parameters as pointer and length, byte a |
| `ffi-libc` | 2026-10-23 | 30 unique commits within 7 days: test(ffi): libc checks are positive: c_errno read as a numbe |
| `ffi-python-adapter` | 2026-10-23 | 16 unique commits within 7 days: docs(notes): python_adapter.md: measured after merging pure- |
| `ffi-sqlite3` | 2026-10-23 | 18 unique commits within 7 days: test(ffi): sqlite checks are positive: a try around a failin |
| `ffi-z` | 2026-10-23 | 26 unique commits within 7 days: test(ffi): zlib checks are positive: a roomy output buffer g |
| `fix-in-lib` | 2026-10-22 | 6 unique commits within 7 days: refactor(fix): drop rust routing, launcher routes fix and be |
| `fix-route` | 2026-10-22 | 6 unique commits within 7 days: feat(launcher): route fix and beautify to lib/fix/command.wa |
| `function-value-read` | 2026-10-21 | 1 unique commits within 7 days: Merge remote-tracking branch 'origin/main' into function-val |
| `gap-element-array` | 2026-10-23 | 5 unique commits within 7 days: fix(parser): back to main's applies and the ignored element  |
| `gpu-multipass` | 2026-10-18 | 3 unique commits within 7 days: notes: gpu: render passes built natively (card gpu-multipass |
| `gpu-multipass-web` | 2026-10-18 | 5 unique commits within 7 days: feature(minor): a paint reading an image carries the image's |
| `gwc-proto` | 2026-10-23 | 2 unique commits within 7 days: feat(extensions): safe navigation a?.b as a rule, loaded wit |
| `hint-makes` | 2026-10-20 | card hint-makes is in Now |
| `host-from-warp` | 2026-10-23 | merged once (5c1adff1a), then 19 more commits from the warp-host stack |
| `host-glue-native` | 2026-10-23 | 2 unique commits within 7 days: wip(compiler): cstr literals in an active data segment for n |
| `keys-of-empty` | 2026-10-22 | 2 unique commits within 7 days: Merge origin/main into keys-of-empty |
| `launcher-host-dispatch` | 2026-10-23 | 23 unique commits within 7 days: feat(launcher): with warp-host, run, pure, self, compile, in |
| `lib-compiler` | 2026-10-22 | 5 unique commits within 7 days: Merge remote-tracking branch 'origin/main' into lib-compiler |
| `lib-json` | 2026-10-22 | 1 unique commits within 7 days: chore: document empty-map JSON dependency |
| `libgap-as-conversions` | 2026-10-23 | 10 unique commits within 7 days: fix(compiler): conversions rename with lambdas.warp's rename |
| `libgap-as-units` | 2026-10-23 | 8 unique commits within 7 days: feat(compiler): `3 km as m`, `5 as ms`: a value as a unit, q |
| `libgap-units-2` | 2026-10-23 | 6 unique commits within 7 days: wip(compiler): one unit rule, $n * $u where $u is unit, read |
| `me` | 2026-10-23 | same tip as host-from-warp; the user's own worktree branch (never removed) |
| `optional-helpers` | 2026-10-22 | 1 unique commits within 7 days: refactor: the compiler helpers that can answer ø declare lis |
| `pass-owners-table` | 2026-10-23 | 14 unique commits within 7 days: feat(compiler): passes.warp names the pass order and each re |
| `path-written` | 2026-10-21 | card path-written (Soon) |
| `pipe-rules-locals` | 2026-10-23 | 1 unique commits within 7 days: fix: rewrites: a name a definition binds anywhere in its bod |
| `pure-big-int-ops` | 2026-10-23 | being merged right now |
| `pure-big-int-power` | 2026-10-23 | being merged right now |
| `pure-c-imports` | 2026-10-23 | 13 unique commits within 7 days: fix(host): warp-host takes the compiler beside it before lib |
| `pure-class-methods` | 2026-10-22 | 1 unique commits within 7 days: wip(compiler): lift read-only class instance methods into fu |
| `pure-class-methods-3` | 2026-10-23 | 1 unique commits within 7 days: Merge commit '624b7d96f' into pure-class-methods-3 |
| `pure-class-methods-3b` | 2026-10-23 | 7 unique commits within 7 days: fix: a class body listing fields with spaces before a method |
| `pure-harness-loose-equal` | 2026-10-23 | 2 unique commits within 7 days: fix(pure-tests): Number from crate::extensions::numbers |
| `pure-index-shadow-fix` | 2026-10-23 | 2 unique commits within 7 days: fix: arity_overloads.warp and analyzer.warp name their calle |
| `pure-match-guard-arms` | 2026-10-23 | worktree in use at /workspace/cow/trees/pure-match-guard-arms |
| `pure-refusal-go` | 2026-10-23 | 2 unique commits within 7 days: feat: pure lane: tasks.warp: a block of statements as a task |
| `pure-repl` | 2026-10-23 | 21 unique commits within 7 days: chore: merge origin/warp-host-wasm-load (8d052c113) into pur |
| `pure-wrong-type-word-prefix` | 2026-10-23 | 1 unique commits within 7 days: merge: origin/main into pure-wrong-type-word-prefix |
| `reflection-bound` | 2026-10-21 | 1 unique commits within 7 days: fix: type(x), dir(x) and x.params of a parameter or local x  |
| `runtime-split` | 2026-10-23 | being merged right now (card type-zoned mentions it) |
| `scripts-off-cargo` | 2026-10-23 | 24 unique commits within 7 days: fix(scripts): box-self-check.sh puts the tree's pair beside  |
| `sin-says` | 2026-10-21 | card sin-says (Soon) |
| `subscripts` | 2026-10-23 | 4 unique commits within 7 days: fix: rewrites: `where $b is ø` / `is not ø` asks whether a h |
| `sugar-declarations` | 2026-10-23 | 3 unique commits within 7 days: chore: merge sugar-definitions 0f0cb389d into sugar-declarat |
| `sugar-definitions` | 2026-10-23 | 1 unique commits within 7 days: chore: merge origin/main into sugar-definitions: main's `def |
| `sugar-foreign-forms` | 2026-10-23 | 4 unique commits within 7 days: Merge remote-tracking branch 'origin/main' into sugar-foreig |
| `sugar-type-tests` | 2026-10-23 | 5 unique commits within 7 days: Merge remote-tracking branch 'origin/main' into sugar-type-t |
| `test-words` | 2026-10-23 | 2 unique commits within 7 days: merge origin/main into test-words: sugar/test_words after ma |
| `time-arithmetic-library` | 2026-10-23 | being merged right now |
| `type-zoned` | 2026-10-21 | card type-zoned (Soon) |
| `warning-positions` | 2026-10-22 | 1 unique commits within 7 days: feature(minor): warp checks' diagnostics name where they wer |
| `warp-host-cache-key` | 2026-10-23 | 25 unique commits within 7 days: Merge remote-tracking branch 'origin/warp-host-integration'  |
| `warp-host-cli-parity` | 2026-10-23 | 27 unique commits within 7 days: fix(warp-host): --fuel gives a store its steps when it is ma |
| `warp-host-compiler-beside` | 2026-10-23 | 18 unique commits within 7 days: merge: origin/main into warp-host-compiler-beside |
| `warp-host-compiler-build` | 2026-10-23 | 14 unique commits within 7 days: build(host): the host build puts compiler-self.wasm and comp |
| `warp-host-demo` | 2026-10-23 | 19 unique commits within 7 days: merge: warp-host-aot (b91a719e0) into host-from-warp (a3c6f5 |
| `warp-host-inline-status` | 2026-10-23 | 2 unique commits within 7 days: test(host): inline code checked by its value, not its status |
| `warp-host-integration` | 2026-10-23 | 23 unique commits within 7 days: chore: merge origin/main (b84496b86) into warp-host-integrat |
| `warp-host-repl-fixes` | 2026-10-23 | 20 unique commits within 7 days: test(warp-host): repl fixes checked by their results only, n |
| `warp-host-wasm-load` | 2026-10-23 | 17 unique commits within 7 days: chore: merge origin/main (0c9660041) into warp-host-wasm-loa |
| `wasm-commands` | 2026-10-23 | being merged right now |

## Superseded

| branch | last commit | reason |
|---|---|---|
| `backup/--help` | 2026-10-23 | only todo.md fallback lines; main's todo.md carries them (pure-class-operators in a newer wording) |
| `backup/compiler-core-link` | 2026-10-23 | merging it into main changes nothing (squash-equivalent, merge-tree == main) |
| `backup/gate-probes` | 2026-10-23 | probes/gate and scripts landed on main; the one hunk left is an older line main replaced (a8a824e44) |
| `backup/host-glue-native-notes` | 2026-10-23 | only a todo.md fallback line already in main; branch merged (dddb84990) |
| `backup/pipe-rules-locals` | 2026-10-23 | fully contained in origin/pipe-rules-locals (merge-tree == that branch) |
| `backup/pure-closure-cells` | 2026-10-23 | merging it into main changes nothing (squash-equivalent, merge-tree == main) |
| `backup/sc-main-now` | 2026-10-23 | probe_tests/test_rewrite_engine.warp is byte-identical to a blob on main |
| `backup/sc-sugar-preposition-heads` | 2026-10-23 | both probe_tests files are byte-identical to blobs on main |
| `backup/test-no-refusal-asserts-2` | 2026-10-23 | only a todo.md fallback line already in main |
| `backup/warp` | 2026-10-23 | only todo.md fallback lines; main's todo.md carries them (pure-class-operators in a newer wording) |
| `batch-2` | 2026-10-22 | its content is contained in wasm-commands (being merged now); integrator batch scratch |
| `code-patterns` | 2026-10-21 | its one commit's subject is on main; landed via compiler-code-patterns (fa0a59686), follow-up is card code-patterns-2 |
| `rewrite-zip-runs` | 2026-10-23 | landed as rewrite-zip-runs-2 (main f3b46668b) |
| `sugar-prefix-comparisons` | 2026-10-22 | landed as sugar-prefix-comparisons-2 (main 9b0dd0ce4, card sugar-prefix-comparisons) |
| `wasm-commands-merge` | 2026-10-22 | all but ~5 lines are in wasm-commands (being merged now) |

## Stale junk

| branch | last commit | reason |
|---|---|---|
| `backup/pcm-bisect2` | 2026-10-23 | bisect scratch: deletes 224 lines from lib/compiler to bisect pcm |
| `backup/pr-go` | 2026-10-23 | test artifacts only (task.txt "from the task", voices.wav) |
| `backup/pr-go-base` | 2026-10-23 | test artifacts only (task.txt, voices.wav) |
| `backup/sc-engine-guarded-call-phrase` | 2026-10-23 | self-check copy probe_tests/ of a branch merged into main |
| `backup/sc-engine-placed-where-chain` | 2026-10-23 | self-check copy probe_tests/ of a branch merged into main |
| `backup/sc-engine-rest-template` | 2026-10-23 | self-check copy probe_tests/ of a branch merged into main |
| `bisect-28993dd601` | 2026-10-21 | bisect run ref from 2026-10-21 |
| `bisect-b81fceddd8` | 2026-10-21 | bisect run ref from 2026-10-21 |
| `bisect-c8bb9c815c` | 2026-10-21 | bisect run ref from 2026-10-21 |
| `bisect-e117efbdff` | 2026-10-21 | bisect run ref from 2026-10-21 |
| `cloud-jobs/batch-4-ee38fe84-08741f6e3669-1792698812-49180` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/batch-4-ee38fe84-2946cb0087fe-1792698818-49180` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/batch-4-ee38fe84-33670a979944-1792698831-49180` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/batch-4-ee38fe84-910f20ec58e9-1792698824-49180` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/batch-4-ee38fe84-c8f933d483d9-1792698838-49180` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/installed-compilers-527dc89f-3b5fb1df5b51-1792659781-80987` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/list-result-autocast-e4119de3-7ffd86edab17-1792661101-67726` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/warp-fix-offers-205901ea-7ffd86edab17-1792658161-93010` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/wasi-runtime-0e16d58a-3c00553842c0-1792674868-35693` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/wasi-runtime-416a1eb0-ac36aac11fdb-1792657387-32421` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/wasi-runtime-67dcaa84-7ffd86edab17-1792656148-31349` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/wasi-runtime-e9761055-b63bb67db90b-1792657761-60127` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/wasm-commands-3de5faae-f7be9f06b4e9-1792655352-93759` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-jobs/wasm-commands-a46aeaf2-f7be9f06b4e9-1792659916-88344` | 2026-10-22 | cloud test job ref (orphan commit, test lane scratch) |
| `cloud-results/batch-4-ee38fe84-08741f6e3669` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/batch-4-ee38fe84-2946cb0087fe` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/batch-4-ee38fe84-33670a979944` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/batch-4-ee38fe84-910f20ec58e9` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/batch-4-ee38fe84-c8f933d483d9` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/list-result-autocast-e4119de3-7ffd86edab17` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/wasi-runtime-03c74c0f-3ebd6e536c89` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/wasi-runtime-03c74c0f-7ffd86edab17` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/wasi-runtime-0e16d58a-3c00553842c0` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `cloud-results/wasm-commands-a46aeaf2-f7be9f06b4e9` | 2026-10-22 | cloud test result ref (orphan commit, test lane scratch) |
| `program-units` | 2026-10-15 | last commit 2026-10-15 (9 days), 6299 commits behind main, no card found; unique unit-definitions work, glance before deleting |
| `vendor` | 2026-10-22 | automatic vendored-crates refresh (29851 files); vendor was deactivated (decision 2026-10-03, 310f4292b) |

## Suggested bulk delete: AWAITING USER APPROVAL

Not run. Superseded:

```sh
git push origin --delete \
  backup/--help \
  backup/compiler-core-link \
  backup/gate-probes \
  backup/host-glue-native-notes \
  backup/pipe-rules-locals \
  backup/pure-closure-cells \
  backup/sc-main-now \
  backup/sc-sugar-preposition-heads \
  backup/test-no-refusal-asserts-2 \
  backup/warp \
  batch-2 \
  code-patterns \
  rewrite-zip-runs \
  sugar-prefix-comparisons \
  wasm-commands-merge
```

Stale junk:

```sh
git push origin --delete \
  backup/pcm-bisect2 \
  backup/pr-go \
  backup/pr-go-base \
  backup/sc-engine-guarded-call-phrase \
  backup/sc-engine-placed-where-chain \
  backup/sc-engine-rest-template \
  bisect-28993dd601 \
  bisect-b81fceddd8 \
  bisect-c8bb9c815c \
  bisect-e117efbdff \
  cloud-jobs/batch-4-ee38fe84-08741f6e3669-1792698812-49180 \
  cloud-jobs/batch-4-ee38fe84-2946cb0087fe-1792698818-49180 \
  cloud-jobs/batch-4-ee38fe84-33670a979944-1792698831-49180 \
  cloud-jobs/batch-4-ee38fe84-910f20ec58e9-1792698824-49180 \
  cloud-jobs/batch-4-ee38fe84-c8f933d483d9-1792698838-49180 \
  cloud-jobs/installed-compilers-527dc89f-3b5fb1df5b51-1792659781-80987 \
  cloud-jobs/list-result-autocast-e4119de3-7ffd86edab17-1792661101-67726 \
  cloud-jobs/warp-fix-offers-205901ea-7ffd86edab17-1792658161-93010 \
  cloud-jobs/wasi-runtime-0e16d58a-3c00553842c0-1792674868-35693 \
  cloud-jobs/wasi-runtime-416a1eb0-ac36aac11fdb-1792657387-32421 \
  cloud-jobs/wasi-runtime-67dcaa84-7ffd86edab17-1792656148-31349 \
  cloud-jobs/wasi-runtime-e9761055-b63bb67db90b-1792657761-60127 \
  cloud-jobs/wasm-commands-3de5faae-f7be9f06b4e9-1792655352-93759 \
  cloud-jobs/wasm-commands-a46aeaf2-f7be9f06b4e9-1792659916-88344 \
  cloud-results/batch-4-ee38fe84-08741f6e3669 \
  cloud-results/batch-4-ee38fe84-2946cb0087fe \
  cloud-results/batch-4-ee38fe84-33670a979944 \
  cloud-results/batch-4-ee38fe84-910f20ec58e9 \
  cloud-results/batch-4-ee38fe84-c8f933d483d9 \
  cloud-results/list-result-autocast-e4119de3-7ffd86edab17 \
  cloud-results/wasi-runtime-03c74c0f-3ebd6e536c89 \
  cloud-results/wasi-runtime-03c74c0f-7ffd86edab17 \
  cloud-results/wasi-runtime-0e16d58a-3c00553842c0 \
  cloud-results/wasm-commands-a46aeaf2-f7be9f06b4e9 \
  program-units \
  vendor
```
